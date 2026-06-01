use thiserror::Error;

#[derive(Debug, Error)]
pub enum RateLimitError {
    #[error("rate limit quota exceeded")]
    QuotaExceeded,
    #[error("internal rate limiter error: {0}")]
    InternalError(String),
}
