use async_trait::async_trait;
use chrono::Utc;
use common::error::{FerroError, Result};
use common::webdav::{LockDepth, LockInfo, LockScope, LockToken, LockType};
use redis::aio::ConnectionManager;
use std::time::Duration;
use tracing::debug;

use common::storage::LockManagerTrait;

const DEFAULT_TIMEOUT_SECS: u32 = 86400;
const REDIS_OP_TIMEOUT: Duration = Duration::from_secs(5);
const KEY_PREFIX: &str = "ferro:lock";
const TOKEN_INDEX_PREFIX: &str = "ferro:lock:token";

fn parent_path(path: &str) -> Option<&str> {
    let trimmed = path.trim_end_matches('/');
    trimmed
        .rsplit_once('/')
        .map(|(parent, _)| if parent.is_empty() { "/" } else { parent })
}

pub struct RedisLockManager {
    client: ConnectionManager,
    default_timeout_secs: u32,
}

impl RedisLockManager {
    pub async fn new(redis_url: &str) -> anyhow::Result<Self> {
        let client = redis::Client::open(redis_url)
            .map_err(|e| anyhow::anyhow!("Failed to create Redis client: {}", e))?;
        let mgr = ConnectionManager::new(client)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create Redis connection manager: {}", e))?;

        debug!("RedisLockManager initialized");
        Ok(Self {
            client: mgr,
            default_timeout_secs: DEFAULT_TIMEOUT_SECS,
        })
    }

    fn lock_key(path: &str) -> String {
        format!("{}:{}", KEY_PREFIX, path)
    }

    fn token_index_key(token: &str) -> String {
        format!("{}:{}", TOKEN_INDEX_PREFIX, token)
    }

    async fn get_lock_info(&self, path: &str) -> Option<LockInfo> {
        let key = Self::lock_key(path);
        let result: Vec<(String, String)> = tokio::time::timeout(REDIS_OP_TIMEOUT, async {
            redis::cmd("HGETALL")
                .arg(&key)
                .query_async(&mut self.client.clone())
                .await
        })
        .await
        .ok()?
        .ok()?;

        if result.is_empty() {
            return None;
        }

        let get = |name: &str| -> Option<String> {
            result
                .iter()
                .find(|(k, _)| k == name)
                .map(|(_, v)| v.clone())
        };

        let token_str = get("token")?;
        let token = LockToken::from_str_custom(&token_str.replace("urn:uuid:", ""))?;

        let scope = match get("scope")?.as_str() {
            "Exclusive" => LockScope::Exclusive,
            _ => LockScope::Shared,
        };

        let depth = match get("depth")?.as_str() {
            "Zero" => LockDepth::Zero,
            "One" => LockDepth::One,
            _ => LockDepth::Infinity,
        };

        let created_at = get("created_at")
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);

        let timeout_seconds: u32 = get("timeout_seconds")
            .and_then(|s| s.parse().ok())
            .unwrap_or(60);
        let refresh_count: u32 = get("refresh_count")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        Some(LockInfo {
            token,
            path: get("path").unwrap_or_else(|| path.to_string()),
            principal: get("principal").unwrap_or_default(),
            scope,
            lock_type: LockType::Write,
            depth,
            timeout_seconds,
            created_at,
            refresh_count,
        })
    }

    async fn store_lock(&self, lock: &LockInfo) -> Result<()> {
        let key = Self::lock_key(&lock.path);
        let token_index = Self::token_index_key(&lock.token.as_opaque());
        let ttl = lock.timeout_seconds as i64;

        let mut conn = self.client.clone();
        let mut pipe = redis::pipe();
        pipe.cmd("HSET")
            .arg(&key)
            .arg("token")
            .arg(lock.token.as_str())
            .arg("path")
            .arg(&lock.path)
            .arg("principal")
            .arg(&lock.principal)
            .arg("scope")
            .arg(format!("{:?}", lock.scope))
            .arg("depth")
            .arg(format!("{:?}", lock.depth))
            .arg("lock_type")
            .arg(format!("{:?}", lock.lock_type))
            .arg("timeout_seconds")
            .arg(lock.timeout_seconds)
            .arg("created_at")
            .arg(lock.created_at.to_rfc3339())
            .arg("refresh_count")
            .arg(lock.refresh_count);
        pipe.cmd("EXPIRE").arg(&key).arg(ttl);
        pipe.cmd("SET")
            .arg(&token_index)
            .arg(&lock.path)
            .arg("EX")
            .arg(ttl)
            .arg("NX");

        tokio::time::timeout(REDIS_OP_TIMEOUT, async {
            pipe.query_async::<()>(&mut conn).await
        })
        .await
        .map_err(|_| FerroError::Timeout)?
        .map_err(|e| FerroError::Internal(format!("Redis HSET failed: {}", e)))?;

        Ok(())
    }
}

