use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;

use crate::AppState;

pub use ferro_server_storage_ops::batch::{
    BatchCopyMoveRequest, BatchOperation, batch_copy_impl, batch_move_impl,
};

pub async fn batch_copy(
    State(state): State<AppState>,
    axum::Json(body): axum::Json<BatchCopyMoveRequest>,
) -> Response {
    batch_copy_impl(&state, &body.operations).await
}

pub async fn batch_move(
    State(state): State<AppState>,
    axum::Json(body): axum::Json<BatchCopyMoveRequest>,
) -> Response {
    batch_move_impl(&state, &body.operations).await
}

#[derive(Debug, Deserialize)]
pub struct BatchDeleteRequest {
    pub paths: Vec<String>,
}

pub async fn batch_delete(
    State(state): State<AppState>,
    axum::Json(body): axum::Json<BatchDeleteRequest>,
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

    (
        StatusCode::OK,
        axum::Json(serde_json::json!({
            "succeeded": succeeded,
            "failed": failed,
            "total_requested": body.paths.len(),
        })),
    )
        .into_response()
}

#[derive(Debug, Deserialize)]
pub struct BatchShareRequest {
    pub paths: Vec<String>,
    pub permissions: String,
    pub expiry: Option<i64>,
}

pub async fn batch_share(
    State(state): State<AppState>,
    axum::Json(body): axum::Json<BatchShareRequest>,
) -> Response {
    let mut results: Vec<serde_json::Value> = Vec::new();

    for path in &body.paths {
        let normalized = common::path::normalize_path(path);

        if !common::path::validate_path(&normalized) {
            results.push(serde_json::json!({
                "path": path,
                "status": "error",
                "error": "Invalid path",
            }));
            continue;
        }

        let req = crate::shares::CreateShareRequest {
            path: normalized.clone(),
            password: None,
            expires_in_hours: body.expiry,
            max_downloads: None,
            allow_download: Some(true),
            allow_upload: None,
        };

        let share = state.share_store.create(req, "batch".to_string()).await;
        results.push(serde_json::json!({
            "path": path,
            "status": "ok",
            "token": share.token,
            "permissions": body.permissions,
        }));
    }

    (
        StatusCode::OK,
        axum::Json(serde_json::json!({ "results": results })),
    )
        .into_response()
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
    async fn test_batch_copy_empty_operations() {
        let app = test_app();
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/batch/copy")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(
                        serde_json::json!({ "operations": [] }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert_eq!(json["results"].as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_batch_move_empty_operations() {
        let app = test_app();
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/batch/move")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(
                        serde_json::json!({ "operations": [] }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert_eq!(json["results"].as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_batch_copy_success() {
        let state = AppState::in_memory();
        state
            .storage
            .put("/a.txt", bytes::Bytes::from("a"), "admin")
            .await
            .unwrap();
        state
            .storage
            .put("/b.txt", bytes::Bytes::from("b"), "admin")
            .await
            .unwrap();

        let app = crate::build_router(state);
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/batch/copy")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(
                        serde_json::json!({
                            "operations": [
                                {"from": "/a.txt", "to": "/dest/a.txt"},
                                {"from": "/b.txt", "to": "/dest/b.txt"}
                            ]
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        let results = json["results"].as_array().unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0]["status"], "ok");
        assert_eq!(results[1]["status"], "ok");
    }

    #[tokio::test]
    async fn test_batch_move_success() {
        let state = AppState::in_memory();
        state
            .storage
            .put("/a.txt", bytes::Bytes::from("a"), "admin")
            .await
            .unwrap();

        let app = crate::build_router(state);
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/batch/move")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(
                        serde_json::json!({
                            "operations": [
                                {"from": "/a.txt", "to": "/moved/a.txt"}
                            ]
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        let results = json["results"].as_array().unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0]["status"], "ok");
    }

    #[tokio::test]
    async fn test_batch_copy_partial_failure() {
        let state = AppState::in_memory();
        state
            .storage
            .put("/exists.txt", bytes::Bytes::from("data"), "admin")
            .await
            .unwrap();

        let app = crate::build_router(state);
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/batch/copy")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(
                        serde_json::json!({
                            "operations": [
                                {"from": "/exists.txt", "to": "/dest/exists.txt"},
                                {"from": "/missing.txt", "to": "/dest/missing.txt"}
                            ]
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        let results = json["results"].as_array().unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0]["status"], "ok");
        assert_eq!(results[1]["status"], "error");
    }
}
