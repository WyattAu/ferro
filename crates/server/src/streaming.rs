//! Video streaming endpoint with Range header support.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::http::header::HeaderMap;
use axum::response::{IntoResponse, Response};

use crate::AppState;

pub use ferro_server_storage_ops::streaming::{normalize_api_path, stream_video_impl};

/// GET /api/stream/{path} — Stream video with Range header support (206 Partial Content).
pub async fn stream_video(
    State(state): State<AppState>,
    Path(path): Path<String>,
    headers: HeaderMap,
) -> Response {
    stream_video_impl(&state, path, headers).await
}

/// GET /api/stream/{path}/manifest.m3u8 — HLS manifest stub (returns 501 if no transcoding).
pub async fn hls_manifest(State(_state): State<AppState>, Path(_path): Path<String>) -> Response {
    // HLS transcoding is not yet implemented
    (
        StatusCode::NOT_IMPLEMENTED,
        axum::Json(serde_json::json!({
            "error": "not_implemented",
            "message": "HLS transcoding is not yet available",
        })),
    )
        .into_response()
}
