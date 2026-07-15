use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use chrono::Utc;
use std::collections::HashMap;

use crate::AppState;
use crate::api_error::ApiError;
use ferro_server_state::ServerState;

pub use ferro_server_sharing::shares::{
    CreateShareRequest, SHARE_LOCKOUT_SECS, ShareLink, ShareStore, ShareStoreTrait, hash_share_password,
    verify_share_password,
};

// ---------------------------------------------------------------------------
// Generic _impl functions — testable without Axum
// ---------------------------------------------------------------------------

/// Core logic for listing all active share links.
async fn list_shares_impl<S: ServerState>(state: &S) -> Response {
    let links: Vec<ShareLink> = state.share_store().list().await;
    let items: Vec<serde_json::Value> = links
        .iter()
        .map(|l| {
            serde_json::json!({
                "token": l.token,
                "url": format!("/s/{}", l.token),
                "path": l.path,
                "expires_at": l.expires_at.to_rfc3339(),
                "max_downloads": l.max_downloads,
                "download_count": l.download_count,
                "created_by": l.created_by,
                "allow_download": l.allow_download,
                "allow_upload": l.allow_upload,
            })
        })
        .collect();
    (StatusCode::OK, axum::Json(serde_json::json!({ "shares": items }))).into_response()
}

/// Core logic for deleting a share link by token.
async fn delete_share_impl<S: ServerState>(state: &S, token: &str) -> Response {
    if state.share_store().delete(token).await {
        (StatusCode::NO_CONTENT, "").into_response()
    } else {
        ApiError::not_found(ApiError::SHARE_NOT_FOUND, "Share not found")
    }
}

/// Core logic for getting a single share link by token.
async fn get_share_impl<S: ServerState>(state: &S, token: &str) -> Response {
    match state.share_store().get(token).await {
        Some(link) => (
            StatusCode::OK,
            axum::Json(serde_json::json!({
                "token": link.token,
                "url": format!("/s/{}", link.token),
                "path": link.path,
                "expires_at": link.expires_at.to_rfc3339(),
                "max_downloads": link.max_downloads,
                "download_count": link.download_count,
                "created_by": link.created_by,
                "allow_download": link.allow_download,
                "allow_upload": link.allow_upload,
            })),
        )
            .into_response(),
        None => ApiError::not_found(ApiError::SHARE_NOT_FOUND, "Share not found"),
    }
}

// ---------------------------------------------------------------------------
// Axum handlers (thin wrappers)
// ---------------------------------------------------------------------------

/// Core logic for creating a share link.
async fn create_share_impl<S: ServerState + ferro_server_api_core::ApiCoreState>(
    state: &S,
    req: CreateShareRequest,
) -> Response {
    for component in std::path::Path::new(&req.path).components() {
        match component {
            std::path::Component::ParentDir | std::path::Component::CurDir => {
                return (
                    StatusCode::BAD_REQUEST,
                    axum::Json(serde_json::json!({
                        "error": "invalid_path",
                        "message": "Path traversal detected: '..' and '.' not allowed in share paths",
                    })),
                )
                    .into_response();
            }
            _ => {}
        }
    }

    let link = state.share_store().create(req, "anonymous".to_string()).await;

    crate::event_triggers::fire_event_triggers(
        state,
        crate::event_triggers::EventType::ShareCreated,
        &link.path,
        &link.created_by,
    )
    .await;

    (
        StatusCode::CREATED,
        axum::Json(serde_json::json!({
            "token": link.token,
            "url": format!("/s/{}", link.token),
            "path": link.path,
            "expires_at": link.expires_at.to_rfc3339(),
            "max_downloads": link.max_downloads,
            "allow_download": link.allow_download,
            "allow_upload": link.allow_upload,
        })),
    )
        .into_response()
}

/// Create a new share link.
pub async fn create_share(State(state): State<AppState>, axum::Json(req): axum::Json<CreateShareRequest>) -> Response {
    create_share_impl(&state, req).await
}

/// List all active share links.
pub async fn list_shares(State(state): State<AppState>) -> Response {
    list_shares_impl(&state).await
}

