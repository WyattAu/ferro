use thiserror::Error;

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("session not found")]
    NotFound,

    #[error("session expired")]
    Expired,

    #[error("invalid session token")]
    InvalidToken,

    #[error("max concurrent sessions exceeded ({0})")]
    MaxSessionsExceeded(usize),

    #[error("session already revoked")]
    AlreadyRevoked,

    #[error("token rotation required")]
    TokenRotationRequired,
}
