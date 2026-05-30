use axum::Json;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::Serialize;

/// Structured API error response body.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ApiError {
    pub error: String,
    pub error_code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

impl ApiError {
    /// Build a JSON error response with the given status and code.
    pub fn respond(status: StatusCode, code: &str, message: impl Into<String>) -> Response {
        let body = Json(Self {
            error: message.into(),
            error_code: code.to_string(),
            details: None,
        });
        (status, body).into_response()
    }

    /// Build a JSON error response with additional details.
    pub fn with_details(
        status: StatusCode,
        code: &str,
        message: impl Into<String>,
        details: impl Into<String>,
    ) -> Response {
        let body = Json(Self {
            error: message.into(),
            error_code: code.to_string(),
            details: Some(details.into()),
        });
        (status, body).into_response()
    }

    /// 400 Bad Request.
    pub fn bad_request(code: &str, message: impl Into<String>) -> Response {
        Self::respond(StatusCode::BAD_REQUEST, code, message)
    }

    /// 401 Unauthorized.
    pub fn unauthorized(code: &str, message: impl Into<String>) -> Response {
        Self::respond(StatusCode::UNAUTHORIZED, code, message)
    }

    /// 403 Forbidden.
    pub fn forbidden(code: &str, message: impl Into<String>) -> Response {
        Self::respond(StatusCode::FORBIDDEN, code, message)
    }

    /// 404 Not Found.
    pub fn not_found(code: &str, message: impl Into<String>) -> Response {
        Self::respond(StatusCode::NOT_FOUND, code, message)
    }

    /// 409 Conflict.
    pub fn conflict(code: &str, message: impl Into<String>) -> Response {
        Self::respond(StatusCode::CONFLICT, code, message)
    }

    /// 500 Internal Server Error.
    pub fn internal(code: &str, message: impl Into<String>) -> Response {
        Self::respond(StatusCode::INTERNAL_SERVER_ERROR, code, message)
    }

    /// 501 Not Implemented.
    pub fn not_implemented(code: &str, message: impl Into<String>) -> Response {
        Self::respond(StatusCode::NOT_IMPLEMENTED, code, message)
    }

    /// 413 Payload Too Large.
    pub fn payload_too_large(code: &str, message: impl Into<String>) -> Response {
        Self::respond(StatusCode::PAYLOAD_TOO_LARGE, code, message)
    }

