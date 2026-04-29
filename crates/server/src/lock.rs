use async_trait::async_trait;
use chrono::Utc;
use common::error::FerroError;
use common::error::Result;
use common::webdav::{LockDepth, LockInfo, LockScope, LockToken, LockType};
use dashmap::DashMap;
use std::sync::Arc;
use tracing::{debug, warn};

/// Trait for managing WebDAV locks across the server.
#[async_trait]
pub trait LockManagerTrait: Send + Sync {
    async fn check_lock(&self, path: &str) -> Option<LockInfo>;
    async fn check_lock_for_write(&self, path: &str) -> Result<()>;
    async fn acquire_lock(
        &self,
        path: &str,
        principal: &str,
        scope: LockScope,
        depth: LockDepth,
        timeout_secs: Option<u32>,
    ) -> Result<LockInfo>;
    async fn release_lock(&self, token: &str) -> Result<()>;
    async fn refresh_lock(&self, token: &str, timeout_secs: Option<u32>) -> Result<LockInfo>;
    async fn all_locks(&self) -> Vec<LockInfo>;
    async fn cleanup_all_expired(&self) {}
}

/// In-memory lock manager backed by a [`DashMap`].
pub struct LockManager {
    locks: Arc<DashMap<String, LockInfo>>,
    default_timeout_secs: u32,
    max_timeout_secs: u32,
}

impl LockManager {
    /// Create a new lock manager with default timeouts (60s default, 3600s max).
    pub fn new() -> Self {
        Self {
            locks: Arc::new(DashMap::new()),
            default_timeout_secs: 60,
            max_timeout_secs: 3600,
        }
    }

    /// Create a new lock manager with custom timeout values.
    pub fn with_timeout(default_timeout_secs: u32, max_timeout_secs: u32) -> Self {
        Self {
            locks: Arc::new(DashMap::new()),
            default_timeout_secs,
            max_timeout_secs,
        }
    }

    /// Synchronously acquire a lock on a path.
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
        Ok(lock)
    }

    /// Synchronously refresh a lock's timeout.
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
                return Ok(lock);
            }
        }

        Err(FerroError::LockTokenNotFound(token.to_string()))
    }

    /// Synchronously release a lock by token.
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
            Ok(())
        } else {
            Err(FerroError::LockTokenNotFound(token.to_string()))
        }
    }

    /// Synchronously check for an active lock on a path.
    pub fn check_lock_sync(&self, path: &str) -> Option<LockInfo> {
        self.cleanup_expired(path);
        self.locks.get(path).map(|r| r.value().clone())
    }

    /// Synchronously check if a write lock (or inherited infinity lock) blocks the path.
    pub fn check_lock_for_write_sync(&self, path: &str) -> common::error::Result<()> {
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

    /// Return the number of active locks.
    pub fn lock_count(&self) -> usize {
        self.locks.len()
    }

    /// Iterate over all active locks.
    pub fn all_locks_sync(&self) -> dashmap::iter::Iter<'_, String, LockInfo> {
        self.locks.iter()
    }

    /// Remove all expired locks.
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

pub(crate) fn parent_path(path: &str) -> Option<&str> {
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
    fn test_shared_lock_allowed() {
        let mgr = LockManager::new();
        mgr.acquire_lock_sync(
            "/test.txt",
            "user1",
            LockScope::Shared,
            LockDepth::Zero,
            None,
        )
        .unwrap();

        let result = mgr.acquire_lock_sync(
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
            .acquire_lock_sync(
                "/test.txt",
                "user1",
                LockScope::Exclusive,
                LockDepth::Zero,
                None,
            )
            .unwrap();

        let refreshed = mgr
            .refresh_lock_sync(&lock.token.as_str(), Some(120))
            .unwrap();
        assert_eq!(refreshed.timeout_seconds, 120);
        assert_eq!(refreshed.refresh_count, 1);
    }

    #[test]
    fn test_release_nonexistent_lock() {
        let mgr = LockManager::new();
        let result = mgr.release_lock_sync("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_infinity_lock_blocks_child_write() {
        let mgr = LockManager::new();
        mgr.acquire_lock_sync(
            "/parent",
            "user1",
            LockScope::Exclusive,
            LockDepth::Infinity,
            None,
        )
        .unwrap();

        let result = mgr.check_lock_for_write_sync("/parent/child.txt");
        assert!(result.is_err());
    }

    #[test]
    fn test_infinity_lock_blocks_deep_child_write() {
        let mgr = LockManager::new();
        mgr.acquire_lock_sync(
            "/a",
            "user1",
            LockScope::Exclusive,
            LockDepth::Infinity,
            None,
        )
        .unwrap();

        assert!(mgr.check_lock_for_write_sync("/a/b/c/d/file.txt").is_err());
    }

    #[test]
    fn test_zero_lock_does_not_block_child_write() {
        let mgr = LockManager::new();
        mgr.acquire_lock_sync(
            "/parent",
            "user1",
            LockScope::Exclusive,
            LockDepth::Zero,
            None,
        )
        .unwrap();

        assert!(mgr.check_lock_for_write_sync("/parent/child.txt").is_ok());
    }

    #[test]
    fn test_parent_path() {
        assert_eq!(parent_path("/a/b/c"), Some("/a/b"));
        assert_eq!(parent_path("/a"), Some("/"));
        assert_eq!(parent_path("/"), None);
    }

    #[tokio::test]
    async fn test_trait_acquire_and_release() {
        let mgr = LockManager::new();
        let lock = mgr
            .acquire_lock(
                "/test.txt",
                "user1",
                LockScope::Exclusive,
                LockDepth::Zero,
                None,
            )
            .await
            .unwrap();
        assert_eq!(mgr.lock_count(), 1);

        mgr.release_lock(&lock.token.as_str()).await.unwrap();
        assert_eq!(mgr.lock_count(), 0);
    }

    #[tokio::test]
    async fn test_trait_check_lock_for_write() {
        let mgr = LockManager::new();
        mgr.acquire_lock(
            "/parent",
            "user1",
            LockScope::Exclusive,
            LockDepth::Infinity,
            None,
        )
        .await
        .unwrap();

        assert!(mgr.check_lock_for_write("/parent/child.txt").await.is_err());
    }

    #[tokio::test]
    async fn test_trait_all_locks() {
        let mgr = LockManager::new();
        mgr.acquire_lock("/a", "u1", LockScope::Exclusive, LockDepth::Zero, None)
            .await
            .unwrap();
        mgr.acquire_lock("/b", "u2", LockScope::Shared, LockDepth::Zero, None)
            .await
            .unwrap();

        let locks = mgr.all_locks().await;
        assert_eq!(locks.len(), 2);
    }
}
