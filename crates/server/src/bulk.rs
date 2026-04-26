use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;

use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct BulkDeleteRequest {
    pub paths: Vec<String>,
}

pub async fn bulk_delete(
    State(state): State<AppState>,
    axum::Json(body): axum::Json<BulkDeleteRequest>,
) -> Response {
    let mut succeeded: Vec<String> = Vec::new();
    let mut failed: Vec<serde_json::Value> = Vec::new();

    for path in &body.paths {
        let normalized = common::path::normalize_path(path);

        if !common::path::validate_path(&normalized) {
            failed.push(serde_json::json!({
                "path": path,
                "error": "Invalid path",
            }));
            continue;
        }

        match state.storage.delete(&normalized).await {
            Ok(()) => {
                succeeded.push(path.clone());
                crate::indexer::remove_file(&state, &normalized).await;
            }
            Err(e) => {
                failed.push(serde_json::json!({
                    "path": path,
                    "error": e.to_string(),
                }));
            }
        }
    }

    (StatusCode::OK, axum::Json(serde_json::json!({
        "succeeded": succeeded,
        "failed": failed,
        "total_requested": body.paths.len(),
    }))).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AppState;

    fn test_state() -> AppState {
        AppState::in_memory()
    }

    #[tokio::test]
    async fn test_bulk_delete_success() {
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

        let resp = bulk_delete(
            State(state.clone()),
            axum::Json(BulkDeleteRequest {
                paths: vec!["/a.txt".to_string(), "/b.txt".to_string()],
            }),
        )
        .await;

        assert_eq!(resp.status(), StatusCode::OK);
        assert!(state.storage.get("/a.txt").await.is_err());
        assert!(state.storage.get("/b.txt").await.is_err());
    }

    #[tokio::test]
    async fn test_bulk_delete_partial_failure() {
        let state = test_state();
        state
            .storage
            .put("/exists.txt", bytes::Bytes::from("data"), "anonymous")
            .await
            .unwrap();

        let resp = bulk_delete(
            State(state.clone()),
            axum::Json(BulkDeleteRequest {
                paths: vec![
                    "/exists.txt".to_string(),
                    "/missing.txt".to_string(),
                ],
            }),
        )
        .await;

        assert_eq!(resp.status(), StatusCode::OK);
        assert!(state.storage.get("/exists.txt").await.is_err());
    }

    #[tokio::test]
    async fn test_bulk_delete_empty_list() {
        let state = test_state();

        let resp = bulk_delete(
            State(state),
            axum::Json(BulkDeleteRequest {
                paths: vec![],
            }),
        )
        .await;

        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_bulk_delete_invalid_path() {
        let state = test_state();

        let resp = bulk_delete(
            State(state),
            axum::Json(BulkDeleteRequest {
                paths: vec!["/../etc/passwd".to_string()],
            }),
        )
        .await;

        assert_eq!(resp.status(), StatusCode::OK);
    }
}
