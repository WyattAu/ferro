// TODO: Migrate to shared `ratelimit` crate (https://github.com/WyattAu/ratelimit).
//
// BLOCKED: The shared `ratelimit` crate implements GCRA (Generic Cell Rate
// Algorithm) with `RateLimiter<B: RateLimitBackend>` and `Quota`, while
// ferro's rate limiter uses a trait-based design with four algorithms:
//   - TokenBucketLimiter (token bucket)
//   - SlidingWindowLimiter (sliding window)
//   - FixedWindowLimiter (fixed window)
//   - MultiTierLimiter (multi-tier)
//
// The APIs are fundamentally different:
//   - ferro: async trait with `check()`, `record()`, `reset()` methods
//   - ratelimit: `check()` returns `RateLimitResult` directly, no `record()`/`reset()`
//
// Three crates depend on ferro-rate-limiter:
//   - server-routes (TokenBucketLimiter)
//   - server-state (TokenBucketLimiter)
//   - server (RateLimiter trait, TokenBucketLimiter)
//
// To migrate: rewrite all consumers to use ratelimit::RateLimiter<InMemoryBackend>
// or ratelimit::RateLimiter<RedisBackend>, replace trait objects with concrete types,
// and remove the multi-algorithm abstraction.

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
