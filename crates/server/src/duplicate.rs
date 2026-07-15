use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use tracing::instrument;

use crate::AppState;
use crate::api_error::ApiError;

#[derive(Debug, Deserialize)]
pub struct DuplicateRequest {
    pub path: String,
}

#[instrument(name = "duplicate_file", skip(state))]
pub async fn duplicate_file(
    State(state): State<AppState>,
    axum::Json(body): axum::Json<DuplicateRequest>,
) -> Response {
    let source = body.path.trim_start_matches('/');
    if source.is_empty() {
        return ApiError::bad_request(ApiError::BAD_REQUEST, "Path is required");
    }

    // Check if source file exists
    match state.storage.head(source).await {
        Ok(_) => {}
        Err(e) => {
            return ApiError::not_found(ApiError::NOT_FOUND, format!("Source file not found: {}", e));
        }
    }

    // Generate destination path with " (copy)" suffix
    let destination = format!("{} (copy)", source);

    // Check if destination already exists
    if state.storage.head(&destination).await.is_ok() {
        // Try with incremented copy number
        let mut copy_num = 2;
        loop {
            let dest = format!("{} (copy {})", source, copy_num);
            if state.storage.head(&dest).await.is_err() {
                return perform_copy(&state, source, &dest).await;
            }
            copy_num += 1;
            if copy_num > 100 {
                return ApiError::internal(
                    ApiError::INTERNAL_ERROR,
                    "Too many copies already exist",
                );
            }
        }
    }

    perform_copy(&state, source, &destination).await
}

async fn perform_copy(state: &AppState, source: &str, destination: &str) -> Response {
    match state.storage.copy(source, destination).await {
        Ok(()) => (
            StatusCode::CREATED,
            axum::Json(serde_json::json!({
                "status": "ok",
                "path": destination,
            })),
        )
            .into_response(),
        Err(e) => ApiError::with_details(
            StatusCode::INTERNAL_SERVER_ERROR,
            "COPY_FAILED",
            "Failed to duplicate file",
            e.to_string(),
        ),
    }
}