#[async_trait]
impl LockManagerTrait for RedisLockManager {
    async fn check_lock(&self, path: &str) -> Option<LockInfo> {
        let lock = self.get_lock_info(path).await?;
        if lock.is_expired() {
            let key = Self::lock_key(path);
            let _ = tokio::time::timeout(REDIS_OP_TIMEOUT, async {
                redis::cmd("DEL")
                    .arg(&key)
                    .query_async::<()>(&mut self.client.clone())
                    .await
            })
            .await;
            return None;
        }
        Some(lock)
    }

    async fn check_lock_for_write(&self, path: &str) -> Result<()> {
        if let Some(lock) = self.check_lock(path).await
            && lock.scope == LockScope::Exclusive
        {
            return Err(FerroError::LockConflict(format!(
                "Resource {} is exclusively locked by {}",
                path, lock.principal
            )));
        }

        let mut check_path = path;
        while let Some(parent) = parent_path(check_path) {
            if let Some(lock) = self.check_lock(parent).await
                && lock.depth == LockDepth::Infinity
                && lock.scope == LockScope::Exclusive
            {
                return Err(FerroError::LockConflict(format!(
                    "Parent {} has an exclusive infinity lock by {}",
                    parent, lock.principal
                )));
            }
            check_path = parent;
            if check_path == "/" {
                break;
            }
        }

        Ok(())
    }

    async fn acquire_lock(
        &self,
        path: &str,
        principal: &str,
        scope: LockScope,
        depth: LockDepth,
        timeout_secs: Option<u32>,
    ) -> Result<LockInfo> {
        if let Some(existing) = self.check_lock(path).await
            && existing.scope == LockScope::Exclusive
        {
            return Err(FerroError::LockConflict(format!(
                "Resource {} is exclusively locked by {}",
                path, existing.principal
            )));
        }

        let timeout = timeout_secs.unwrap_or(self.default_timeout_secs);

        let lock = LockInfo {
            token: LockToken::new(),
            path: path.to_string(),
            principal: principal.to_string(),
            scope,
            lock_type: LockType::Write,
            depth,
            timeout_seconds: timeout,
            created_at: Utc::now(),
            refresh_count: 0,
        };

        self.store_lock(&lock).await?;

        debug!(
            "LOCK acquired (Redis): {} by {} (scope={:?}, timeout={}s)",
            path, principal, scope, timeout
        );

        Ok(lock)
    }

    async fn release_lock(&self, token: &str) -> Result<()> {
        let token_index_key = Self::token_index_key(token);

        let path: Option<String> = tokio::time::timeout(REDIS_OP_TIMEOUT, async {
            redis::cmd("GET")
                .arg(&token_index_key)
                .query_async(&mut self.client.clone())
                .await
        })
        .await
        .map_err(|_| FerroError::Timeout)?
        .map_err(|e| FerroError::Internal(format!("Redis GET failed: {}", e)))?;

        let path = match path {
            Some(p) => p,
            None => return Err(FerroError::LockTokenNotFound(token.to_string())),
        };

        let lock_key = Self::lock_key(&path);

        let script = redis::Script::new(
            r#"
            local token = ARGV[1]
            local actual = redis.call('HGET', KEYS[1], 'token')
            if actual and string.find(actual, token) then
                return redis.call('DEL', KEYS[1])
            else
                return 0
            end
            "#,
        );

        let mut conn = self.client.clone();
        let deleted: i32 = tokio::time::timeout(REDIS_OP_TIMEOUT, async {
            script
                .key(&lock_key)
                .arg(token)
                .invoke_async(&mut conn)
                .await
        })
        .await
        .map_err(|_| FerroError::Timeout)?
        .map_err(|e| FerroError::Internal(format!("Redis Lua script failed: {}", e)))?;

        let _ = tokio::time::timeout(REDIS_OP_TIMEOUT, async {
            redis::cmd("DEL")
                .arg(&token_index_key)
                .query_async::<()>(&mut self.client.clone())
                .await
        })
        .await;

        if deleted > 0 {
            debug!("LOCK released (Redis): {}", path);
            Ok(())
        } else {
            Err(FerroError::LockTokenNotFound(token.to_string()))
        }
    }