/// Delete a share link by token.
pub async fn delete_share(State(state): State<AppState>, Path(token): Path<String>) -> Response {
    delete_share_impl(&state, &token).await
}

/// Get a single share link by token.
pub async fn get_share(State(state): State<AppState>, Path(token): Path<String>) -> Response {
    get_share_impl(&state, &token).await
}

/// Core logic for serving a shared file by token.
async fn serve_share_impl<S: ServerState>(state: &S, token: &str, params: &HashMap<String, String>) -> Response {
    // Check if this token is temporarily locked due to too many failed attempts
    if state.share_store().is_share_locked(token) {
        return ApiError::with_details(
            StatusCode::TOO_MANY_REQUESTS,
            ApiError::RATE_LIMITED,
            "Too many failed password attempts. Try again later.",
            format!("{} seconds remaining", SHARE_LOCKOUT_SECS),
        );
    }

    let link = match state.share_store().get(token).await {
        Some(l) => l,
        None => return ApiError::not_found(ApiError::SHARE_NOT_FOUND, "Share not found"),
    };

    if link.expires_at < Utc::now() {
        return ApiError::gone(ApiError::SHARE_EXPIRED, "Share expired");
    }

    // Upload-only share: serve drop zone UI
    if link.allow_upload == Some(true) {
        return crate::shares_ext::serve_upload_dropzone(&link);
    }

    if let Some(max) = link.max_downloads
        && link.download_count >= max
    {
        return ApiError::gone(ApiError::SHARE_EXPIRED, "Download limit reached");
    }

    // Check password if set
    if let Some(ref stored_hash) = link.password {
        let provided_password = params.get("password").map(|s| s.as_str());
        match provided_password {
            Some(pw) if verify_share_password(pw, stored_hash) => {
                state.share_store().clear_failed_attempts(token);
            }
            Some(_) => {
                state.share_store().record_failed_attempt(token);
                return ApiError::unauthorized(ApiError::SHARE_PASSWORD_INVALID, "Invalid password");
            }
            None => {
                return ApiError::with_details(
                    StatusCode::UNAUTHORIZED,
                    ApiError::SHARE_PASSWORD_REQUIRED,
                    "Password required",
                    "true",
                );
            }
        }
    }

    let meta = match state.storage().head(&link.path).await {
        Ok(m) => m,
        Err(_) => return ApiError::not_found(ApiError::FILE_NOT_FOUND, "File not found"),
    };

    // Secure view (allow_download=false): serve HTML preview page
    if link.allow_download == Some(false) {
        state.share_store().increment_download(token).await;
        return crate::shares_ext::serve_preview_html(&link, &meta);
    }

    let reader = match state.storage().get_stream(&link.path).await {
        Ok(r) => r,
        Err(_) => return ApiError::not_found(ApiError::FILE_NOT_FOUND, "File not found"),
    };

    state.share_store().increment_download(token).await;

    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        "Content-Type",
        axum::http::HeaderValue::from_str(&meta.mime_type)
            .unwrap_or_else(|_| axum::http::HeaderValue::from_static("application/octet-stream")),
    );
    headers.insert(
        "Content-Length",
        axum::http::HeaderValue::from_str(&meta.size.to_string())
            .unwrap_or_else(|_| axum::http::HeaderValue::from_static("0")),
    );
    headers.insert(
        "Content-Disposition",
        axum::http::HeaderValue::from_str(&format!(
            "inline; filename=\"{}\"",
            link.path.rsplit('/').next().unwrap_or("download")
        ))
        .unwrap_or_else(|_| axum::http::HeaderValue::from_static("inline; filename=\"download\"")),
    );

    let stream = tokio_util::io::ReaderStream::new(reader);
    let body = axum::body::Body::from_stream(stream);

    (StatusCode::OK, headers, body).into_response()
}

