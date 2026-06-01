use std::time::{Duration, Instant};

use async_trait::async_trait;
use dashmap::DashMap;

use crate::{RateLimitError, RateLimitResult, RateLimiter};

struct BucketState {
    tokens: u32,
    #[allow(dead_code)]
    max_tokens: u32,
    last_refill: Instant,
}

pub struct TokenBucketLimiter {
    max_tokens: u32,
    refill_rate: u32,
    refill_interval: Duration,
    buckets: DashMap<String, BucketState>,
}

impl TokenBucketLimiter {
    pub fn new(max_tokens: u32, refill_rate: u32, refill_interval: Duration) -> Self {
        Self {
            max_tokens,
            refill_rate,
            refill_interval,
            buckets: DashMap::new(),
        }
    }

    fn refill_tokens(
        state: &mut BucketState,
        refill_rate: u32,
        refill_interval: Duration,
        max_tokens: u32,
    ) {
        let now = Instant::now();
        let elapsed = now.duration_since(state.last_refill);
        if elapsed >= refill_interval && refill_interval.as_nanos() > 0 {
            let intervals = elapsed.as_nanos() / refill_interval.as_nanos();
            let refill = (intervals as u32).saturating_mul(refill_rate);
            state.tokens = state.tokens.saturating_add(refill).min(max_tokens);
            state.last_refill = now;
        }
    }
}

#[async_trait]
impl RateLimiter for TokenBucketLimiter {
    async fn check(&self, key: &str) -> Result<RateLimitResult, RateLimitError> {
        let mut entry = self
            .buckets
            .entry(key.to_owned())
            .or_insert_with(|| BucketState {
                tokens: self.max_tokens,
                max_tokens: self.max_tokens,
                last_refill: Instant::now(),
            });

        let state = entry.value_mut();
        Self::refill_tokens(
            state,
            self.refill_rate,
            self.refill_interval,
            self.max_tokens,
        );

        if state.tokens > 0 {
            state.tokens -= 1;
            let remaining = state.tokens;
            let window_end = state.last_refill + self.refill_interval;
            Ok(RateLimitResult {
                allowed: true,
                remaining,
                reset_at: window_end,
                retry_after: None,
            })
        } else {
            let next_refill = state.last_refill + self.refill_interval;
            let retry_after = next_refill.saturating_duration_since(Instant::now());
            Ok(RateLimitResult {
                allowed: false,
                remaining: 0,
                reset_at: next_refill,
                retry_after: Some(retry_after),
            })
        }
    }

    async fn record(&self, key: &str, cost: u32) -> Result<(), RateLimitError> {
        let mut entry = self
            .buckets
            .entry(key.to_owned())
            .or_insert_with(|| BucketState {
                tokens: self.max_tokens,
                max_tokens: self.max_tokens,
                last_refill: Instant::now(),
            });

        let state = entry.value_mut();
        Self::refill_tokens(
            state,
            self.refill_rate,
            self.refill_interval,
            self.max_tokens,
        );
        state.tokens = state.tokens.saturating_sub(cost);
        Ok(())
    }

    async fn reset(&self, key: &str) {
        self.buckets.remove(key);
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
        for _ in 0..9 {
            assert!(!limiter.check("burst").await.unwrap().allowed);
        }
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
        use std::sync::Arc;
        let limiter = Arc::new(TokenBucketLimiter::new(100, 0, Duration::from_secs(60)));
        let handles: Vec<_> = (0..10)
            .map(|_| {
                let limiter = Arc::clone(&limiter);
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
