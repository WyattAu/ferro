use chrono::Utc;
use common::webdav::{LockDepth, LockInfo, LockScope, LockToken, LockType};
use common::error::FerroError;
use common::error::Result;
use dashmap::DashMap;
use std::sync::Arc;
use tracing::{debug, warn};

pub struct LockManager {
    locks: Arc<DashMap<String, LockInfo>>,
    default_timeout_secs: u32,
    max_timeout_secs: u32,
}

impl LockManager {
    pub fn new() -> Self {
        Self {
            locks: Arc::new(DashMap::new()),
            default_timeout_secs: 60,
            max_timeout_secs: 3600,
        }
    }

    pub fn with_timeout(default_timeout_secs: u32, max_timeout_secs: u32) -> Self {
        Self {
            locks: Arc::new(DashMap::new()),
            default_timeout_secs,
            max_timeout_secs,
        }
    }

    pub fn acquire_lock(
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
        Ok(lock)
    }

    pub fn refresh_lock(&self, token: &str, timeout_secs: Option<u32>) -> Result<LockInfo> {
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
                return Ok(lock);
            }
        }

        Err(FerroError::LockTokenNotFound(token.to_string()))
    }

    pub fn release_lock(&self, token: &str) -> Result<()> {
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
            Ok(())
        } else {
            Err(FerroError::LockTokenNotFound(token.to_string()))
        }
    }

    pub fn check_lock(&self, path: &str) -> Option<LockInfo> {
        self.cleanup_expired(path);
        self.locks.get(path).map(|r| r.value().clone())
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

    pub fn all_locks(&self) -> dashmap::iter::Iter<'_, String, LockInfo> {
        self.locks.iter()
    }
}

impl Default for LockManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_acquire_and_release_lock() {
        let mgr = LockManager::new();
        let lock = mgr
            .acquire_lock(
                "/test.txt",
                "user1",
                LockScope::Exclusive,
                LockDepth::Zero,
                None,
            )
            .unwrap();
        assert_eq!(mgr.lock_count(), 1);

        mgr.release_lock(&lock.token.as_str()).unwrap();
        assert_eq!(mgr.lock_count(), 0);
    }

    #[test]
    fn test_exclusive_lock_conflict() {
        let mgr = LockManager::new();
        mgr.acquire_lock(
            "/test.txt",
            "user1",
            LockScope::Exclusive,
            LockDepth::Zero,
            None,
        )
        .unwrap();

        let result = mgr.acquire_lock(
            "/test.txt",
            "user2",
            LockScope::Exclusive,
            LockDepth::Zero,
            None,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_shared_lock_allowed() {
        let mgr = LockManager::new();
        mgr.acquire_lock(
            "/test.txt",
            "user1",
            LockScope::Shared,
            LockDepth::Zero,
            None,
        )
        .unwrap();

        let result = mgr.acquire_lock(
            "/test.txt",
            "user2",
            LockScope::Shared,
            LockDepth::Zero,
            None,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_refresh_lock() {
        let mgr = LockManager::new();
        let lock = mgr
            .acquire_lock(
                "/test.txt",
                "user1",
                LockScope::Exclusive,
                LockDepth::Zero,
                None,
            )
            .unwrap();

        let refreshed = mgr.refresh_lock(&lock.token.as_str(), Some(120)).unwrap();
        assert_eq!(refreshed.timeout_seconds, 120);
        assert_eq!(refreshed.refresh_count, 1);
    }

    #[test]
    fn test_release_nonexistent_lock() {
        let mgr = LockManager::new();
        let result = mgr.release_lock("nonexistent");
        assert!(result.is_err());
    }
}
