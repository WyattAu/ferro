use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::header::HeaderName;
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use chrono::Utc;
use dashmap::DashMap;
use http_body_util::BodyExt;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::AppState;
use crate::api_error::ApiError;
use crate::db::DbHandle;

const CONNECT_TIMEOUT_SECS: u64 = 10;
const READ_TIMEOUT_SECS: u64 = 30;

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RemoteMount {
    pub id: String,
    pub name: String,
    pub remote_url: String,
    pub local_path: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub enabled: bool,
    pub created_at: String,
}

impl RemoteMount {
    fn from_row(row: &rusqlite::Row<'_>) -> Result<Self, rusqlite::Error> {
        let enabled: i32 = row.get("enabled")?;
        Ok(Self {
            id: row.get("id")?,
            name: row.get("name")?,
            remote_url: row.get("remote_url")?,
            local_path: row.get("local_path")?,
            username: row.get("username")?,
            password: row.get("password")?,
            enabled: enabled != 0,
            created_at: row.get("created_at")?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct RemoteMountStore {
    mounts: Arc<DashMap<String, RemoteMount>>,
    db: Option<DbHandle>,
}

impl RemoteMountStore {
    pub fn new() -> Self {
        Self {
            mounts: Arc::new(DashMap::new()),
            db: None,
        }
    }

    pub fn with_db(mut self, db: DbHandle) -> Self {
        self.db = Some(db.clone());
        if let Err(e) = self.load_all_from_db(&db.lock().unwrap_or_else(|e| e.into_inner())) {
            tracing::warn!(error = %e, "failed to load remote mounts from database");
        }
        self
    }

    fn load_all_from_db(&self, conn: &rusqlite::Connection) -> Result<(), rusqlite::Error> {
        let mut stmt = conn.prepare(
            "SELECT id, name, remote_url, local_path, username, password, enabled, created_at FROM remote_mounts",
        )?;
        let rows = stmt.query_map([], RemoteMount::from_row)?;
        for row in rows {
            let mount = row?;
            if mount.enabled {
                self.mounts.insert(mount.name.clone(), mount);
            }
        }
        Ok(())
    }

    pub fn get_by_name(&self, name: &str) -> Option<RemoteMount> {
        self.mounts.get(name).map(|r| r.clone())
    }

    pub fn list_all(&self) -> Vec<RemoteMount> {
        self.mounts.iter().map(|r| r.value().clone()).collect()
    }

    pub fn insert(&self, mount: RemoteMount) -> Result<(), String> {
        if let Some(ref db) = self.db {
            let conn = db.lock().unwrap_or_else(|e| e.into_inner());
            conn.execute(
                "INSERT INTO remote_mounts (id, name, remote_url, local_path, username, password, enabled, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                rusqlite::params![
                    mount.id,
                    mount.name,
                    mount.remote_url,
                    mount.local_path,
                    mount.username,
                    mount.password,
                    mount.enabled as i32,
                    mount.created_at,
                ],
            )
            .map_err(|e| e.to_string())?;
        }
        self.mounts.insert(mount.name.clone(), mount);
        Ok(())
    }

    pub fn delete(&self, id: &str) -> Result<(), String> {
        let mount_name = {
            let found = self.mounts.iter().find(|r| r.value().id == id);
            match found {
                Some(entry) => Some(entry.key().clone()),
                None => {
                    if let Some(ref db) = self.db {
                        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
                        let name: Option<String> = conn
                            .query_row(
                                "SELECT name FROM remote_mounts WHERE id = ?1",
                                rusqlite::params![id],
                                |row| row.get(0),
                            )
                            .ok();
                        name
                    } else {
                        None
                    }
                }
            }
        };

        if let Some(ref db) = self.db {
            let conn = db.lock().unwrap_or_else(|e| e.into_inner());
            conn.execute(
                "DELETE FROM remote_mounts WHERE id = ?1",
                rusqlite::params![id],
            )
            .map_err(|e| e.to_string())?;
        }

        if let Some(name) = mount_name {
            self.mounts.remove(&name);
        }
        Ok(())
    }
}

impl Default for RemoteMountStore {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Proxy handler
// ---------------------------------------------------------------------------

fn build_proxy_client() -> reqwest::Client {
    reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(CONNECT_TIMEOUT_SECS))
        .timeout(std::time::Duration::from_secs(READ_TIMEOUT_SECS))
        .no_proxy()
        .build()
        .unwrap()
}

fn make_cache_key(mount_name: &str, remainder: &str, method: &str) -> String {
    format!("remote:{}:{}:{}", mount_name, remainder, method)
}

pub async fn proxy_remote_mount(
    method: Method,
    uri: axum::http::Uri,
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Body,
) -> Response {
    let path = uri.path();
    let remainder = match path.strip_prefix("/remote/") {
        Some(r) => r,
        None => {
            return ApiError::not_found(ApiError::NOT_FOUND, "Not a remote mount path");
        }
    };

    let (mount_name, sub_path) = match remainder.split_once('/') {
        Some((name, rest)) => (name, rest),
        None => (remainder, ""),
    };

    let mount = match state.remote_mounts.get_by_name(mount_name) {
        Some(m) => m,
        None => {
            return ApiError::not_found(
                ApiError::NOT_FOUND,
                format!("Remote mount '{}' not found", mount_name),
            );
        }
    };

    if !mount.enabled {
        return ApiError::bad_request(
            ApiError::BAD_REQUEST,
            format!("Remote mount '{}' is disabled", mount_name),
        );
    }

    let remote_base = mount.remote_url.trim_end_matches('/');
    let remote_path = if sub_path.is_empty() {
        remote_base.to_string()
    } else {
        format!("{}/{}", remote_base, sub_path)
    };

    let client = build_proxy_client();

    let mut req_builder = client.request(method.clone(), &remote_path);

    if let (Some(user), Some(pass)) = (&mount.username, &mount.password) {
        let creds = format!("{}:{}", user, pass);
        let encoded = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, creds);
        req_builder = req_builder.header("Authorization", format!("Basic {}", encoded));
    }

    let hop_by_hop: &[&str] = &[
        "connection",
        "keep-alive",
        "proxy-authenticate",
        "proxy-authorization",
        "te",
        "trailers",
        "transfer-encoding",
        "upgrade",
        "host",
    ];

    for (name, value) in headers.iter() {
        let name_lower = name.as_str().to_lowercase();
        if hop_by_hop.contains(&name_lower.as_str()) {
            continue;
        }
        if let Ok(val_str) = value.to_str() {
            req_builder = req_builder.header(name.as_str(), val_str);
        }
    }

    if method != Method::GET && method != Method::HEAD && method != Method::OPTIONS {
        let body_bytes = match body.collect().await {
            Ok(collected) => collected.to_bytes(),
            Err(e) => {
                return ApiError::internal(
                    ApiError::INTERNAL_ERROR,
                    format!("Failed to read request body: {}", e),
                );
            }
        };
        req_builder = req_builder.body(body_bytes.to_vec());
    }

    let response = match req_builder.send().await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(
                error = %e,
                mount = %mount_name,
                remote_url = %remote_path,
                "failed to proxy request to remote WebDAV server"
            );
            return ApiError::bad_gateway(
                ApiError::BAD_GATEWAY,
                format!("Remote server unreachable: {}", e),
            );
        }
    };

