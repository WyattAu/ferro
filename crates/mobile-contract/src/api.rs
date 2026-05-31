use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileAuthRequest {
    pub username: String,
    pub password: Option<String>,
    pub token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileAuthResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
    pub user: MobileUser,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileUser {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub storage_quota_bytes: u64,
    pub storage_used_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileListRequest {
    pub path: String,
    #[serde(default = "default_depth")]
    pub depth: u32,
    #[serde(default = "default_true")]
    pub include_metadata: bool,
    #[serde(default = "default_page_size")]
    pub page_size: u32,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileListResponse {
    pub files: Vec<MobileFile>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileFile {
    pub path: String,
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: DateTime<Utc>,
    pub created: DateTime<Utc>,
    pub content_type: Option<String>,
    pub etag: String,
    pub permissions: FilePermissions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilePermissions {
    pub can_read: bool,
    pub can_write: bool,
    pub can_delete: bool,
    pub can_share: bool,
}

impl Default for FilePermissions {
    fn default() -> Self {
        Self {
            can_read: true,
            can_write: true,
            can_delete: true,
            can_share: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadRequest {
    pub path: String,
    pub content_type: String,
    pub size: u64,
    pub overwrite: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadResponse {
    pub path: String,
    pub etag: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadRequest {
    pub path: String,
    pub range_start: Option<u64>,
    pub range_end: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateFolderRequest {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteRequest {
    pub path: String,
    pub recursive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveRequest {
    #[serde(rename = "from")]
    pub from: String,
    pub to: String,
    pub overwrite: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareCreateRequest {
    pub path: String,
    pub expiry_hours: Option<u32>,
    pub allow_download: bool,
    pub allow_upload: bool,
    pub password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareInfo {
    pub id: String,
    pub token: String,
    pub url: String,
    pub expires_at: Option<DateTime<Utc>>,
}

fn default_depth() -> u32 {
    1
}

fn default_true() -> bool {
    true
}

fn default_page_size() -> u32 {
    100
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_list_request_roundtrip() {
        let req = FileListRequest {
            path: "/Documents".to_string(),
            depth: 1,
            include_metadata: true,
            page_size: 100,
            cursor: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        let de: FileListRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(de.path, req.path);
        assert_eq!(de.depth, 1);
        assert!(de.include_metadata);
        assert_eq!(de.page_size, 100);
    }

    #[test]
    fn test_mobile_file_deserialization() {
        let json = r#"{
            "path": "/Documents/report.pdf",
            "name": "report.pdf",
            "is_dir": false,
            "size": 2048,
            "modified": "2025-01-15T10:30:00Z",
            "created": "2025-01-10T08:00:00Z",
            "content_type": "application/pdf",
            "etag": "\"abc123\"",
            "permissions": {"can_read": true, "can_write": true, "can_delete": false, "can_share": false}
        }"#;
        let file: MobileFile = serde_json::from_str(json).unwrap();
        assert_eq!(file.path, "/Documents/report.pdf");
        assert_eq!(file.size, 2048);
        assert_eq!(file.content_type.as_deref(), Some("application/pdf"));
        assert!(!file.permissions.can_delete);
    }

    #[test]
    fn test_share_create_request() {
        let req = ShareCreateRequest {
            path: "/Photos".to_string(),
            expiry_hours: Some(48),
            allow_download: true,
            allow_upload: false,
            password: Some("secret".to_string()),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("48"));
        assert!(json.contains("secret"));
        let de: ShareCreateRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(de.expiry_hours, Some(48));
    }

    #[test]
    fn test_file_permissions_defaults() {
        let perms = FilePermissions::default();
        assert!(perms.can_read);
        assert!(perms.can_write);
        assert!(perms.can_delete);
        assert!(!perms.can_share);
    }

    #[test]
    fn test_empty_file_list_response() {
        let resp = FileListResponse {
            files: vec![],
            next_cursor: None,
        };
        let json = serde_json::to_string(&resp).unwrap();
        let de: FileListResponse = serde_json::from_str(&json).unwrap();
        assert!(de.files.is_empty());
        assert!(de.next_cursor.is_none());
    }
}
