use axum::extract::{Extension, Path, Query};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use tracing::warn;

pub use ferro_server_collaboration::comments::{
    Comment, CommentStore, CreateCommentRequest, ListCommentsQuery, UpdateCommentRequest,
};

use crate::SharingState;
use crate::api_error::ApiError;
use crate::audit;
use crate::security;

fn get_user_id(req: &axum::http::Request<axum::body::Body>) -> String {
    req.headers()
        .get("X-Ferro-User")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("anonymous")
        .to_string()
}

fn is_admin(state: &SharingState, user_id: &str) -> bool {
    state.admin_user.as_deref() == Some(user_id)
}

async fn log_comment_audit(state: &SharingState, method: &str, path: &str, user: &str) {
    state
        .audit_log
        .log_audit(audit::build_audit_entry(method, path, user, 200, None, None))
        .await;
}

pub async fn list_comments_handler(
    Extension(state): Extension<SharingState>,
    Query(params): Query<ListCommentsQuery>,
) -> Response {
    let user_id = "anonymous".to_string();

    if state.comments.db.is_none() {
        return ApiError::service_unavailable(
            ApiError::NOT_CONFIGURED,
            "Comments require SQLite database (--data-dir)",
        );
    }

    match state.comments.list_comments(&params.path) {
        Ok(comments) => {
            state
                .audit_log
                .log_audit(audit::build_audit_entry(
                    "GET",
                    &format!("/api/comments?path={}", params.path),
                    &user_id,
                    200,
                    None,
                    None,
                ))
                .await;
            (StatusCode::OK, axum::Json(serde_json::json!({ "comments": comments }))).into_response()
        }
        Err(e) => {
            warn!(error = %e, "Failed to list comments");
            ApiError::internal(ApiError::INTERNAL_ERROR, "Failed to list comments")
        }
    }
}

pub async fn create_comment_handler(
    Extension(state): Extension<SharingState>,
    req: axum::http::Request<axum::body::Body>,
) -> Response {
    let user_id = get_user_id(&req);

    if state.comments.db.is_none() {
        return ApiError::service_unavailable(
            ApiError::NOT_CONFIGURED,
            "Comments require SQLite database (--data-dir)",
        );
    }

    let (_parts, body) = req.into_parts();
    let body_bytes = match axum::body::to_bytes(body, 1024 * 1024).await {
        Ok(b) => b,
        Err(_) => {
            return ApiError::bad_request(ApiError::INVALID_BODY, "Failed to read request body");
        }
    };

    if body_bytes.len() > 1024 * 1024 {
        return ApiError::bad_request(ApiError::INVALID_BODY, "Request body too large");
    }

    let request: CreateCommentRequest = match serde_json::from_slice(&body_bytes) {
        Ok(r) => r,
        Err(e) => {
            return ApiError::bad_request(ApiError::INVALID_JSON, format!("Invalid JSON: {}", e));
        }
    };

    if !common::path::validate_path(&request.path) {
        return ApiError::bad_request(ApiError::PATH_INVALID, "Invalid path: path traversal is not allowed");
    }

    let normalized_path = common::path::normalize_path(&request.path);

    let sanitized_body = security::sanitize_control_chars(&request.body);
    if security::contains_html(&sanitized_body) {
        return ApiError::bad_request(
            ApiError::BAD_REQUEST,
            "Comment body contains HTML content, which is not permitted",
        );
    }

    match state.comments.add_comment(
        &normalized_path,
        &user_id,
        &sanitized_body,
        request.parent_id.as_deref(),
    ) {
        Ok(comment) => {
            log_comment_audit(
                &state,
                "POST",
                &format!("/api/comments (path={})", normalized_path),
                &user_id,
            )
            .await;
            (StatusCode::CREATED, axum::Json(comment)).into_response()
        }
        Err(e) => ApiError::bad_request(ApiError::BAD_REQUEST, e),
    }
}

pub async fn update_comment_handler(
    Extension(state): Extension<SharingState>,
    Path(id): Path<String>,
    req: axum::http::Request<axum::body::Body>,
) -> Response {
    let user_id = get_user_id(&req);

    if state.comments.db.is_none() {
        return ApiError::service_unavailable(
            ApiError::NOT_CONFIGURED,
            "Comments require SQLite database (--data-dir)",
        );
    }

    let (_parts, body) = req.into_parts();
    let body_bytes = match axum::body::to_bytes(body, 1024 * 1024).await {
        Ok(b) => b,
        Err(_) => {
            return ApiError::bad_request(ApiError::INVALID_BODY, "Failed to read request body");
        }
    };

    if body_bytes.len() > 1024 * 1024 {
        return ApiError::bad_request(ApiError::INVALID_BODY, "Request body too large");
    }

    let request: UpdateCommentRequest = match serde_json::from_slice(&body_bytes) {
        Ok(r) => r,
        Err(e) => {
            return ApiError::bad_request(ApiError::INVALID_JSON, format!("Invalid JSON: {}", e));
        }
    };

    match state.comments.update_comment(&id, &user_id, &request.body) {
        Ok(comment) => {
            log_comment_audit(&state, "PUT", &format!("/api/comments/{}", id), &user_id).await;
            (StatusCode::OK, axum::Json(comment)).into_response()
        }
        Err(e) => {
            if e.contains("not found") {
                ApiError::not_found(ApiError::NOT_FOUND, e)
            } else if e.contains("Permission denied") {
                ApiError::forbidden(ApiError::POLICY_DENIED, e)
            } else {
                ApiError::bad_request(ApiError::BAD_REQUEST, e)
            }
        }
    }
}

pub async fn delete_comment_handler(
    Extension(state): Extension<SharingState>,
    Path(id): Path<String>,
    req: axum::http::Request<axum::body::Body>,
) -> Response {
    let user_id = get_user_id(&req);
    let admin = is_admin(&state, &user_id);

    if state.comments.db.is_none() {
        return ApiError::service_unavailable(
            ApiError::NOT_CONFIGURED,
            "Comments require SQLite database (--data-dir)",
        );
    }

    match state.comments.delete_comment(&id, &user_id, admin) {
        Ok(()) => {
            log_comment_audit(&state, "DELETE", &format!("/api/comments/{}", id), &user_id).await;
            (StatusCode::NO_CONTENT, "").into_response()
        }
        Err(e) => {
            if e.contains("not found") {
                ApiError::not_found(ApiError::NOT_FOUND, e)
            } else if e.contains("Permission denied") {
                ApiError::forbidden(ApiError::POLICY_DENIED, e)
            } else {
                ApiError::internal(ApiError::INTERNAL_ERROR, e)
            }
        }
    }
}

pub async fn resolve_comment_handler(
    Extension(state): Extension<SharingState>,
    Path(id): Path<String>,
    req: axum::http::Request<axum::body::Body>,
) -> Response {
    let user_id = get_user_id(&req);

    if state.comments.db.is_none() {
        return ApiError::service_unavailable(
            ApiError::NOT_CONFIGURED,
            "Comments require SQLite database (--data-dir)",
        );
    }

    match state.comments.resolve_comment(&id, &user_id) {
        Ok(comment) => {
            log_comment_audit(&state, "POST", &format!("/api/comments/{}/resolve", id), &user_id).await;
            (StatusCode::OK, axum::Json(comment)).into_response()
        }
        Err(e) => {
            if e.contains("not found") {
                ApiError::not_found(ApiError::NOT_FOUND, e)
            } else if e.contains("Permission denied") {
                ApiError::forbidden(ApiError::POLICY_DENIED, e)
            } else {
                ApiError::internal(ApiError::INTERNAL_ERROR, e)
            }
        }
    }
}