    let status = response.status();
    let mut resp_headers = HeaderMap::new();

    for (name, value) in response.headers().iter() {
        let name_lower = name.as_str().to_lowercase();
        if hop_by_hop.contains(&name_lower.as_str()) {
            continue;
        }
        if let Ok(hn) = HeaderName::from_bytes(name.as_str().as_bytes())
            && let Ok(hv) = HeaderValue::from_bytes(value.as_bytes())
        {
            resp_headers.insert(hn, hv);
        }
    }

    if method == Method::GET {
        let cache_key = make_cache_key(mount_name, sub_path, "GET");
        let etag = resp_headers
            .get("etag")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        if let Some(cached) = state.read_cache.get(&cache_key, &etag) {
            return (
                StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::OK),
                resp_headers,
                cached,
            )
                .into_response();
        }

        let bytes = match response.bytes().await {
            Ok(b) => b,
            Err(e) => {
                return ApiError::bad_gateway(
                    ApiError::BAD_GATEWAY,
                    format!("Failed to read remote response: {}", e),
                );
            }
        };

        if !etag.is_empty() && bytes.len() < 10 * 1024 * 1024 {
            state.read_cache.put(&cache_key, &etag, bytes.clone());
        }

        return (
            StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::OK),
            resp_headers,
            bytes,
        )
            .into_response();
    }

    let body_bytes = match response.bytes().await {
        Ok(b) => b,
        Err(e) => {
            return ApiError::bad_gateway(
                ApiError::BAD_GATEWAY,
                format!("Failed to read remote response: {}", e),
            );
        }
    };

    (
        StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::OK),
        resp_headers,
        body_bytes,
    )
        .into_response()
}

