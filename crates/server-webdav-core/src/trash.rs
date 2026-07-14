use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use dashmap::DashMap;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::warn;

use crate::{DbHandle, WebDavCoreState};

const MAX_TRASH_ENTRIES: usize = 1_000;

/// Local API error type for trash operations.
#[allow(dead_code)]
pub struct ApiError {
    status: StatusCode,
    code: &'static str,
    message: String,
}

impl ApiError {
    pub const PATH_INVALID: &'static str = "PATH_INVALID";
    pub const FILE_NOT_FOUND: &'static str = "FILE_NOT_FOUND";
    pub const TRASH_NOT_FOUND: &'static str = "TRASH_NOT_FOUND";
    pub const INTERNAL_ERROR: &'static str = "INTERNAL_ERROR";

    pub fn bad_request(code: &'static str, message: &'static str) -> Response {
        (
            StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({
                "error": code,
                "detail": message,
            })),
        )
            .into_response()
    }

    pub fn not_found(code: &'static str, message: &'static str) -> Response {
        (
            StatusCode::NOT_FOUND,
            axum::Json(serde_json::json!({
                "error": code,
                "detail": message,
            })),
        )
            .into_response()
    }

    pub fn internal(code: &'static str, message: impl Into<String>) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(serde_json::json!({
                "error": code,
                "detail": message.into(),
            })),
        )
            .into_response()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrashedEntry {
    pub original_path: String,
    pub trash_path: String,
    pub deleted_at: chrono::DateTime<chrono::Utc>,
    pub size: u64,
    pub mime_type: String,
}

// ---------------------------------------------------------------------------
// TrashStore – encapsulates the in-memory DashMap + optional SQLite persistence
// ---------------------------------------------------------------------------

pub struct TrashStore {
    entries: Arc<DashMap<String, TrashedEntry>>,
    trash_dir: Option<String>,
    db: Option<DbHandle>,
}

impl Clone for TrashStore {
    fn clone(&self) -> Self {
        Self {
            entries: Arc::clone(&self.entries),
            trash_dir: self.trash_dir.clone(),
            db: self.db.clone(),
        }
    }
}

impl Default for TrashStore {
    fn default() -> Self {
        Self::new()
    }
}

