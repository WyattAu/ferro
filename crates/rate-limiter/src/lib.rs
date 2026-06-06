//! Request rate limiting for Ferro API endpoints.
//!
//! Supports multiple algorithms (token bucket, sliding window, fixed window)
//! and can be used per-user, per-IP, or globally.

mod bucket;
mod error;
mod fixed_window;
mod multi;
mod sliding_window;
pub mod tenant;

pub use bucket::TokenBucketLimiter;
pub use error::RateLimitError;
pub use fixed_window::FixedWindowLimiter;
pub use multi::MultiTierLimiter;
pub use sliding_window::SlidingWindowLimiter;

use std::time::{Duration, Instant};

#[async_trait::async_trait]
pub trait RateLimiter: Send + Sync {
    async fn check(&self, key: &str) -> Result<RateLimitResult, RateLimitError>;
    async fn record(&self, key: &str, cost: u32) -> Result<(), RateLimitError>;
    async fn reset(&self, key: &str);
}

pub struct RateLimitResult {
    pub allowed: bool,
    pub remaining: u32,
    pub reset_at: Instant,
    pub retry_after: Option<Duration>,
}
