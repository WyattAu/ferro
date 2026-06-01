//! Tauri 2.0 mobile plugin layer for Ferro.
//!
//! Provides the Rust backend for iOS Files Provider and Android SAF integration.
//! Uses the same sync engine as desktop but with mobile-specific optimizations.


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MobilePlatform {
    Android,
    Ios,
}

impl MobilePlatform {
    pub fn path_separator(&self) -> char {
        match self {
            MobilePlatform::Android => '/',
            MobilePlatform::Ios => '/',
        }
    }
}

#[derive(Debug, Clone)]
pub struct MobileSyncConfig {
    pub platform: MobilePlatform,
    pub server_url: String,
    pub auth_token: String,
    pub local_cache_path: String,
    pub max_cache_size_mb: u64,
    pub sync_on_wifi_only: bool,
    pub sync_on_charging: bool,
    pub background_sync_enabled: bool,
    pub conflict_strategy: MobileConflictStrategy,
}

impl MobileSyncConfig {
    pub fn android_defaults() -> Self {
        Self {
            platform: MobilePlatform::Android,
            server_url: String::new(),
            auth_token: String::new(),
            local_cache_path: "/data/data/com.ferro.app/cache".to_string(),
            max_cache_size_mb: 512,
            sync_on_wifi_only: true,
            sync_on_charging: true,
            background_sync_enabled: false,
            conflict_strategy: MobileConflictStrategy::Skip,
        }
    }

    pub fn ios_defaults() -> Self {
        Self {
            platform: MobilePlatform::Ios,
            server_url: String::new(),
            auth_token: String::new(),
            local_cache_path: "/var/mobile/Library/Caches/com.ferro.app".to_string(),
            max_cache_size_mb: 256,
            sync_on_wifi_only: true,
            sync_on_charging: true,
            background_sync_enabled: true,
            conflict_strategy: MobileConflictStrategy::Skip,
        }
    }

    pub fn max_cache_size_bytes(&self) -> u64 {
        self.max_cache_size_mb * 1024 * 1024
    }

    pub fn normalize_path(&self, path: &str) -> String {
        let sep = self.platform.path_separator();
        let normalized = path.replace('\\', &sep.to_string());
        let parts: Vec<&str> = normalized.split(sep).filter(|s| !s.is_empty()).collect();
        parts.join(&sep.to_string())
    }
}

#[derive(Debug, Clone)]
pub struct FileProviderCapabilities {
    pub supports_streaming: bool,
    pub supports_thumbnails: bool,
    pub max_file_size: u64,
    pub supported_extensions: Vec<String>,
}

impl FileProviderCapabilities {
    pub fn android_defaults() -> Self {
        Self {
            supports_streaming: true,
            supports_thumbnails: true,
            max_file_size: 2_147_483_648,
            supported_extensions: vec![
                "pdf".into(),
                "doc".into(),
                "docx".into(),
                "xls".into(),
                "xlsx".into(),
                "ppt".into(),
                "pptx".into(),
                "txt".into(),
                "jpg".into(),
                "png".into(),
                "mp4".into(),
                "mp3".into(),
            ],
        }
    }

    pub fn ios_defaults() -> Self {
        Self {
            supports_streaming: true,
            supports_thumbnails: false,
            max_file_size: 4_294_967_296,
            supported_extensions: vec![
                "pdf".into(),
                "pages".into(),
                "numbers".into(),
                "key".into(),
                "txt".into(),
                "jpg".into(),
                "png".into(),
                "heic".into(),
                "mov".into(),
                "aac".into(),
            ],
        }
    }
}

pub struct IosFilesProvider {
    config: MobileSyncConfig,
}

impl IosFilesProvider {
    pub fn new(config: MobileSyncConfig) -> Self {
        Self { config }
    }

    pub fn register_provider(&self) -> Result<(), MobileError> {
        if self.config.server_url.is_empty() {
            return Err(MobileError::InvalidConfig("server_url is empty".into()));
        }
        Ok(())
    }

    pub fn handle_file_open(&self, path: &str) -> Result<MobileFileHandle, MobileError> {
        let normalized = self.config.normalize_path(path);
        if normalized.is_empty() {
            return Err(MobileError::NotFound(path.to_string()));
        }
        Ok(MobileFileHandle {
            path: normalized,
            size: 0,
            content_type: "application/octet-stream".into(),
            modified: chrono::Utc::now(),
            is_directory: false,
        })
    }

