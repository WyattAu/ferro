use async_trait::async_trait;
use chrono::Utc;
use common::error::FerroError;
use common::error::Result;
use common::storage::LockManagerTrait;
use common::webdav::{LockDepth, LockInfo, LockScope, LockToken, LockType};
use dashmap::DashMap;
use rusqlite::params;
use std::sync::Arc;
use tracing::{debug, warn};

pub type DbHandle = Arc<std::sync::Mutex<rusqlite::Connection>>;

pub use common::storage::LockManagerTrait as _;

pub struct LockManager {
    pub(crate) locks: Arc<DashMap<String, LockInfo>>,
    default_timeout_secs: u32,
    max_timeout_secs: u32,
    db: Option<DbHandle>,
}

impl LockManager {
    pub fn new() -> Self {
        Self {
            locks: Arc::new(DashMap::new()),
            default_timeout_secs: 60,
            max_timeout_secs: 3600,
            db: None,
        }
    }

    pub fn with_timeout(default_timeout_secs: u32, max_timeout_secs: u32) -> Self {
        Self {
            locks: Arc::new(DashMap::new()),
            default_timeout_secs,
            max_timeout_secs,
            db: None,
        }
    }

    pub fn with_db(mut self, db: DbHandle) -> Self {
        self.db = Some(db);
        self
    }

    fn persist_lock(&self, lock: &LockInfo) {
        if let Some(ref db) = self.db
            && let Err(e) = db.lock().unwrap_or_else(|e| e.into_inner()).execute(
                "INSERT OR REPLACE INTO locks (token, path, principal, scope, lock_type, depth, timeout_seconds, created_at, refresh_count) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    lock.token.as_str(),
                    lock.path,
                    lock.principal,
                    format!("{:?}", lock.scope),
                    format!("{:?}", lock.lock_type),
                    format!("{:?}", lock.depth),
                    lock.timeout_seconds as i64,
                    lock.created_at.to_rfc3339(),
                    lock.refresh_count as i64,
                ],
            )
        {
            warn!("Failed to persist lock to SQLite: {}", e);
        }
    }

    fn persist_release(&self, token: &str) {
        if let Some(ref db) = self.db
            && let Err(e) = db
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .execute("DELETE FROM locks WHERE token = ?1", params![token])
        {
            warn!("Failed to delete lock from SQLite: {}", e);
        }
    }

    pub fn acquire_lock_sync(
        &self,
        path: &str,
        principal: &str,
        scope: LockScope,
        depth: LockDepth,
        timeout_secs: Option<u32>,
    ) -> Result<LockInfo> {
        self.cleanup_expired(path);

        if let Some(existing) = self.locks.get(path) {
            if existing.is_expired() {
                drop(existing);
                self.locks.remove(path);
            } else if existing.scope == LockScope::Exclusive {
                return Err(FerroError::LockConflict(format!(
                    "Resource {} is exclusively locked by {}",
                    path, existing.principal
                )));
            }
        }

        let timeout = timeout_secs
            .unwrap_or(self.default_timeout_secs)
            .min(self.max_timeout_secs);

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

        debug!(
            "LOCK acquired: {} by {} (scope={:?}, timeout={}s)",
            path, principal, scope, timeout
        );

        self.locks.insert(path.to_string(), lock.clone());
        self.persist_lock(&lock);
        Ok(lock)
    }

    pub fn refresh_lock_sync(&self, token: &str, timeout_secs: Option<u32>) -> Result<LockInfo> {
        let timeout = timeout_secs
            .unwrap_or(self.default_timeout_secs)
            .min(self.max_timeout_secs);

        for entry in self.locks.iter() {
            if entry.token.as_str() == token {
                if entry.is_expired() {
                    return Err(FerroError::LockTokenNotFound(token.to_string()));
                }
                let mut lock = entry.value().clone();
                lock.timeout_seconds = timeout;
                lock.created_at = Utc::now();
                lock.refresh_count += 1;
                debug!(
                    "LOCK refreshed: {} (refresh #{})",
                    entry.key(),
                    lock.refresh_count
                );
                self.persist_lock(&lock);
                return Ok(lock);
            }
        }

        Err(FerroError::LockTokenNotFound(token.to_string()))
    }

    pub fn release_lock_sync(&self, token: &str) -> Result<()> {
        let mut found = None;
        for entry in self.locks.iter() {
            if entry.token.as_str() == token {
                found = Some(entry.key().clone());
                break;
            }
        }

        if let Some(key) = found {
            self.locks.remove(&key);
            debug!("LOCK released: {}", key);
            self.persist_release(token);
            Ok(())
        } else {
            Err(FerroError::LockTokenNotFound(token.to_string()))
        }
    }

    pub fn check_lock_sync(&self, path: &str) -> Option<LockInfo> {
        self.cleanup_expired(path);
        self.locks.get(path).map(|r| r.value().clone())
    }

    pub fn check_lock_for_write_sync(&self, path: &str) -> Result<()> {
        if let Some(lock) = self.check_lock_sync(path)
            && lock.scope == LockScope::Exclusive
        {
            return Err(FerroError::LockConflict(format!(
                "Resource {} is exclusively locked by {}",
                path, lock.principal
            )));
        }

        let mut check_path = path;
        while let Some(parent) = parent_path(check_path) {
            self.cleanup_expired(parent);
            if let Some(lock) = self.locks.get(parent)
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

    fn cleanup_expired(&self, path: &str) {
        if let Some(entry) = self.locks.get(path)
            && entry.is_expired()
        {
            warn!("LOCK expired: {}", path);
            drop(entry);
            self.locks.remove(path);
        }
    }

    pub fn lock_count(&self) -> usize {
        self.locks.len()
    }

    pub fn all_locks_sync(&self) -> dashmap::iter::Iter<'_, String, LockInfo> {
        self.locks.iter()
    }

    pub fn cleanup_all_expired_sync(&self) {
        self.locks.retain(|_, entry| {
            if entry.is_expired() {
                warn!("LOCK expired (global cleanup): {}", entry.path);
                false
            } else {
                true
            }
        });
    }

    pub fn load_all_from_db(
        &self,
        conn: &rusqlite::Connection,
    ) -> std::result::Result<(), rusqlite::Error> {
        let mut stmt = conn.prepare(
            "SELECT token, path, principal, scope, lock_type, depth, timeout_seconds, created_at, refresh_count FROM locks",
        )?;
        let rows = stmt.query_map([], |row| {
            let token_str: String = row.get(0)?;
            let token_uuid = token_str.strip_prefix("urn:uuid:").unwrap_or(&token_str);
            let token = common::webdav::LockToken::from_str_custom(token_uuid).unwrap_or_default();
            let scope_str: String = row.get(3)?;
            let scope = match scope_str.as_str() {
                "Exclusive" => LockScope::Exclusive,
                "Shared" => LockScope::Shared,
                _ => LockScope::Exclusive,
            };
            let lock_type_str: String = row.get(4)?;
            let lock_type = match lock_type_str.as_str() {
                "Write" => LockType::Write,
                _ => LockType::Write,
            };
            let depth_str: String = row.get(5)?;
            let depth = match depth_str.as_str() {
                "Infinity" | "infinity" => LockDepth::Infinity,
                _ => LockDepth::Zero,
            };
            let created_at_str: String = row.get(7)?;
            let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| Utc::now());
            Ok(LockInfo {
                token,
                path: row.get(1)?,
                principal: row.get(2)?,
                scope,
                lock_type,
                depth,
                timeout_seconds: row.get::<_, i64>(6)? as u32,
                created_at,
                refresh_count: row.get::<_, i64>(8)? as u32,
            })
        })?;
        for row in rows {
            let lock: LockInfo = row?;
            if !lock.is_expired() {
                self.locks.insert(lock.path.clone(), lock);
            }
        }
        Ok(())
    }
}

