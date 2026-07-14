pub mod mail_api;
pub mod offline_api;
pub mod offline_wiring;
pub mod push_notifications;
pub mod read_cache;
pub mod remote_mount;

use std::sync::Arc;

use common::storage::StorageEngine;

/// DbHandle: shared SQLite connection handle.
pub use common::DbHandle;

pub use ferro_server_security_middleware::api_error::ApiError;

// ---------------------------------------------------------------------------
// IntegrationsState trait
// ---------------------------------------------------------------------------

/// Trait抽象 AppState 中 integrations 模块需要的字段。
/// 实现在 ferro-server 的 lib.rs 中。
pub trait IntegrationsState: Clone + Send + Sync + 'static {
    fn mail_store(&self) -> &mail_api::MailStore;
    fn push_notification_store(&self) -> &Option<Arc<tokio::sync::RwLock<push_notifications::PushNotificationStore>>>;
    fn push_notification_config(&self) -> &push_notifications::PushNotificationConfig;
    fn connection_monitor(&self) -> &Arc<ferro_offline::monitor::ConnectionMonitor>;
    fn offline_cache(&self) -> &Arc<tokio::sync::RwLock<ferro_offline::cache::ContentCache>>;
    fn offline_queue(&self) -> &Option<Arc<ferro_offline::change_queue::SqliteChangeQueue>>;
    fn storage(&self) -> &Arc<dyn StorageEngine>;
    fn read_cache(&self) -> &Arc<crate::read_cache::ReadCache>;
    fn remote_mounts(&self) -> &Arc<crate::remote_mount::RemoteMountStore>;
}
