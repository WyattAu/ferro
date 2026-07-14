//! API key authentication for service-to-service and CLI access.
//!
//! API keys are hashed (SHA-256) before storage. The raw key is returned
//! to the caller exactly once during creation. Authentication extracts the
//! key from the `X-API-Key` header or `?api_key=` query parameter.

use async_trait::async_trait;
use chrono::Utc;
use dashmap::DashMap;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use tracing::warn;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Maximum number of API keys per user.
const MAX_KEYS_PER_USER: usize = 25;

/// Prefix for raw keys returned to callers (identifiable in logs).
pub const KEY_PREFIX: &str = "ferro_";

/// Raw key byte length before Base58 encoding (256-bit keys).
const KEY_ENTROPY_BYTES: usize = 32;

/// Error returned by API key store operations.
#[derive(Debug)]
pub struct ApiKeyError {
    pub kind: ApiKeyErrorKind,
    pub message: String,
}

#[non_exhaustive]
#[derive(Debug, PartialEq)]
pub enum ApiKeyErrorKind {
    NotFound,
    Forbidden,
    BadRequest,
    Conflict,
    QuotaExceeded,
}

impl ApiKeyError {
    /// Create a "not found" error.
    ///
    /// # Panics
    ///
    /// This function never panics.
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self {
            kind: ApiKeyErrorKind::NotFound,
            message: msg.into(),
        }
    }
    /// Create a "forbidden" error.
    ///
    /// # Panics
    ///
    /// This function never panics.
    pub fn forbidden(msg: impl Into<String>) -> Self {
        Self {
            kind: ApiKeyErrorKind::Forbidden,
            message: msg.into(),
        }
    }
    /// Create a "bad request" error.
    ///
    /// # Panics
    ///
    /// This function never panics.
    pub fn bad_request(msg: impl Into<String>) -> Self {
        Self {
            kind: ApiKeyErrorKind::BadRequest,
            message: msg.into(),
        }
    }
    /// Create a "conflict" (duplicate) error.
    ///
    /// # Panics
    ///
    /// This function never panics.
    pub fn conflict(msg: impl Into<String>) -> Self {
        Self {
            kind: ApiKeyErrorKind::Conflict,
            message: msg.into(),
        }
    }
    /// Create a "quota exceeded" error.
    ///
    /// # Panics
    ///
    /// This function never panics.
    pub fn quota_exceeded(msg: impl Into<String>) -> Self {
        Self {
            kind: ApiKeyErrorKind::QuotaExceeded,
            message: msg.into(),
        }
    }
}

/// Permissions granted by an API key.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum ApiKeyPermission {
    /// Full read access (GET, HEAD, PROPFIND).
    #[default]
    Read,
    /// Full write access (PUT, POST, PATCH, DELETE, MKCOL, COPY, MOVE).
    Write,
    /// Administrative operations (LOCK, UNLOCK, user management).
    Admin,
}

impl ApiKeyPermission {
    /// Check if this permission level allows a given HTTP-style action.
    #[must_use]
    pub fn allows_action(&self, action: &str) -> bool {
        match self {
            Self::Admin => true,
            Self::Write => matches!(action, "read" | "write" | "delete" | "list"),
            Self::Read => matches!(action, "read" | "list"),
        }
    }
}

/// A persisted API key record (hash only — raw key is never stored).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    /// Unique identifier for the key.
    pub id: String,
    /// Human-readable name chosen by the user.
    pub name: String,
    /// SHA-256 hash of the raw key.
    pub key_hash: String,
    /// Owner user ID.
    pub user_id: String,
    /// Permission level.
    pub permission: ApiKeyPermission,
    /// Creation timestamp.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Optional expiration timestamp. `None` = never expires.
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Last time this key was used for authentication.
    pub last_used_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl ApiKey {
    /// Check whether the key has expired.
    #[must_use]
    pub fn is_expired(&self) -> bool {
        self.expires_at.is_some_and(|exp| exp < Utc::now())
    }

    /// Update the last-used timestamp.
    pub fn touch(&mut self) {
        self.last_used_at = Some(Utc::now());
    }
}

