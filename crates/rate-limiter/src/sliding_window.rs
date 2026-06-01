use std::time::{Duration, Instant};

use async_trait::async_trait;
use dashmap::DashMap;

use crate::{RateLimitError, RateLimitResult, RateLimiter};

pub struct SlidingWindowLimiter {
    max_requests: u32,
    window: Duration,
    timestamps: DashMap<String, Vec<Instant>>,
}

impl SlidingWindowLimiter {
    pub fn new(max_requests: u32, window: Duration) -> Self {
        Self {
            max_requests,
            window,
            timestamps: DashMap::new(),
        }
    }

    fn prune_old(timestamps: &mut Vec<Instant>, window: Duration) {
        let cutoff = Instant::now() - window;
        timestamps.retain(|&t| t > cutoff);
    }
}

#[async_trait]
impl RateLimiter for SlidingWindowLimiter {
    async fn check(&self, key: &str) -> Result<RateLimitResult, RateLimitError> {
        let mut entry = self.timestamps.entry(key.to_owned()).or_default();
        let timestamps = entry.value_mut();

        Self::prune_old(timestamps, self.window);

        if timestamps.len() < self.max_requests as usize {
            timestamps.push(Instant::now());
            let remaining = (self.max_requests as usize).saturating_sub(timestamps.len());
            let window_end = timestamps[0] + self.window;
            Ok(RateLimitResult {
                allowed: true,
                remaining: remaining as u32,
                reset_at: window_end,
                retry_after: None,
            })
        } else {
            let now = Instant::now();
            let retry_after = if let Some(&oldest) = timestamps.first() {
                (oldest + self.window - now).max(Duration::ZERO)
            } else {
                self.window
            };
            Ok(RateLimitResult {
                allowed: false,
                remaining: 0,
                reset_at: now + self.window,
                retry_after: Some(retry_after),
            })
        }
    }

    async fn record(&self, key: &str, cost: u32) -> Result<(), RateLimitError> {
        let mut entry = self.timestamps.entry(key.to_owned()).or_default();
        let timestamps = entry.value_mut();
        Self::prune_old(timestamps, self.window);
        let now = Instant::now();
        for _ in 0..cost {
            timestamps.push(now);
        }
        Ok(())
    }

    async fn reset(&self, key: &str) {
        self.timestamps.remove(key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[tokio::test]
    async fn allows_within_limit() {
        let limiter = SlidingWindowLimiter::new(3, Duration::from_secs(1));
        for i in 0..3 {
            let result = limiter.check("user1").await.unwrap();
            assert!(result.allowed, "request {} should be allowed", i);
            assert_eq!(result.remaining, 2 - i);
        }
    }

    #[tokio::test]
    async fn denies_over_limit() {
        let limiter = SlidingWindowLimiter::new(2, Duration::from_secs(60));
        limiter.check("user1").await.unwrap();
        limiter.check("user1").await.unwrap();
        let result = limiter.check("user1").await.unwrap();
        assert!(!result.allowed);
        assert!(result.retry_after.is_some());
    }

    #[tokio::test]
    async fn window_expires() {
        let limiter = SlidingWindowLimiter::new(1, Duration::from_millis(100));
        limiter.check("user1").await.unwrap();
        thread::sleep(Duration::from_millis(150));
        let result = limiter.check("user1").await.unwrap();
        assert!(result.allowed);
    }

    #[tokio::test]
    async fn multiple_keys_independent() {
        let limiter = SlidingWindowLimiter::new(1, Duration::from_secs(60));
        assert!(limiter.check("a").await.unwrap().allowed);
        assert!(!limiter.check("a").await.unwrap().allowed);
        assert!(limiter.check("b").await.unwrap().allowed);
    }

    #[tokio::test]
    async fn reset_clears_state() {
        let limiter = SlidingWindowLimiter::new(1, Duration::from_secs(60));
        limiter.check("user1").await.unwrap();
        assert!(!limiter.check("user1").await.unwrap().allowed);
        limiter.reset("user1").await;
        assert!(limiter.check("user1").await.unwrap().allowed);
    }

    #[tokio::test]
    async fn zero_capacity() {
        let limiter = SlidingWindowLimiter::new(0, Duration::from_secs(1));
        let result = limiter.check("zero").await.unwrap();
        assert!(!result.allowed);
    }
}
