use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncCheckpoint {
    pub cursor: String,
    pub last_sync: DateTime<Utc>,
    pub device_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangesSinceRequest {
    pub cursor: String,
    pub path_prefix: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChange {
    pub path: String,
    pub change_type: ChangeType,
    pub etag: Option<String>,
    pub size: Option<u64>,
    pub modified: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ChangeType {
    Created,
    Modified,
    Deleted,
    Moved { new_path: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangesSinceResponse {
    pub changes: Vec<FileChange>,
    pub new_cursor: String,
    pub has_more: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncUploadBatch {
    pub files: Vec<SyncFileUpload>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncFileUpload {
    pub path: String,
    pub content_type: String,
    pub size: u64,
    pub etag: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncUploadResponse {
    pub results: Vec<SyncUploadResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncUploadResult {
    pub path: String,
    pub status: SyncUploadStatus,
    pub etag: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncUploadStatus {
    Created,
    Updated,
    Conflict,
    QuotaExceeded,
}