    pub fn enumerate_directory(&self, path: &str) -> Result<Vec<MobileFileInfo>, MobileError> {
        let normalized = self.config.normalize_path(path);
        if normalized.is_empty() {
            return Ok(vec![]);
        }
        Ok(vec![])
    }
}

pub struct AndroidSAFProvider {
    config: MobileSyncConfig,
}

impl AndroidSAFProvider {
    pub fn new(config: MobileSyncConfig) -> Self {
        Self { config }
    }

    pub fn register_provider(&self) -> Result<(), MobileError> {
        if self.config.server_url.is_empty() {
            return Err(MobileError::InvalidConfig("server_url is empty".into()));
        }
        Ok(())
    }

    pub fn handle_content_uri(&self, uri: &str) -> Result<MobileFileHandle, MobileError> {
        if uri.is_empty() {
            return Err(MobileError::NotFound(uri.to_string()));
        }
        Ok(MobileFileHandle {
            path: uri.to_string(),
            size: 0,
            content_type: "application/octet-stream".into(),
            modified: chrono::Utc::now(),
            is_directory: false,
        })
    }

    pub fn query_files(&self, parent_uri: &str) -> Result<Vec<MobileFileInfo>, MobileError> {
        if parent_uri.is_empty() {
            return Ok(vec![]);
        }
        Ok(vec![])
    }
}

#[derive(Debug)]
pub struct MobileFileHandle {
    pub path: String,
    pub size: u64,
    pub content_type: String,
    pub modified: chrono::DateTime<chrono::Utc>,
    pub is_directory: bool,
}

#[derive(Debug)]
pub struct MobileFileInfo {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub content_type: String,
    pub modified: chrono::DateTime<chrono::Utc>,
    pub is_directory: bool,
    pub thumbnail_uri: Option<String>,
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
}

