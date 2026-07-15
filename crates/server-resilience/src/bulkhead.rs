use std::time::Duration;

use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tracing::warn;

/// Error when a bulkhead pool is at capacity.
#[derive(Debug)]
pub struct BulkheadError {
    pub pool_name: String,
    pub message: String,
}

impl std::fmt::Display for BulkheadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Bulkhead pool '{}' is at capacity: {}", self.pool_name, self.message)
    }
}

impl std::error::Error for BulkheadError {}

/// A bulkhead pool that limits concurrent access to a subsystem.
///
/// Uses a semaphore to enforce the maximum number of concurrent operations.
/// Callers await acquisition of a permit before proceeding.
#[derive(Clone)]
pub struct BulkheadPool {
    name: String,
    semaphore: std::sync::Arc<Semaphore>,
    max_concurrent: usize,
}

impl BulkheadPool {
    /// Create a new bulkhead pool with the given capacity.
    #[must_use]
    pub fn new(name: impl Into<String>, max_concurrent: usize) -> Self {
        Self {
            name: name.into(),
            semaphore: std::sync::Arc::new(Semaphore::new(max_concurrent)),
            max_concurrent,
        }
    }

    /// Get the name of this pool.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the maximum concurrent permits.
    pub fn max_concurrent(&self) -> usize {
        self.max_concurrent
    }

    /// Get the number of available permits.
    pub fn available(&self) -> usize {
        self.semaphore.available_permits()
    }

    /// Try to acquire a permit immediately. Returns `None` if at capacity.
    pub fn try_acquire(&self) -> Option<OwnedSemaphorePermit> {
        self.semaphore.clone().try_acquire_owned().ok()
    }

    /// Acquire a permit, waiting up to `timeout`.
    ///
    /// Returns `BulkheadError` if the timeout expires.
    pub async fn acquire(&self, timeout: Duration) -> Result<OwnedSemaphorePermit, BulkheadError> {
        match tokio::time::timeout(timeout, self.semaphore.clone().acquire_owned()).await {
            Ok(Ok(permit)) => Ok(permit),
            Ok(Err(_)) => Err(BulkheadError {
                pool_name: self.name.clone(),
                message: "semaphore closed".to_string(),
            }),
            Err(_) => {
                warn!("Bulkhead pool '{}' timed out after {:?}", self.name, timeout);
                Err(BulkheadError {
                    pool_name: self.name.clone(),
                    message: format!("timed out after {:?}", timeout),
                })
            }
        }
    }
}

/// A named bulkhead pool for a specific subsystem.
#[derive(Clone)]
pub struct NamedBulkhead {
    inner: BulkheadPool,
}

impl NamedBulkhead {
    /// Create a new named bulkhead.
    #[must_use]
    pub fn new(name: impl Into<String>, max_concurrent: usize) -> Self {
        Self {
            inner: BulkheadPool::new(name, max_concurrent),
        }
    }

    /// Get the name.
    pub fn name(&self) -> &str {
        self.inner.name()
    }

    /// Get the number of available permits.
    pub fn available(&self) -> usize {
        self.inner.available()
    }

    /// Get the maximum concurrent permits.
    pub fn max_concurrent(&self) -> usize {
        self.inner.max_concurrent()
    }

    /// Try to acquire a permit immediately.
    pub fn try_acquire(&self) -> Option<OwnedSemaphorePermit> {
        self.inner.try_acquire()
    }

    /// Acquire a permit, waiting up to `timeout`.
    pub async fn acquire(&self, timeout: Duration) -> Result<OwnedSemaphorePermit, BulkheadError> {
        self.inner.acquire(timeout).await
    }
}

/// Configuration for a set of bulkhead pools.
#[derive(Debug, Clone)]
pub struct BulkheadConfig {
    /// Maximum concurrent operations for storage operations.
    pub storage_pool_size: usize,
    /// Maximum concurrent operations for auth/OIDC operations.
    pub auth_pool_size: usize,
    /// Maximum concurrent operations for database operations.
    pub db_pool_size: usize,
    /// Maximum concurrent operations for cache operations.
    pub cache_pool_size: usize,
    /// Timeout for acquiring a permit from any pool.
    pub acquire_timeout: Duration,
}

impl Default for BulkheadConfig {
    fn default() -> Self {
        Self {
            storage_pool_size: 50,
            auth_pool_size: 20,
            db_pool_size: 30,
            cache_pool_size: 40,
            acquire_timeout: Duration::from_secs(5),
        }
    }
}

/// A collection of bulkhead pools for different subsystems.
#[derive(Clone)]
pub struct BulkheadPools {
    pub storage: NamedBulkhead,
    pub auth: NamedBulkhead,
    pub db: NamedBulkhead,
    pub cache: NamedBulkhead,
    acquire_timeout: Duration,
}

impl BulkheadPools {
    /// Create a new set of bulkhead pools from configuration.
    #[must_use]
    pub fn new(config: BulkheadConfig) -> Self {
        Self {
            storage: NamedBulkhead::new("storage", config.storage_pool_size),
            auth: NamedBulkhead::new("auth", config.auth_pool_size),
            db: NamedBulkhead::new("db", config.db_pool_size),
            cache: NamedBulkhead::new("cache", config.cache_pool_size),
            acquire_timeout: config.acquire_timeout,
        }
    }

    /// Get the acquisition timeout.
    pub fn acquire_timeout(&self) -> Duration {
        self.acquire_timeout
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_bulkhead_acquire_and_release() {
        let pool = BulkheadPool::new("test", 2);
        assert_eq!(pool.available(), 2);

        let p1 = pool.try_acquire();
        assert!(p1.is_some());
        assert_eq!(pool.available(), 1);

        let p2 = pool.try_acquire();
        assert!(p2.is_some());
        assert_eq!(pool.available(), 0);

        let p3 = pool.try_acquire();
        assert!(p3.is_none());

        drop(p1);
        assert_eq!(pool.available(), 1);
    }

    #[tokio::test]
    async fn test_bulkhead_acquire_with_timeout() {
        let pool = BulkheadPool::new("test", 1);
        let p1 = pool.try_acquire().unwrap();

        let result = pool.acquire(Duration::from_millis(50)).await;
        assert!(result.is_err());

        drop(p1);
        let result = pool.acquire(Duration::from_millis(50)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_bulkhead_pools_creation() {
        let pools = BulkheadPools::new(BulkheadConfig::default());
        assert_eq!(pools.storage.name(), "storage");
        assert_eq!(pools.auth.name(), "auth");
        assert_eq!(pools.db.name(), "db");
        assert_eq!(pools.cache.name(), "cache");
    }

    #[test]
    fn test_bulkhead_error_display() {
        let err = BulkheadError {
            pool_name: "test".to_string(),
            message: "timed out".to_string(),
        };
        assert!(format!("{err}").contains("test"));
        assert!(format!("{err}").contains("timed out"));
    }
}
