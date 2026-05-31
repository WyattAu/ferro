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

    #[error("WORM-protected resource cannot be modified or deleted: {0}")]
    WormProtected(String),
}

impl FerroError {
    /// Map this error to an HTTP status code.
    pub fn status_code(&self) -> u16 {
        match self {
            Self::NotFound(_) => 404,
            Self::AlreadyExists(_) => 409,
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
            Self::WormProtected(_) => 403,
        }
    }
}

/// Result alias using [`FerroError`].
pub type Result<T> = std::result::Result<T, FerroError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_code_mapping() {
        assert_eq!(FerroError::NotFound("x".into()).status_code(), 404);
        assert_eq!(FerroError::AlreadyExists("x".into()).status_code(), 409);
        assert_eq!(FerroError::PermissionDenied("x".into()).status_code(), 403);
        assert_eq!(FerroError::InvalidArgument("x".into()).status_code(), 400);
        assert_eq!(FerroError::Internal("x".into()).status_code(), 500);
        assert_eq!(FerroError::LockConflict("x".into()).status_code(), 423);
        assert_eq!(FerroError::LockTokenNotFound("x".into()).status_code(), 409);
        assert_eq!(
            FerroError::PreconditionFailed("x".into()).status_code(),
            412
        );
        assert_eq!(
            FerroError::UnsupportedMediaType("x".into()).status_code(),
            415
        );
        assert_eq!(FerroError::Timeout.status_code(), 504);
        assert_eq!(FerroError::StorageBackend("x".into()).status_code(), 502);
        assert_eq!(FerroError::Unauthorized.status_code(), 401);
        assert_eq!(FerroError::XmlError("x".into()).status_code(), 400);
        assert_eq!(FerroError::WormProtected("x".into()).status_code(), 403);
    }

    #[test]
    fn test_error_display() {
        let err = FerroError::NotFound("file.txt".into());
        let msg = format!("{err}");
        assert!(msg.contains("file.txt"));
        assert!(msg.contains("Not found"));
    }

    #[test]
    fn test_result_type_alias() {
        fn returns_ok() -> Result<String> {
            Ok("hello".to_string())
        }
        fn returns_err() -> Result<String> {
            Err(FerroError::Internal("oops".into()))
        }
        assert!(returns_ok().is_ok());
        assert!(returns_err().is_err());
    }
}
