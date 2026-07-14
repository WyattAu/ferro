use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub is_dir: bool,
    pub modified: String,
    pub etag: Option<String>,
    pub content_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryInfo {
    pub path: String,
    pub entries: Vec<FileEntry>,
    pub total_size: u64,
}

#[allow(dead_code)]
pub type ProgressCallback = Box<dyn Fn(u64, u64) + Send + Sync>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadProgress {
    pub bytes_uploaded: u64,
    pub total_bytes: u64,
    pub percent: f64,
}

impl UploadProgress {
    pub fn new(uploaded: u64, total: u64) -> Self {
        Self {
            bytes_uploaded: uploaded,
            total_bytes: total,
            percent: if total > 0 {
                (uploaded as f64 / total as f64) * 100.0
            } else {
                0.0
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub server_url: String,
    pub root_entries: u64,
    pub is_authenticated: bool,
}
