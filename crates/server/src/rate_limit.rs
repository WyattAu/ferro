//! Simple in-memory rate limiter using a token bucket approach.
//! Limits requests per client IP within a time window.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::warn;

/// Per-client rate limit state.
struct ClientBucket {
    tokens: u32,
    last_refill: Instant,
}

/// Rate limiter configuration.
#[derive(Clone)]
pub struct RateLimiterConfig {
    /// Maximum requests per window.
    pub max_requests: u32,
    /// Time window duration.
    pub window: Duration,
}

impl Default for RateLimiterConfig {
    fn default() -> Self {
        Self {
            max_requests: 100,
            window: Duration::from_secs(60),
        }
    }
}

/// Simple sliding window rate limiter.
pub struct RateLimiter {
    clients: Arc<RwLock<HashMap<String, ClientBucket>>>,
    config: RateLimiterConfig,
}

impl RateLimiter {
    pub fn new(config: RateLimiterConfig) -> Self {
        Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Check if a request from the given client IP is allowed.
    /// Returns true if allowed, false if rate limited.
    pub async fn check(&self, client_ip: &str) -> bool {
        let mut clients = self.clients.write().await;
        let now = Instant::now();

        let cutoff = now - self.config.window * 2;
        clients.retain(|_, bucket| bucket.last_refill > cutoff);

        let bucket = clients.entry(client_ip.to_string()).or_insert(ClientBucket {
            tokens: self.config.max_requests,
            last_refill: now,
        });

        // Refill tokens based on elapsed time
        let elapsed = now.duration_since(bucket.last_refill);
        let tokens_to_add = (elapsed.as_secs_f64()
            / self.config.window.as_secs_f64()
            * self.config.max_requests as f64) as u32;

        if tokens_to_add > 0 {
            bucket.tokens = (bucket.tokens + tokens_to_add).min(self.config.max_requests);
            bucket.last_refill = now;
        }

        if bucket.tokens > 0 {
            bucket.tokens -= 1;
            true
        } else {
            warn!("Rate limit exceeded for {}", client_ip);
            false
        }
    }

    /// Remove stale entries to prevent memory leaks.
    pub async fn cleanup(&self) {
        let mut clients = self.clients.write().await;
        let cutoff = Instant::now() - self.config.window * 2;
        clients.retain(|_, bucket| bucket.last_refill > cutoff);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter_allows_under_limit() {
        let limiter = RateLimiter::new(RateLimiterConfig {
            max_requests: 5,
            window: Duration::from_secs(60),
        });

        for _ in 0..5 {
            assert!(limiter.check("1.2.3.4").await);
        }
    }

    #[tokio::test]
    async fn test_rate_limiter_blocks_over_limit() {
        let limiter = RateLimiter::new(RateLimiterConfig {
            max_requests: 3,
            window: Duration::from_secs(60),
        });

        assert!(limiter.check("1.2.3.4").await);
        assert!(limiter.check("1.2.3.4").await);
        assert!(limiter.check("1.2.3.4").await);
        assert!(!limiter.check("1.2.3.4").await); // 4th should be blocked
    }

    #[tokio::test]
    async fn test_rate_limiter_independent_clients() {
        let limiter = RateLimiter::new(RateLimiterConfig {
            max_requests: 2,
            window: Duration::from_secs(60),
        });

        // Client A exhausts their limit
        assert!(limiter.check("client-a").await);
        assert!(limiter.check("client-a").await);
        assert!(!limiter.check("client-a").await);

        // Client B should still be allowed
        assert!(limiter.check("client-b").await);
    }

    #[tokio::test]
    async fn test_rate_limiter_cleanup() {
        let limiter = RateLimiter::new(RateLimiterConfig {
            max_requests: 10,
            window: Duration::from_millis(100),
        });

        limiter.check("temp-client").await;
        assert_eq!(limiter.clients.read().await.len(), 1);

        // Wait for window to expire and cleanup
        tokio::time::sleep(Duration::from_millis(250)).await;
        limiter.cleanup().await;
        assert_eq!(limiter.clients.read().await.len(), 0);
    }

    #[tokio::test]
    async fn test_rate_limit_exact_precision() {
        let limiter = RateLimiter::new(RateLimiterConfig {
            max_requests: 5,
            window: Duration::from_secs(60),
        });

        for i in 0..5 {
            assert!(limiter.check("exact").await, "Request {} should be allowed", i + 1);
        }
        assert!(!limiter.check("exact").await, "6th request should be blocked");
    }

    #[tokio::test]
    async fn test_rate_limit_recovery_after_window() {
        let limiter = RateLimiter::new(RateLimiterConfig {
            max_requests: 3,
            window: Duration::from_millis(200),
        });

        for _ in 0..3 {
            assert!(limiter.check("recovery").await);
        }
        assert!(!limiter.check("recovery").await);

        tokio::time::sleep(Duration::from_millis(300)).await;

        assert!(limiter.check("recovery").await, "Should be allowed after window expires");
    }
}