/// Request body for creating a new API key.
#[derive(Debug, Deserialize)]
pub struct CreateApiKeyRequest {
    /// Human-readable name.
    pub name: String,
    /// Permission level (defaults to Read).
    #[serde(default)]
    pub permission: ApiKeyPermission,
    /// Optional expiration as an ISO 8601 datetime string.
    pub expires_at: Option<String>,
}

/// Response returned after creating a key (contains the raw key — only shown once).
#[derive(Debug, Serialize)]
pub struct ApiKeyCreatedResponse {
    /// Unique identifier for the key.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Permission level.
    pub permission: ApiKeyPermission,
    /// Creation timestamp.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Expiration timestamp, if any.
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    /// The raw API key. **Show this to the user once — it cannot be recovered.**
    pub raw_key: String,
}

impl Zeroize for ApiKeyCreatedResponse {
    fn zeroize(&mut self) {
        self.raw_key.zeroize();
    }
}

impl ZeroizeOnDrop for ApiKeyCreatedResponse {}

/// Async interface for persisting and authenticating API keys.
#[async_trait]
pub trait ApiKeyStoreTrait: Send + Sync {
    /// Create a new API key, returning the full record and the raw key.
    async fn create_key(&self, user_id: &str, request: CreateApiKeyRequest) -> Result<(ApiKey, String), ApiKeyError>;

    /// Look up a key by its ID.
    async fn get_key(&self, id: &str) -> Result<ApiKey, ApiKeyError>;

    /// List all keys owned by a user.
    async fn list_keys(&self, user_id: &str) -> Vec<ApiKey>;

    /// Revoke (delete) a key by ID. Enforces ownership unless caller is admin.
    async fn revoke_key(&self, id: &str, requesting_user_id: &str) -> Result<(), ApiKeyError>;

    /// Authenticate a raw key, returning the key record if valid.
    /// Also updates the last-used timestamp.
    async fn authenticate(&self, raw_key: &str) -> Result<ApiKey, ApiKeyError>;

    /// Count keys owned by a user.
    async fn count_keys(&self, user_id: &str) -> usize;
}

/// Hash a raw key string using SHA-256, returning a hex-encoded hash.
#[must_use]
pub fn hash_api_key(raw_key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw_key.as_bytes());
    hex::encode(hasher.finalize())
}

/// Generate a new random API key with the `ferro_` prefix.
#[must_use]
pub fn generate_raw_key() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; KEY_ENTROPY_BYTES];
    rand::rng().fill_bytes(&mut bytes);
    let key = format!("{}{}", KEY_PREFIX, hex::encode(bytes));
    bytes.zeroize();
    key
}

/// Extract an API key from standard locations (header or query param).
///
/// Checks `X-API-Key` header first, then `?api_key=` query parameter.
#[must_use]
pub fn extract_api_key(headers: &axum::http::HeaderMap, query: Option<&str>) -> Option<String> {
    // Header takes precedence
    if let Some(h) = headers.get("X-API-Key").and_then(|v| v.to_str().ok()) {
        let trimmed = h.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    // Query param fallback
    if let Some(q) = query {
        for pair in q.split('&') {
            if let Some((k, v)) = pair.split_once('=')
                && k == "api_key"
            {
                let trimmed = v.trim();
                if !trimmed.is_empty() {
                    return Some(urlencoding_decode(trimmed));
                }
            }
        }
    }
    None
}

/// Minimal URL-decoding for `+` and `%XX` sequences.
fn urlencoding_decode(s: &str) -> String {
    let mut result = Vec::new();
    let mut chars = s.bytes();
    while let Some(b) = chars.next() {
        match b {
            b'+' => result.push(b' '),
            b'%' => {
                let hex: Vec<u8> = chars.by_ref().take(2).collect();
                if hex.len() == 2
                    && let Ok(byte) = u8::from_str_radix(&String::from_utf8_lossy(&hex), 16)
                {
                    result.push(byte);
                    continue;
                }
                result.push(b'%');
                result.extend(hex);
            }
            _ => result.push(b),
        }
    }
    String::from_utf8(result).unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned())
}

/// In-memory API key store backed by a concurrent hash map with optional `SQLite` persistence.
pub struct InMemoryApiKeyStore {
    keys: DashMap<String, ApiKey>,
    /// Secondary index: `user_id` -> Vec<`key_id`>
    user_keys: DashMap<String, Vec<String>>,
    /// Hash -> `key_id` for authentication lookup
    hash_index: DashMap<String, String>,
    db: Option<DbHandle>,
}

