use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

pub struct ApiError;

impl ApiError {
    pub fn respond(status: StatusCode, code: &str, message: impl Into<String>) -> Response {
        let body = axum::Json(serde_json::json!({
            "error": message.into(),
            "error_code": code,
        }));
        (status, body).into_response()
    }

    pub fn bad_request(code: &str, message: impl Into<String>) -> Response {
        Self::respond(StatusCode::BAD_REQUEST, code, message)
    }

    pub fn unauthorized(code: &str, message: impl Into<String>) -> Response {
        Self::respond(StatusCode::UNAUTHORIZED, code, message)
    }

    pub fn forbidden(code: &str, message: impl Into<String>) -> Response {
        Self::respond(StatusCode::FORBIDDEN, code, message)
    }

    pub fn not_found(code: &str, message: impl Into<String>) -> Response {
        Self::respond(StatusCode::NOT_FOUND, code, message)
    }

    pub fn internal(code: &str, message: impl Into<String>) -> Response {
        Self::respond(StatusCode::INTERNAL_SERVER_ERROR, code, message)
    }

    pub fn payload_too_large(code: &str, message: impl Into<String>) -> Response {
        Self::respond(StatusCode::PAYLOAD_TOO_LARGE, code, message)
    }

    pub const AUTH_REQUIRED: &str = "AUTH_REQUIRED";
    pub const FILE_NOT_FOUND: &str = "FILE_NOT_FOUND";
    pub const PATH_INVALID: &str = "PATH_INVALID";
    pub const NOT_ENCRYPTED: &str = "NOT_ENCRYPTED";
    pub const ENCRYPT_FAILED: &str = "ENCRYPT_FAILED";
    pub const DECRYPT_FAILED: &str = "DECRYPT_FAILED";
    pub const INTERNAL_ERROR: &str = "INTERNAL_ERROR";
    pub const INVALID_INPUT: &str = "INVALID_INPUT";
    pub const ADMIN_REQUIRED: &str = "ADMIN_REQUIRED";
    pub const USER_NOT_FOUND: &str = "USER_NOT_FOUND";
    pub const USER_EXISTS: &str = "USER_EXISTS";
    pub const USER_CONFLICT: &str = "USER_CONFLICT";
    pub const USER_CREATE_ERROR: &str = "USER_CREATE_ERROR";
    pub const USER_ERROR: &str = "USER_ERROR";
    pub const PASSWORD_ERROR: &str = "PASSWORD_ERROR";
    pub const GUEST_EXPIRED: &str = "GUEST_EXPIRED";
    pub const NOT_FOUND: &str = "NOT_FOUND";
    pub const API_KEY_NOT_FOUND: &str = "API_KEY_NOT_FOUND";
    pub const API_KEY_QUOTA_EXCEEDED: &str = "API_KEY_QUOTA_EXCEEDED";
    pub const PASSWORD_CHANGE_REQUIRED: &str = "PASSWORD_CHANGE_REQUIRED";

    pub fn conflict(code: &str, message: impl Into<String>) -> Response {
        Self::respond(StatusCode::CONFLICT, code, message)
    }

    pub fn gone(code: &str, message: impl Into<String>) -> Response {
        Self::respond(StatusCode::GONE, code, message)
    }
}
