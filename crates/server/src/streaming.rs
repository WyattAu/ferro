//! Video streaming endpoint with Range header support.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::http::header::HeaderMap;
use axum::response::{IntoResponse, Response};

use crate::AppState;

pub use ferro_server_storage_ops::streaming::{normalize_api_path, stream_video_impl};

/// GET /api/stream/{path} — Stream video with Range header support (206 Partial Content).
pub async fn stream_video(State(state): State<AppState>, Path(path): Path<String>, headers: HeaderMap) -> Response {
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

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::BodyExt;

    async fn body_json(resp: Response) -> serde_json::Value {
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn test_hls_manifest_not_implemented() {
        let state = AppState::in_memory();
        let resp = hls_manifest(State(state), Path("/video.mp4".to_string())).await;
        assert_eq!(resp.status(), StatusCode::NOT_IMPLEMENTED);
        let json = body_json(resp).await;
        assert_eq!(json["error"], "not_implemented");
        assert!(json["message"].as_str().unwrap().contains("HLS"));
    }
}
