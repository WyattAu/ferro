use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum MobileError {
    #[error("Provider not registered")]
    NotRegistered,
    #[error("File not found: {0}")]
    NotFound(String),
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    #[error("Network error: {0}")]
    NetworkError(String),
    #[error("Cache full: {0} bytes used, {1} bytes limit")]
    CacheFull(u64, u64),
    #[error("Sync conflict: {0}")]
    Conflict(String),
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
    #[error("Camera error: {0}")]
    CameraError(String),
    #[error("Biometric auth error: {0}")]
    BiometricError(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MobileConflictStrategy {
    Skip,
    KeepLocal,
    KeepRemote,
    KeepBoth,
}
