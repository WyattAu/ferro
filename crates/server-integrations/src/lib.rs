pub mod mail_api;
pub mod offline_api;
pub mod offline_wiring;
pub mod push_notifications;
pub mod read_cache;
pub mod remote_mount;

use std::sync::Arc;

use common::storage::StorageEngine;

/// DbHandle: shared SQLite connection handle.
pub type DbHandle = Arc<std::sync::Mutex<rusqlite::Connection>>;

// ---------------------------------------------------------------------------
// ApiError (local copy matching ferro-server's api_error module)
// ---------------------------------------------------------------------------

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

pub struct ApiError;

impl ApiError {
    pub fn respond(status: StatusCode, code: &str, message: impl Into<String>) -> Response {
        let body = axum::Json(serde_json::json!({
            "error": message.into(),
            "error_code": code,
        }));
        (status, body).into_response()
    }

    pub fn bad_request(code: &str, message: impl Into<String>) -> Response {
        Self::respond(StatusCode::BAD_REQUEST, code, message)
    }

    pub fn not_found(code: &str, message: impl Into<String>) -> Response {
        Self::respond(StatusCode::NOT_FOUND, code, message)
    }

    pub fn conflict(code: &str, message: impl Into<String>) -> Response {
        Self::respond(StatusCode::CONFLICT, code, message)
    }

    pub fn internal(code: &str, message: impl Into<String>) -> Response {
        Self::respond(StatusCode::INTERNAL_SERVER_ERROR, code, message)
    }

    pub fn not_implemented(code: &str, message: impl Into<String>) -> Response {
        Self::respond(StatusCode::NOT_IMPLEMENTED, code, message)
    }

    pub fn bad_gateway(code: &str, message: impl Into<String>) -> Response {
        Self::respond(StatusCode::BAD_GATEWAY, code, message)
    }

    pub const INTERNAL_ERROR: &str = "INTERNAL_ERROR";
    pub const NOT_FOUND: &str = "NOT_FOUND";
    pub const BAD_REQUEST: &str = "BAD_REQUEST";
    pub const CONFLICT: &str = "CONFLICT";
    pub const INVALID_INPUT: &str = "INVALID_INPUT";
    pub const NOT_IMPLEMENTED: &str = "NOT_IMPLEMENTED";
    pub const BAD_GATEWAY: &str = "BAD_GATEWAY";
}

// ---------------------------------------------------------------------------
// IntegrationsState trait
// ---------------------------------------------------------------------------

/// Trait抽象 AppState 中 integrations 模块需要的字段。
/// 实现在 ferro-server 的 lib.rs 中。
pub trait IntegrationsState: Clone + Send + Sync + 'static {
    fn mail_store(&self) -> &mail_api::MailStore;
    fn push_notification_store(
        &self,
    ) -> &Option<Arc<tokio::sync::RwLock<push_notifications::PushNotificationStore>>>;
    fn push_notification_config(&self) -> &push_notifications::PushNotificationConfig;
    fn connection_monitor(&self) -> &Arc<ferro_offline::monitor::ConnectionMonitor>;
    fn offline_cache(&self) -> &Arc<tokio::sync::RwLock<ferro_offline::cache::ContentCache>>;
    fn offline_queue(&self) -> &Option<Arc<ferro_offline::change_queue::SqliteChangeQueue>>;
    fn storage(&self) -> &Arc<dyn StorageEngine>;
    fn read_cache(&self) -> &Arc<crate::read_cache::ReadCache>;
    fn remote_mounts(&self) -> &Arc<crate::remote_mount::RemoteMountStore>;
}