// Re-use the same DbHandle type from the users module.
pub use super::users::DbHandle;

impl InMemoryApiKeyStore {
    /// Create a new empty in-memory store.
    #[must_use]
    pub fn new() -> Self {
        Self {
            keys: DashMap::new(),
            user_keys: DashMap::new(),
            hash_index: DashMap::new(),
            db: None,
        }
    }

    /// Attach a `SQLite` database handle for persistence.
    pub fn with_db(mut self, db: DbHandle) -> Self {
        self.db = Some(db);
        self
    }

    fn persist_key(&self, key: &ApiKey) {
        if let Some(ref db) = self.db {
            let conn = db.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Err(e) = conn.execute(
                "INSERT OR REPLACE INTO api_keys (id, name, key_hash, user_id, permission, created_at, expires_at, last_used_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    key.id,
                    key.name,
                    key.key_hash,
                    key.user_id,
                    format!("{:?}", key.permission),
                    key.created_at.to_rfc3339(),
                    key.expires_at.map(|e| e.to_rfc3339()),
                    key.last_used_at.map(|l| l.to_rfc3339()),
                ],
            ) {
                warn!("Failed to persist API key to SQLite: {}", e);
            }
        }
    }

    fn delete_key_from_db(&self, id: &str) {
        if let Some(ref db) = self.db {
            let conn = db.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Err(e) = conn.execute("DELETE FROM api_keys WHERE id = ?1", params![id]) {
                warn!("Failed to delete API key from SQLite: {}", e);
            }
        }
    }

    fn update_last_used_in_db(&self, id: &str, last_used: &str) {
        if let Some(ref db) = self.db {
            let conn = db.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Err(e) = conn.execute(
                "UPDATE api_keys SET last_used_at = ?1 WHERE id = ?2",
                params![last_used, id],
            ) {
                warn!("Failed to update API key last_used_at in SQLite: {}", e);
            }
        }
    }

    /// Load all API keys from a `SQLite` connection into memory.
    pub fn load_all_from_db(conn: &rusqlite::Connection) -> Result<Vec<ApiKey>, rusqlite::Error> {
        let mut stmt = conn.prepare(
            "SELECT id, name, key_hash, user_id, permission, created_at, expires_at, last_used_at FROM api_keys",
        )?;
        let rows = stmt.query_map([], |row| {
            let perm_str: String = row.get(4)?;
            let permission = match perm_str.as_str() {
                "Admin" => ApiKeyPermission::Admin,
                "Write" => ApiKeyPermission::Write,
                _ => ApiKeyPermission::Read,
            };
            let created_str: String = row.get(5)?;
            let created_at = chrono::DateTime::parse_from_rfc3339(&created_str)
                .map_or_else(|_| Utc::now(), |dt| dt.with_timezone(&chrono::Utc));
            let expires_str: Option<String> = row.get(6)?;
            let expires_at = expires_str.and_then(|s| {
                chrono::DateTime::parse_from_rfc3339(&s)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .ok()
            });
            let last_used_str: Option<String> = row.get(7)?;
            let last_used_at = last_used_str.and_then(|s| {
                chrono::DateTime::parse_from_rfc3339(&s)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .ok()
            });
            Ok(ApiKey {
                id: row.get(0)?,
                name: row.get(1)?,
                key_hash: row.get(2)?,
                user_id: row.get(3)?,
                permission,
                created_at,
                expires_at,
                last_used_at,
            })
        })?;
        let mut keys = Vec::new();
        for row in rows {
            keys.push(row?);
        }
        Ok(keys)
    }

    /// Load a single key into the in-memory store (used during DB restore).
    pub fn load_key(&self, key: ApiKey) {
        let user_id = key.user_id.clone();
        let id = key.id.clone();
        let hash = key.key_hash.clone();
        self.keys.insert(id.clone(), key);
        self.hash_index.insert(hash, id.clone());
        self.user_keys.entry(user_id).or_default().push(id);
    }
}

