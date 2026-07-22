//! Typed API endpoint definitions.
//!
//! All endpoints use `/api/v1/` prefix.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub is_collection: bool,
    pub modified_at: String,
    pub mime_type: Option<String>,
    pub etag: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListFilesResponse {
    pub entries: Vec<FileEntry>,
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub struct AuthConfig {
    pub configured: bool,
    pub login_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AuthInfo {
    pub sub: String,
    pub email: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SearchParams {
    pub query: String,
    pub file_type: Option<String>,
    pub sort: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SearchResult {
    pub entries: Vec<FileEntry>,
    pub total: usize,
}

#[derive(Debug, Deserialize)]
pub struct FavoritesResponse {
    pub paths: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct TrashEntry {
    pub name: String,
    pub path: String,
    pub original_path: String,
    pub deleted_at: String,
    pub size: u64,
}

#[derive(Debug, Serialize)]
pub struct CreateShareRequest {
    pub path: String,
    pub expires_at: Option<String>,
    pub password: Option<String>,
    pub max_downloads: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct ShareLink {
    pub token: String,
    pub url: String,
    pub path: String,
    pub expires_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct VersionInfo {
    pub id: String,
    pub version: u32,
    pub created_at: String,
    pub size: u64,
    pub author: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DashboardData {
    pub total_files: u64,
    pub total_size: u64,
    pub recent_files: Vec<FileEntry>,
    pub shared_links: u64,
}

#[derive(Debug, Deserialize)]
pub struct QuotaInfo {
    pub used: u64,
    pub total: u64,
    pub unlimited: bool,
}

#[derive(Debug, Deserialize)]
pub struct ActivityEntry {
    pub action: String,
    pub path: String,
    pub timestamp: String,
    pub user: Option<String>,
}
