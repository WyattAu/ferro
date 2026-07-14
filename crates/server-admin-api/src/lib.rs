pub mod activity;
pub mod admin_api;
pub mod backup;
pub mod branding;
pub mod gdpr;

use std::sync::Arc;
use std::sync::atomic::AtomicBool;

pub use common::DbHandle;

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

/// Minimal audit log trait for admin handlers that need to record audit events.
#[async_trait::async_trait]
pub trait AuditLogTrait: Send + Sync {
    async fn log(&self, entry: AuditEntry);
    async fn entries(&self) -> Vec<AuditEntry>;
    async fn verify_chain(&self) -> Option<serde_json::Value>;
}

pub use ferro_server_security_middleware::api_error::ApiError;

/// Minimal share store trait for admin handlers.
#[async_trait::async_trait]
pub trait AdminShareStoreTrait: Send + Sync {
    async fn list(&self) -> Vec<AdminShareLink>;
    async fn delete(&self, token: &str) -> bool;
}

/// Share link representation for admin handlers.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminShareLink {
    pub token: String,
    pub path: String,
    pub expires_at: String,
    pub max_downloads: Option<u32>,
    pub download_count: u32,
    pub created_by: String,
    pub allow_download: Option<bool>,
    pub allow_upload: Option<bool>,
}

/// Minimal favorite store trait for admin handlers.
#[async_trait::async_trait]
pub trait AdminFavoriteStoreTrait: Send + Sync {
    async fn list(&self) -> Vec<String>;
    async fn remove(&self, path: &str);
}

/// Minimal tag store trait for admin handlers.
pub trait AdminTagStoreTrait: Send + Sync {
    fn all_tags(&self) -> Vec<(String, Vec<String>)>;
    fn all_tag_pairs(&self) -> Vec<(String, String)>;
    fn remove_tag(&self, path: &str, tag: &str) -> bool;
}

/// Trait that AppState must implement for admin handlers.
///
/// This allows admin handler functions to be generic over the trait,
/// avoiding a circular dependency on `ferro-server`.
pub trait AdminState: Send + Sync + Clone + 'static + common::server_context::HasStorage {
    fn started_at(&self) -> std::time::Instant;
    fn oidc_enabled(&self) -> bool;
    fn admin_user_enabled(&self) -> bool;
    fn search_enabled(&self) -> bool;
    fn cedar_enabled(&self) -> bool;
    fn maintenance_mode(&self) -> &Arc<AtomicBool>;
    fn data_dir(&self) -> Option<&str>;
    fn db(&self) -> &Option<DbHandle>;
    fn cas_store(&self) -> Option<&Arc<dyn ferro_core::cas::CasStore>>;
    fn audit_log(&self) -> &Arc<dyn AuditLogTrait>;
    fn user_store(&self) -> &Arc<dyn ferro_auth::users::UserStoreTrait>;
    fn share_store(&self) -> &Arc<dyn AdminShareStoreTrait>;
    fn favorites(&self) -> &Arc<dyn AdminFavoriteStoreTrait>;
    fn tags(&self) -> &Arc<dyn AdminTagStoreTrait>;
    fn branding_store(&self) -> &branding::BrandingStore;
    fn gdpr_store(&self) -> &gdpr::GdprStore;
}

/// Re-export from ferro_core for atomic_write.
pub use ferro_core::fs_util::atomic_write;

/// Helper function to build audit entry.
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

/// Re-export users types from ferro-auth.
pub use ferro_auth::users;
