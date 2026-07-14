pub mod antivirus_api;
pub mod clamav;
pub mod dlp_api;
pub mod retention;
pub mod worm;

use std::sync::Arc;
use std::sync::atomic::AtomicU64;

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

/// Minimal audit log trait for compliance handlers that need to record audit events.
#[async_trait::async_trait]
pub trait AuditLogTrait: Send + Sync {
    async fn log(&self, entry: AuditEntry);
}

pub use ferro_server_security_middleware::api_error::ApiError;

/// Trait that AppState must implement for compliance handlers.
///
/// This allows compliance handler functions to be generic over the trait,
/// avoiding a circular dependency on `ferro-server`.
pub trait ComplianceState: Send + Sync + Clone + 'static + common::server_context::HasStorage {
    fn used_bytes(&self) -> &Arc<AtomicU64>;
    fn db(&self) -> &Option<DbHandle>;
    fn retention_store(&self) -> &retention::RetentionStore;
    fn worm_store(&self) -> &worm::WormPolicyStore;
    fn dlp_store(&self) -> &dlp_api::DlpStore;
    fn audit_log(&self) -> &Arc<dyn AuditLogTrait>;
}
