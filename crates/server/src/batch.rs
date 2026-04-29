use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;

use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct BatchCopyMoveRequest {
    pub operations: Vec<BatchOperation>,
}

#[derive(Debug, Deserialize)]
pub struct BatchOperation {
    pub from: String,
    pub to: String,
}

pub async fn batch_copy(
    State(state): State<AppState>,
    axum::Json(body): axum::Json<BatchCopyMoveRequest>,
) -> Response {
    let mut results: Vec<serde_json::Value> = Vec::new();

    for op in &body.operations {
        let from = common::path::normalize_path(&op.from);
        let to = common::path::normalize_path(&op.to);

        if !common::path::validate_path(&from) || !common::path::validate_path(&to) {
            results.push(serde_json::json!({
                "from": op.from,
                "to": op.to,
                "status": "error",
                "error": "Invalid path",
            }));
            continue;
        }

        if from == to {
            results.push(serde_json::json!({
                "from": op.from,
                "to": op.to,
                "status": "error",
                "error": "Source and destination are the same",
            }));
            continue;
        }

        match state.storage.head(&from).await {
            Ok(_) => {
                match state.storage.copy(&from, &to).await {
                    Ok(()) => {
                        results.push(serde_json::json!({
                            "from": op.from,
                            "to": op.to,
                            "status": "ok",
                        }));
                    }
                    Err(e) => {
                        results.push(serde_json::json!({
                            "from": op.from,
                            "to": op.to,
                            "status": "error",
                            "error": e.to_string(),
                        }));
                    }
                }
            }
            Err(_) => {
                results.push(serde_json::json!({
                    "from": op.from,
                    "to": op.to,
                    "status": "error",
                    "error": "Source not found",
                }));
            }
        }
    }

    (StatusCode::OK, axum::Json(serde_json::json!({ "results": results }))).into_response()
}

pub async fn batch_move(
    State(state): State<AppState>,
    axum::Json(body): axum::Json<BatchCopyMoveRequest>,
) -> Response {
    let mut results: Vec<serde_json::Value> = Vec::new();

    for op in &body.operations {
        let from = common::path::normalize_path(&op.from);
        let to = common::path::normalize_path(&op.to);

        if !common::path::validate_path(&from) || !common::path::validate_path(&to) {
            results.push(serde_json::json!({
                "from": op.from,
                "to": op.to,
                "status": "error",
                "error": "Invalid path",
            }));
            continue;
        }

        if from == to {
            results.push(serde_json::json!({
                "from": op.from,
                "to": op.to,
                "status": "error",
                "error": "Source and destination are the same",
            }));
            continue;
        }

        match state.storage.head(&from).await {
            Ok(_) => {
                match state.storage.move_path(&from, &to).await {
                    Ok(()) => {
                        results.push(serde_json::json!({
                            "from": op.from,
                            "to": op.to,
                            "status": "ok",
                        }));
                    }
                    Err(e) => {
                        results.push(serde_json::json!({
                            "from": op.from,
                            "to": op.to,
                            "status": "error",
                            "error": e.to_string(),
                        }));
                    }
                }
            }
            Err(_) => {
                results.push(serde_json::json!({
                    "from": op.from,
                    "to": op.to,
                    "status": "error",
                    "error": "Source not found",
                }));
            }
        }
    }

    (StatusCode::OK, axum::Json(serde_json::json!({ "results": results }))).into_response()
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
