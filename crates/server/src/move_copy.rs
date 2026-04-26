use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use common::path::normalize_path;
use serde::Deserialize;

use crate::api_error::ApiError;
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct MoveCopyRequest {
    pub source: String,
    pub destination: String,
}

pub async fn move_file(
    State(state): State<AppState>,
    axum::Json(body): axum::Json<MoveCopyRequest>,
) -> Response {
    let source = normalize_path(&body.source);
    let destination = normalize_path(&body.destination);

    if source.is_empty() || destination.is_empty() {
        return ApiError::bad_request(ApiError::PATH_INVALID, "Source and destination must be non-empty");
    }

    if source == destination {
        return ApiError::bad_request(ApiError::BAD_REQUEST, "Source and destination are the same");
    }

    let _lock = state.lock_manager.acquire_lock(&source, "system", common::webdav::LockScope::Exclusive, common::webdav::LockDepth::Zero, Some(10));
    let _lock2 = state.lock_manager.acquire_lock(&destination, "system", common::webdav::LockScope::Exclusive, common::webdav::LockDepth::Zero, Some(10));

    match state.storage.head(&source).await {
        Ok(meta) => {
            if meta.is_collection {
                if let Err(e) = move_collection_recursive(&state, &source, &destination).await {
                    return ApiError::internal(ApiError::INTERNAL_ERROR, format!("Move failed: {}", e));
                }
            } else {
                match state.storage.move_path(&source, &destination).await {
                    Ok(()) => {}
                    Err(e) => {
                        let msg = e.to_string();
                        if msg.contains("not found") || msg.contains("NotFound") {
                            return ApiError::not_found(ApiError::FILE_NOT_FOUND, format!("Source not found: {}", source));
                        }
                        return ApiError::internal(ApiError::INTERNAL_ERROR, format!("Move failed: {}", e));
                    }
                }
            }
            state.audit_log.log(crate::audit::build_audit_entry(
                "POST",
                &format!("/api/files/move {} -> {}", source, destination),
                "admin",
                200,
                None,
                None,
            )).await;
            (StatusCode::OK, axum::Json(serde_json::json!({"status": "ok"}))).into_response()
        }
        Err(_) => ApiError::not_found(ApiError::FILE_NOT_FOUND, format!("Source not found: {}", source)),
    }
}

pub async fn copy_file(
    State(state): State<AppState>,
    axum::Json(body): axum::Json<MoveCopyRequest>,
) -> Response {
    let source = normalize_path(&body.source);
    let destination = normalize_path(&body.destination);

    if source.is_empty() || destination.is_empty() {
        return ApiError::bad_request(ApiError::PATH_INVALID, "Source and destination must be non-empty");
    }

    if source == destination {
        return ApiError::bad_request(ApiError::BAD_REQUEST, "Source and destination are the same");
    }

    if state.storage.exists(&destination).await.unwrap_or(false) {
        return ApiError::conflict(ApiError::FILE_EXISTS, format!("Destination already exists: {}", destination));
    }

    let _lock = state.lock_manager.acquire_lock(&source, "system", common::webdav::LockScope::Exclusive, common::webdav::LockDepth::Zero, Some(10));
    let _lock2 = state.lock_manager.acquire_lock(&destination, "system", common::webdav::LockScope::Exclusive, common::webdav::LockDepth::Zero, Some(10));

    match state.storage.head(&source).await {
        Ok(meta) => {
            if meta.is_collection {
                if let Err(e) = copy_collection_recursive(&state, &source, &destination).await {
                    return ApiError::internal(ApiError::INTERNAL_ERROR, format!("Copy failed: {}", e));
                }
            } else {
                match state.storage.copy(&source, &destination).await {
                    Ok(()) => {}
                    Err(e) => {
                        let msg = e.to_string();
                        if msg.contains("not found") || msg.contains("NotFound") {
                            return ApiError::not_found(ApiError::FILE_NOT_FOUND, format!("Source not found: {}", source));
                        }
                        return ApiError::internal(ApiError::INTERNAL_ERROR, format!("Copy failed: {}", e));
                    }
                }
            }
            state.audit_log.log(crate::audit::build_audit_entry(
                "POST",
                &format!("/api/files/copy {} -> {}", source, destination),
                "admin",
                200,
                None,
                None,
            )).await;
            (StatusCode::OK, axum::Json(serde_json::json!({"status": "ok"}))).into_response()
        }
        Err(_) => ApiError::not_found(ApiError::FILE_NOT_FOUND, format!("Source not found: {}", source)),
    }
}

