use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use crate::AppState;
use crate::api_error::ApiError;
use ferro_server_api_core::file_requests::CreateFileRequest;

// ---------------------------------------------------------------------------
// File Request handlers
// ---------------------------------------------------------------------------

/// Create a new file request (upload-only share link).
pub async fn create_file_request(
    State(state): State<AppState>,
    axum::Json(req): axum::Json<CreateFileRequest>,
) -> Response {
    let created_by = state
        .admin_user
        .clone()
        .unwrap_or_else(|| "anonymous".to_string());

    // Validate path
    for component in std::path::Path::new(&req.path).components() {
        match component {
            std::path::Component::ParentDir | std::path::Component::CurDir => {
                return (
                    StatusCode::BAD_REQUEST,
                    axum::Json(serde_json::json!({
                        "error": "invalid_path",
                        "message": "Path traversal detected: '..' and '.' not allowed in file request paths",
                    })),
                )
                    .into_response();
            }
            _ => {}
        }
    }

    let file_request = state.file_request_store.create(req, created_by).await;

    // Create an upload-only share link for this file request
    let share_req = ferro_server_sharing::shares::CreateShareRequest {
        path: file_request.path.clone(),
        password: None,
        expires_in_hours: None,
        max_downloads: None,
        allow_download: Some(false),
        allow_upload: Some(true),
    };
    let share = state.share_store.create(share_req, "system".to_string()).await;

    (
        StatusCode::CREATED,
        axum::Json(serde_json::json!({
            "id": file_request.id,
            "path": file_request.path,
            "message": file_request.message,
            "expires_at": file_request.expires_at.map(|e| e.to_rfc3339()),
            "max_uploads": file_request.max_uploads,
            "upload_count": file_request.upload_count,
            "created_by": file_request.created_by,
            "token": file_request.token,
            "share_url": format!("/s/{}", share.token),
        })),
    )
        .into_response()
}

/// List all active file requests.
pub async fn list_file_requests(State(state): State<AppState>) -> Response {
    let requests = state.file_request_store.list().await;
    let items: Vec<serde_json::Value> = requests
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.id,
                "path": r.path,
                "message": r.message,
                "expires_at": r.expires_at.map(|e| e.to_rfc3339()),
                "max_uploads": r.max_uploads,
                "upload_count": r.upload_count,
                "created_by": r.created_by,
                "token": r.token,
            })
        })
        .collect();
    (
        StatusCode::OK,
        axum::Json(serde_json::json!({ "file_requests": items })),
    )
        .into_response()
}

/// Delete a file request by ID.
pub async fn delete_file_request(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Response {
    if state.file_request_store.delete(&id).await {
        (StatusCode::NO_CONTENT, "").into_response()
    } else {
        ApiError::not_found(ApiError::SHARE_NOT_FOUND, "File request not found")
    }
}
