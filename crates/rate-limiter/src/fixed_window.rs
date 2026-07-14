use std::time::{Duration, Instant};

use async_trait::async_trait;
use dashmap::DashMap;

use crate::{RateLimitError, RateLimitResult, RateLimiter};

struct WindowState {
    count: u32,
    window_start: Instant,
}

pub struct FixedWindowLimiter {
    max_requests: u32,
    window: Duration,
    states: DashMap<String, WindowState>,
}

impl FixedWindowLimiter {
    pub fn new(max_requests: u32, window: Duration) -> Self {
        Self {
            max_requests,
            window,
            states: DashMap::new(),
        }
    }
}

#[async_trait]
impl RateLimiter for FixedWindowLimiter {
    async fn check(&self, key: &str) -> Result<RateLimitResult, RateLimitError> {
        let now = Instant::now();
        let mut entry = self.states.entry(key.to_owned()).or_insert_with(|| WindowState {
            count: 0,
            window_start: now,
        });

        let state = entry.value_mut();
        if now.duration_since(state.window_start) >= self.window {
            state.count = 0;
            state.window_start = now;
        }

        if state.count < self.max_requests {
            state.count += 1;
            let remaining = self.max_requests - state.count;
            let reset_at = state.window_start + self.window;
            Ok(RateLimitResult {
                allowed: true,
                remaining,
                reset_at,
                retry_after: None,
            })
        } else {
            let reset_at = state.window_start + self.window;
            let retry_after = reset_at.saturating_duration_since(now);
            Ok(RateLimitResult {
                allowed: false,
                remaining: 0,
                reset_at,
                retry_after: Some(retry_after),
            })
        }
    }

    async fn record(&self, key: &str, cost: u32) -> Result<(), RateLimitError> {
        let now = Instant::now();
        let mut entry = self.states.entry(key.to_owned()).or_insert_with(|| WindowState {
            count: 0,
            window_start: now,
        });

        let state = entry.value_mut();
        if now.duration_since(state.window_start) >= self.window {
            state.count = 0;
            state.window_start = now;
        }
        state.count = state.count.saturating_add(cost);
        Ok(())
    }

    async fn reset(&self, key: &str) {
        self.states.remove(key);
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
