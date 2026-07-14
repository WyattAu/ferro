//! Generic REST API handlers for file operations.
//!
//! These handlers are generic over the `HasStorage` trait and can be used
//! with any state type that implements it.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use common::server_context::HasStorage;
use serde::{Deserialize, Serialize};

pub use crate::streaming::normalize_api_path;

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ListFilesResponse {
    pub entries: Vec<FileEntryJson>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct PutFileResponse {
    pub path: String,
    pub size: u64,
    pub etag: String,
    pub content_hash: String,
    pub created_at: String,
    pub modified_at: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct MkdirResponse {
    pub path: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct CopyMoveResponse {
    #[serde(rename = "from")]
    pub from_path: String,
    #[serde(rename = "to")]
    pub to_path: String,
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct ListFilesParams {
    pub path: Option<String>,
    pub depth: Option<u32>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct FileEntryJson {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub is_collection: bool,
    pub mime_type: String,
    pub etag: String,
    pub content_hash: String,
    pub modified_at: String,
    pub created_at: String,
}

/// GET /api/v1/files — JSON file listing (alternative to `WebDAV` PROPFIND).
pub async fn list_files_impl<S: HasStorage>(state: &S, params: &ListFilesParams) -> Response {
    let path = params.path.as_deref().unwrap_or("/").trim_matches('/');
    let normalized = if path.is_empty() { "/" } else { &format!("/{path}") };
    let depth = params.depth.unwrap_or(1);

    if normalized == "/" {
        let _ = state.storage().head("/").await;
    } else {
        match state.storage().head(normalized).await {
            Ok(meta) if meta.is_collection => {}
            Ok(_) => {
                return (
                    StatusCode::CONFLICT,
                    axum::Json(serde_json::json!({
                        "error": "not_a_collection",
                        "message": format!("{} is not a directory", normalized),
                    })),
                )
                    .into_response();
            }
            Err(e) => {
                return (
                    StatusCode::NOT_FOUND,
                    axum::Json(serde_json::json!({
                        "error": "not_found",
                        "message": e.to_string(),
                    })),
                )
                    .into_response();
            }
        }
    }

    let entries = if depth == 0 {
        vec![]
    } else {
        match state.storage().list(normalized).await {
            Ok(items) => items,
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    axum::Json(serde_json::json!({
                        "error": "list_failed",
                        "message": e.to_string(),
                    })),
                )
                    .into_response();
            }
        }
    };

    let json_entries: Vec<FileEntryJson> = entries
        .into_iter()
        .map(|m| {
            let name = m.path.rsplit('/').next().unwrap_or(&m.path).to_string();
            FileEntryJson {
                name,
                path: m.path,
                size: m.size,
                is_collection: m.is_collection,
                mime_type: m.mime_type,
                etag: m.etag,
                content_hash: m.content_hash.as_str().to_string(),
                modified_at: m.modified_at.to_rfc3339(),
                created_at: m.created_at.to_rfc3339(),
            }
        })
        .collect();

    (StatusCode::OK, axum::Json(ListFilesResponse { entries: json_entries })).into_response()
}

pub async fn mkdir_impl<S: HasStorage>(state: &S, path: &str) -> Response {
    let path = match normalize_api_path(path) {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                axum::Json(serde_json::json!({
                    "error": "invalid_path", "message": e,
                })),
            )
                .into_response();
        }
    };

    if let Err(reason) = ferro_server_security::security::validate_path(&path) {
        return (
            StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({
                "error": "invalid_path", "message": reason,
            })),
        )
            .into_response();
    }

    if ferro_server_security::security::contains_html(&path) {
        return (
            StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({
                "error": "invalid_path",
                "message": "Path contains HTML content, which is not permitted",
            })),
        )
            .into_response();
    }

    let owner = "anonymous".to_string();

    match state.storage().create_collection(&path, &owner).await {
        Ok(meta) => {
            let location = meta.path.clone();
            (
                StatusCode::CREATED,
                [(axum::http::header::LOCATION, location)],
                axum::Json(MkdirResponse {
                    path: meta.path,
                    created_at: meta.created_at.to_rfc3339(),
                }),
            )
                .into_response()
        }
        Err(e) => {
            let status = if e.to_string().contains("exists") {
                StatusCode::CONFLICT
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (
                status,
                axum::Json(serde_json::json!({
                    "error": "mkdir_failed",
                    "message": e.to_string(),
                })),
            )
                .into_response()
        }
    }
}

pub async fn copy_file_impl<S: HasStorage>(state: &S, from: &str, to: &str) -> Response {
    match state.storage().copy(from, to).await {
        Ok(()) => (
            StatusCode::CREATED,
            axum::Json(CopyMoveResponse {
                from_path: from.to_string(),
                to_path: to.to_string(),
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::NOT_FOUND,
            axum::Json(serde_json::json!({
                "error": "copy_failed",
                "message": e.to_string(),
            })),
        )
            .into_response(),
    }
}

pub async fn move_file_rest_impl<S: HasStorage>(state: &S, from: &str, to: &str) -> Response {
    match state.storage().move_path(from, to).await {
        Ok(()) => (
            StatusCode::CREATED,
            axum::Json(CopyMoveResponse {
                from_path: from.to_string(),
                to_path: to.to_string(),
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::NOT_FOUND,
            axum::Json(serde_json::json!({
                "error": "move_failed",
                "message": e.to_string(),
            })),
        )
            .into_response(),
    }
}
