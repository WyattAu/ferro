use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};

use crate::activity::ActivityEntry;
use crate::AppState;

/// Dashboard overview returned to the web frontend.
#[derive(Debug, Serialize, Deserialize)]
pub struct DashboardResponse {
    pub storage_used: u64,
    pub storage_total: u64,
    pub file_count: u64,
    pub recent_files: Vec<RecentFile>,
    pub shared_files: Vec<SharedFile>,
    pub activity: Vec<ActivityEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RecentFile {
    pub path: String,
    pub modified_at: String,
    pub size: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SharedFile {
    pub token: String,
    pub path: String,
    pub expires_at: String,
    pub download_count: u32,
    pub created_by: String,
}

/// GET /api/dashboard — aggregated dashboard data for the web frontend.
pub async fn get_dashboard(State(state): State<AppState>) -> Response {
    let quota_bytes = state.quota_bytes.unwrap_or(0);
    let used_bytes = state.used_bytes.load(std::sync::atomic::Ordering::Relaxed);
    let file_count = state.file_count.load(std::sync::atomic::Ordering::Relaxed);

    // Recent activity (last 10 events)
    let audit_entries = state.audit_log.entries().await;
    let activity: Vec<ActivityEntry> = audit_entries
        .iter()
        .rev()
        .take(10)
        .map(|e| {
            let action = match (e.method.as_str(), e.status) {
                ("PUT", 200..=299) => "upload",
                ("DELETE", 200..=299) => "delete",
                ("MKCOL", 200..=299) => "create_folder",
                ("COPY", 200..=299) => "copy",
                ("MOVE", 200..=299) => "move",
                _ => "access",
            };
            ActivityEntry {
                action: action.to_string(),
                path: e.path.clone(),
                size: e.content_length,
                timestamp: e.timestamp.clone(),
                user: e.user.clone(),
            }
        })
        .collect();

    // Shared files (from share store)
    let shares = state.share_store.list().await;
    let shared_files: Vec<SharedFile> = shares
        .iter()
        .take(10)
        .map(|s| SharedFile {
            token: s.token.clone(),
            path: s.path.clone(),
            expires_at: s.expires_at.to_rfc3339(),
            download_count: s.download_count,
            created_by: s.created_by.clone(),
        })
        .collect();

    // Recent files — derive from audit log (most recent PUT/MKCOL events)
    let mut recent_files: Vec<RecentFile> = audit_entries
        .iter()
        .rev()
        .filter(|e| e.method == "PUT" || e.method == "MKCOL")
        .take(10)
        .map(|e| RecentFile {
            path: e.path.clone(),
            modified_at: e.timestamp.clone(),
            size: e.content_length.unwrap_or(0),
        })
        .collect();
    // Deduplicate by path (keep the first/most-recent entry per path)
    let mut seen = std::collections::HashSet::new();
    recent_files.retain(|f| seen.insert(f.path.clone()));

    (
        StatusCode::OK,
        axum::Json(DashboardResponse {
            storage_used: used_bytes,
            storage_total: quota_bytes,
            file_count,
            recent_files,
            shared_files,
            activity,
        }),
    )
        .into_response()
}
