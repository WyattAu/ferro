use thiserror::Error;

#[derive(Debug, Error)]
pub enum WebhookError {
    #[error("webhook not found: {0}")]
    NotFound(String),
    #[error("webhook already exists: {0}")]
    AlreadyExists(String),
    #[error("webhook is disabled: {0}")]
    Disabled(String),
    #[error("invalid webhook URL: {0}")]
    InvalidUrl(String),
    #[error("signing error: {0}")]
    SigningError(String),
    #[error("dispatch error: {0}")]
    DispatchError(String),
    #[error("invalid signature")]
    InvalidSignature,
    #[error("serialization error: {0}")]
    Serialization(String),
}
