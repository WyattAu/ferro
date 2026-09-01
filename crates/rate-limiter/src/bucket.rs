use std::sync::Arc;
use std::time::Duration;

use parking_lot::RwLock;
use throttle_kit::{InMemoryBackend, Quota, RateLimitBackend as _};

use crate::{RateLimitError, RateLimitResult, RateLimiter, convert_result};

/// Token-bucket rate limiter backed by throttle-kit's GCRA implementation.
///
/// Wraps [`throttle_kit::InMemoryBackend`] with a [`Quota`] derived from the
/// constructor parameters so the check/record/reset trait is preserved.
pub struct TokenBucketLimiter {
    quota: Quota,
    backend: RwLock<Arc<InMemoryBackend>>,
}

impl TokenBucketLimiter {
    pub fn new(max_tokens: u32, refill_rate: u32, refill_interval: Duration) -> Self {
        let interval = if refill_rate == 0 || refill_interval.is_zero() {
            Duration::from_secs(365 * 24 * 3600)
        } else {
            refill_interval / refill_rate
        };

        Self {
            quota: Quota::from_parts(interval, max_tokens),
            backend: RwLock::new(Arc::new(InMemoryBackend::new())),
        }
    }
}

#[async_trait::async_trait]
impl RateLimiter for TokenBucketLimiter {
    async fn check(&self, key: &str) -> Result<RateLimitResult, RateLimitError> {
        // Clone the Arc so the RwLock read guard is dropped before the await.
        let backend = self.backend.read().clone();
        let result = backend.check(key, &self.quota).await;
        Ok(convert_result(result))
    }

    async fn record(&self, key: &str, cost: u32) -> Result<(), RateLimitError> {
        let backend = self.backend.read().clone();
        for _ in 0..cost {
            backend.check(key, &self.quota).await;
        }
        Ok(())
    }

    async fn reset(&self, _key: &str) {
        // throttle-kit GCRA does not support per-key reset — recreate the
        // entire backend.  This is acceptable because reset is only called
        // during tests or configuration changes.
        *self.backend.write() = Arc::new(InMemoryBackend::new());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[tokio::test]
    async fn allows_requests_within_limit() {
        let limiter = TokenBucketLimiter::new(5, 1, Duration::from_secs(1));
        for i in 0..5 {
            let result = limiter.check("user1").await.unwrap();
            assert!(result.allowed, "request {} should be allowed", i);
            assert_eq!(result.remaining, 5 - i - 1);
        }
    }

    #[tokio::test]
    async fn denies_requests_over_limit() {
        let limiter = TokenBucketLimiter::new(2, 1, Duration::from_secs(60));
        limiter.check("user1").await.unwrap();
        limiter.check("user1").await.unwrap();
        let result = limiter.check("user1").await.unwrap();
        assert!(!result.allowed);
        assert_eq!(result.remaining, 0);
        assert!(result.retry_after.is_some());
    }

    #[tokio::test]
    async fn refills_over_time() {
        let limiter = TokenBucketLimiter::new(2, 2, Duration::from_millis(100));
        limiter.check("user1").await.unwrap();
        limiter.check("user1").await.unwrap();
        let result = limiter.check("user1").await.unwrap();
        assert!(!result.allowed);
        thread::sleep(Duration::from_millis(150));
        let result = limiter.check("user1").await.unwrap();
        assert!(result.allowed);
        assert!(result.remaining >= 1);
    }

    #[tokio::test]
    async fn burst_handling() {
        let limiter = TokenBucketLimiter::new(10, 1, Duration::from_secs(1));
        for _ in 0..10 {
            assert!(limiter.check("burst").await.unwrap().allowed);
        }
        assert!(!limiter.check("burst").await.unwrap().allowed);
    }

    #[tokio::test]
    async fn separate_keys_independent() {
        let limiter = TokenBucketLimiter::new(1, 1, Duration::from_secs(60));
        assert!(limiter.check("a").await.unwrap().allowed);
        assert!(!limiter.check("a").await.unwrap().allowed);
        assert!(limiter.check("b").await.unwrap().allowed);
    }

    #[tokio::test]
    async fn concurrent_access() {
        use std::sync::Arc as StdArc;
        let limiter = StdArc::new(TokenBucketLimiter::new(100, 0, Duration::from_secs(60)));
        let handles: Vec<_> = (0..10)
            .map(|_| {
                let limiter = StdArc::clone(&limiter);
                tokio::spawn(async move {
                    let mut allowed = 0u32;
                    for _ in 0..10 {
                        if limiter.check("concurrent").await.unwrap().allowed {
                            allowed += 1;
                        }
                    }
                    allowed
                })
            })
            .collect();
        let mut total = 0u32;
        for handle in handles {
            total += handle.await.unwrap();
        }
        assert_eq!(total, 100);
    }

    #[tokio::test]
    async fn reset_clears_bucket() {
        let limiter = TokenBucketLimiter::new(1, 0, Duration::from_secs(60));
        limiter.check("user1").await.unwrap();
        assert!(!limiter.check("user1").await.unwrap().allowed);
        limiter.reset("user1").await;
        assert!(limiter.check("user1").await.unwrap().allowed);
    }

    #[tokio::test]
    async fn zero_capacity() {
        let limiter = TokenBucketLimiter::new(0, 1, Duration::from_secs(1));
        let result = limiter.check("zero").await.unwrap();
        assert!(!result.allowed);
        assert_eq!(result.remaining, 0);
    }

    #[tokio::test]
    async fn record_consumes_tokens() {
        let limiter = TokenBucketLimiter::new(10, 0, Duration::from_secs(60));
        limiter.record("user1", 8).await.unwrap();
        let result = limiter.check("user1").await.unwrap();
        assert!(result.allowed);
        assert_eq!(result.remaining, 1);
        limiter.check("user1").await.unwrap();
        let result2 = limiter.check("user1").await.unwrap();
        assert!(!result2.allowed);
    }

    #[tokio::test]
    async fn zero_refill_rate() {
        let limiter = TokenBucketLimiter::new(3, 0, Duration::from_secs(1));
        limiter.check("user1").await.unwrap();
        limiter.check("user1").await.unwrap();
        limiter.check("user1").await.unwrap();
        thread::sleep(Duration::from_millis(150));
        let result = limiter.check("user1").await.unwrap();
        assert!(!result.allowed);
    }

    #[tokio::test]
    async fn tokens_capped_at_max() {
        let limiter = TokenBucketLimiter::new(3, 10, Duration::from_millis(50));
        limiter.check("user1").await.unwrap();
        thread::sleep(Duration::from_millis(200));
        let result = limiter.check("user1").await.unwrap();
        assert!(result.allowed);
        assert_eq!(result.remaining, 2);
    }
}
