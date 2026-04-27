use thiserror::Error;

/// Top-level error type for the Ferro application.
#[derive(Error, Debug)]
pub enum FerroError {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Already exists: {0}")]
    AlreadyExists(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Lock conflict: {0}")]
    LockConflict(String),

    #[error("Lock token not found: {0}")]
    LockTokenNotFound(String),

    #[error("Precondition failed: {0}")]
    PreconditionFailed(String),

    #[error("Unsupported media type: {0}")]
    UnsupportedMediaType(String),

    #[error("Request timeout")]
    Timeout,

    #[error("Storage backend error: {0}")]
    StorageBackend(String),

    #[error("Authentication required")]
    Unauthorized,

    #[error("XML processing error: {0}")]
    XmlError(String),
}

impl FerroError {
    /// Map this error to an HTTP status code.
    pub fn status_code(&self) -> u16 {
        match self {
            Self::NotFound(_) => 404,
            Self::AlreadyExists(_) => 405,
            Self::PermissionDenied(_) => 403,
            Self::InvalidArgument(_) => 400,
            Self::Internal(_) => 500,
            Self::LockConflict(_) => 423,
            Self::LockTokenNotFound(_) => 409,
            Self::PreconditionFailed(_) => 412,
            Self::UnsupportedMediaType(_) => 415,
            Self::Timeout => 504,
            Self::StorageBackend(_) => 502,
            Self::Unauthorized => 401,
            Self::XmlError(_) => 400,
        }
    }
}

/// Result alias using [`FerroError`].
pub type Result<T> = std::result::Result<T, FerroError>;