    async fn refresh_lock(&self, token: &str, timeout_secs: Option<u32>) -> Result<LockInfo> {
        let token_index_key = Self::token_index_key(token);

        let path: Option<String> = tokio::time::timeout(REDIS_OP_TIMEOUT, async {
            redis::cmd("GET")
                .arg(&token_index_key)
                .query_async(&mut self.client.clone())
                .await
        })
        .await
        .map_err(|_| FerroError::Timeout)?
        .map_err(|e| FerroError::Internal(format!("Redis GET failed: {}", e)))?;

        let path = match path {
            Some(p) => p,
            None => return Err(FerroError::LockTokenNotFound(token.to_string())),
        };

        let mut lock = match self.get_lock_info(&path).await {
            Some(l) if !l.is_expired() => l,
            _ => return Err(FerroError::LockTokenNotFound(token.to_string())),
        };

        let timeout = timeout_secs.unwrap_or(self.default_timeout_secs);
        lock.timeout_seconds = timeout;
        lock.created_at = Utc::now();
        lock.refresh_count += 1;

        self.store_lock(&lock).await?;

        debug!(
            "LOCK refreshed (Redis): {} (refresh #{})",
            path, lock.refresh_count
        );

        Ok(lock)
    }

    async fn all_locks(&self) -> Vec<LockInfo> {
        let pattern = format!("{}:*", KEY_PREFIX);
        let mut locks = Vec::new();
        let mut cursor: u64 = 0;

        loop {
            let (new_cursor, keys): (u64, Vec<String>) =
                match tokio::time::timeout(REDIS_OP_TIMEOUT, async {
                    redis::cmd("SCAN")
                        .arg(cursor)
                        .arg("MATCH")
                        .arg(&pattern)
                        .arg("COUNT")
                        .arg(100)
                        .query_async(&mut self.client.clone())
                        .await
                })
                .await
                {
                    Ok(Ok(result)) => result,
                    _ => break,
                };

            cursor = new_cursor;
            for key in keys {
                if let Some(lock) = self.get_lock_info_by_key(&key).await
                    && !lock.is_expired()
                {
                    locks.push(lock);
                }
            }

            if cursor == 0 {
                break;
            }
        }

        locks
    }
}

impl RedisLockManager {
    async fn get_lock_info_by_key(&self, key: &str) -> Option<LockInfo> {
        let result: Vec<(String, String)> = tokio::time::timeout(REDIS_OP_TIMEOUT, async {
            redis::cmd("HGETALL")
                .arg(key)
                .query_async(&mut self.client.clone())
                .await
        })
        .await
        .ok()?
        .ok()?;

        if result.is_empty() {
            return None;
        }

        let get = |name: &str| -> Option<String> {
            result
                .iter()
                .find(|(k, _)| k == name)
                .map(|(_, v)| v.clone())
        };

        let token_str = get("token")?;
        let token = LockToken::from_str_custom(&token_str.replace("urn:uuid:", ""))?;

        let scope = match get("scope")?.as_str() {
            "Exclusive" => LockScope::Exclusive,
            _ => LockScope::Shared,
        };

        let depth = match get("depth")?.as_str() {
            "Zero" => LockDepth::Zero,
            "One" => LockDepth::One,
            _ => LockDepth::Infinity,
        };

        let created_at = get("created_at")
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);

        let timeout_seconds: u32 = get("timeout_seconds")
            .and_then(|s| s.parse().ok())
            .unwrap_or(60);
        let refresh_count: u32 = get("refresh_count")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        Some(LockInfo {
            token,
            path: get("path").unwrap_or_default(),
            principal: get("principal").unwrap_or_default(),
            scope,
            lock_type: LockType::Write,
            depth,
            timeout_seconds,
            created_at,
            refresh_count,
        })
    }
}
