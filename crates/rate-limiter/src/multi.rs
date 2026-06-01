use std::time::Duration;

use async_trait::async_trait;
use dashmap::DashMap;

use crate::{RateLimitError, RateLimitResult, RateLimiter};

struct Tier {
    #[allow(dead_code)]
    name: String,
    limiter: Box<dyn RateLimiter>,
    priority: u32,
}

pub struct MultiTierLimiter {
    tiers: DashMap<String, Tier>,
    priority_order: parking_lot::RwLock<Vec<String>>,
}

impl MultiTierLimiter {
    pub fn new() -> Self {
        Self {
            tiers: DashMap::new(),
            priority_order: parking_lot::RwLock::new(Vec::new()),
        }
    }

    pub fn add_tier(&self, name: &str, limiter: Box<dyn RateLimiter>) {
        self.add_tier_with_priority(name, limiter, 0);
    }

    pub fn add_tier_with_priority(&self, name: &str, limiter: Box<dyn RateLimiter>, priority: u32) {
        self.tiers.insert(
            name.to_owned(),
            Tier {
                name: name.to_owned(),
                limiter,
                priority,
            },
        );
        let mut order = self.priority_order.write();
        order.push(name.to_owned());
        order.sort_by_key(|k| self.tiers.get(k).map(|t| t.priority).unwrap_or(u32::MAX));
    }
}

impl Default for MultiTierLimiter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl RateLimiter for MultiTierLimiter {
    async fn check(&self, key: &str) -> Result<RateLimitResult, RateLimitError> {
        let order: Vec<String> = self.priority_order.read().clone();
        let mut worst_remaining = u32::MAX;
        let mut latest_reset = std::time::Instant::now();
        let mut max_retry_after: Option<Duration> = None;

        for name in &order {
            let Some(tier) = self.tiers.get(name) else {
                continue;
            };
            let result = tier.limiter.check(key).await?;
            if !result.allowed {
                let retry = result.retry_after.unwrap_or(Duration::ZERO);
                match &mut max_retry_after {
                    Some(current) => {
                        if retry > *current {
                            *current = retry;
                        }
                    }
                    None => max_retry_after = Some(retry),
                }
                return Ok(RateLimitResult {
                    allowed: false,
                    remaining: result.remaining,
                    reset_at: result.reset_at,
                    retry_after: max_retry_after,
                });
            }
            worst_remaining = worst_remaining.min(result.remaining);
            if result.reset_at > latest_reset {
                latest_reset = result.reset_at;
            }
        }

        Ok(RateLimitResult {
            allowed: true,
            remaining: worst_remaining,
            reset_at: latest_reset,
            retry_after: None,
        })
    }

    async fn record(&self, key: &str, cost: u32) -> Result<(), RateLimitError> {
        let order: Vec<String> = self.priority_order.read().clone();
        for name in &order {
            let Some(tier) = self.tiers.get(name) else {
                continue;
            };
            tier.limiter.record(key, cost).await?;
        }
        Ok(())
    }

    async fn reset(&self, key: &str) {
        let order: Vec<String> = self.priority_order.read().clone();
        for name in &order {
            let Some(tier) = self.tiers.get(name) else {
                continue;
            };
            tier.limiter.reset(key).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FixedWindowLimiter, TokenBucketLimiter};
    use std::time::Duration;

    #[tokio::test]
    async fn all_allow() {
        let multi = MultiTierLimiter::new();
        multi.add_tier(
            "token",
            Box::new(TokenBucketLimiter::new(10, 0, Duration::from_secs(60))),
        );
        multi.add_tier(
            "fixed",
            Box::new(FixedWindowLimiter::new(10, Duration::from_secs(60))),
        );

        let result = multi.check("user1").await.unwrap();
        assert!(result.allowed);
        assert!(result.remaining >= 1);
    }

    #[tokio::test]
    async fn one_denies() {
        let multi = MultiTierLimiter::new();
        multi.add_tier(
            "token",
            Box::new(TokenBucketLimiter::new(10, 0, Duration::from_secs(60))),
        );
        multi.add_tier(
            "fixed",
            Box::new(FixedWindowLimiter::new(1, Duration::from_secs(60))),
        );

        multi.check("user1").await.unwrap();
        let result = multi.check("user1").await.unwrap();
        assert!(!result.allowed);
    }

    #[tokio::test]
    async fn priority_ordering() {
        let multi = MultiTierLimiter::new();
        multi.add_tier_with_priority(
            "permissive",
            Box::new(TokenBucketLimiter::new(100, 0, Duration::from_secs(60))),
            1,
        );
        multi.add_tier_with_priority(
            "restrictive",
            Box::new(TokenBucketLimiter::new(1, 0, Duration::from_secs(60))),
            0,
        );

        multi.check("user1").await.unwrap();
        let result = multi.check("user1").await.unwrap();
        assert!(!result.allowed);
    }

    #[tokio::test]
    async fn record_propagates_to_all() {
        let multi = MultiTierLimiter::new();
        multi.add_tier(
            "token",
            Box::new(TokenBucketLimiter::new(10, 0, Duration::from_secs(60))),
        );
        multi.add_tier(
            "fixed",
            Box::new(FixedWindowLimiter::new(10, Duration::from_secs(60))),
        );

        multi.record("user1", 5).await.unwrap();
        let result = multi.check("user1").await.unwrap();
        assert!(result.allowed);
        assert!(result.remaining <= 4);
    }

    #[tokio::test]
    async fn reset_clears_all_tiers() {
        let multi = MultiTierLimiter::new();
        multi.add_tier(
            "token",
            Box::new(TokenBucketLimiter::new(1, 0, Duration::from_secs(60))),
        );
        multi.add_tier(
            "fixed",
            Box::new(FixedWindowLimiter::new(1, Duration::from_secs(60))),
        );

        multi.check("user1").await.unwrap();
        assert!(!multi.check("user1").await.unwrap().allowed);
        multi.reset("user1").await;
        assert!(multi.check("user1").await.unwrap().allowed);
    }

    #[tokio::test]
    async fn empty_tiers_allows() {
        let multi = MultiTierLimiter::new();
        let result = multi.check("user1").await.unwrap();
        assert!(result.allowed);
        assert_eq!(result.remaining, u32::MAX);
    }
}
