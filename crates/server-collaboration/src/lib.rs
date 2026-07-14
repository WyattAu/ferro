pub mod chat_api;
pub mod collab_ws;
pub mod comments;
pub mod tags;

use std::sync::Arc;

pub use common::DbHandle;

/// API error type for collaboration handlers.
///
/// Re-exports `ferro_server_security::ApiError` and adds missing constants.
pub struct ApiError;

impl ApiError {
    pub fn respond(status: axum::http::StatusCode, code: &str, message: impl Into<String>) -> axum::response::Response {
        ferro_server_security::ApiError::respond(status, code, message)
    }

    pub fn bad_request(code: &str, message: impl Into<String>) -> axum::response::Response {
        ferro_server_security::ApiError::bad_request(code, message)
    }

    pub fn not_found(code: &str, message: impl Into<String>) -> axum::response::Response {
        ferro_server_security::ApiError::not_found(code, message)
    }

    pub fn forbidden(code: &str, message: impl Into<String>) -> axum::response::Response {
        ferro_server_security::ApiError::forbidden(code, message)
    }

    pub fn internal(code: &str, message: impl Into<String>) -> axum::response::Response {
        ferro_server_security::ApiError::internal(code, message)
    }

    pub fn service_unavailable(code: &str, message: impl Into<String>) -> axum::response::Response {
        ferro_server_security::ApiError::respond(axum::http::StatusCode::SERVICE_UNAVAILABLE, code, message)
    }

    pub const BAD_REQUEST: &'static str = "BAD_REQUEST";
    pub const NOT_FOUND: &'static str = "NOT_FOUND";
    pub const INTERNAL_ERROR: &'static str = "INTERNAL_ERROR";
    pub const NOT_CONFIGURED: &'static str = "NOT_CONFIGURED";
    pub const INVALID_BODY: &'static str = "INVALID_BODY";
    pub const INVALID_JSON: &'static str = "INVALID_JSON";
    pub const PATH_INVALID: &'static str = "PATH_INVALID";
    pub const POLICY_DENIED: &'static str = "POLICY_DENIED";
}

pub use ferro_server_security_middleware::security::contains_html;
/// Re-export security functions used by collaboration modules.
pub use ferro_server_security_middleware::security::sanitize_control_chars;

/// Audit log entry (mirrored from ferro-server for trait purposes).
#[derive(Debug, Clone, serde::Serialize)]
pub struct AuditEntry {
    pub timestamp: String,
    pub method: String,
    pub path: String,
    pub user: String,
    pub status: u16,
    pub client_ip: Option<String>,
    pub user_agent: Option<String>,
    pub content_length: Option<u64>,
}

/// Minimal audit log trait for collaboration handlers that need to record audit events.
#[async_trait::async_trait]
pub trait AuditLogTrait: Send + Sync {
    async fn log(&self, entry: AuditEntry);
}

/// Trait that AppState must implement for collaboration handlers.
///
/// This allows collaboration handler functions to be generic over the trait,
/// avoiding a circular dependency on `ferro-server`.
pub trait CollaborationState: Send + Sync + Clone + 'static + common::server_context::HasDataDir {
    fn admin_user(&self) -> Option<&str>;
    fn audit_log(&self) -> &Arc<dyn AuditLogTrait>;
    fn comments(&self) -> &Arc<comments::CommentStore>;
    fn tags(&self) -> &Arc<tags::TagStore>;
    fn storage(&self) -> &Arc<dyn common::storage::StorageEngine>;
    fn collab_rooms(&self) -> &collab_ws::CollabRoomManager;
    fn db(&self) -> &Option<DbHandle>;
}

