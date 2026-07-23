//! Tauri 2.0 mobile plugin layer for Ferro.
//!
//! Provides the Rust backend for iOS Files Provider and Android SAF integration.
//! Uses the same sync engine as desktop but with mobile-specific optimizations.

use serde::{Deserialize, Serialize};

const PROPFIND_BODY: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<D:propfind xmlns:D="DAV:">
  <D:prop>
    <D:resourcetype/>
    <D:getcontentlength/>
    <D:getlastmodified/>
    <D:getetag/>
  </D:prop>
</D:propfind>"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MobilePlatform {
    Android,
    Ios,
}

impl MobilePlatform {
    pub fn path_separator(&self) -> char {
        '/'
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MobileConflictStrategy {
    Skip,
    KeepLocal,
    KeepRemote,
    KeepBoth,
}

fn build_http_client(auth_token: &str) -> Result<reqwest::Client, MobileError> {
    common::http_client::build_client(
        auth_token,
        common::http_client::HttpClientOptions::default(),
    )
    .map_err(|e| MobileError::NetworkError(e))
}

async fn do_propfind_http(client: &reqwest::Client, server_url: &str, path: &str) -> Result<String, MobileError> {
    let url = format!("{}{}", server_url.trim_end_matches('/'), path);
    let response = client
        .request(
            reqwest::Method::from_bytes(b"PROPFIND").expect("valid HTTP method"),
            &url,
        )
        .header("Depth", "1")
        .header(reqwest::header::CONTENT_TYPE, "application/xml")
        .body(PROPFIND_BODY)
        .send()
        .await
        .map_err(|e| MobileError::NetworkError(e.to_string()))?;

    if response.status().as_u16() != 207 {
        return Err(MobileError::NetworkError(format!(
            "PROPFIND failed: {}",
            response.status()
        )));
    }

    response
        .text()
        .await
        .map_err(|e| MobileError::NetworkError(e.to_string()))
}

fn parse_http_date(s: &str) -> chrono::DateTime<chrono::Utc> {
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        return dt.with_timezone(&chrono::Utc);
    }
    let trimmed = s.trim_end_matches(" GMT");
    let parts: Vec<&str> = trimmed.splitn(2, ", ").collect();
    let date_str = if parts.len() == 2 { parts[1] } else { trimmed };
    if let Ok(ndt) = chrono::NaiveDateTime::parse_from_str(date_str, "%d %b %Y %H:%M:%S") {
        return chrono::TimeZone::from_utc_datetime(&chrono::Utc, &ndt);
    }
    chrono::Utc::now()
}

fn parse_propfind_xml(xml: &str, base_path: &str) -> Result<Vec<MobileFileInfo>, MobileError> {
    let document =
        roxmltree::Document::parse(xml).map_err(|e| MobileError::NetworkError(format!("XML parse error: {}", e)))?;
    let base_normalized = base_path.trim_end_matches('/');
    let mut entries = Vec::new();

    for node in document.descendants() {
        if !node.is_element() || node.tag_name().name() != "response" {
            continue;
        }

        let href = node
            .children()
            .find(|n| n.is_element() && n.tag_name().name() == "href")
            .and_then(|n| n.text())
            .unwrap_or("");

        let href_normalized = href.trim_end_matches('/');

        if href_normalized.is_empty() || href_normalized == base_normalized {
            continue;
        }

        let name = href_normalized
            .trim_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or("")
            .to_string();

        if name.is_empty() {
            continue;
        }

        let prop = node
            .children()
            .find(|n| n.is_element() && n.tag_name().name() == "propstat")
            .and_then(|ps| ps.children().find(|n| n.is_element() && n.tag_name().name() == "prop"));

        let is_dir = prop.is_some_and(|p| {
            p.descendants()
                .any(|n| n.is_element() && n.tag_name().name() == "collection")
        });

        let size = prop
            .and_then(|p| {
                p.children()
                    .find(|n| n.is_element() && n.tag_name().name() == "getcontentlength")
                    .and_then(|n| n.text())
                    .and_then(|t| t.parse::<u64>().ok())
            })
            .unwrap_or(0);

        let modified_str = prop
            .and_then(|p| {
                p.children()
                    .find(|n| n.is_element() && n.tag_name().name() == "getlastmodified")
                    .and_then(|n| n.text())
                    .map(|t| t.to_string())
            })
            .unwrap_or_default();

        let content_type = if is_dir {
            "inode/directory".to_string()
        } else {
            "application/octet-stream".to_string()
        };

        entries.push(MobileFileInfo {
            name,
            path: href.to_string(),
            size,
            content_type,
            modified: if modified_str.is_empty() {
                chrono::Utc::now()
            } else {
                parse_http_date(&modified_str)
            },
            is_directory: is_dir,
            thumbnail_uri: None,
        });
    }

    Ok(entries)
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

    pub async fn handle_file_open(&self, path: &str) -> Result<MobileFileHandle, MobileError> {
        let normalized = self.config.normalize_path(path);
        if normalized.is_empty() {
            return Err(MobileError::NotFound(path.to_string()));
        }
        let client = build_http_client(&self.config.auth_token)?;
        let url = format!("{}{}", self.config.server_url.trim_end_matches('/'), path);
        let response = client
            .head(&url)
            .send()
            .await
            .map_err(|e| MobileError::NetworkError(e.to_string()))?;
        if !response.status().is_success() {
            return Err(MobileError::NotFound(path.to_string()));
        }
        let size = response
            .headers()
            .get(reqwest::header::CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/octet-stream")
            .to_string();
        let modified = response
            .headers()
            .get("last-modified")
            .and_then(|v| v.to_str().ok())
            .map(parse_http_date)
            .unwrap_or_else(chrono::Utc::now);
        Ok(MobileFileHandle {
            path: normalized,
            size,
            content_type,
            modified,
            is_directory: path.ends_with('/'),
        })
    }

    pub async fn enumerate_directory(&self, path: &str) -> Result<Vec<MobileFileInfo>, MobileError> {
        let normalized = self.config.normalize_path(path);
        if normalized.is_empty() {
            return Ok(vec![]);
        }
        let client = build_http_client(&self.config.auth_token)?;
        let xml = do_propfind_http(&client, &self.config.server_url, path).await?;
        parse_propfind_xml(&xml, path)
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

    pub async fn handle_content_uri(&self, uri: &str) -> Result<MobileFileHandle, MobileError> {
        if uri.is_empty() {
            return Err(MobileError::NotFound(uri.to_string()));
        }
        let client = build_http_client(&self.config.auth_token)?;
        let url = format!("{}{}", self.config.server_url.trim_end_matches('/'), uri);
        let response = client
            .head(&url)
            .send()
            .await
            .map_err(|e| MobileError::NetworkError(e.to_string()))?;
        if !response.status().is_success() {
            return Err(MobileError::NotFound(uri.to_string()));
        }
        let size = response
            .headers()
            .get(reqwest::header::CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/octet-stream")
            .to_string();
        let modified = response
            .headers()
            .get("last-modified")
            .and_then(|v| v.to_str().ok())
            .map(parse_http_date)
            .unwrap_or_else(chrono::Utc::now);
        Ok(MobileFileHandle {
            path: uri.to_string(),
            size,
            content_type,
            modified,
            is_directory: false,
        })
    }

    pub async fn query_files(&self, parent_uri: &str) -> Result<Vec<MobileFileInfo>, MobileError> {
        if parent_uri.is_empty() {
            return Ok(vec![]);
        }
        let client = build_http_client(&self.config.auth_token)?;
        let xml = do_propfind_http(&client, &self.config.server_url, parent_uri).await?;
        parse_propfind_xml(&xml, parent_uri)
    }
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

    #[tokio::test]
    async fn test_ios_handle_file_open_empty_path() {
        let mut config = MobileSyncConfig::ios_defaults();
        config.server_url = "https://example.com".into();
        let provider = IosFilesProvider::new(config);
        let result = provider.handle_file_open("").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_ios_enumerate_empty_path() {
        let mut config = MobileSyncConfig::ios_defaults();
        config.server_url = "https://example.com".into();
        let provider = IosFilesProvider::new(config);
        let result = provider.enumerate_directory("").await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_android_query_files_empty_uri() {
        let mut config = MobileSyncConfig::android_defaults();
        config.server_url = "https://example.com".into();
        let provider = AndroidSAFProvider::new(config);
        let result = provider.query_files("").await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_conflict_strategy_variants() {
        assert_eq!(
            serde_json::to_string(&MobileConflictStrategy::Skip).unwrap(),
            "\"skip\""
        );
        assert_eq!(
            serde_json::to_string(&MobileConflictStrategy::KeepLocal).unwrap(),
            "\"keep_local\""
        );
        assert_eq!(
            serde_json::to_string(&MobileConflictStrategy::KeepRemote).unwrap(),
            "\"keep_remote\""
        );
        assert_eq!(
            serde_json::to_string(&MobileConflictStrategy::KeepBoth).unwrap(),
            "\"keep_both\""
        );
    }

    #[test]
    fn test_parse_http_date() {
        let dt = parse_http_date("Wed, 01 Jan 2024 00:00:00 GMT");
        assert_eq!(dt.format("%Y-%m-%d").to_string(), "2024-01-01");

        let dt = parse_http_date("2024-01-01T00:00:00Z");
        assert_eq!(dt.format("%Y-%m-%d").to_string(), "2024-01-01");

        let dt = parse_http_date("invalid");
        let now = chrono::Utc::now();
        assert!((now - dt).num_seconds().abs() <= 1);
    }

    #[test]
    fn test_parse_propfind_xml() {
        let xml = r#"<?xml version="1.0"?>
<D:multistatus xmlns:D="DAV:">
  <D:response>
    <D:href>/docs/</D:href>
    <D:propstat>
      <D:prop>
        <D:resourcetype><D:collection/></D:resourcetype>
      </D:prop>
    </D:propstat>
  </D:response>
  <D:response>
    <D:href>/readme.txt</D:href>
    <D:propstat>
      <D:prop>
        <D:getcontentlength>42</D:getcontentlength>
        <D:getlastmodified>Wed, 01 Jan 2024 00:00:00 GMT</D:getlastmodified>
      </D:prop>
    </D:propstat>
  </D:response>
</D:multistatus>"#;
        let entries = parse_propfind_xml(xml, "/").unwrap();
        assert_eq!(entries.len(), 2);
        assert!(entries[0].is_directory);
        assert_eq!(entries[0].name, "docs");
        assert!(!entries[1].is_directory);
        assert_eq!(entries[1].name, "readme.txt");
        assert_eq!(entries[1].size, 42);
    }
}
