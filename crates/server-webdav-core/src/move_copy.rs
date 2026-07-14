use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use common::path::normalize_path;
use serde::Deserialize;

use crate::WebDavCoreState;
use ferro_server_security_middleware::api_error::ApiError;

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
        return ApiError::with_details(StatusCode::LOCKED, "FILE_LOCKED", "Locked", e.to_string());
    }
    if let Err(e) = state.lock_manager().check_lock_for_write(&destination).await {
        return ApiError::with_details(StatusCode::LOCKED, "FILE_LOCKED", "Locked", e.to_string());
    }

    match state.storage().move_path(&source, &destination).await {
        Ok(()) => (StatusCode::OK, axum::Json(serde_json::json!({"status": "ok"}))).into_response(),
        Err(e) => ApiError::with_details(StatusCode::INTERNAL_SERVER_ERROR, "MOVE_FAILED", "Move failed", e.to_string()),
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
        Err(e) => ApiError::with_details(StatusCode::INTERNAL_SERVER_ERROR, "COPY_FAILED", "Copy failed", e.to_string()),
    }
}
