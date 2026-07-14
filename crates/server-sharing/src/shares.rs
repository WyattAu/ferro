use async_trait::async_trait;
use axum::extract::{Extension, Path};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use chrono::{Duration, Utc};
use dashmap::DashMap;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::warn;

use crate::SharingState;
use crate::api_error::ApiError;
use crate::audit::build_audit_entry;
use crate::db::DbHandle;

pub const MAX_SHARE_LINKS: usize = 10_000;
pub const MAX_SHARE_PASSWORD_ATTEMPTS: u32 = 10;
pub const SHARE_LOCKOUT_SECS: i64 = 300;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareLink {
    pub token: String,
    pub path: String,
    pub password: Option<String>,
    pub expires_at: chrono::DateTime<Utc>,
    pub max_downloads: Option<u32>,
    pub download_count: u32,
    pub created_by: String,
    #[serde(default)]
    pub allow_download: Option<bool>,
    #[serde(default)]
    pub allow_upload: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct CreateShareRequest {
    pub path: String,
    pub password: Option<String>,
    pub expires_in_hours: Option<i64>,
    pub max_downloads: Option<u32>,
    pub allow_download: Option<bool>,
    pub allow_upload: Option<bool>,
}

#[async_trait]
pub trait ShareStoreTrait: Send + Sync {
    async fn create(&self, req: CreateShareRequest, created_by: String) -> ShareLink;
    async fn get(&self, token: &str) -> Option<ShareLink>;
    async fn delete(&self, token: &str) -> bool;
    async fn list(&self) -> Vec<ShareLink>;
    async fn increment_download(&self, token: &str) -> bool;
    fn is_share_locked(&self, token: &str) -> bool {
        let _ = token;
        false
    }
    fn record_failed_attempt(&self, token: &str) {
        let _ = token;
    }
    fn clear_failed_attempts(&self, token: &str) {
        let _ = token;
    }
}

pub struct ShareStore {
    links: Arc<RwLock<Vec<ShareLink>>>,
    db: Option<DbHandle>,
    failed_attempts: Arc<DashMap<String, (u32, chrono::DateTime<Utc>)>>,
}

impl ShareStore {
    pub fn new() -> Self {
        Self {
            links: Arc::new(RwLock::new(Vec::new())),
            db: None,
            failed_attempts: Arc::new(DashMap::new()),
        }
    }

    pub fn with_db(mut self, db: DbHandle) -> Self {
        self.db = Some(db);
        self
    }

    pub fn is_share_locked(&self, token: &str) -> bool {
        if let Some(entry) = self.failed_attempts.get(token) {
            let (count, first_failure) = entry.value();
            if *count >= MAX_SHARE_PASSWORD_ATTEMPTS {
                let elapsed = Utc::now().signed_duration_since(*first_failure);
                if elapsed.num_seconds() < SHARE_LOCKOUT_SECS {
                    return true;
                }
                drop(entry);
                self.failed_attempts.remove(token);
            }
        }
        false
    }

    pub fn record_failed_attempt(&self, token: &str) {
        self.failed_attempts
            .entry(token.to_string())
            .and_modify(|(count, first)| {
                *count += 1;
                let _ = first;
            })
            .or_insert((1, Utc::now()));
    }

    pub fn clear_failed_attempts(&self, token: &str) {
        self.failed_attempts.remove(token);
    }

    fn persist_create(&self, link: &ShareLink) {
        if let Some(ref db) = self.db {
            let conn = db.lock().unwrap_or_else(|e| e.into_inner());
            let allow_download_val = link.allow_download.map(|v| if v { 1i32 } else { 0i32 }).unwrap_or(-1);
            let _allow_upload_val = link.allow_upload.map(|v| if v { 1i32 } else { 0i32 }).unwrap_or(-1);
            if let Err(e) = conn.execute(
                "INSERT OR REPLACE INTO shares (token, file_path, password, expires_at, created_at, created_by, download_count, max_downloads, is_public, share_type, allow_download, upload_target) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    link.token,
                    link.path,
                    link.password,
                    link.expires_at.to_rfc3339(),
                    chrono::Utc::now().to_rfc3339(),
                    link.created_by,
                    link.download_count as i64,
                    link.max_downloads.map(|d| d as i64),
                    link.password.is_none() as i32,
                    match (link.allow_upload, link.allow_download) {
                        (Some(true), _) => "upload",
                        (_, Some(false)) => "view",
                        _ => "download",
                    },
                    allow_download_val,
                    if link.allow_upload == Some(true) {
                        Some(link.path.clone())
                    } else {
                        None::<String>
                    },
                ],
            ) {
                warn!("Failed to persist share to SQLite: {}", e);
            }
        }
    }

    fn persist_delete(&self, token: &str) {
        if let Some(ref db) = self.db {
            let conn = db.lock().unwrap_or_else(|e| e.into_inner());
            if let Err(e) = conn.execute("DELETE FROM shares WHERE token = ?1", params![token]) {
                warn!("Failed to delete share from SQLite: {}", e);
            }
        }
    }

    fn persist_download(&self, token: &str, count: u32) {
        if let Some(ref db) = self.db {
            let conn = db.lock().unwrap_or_else(|e| e.into_inner());
            if let Err(e) = conn.execute(
                "UPDATE shares SET download_count = ?1 WHERE token = ?2",
                params![count as i64, token],
            ) {
                warn!("Failed to update share download count in SQLite: {}", e);
            }
        }
    }

    pub fn load_all_from_db(conn: &rusqlite::Connection) -> Result<Vec<ShareLink>, rusqlite::Error> {
        let has_extended = conn.prepare("SELECT share_type FROM shares LIMIT 0").is_ok();
        let mut stmt = if has_extended {
            conn.prepare(
                "SELECT token, file_path, password, expires_at, created_by, download_count, max_downloads, allow_download, allow_upload FROM shares",
            )?
        } else {
            conn.prepare(
                "SELECT token, file_path, password, expires_at, created_by, download_count, max_downloads FROM shares",
            )?
        };
        let rows = stmt.query_map([], |row| {
            let expires_at_str: String = row.get(3)?;
            let expires_at = chrono::DateTime::parse_from_rfc3339(&expires_at_str)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| Utc::now());
            if has_extended {
                let allow_download_raw: Option<i32> = row.get(7).ok();
                let allow_upload_raw: Option<i32> = row.get(8).ok();
                let allow_download = allow_download_raw.map(|v| v != 0);
                let allow_upload = allow_upload_raw.map(|v| v != 0);
                Ok(ShareLink {
                    token: row.get(0)?,
                    path: row.get(1)?,
                    password: row.get(2)?,
                    expires_at,
                    max_downloads: row.get::<_, Option<i64>>(6)?.map(|d| d as u32),
                    download_count: row.get::<_, i64>(5)? as u32,
                    created_by: row.get(4)?,
                    allow_download,
                    allow_upload,
                })
            } else {
                Ok(ShareLink {
                    token: row.get(0)?,
                    path: row.get(1)?,
                    password: row.get(2)?,
                    expires_at,
                    max_downloads: row.get::<_, Option<i64>>(6)?.map(|d| d as u32),
                    download_count: row.get::<_, i64>(5)? as u32,
                    created_by: row.get(4)?,
                    allow_download: None,
                    allow_upload: None,
                })
            }
        })?;
        let mut links = Vec::new();
        for row in rows {
            links.push(row?);
        }
        Ok(links)
    }

    pub async fn load_link(&self, link: ShareLink) {
        self.links.write().await.push(link);
    }

    pub fn load_links_blocking(&self, links: Vec<ShareLink>) {
        tokio::task::block_in_place(|| {
            let mut guard = self.links.blocking_write();
            for link in links {
                guard.push(link);
            }
        });
    }
}

