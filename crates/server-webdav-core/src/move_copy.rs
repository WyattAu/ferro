use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use common::path::normalize_path;
use serde::Deserialize;

use crate::WebDavCoreState;

/// Local API error type for move/copy operations.
#[allow(dead_code)]
pub struct ApiError {
    status: StatusCode,
    code: &'static str,
    message: &'static str,
}

impl ApiError {
    pub const PATH_INVALID: &'static str = "PATH_INVALID";
    pub const BAD_REQUEST: &'static str = "BAD_REQUEST";

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
}

/// Request body for move and copy operations.
#[derive(Debug, Deserialize)]
pub struct MoveCopyRequest {
    pub source: String,
    pub destination: String,
}

/// POST /api/files/move — move a file or collection.
pub async fn move_file<S: WebDavCoreState>(
    State(state): State<S>,
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

    if let Err(e) = state.lock_manager().check_lock_for_write(&source).await {
        return (
            StatusCode::LOCKED,
            axum::Json(serde_json::json!({
                "error": "Locked",
                "detail": e.to_string(),
            })),
        )
            .into_response();
    }
    if let Err(e) = state.lock_manager().check_lock_for_write(&destination).await {
        return (
            StatusCode::LOCKED,
            axum::Json(serde_json::json!({
                "error": "Locked",
                "detail": e.to_string(),
            })),
        )
            .into_response();
    }

    match state.storage().move_path(&source, &destination).await {
        Ok(()) => (StatusCode::OK, axum::Json(serde_json::json!({"status": "ok"}))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(serde_json::json!({
                "error": "MoveFailed",
                "detail": e.to_string(),
            })),
        )
            .into_response(),
    }
}

/// POST /api/files/copy — copy a file or collection.
pub async fn copy_file<S: WebDavCoreState>(
    State(state): State<S>,
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

    match state.storage().copy(&source, &destination).await {
        Ok(()) => (StatusCode::OK, axum::Json(serde_json::json!({"status": "ok"}))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(serde_json::json!({
                "error": "CopyFailed",
                "detail": e.to_string(),
            })),
        )
            .into_response(),
    }
}
