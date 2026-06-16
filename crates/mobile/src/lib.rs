use serde::{Deserialize, Serialize};

#[cfg(any(feature = "ios", feature = "android"))]
pub mod commands;
mod platform;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MobilePlatform {
    Ios,
    Android,
}

impl MobilePlatform {
    pub fn current() -> Self {
        #[cfg(target_os = "ios")]
        {
            MobilePlatform::Ios
        }
        #[cfg(target_os = "android")]
        {
            MobilePlatform::Android
        }
        #[cfg(not(any(target_os = "ios", target_os = "android")))]
        {
            MobilePlatform::Android
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageStats {
    pub local_cache_bytes: u64,
    pub local_cache_limit_bytes: u64,
    pub server_used_bytes: u64,
    pub server_total_bytes: u64,
    pub pinned_files: u32,
    pub pinned_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileFileEntry {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub is_dir: bool,
    pub modified: String,
    pub content_type: String,
    pub is_pinned: bool,
    pub is_available_offline: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SyncStatus {
    Idle,
    Syncing,
    Error(String),
    Conflict,
    Paused,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectivityState {
    pub connected: bool,
    pub wifi: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CameraUploadResult {
    pub success: bool,
    pub file_path: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BiometricAuthResult {
    pub authenticated: bool,
    pub error: Option<String>,
}

#[derive(Debug, thiserror::Error)]
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