#[async_trait]
impl ShareStoreTrait for ShareStore {
    async fn create(&self, req: CreateShareRequest, created_by: String) -> ShareLink {
        let token = uuid::Uuid::new_v4().to_string();
        let expires_at = match req.expires_in_hours {
            Some(hours) => Utc::now() + Duration::hours(hours),
            None => Utc::now() + Duration::days(7),
        };
        let link = ShareLink {
            token: token.clone(),
            path: req.path,
            password: req.password.map(|p| hash_share_password(&p)),
            expires_at,
            max_downloads: req.max_downloads,
            download_count: 0,
            created_by,
            allow_download: req.allow_download,
            allow_upload: req.allow_upload,
        };
        let mut links = self.links.write().await;
        links.push(link.clone());
        if links.len() > MAX_SHARE_LINKS {
            links.retain(|l| l.expires_at > Utc::now());
            if links.len() > MAX_SHARE_LINKS {
                let excess = links.len() - MAX_SHARE_LINKS;
                links.drain(..excess);
            }
        }
        self.persist_create(&link);
        link
    }

    async fn get(&self, token: &str) -> Option<ShareLink> {
        let links = self.links.read().await;
        links.iter().find(|l| l.token == token).cloned()
    }

    async fn delete(&self, token: &str) -> bool {
        let mut links = self.links.write().await;
        if let Some(pos) = links.iter().position(|l| l.token == token) {
            links.remove(pos);
            self.persist_delete(token);
            true
        } else {
            false
        }
    }