/// Build an audit entry from request details.
pub fn build_audit_entry(
    method: &str,
    path: &str,
    user: &str,
    status: u16,
    client_ip: Option<String>,
    user_agent: Option<String>,
) -> AuditEntry {
    AuditEntry {
        timestamp: chrono::Utc::now().to_rfc3339(),
        method: method.to_string(),
        path: path.to_string(),
        user: user.to_string(),
        status,
        client_ip,
        user_agent,
        content_length: None,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use bytes::Bytes;
    use common::error::FerroError;
    use common::error::Result;
    use common::metadata::{ContentHash, FileMetadata};
    use common::path::normalize_path;
    use common::storage::StorageEngine;
    use dashmap::DashMap;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    /// Test-only in-memory storage engine for unit tests.
    #[derive(Clone)]
    pub struct InMemoryStorageEngine {
        data: Arc<RwLock<DashMap<String, Bytes>>>,
        metadata: Arc<RwLock<DashMap<String, FileMetadata>>>,
    }

    impl InMemoryStorageEngine {
        pub fn new() -> Self {
            Self {
                data: Arc::new(RwLock::new(DashMap::new())),
                metadata: Arc::new(RwLock::new(DashMap::new())),
            }
        }
    }

    impl Default for InMemoryStorageEngine {
        fn default() -> Self {
            Self::new()
        }
    }

    #[async_trait::async_trait]
    impl StorageEngine for InMemoryStorageEngine {
        async fn head(&self, path: &str) -> Result<FileMetadata> {
            let path = normalize_path(path).into_owned();
            self.metadata
                .read()
                .await
                .get(&path)
                .map(|m| m.value().clone())
                .ok_or(FerroError::NotFound(path))
        }

        async fn get(&self, path: &str) -> Result<Bytes> {
            let path = normalize_path(path).into_owned();
            self.data
                .read()
                .await
                .get(&path)
                .map(|d| d.value().clone())
                .ok_or(FerroError::NotFound(path))
        }

        async fn put(&self, path: &str, data: Bytes, owner: &str) -> Result<FileMetadata> {
            let path = normalize_path(path).into_owned();
            let now = chrono::Utc::now();
            let hash = ContentHash::compute(data.as_ref());
            let meta = FileMetadata {
                path: path.clone(),
                content_hash: hash.clone(),
                size: data.len() as u64,
                mime_type: "application/octet-stream".to_string(),
                is_collection: false,
                created_at: now,
                modified_at: now,
                owner: owner.to_string(),
                etag: format!("\"{}\"", hash.as_str()),
            };
            self.data.write().await.insert(path.clone(), data);
            self.metadata.write().await.insert(path.clone(), meta.clone());
            Ok(meta)
        }

        async fn delete(&self, path: &str) -> Result<()> {
            let path = normalize_path(path).into_owned();
            self.data.write().await.remove(&path);
            self.metadata.write().await.remove(&path);
            Ok(())
        }

        async fn list(&self, prefix: &str) -> Result<Vec<FileMetadata>> {
            let prefix = normalize_path(prefix).into_owned();
            let metadata = self.metadata.read().await;
            Ok(metadata
                .iter()
                .filter(|entry| entry.key().starts_with(&prefix))
                .map(|entry| entry.value().clone())
                .collect())
        }

        async fn copy(&self, _from: &str, _to: &str) -> Result<()> {
            Ok(())
        }

        async fn move_path(&self, _from: &str, _to: &str) -> Result<()> {
            Ok(())
        }

        async fn exists(&self, path: &str) -> Result<bool> {
            let path = normalize_path(path).into_owned();
            Ok(self.data.read().await.contains_key(&path))
        }

        async fn create_collection(&self, _path: &str, _owner: &str) -> Result<FileMetadata> {
            let now = chrono::Utc::now();
            Ok(FileMetadata {
                path: _path.to_string(),
                content_hash: ContentHash::compute(b""),
                size: 0,
                mime_type: "inode/directory".to_string(),
                is_collection: true,
                created_at: now,
                modified_at: now,
                owner: _owner.to_string(),
                etag: String::new(),
            })
        }

        async fn list_all(&self, path: &str, _max_depth: u32) -> Result<Vec<FileMetadata>> {
            self.list(path).await
        }
    }

    /// Open a SQLite database for tests.
    pub fn open_test_db(data_dir: &str) -> rusqlite::Connection {
        let db_path = std::path::Path::new(data_dir).join("test.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA busy_timeout=5000;")
            .unwrap();

        // Create the comments table for tests.
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS comments (
                id TEXT PRIMARY KEY,
                path TEXT NOT NULL,
                user_id TEXT NOT NULL,
                parent_id TEXT,
                body TEXT NOT NULL,
                resolved INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );",
        )
        .unwrap();

        // Create the file_tags table for tests.
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS file_tags (
                file_path TEXT NOT NULL,
                tag TEXT NOT NULL,
                PRIMARY KEY (file_path, tag)
            );",
        )
        .unwrap();

        conn
    }
}
