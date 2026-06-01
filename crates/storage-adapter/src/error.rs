use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageAdapterError {
    #[error("object not found: {0}")]
    NotFound(String),
    #[error("permission denied: {0}")]
    PermissionDenied(String),
    #[error("quota exceeded: {0}")]
    QuotaExceeded(String),
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("backend unavailable: {0}")]
    BackendUnavailable(String),
    #[error("invalid path: {0}")]
    InvalidPath(String),
    #[error("not implemented: {0}")]
    NotImplemented(String),
    #[error("mount failed: {0}")]
    MountFailed(String),
    #[error("connection failed: {0}")]
    ConnectionFailed(String),
}
