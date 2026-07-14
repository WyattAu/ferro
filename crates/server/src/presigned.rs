use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;

use crate::AppState;
use crate::api_error::ApiError;

/// Query parameters for presigned URL generation.
#[derive(Debug, Deserialize)]
pub struct PresignedParams {
    pub path: String,
    #[serde(default = "default_expires")]
    pub expires: u32,
}

fn default_expires() -> u32 {
    3600
}

/// GET /api/upload-url — generate a presigned upload URL.
pub async fn get_upload_url(State(state): State<AppState>, Query(params): Query<PresignedParams>) -> Response {
    match &state.presigned_generator {
        Some(generator) => match generator.generate_put_url(&params.path, params.expires).await {
            Ok(url) => (
                StatusCode::OK,
                axum::Json(serde_json::json!({
                    "url": url.as_str(),
                    "method": "PUT",
                    "expires_in": params.expires,
                    "path": params.path,
                })),
            )
                .into_response(),
            Err(e) => ApiError::with_details(
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiError::INTERNAL_ERROR,
                "Failed to generate upload URL",
                e.to_string(),
            ),
        },
        None => ApiError::service_unavailable(ApiError::NOT_CONFIGURED, "Pre-signed URLs not configured"),
    }
}

/// GET /api/download-url — generate a presigned download URL.
pub async fn get_download_url(State(state): State<AppState>, Query(params): Query<PresignedParams>) -> Response {
    match &state.presigned_generator {
        Some(generator) => match generator.generate_get_url(&params.path, params.expires).await {
            Ok(url) => (
                StatusCode::OK,
                axum::Json(serde_json::json!({
                    "url": url.as_str(),
                    "method": "GET",
                    "expires_in": params.expires,
                    "path": params.path,
                })),
            )
                .into_response(),
            Err(e) => ApiError::with_details(
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiError::INTERNAL_ERROR,
                "Failed to generate download URL",
                e.to_string(),
            ),
        },
        None => ApiError::service_unavailable(ApiError::NOT_CONFIGURED, "Pre-signed URLs not configured"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::extract::{Query, State};

    fn test_state() -> AppState {
        AppState::in_memory()
    }

    #[allow(dead_code)]
    async fn response_status(resp: Response) -> StatusCode {
        resp.status()
    }

    #[tokio::test]
    async fn test_upload_url_no_generator() {
        let state = test_state();
        let params = PresignedParams {
            path: "/test.txt".to_string(),
            expires: 3600,
        };
        let resp = get_upload_url(State(state), Query(params)).await;
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn test_download_url_no_generator() {
        let state = test_state();
        let params = PresignedParams {
            path: "/test.txt".to_string(),
            expires: 3600,
        };
        let resp = get_download_url(State(state), Query(params)).await;
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[test]
    fn test_presigned_params_default_expires() {
        let json = r#"{"path": "/file.txt"}"#;
        let params: PresignedParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.path, "/file.txt");
        assert_eq!(params.expires, 3600);
    }

    #[test]
    fn test_presigned_params_custom_expires() {
        let json = r#"{"path": "/file.txt", "expires": 600}"#;
        let params: PresignedParams = serde_json::from_str(json).unwrap();
        assert_eq!(params.expires, 600);
    }

    #[test]
    fn test_default_expires_value() {
        assert_eq!(default_expires(), 3600);
    }
}
