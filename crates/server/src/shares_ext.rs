//! Extended share types: secure view, file drop, upload shares.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::AppState;
use crate::api_error::ApiError;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// The type of share link.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ShareType {
    Download,
    Upload,
    View,
}

impl Default for ShareType {
    fn default() -> Self {
        Self::Download
    }
}

impl std::fmt::Display for ShareType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Download => write!(f, "download"),
            Self::Upload => write!(f, "upload"),
            Self::View => write!(f, "view"),
        }
    }
}

impl ShareType {
    pub fn from_str_opt(s: &str) -> Self {
        match s {
            "upload" => Self::Upload,
            "view" => Self::View,
            _ => Self::Download,
        }
    }
}

/// Extended request body for creating a new share link.
#[derive(Debug, Deserialize)]
pub struct CreateShareRequestExt {
    pub path: String,
    pub password: Option<String>,
    pub expires_in_hours: Option<i64>,
    pub max_downloads: Option<u32>,
    #[serde(default)]
    pub share_type: ShareType,
    #[serde(default = "default_true")]
    pub allow_download: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize)]
pub struct ShareUploadEntry {
    pub id: String,
    pub file_name: String,
    pub size: u64,
    pub mime_type: String,
    pub uploaded_at: String,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// `POST /api/shares/ext` -- Create a share link with extended type support.
pub async fn create_share_ext(
    State(state): State<AppState>,
    axum::Json(req): axum::Json<CreateShareRequestExt>,
) -> Response {
    use crate::shares::CreateShareRequest;

    if req.share_type == ShareType::Upload {
        if let Ok(meta) = state.storage.head(&req.path).await {
            if !meta.is_collection {
                return ApiError::bad_request(
                    ApiError::INVALID_INPUT,
                    "Upload share target must be a directory",
                );
            }
        }
    }

    if req.share_type == ShareType::View {
        if let Ok(meta) = state.storage.head(&req.path).await {
            if meta.is_collection {
                return ApiError::bad_request(
                    ApiError::INVALID_INPUT,
                    "View share target must be a file, not a directory",
                );
            }
        }
    }

    let base_req = CreateShareRequest {
        path: req.path.clone(),
        password: req.password.clone(),
        expires_in_hours: req.expires_in_hours,
        max_downloads: req.max_downloads,
    };
    let link = state
        .share_store
        .create(base_req, "anonymous".to_string())
        .await;

    if let Some(ref db) = state.db {
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        if let Err(e) = conn.execute(
            "UPDATE shares SET share_type = ?1, allow_download = ?2, upload_target = ?3 WHERE token = ?4",
            params![
                req.share_type.to_string(),
                if req.allow_download { 1i32 } else { 0i32 },
                if req.share_type == ShareType::Upload {
                    Some(req.path.clone())
                } else {
                    None::<String>
                },
                link.token,
            ],
        ) {
            tracing::warn!(error = %e, "failed to persist extended share fields");
        }
    }

    (
        StatusCode::CREATED,
        axum::Json(serde_json::json!({
            "token": link.token,
            "url": format!("/s/{}", link.token),
            "path": link.path,
            "share_type": req.share_type.to_string(),
            "expires_at": link.expires_at.to_rfc3339(),
            "max_downloads": link.max_downloads,
            "allow_download": req.allow_download,
        })),
    )
        .into_response()
}

/// `POST /s/:token/upload` -- Upload a file to an upload-only share.
pub async fn upload_to_share(
    State(state): State<AppState>,
    Path(token): Path<String>,
    body: axum::body::Body,
) -> Response {
    let link = match state.share_store.get(&token).await {
        Some(l) => l,
        None => return ApiError::not_found(ApiError::SHARE_NOT_FOUND, "Share not found"),
    };

    if link.expires_at < chrono::Utc::now() {
        return ApiError::gone(ApiError::SHARE_EXPIRED, "Share expired");
    }

    let share_type = get_share_type(&state, &token);
    if share_type != ShareType::Upload {
        return ApiError::bad_request(
            ApiError::INVALID_INPUT,
            "This share link does not accept uploads",
        );
    }

    let bytes = match axum::body::to_bytes(body, state.max_body_size as usize).await {
        Ok(b) => b,
        Err(e) => {
            return ApiError::with_details(
                StatusCode::PAYLOAD_TOO_LARGE,
                ApiError::PAYLOAD_TOO_LARGE,
                "Upload too large",
                e.to_string(),
            );
        }
    };

    let file_name = format!("upload_{}", uuid::Uuid::new_v4());
    let target_path = format!("{}/{}", link.path.trim_end_matches('/'), file_name);

    if let Err(_) = state.storage.head(&link.path).await {
        if let Err(e) = state
            .storage
            .create_collection(&link.path, "anonymous")
            .await
        {
            tracing::warn!(error = %e, path = %link.path, "failed to create upload target directory");
            return ApiError::internal(
                ApiError::INTERNAL_ERROR,
                "Failed to create upload directory",
            );
        }
    }

    let content_type = sniff_mime_type(&file_name);
    if let Err(e) = state
        .storage
        .put(&target_path, bytes.clone().into(), "anonymous")
        .await
    {
        tracing::warn!(error = %e, path = %target_path, "failed to store uploaded file");
        return ApiError::internal(ApiError::INTERNAL_ERROR, "Failed to store uploaded file");
    }

    if let Some(ref db) = state.db {
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        let upload_id = uuid::Uuid::new_v4().to_string();
        if let Err(e) = conn.execute(
            "INSERT INTO share_uploads (id, share_token, file_path, size, mime_type, uploaded_by) VALUES (?1, ?2, ?3, ?4, ?5, 'anonymous')",
            params![upload_id, token, target_path, bytes.len() as i64, content_type],
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

/// `GET /s/:token/uploads` -- List uploads for an upload share.
pub async fn list_share_uploads(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Response {
    let entries = if let Some(ref db) = state.db {
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = match conn.prepare(
            "SELECT id, file_path, size, mime_type, uploaded_at FROM share_uploads WHERE share_token = ?1 ORDER BY uploaded_at DESC",
        ) {
            Ok(s) => s,
            Err(_) => {
                return (
                    StatusCode::OK,
                    axum::Json(serde_json::json!({ "uploads": [] })),
                )
                    .into_response();
            }
        };
        let mut entries = Vec::new();
        if let Ok(rows) = stmt.query_map(params![token], |row| {
            Ok(ShareUploadEntry {
                id: row.get(0)?,
                file_name: row
                    .get::<_, String>(1)?
                    .rsplit('/')
                    .next()
                    .unwrap_or("unknown")
                    .to_string(),
                size: row.get::<_, i64>(2)? as u64,
                mime_type: row.get(3)?,
                uploaded_at: row.get(4)?,
            })
        }) {
            for row in rows.flatten() {
                entries.push(row);
            }
        }
        entries
    } else {
        Vec::new()
    };

    (
        StatusCode::OK,
        axum::Json(serde_json::json!({ "uploads": entries })),
    )
        .into_response()
}

/// `GET /s/:token/view` -- Serve a secure view share (preview only).
pub async fn serve_view_share(
    State(state): State<AppState>,
    Path(token): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    if state.share_store.is_share_locked(&token) {
        return ApiError::with_details(
            StatusCode::TOO_MANY_REQUESTS,
            ApiError::RATE_LIMITED,
            "Too many failed password attempts. Try again later.",
            format!("{} seconds remaining", 300),
        );
    }

    let link = match state.share_store.get(&token).await {
        Some(l) => l,
        None => return ApiError::not_found(ApiError::SHARE_NOT_FOUND, "Share not found"),
    };

    if link.expires_at < chrono::Utc::now() {
        return ApiError::gone(ApiError::SHARE_EXPIRED, "Share expired");
    }

    let share_type = get_share_type(&state, &token);
    if share_type != ShareType::View {
        return ApiError::bad_request(
            ApiError::INVALID_INPUT,
            "This share link is not a view share",
        );
    }

    if let Some(ref required_password) = link.password {
        let provided = params.get("password").map(|s| s.as_str());
        match provided {
            Some(pw) if constant_time_eq(pw, required_password) => {
                state.share_store.clear_failed_attempts(&token);
            }
            Some(_) => {
                state.share_store.record_failed_attempt(&token);
                return ApiError::unauthorized(
                    ApiError::SHARE_PASSWORD_INVALID,
                    "Invalid password",
                );
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

    let meta = match state.storage.head(&link.path).await {
        Ok(m) => m,
        Err(_) => return ApiError::not_found(ApiError::FILE_NOT_FOUND, "File not found"),
    };

    if meta.is_collection {
        return ApiError::bad_request(
            ApiError::INVALID_INPUT,
            "View share target must be a file, not a directory",
        );
    }

    let allow_download = get_allow_download(&state, &token);

    let reader = match state.storage.get_stream(&link.path).await {
        Ok(r) => r,
        Err(_) => return ApiError::not_found(ApiError::FILE_NOT_FOUND, "File not found"),
    };

    state.share_store.increment_download(&token).await;

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

    if allow_download {
        headers.insert(
            "Content-Disposition",
            axum::http::HeaderValue::from_str(&format!(
                "inline; filename=\"{}\"",
                link.path.rsplit('/').next().unwrap_or("preview")
            ))
            .unwrap_or_else(|_| {
                axum::http::HeaderValue::from_static("inline; filename=\"preview\"")
            }),
        );
    } else {
        headers.insert(
            "Content-Disposition",
            axum::http::HeaderValue::from_static("inline"),
        );
        headers.insert(
            "Content-Security-Policy",
            axum::http::HeaderValue::from_static(
                "default-src 'none'; img-src 'self' data:; style-src 'self' 'unsafe-inline';",
            ),
        );
    }

    let stream = tokio_util::io::ReaderStream::new(reader);
    let body = axum::body::Body::from_stream(stream);
    (StatusCode::OK, headers, body).into_response()
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn get_share_type(state: &AppState, token: &str) -> ShareType {
    if let Some(ref db) = state.db {
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        if let Ok(st) = conn.query_row(
            "SELECT share_type FROM shares WHERE token = ?1",
            params![token],
            |row| row.get::<_, Option<String>>(0),
        ) {
            return st
                .as_deref()
                .map(ShareType::from_str_opt)
                .unwrap_or_default();
        }
    }
    ShareType::Download
}

fn get_allow_download(state: &AppState, token: &str) -> bool {
    if let Some(ref db) = state.db {
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        if let Ok(allowed) = conn.query_row(
            "SELECT allow_download FROM shares WHERE token = ?1",
            params![token],
            |row| row.get::<_, i32>(0),
        ) {
            return allowed != 0;
        }
    }
    true
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn sniff_mime_type(name: &str) -> String {
    let ext = name.rsplit('.').next().unwrap_or("");
    match ext.to_lowercase().as_str() {
        "pdf" => "application/pdf".to_string(),
        "png" => "image/png".to_string(),
        "jpg" | "jpeg" => "image/jpeg".to_string(),
        "gif" => "image/gif".to_string(),
        "webp" => "image/webp".to_string(),
        "svg" => "image/svg+xml".to_string(),
        "mp4" => "video/mp4".to_string(),
        "mp3" => "audio/mpeg".to_string(),
        "zip" => "application/zip".to_string(),
        "txt" | "md" => "text/plain".to_string(),
        "html" | "htm" => "text/html".to_string(),
        "json" => "application/json".to_string(),
        _ => "application/octet-stream".to_string(),
    }
}

fn constant_time_eq(a: &str, b: &str) -> bool {
    use subtle::ConstantTimeEq;
    a.as_bytes().ct_eq(b.as_bytes()).into()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_share_type_parsing() {
        assert_eq!(ShareType::from_str_opt("download"), ShareType::Download);
        assert_eq!(ShareType::from_str_opt("upload"), ShareType::Upload);
        assert_eq!(ShareType::from_str_opt("view"), ShareType::View);
        assert_eq!(ShareType::from_str_opt("unknown"), ShareType::Download);
    }

    #[test]
    fn test_share_type_display() {
        assert_eq!(ShareType::Download.to_string(), "download");
        assert_eq!(ShareType::Upload.to_string(), "upload");
        assert_eq!(ShareType::View.to_string(), "view");
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("hello.txt"), "hello.txt");
        assert_eq!(sanitize_filename("my file.pdf"), "my_file.pdf");
        assert_eq!(
            sanitize_filename("../../../etc/passwd"),
            ".._.._.._etc_passwd"
        );
    }

    #[test]
    fn test_sniff_mime_type() {
        assert_eq!(sniff_mime_type("photo.jpg"), "image/jpeg");
        assert_eq!(sniff_mime_type("doc.pdf"), "application/pdf");
        assert_eq!(sniff_mime_type("data.csv"), "application/octet-stream");
    }
}