    async fn list(&self) -> Vec<ShareLink> {
        let links = self.links.read().await;
        links.iter().filter(|l| l.expires_at > Utc::now()).cloned().collect()
    }

    async fn increment_download(&self, token: &str) -> bool {
        let mut links = self.links.write().await;
        if let Some(link) = links.iter_mut().find(|l| l.token == token) {
            link.download_count += 1;
            let count = link.download_count;
            self.persist_download(token, count);
            true
        } else {
            false
        }
    }
}

impl Default for ShareStore {
    fn default() -> Self {
        Self::new()
    }
}

pub async fn create_share(
    Extension(state): Extension<SharingState>,
    axum::Json(req): axum::Json<CreateShareRequest>,
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

    let link = state.share_store.create(req, "anonymous".to_string()).await;

    if let Some(ref cb) = state.on_share_created {
        let path = link.path.clone();
        let created_by = link.created_by.clone();
        cb(&path, &created_by).await;
    }

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

pub async fn list_shares(Extension(state): Extension<SharingState>) -> Response {
    let links: Vec<ShareLink> = state.share_store.list().await;
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

pub async fn delete_share(Extension(state): Extension<SharingState>, Path(token): Path<String>) -> Response {
    if state.share_store.delete(&token).await {
        (StatusCode::NO_CONTENT, "").into_response()
    } else {
        ApiError::not_found(ApiError::SHARE_NOT_FOUND, "Share not found")
    }
}

pub async fn serve_share(
    Extension(state): Extension<SharingState>,
    Path(token): Path<String>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> Response {
    if state.share_store.is_share_locked(&token) {
        return ApiError::with_details(
            StatusCode::TOO_MANY_REQUESTS,
            ApiError::RATE_LIMITED,
            "Too many failed password attempts. Try again later.",
            format!("{} seconds remaining", SHARE_LOCKOUT_SECS),
        );
    }

    let link = match state.share_store.get(&token).await {
        Some(l) => l,
        None => return ApiError::not_found(ApiError::SHARE_NOT_FOUND, "Share not found"),
    };

    if link.expires_at < Utc::now() {
        return ApiError::gone(ApiError::SHARE_EXPIRED, "Share expired");
    }

    if link.allow_upload == Some(true) {
        return crate::shares_ext::serve_upload_dropzone(&link);
    }

    if let Some(max) = link.max_downloads
        && link.download_count >= max
    {
        return ApiError::gone(ApiError::SHARE_EXPIRED, "Download limit reached");
    }

    if let Some(ref stored_hash) = link.password {
        let provided_password = params.get("password").map(|s| s.as_str());
        match provided_password {
            Some(pw) if verify_share_password(pw, stored_hash) => {
                state.share_store.clear_failed_attempts(&token);
            }
            Some(_) => {
                state.share_store.record_failed_attempt(&token);
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

    let meta = match state.storage.head(&link.path).await {
        Ok(m) => m,
        Err(_) => return ApiError::not_found(ApiError::FILE_NOT_FOUND, "File not found"),
    };

    if link.allow_download == Some(false) {
        state.share_store.increment_download(&token).await;
        return crate::shares_ext::serve_preview_html(&link, &meta);
    }

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

fn constant_time_eq(a: &str, b: &str) -> bool {
    use subtle::ConstantTimeEq;
    a.as_bytes().ct_eq(b.as_bytes()).into()
}

pub fn hash_share_password(password: &str) -> String {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(password.as_bytes());
    hex::encode(hash)
}

pub fn verify_share_password(provided: &str, stored_hash: &str) -> bool {
    let provided_hash = hash_share_password(provided);
    constant_time_eq(&provided_hash, stored_hash)
}

pub async fn handle_share_upload(
    Extension(state): Extension<SharingState>,
    Path(token): Path<String>,
    mut multipart: axum::extract::Multipart,
) -> Response {
    let link = match state.share_store.get(&token).await {
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

    if bytes.len() > state.max_body_size as usize {
        return ApiError::with_details(
            StatusCode::PAYLOAD_TOO_LARGE,
            ApiError::PAYLOAD_TOO_LARGE,
            "Upload exceeds size limit",
            format!("max {} bytes", state.max_body_size),
        );
    }

    let target_path = format!("{}/{}", link.path.trim_end_matches('/'), file_name);

    if state.storage.head(&link.path).await.is_err()
        && let Err(e) = state.storage.create_collection(&link.path, "anonymous").await
    {
        tracing::warn!(error = %e, path = %link.path, "failed to create upload target directory");
        return ApiError::internal(ApiError::INTERNAL_ERROR, "Failed to create upload directory");
    }

    let content_type = crate::shares_ext::sniff_mime_type(&file_name);
    if let Err(e) = state.storage.put(&target_path, bytes.clone(), "anonymous").await {
        tracing::warn!(error = %e, path = %target_path, "failed to store uploaded file");
        return ApiError::internal(ApiError::INTERNAL_ERROR, "Failed to store uploaded file");
    }

    state
        .audit_log
        .log_audit(build_audit_entry(
            "POST",
            &format!("/s/{}", token),
            "anonymous",
            201,
            None,
            None,
        ))
        .await;

    if let Some(ref db) = state.db {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_store() -> ShareStore {
        ShareStore::new()
    }

    fn make_req(path: &str) -> CreateShareRequest {
        CreateShareRequest {
            path: path.to_string(),
            password: None,
            expires_in_hours: None,
            max_downloads: None,
            allow_download: None,
            allow_upload: None,
        }
    }

    #[tokio::test]
    async fn test_create_share_basic() {
        let store = make_store();
        let req = make_req("/documents/report.pdf");
        let link = store.create(req, "alice".to_string()).await;
        assert_eq!(link.path, "/documents/report.pdf");
        assert_eq!(link.created_by, "alice");
        assert!(!link.token.is_empty());
        assert_eq!(link.download_count, 0);
        assert!(link.password.is_none());
    }

    #[tokio::test]
    async fn test_create_share_with_password() {
        let store = make_store();
        let req = CreateShareRequest {
            path: "/secret.txt".to_string(),
            password: Some("my_password".to_string()),
            expires_in_hours: None,
            max_downloads: None,
            allow_download: None,
            allow_upload: None,
        };
        let link = store.create(req, "bob".to_string()).await;
        assert!(link.password.is_some());
        assert!(verify_share_password("my_password", link.password.as_ref().unwrap()));
        assert!(!verify_share_password("wrong", link.password.as_ref().unwrap()));
    }

    #[tokio::test]
    async fn test_create_share_with_expiry() {
        let store = make_store();
        let req = CreateShareRequest {
            path: "/temp.dat".to_string(),
            password: None,
            expires_in_hours: Some(1),
            max_downloads: None,
            allow_download: None,
            allow_upload: None,
        };
        let link = store.create(req, "carol".to_string()).await;
        let diff = link.expires_at - Utc::now();
        assert!(diff.num_hours() >= 0 && diff.num_hours() <= 2);
    }

    #[tokio::test]
    async fn test_create_share_with_max_downloads() {
        let store = make_store();
        let req = CreateShareRequest {
            path: "/limited.txt".to_string(),
            password: None,
            expires_in_hours: None,
            max_downloads: Some(5),
            allow_download: None,
            allow_upload: None,
        };
        let link = store.create(req, "dave".to_string()).await;
        assert_eq!(link.max_downloads, Some(5));
    }

    #[tokio::test]
    async fn test_create_share_with_permissions() {
        let store = make_store();
        let req = CreateShareRequest {
            path: "/upload_dir".to_string(),
            password: None,
            expires_in_hours: None,
            max_downloads: None,
            allow_download: Some(false),
            allow_upload: Some(true),
        };
        let link = store.create(req, "eve".to_string()).await;
        assert_eq!(link.allow_download, Some(false));
        assert_eq!(link.allow_upload, Some(true));
    }

    #[tokio::test]
    async fn test_get_share_by_token() {
        let store = make_store();
        let req = make_req("/file.txt");
        let link = store.create(req, "alice".to_string()).await;
        let found = store.get(&link.token).await;
        assert!(found.is_some());
        assert_eq!(found.unwrap().path, "/file.txt");
    }

    #[tokio::test]
    async fn test_get_nonexistent_share() {
        let store = make_store();
        let found = store.get("nonexistent-token").await;
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_delete_share() {
        let store = make_store();
        let req = make_req("/to_delete.txt");
        let link = store.create(req, "alice".to_string()).await;
        assert!(store.delete(&link.token).await);
        assert!(store.get(&link.token).await.is_none());
    }

    #[tokio::test]
    async fn test_delete_nonexistent_share() {
        let store = make_store();
        assert!(!store.delete("no-such-token").await);
    }

    #[tokio::test]
    async fn test_list_shares_filters_expired() {
        let store = make_store();
        let active_req = CreateShareRequest {
            path: "/active.txt".to_string(),
            password: None,
            expires_in_hours: Some(24),
            max_downloads: None,
            allow_download: None,
            allow_upload: None,
        };
        let expired_req = CreateShareRequest {
            path: "/expired.txt".to_string(),
            password: None,
            expires_in_hours: Some(-1),
            max_downloads: None,
            allow_download: None,
            allow_upload: None,
        };
        store.create(active_req, "alice".to_string()).await;
        store.create(expired_req, "alice".to_string()).await;
        let links = store.list().await;
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].path, "/active.txt");
    }

    #[tokio::test]
    async fn test_list_shares_empty() {
        let store = make_store();
        let links = store.list().await;
        assert!(links.is_empty());
    }

    #[tokio::test]
    async fn test_increment_download() {
        let store = make_store();
        let req = make_req("/download.txt");
        let link = store.create(req, "alice".to_string()).await;
        assert!(store.increment_download(&link.token).await);
        let updated = store.get(&link.token).await.unwrap();
        assert_eq!(updated.download_count, 1);
        store.increment_download(&link.token).await;
        let updated = store.get(&link.token).await.unwrap();
        assert_eq!(updated.download_count, 2);
    }

    #[tokio::test]
    async fn test_increment_download_nonexistent() {
        let store = make_store();
        assert!(!store.increment_download("fake-token").await);
    }

    #[tokio::test]
    async fn test_share_store_default() {
        let store = ShareStore::default();
        let links = store.list().await;
        assert!(links.is_empty());
    }

    #[tokio::test]
    async fn test_load_link() {
        let store = make_store();
        let link = ShareLink {
            token: "test-token-1".to_string(),
            path: "/loaded.txt".to_string(),
            password: None,
            expires_at: Utc::now() + Duration::days(1),
            max_downloads: None,
            download_count: 0,
            created_by: "admin".to_string(),
            allow_download: None,
            allow_upload: None,
        };
        store.load_link(link.clone()).await;
        let found = store.get("test-token-1").await;
        assert!(found.is_some());
        assert_eq!(found.unwrap().path, "/loaded.txt");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_load_links_blocking() {
        let store = make_store();
        let links = vec![
            ShareLink {
                token: "bulk-1".to_string(),
                path: "/a.txt".to_string(),
                password: None,
                expires_at: Utc::now() + Duration::days(1),
                max_downloads: None,
                download_count: 0,
                created_by: "admin".to_string(),
                allow_download: None,
                allow_upload: None,
            },
            ShareLink {
                token: "bulk-2".to_string(),
                path: "/b.txt".to_string(),
                password: None,
                expires_at: Utc::now() + Duration::days(1),
                max_downloads: None,
                download_count: 0,
                created_by: "admin".to_string(),
                allow_download: None,
                allow_upload: None,
            },
        ];
        store.load_links_blocking(links);
        assert!(store.get("bulk-1").await.is_some());
        assert!(store.get("bulk-2").await.is_some());
    }

    #[test]
    fn test_hash_share_password_deterministic() {
        let h1 = hash_share_password("test123");
        let h2 = hash_share_password("test123");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_hash_share_password_different_inputs() {
        let h1 = hash_share_password("abc");
        let h2 = hash_share_password("xyz");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_verify_share_password_correct() {
        let hash = hash_share_password("correct");
        assert!(verify_share_password("correct", &hash));
    }

    #[test]
    fn test_verify_share_password_incorrect() {
        let hash = hash_share_password("correct");
        assert!(!verify_share_password("incorrect", &hash));
    }

    #[test]
    fn test_constant_time_eq_equal() {
        assert!(constant_time_eq("abc", "abc"));
    }

    #[test]
    fn test_constant_time_eq_not_equal() {
        assert!(!constant_time_eq("abc", "def"));
    }

    #[test]
    fn test_constant_time_eq_different_length() {
        assert!(!constant_time_eq("abc", "abcd"));
    }

    #[tokio::test]
    async fn test_record_failed_attempts() {
        let store = make_store();
        store.record_failed_attempt("token1");
        store.record_failed_attempt("token1");
        assert_eq!(store.failed_attempts.get("token1").unwrap().value().0, 2);
    }

    #[tokio::test]
    async fn test_clear_failed_attempts() {
        let store = make_store();
        store.record_failed_attempt("token1");
        store.clear_failed_attempts("token1");
        assert!(store.failed_attempts.get("token1").is_none());
    }

    #[tokio::test]
    async fn test_is_share_locked_false_initially() {
        let store = make_store();
        assert!(!store.is_share_locked("any-token"));
    }

    #[tokio::test]
    async fn test_is_share_locked_after_max_attempts() {
        let store = make_store();
        for _ in 0..MAX_SHARE_PASSWORD_ATTEMPTS {
            store.record_failed_attempt("token1");
        }
        assert!(store.is_share_locked("token1"));
    }

    #[tokio::test]
    async fn test_is_share_locked_resets_after_timeout() {
        let store = make_store();
        for _ in 0..MAX_SHARE_PASSWORD_ATTEMPTS {
            store.record_failed_attempt("token1");
        }
        // Simulate old failure by modifying timestamp
        {
            let mut entry = store.failed_attempts.get_mut("token1").unwrap();
            *entry.value_mut() = (
                MAX_SHARE_PASSWORD_ATTEMPTS,
                Utc::now() - Duration::seconds(SHARE_LOCKOUT_SECS + 1),
            );
        }
        assert!(!store.is_share_locked("token1"));
    }

    #[tokio::test]
    async fn test_create_multiple_shares() {
        let store = make_store();
        for i in 0..5 {
            let req = make_req(&format!("/file_{}.txt", i));
            store.create(req, "user".to_string()).await;
        }
        let links = store.list().await;
        assert_eq!(links.len(), 5);
    }

    #[tokio::test]
    async fn test_share_default_expiry() {
        let store = make_store();
        let req = make_req("/default_expiry.txt");
        let link = store.create(req, "alice".to_string()).await;
        let diff = link.expires_at - Utc::now();
        assert!(diff.num_days() >= 6 && diff.num_days() <= 8);
    }

    #[tokio::test]
    async fn test_share_download_count_persists() {
        let store = make_store();
        let req = make_req("/counter.txt");
        let link = store.create(req, "alice".to_string()).await;
        store.increment_download(&link.token).await;
        store.increment_download(&link.token).await;
        store.increment_download(&link.token).await;
        let found = store.get(&link.token).await.unwrap();
        assert_eq!(found.download_count, 3);
    }

    #[tokio::test]
    async fn test_delete_then_list() {
        let store = make_store();
        let link1 = store.create(make_req("/a.txt"), "alice".to_string()).await;
        store.create(make_req("/b.txt"), "alice".to_string()).await;
        store.delete(&link1.token).await;
        let links = store.list().await;
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].path, "/b.txt");
    }

    #[test]
    fn test_hash_password_is_hex() {
        let hash = hash_share_password("test");
        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[tokio::test]
    async fn test_share_token_unique() {
        let store = make_store();
        let link1 = store.create(make_req("/a.txt"), "alice".to_string()).await;
        let link2 = store.create(make_req("/b.txt"), "alice".to_string()).await;
        assert_ne!(link1.token, link2.token);
    }

    #[tokio::test]
    async fn test_list_only_non_expired() {
        let store = make_store();
        store
            .create(
                CreateShareRequest {
                    path: "/future.txt".to_string(),
                    password: None,
                    expires_in_hours: Some(100),
                    max_downloads: None,
                    allow_download: None,
                    allow_upload: None,
                },
                "alice".to_string(),
            )
            .await;
        store
            .create(
                CreateShareRequest {
                    path: "/past.txt".to_string(),
                    password: None,
                    expires_in_hours: Some(-100),
                    max_downloads: None,
                    allow_download: None,
                    allow_upload: None,
                },
                "alice".to_string(),
            )
            .await;
        let links = store.list().await;
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].path, "/future.txt");
    }

    #[test]
    fn test_share_store_lockout_constants() {
        assert_eq!(MAX_SHARE_LINKS, 10_000);
        assert_eq!(MAX_SHARE_PASSWORD_ATTEMPTS, 10);
        assert_eq!(SHARE_LOCKOUT_SECS, 300);
    }
}
