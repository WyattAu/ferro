use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use dashmap::DashMap;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::warn;

use crate::AppState;
use crate::api_error::ApiError;
use crate::db::DbHandle;

const MAX_TRASH_ENTRIES: usize = 1_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrashedEntry {
    pub original_path: String,
    pub trash_path: String,
    pub deleted_at: chrono::DateTime<chrono::Utc>,
    pub size: u64,
    pub mime_type: String,
}

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

fn generate_trash_path() -> String {
    let ts = chrono::Utc::now().timestamp_millis();
    let hash = uuid::Uuid::new_v4().to_string().replace('-', "");
    format!("{}_{}", ts, &hash[..16])
}

fn write_trash_file(
    trash_dir: &str,
    filename: &str,
    content: &[u8],
) -> Result<PathBuf, std::io::Error> {
    let dir = PathBuf::from(trash_dir);
    std::fs::create_dir_all(&dir)?;
    let file_path = dir.join(filename);
    std::fs::write(&file_path, content)?;
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

fn evict_oldest_if_needed(trash: &DashMap<String, TrashedEntry>) {
    if trash.len() <= MAX_TRASH_ENTRIES {
        return;
    }
    while trash.len() > MAX_TRASH_ENTRIES {
        let oldest_key = trash
            .iter()
            .min_by_key(|e| e.value().deleted_at)
            .map(|e| e.key().clone());
        if let Some(key) = oldest_key {
            if let Some((_, entry)) = trash.remove(&key) {
                delete_trash_file(&entry.trash_path);
            }
        } else {
            break;
        }
    }
}

pub fn persist_trash_insert(db: &DbHandle, entry: &TrashedEntry) {
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

pub fn persist_trash_remove(db: &DbHandle, original_path: &str) {
    let conn = db.lock().unwrap_or_else(|e| e.into_inner());
    if let Err(e) = conn.execute(
        "DELETE FROM trash WHERE original_path = ?1",
        params![original_path],
    ) {
        warn!("Failed to remove trash entry from SQLite: {}", e);
    }
}

pub fn persist_trash_clear(db: &DbHandle) {
    let conn = db.lock().unwrap_or_else(|e| e.into_inner());
    if let Err(e) = conn.execute("DELETE FROM trash", []) {
        warn!("Failed to clear trash entries from SQLite: {}", e);
    }
}

pub fn load_trash_from_db(
    conn: &rusqlite::Connection,
) -> Result<Vec<TrashedEntry>, rusqlite::Error> {
    let mut stmt =
        conn.prepare("SELECT original_path, trash_path, deleted_at, size, mime_type FROM trash")?;
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

pub async fn list_trash(State(state): State<AppState>) -> Response {
    let entries: Vec<TrashedEntryResponse> = state
        .trash
        .iter()
        .map(|r| TrashedEntryResponse::from(r.value()))
        .collect();
    (
        StatusCode::OK,
        axum::Json(serde_json::json!({ "entries": entries })),
    )
        .into_response()
}

pub async fn move_to_trash(
    State(state): State<AppState>,
    axum::extract::Path(path): axum::extract::Path<String>,
) -> Response {
    let normalized = common::path::normalize_path(&path);

    if !common::path::validate_path(&normalized) {
        return ApiError::bad_request(ApiError::PATH_INVALID, "Invalid path");
    }

    let content = match state.storage.get(&normalized).await {
        Ok(c) => c,
        Err(_) => return ApiError::not_found(ApiError::FILE_NOT_FOUND, "File not found"),
    };

    let size = content.len() as u64;
    let mime_type = state
        .storage
        .head(&normalized)
        .await
        .map(|m| m.mime_type)
        .unwrap_or_else(|_| "application/octet-stream".to_string());

    if let Err(e) = state.storage.delete(&normalized).await {
        return ApiError::internal(ApiError::INTERNAL_ERROR, format!("Delete failed: {}", e));
    }

    let trash_filename = generate_trash_path();
    let trash_path_str = if let Some(ref dir) = state.trash_dir {
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
        original_path: normalized.clone(),
        trash_path: trash_path_str,
        deleted_at: chrono::Utc::now(),
        size,
        mime_type,
    };

    state.trash.insert(normalized.clone(), entry.clone());
    evict_oldest_if_needed(&state.trash);
    if let Some(ref db) = state.db {
        // Use the local entry directly instead of re-fetching from DashMap,
        // which may have evicted this entry during evict_oldest_if_needed.
        persist_trash_insert(db, &entry);
    }

    (
        StatusCode::OK,
        axum::Json(serde_json::json!({ "ok": true })),
    )
        .into_response()
}

pub async fn restore_trash(
    State(state): State<AppState>,
    axum::Json(body): axum::Json<TrashPathRequest>,
) -> Response {
    let normalized = common::path::normalize_path(&body.original_path);

    let entry = match state.trash.remove(&normalized) {
        Some((_, entry)) => entry,
        None => return ApiError::not_found(ApiError::TRASH_NOT_FOUND, "File not found in trash"),
    };

    if let Some(ref db) = state.db {
        persist_trash_remove(db, &normalized);
    }

    let content = match read_trash_file_async(&entry.trash_path).await {
        Ok(bytes) => bytes,
        Err(_) => {
            state.trash.insert(normalized, entry);
            return ApiError::internal(ApiError::INTERNAL_ERROR, "Trash file not found on disk");
        }
    };

    if let Err(e) = state
        .storage
        .put(&entry.original_path, content.clone(), "anonymous")
        .await
    {
        state.trash.insert(normalized, entry);
        return ApiError::internal(ApiError::INTERNAL_ERROR, format!("Restore failed: {}", e));
    }

    delete_trash_file(&entry.trash_path);

    (
        StatusCode::OK,
        axum::Json(serde_json::json!({ "ok": true })),
    )
        .into_response()
}

pub async fn purge_trash(
    State(state): State<AppState>,
    axum::Json(body): axum::Json<TrashPathRequest>,
) -> Response {
    let normalized = common::path::normalize_path(&body.original_path);

    if let Some((_, entry)) = state.trash.remove(&normalized) {
        delete_trash_file(&entry.trash_path);
        if let Some(ref db) = state.db {
            persist_trash_remove(db, &normalized);
        }
    } else {
        return ApiError::not_found(ApiError::TRASH_NOT_FOUND, "File not found in trash");
    }

    (
        StatusCode::OK,
        axum::Json(serde_json::json!({ "ok": true })),
    )
        .into_response()
}

pub async fn empty_trash(State(state): State<AppState>) -> Response {
    for entry in state.trash.iter() {
        delete_trash_file(&entry.trash_path);
    }
    state.trash.clear();
    if let Some(ref db) = state.db {
        persist_trash_clear(db);
    }
    (
        StatusCode::OK,
        axum::Json(serde_json::json!({ "ok": true })),
    )
        .into_response()
}

pub async fn purge_expired(state: &AppState, ttl: std::time::Duration) -> usize {
    let cutoff = chrono::Utc::now() - ttl;
    let mut keys_to_remove = Vec::new();

    for entry in state.trash.iter() {
        if entry.deleted_at < cutoff {
            keys_to_remove.push(entry.key().clone());
        }
    }

    let mut purged = 0;
    for key in keys_to_remove {
        if let Some((_, entry)) = state.trash.remove(&key) {
            let trash_path = entry.trash_path.clone();
            tokio::spawn(async move {
                tokio::fs::remove_file(&trash_path).await.ok();
            });
            purged += 1;
        }
    }

    purged
}

pub async fn soft_delete(state: &AppState, path: &str) -> Result<(), Response> {
    let normalized = common::path::normalize_path(path);

    let content = match state.storage.get(&normalized).await {
        Ok(c) => c,
        Err(_) => {
            return Err(ApiError::not_found(
                ApiError::FILE_NOT_FOUND,
                "File not found",
            ));
        }
    };

    let size = content.len() as u64;
    let mime_type = state
        .storage
        .head(&normalized)
        .await
        .map(|m| m.mime_type)
        .unwrap_or_else(|_| "application/octet-stream".to_string());

    if let Err(e) = state.storage.delete(&normalized).await {
        return Err(ApiError::internal(
            ApiError::INTERNAL_ERROR,
            format!("Delete failed: {}", e),
        ));
    }

    let trash_filename = generate_trash_path();
    let trash_path_str = if let Some(ref dir) = state.trash_dir {
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
        original_path: normalized.clone(),
        trash_path: trash_path_str,
        deleted_at: chrono::Utc::now(),
        size,
        mime_type,
    };

    state.trash.insert(normalized.clone(), entry.clone());
    evict_oldest_if_needed(&state.trash);
    if let Some(ref db) = state.db {
        // Use the local entry directly instead of re-fetching from DashMap,
        // which may have evicted this entry during evict_oldest_if_needed.
        persist_trash_insert(db, &entry);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AppState;
    use axum::http::StatusCode;

    fn test_state() -> AppState {
        AppState::in_memory()
    }

    #[tokio::test]
    async fn test_list_trash_empty() {
        let state = test_state();
        let resp = list_trash(State(state)).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_move_to_trash_and_list() {
        let state = test_state();
        state
            .storage
            .put("/test.txt", bytes::Bytes::from("hello"), "anonymous")
            .await
            .unwrap();

        let resp = move_to_trash(
            State(state.clone()),
            axum::extract::Path("test.txt".to_string()),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK);

        assert!(!state.trash.is_empty());

        let resp = list_trash(State(state)).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_restore_trash() {
        let tmp = tempfile::tempdir().unwrap();
        let trash_dir = tmp.path().join(".trash");
        std::fs::create_dir_all(&trash_dir).unwrap();
        let state = AppState::in_memory().with_trash_dir(trash_dir.to_string_lossy().to_string());
        state
            .storage
            .put("/restore-me.txt", bytes::Bytes::from("data"), "anonymous")
            .await
            .unwrap();

        move_to_trash(
            State(state.clone()),
            axum::extract::Path("restore-me.txt".to_string()),
        )
        .await;

        assert!(state.storage.get("/restore-me.txt").await.is_err());

        let resp = restore_trash(
            State(state.clone()),
            axum::Json(TrashPathRequest {
                original_path: "/restore-me.txt".to_string(),
            }),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK);

        let content = state.storage.get("/restore-me.txt").await.unwrap();
        assert_eq!(content, bytes::Bytes::from("data"));
    }

    #[tokio::test]
    async fn test_purge_trash() {
        let state = test_state();
        state
            .storage
            .put("/purge-me.txt", bytes::Bytes::from("gone"), "anonymous")
            .await
            .unwrap();

        move_to_trash(
            State(state.clone()),
            axum::extract::Path("purge-me.txt".to_string()),
        )
        .await;

        let resp = purge_trash(
            State(state.clone()),
            axum::Json(TrashPathRequest {
                original_path: "/purge-me.txt".to_string(),
            }),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK);

        assert!(state.trash.is_empty());
        assert!(state.storage.get("/purge-me.txt").await.is_err());
    }

    #[tokio::test]
    async fn test_empty_trash() {
        let state = test_state();
        state
            .storage
            .put("/a.txt", bytes::Bytes::from("a"), "anonymous")
            .await
            .unwrap();
        state
            .storage
            .put("/b.txt", bytes::Bytes::from("b"), "anonymous")
            .await
            .unwrap();

        move_to_trash(
            State(state.clone()),
            axum::extract::Path("a.txt".to_string()),
        )
        .await;
        move_to_trash(
            State(state.clone()),
            axum::extract::Path("b.txt".to_string()),
        )
        .await;

        assert_eq!(state.trash.len(), 2);

        let resp = empty_trash(State(state.clone())).await;
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(state.trash.is_empty());
    }

    #[tokio::test]
    async fn test_move_nonexistent_to_trash() {
        let state = test_state();
        let resp = move_to_trash(State(state), axum::extract::Path("nope.txt".to_string())).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_restore_nonexistent_trash_entry() {
        let state = test_state();
        let resp = restore_trash(
            State(state),
            axum::Json(TrashPathRequest {
                original_path: "/nope.txt".to_string(),
            }),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_soft_delete_helper() {
        let state = test_state();
        state
            .storage
            .put("/soft-del.txt", bytes::Bytes::from("soft"), "anonymous")
            .await
            .unwrap();

        let result = soft_delete(&state, "/soft-del.txt").await;
        assert!(result.is_ok());

        assert!(state.storage.get("/soft-del.txt").await.is_err());
        assert_eq!(state.trash.len(), 1);

        let entry = state.trash.get("/soft-del.txt").unwrap();
        assert_eq!(entry.size, 4);
    }

    #[tokio::test]
    async fn test_disk_trash_move_and_restore() {
        let tmp = tempfile::tempdir().unwrap();
        let trash_dir = tmp.path().join(".trash");
        std::fs::create_dir_all(&trash_dir).unwrap();
        let state = AppState::in_memory().with_trash_dir(trash_dir.to_string_lossy().to_string());

        state
            .storage
            .put(
                "/disk-test.txt",
                bytes::Bytes::from("disk content"),
                "anonymous",
            )
            .await
            .unwrap();

        let resp = move_to_trash(
            State(state.clone()),
            axum::extract::Path("disk-test.txt".to_string()),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(state.storage.get("/disk-test.txt").await.is_err());

        let entry = state.trash.get("/disk-test.txt").unwrap();
        assert!(PathBuf::from(&entry.trash_path).exists());
        assert_eq!(entry.size, 12);
        drop(entry);

        let resp = restore_trash(
            State(state.clone()),
            axum::Json(TrashPathRequest {
                original_path: "/disk-test.txt".to_string(),
            }),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK);

        let content = state.storage.get("/disk-test.txt").await.unwrap();
        assert_eq!(content, bytes::Bytes::from("disk content"));
        assert!(state.trash.is_empty());
    }

    #[tokio::test]
    async fn test_disk_trash_purge_deletes_file() {
        let tmp = tempfile::tempdir().unwrap();
        let trash_dir = tmp.path().join(".trash");
        std::fs::create_dir_all(&trash_dir).unwrap();
        let state = AppState::in_memory().with_trash_dir(trash_dir.to_string_lossy().to_string());

        state
            .storage
            .put(
                "/purge-disk.txt",
                bytes::Bytes::from("purge me"),
                "anonymous",
            )
            .await
            .unwrap();

        move_to_trash(
            State(state.clone()),
            axum::extract::Path("purge-disk.txt".to_string()),
        )
        .await;

        let entry = state.trash.get("/purge-disk.txt").unwrap();
        let disk_path = entry.trash_path.clone();
        assert!(PathBuf::from(&disk_path).exists());
        drop(entry);

        let resp = purge_trash(
            State(state.clone()),
            axum::Json(TrashPathRequest {
                original_path: "/purge-disk.txt".to_string(),
            }),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(state.trash.is_empty());
        assert!(!PathBuf::from(&disk_path).exists());
    }

    #[tokio::test]
    async fn test_purge_expired() {
        let tmp = tempfile::tempdir().unwrap();
        let trash_dir = tmp.path().join(".trash");
        std::fs::create_dir_all(&trash_dir).unwrap();
        let state = AppState::in_memory().with_trash_dir(trash_dir.to_string_lossy().to_string());

        state
            .storage
            .put("/old1.txt", bytes::Bytes::from("old1"), "anonymous")
            .await
            .unwrap();
        state
            .storage
            .put("/old2.txt", bytes::Bytes::from("old2"), "anonymous")
            .await
            .unwrap();
        state
            .storage
            .put("/new.txt", bytes::Bytes::from("new"), "anonymous")
            .await
            .unwrap();

        move_to_trash(
            State(state.clone()),
            axum::extract::Path("old1.txt".to_string()),
        )
        .await;
        move_to_trash(
            State(state.clone()),
            axum::extract::Path("old2.txt".to_string()),
        )
        .await;

        let short_ttl = std::time::Duration::from_secs(0);
        let purged = purge_expired(&state, short_ttl).await;
        assert_eq!(purged, 2, "Both old entries should be purged with 0 TTL");
        assert_eq!(state.trash.len(), 0);

        move_to_trash(
            State(state.clone()),
            axum::extract::Path("new.txt".to_string()),
        )
        .await;

        let long_ttl = std::time::Duration::from_secs(3600);
        let purged = purge_expired(&state, long_ttl).await;
        assert_eq!(purged, 0, "No entries should be purged with long TTL");
        assert_eq!(state.trash.len(), 1);
    }

    #[tokio::test]
    async fn test_disk_trash_empty_deletes_all_files() {
        let tmp = tempfile::tempdir().unwrap();
        let trash_dir = tmp.path().join(".trash");
        std::fs::create_dir_all(&trash_dir).unwrap();
        let state = AppState::in_memory().with_trash_dir(trash_dir.to_string_lossy().to_string());

        state
            .storage
            .put("/e1.txt", bytes::Bytes::from("one"), "anonymous")
            .await
            .unwrap();
        state
            .storage
            .put("/e2.txt", bytes::Bytes::from("two"), "anonymous")
            .await
            .unwrap();

        move_to_trash(
            State(state.clone()),
            axum::extract::Path("e1.txt".to_string()),
        )
        .await;
        move_to_trash(
            State(state.clone()),
            axum::extract::Path("e2.txt".to_string()),
        )
        .await;

        let paths: Vec<String> = state.trash.iter().map(|e| e.trash_path.clone()).collect();
        assert_eq!(paths.len(), 2);
        for p in &paths {
            assert!(PathBuf::from(p).exists());
        }

        let resp = empty_trash(State(state.clone())).await;
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(state.trash.is_empty());
        for p in &paths {
            assert!(!PathBuf::from(p).exists());
        }
    }
}