    /// 429 Too Many Requests (includes `Retry-After` header).
    pub fn too_many_requests(code: &str, message: impl Into<String>) -> Response {
        let body = Json(Self {
            error: message.into(),
            error_code: code.to_string(),
            details: None,
        });
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::RETRY_AFTER,
            axum::http::HeaderValue::from_static("60"),
        );
        let mut response = (StatusCode::TOO_MANY_REQUESTS, headers, body).into_response();
        response.headers_mut().insert(
            axum::http::header::CONTENT_TYPE,
            axum::http::HeaderValue::from_static("application/json"),
        );
        response
    }

    /// 503 Service Unavailable.
    pub fn service_unavailable(code: &str, message: impl Into<String>) -> Response {
        Self::respond(StatusCode::SERVICE_UNAVAILABLE, code, message)
    }

    /// 401 with `WWW-Authenticate: Basic` header.
    pub fn unauthorized_with_www_authenticate(code: &str, message: impl Into<String>) -> Response {
        let body = Json(Self {
            error: message.into(),
            error_code: code.to_string(),
            details: None,
        });
        let mut response = (StatusCode::UNAUTHORIZED, body).into_response();
        response.headers_mut().insert(
            axum::http::header::WWW_AUTHENTICATE,
            axum::http::HeaderValue::from_static(r#"Basic realm="Ferro""#),
        );
        response
    }

    /// 410 Gone.
    pub fn gone(code: &str, message: impl Into<String>) -> Response {
        Self::respond(StatusCode::GONE, code, message)
    }

    /// 502 Bad Gateway.
    pub fn bad_gateway(code: &str, message: impl Into<String>) -> Response {
        Self::respond(StatusCode::BAD_GATEWAY, code, message)
    }

    /// 413 Payload Too Large — storage quota exceeded.
    pub fn quota_exceeded(current: u64, limit: u64, requested: u64) -> Response {
        Self::with_details(
            StatusCode::PAYLOAD_TOO_LARGE,
            Self::QUOTA_EXCEEDED,
            "Storage quota exceeded",
            format!(
                "Current usage: {} bytes ({} MB), quota: {} bytes ({} MB), requested: {} bytes ({} MB)",
                current,
                current / 1_048_576,
                limit,
                limit / 1_048_576,
                requested,
                requested / 1_048_576,
            ),
        )
    }

    pub const AUTH_REQUIRED: &'static str = "AUTH_REQUIRED";
    pub const INVALID_CREDENTIALS: &'static str = "INVALID_CREDENTIALS";
    pub const TOKEN_EXPIRED: &'static str = "TOKEN_EXPIRED";
    pub const TOKEN_INVALID: &'static str = "TOKEN_INVALID";

    pub const FILE_NOT_FOUND: &'static str = "FILE_NOT_FOUND";
    pub const FILE_EXISTS: &'static str = "FILE_EXISTS";
    pub const FILE_LOCKED: &'static str = "FILE_LOCKED";
    pub const PATH_INVALID: &'static str = "PATH_INVALID";
    pub const PATH_TRAVERSAL: &'static str = "PATH_TRAVERSAL";

    pub const SHARE_NOT_FOUND: &'static str = "SHARE_NOT_FOUND";
    pub const SHARE_EXPIRED: &'static str = "SHARE_EXPIRED";
    pub const SHARE_PASSWORD_REQUIRED: &'static str = "SHARE_PASSWORD_REQUIRED";
    pub const SHARE_PASSWORD_INVALID: &'static str = "SHARE_PASSWORD_INVALID";

    pub const WASM_INVALID: &'static str = "WASM_INVALID";
    pub const WASM_EXECUTION_FAILED: &'static str = "WASM_EXECUTION_FAILED";
    pub const WASM_TIMEOUT: &'static str = "WASM_TIMEOUT";

    pub const POLICY_DENIED: &'static str = "POLICY_DENIED";
    pub const POLICY_INVALID: &'static str = "POLICY_INVALID";

    pub const RATE_LIMITED: &'static str = "RATE_LIMITED";

    pub const INTERNAL_ERROR: &'static str = "INTERNAL_ERROR";
    pub const NOT_FOUND: &'static str = "NOT_FOUND";
    pub const BAD_REQUEST: &'static str = "BAD_REQUEST";
    pub const CONFLICT: &'static str = "CONFLICT";

    // User management
    pub const USER_NOT_FOUND: &'static str = "USER_NOT_FOUND";
    pub const USER_EXISTS: &'static str = "USER_EXISTS";
    pub const USER_CONFLICT: &'static str = "USER_CONFLICT";
    pub const USER_CREATE_ERROR: &'static str = "USER_CREATE_ERROR";
    pub const USER_ERROR: &'static str = "USER_ERROR";
    pub const ADMIN_REQUIRED: &'static str = "ADMIN_REQUIRED";
    pub const INVALID_INPUT: &'static str = "INVALID_INPUT";
    pub const INVALID_BODY: &'static str = "INVALID_BODY";
    pub const INVALID_JSON: &'static str = "INVALID_JSON";
    pub const MISSING_FIELD: &'static str = "MISSING_FIELD";
    pub const WEAK_PASSWORD: &'static str = "WEAK_PASSWORD";
    pub const PASSWORD_CHANGE_REQUIRED: &'static str = "PASSWORD_CHANGE_REQUIRED";
    pub const PASSWORD_ERROR: &'static str = "PASSWORD_ERROR";

    pub const GUEST_EXPIRED: &'static str = "GUEST_EXPIRED";

    // Configuration / feature flags
    pub const NOT_CONFIGURED: &'static str = "NOT_CONFIGURED";
    pub const MAINTENANCE_MODE: &'static str = "MAINTENANCE_MODE";

    // Trash & snapshots
    pub const TRASH_NOT_FOUND: &'static str = "TRASH_NOT_FOUND";
    pub const SNAPSHOT_NOT_FOUND: &'static str = "SNAPSHOT_NOT_FOUND";

    // Encryption
    pub const NOT_ENCRYPTED: &'static str = "NOT_ENCRYPTED";
    pub const ENCRYPT_FAILED: &'static str = "ENCRYPT_FAILED";
    pub const DECRYPT_FAILED: &'static str = "DECRYPT_FAILED";

    // Generic HTTP helpers
    pub const QUOTA_EXCEEDED: &'static str = "QUOTA_EXCEEDED";
    pub const SERVICE_UNAVAILABLE: &'static str = "SERVICE_UNAVAILABLE";
    pub const NOT_IMPLEMENTED: &'static str = "NOT_IMPLEMENTED";
    pub const PAYLOAD_TOO_LARGE: &'static str = "PAYLOAD_TOO_LARGE";
    pub const BAD_GATEWAY: &'static str = "BAD_GATEWAY";
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use http_body_util::BodyExt;

    async fn body_bytes(response: axum::response::Response) -> bytes::Bytes {
        response.into_body().collect().await.unwrap().to_bytes()
    }

    #[tokio::test]
    async fn test_api_error_json_format() {
        let response = ApiError::not_found(ApiError::FILE_NOT_FOUND, "File not found");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let body = body_bytes(response).await;
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["error"], "File not found");
        assert_eq!(json["error_code"], "FILE_NOT_FOUND");
        assert!(json.get("details").is_none());
    }

    #[tokio::test]
    async fn test_api_error_with_details() {
        let response = ApiError::with_details(
            StatusCode::BAD_REQUEST,
            "CUSTOM",
            "Bad input",
            "field 'x' is missing",
        );
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = body_bytes(response).await;
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["error"], "Bad input");
        assert_eq!(json["error_code"], "CUSTOM");
        assert_eq!(json["details"], "field 'x' is missing");
    }

    #[tokio::test]
    async fn test_bad_request() {
        let response = ApiError::bad_request(ApiError::BAD_REQUEST, "invalid input");
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_unauthorized() {
        let response = ApiError::unauthorized(ApiError::AUTH_REQUIRED, "auth needed");
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_forbidden() {
        let response = ApiError::forbidden(ApiError::POLICY_DENIED, "denied");
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_not_found() {
        let response = ApiError::not_found(ApiError::NOT_FOUND, "gone");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_conflict() {
        let response = ApiError::conflict(ApiError::CONFLICT, "already exists");
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn test_internal() {
        let response = ApiError::internal(ApiError::INTERNAL_ERROR, "oops");
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn test_not_implemented() {
        let response = ApiError::not_implemented("NOT_IMPL", "not ready");
        assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
    }

    #[tokio::test]
    async fn test_payload_too_large() {
        let response = ApiError::payload_too_large("TOO_BIG", "file too big");
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[tokio::test]
    async fn test_too_many_requests_has_retry_after() {
        let response = ApiError::too_many_requests(ApiError::RATE_LIMITED, "Rate limit exceeded");
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        let retry = response.headers().get("retry-after").unwrap();
        assert_eq!(retry, "60");
        let body = body_bytes(response).await;
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["error"], "Rate limit exceeded");
        assert_eq!(json["error_code"], "RATE_LIMITED");
    }

    #[tokio::test]
    async fn test_service_unavailable() {
        let response = ApiError::service_unavailable("UNAVAILABLE", "not ready");
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn test_unauthorized_with_www_authenticate() {
        let response =
            ApiError::unauthorized_with_www_authenticate(ApiError::AUTH_REQUIRED, "auth required");
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let www_auth = response.headers().get("www-authenticate").unwrap();
        assert_eq!(www_auth, r#"Basic realm="Ferro""#);
        let body = body_bytes(response).await;
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["error"], "auth required");
        assert_eq!(json["error_code"], "AUTH_REQUIRED");
    }

    #[tokio::test]
    async fn test_gone() {
        let response = ApiError::gone(ApiError::SHARE_EXPIRED, "expired");
        assert_eq!(response.status(), StatusCode::GONE);
    }

    #[tokio::test]
    async fn test_bad_gateway() {
        let response = ApiError::bad_gateway("BAD_GW", "upstream error");
        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    }

    #[tokio::test]
    async fn test_quota_exceeded() {
        let response = ApiError::quota_exceeded(500, 1000, 600);
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
        let body = body_bytes(response).await;
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["error_code"], "QUOTA_EXCEEDED");
        assert!(json.get("details").is_some());
    }
}