/// Serve a shared file by token, enforcing expiration and password.
/// Supports download, secure-view (preview-only), and file-drop (upload) shares.
pub async fn serve_share(
    State(state): State<AppState>,
    Path(token): Path<String>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> Response {
    serve_share_impl(&state, &token, &params).await
}

/// Core logic for handling share upload.
async fn handle_share_upload_impl<S: ServerState>(
    state: &S,
    token: &str,
    mut multipart: axum::extract::Multipart,
) -> Response {
    let link = match state.share_store().get(token).await {
        Some(l) => l,
        None => return ApiError::not_found(ApiError::SHARE_NOT_FOUND, "Share not found"),
    };

    if link.expires_at < chrono::Utc::now() {
        return ApiError::gone(ApiError::SHARE_EXPIRED, "Share expired");
    }

    if link.allow_upload != Some(true) {
        return ApiError::bad_request(ApiError::INVALID_INPUT, "This share link does not accept uploads");
    }

    let field = match multipart.next_field().await {
        Ok(Some(f)) => f,
        Ok(None) => {
            return ApiError::bad_request(ApiError::INVALID_INPUT, "No file field found in upload");
        }
        Err(e) => {
            return ApiError::with_details(
                StatusCode::BAD_REQUEST,
                ApiError::INVALID_INPUT,
                "Invalid multipart data",
                e.to_string(),
            );
        }
    };

    let file_name = match field.file_name() {
        Some(name) if !name.is_empty() => crate::shares_ext::sanitize_filename(name),
        _ => format!("upload_{}", uuid::Uuid::new_v4()),
    };

    let bytes = match field.bytes().await {
        Ok(b) => b,
        Err(e) => {
            return ApiError::with_details(
                StatusCode::PAYLOAD_TOO_LARGE,
                ApiError::PAYLOAD_TOO_LARGE,
                "Failed to read upload",
                e.to_string(),
            );
        }
    };

    if bytes.len() > state.max_body_size() as usize {
        return ApiError::with_details(
            StatusCode::PAYLOAD_TOO_LARGE,
            ApiError::PAYLOAD_TOO_LARGE,
            "Upload exceeds size limit",
            format!("max {} bytes", state.max_body_size()),
        );
    }

    let target_path = format!("{}/{}", link.path.trim_end_matches('/'), file_name);

    if state.storage().head(&link.path).await.is_err()
        && let Err(e) = state.storage().create_collection(&link.path, "anonymous").await
    {
        tracing::warn!(error = %e, path = %link.path, "failed to create upload target directory");
        return ApiError::internal(ApiError::INTERNAL_ERROR, "Failed to create upload directory");
    }

    let content_type = crate::shares_ext::sniff_mime_type(&file_name);
    if let Err(e) = state.storage().put(&target_path, bytes.clone(), "anonymous").await {
        tracing::warn!(error = %e, path = %target_path, "failed to store uploaded file");
        return ApiError::internal(ApiError::INTERNAL_ERROR, "Failed to store uploaded file");
    }

    state
        .audit_log()
        .log(crate::state::traits::convert_entry(crate::audit::build_audit_entry(
            "POST",
            &format!("/s/{}", token),
            "anonymous",
            201,
            None,
            None,
        )))
        .await;

    if let Some(db) = state.db() {
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        let upload_id = uuid::Uuid::new_v4().to_string();
        if let Err(e) = conn.execute(
            "INSERT INTO share_uploads (id, share_token, file_path, size, mime_type, uploaded_by) VALUES (?1, ?2, ?3, ?4, ?5, 'anonymous')",
            rusqlite::params![upload_id, token, target_path, bytes.len() as i64, content_type],
        ) {
            tracing::warn!(error = %e, "failed to record share upload");
        }
    }

    (
        StatusCode::CREATED,
        axum::Json(serde_json::json!({
            "path": target_path,
            "size": bytes.len(),
            "content_type": content_type,
        })),
    )
        .into_response()
}

/// `POST /s/:token` -- Upload a file to a file-drop (upload-only) share via multipart form.
pub async fn handle_share_upload(
    State(state): State<AppState>,
    Path(token): Path<String>,
    multipart: axum::extract::Multipart,
) -> Response {
    handle_share_upload_impl(&state, &token, multipart).await
}