// ---------------------------------------------------------------------------
// Admin API handlers
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateMountRequest {
    pub name: String,
    pub remote_url: String,
    pub local_path: String,
    pub username: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct MountTestResponse {
    pub reachable: bool,
    pub status: u16,
    pub error: Option<String>,
}

pub async fn list_mounts(State(state): State<AppState>) -> Response {
    let mounts = state.remote_mounts.list_all();
    (StatusCode::OK, axum::Json(mounts)).into_response()
}

pub async fn create_mount(
    State(state): State<AppState>,
    axum::Json(input): axum::Json<CreateMountRequest>,
) -> Response {
    if input.name.is_empty() {
        return ApiError::bad_request(ApiError::INVALID_INPUT, "name is required");
    }
    if input.remote_url.is_empty() {
        return ApiError::bad_request(ApiError::INVALID_INPUT, "remote_url is required");
    }
    if state.remote_mounts.get_by_name(&input.name).is_some() {
        return ApiError::conflict(
            ApiError::CONFLICT,
            format!("Remote mount '{}' already exists", input.name),
        );
    }

    let mount = RemoteMount {
        id: uuid::Uuid::new_v4().to_string(),
        name: input.name,
        remote_url: input.remote_url,
        local_path: input.local_path,
        username: input.username,
        password: input.password,
        enabled: true,
        created_at: Utc::now().to_rfc3339(),
    };

    match state.remote_mounts.insert(mount.clone()) {
        Ok(()) => (StatusCode::CREATED, axum::Json(mount)).into_response(),
        Err(e) => ApiError::internal(ApiError::INTERNAL_ERROR, e),
    }
}

pub async fn delete_mount(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    match state.remote_mounts.delete(&id) {
        Ok(()) => (StatusCode::NO_CONTENT, "").into_response(),
        Err(e) => ApiError::internal(ApiError::INTERNAL_ERROR, e),
    }
}

pub async fn test_mount(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    let mount = state
        .remote_mounts
        .list_all()
        .into_iter()
        .find(|m| m.id == id);
    let mount = match mount {
        Some(m) => m,
        None => {
            return ApiError::not_found(ApiError::NOT_FOUND, "Remote mount not found");
        }
    };

    let client = build_proxy_client();
    let mut req = client.request(Method::OPTIONS, &mount.remote_url);

    if let (Some(user), Some(pass)) = (&mount.username, &mount.password) {
        let creds = format!("{}:{}", user, pass);
        let encoded = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, creds);
        req = req.header("Authorization", format!("Basic {}", encoded));
    }

    match req.send().await {
        Ok(resp) => {
            let status = resp.status().as_u16();
            (
                StatusCode::OK,
                axum::Json(MountTestResponse {
                    reachable: true,
                    status,
                    error: None,
                }),
            )
                .into_response()
        }
        Err(e) => (
            StatusCode::OK,
            axum::Json(MountTestResponse {
                reachable: false,
                status: 0,
                error: Some(e.to_string()),
            }),
        )
            .into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remote_mount_store_insert_and_get() {
        let store = RemoteMountStore::new();
        let mount = RemoteMount {
            id: "test-id".to_string(),
            name: "my-mount".to_string(),
            remote_url: "https://example.com/dav".to_string(),
            local_path: "/remote/my-mount".to_string(),
            username: Some("user".to_string()),
            password: Some("pass".to_string()),
            enabled: true,
            created_at: Utc::now().to_rfc3339(),
        };
        store.insert(mount.clone()).unwrap();

        let retrieved = store.get_by_name("my-mount").unwrap();
        assert_eq!(retrieved.id, "test-id");
        assert_eq!(retrieved.remote_url, "https://example.com/dav");
        assert_eq!(retrieved.username.as_deref(), Some("user"));

        assert!(store.get_by_name("nonexistent").is_none());
    }

    #[test]
    fn test_remote_mount_store_delete() {
        let store = RemoteMountStore::new();
        let mount = RemoteMount {
            id: "del-id".to_string(),
            name: "del-mount".to_string(),
            remote_url: "https://example.com".to_string(),
            local_path: "/remote/del".to_string(),
            username: None,
            password: None,
            enabled: true,
            created_at: Utc::now().to_rfc3339(),
        };
        store.insert(mount).unwrap();
        assert!(store.get_by_name("del-mount").is_some());

        store.delete("del-id").unwrap();
        assert!(store.get_by_name("del-mount").is_none());
    }

    #[test]
    fn test_remote_mount_store_list() {
        let store = RemoteMountStore::new();
        for i in 0..3 {
            store
                .insert(RemoteMount {
                    id: format!("id-{}", i),
                    name: format!("mount-{}", i),
                    remote_url: format!("https://{}.example.com", i),
                    local_path: format!("/remote/mount-{}", i),
                    username: None,
                    password: None,
                    enabled: true,
                    created_at: Utc::now().to_rfc3339(),
                })
                .unwrap();
        }
        assert_eq!(store.list_all().len(), 3);
    }

    #[test]
    fn test_make_cache_key() {
        let key = make_cache_key("mymount", "path/to/file.txt", "GET");
        assert_eq!(key, "remote:mymount:path/to/file.txt:GET");
    }

    #[test]
    fn test_remote_mount_from_row_parsing() {
        let dir = tempfile::tempdir().unwrap();
        let conn = crate::db::open_db(dir.path().to_str().unwrap()).unwrap();

        conn.execute(
            "INSERT INTO remote_mounts (id, name, remote_url, local_path, username, password, enabled, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, '2024-01-01T00:00:00+00:00')",
            rusqlite::params!["row-id", "row-mount", "https://row.example.com", "/remote/row", "u", "p"],
        )
        .unwrap();

        let mount: RemoteMount = conn
            .query_row(
                "SELECT id, name, remote_url, local_path, username, password, enabled, created_at FROM remote_mounts WHERE id = 'row-id'",
                [],
                RemoteMount::from_row,
            )
            .unwrap();

        assert_eq!(mount.id, "row-id");
        assert_eq!(mount.name, "row-mount");
        assert_eq!(mount.remote_url, "https://row.example.com");
        assert!(mount.enabled);
        assert_eq!(mount.username.as_deref(), Some("u"));
        assert_eq!(mount.password.as_deref(), Some("p"));
    }

    #[tokio::test]
    async fn test_admin_create_and_list_mounts() {
        let state = AppState::in_memory()
            .with_admin_user(Some("admin".to_string()))
            .with_admin_password(Some("secret".to_string()));

        let input = CreateMountRequest {
            name: "test-mount".to_string(),
            remote_url: "https://dav.example.com".to_string(),
            local_path: "/remote/test-mount".to_string(),
            username: Some("admin".to_string()),
            password: Some("pass".to_string()),
        };

        let resp = create_mount(axum::extract::State(state.clone()), axum::Json(input)).await;
        assert_eq!(resp.status(), StatusCode::CREATED);

        let list_resp = list_mounts(axum::extract::State(state.clone())).await;
        assert_eq!(list_resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_admin_delete_mount() {
        let state = AppState::in_memory();

        let mount = RemoteMount {
            id: "delete-me".to_string(),
            name: "deleteme".to_string(),
            remote_url: "https://example.com".to_string(),
            local_path: "/remote/deleteme".to_string(),
            username: None,
            password: None,
            enabled: true,
            created_at: Utc::now().to_rfc3339(),
        };
        state.remote_mounts.insert(mount).unwrap();

        let resp = delete_mount(
            axum::extract::State(state.clone()),
            axum::extract::Path("delete-me".to_string()),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
        assert!(state.remote_mounts.get_by_name("deleteme").is_none());
    }

    #[tokio::test]
    async fn test_admin_test_mount_not_found() {
        let state = AppState::in_memory();

        let resp = test_mount(
            axum::extract::State(state),
            axum::extract::Path("nonexistent".to_string()),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_proxy_mount_not_found() {
        let state = AppState::in_memory();

        let resp = proxy_remote_mount(
            Method::GET,
            axum::http::Uri::from_static("/remote/noexist/somefile.txt"),
            axum::extract::State(state),
            HeaderMap::new(),
            Body::empty(),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}
