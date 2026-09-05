//! Request rate limiting for Ferro API endpoints.
//!
//! Delegates to the shared `throttle-kit` crate (GCRA algorithm with
//! in-memory backend) for token bucket, sliding window, and fixed window
//! algorithms. The `MultiTierLimiter` composes multiple limiters for
//! layered rate limiting.

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

/// Convert a throttle-kit [`throttle_kit::RateLimitResult`] into a ferro
/// [`RateLimitResult`].
fn convert_result(r: throttle_kit::RateLimitResult) -> RateLimitResult {
    RateLimitResult {
        allowed: r.allowed,
        remaining: r.remaining as u32,
        reset_at: r.reset_at,
        retry_after: r.retry_after,
    }
}

/// Client-IP identity resolution (throttle-kit 0.4 semantics).
///
/// Since throttle-kit 0.4, `X-Forwarded-For` is ignored unless trusted
/// proxies are configured; callers key by the direct peer address by
/// default. Re-exported so the server can build a
/// [`ClientIpConfig`] from `FERRO_RATE_LIMIT_TRUSTED_PROXIES` /
/// `FERRO_RATE_LIMIT_TRUSTED_HOPS`.
///
/// CAUTION: throttle-kit's `client_ip::peer_ip_from_extensions` is typed
/// on axum 0.8 and never matches axum 0.7's `ConnectInfo` extension —
/// callers here extract the peer with their own axum version and pass
/// `Option<IpAddr>` to the pure `resolve_client_identity`.
pub use throttle_kit::client_ip;
pub use throttle_kit::{
    ClientIpConfig, ClientIpError, ClientIpSource, IpNet, MissingClientIdentity, MissingClientPolicy, ResolvedClient,
};
