use std::sync::Arc;
use std::time::Duration;

use parking_lot::RwLock;
use throttle_kit::{InMemoryBackend, Quota, RateLimitBackend as _};

use crate::{RateLimitError, RateLimitResult, RateLimiter, convert_result};

/// Fixed-window rate limiter backed by throttle-kit's GCRA implementation.
///
/// Maps `max_requests` per `window` to a throttle-kit [`Quota`] where each
/// token represents one request.
pub struct FixedWindowLimiter {
    quota: Quota,
    backend: RwLock<Arc<InMemoryBackend>>,
}

impl FixedWindowLimiter {
    pub fn new(max_requests: u32, window: Duration) -> Self {
        let interval = if max_requests == 0 || window.is_zero() {
            Duration::from_secs(365 * 24 * 3600)
        } else {
            window / max_requests
        };

        Self {
            quota: Quota::from_parts(interval, max_requests),
            backend: RwLock::new(Arc::new(InMemoryBackend::new())),
        }
    }
}

#[async_trait::async_trait]
impl RateLimiter for FixedWindowLimiter {
    async fn check(&self, key: &str) -> Result<RateLimitResult, RateLimitError> {
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
        *self.backend.write() = Arc::new(InMemoryBackend::new());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[tokio::test]
    async fn allows_within_limit() {
        let limiter = FixedWindowLimiter::new(5, Duration::from_secs(1));
        for i in 0..5 {
            let result = limiter.check("user1").await.unwrap();
            assert!(result.allowed, "request {} should be allowed", i);
        }
    }

    #[tokio::test]
    async fn denies_over_limit() {
        let limiter = FixedWindowLimiter::new(2, Duration::from_secs(60));
        limiter.check("user1").await.unwrap();
        limiter.check("user1").await.unwrap();
        let result = limiter.check("user1").await.unwrap();
        assert!(!result.allowed);
        assert_eq!(result.remaining, 0);
    }

    #[tokio::test]
    async fn window_resets() {
        let limiter = FixedWindowLimiter::new(1, Duration::from_millis(100));
        limiter.check("user1").await.unwrap();
        thread::sleep(Duration::from_millis(150));
        let result = limiter.check("user1").await.unwrap();
        assert!(result.allowed);
    }

    #[tokio::test]
    async fn boundary_timing() {
        let limiter = FixedWindowLimiter::new(100, Duration::from_millis(50));
        for i in 0..100 {
            assert!(
                limiter.check("boundary").await.unwrap().allowed,
                "request {} allowed",
                i
            );
        }
        assert!(!limiter.check("boundary").await.unwrap().allowed);
        thread::sleep(Duration::from_millis(80));
        assert!(limiter.check("boundary").await.unwrap().allowed);
    }

    #[tokio::test]
    async fn reset_clears_state() {
        let limiter = FixedWindowLimiter::new(1, Duration::from_secs(60));
        limiter.check("user1").await.unwrap();
        assert!(!limiter.check("user1").await.unwrap().allowed);
        limiter.reset("user1").await;
        assert!(limiter.check("user1").await.unwrap().allowed);
    }

    #[tokio::test]
    async fn zero_capacity() {
        let limiter = FixedWindowLimiter::new(0, Duration::from_secs(1));
        let result = limiter.check("zero").await.unwrap();
        assert!(!result.allowed);
    }
}
