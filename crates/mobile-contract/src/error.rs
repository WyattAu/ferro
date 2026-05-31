use std::fmt;

#[derive(Debug, Clone)]
pub enum MobileApiError {
    Unauthorized,
    Forbidden,
    NotFound { resource: String },
    Conflict { reason: String },
    QuotaExceeded,
    ServerError { code: u16, message: String },
    NetworkError { reason: String },
    SyncConflict { path: String },
}

impl fmt::Display for MobileApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MobileApiError::Unauthorized => write!(f, "unauthorized"),
            MobileApiError::Forbidden => write!(f, "forbidden"),
            MobileApiError::NotFound { resource } => write!(f, "not found: {resource}"),
            MobileApiError::Conflict { reason } => write!(f, "conflict: {reason}"),
            MobileApiError::QuotaExceeded => write!(f, "storage quota exceeded"),
            MobileApiError::ServerError { code, message } => {
                write!(f, "server error ({code}): {message}")
            }
            MobileApiError::NetworkError { reason } => write!(f, "network error: {reason}"),
            MobileApiError::SyncConflict { path } => write!(f, "sync conflict: {path}"),
        }
    }
}

impl std::error::Error for MobileApiError {}