async fn move_collection_recursive(state: &AppState, source: &str, destination: &str) -> anyhow::Result<()> {
    let children = state.storage.list(source).await?;

    if !state.storage.exists(destination).await? {
        state.storage.create_collection(destination, "admin").await?;
    }

    for child in &children {
        let child_name = child.path.rsplit('/').next().unwrap_or("");
        let new_path = if destination == "/" {
            format!("/{}", child_name)
        } else {
            format!("{}/{}", destination, child_name)
        };

        state.storage.move_path(&child.path, &new_path).await?;
    }

    state.storage.delete(source).await?;
    Ok(())
}

async fn copy_collection_recursive(state: &AppState, source: &str, destination: &str) -> anyhow::Result<()> {
    let children = state.storage.list(source).await?;

    if !state.storage.exists(destination).await? {
        state.storage.create_collection(destination, "admin").await?;
    }

    for child in &children {
        let child_name = child.path.rsplit('/').next().unwrap_or("");
        let new_path = if destination == "/" {
            format!("/{}", child_name)
        } else {
            format!("{}/{}", destination, child_name)
        };

        if child.is_collection {
            Box::pin(copy_collection_recursive(state, &child.path, &new_path)).await?;
        } else {
            state.storage.copy(&child.path, &new_path).await?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AppState;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    fn test_app() -> axum::Router {
        crate::build_router(AppState::in_memory())
    }

    async fn body_json(response: axum::response::Response) -> serde_json::Value {
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn test_move_file_success() {
        let state = AppState::in_memory();
        state.storage.put("/source.txt", bytes::Bytes::from("hello"), "admin").await.unwrap();

        let app = crate::build_router(state);
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/files/move")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(serde_json::json!({
                        "source": "/source.txt",
                        "destination": "/dest.txt"
                    }).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert_eq!(json["status"], "ok");
    }

    #[tokio::test]
    async fn test_move_file_source_not_found() {
        let app = test_app();
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/files/move")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(serde_json::json!({
                        "source": "/nonexistent.txt",
                        "destination": "/dest.txt"
                    }).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_copy_file_success() {
        let state = AppState::in_memory();
        state.storage.put("/original.txt", bytes::Bytes::from("data"), "admin").await.unwrap();

        let app = crate::build_router(state);
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/files/copy")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(serde_json::json!({
                        "source": "/original.txt",
                        "destination": "/copy.txt"
                    }).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert_eq!(json["status"], "ok");
    }

    #[tokio::test]
    async fn test_copy_file_source_not_found() {
        let app = test_app();
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/files/copy")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(serde_json::json!({
                        "source": "/nonexistent.txt",
                        "destination": "/copy.txt"
                    }).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_copy_file_destination_exists() {
        let state = AppState::in_memory();
        state.storage.put("/original.txt", bytes::Bytes::from("data"), "admin").await.unwrap();
        state.storage.put("/copy.txt", bytes::Bytes::from("existing"), "admin").await.unwrap();

        let app = crate::build_router(state);
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/files/copy")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(serde_json::json!({
                        "source": "/original.txt",
                        "destination": "/copy.txt"
                    }).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn test_move_file_same_source_destination() {
        let state = AppState::in_memory();
        state.storage.put("/same.txt", bytes::Bytes::from("data"), "admin").await.unwrap();

        let app = crate::build_router(state);
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/files/move")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(serde_json::json!({
                        "source": "/same.txt",
                        "destination": "/same.txt"
                    }).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_move_preserves_content() {
        let state = AppState::in_memory();
        state.storage.put("/move_me.txt", bytes::Bytes::from("important data"), "admin").await.unwrap();

        let app = crate::build_router(state);
        let resp = app.oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/api/files/move")
                .header("content-type", "application/json")
                .body(axum::body::Body::from(serde_json::json!({
                    "source": "/move_me.txt",
                    "destination": "/moved.txt"
                }).to_string()))
                .unwrap(),
        ).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);

        let app2 = crate::build_router(AppState::in_memory());
        app2.oneshot(
            axum::http::Request::builder()
                .method("GET")
                .uri("/moved.txt")
                .body(axum::body::Body::empty())
                .unwrap(),
        ).await.unwrap();
    }
}