#[async_trait]
impl LockManagerTrait for LockManager {
    async fn check_lock(&self, path: &str) -> Option<LockInfo> {
        self.check_lock_sync(path)
    }

    async fn check_lock_for_write(&self, path: &str) -> Result<()> {
        self.check_lock_for_write_sync(path)
    }

    async fn acquire_lock(
        &self,
        path: &str,
        principal: &str,
        scope: LockScope,
        depth: LockDepth,
        timeout_secs: Option<u32>,
    ) -> Result<LockInfo> {
        self.acquire_lock_sync(path, principal, scope, depth, timeout_secs)
    }

    async fn release_lock(&self, token: &str) -> Result<()> {
        self.release_lock_sync(token)
    }

    async fn refresh_lock(&self, token: &str, timeout_secs: Option<u32>) -> Result<LockInfo> {
        self.refresh_lock_sync(token, timeout_secs)
    }

    async fn all_locks(&self) -> Vec<LockInfo> {
        self.locks.iter().map(|e| e.value().clone()).collect()
    }

    async fn cleanup_all_expired(&self) {
        self.cleanup_all_expired_sync();
    }
}

impl Default for LockManager {
    fn default() -> Self {
        Self::new()
    }
}

pub fn parent_path(path: &str) -> Option<&str> {
    if path == "/" {
        return None;
    }
    let trimmed = path.trim_end_matches('/');
    match trimmed.rfind('/') {
        Some(0) => Some("/"),
        Some(idx) => Some(&trimmed[..idx.max(1)]),
        None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_acquire_and_release_lock() {
        let mgr = LockManager::new();
        let lock = mgr
            .acquire_lock_sync(
                "/test.txt",
                "user1",
                LockScope::Exclusive,
                LockDepth::Zero,
                None,
            )
            .unwrap();
        assert_eq!(mgr.lock_count(), 1);

        mgr.release_lock_sync(&lock.token.as_str()).unwrap();
        assert_eq!(mgr.lock_count(), 0);
    }

    #[test]
    fn test_exclusive_lock_conflict() {
        let mgr = LockManager::new();
        mgr.acquire_lock_sync(
            "/test.txt",
            "user1",
            LockScope::Exclusive,
            LockDepth::Zero,
            None,
        )
        .unwrap();

        let result = mgr.acquire_lock_sync(
            "/test.txt",
            "user2",
            LockScope::Exclusive,
            LockDepth::Zero,
            None,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_parent_path() {
        assert_eq!(parent_path("/a/b/c"), Some("/a/b"));
        assert_eq!(parent_path("/a"), Some("/"));
        assert_eq!(parent_path("/"), None);
    }
}
