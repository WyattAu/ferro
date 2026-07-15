pub mod bulkhead;
pub mod circuit_breaker;
pub mod retry;

pub use bulkhead::{BulkheadConfig, BulkheadError, BulkheadPool, BulkheadPools, NamedBulkhead};
pub use circuit_breaker::{CircuitBreakerConfig, NamedCircuitBreaker, ResilientCall};
pub use retry::{RetryPolicy, retry_with_backoff};

#[cfg(test)]
mod tests {
    use super::*;
    use ferro_circuit_breaker::CircuitState;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    // ---- Circuit Breaker State Transitions ----

    #[tokio::test]
    async fn test_circuit_breaker_closed_to_open() {
        let rc = ResilientCall::new(CircuitBreakerConfig {
            failure_threshold: 3,
            recovery_timeout: Duration::from_secs(60),
            half_open_max: 1,
        });
        assert_eq!(rc.state(), CircuitState::Closed);

        for _ in 0..3 {
            let _ = rc.call(|| async { Err::<(), &str>("fail") }).await;
        }
        assert_eq!(rc.state(), CircuitState::Open);
    }

    #[tokio::test]
    async fn test_circuit_breaker_open_to_half_open() {
        let rc = ResilientCall::new(CircuitBreakerConfig {
            failure_threshold: 2,
            recovery_timeout: Duration::from_millis(50),
            half_open_max: 1,
        });

        for _ in 0..2 {
            let _ = rc.call(|| async { Err::<(), &str>("fail") }).await;
        }
        assert_eq!(rc.state(), CircuitState::Open);

        tokio::time::sleep(Duration::from_millis(60)).await;
        assert_eq!(rc.state(), CircuitState::HalfOpen);
    }

    #[tokio::test]
    async fn test_circuit_breaker_half_open_to_closed_on_success() {
        let rc = ResilientCall::new(CircuitBreakerConfig {
            failure_threshold: 2,
            recovery_timeout: Duration::from_millis(50),
            half_open_max: 1,
        });

        for _ in 0..2 {
            let _ = rc.call(|| async { Err::<(), &str>("fail") }).await;
        }
        tokio::time::sleep(Duration::from_millis(60)).await;
        assert_eq!(rc.state(), CircuitState::HalfOpen);

        let result = rc.call(|| async { Ok::<(), &str>(()) }).await;
        assert!(result.is_ok());
        assert_eq!(rc.state(), CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_circuit_breaker_half_open_to_open_on_failure() {
        let rc = ResilientCall::new(CircuitBreakerConfig {
            failure_threshold: 2,
            recovery_timeout: Duration::from_millis(50),
            half_open_max: 1,
        });

        for _ in 0..2 {
            let _ = rc.call(|| async { Err::<(), &str>("fail") }).await;
        }
        tokio::time::sleep(Duration::from_millis(60)).await;
        assert_eq!(rc.state(), CircuitState::HalfOpen);

        let _ = rc.call(|| async { Err::<(), &str>("still broken") }).await;
        assert_eq!(rc.state(), CircuitState::Open);
    }

    #[tokio::test]
    async fn test_named_circuit_breaker_rejects_when_open() {
        let ncb = NamedCircuitBreaker::new(
            "test",
            CircuitBreakerConfig {
                failure_threshold: 1,
                recovery_timeout: Duration::from_secs(60),
                half_open_max: 1,
            },
        );

        let _ = ncb.call(|| async { Err::<(), &str>("fail") }).await;
        assert_eq!(ncb.state(), CircuitState::Open);

        let result = ncb.call(|| async { Ok::<(), &str>(()) }).await;
        assert!(result.is_err());
    }

    // ---- Retry with Backoff ----

    #[tokio::test]
    async fn test_retry_respects_max_retries() {
        let policy = RetryPolicy {
            max_retries: 4,
            base_delay: Duration::from_millis(1),
            max_delay: Duration::from_millis(10),
            jitter: 0.0,
        };

        let count = Arc::new(AtomicU32::new(0));
        let c = count.clone();
        let result: Result<(), &str> = retry_with_backoff(&policy, || {
            let c = c.clone();
            async move {
                c.fetch_add(1, Ordering::Relaxed);
                Err("always fail")
            }
        })
        .await;

        assert!(result.is_err());
        // 1 initial + 4 retries = 5 total
        assert_eq!(count.load(Ordering::Relaxed), 5);
    }

    #[tokio::test]
    async fn test_retry_delay_increases_exponentially() {
        let policy = RetryPolicy {
            max_retries: 3,
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            jitter: 0.0,
        };

        let d0 = retry::delay_for_attempt(&policy, 0);
        let d1 = retry::delay_for_attempt(&policy, 1);
        let d2 = retry::delay_for_attempt(&policy, 2);

        assert_eq!(d0, Duration::from_millis(100));
        assert_eq!(d1, Duration::from_millis(200));
        assert_eq!(d2, Duration::from_millis(400));
    }

    #[tokio::test]
    async fn test_retry_no_policy_fails_immediately() {
        let policy = RetryPolicy::none();
        let count = Arc::new(AtomicU32::new(0));
        let c = count.clone();

        let _ = retry_with_backoff(&policy, || {
            let c = c.clone();
            async move {
                c.fetch_add(1, Ordering::Relaxed);
                Err::<(), &str>("fail")
            }
        })
        .await;

        assert_eq!(count.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn test_retry_succeeds_after_partial_failures() {
        let policy = RetryPolicy {
            max_retries: 3,
            base_delay: Duration::from_millis(1),
            max_delay: Duration::from_millis(5),
            jitter: 0.0,
        };

        let count = Arc::new(AtomicU32::new(0));
        let c = count.clone();
        let result = retry_with_backoff(&policy, || {
            let c = c.clone();
            async move {
                let n = c.fetch_add(1, Ordering::Relaxed) + 1;
                if n < 3 { Err("not yet") } else { Ok(99) }
            }
        })
        .await;

        assert_eq!(result.unwrap(), 99);
        assert_eq!(count.load(Ordering::Relaxed), 3);
    }

    // ---- Bulkhead Pool ----

    #[tokio::test]
    async fn test_bulkhead_acquire_timeout() {
        let pool = BulkheadPool::new("test", 1);
        let _permit = pool.try_acquire().unwrap();

        let result = pool.acquire(Duration::from_millis(50)).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("timed out"));
    }

    #[tokio::test]
    async fn test_bulkhead_concurrent_access() {
        let pool = BulkheadPool::new("test", 3);

        let p1 = pool.try_acquire();
        let p2 = pool.try_acquire();
        let p3 = pool.try_acquire();
        assert!(p1.is_some());
        assert!(p2.is_some());
        assert!(p3.is_some());

        assert_eq!(pool.available(), 0);
        assert!(pool.try_acquire().is_none());

        drop(p2);
        assert_eq!(pool.available(), 1);
        assert!(pool.try_acquire().is_some());
    }

    #[tokio::test]
    async fn test_bulkhead_pools_different_sizes() {
        let config = BulkheadConfig {
            storage_pool_size: 10,
            auth_pool_size: 5,
            db_pool_size: 8,
            cache_pool_size: 3,
            acquire_timeout: Duration::from_secs(1),
        };
        let pools = BulkheadPools::new(config);
        assert_eq!(pools.storage.max_concurrent(), 10);
        assert_eq!(pools.auth.max_concurrent(), 5);
        assert_eq!(pools.db.max_concurrent(), 8);
        assert_eq!(pools.cache.max_concurrent(), 3);
        assert_eq!(pools.acquire_timeout(), Duration::from_secs(1));
    }
}
