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

/// API error type for compliance handlers.
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

    pub fn internal(code: &str, message: impl Into<String>) -> axum::response::Response {
        ferro_server_security::ApiError::internal(code, message)
    }

    pub const BAD_REQUEST: &'static str = "BAD_REQUEST";
    pub const NOT_FOUND: &'static str = "NOT_FOUND";
    pub const INTERNAL_ERROR: &'static str = "INTERNAL_ERROR";
}

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