impl TrashStore {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(DashMap::new()),
            trash_dir: None,
            db: None,
        }
    }

    pub fn with_db(mut self, db: DbHandle) -> Self {
        self.db = Some(db);
        self
    }

    pub fn with_trash_dir(mut self, dir: String) -> Self {
        self.trash_dir = Some(dir);
        self
    }

    // -- DashMap delegation ------------------------------------------------

    pub fn list(&self) -> Vec<TrashedEntry> {
        self.entries.iter().map(|r| r.value().clone()).collect()
    }

    pub fn insert(&self, key: String, entry: TrashedEntry) {
        self.entries.insert(key, entry);
    }

    pub fn remove(&self, key: &str) -> Option<(String, TrashedEntry)> {
        self.entries.remove(key)
    }

    pub fn contains(&self, key: &str) -> bool {
        self.entries.contains_key(key)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn clear(&self) {
        self.entries.clear();
    }

    pub fn get(&self, key: &str) -> Option<dashmap::mapref::one::Ref<'_, String, TrashedEntry>> {
        self.entries.get(key)
    }

    pub fn iter(&self) -> dashmap::iter::Iter<'_, String, TrashedEntry> {
        self.entries.iter()
    }

    // -- Trash directory ---------------------------------------------------

    pub fn trash_dir(&self) -> Option<&str> {
        self.trash_dir.as_deref()
    }

    // -- Eviction ----------------------------------------------------------

    pub fn evict_oldest_if_needed(&self) {
        if self.entries.len() <= MAX_TRASH_ENTRIES {
            return;
        }
        while self.entries.len() > MAX_TRASH_ENTRIES {
            let oldest_key = self
                .entries
                .iter()
                .min_by_key(|e| e.value().deleted_at)
                .map(|e| e.key().clone());
            if let Some(key) = oldest_key {
                if let Some((_, entry)) = self.entries.remove(&key) {
                    delete_trash_file(&entry.trash_path);
                }
            } else {
                break;
            }
        }
    }

    // -- Persistence -------------------------------------------------------

    pub fn persist_insert(&self, entry: &TrashedEntry) {
        let Some(ref db) = self.db else {
            return;
        };
        persist_trash_insert(db, entry);
    }

    pub fn persist_remove(&self, original_path: &str) {
        let Some(ref db) = self.db else {
            return;
        };
        persist_trash_remove(db, original_path);
    }

    pub fn persist_clear(&self) {
        let Some(ref db) = self.db else {
            return;
        };
        persist_trash_clear(db);
    }

    pub fn load_from_db(&self) {
        let Some(ref db) = self.db else {
            return;
        };
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        if let Ok(entries) = load_trash_from_db(&conn) {
            for entry in entries {
                self.entries.insert(entry.original_path.clone(), entry);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct TrashedEntryResponse {
    pub original_path: String,
    pub deleted_at: String,
    pub size: u64,
    pub mime_type: String,
}

impl From<&TrashedEntry> for TrashedEntryResponse {
    fn from(e: &TrashedEntry) -> Self {
        Self {
            original_path: e.original_path.clone(),
            deleted_at: e.deleted_at.to_rfc3339(),
            size: e.size,
            mime_type: e.mime_type.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct TrashPathRequest {
    pub original_path: String,
}

// ---------------------------------------------------------------------------
// File-system helpers
// ---------------------------------------------------------------------------

fn generate_trash_path() -> String {
    let ts = chrono::Utc::now().timestamp_millis();
    let hash = uuid::Uuid::new_v4().to_string().replace('-', "");
    format!("{}_{}", ts, &hash[..16])
}

fn write_trash_file(trash_dir: &str, filename: &str, content: &[u8]) -> Result<PathBuf, std::io::Error> {
    let dir = PathBuf::from(trash_dir);
    std::fs::create_dir_all(&dir)?;
    let file_path = dir.join(filename);
    ferro_core::fs_util::atomic_write(&file_path, content)?;
    Ok(file_path)
}

async fn write_trash_file_async(
    trash_dir: &str,
    filename: &str,
    content: bytes::Bytes,
) -> Result<PathBuf, std::io::Error> {
    let dir = trash_dir.to_string();
    let filename = filename.to_string();
    tokio::task::spawn_blocking(move || write_trash_file(&dir, &filename, &content))
        .await
        .map_err(std::io::Error::other)?
}

async fn read_trash_file_async(trash_path: &str) -> Result<bytes::Bytes, std::io::Error> {
    let path = trash_path.to_string();
    tokio::task::spawn_blocking(move || std::fs::read(&path).map(bytes::Bytes::from))
        .await
        .map_err(std::io::Error::other)?
}

fn delete_trash_file(trash_path: &str) {
    if let Err(e) = std::fs::remove_file(trash_path) {
        warn!("Failed to delete trash file {}: {}", trash_path, e);
    }
}

// ---------------------------------------------------------------------------
// SQLite persistence helpers (standalone, used by TrashStore)
// ---------------------------------------------------------------------------

fn persist_trash_insert(db: &DbHandle, entry: &TrashedEntry) {
    let conn = db.lock().unwrap_or_else(|e| e.into_inner());
    if let Err(e) = conn.execute(
        "INSERT OR REPLACE INTO trash (original_path, trash_path, deleted_at, size, mime_type) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            entry.original_path,
            entry.trash_path,
            entry.deleted_at.to_rfc3339(),
            entry.size as i64,
            entry.mime_type,
        ],
    ) {
        warn!("Failed to persist trash entry to SQLite: {}", e);
    }
}

fn persist_trash_remove(db: &DbHandle, original_path: &str) {
    let conn = db.lock().unwrap_or_else(|e| e.into_inner());
    if let Err(e) = conn.execute("DELETE FROM trash WHERE original_path = ?1", params![original_path]) {
        warn!("Failed to remove trash entry from SQLite: {}", e);
    }
}

fn persist_trash_clear(db: &DbHandle) {
    let conn = db.lock().unwrap_or_else(|e| e.into_inner());
    if let Err(e) = conn.execute("DELETE FROM trash", []) {
        warn!("Failed to clear trash entries from SQLite: {}", e);
    }
}

pub fn load_trash_from_db(conn: &rusqlite::Connection) -> Result<Vec<TrashedEntry>, rusqlite::Error> {
    let mut stmt = conn.prepare("SELECT original_path, trash_path, deleted_at, size, mime_type FROM trash")?;
    let rows = stmt.query_map([], |row| {
        let deleted_at_str: String = row.get(2)?;
        let deleted_at = chrono::DateTime::parse_from_rfc3339(&deleted_at_str)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now());
        Ok(TrashedEntry {
            original_path: row.get(0)?,
            trash_path: row.get(1)?,
            deleted_at,
            size: row.get::<_, i64>(3)? as u64,
            mime_type: row.get(4)?,
        })
    })?;
    let mut entries = Vec::new();
    for row in rows {
        entries.push(row?);
    }
    Ok(entries)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

pub async fn list_trash<S: WebDavCoreState>(State(state): State<S>) -> Response {
    let trash = state.trash_store();
    let entries: Vec<TrashedEntryResponse> = trash.list().iter().map(TrashedEntryResponse::from).collect();
    (StatusCode::OK, axum::Json(serde_json::json!({ "entries": entries }))).into_response()
}

pub async fn move_to_trash<S: WebDavCoreState>(
    State(state): State<S>,
    axum::extract::Path(path): axum::extract::Path<String>,
) -> Response {
    let normalized = common::path::normalize_path(&path);

    if !common::path::validate_path(&normalized) {
        return ApiError::bad_request(ApiError::PATH_INVALID, "Invalid path");
    }

    let content = match state.storage().get(&normalized).await {
        Ok(c) => c,
        Err(_) => return ApiError::not_found(ApiError::FILE_NOT_FOUND, "File not found"),
    };

    let size = content.len() as u64;
    let mime_type = state
        .storage()
        .head(&normalized)
        .await
        .map(|m| m.mime_type)
        .unwrap_or_else(|_| "application/octet-stream".to_string());

    if let Err(e) = state.storage().delete(&normalized).await {
        return ApiError::internal(ApiError::INTERNAL_ERROR, format!("Delete failed: {}", e));
    }

    let trash_filename = generate_trash_path();
    let trash_path_str = if let Some(dir) = state.trash_store().trash_dir() {
        match write_trash_file_async(dir, &trash_filename, content.clone()).await {
            Ok(p) => p.to_string_lossy().to_string(),
            Err(e) => {
                warn!("Failed to write trash file, using memory fallback: {}", e);
                format!(".trash/{}", trash_filename)
            }
        }
    } else {
        format!(".trash/{}", trash_filename)
    };

    let entry = TrashedEntry {
        original_path: normalized.to_string(),
        trash_path: trash_path_str,
        deleted_at: chrono::Utc::now(),
        size,
        mime_type,
    };

    state.trash_store().insert(normalized.to_string(), entry.clone());
    state.trash_store().evict_oldest_if_needed();
    // Use the local entry directly instead of re-fetching from DashMap,
    // which may have evicted this entry during evict_oldest_if_needed.
    state.trash_store().persist_insert(&entry);

    (StatusCode::OK, axum::Json(serde_json::json!({ "ok": true }))).into_response()
}

pub async fn restore_trash<S: WebDavCoreState>(
    State(state): State<S>,
    axum::Json(body): axum::Json<TrashPathRequest>,
) -> Response {
    let normalized = common::path::normalize_path(&body.original_path);

    let entry = match state.trash_store().remove(&normalized) {
        Some((_, entry)) => entry,
        None => return ApiError::not_found(ApiError::TRASH_NOT_FOUND, "File not found in trash"),
    };

    state.trash_store().persist_remove(&normalized);

    let content = match read_trash_file_async(&entry.trash_path).await {
        Ok(bytes) => bytes,
        Err(_) => {
            state.trash_store().insert(normalized.to_string(), entry);
            return ApiError::internal(ApiError::INTERNAL_ERROR, "Trash file not found on disk");
        }
    };

    if let Err(e) = state
        .storage()
        .put(&entry.original_path, content.clone(), "anonymous")
        .await
    {
        state.trash_store().insert(normalized.to_string(), entry);
        return ApiError::internal(ApiError::INTERNAL_ERROR, format!("Restore failed: {}", e));
    }

    delete_trash_file(&entry.trash_path);

    (StatusCode::OK, axum::Json(serde_json::json!({ "ok": true }))).into_response()
}

pub async fn purge_trash<S: WebDavCoreState>(
    State(state): State<S>,
    axum::Json(body): axum::Json<TrashPathRequest>,
) -> Response {
    let normalized = common::path::normalize_path(&body.original_path);

    if let Some((_, entry)) = state.trash_store().remove(&normalized) {
        delete_trash_file(&entry.trash_path);
        state.trash_store().persist_remove(&normalized);
    } else {
        return ApiError::not_found(ApiError::TRASH_NOT_FOUND, "File not found in trash");
    }

    (StatusCode::OK, axum::Json(serde_json::json!({ "ok": true }))).into_response()
}

pub async fn empty_trash<S: WebDavCoreState>(State(state): State<S>) -> Response {
    for entry in state.trash_store().iter() {
        delete_trash_file(&entry.trash_path);
    }
    state.trash_store().clear();
    state.trash_store().persist_clear();
    (StatusCode::OK, axum::Json(serde_json::json!({ "ok": true }))).into_response()
}

pub async fn purge_expired<S: WebDavCoreState>(state: &S, ttl: std::time::Duration) -> usize {
    let cutoff = chrono::Utc::now() - ttl;
    let mut keys_to_remove = Vec::new();

    for entry in state.trash_store().iter() {
        if entry.deleted_at < cutoff {
            keys_to_remove.push(entry.key().clone());
        }
    }

    let mut purged = 0;
    for key in keys_to_remove {
        if let Some((_, entry)) = state.trash_store().remove(&key) {
            let trash_path = entry.trash_path.clone();
            tokio::spawn(async move {
                tokio::fs::remove_file(&trash_path).await.ok();
            });
            purged += 1;
        }
    }

    purged
}

pub async fn soft_delete<S: WebDavCoreState>(state: &S, path: &str) -> Result<(), Response> {
    let normalized = common::path::normalize_path(path);

    let content = match state.storage().get(&normalized).await {
        Ok(c) => c,
        Err(_) => {
            return Err(ApiError::not_found(ApiError::FILE_NOT_FOUND, "File not found"));
        }
    };

    let size = content.len() as u64;
    let mime_type = state
        .storage()
        .head(&normalized)
        .await
        .map(|m| m.mime_type)
        .unwrap_or_else(|_| "application/octet-stream".to_string());

    if let Err(e) = state.storage().delete(&normalized).await {
        return Err(ApiError::internal(
            ApiError::INTERNAL_ERROR,
            format!("Delete failed: {}", e),
        ));
    }

    let trash_filename = generate_trash_path();
    let trash_path_str = if let Some(dir) = state.trash_store().trash_dir() {
        match write_trash_file_async(dir, &trash_filename, content.clone()).await {
            Ok(p) => p.to_string_lossy().to_string(),
            Err(e) => {
                warn!("Failed to write trash file, using memory fallback: {}", e);
                format!(".trash/{}", trash_filename)
            }
        }
    } else {
        format!(".trash/{}", trash_filename)
    };

    let entry = TrashedEntry {
        original_path: normalized.to_string(),
        trash_path: trash_path_str,
        deleted_at: chrono::Utc::now(),
        size,
        mime_type,
    };

    state.trash_store().insert(normalized.to_string(), entry.clone());
    state.trash_store().evict_oldest_if_needed();
    // Use the local entry directly instead of re-fetching from DashMap,
    // which may have evicted this entry during evict_oldest_if_needed.
    state.trash_store().persist_insert(&entry);
    Ok(())
}