impl Default for InMemoryApiKeyStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ApiKeyStoreTrait for InMemoryApiKeyStore {
    async fn create_key(&self, user_id: &str, request: CreateApiKeyRequest) -> Result<(ApiKey, String), ApiKeyError> {
        // Enforce per-user quota
        if self.count_keys(user_id).await >= MAX_KEYS_PER_USER {
            return Err(ApiKeyError::quota_exceeded(format!(
                "Maximum {MAX_KEYS_PER_USER} API keys per user"
            )));
        }

        let raw_key = generate_raw_key();
        let key_hash = hash_api_key(&raw_key);
        let id = uuid::Uuid::new_v4().to_string();

        let expires_at = request
            .expires_at
            .as_deref()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc));

        let key = ApiKey {
            id: id.clone(),
            name: request.name,
            key_hash,
            user_id: user_id.to_string(),
            permission: request.permission,
            created_at: Utc::now(),
            expires_at,
            last_used_at: None,
        };

        self.keys.insert(id.clone(), key.clone());
        self.hash_index.insert(key.key_hash.clone(), id.clone());
        self.user_keys.entry(user_id.to_string()).or_default().push(id);
        self.persist_key(&key);

        Ok((key, raw_key))
    }

    async fn get_key(&self, id: &str) -> Result<ApiKey, ApiKeyError> {
        self.keys
            .get(id)
            .map(|r| r.value().clone())
            .ok_or_else(|| ApiKeyError::not_found(format!("API key '{id}' not found")))
    }

    async fn list_keys(&self, user_id: &str) -> Vec<ApiKey> {
        if let Some(ids) = self.user_keys.get(user_id) {
            ids.iter()
                .filter_map(|id| self.keys.get(id).map(|r| r.value().clone()))
                .collect()
        } else {
            Vec::new()
        }
    }

    async fn revoke_key(&self, id: &str, requesting_user_id: &str) -> Result<(), ApiKeyError> {
        let key = self.get_key(id).await?;
        if key.user_id != requesting_user_id {
            return Err(ApiKeyError::forbidden("Cannot revoke another user's API key"));
        }
        self.keys.remove(id);
        self.hash_index.remove(&key.key_hash);
        self.delete_key_from_db(id);
        // Clean up user index
        if let Some(mut ids) = self.user_keys.get_mut(&key.user_id) {
            ids.retain(|k| k != id);
        }
        Ok(())
    }

    async fn authenticate(&self, raw_key: &str) -> Result<ApiKey, ApiKeyError> {
        let hash = hash_api_key(raw_key);
        let id = self
            .hash_index
            .iter()
            .find(|entry| entry.key().as_bytes().ct_eq(hash.as_bytes()).into())
            .map(|entry| entry.value().clone())
            .ok_or_else(|| ApiKeyError::forbidden("Invalid API key"))?;

        let mut key = self.get_key(&id).await?;
        if key.is_expired() {
            return Err(ApiKeyError::forbidden("API key has expired"));
        }
        key.touch();
        self.keys.insert(id.clone(), key.clone());
        let last_used = key.last_used_at.map(|t| t.to_rfc3339()).unwrap_or_default();
        self.update_last_used_in_db(&id, &last_used);
        Ok(key)
    }

    async fn count_keys(&self, user_id: &str) -> usize {
        self.user_keys.get(user_id).map_or(0, |r| r.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn store() -> Arc<InMemoryApiKeyStore> {
        Arc::new(InMemoryApiKeyStore::new())
    }

    fn create_request(name: &str) -> CreateApiKeyRequest {
        CreateApiKeyRequest {
            name: name.to_string(),
            permission: ApiKeyPermission::Read,
            expires_at: None,
        }
    }

    #[tokio::test]
    async fn test_create_and_list_keys() {
        let s = store();
        let (key, raw) = s.create_key("user1", create_request("my-key")).await.unwrap();

        assert!(raw.starts_with(KEY_PREFIX));
        assert_eq!(key.name, "my-key");
        assert!(!key.id.is_empty());
        assert_eq!(key.permission, ApiKeyPermission::Read);

        let keys = s.list_keys("user1").await;
        assert_eq!(keys.len(), 1);
    }

    #[tokio::test]
    async fn test_authenticate_valid_key() {
        let s = store();
        let (_, raw) = s.create_key("user1", create_request("auth-test")).await.unwrap();

        let key = s.authenticate(&raw).await.unwrap();
        assert_eq!(key.name, "auth-test");
        assert!(key.last_used_at.is_some());
    }

    #[tokio::test]
    async fn test_authenticate_invalid_key() {
        let s = store();
        let err = s.authenticate("ferro_invalidkey").await.unwrap_err();
        assert_eq!(err.kind, ApiKeyErrorKind::Forbidden);
    }

    #[tokio::test]
    async fn test_revoke_key() {
        let s = store();
        let (key, _raw) = s.create_key("user1", create_request("revokable")).await.unwrap();
        let id = key.id.clone();

        s.revoke_key(&id, "user1").await.unwrap();
        assert!(s.get_key(&id).await.is_err());
        assert!(s.list_keys("user1").await.is_empty());
    }

    #[tokio::test]
    async fn test_revoke_other_users_key_forbidden() {
        let s = store();
        let (key, _) = s.create_key("user1", create_request("mine")).await.unwrap();
        let err = s.revoke_key(&key.id, "user2").await.unwrap_err();
        assert_eq!(err.kind, ApiKeyErrorKind::Forbidden);
    }

    #[tokio::test]
    async fn test_quota_enforced() {
        let s = store();
        for i in 0..MAX_KEYS_PER_USER {
            let req = CreateApiKeyRequest {
                name: format!("key-{}", i),
                permission: ApiKeyPermission::Read,
                expires_at: None,
            };
            s.create_key("user1", req).await.unwrap();
        }
        let err = s.create_key("user1", create_request("overflow")).await.unwrap_err();
        assert_eq!(err.kind, ApiKeyErrorKind::QuotaExceeded);
    }

    #[tokio::test]
    async fn test_expired_key_rejected() {
        let s = store();
        let req = CreateApiKeyRequest {
            name: "expiring".to_string(),
            permission: ApiKeyPermission::Read,
            expires_at: Some("2020-01-01T00:00:00+00:00".to_string()),
        };
        let (_, raw) = s.create_key("user1", req).await.unwrap();
        let err = s.authenticate(&raw).await.unwrap_err();
        assert_eq!(err.kind, ApiKeyErrorKind::Forbidden);
    }

    #[tokio::test]
    async fn test_key_hash_deterministic() {
        let raw = "ferro_abc123";
        let h1 = hash_api_key(raw);
        let h2 = hash_api_key(raw);
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64); // SHA-256 hex
    }

    #[tokio::test]
    async fn test_key_prefix() {
        let raw = generate_raw_key();
        assert!(raw.starts_with(KEY_PREFIX));
        // 32 bytes hex = 64 chars + "ferro_" prefix = 70 chars
        assert_eq!(raw.len(), KEY_PREFIX.len() + 64);
    }

    #[test]
    fn test_permission_allows_action() {
        assert!(ApiKeyPermission::Admin.allows_action("read"));
        assert!(ApiKeyPermission::Admin.allows_action("write"));
        assert!(ApiKeyPermission::Admin.allows_action("delete"));
        assert!(ApiKeyPermission::Admin.allows_action("admin"));

        assert!(ApiKeyPermission::Write.allows_action("read"));
        assert!(ApiKeyPermission::Write.allows_action("write"));
        assert!(ApiKeyPermission::Write.allows_action("delete"));
        assert!(!ApiKeyPermission::Write.allows_action("admin"));

        assert!(ApiKeyPermission::Read.allows_action("read"));
        assert!(ApiKeyPermission::Read.allows_action("list"));
        assert!(!ApiKeyPermission::Read.allows_action("write"));
        assert!(!ApiKeyPermission::Read.allows_action("delete"));
    }

    #[test]
    fn test_extract_api_key_from_header() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("X-API-Key", "ferro_test123".parse().unwrap());
        let key = extract_api_key(&headers, None);
        assert_eq!(key.as_deref(), Some("ferro_test123"));
    }

    #[test]
    fn test_extract_api_key_from_query() {
        let headers = axum::http::HeaderMap::new();
        let key = extract_api_key(&headers, Some("api_key=ferro_qkey"));
        assert_eq!(key.as_deref(), Some("ferro_qkey"));
    }

    #[test]
    fn test_extract_api_key_header_precedence() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("X-API-Key", "ferro_header".parse().unwrap());
        let key = extract_api_key(&headers, Some("api_key=ferro_query"));
        assert_eq!(key.as_deref(), Some("ferro_header"));
    }

    #[test]
    fn test_extract_api_key_none() {
        let headers = axum::http::HeaderMap::new();
        assert!(extract_api_key(&headers, None).is_none());
    }

    #[test]
    fn test_url_decode() {
        assert_eq!(urlencoding_decode("hello+world"), "hello world");
        assert_eq!(urlencoding_decode("%41%42"), "AB");
    }

    #[test]
    fn test_key_serialization() {
        let key = ApiKey {
            id: "k1".into(),
            name: "test".into(),
            key_hash: "hash123".into(),
            user_id: "u1".into(),
            permission: ApiKeyPermission::Write,
            created_at: Utc::now(),
            expires_at: None,
            last_used_at: None,
        };
        let json = serde_json::to_string(&key).unwrap();
        let deser: ApiKey = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.id, key.id);
        assert_eq!(deser.permission, ApiKeyPermission::Write);
    }

    #[tokio::test]
    async fn test_create_key_with_write_permission() {
        let s = store();
        let req = CreateApiKeyRequest {
            name: "write-key".into(),
            permission: ApiKeyPermission::Write,
            expires_at: None,
        };
        let (key, _raw) = s.create_key("user1", req).await.unwrap();
        assert_eq!(key.permission, ApiKeyPermission::Write);
    }

    #[tokio::test]
    async fn test_create_key_with_admin_permission() {
        let s = store();
        let req = CreateApiKeyRequest {
            name: "admin-key".into(),
            permission: ApiKeyPermission::Admin,
            expires_at: None,
        };
        let (key, _raw) = s.create_key("user1", req).await.unwrap();
        assert_eq!(key.permission, ApiKeyPermission::Admin);
    }

    #[tokio::test]
    async fn test_list_keys_empty_for_new_user() {
        let s = store();
        let keys = s.list_keys("newuser").await;
        assert!(keys.is_empty());
    }

    #[tokio::test]
    async fn test_count_keys() {
        let s = store();
        assert_eq!(s.count_keys("user1").await, 0);
        s.create_key("user1", create_request("a")).await.unwrap();
        s.create_key("user1", create_request("b")).await.unwrap();
        assert_eq!(s.count_keys("user1").await, 2);
    }

    #[tokio::test]
    async fn test_revoke_key_removes_from_user_index() {
        let s = store();
        let (key1, _) = s.create_key("user1", create_request("k1")).await.unwrap();
        let (key2, _) = s.create_key("user1", create_request("k2")).await.unwrap();
        assert_eq!(s.count_keys("user1").await, 2);

        s.revoke_key(&key1.id, "user1").await.unwrap();
        assert_eq!(s.count_keys("user1").await, 1);
        let remaining = s.list_keys("user1").await;
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].id, key2.id);

        s.revoke_key(&key2.id, "user1").await.unwrap();
        assert_eq!(s.count_keys("user1").await, 0);
        assert!(s.list_keys("user1").await.is_empty());
    }

    #[tokio::test]
    async fn test_revoke_key_removes_from_hash_index() {
        let s = store();
        let (key, raw) = s.create_key("user1", create_request("k1")).await.unwrap();

        assert!(s.authenticate(&raw).await.is_ok());
        s.revoke_key(&key.id, "user1").await.unwrap();
        assert!(s.authenticate(&raw).await.is_err());
    }

    #[tokio::test]
    async fn test_is_expired_boundary_future() {
        let future = Utc::now() + chrono::Duration::hours(1);
        let key = ApiKey {
            id: "k1".into(),
            name: "test".into(),
            key_hash: "hash".into(),
            user_id: "u1".into(),
            permission: ApiKeyPermission::Read,
            created_at: Utc::now(),
            expires_at: Some(future),
            last_used_at: None,
        };
        assert!(!key.is_expired());
    }

    #[tokio::test]
    async fn test_is_expired_boundary_past() {
        let past = Utc::now() - chrono::Duration::hours(1);
        let key = ApiKey {
            id: "k1".into(),
            name: "test".into(),
            key_hash: "hash".into(),
            user_id: "u1".into(),
            permission: ApiKeyPermission::Read,
            created_at: Utc::now(),
            expires_at: Some(past),
            last_used_at: None,
        };
        assert!(key.is_expired());
    }

    #[tokio::test]
    async fn test_is_expired_none_means_never_expires() {
        let key = ApiKey {
            id: "k1".into(),
            name: "test".into(),
            key_hash: "hash".into(),
            user_id: "u1".into(),
            permission: ApiKeyPermission::Read,
            created_at: Utc::now(),
            expires_at: None,
            last_used_at: None,
        };
        assert!(!key.is_expired());
    }

    #[test]
    fn test_api_key_error_constructors() {
        let e = ApiKeyError::not_found("missing");
        assert_eq!(e.kind, ApiKeyErrorKind::NotFound);
        assert_eq!(e.message, "missing");

        let e = ApiKeyError::forbidden("no access");
        assert_eq!(e.kind, ApiKeyErrorKind::Forbidden);
        assert_eq!(e.message, "no access");

        let e = ApiKeyError::bad_request("invalid");
        assert_eq!(e.kind, ApiKeyErrorKind::BadRequest);
        assert_eq!(e.message, "invalid");

        let e = ApiKeyError::conflict("dup");
        assert_eq!(e.kind, ApiKeyErrorKind::Conflict);
        assert_eq!(e.message, "dup");

        let e = ApiKeyError::quota_exceeded("full");
        assert_eq!(e.kind, ApiKeyErrorKind::QuotaExceeded);
        assert_eq!(e.message, "full");
    }

    #[test]
    fn test_url_decode_incomplete_percent() {
        assert_eq!(urlencoding_decode("test%2"), "test%2");
        assert_eq!(urlencoding_decode("test%"), "test%");
    }

    #[test]
    fn test_url_decode_no_plus_no_percent() {
        assert_eq!(urlencoding_decode("hello"), "hello");
        assert_eq!(urlencoding_decode(""), "");
    }

    #[test]
    fn test_url_decode_mixed() {
        assert_eq!(urlencoding_decode("a+b%2Fc"), "a b/c");
    }

    #[test]
    fn test_extract_api_key_empty_header_value() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("X-API-Key", "".parse().unwrap());
        assert!(extract_api_key(&headers, None).is_none());
    }

    #[test]
    fn test_extract_api_key_empty_query_value() {
        let headers = axum::http::HeaderMap::new();
        assert!(extract_api_key(&headers, Some("api_key=")).is_none());
    }

    #[test]
    fn test_extract_api_key_query_no_match() {
        let headers = axum::http::HeaderMap::new();
        assert!(extract_api_key(&headers, Some("other_key=value")).is_none());
    }

    #[test]
    fn test_extract_api_key_query_url_encoded() {
        let headers = axum::http::HeaderMap::new();
        let key = extract_api_key(&headers, Some("api_key=ferro%20key"));
        assert_eq!(key.as_deref(), Some("ferro key"));
    }

    #[test]
    fn test_api_key_touch() {
        let mut key = ApiKey {
            id: "k1".into(),
            name: "test".into(),
            key_hash: "hash".into(),
            user_id: "u1".into(),
            permission: ApiKeyPermission::Read,
            created_at: Utc::now(),
            expires_at: None,
            last_used_at: None,
        };
        assert!(key.last_used_at.is_none());
        key.touch();
        assert!(key.last_used_at.is_some());
    }

    #[test]
    fn test_api_key_created_response_zeroize() {
        let mut resp = ApiKeyCreatedResponse {
            id: "id".into(),
            name: "name".into(),
            permission: ApiKeyPermission::Read,
            created_at: Utc::now(),
            expires_at: None,
            raw_key: "ferro_secretkey12345".into(),
        };
        resp.zeroize();
        assert!(resp.raw_key.is_empty());
    }

    #[test]
    fn test_api_key_permission_serialization_roundtrip() {
        let perms = vec![ApiKeyPermission::Read, ApiKeyPermission::Write, ApiKeyPermission::Admin];
        for perm in perms {
            let json = serde_json::to_string(&perm).unwrap();
            let deser: ApiKeyPermission = serde_json::from_str(&json).unwrap();
            assert_eq!(deser, perm);
        }
    }

    #[test]
    fn test_api_key_permission_default() {
        let perm = ApiKeyPermission::default();
        assert_eq!(perm, ApiKeyPermission::Read);
    }

    #[tokio::test]
    async fn test_in_memory_api_key_store_default() {
        let s = InMemoryApiKeyStore::default();
        assert!(s.list_keys("anyone").await.is_empty());
    }

    #[tokio::test]
    async fn test_load_key_into_store() {
        let s = InMemoryApiKeyStore::new();
        let key = ApiKey {
            id: "loaded-id".into(),
            name: "loaded".into(),
            key_hash: "hash123".into(),
            user_id: "user1".into(),
            permission: ApiKeyPermission::Admin,
            created_at: Utc::now(),
            expires_at: None,
            last_used_at: None,
        };
        s.load_key(key.clone());
        let fetched = s.get_key("loaded-id").await.unwrap();
        assert_eq!(fetched.name, "loaded");
        assert_eq!(fetched.permission, ApiKeyPermission::Admin);
        assert_eq!(s.count_keys("user1").await, 1);
    }

    #[tokio::test]
    async fn test_get_key_not_found() {
        let s = InMemoryApiKeyStore::new();
        let err = s.get_key("nonexistent").await.unwrap_err();
        assert_eq!(err.kind, ApiKeyErrorKind::NotFound);
    }

    #[tokio::test]
    async fn test_revoke_nonexistent_key() {
        let s = InMemoryApiKeyStore::new();
        let err = s.revoke_key("nonexistent", "user1").await.unwrap_err();
        assert_eq!(err.kind, ApiKeyErrorKind::NotFound);
    }

    #[tokio::test]
    async fn test_authenticate_updates_last_used() {
        let s = InMemoryApiKeyStore::new();
        let (key, raw) = s.create_key("u1", create_request("k")).await.unwrap();
        assert!(key.last_used_at.is_none());
        let authenticated = s.authenticate(&raw).await.unwrap();
        assert!(authenticated.last_used_at.is_some());
    }

    #[tokio::test]
    async fn test_create_key_with_valid_expiry() {
        let s = InMemoryApiKeyStore::new();
        let req = CreateApiKeyRequest {
            name: "future-key".into(),
            permission: ApiKeyPermission::Read,
            expires_at: Some("2099-12-31T23:59:59+00:00".to_string()),
        };
        let (key, _) = s.create_key("u1", req).await.unwrap();
        assert!(key.expires_at.is_some());
        assert!(!key.is_expired());
    }

    #[tokio::test]
    async fn test_create_key_with_invalid_expiry_format() {
        let s = InMemoryApiKeyStore::new();
        let req = CreateApiKeyRequest {
            name: "bad-expiry".into(),
            permission: ApiKeyPermission::Read,
            expires_at: Some("not-a-date".to_string()),
        };
        let (key, _) = s.create_key("u1", req).await.unwrap();
        assert!(key.expires_at.is_none());
    }

    #[test]
    fn test_api_key_debug_format() {
        let key = ApiKey {
            id: "k1".into(),
            name: "test".into(),
            key_hash: "hash".into(),
            user_id: "u1".into(),
            permission: ApiKeyPermission::Read,
            created_at: Utc::now(),
            expires_at: None,
            last_used_at: None,
        };
        let debug = format!("{:?}", key);
        assert!(debug.contains("ApiKey"));
        assert!(debug.contains("k1"));
    }

    #[test]
    fn test_api_key_error_debug_format() {
        let e = ApiKeyError::not_found("test");
        let debug = format!("{:?}", e);
        assert!(debug.contains("ApiKeyError"));
    }

    #[test]
    fn test_api_key_clone() {
        let key = ApiKey {
            id: "k1".into(),
            name: "test".into(),
            key_hash: "hash".into(),
            user_id: "u1".into(),
            permission: ApiKeyPermission::Write,
            created_at: Utc::now(),
            expires_at: None,
            last_used_at: None,
        };
        let cloned = key.clone();
        assert_eq!(cloned.id, key.id);
        assert_eq!(cloned.permission, key.permission);
    }
}
