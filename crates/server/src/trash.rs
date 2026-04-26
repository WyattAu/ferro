use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};

use crate::api_error::ApiError;
use crate::AppState;

#[derive(Debug, Clone)]
pub struct TrashedEntry {
    pub original_path: String,
    pub content: bytes::Bytes,
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

pub async fn list_trash(State(state): State<AppState>) -> Response {
    let entries: Vec<TrashedEntryResponse> = state
        .trash
        .iter()
        .map(|r| TrashedEntryResponse::from(r.value()))
        .collect();
    (StatusCode::OK, axum::Json(serde_json::json!({ "entries": entries }))).into_response()
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

    let entry = TrashedEntry {
        original_path: normalized.clone(),
        content,
        deleted_at: chrono::Utc::now(),
        size,
        mime_type,
    };

    state.trash.insert(normalized, entry);

    (StatusCode::OK, axum::Json(serde_json::json!({ "ok": true }))).into_response()
}

pub async fn restore_trash(
    State(state): State<AppState>,
    axum::Json(body): axum::Json<TrashPathRequest>,
) -> Response {
    let normalized = common::path::normalize_path(&body.original_path);

    let entry = match state.trash.remove(&normalized) {
        Some((_, entry)) => entry,
        None => return ApiError::not_found("TRASH_NOT_FOUND", "File not found in trash"),
    };

    if let Err(e) = state
        .storage
        .put(&entry.original_path, entry.content.clone(), "anonymous")
        .await
    {
        state.trash.insert(normalized, entry);
        return ApiError::internal(
            ApiError::INTERNAL_ERROR,
            format!("Restore failed: {}", e),
        );
    }

    (StatusCode::OK, axum::Json(serde_json::json!({ "ok": true }))).into_response()
}

pub async fn purge_trash(
    State(state): State<AppState>,
    axum::Json(body): axum::Json<TrashPathRequest>,
) -> Response {
    let normalized = common::path::normalize_path(&body.original_path);

    if state.trash.remove(&normalized).is_none() {
        return ApiError::not_found("TRASH_NOT_FOUND", "File not found in trash");
    }

    (StatusCode::OK, axum::Json(serde_json::json!({ "ok": true }))).into_response()
}

pub async fn empty_trash(State(state): State<AppState>) -> Response {
    state.trash.clear();
    (StatusCode::OK, axum::Json(serde_json::json!({ "ok": true }))).into_response()
}

pub async fn soft_delete(
    state: &AppState,
    path: &str,
) -> Result<(), Response> {
    let normalized = common::path::normalize_path(path);

    let content = match state.storage.get(&normalized).await {
        Ok(c) => c,
        Err(_) => return Err(ApiError::not_found(ApiError::FILE_NOT_FOUND, "File not found")),
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

    let entry = TrashedEntry {
        original_path: normalized.clone(),
        content,
        deleted_at: chrono::Utc::now(),
        size,
        mime_type,
    };

    state.trash.insert(normalized, entry);
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
        let state = test_state();
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
        let resp = move_to_trash(
            State(state),
            axum::extract::Path("nope.txt".to_string()),
        )
        .await;
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
}