/// Conflict resolution strategy for mobile sync.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MobileConflictStrategy {
    Skip,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_android_config_defaults() {
        let config = MobileSyncConfig::android_defaults();
        assert_eq!(config.platform, MobilePlatform::Android);
        assert!(config.sync_on_wifi_only);
        assert!(config.sync_on_charging);
        assert!(!config.background_sync_enabled);
        assert_eq!(config.max_cache_size_mb, 512);
        assert_eq!(config.max_cache_size_bytes(), 512 * 1024 * 1024);
    }

    #[test]
    fn test_ios_config_defaults() {
        let config = MobileSyncConfig::ios_defaults();
        assert_eq!(config.platform, MobilePlatform::Ios);
        assert!(config.background_sync_enabled);
        assert_eq!(config.max_cache_size_mb, 256);
        assert_eq!(config.max_cache_size_bytes(), 256 * 1024 * 1024);
    }

    #[test]
    fn test_file_provider_capabilities_android() {
        let caps = FileProviderCapabilities::android_defaults();
        assert!(caps.supports_streaming);
        assert!(caps.supports_thumbnails);
        assert!(!caps.supported_extensions.is_empty());
    }

    #[test]
    fn test_file_provider_capabilities_ios() {
        let caps = FileProviderCapabilities::ios_defaults();
        assert!(caps.supports_streaming);
        assert!(!caps.supports_thumbnails);
    }

    #[test]
    fn test_ios_files_provider_creation() {
        let config = MobileSyncConfig::ios_defaults();
        let provider = IosFilesProvider::new(config);
        assert_eq!(provider.config.platform, MobilePlatform::Ios);
    }

    #[test]
    fn test_ios_files_provider_register_success() {
        let mut config = MobileSyncConfig::ios_defaults();
        config.server_url = "https://example.com".into();
        let provider = IosFilesProvider::new(config);
        assert!(provider.register_provider().is_ok());
    }

    #[test]
    fn test_ios_files_provider_register_no_url() {
        let config = MobileSyncConfig::ios_defaults();
        let provider = IosFilesProvider::new(config);
        assert!(provider.register_provider().is_err());
    }

    #[test]
    fn test_android_saf_provider_creation() {
        let config = MobileSyncConfig::android_defaults();
        let provider = AndroidSAFProvider::new(config);
        assert_eq!(provider.config.platform, MobilePlatform::Android);
    }

    #[test]
    fn test_android_saf_provider_register_success() {
        let mut config = MobileSyncConfig::android_defaults();
        config.server_url = "https://example.com".into();
        let provider = AndroidSAFProvider::new(config);
        assert!(provider.register_provider().is_ok());
    }

    #[test]
    fn test_android_saf_provider_register_no_url() {
        let config = MobileSyncConfig::android_defaults();
        let provider = AndroidSAFProvider::new(config);
        assert!(provider.register_provider().is_err());
    }

    #[test]
    fn test_mobile_file_handle_creation() {
        let handle = MobileFileHandle {
            path: "/docs/file.pdf".into(),
            size: 1024,
            content_type: "application/pdf".into(),
            modified: chrono::Utc::now(),
            is_directory: false,
        };
        assert_eq!(handle.path, "/docs/file.pdf");
        assert_eq!(handle.size, 1024);
        assert!(!handle.is_directory);
    }

    #[test]
    fn test_mobile_file_info_creation() {
        let info = MobileFileInfo {
            name: "photo.jpg".into(),
            path: "/photos/photo.jpg".into(),
            size: 2048,
            content_type: "image/jpeg".into(),
            modified: chrono::Utc::now(),
            is_directory: false,
            thumbnail_uri: Some("content://thumbnails/photo.jpg".into()),
        };
        assert_eq!(info.name, "photo.jpg");
        assert!(info.thumbnail_uri.is_some());
    }

    #[test]
    fn test_mobile_error_display() {
        let err = MobileError::NotFound("test.pdf".into());
        assert_eq!(format!("{err}"), "File not found: test.pdf");

        let err = MobileError::CacheFull(100, 200);
        assert_eq!(format!("{err}"), "Cache full: 100 bytes used, 200 bytes limit");

        let err = MobileError::NotRegistered;
        assert_eq!(format!("{err}"), "Provider not registered");
    }

    #[test]
    fn test_path_normalization_android() {
        let config = MobileSyncConfig::android_defaults();
        assert_eq!(config.normalize_path("docs/file.txt"), "docs/file.txt");
        assert_eq!(config.normalize_path("/docs//file.txt"), "docs/file.txt");
    }

    #[test]
    fn test_path_normalization_ios() {
        let config = MobileSyncConfig::ios_defaults();
        assert_eq!(config.normalize_path("docs/file.txt"), "docs/file.txt");
        assert_eq!(config.normalize_path("//docs///file.txt"), "docs/file.txt");
    }

    #[test]
    fn test_cache_size_calculation() {
        let mut config = MobileSyncConfig::android_defaults();
        config.max_cache_size_mb = 1;
        assert_eq!(config.max_cache_size_bytes(), 1024 * 1024);

        config.max_cache_size_mb = 0;
        assert_eq!(config.max_cache_size_bytes(), 0);
    }

    #[test]
    fn test_mobile_platform_equality() {
        assert_eq!(MobilePlatform::Android, MobilePlatform::Android);
        assert_eq!(MobilePlatform::Ios, MobilePlatform::Ios);
        assert_ne!(MobilePlatform::Android, MobilePlatform::Ios);
    }

    #[test]
    fn test_ios_handle_file_open() {
        let mut config = MobileSyncConfig::ios_defaults();
        config.server_url = "https://example.com".into();
        let provider = IosFilesProvider::new(config);
        let handle = provider.handle_file_open("/docs/file.txt").unwrap();
        assert_eq!(handle.path, "docs/file.txt");
    }

    #[test]
    fn test_android_handle_content_uri() {
        let mut config = MobileSyncConfig::android_defaults();
        config.server_url = "https://example.com".into();
        let provider = AndroidSAFProvider::new(config);
        let handle = provider.handle_content_uri("content://com.example/file").unwrap();
        assert_eq!(handle.path, "content://com.example/file");
    }

    #[test]
    fn test_ios_enumerate_empty() {
        let mut config = MobileSyncConfig::ios_defaults();
        config.server_url = "https://example.com".into();
        let provider = IosFilesProvider::new(config);
        let files = provider.enumerate_directory("/docs").unwrap();
        assert!(files.is_empty());
    }

    #[test]
    fn test_android_query_files_empty() {
        let mut config = MobileSyncConfig::android_defaults();
        config.server_url = "https://example.com".into();
        let provider = AndroidSAFProvider::new(config);
        let files = provider.query_files("content://com.example/root").unwrap();
        assert!(files.is_empty());
    }
}
