pub mod batch;
pub mod event_triggers;
pub mod ocr;
pub mod policies;
pub mod push_notifications;
pub mod retention;
pub mod triggers;
pub mod webhooks;
pub mod worm;

use common::storage::StorageEngine;
use ferro_core::wasm::WasmWorkerRuntime;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use tokio::sync::RwLock;

pub type DbHandle = Arc<std::sync::Mutex<rusqlite::Connection>>;

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

pub trait AuditLogger: Send + Sync {
    fn log(
        &self,
        entry: AuditEntry,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + '_>>;
}

#[derive(Clone)]
pub struct AutomationState {
    pub storage: Arc<dyn StorageEngine>,
    pub db: Option<DbHandle>,
    pub webhooks: Arc<RwLock<Vec<webhooks::WebhookConfig>>>,
    pub push_notification_store: Option<Arc<RwLock<push_notifications::PushNotificationStore>>>,
    pub push_notification_config: push_notifications::PushNotificationConfig,
    pub used_bytes: Arc<AtomicU64>,
    pub audit_log: Option<Arc<dyn AuditLogger>>,
    pub cedar: Option<Arc<ferro_auth::cedar::CedarAuthorizer>>,
    pub wasm_runtime: Option<Arc<WasmWorkerRuntime>>,
    pub workers_dir: Option<PathBuf>,
    pub wasm_dispatch_count: Arc<AtomicU64>,
    pub wasm_error_count: Arc<AtomicU64>,
    pub wasm_fuel_total: Arc<AtomicU64>,
}
