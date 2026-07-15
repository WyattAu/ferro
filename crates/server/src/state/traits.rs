use crate::state::AppState;
use common::metadata::FileMetadata;
use common::storage::LockManagerTrait;
use common::storage::StorageEngine;
use dashmap::{DashMap, DashSet};
use ferro_core::wasm::WasmWorkerRuntime;
use std::sync::Arc;
use tracing::warn;

// ---------------------------------------------------------------------------
// SecurityAppState
// ---------------------------------------------------------------------------

impl ferro_server_security::SecurityAppState for AppState {
    fn auth_attempt_tracker(&self) -> &std::sync::Arc<ferro_server_security::AuthAttemptTracker> {
        &self.auth_attempt_tracker
    }

    fn login_rate_limiter(&self) -> &std::sync::Arc<ferro_server_security::LoginRateLimiter> {
        &self.login_rate_limiter
    }

    fn storage(&self) -> &Arc<dyn common::storage::StorageEngine> {
        &self.storage
    }

    fn api_key_store(&self) -> &Arc<dyn ferro_auth::api_keys::ApiKeyStoreTrait> {
        &self.api_key_store
    }

    fn admin_user(&self) -> &Option<String> {
        &self.admin_user
    }

    fn admin_password(&self) -> &Option<String> {
        &self.admin_password
    }

    fn user_store(&self) -> &Arc<dyn ferro_auth::users::UserStoreTrait> {
        &self.user_store
    }

    fn db(&self) -> &Option<ferro_server_security::DbHandle> {
        &self.db
    }

    #[cfg(feature = "webauthn")]
    fn webauthn_store(&self) -> &Arc<tokio::sync::RwLock<ferro_auth::webauthn::WebAuthnStore>> {
        &self.webauthn_store
    }
}

// ---------------------------------------------------------------------------
// CollaborationState
// ---------------------------------------------------------------------------

impl ferro_server_collaboration::CollaborationState for AppState {
    fn admin_user(&self) -> Option<&str> {
        self.admin_user.as_deref()
    }

    fn audit_log(&self) -> &Arc<dyn ferro_server_collaboration::AuditLogTrait> {
        &self.collab_audit_adapter
    }

    fn comments(&self) -> &Arc<crate::comments::CommentStore> {
        &self.comments
    }

    fn tags(&self) -> &Arc<ferro_server_collaboration::tags::TagStore> {
        &self.tags
    }

    fn storage(&self) -> &Arc<dyn common::storage::StorageEngine> {
        &self.storage
    }

    fn collab_rooms(&self) -> &crate::collab_ws::CollabRoomManager {
        &self.collab_rooms
    }

    fn db(&self) -> &Option<ferro_server_collaboration::DbHandle> {
        &self.db
    }
}

// ---------------------------------------------------------------------------
// UserMgmtState
// ---------------------------------------------------------------------------

impl ferro_server_user_mgmt::UserMgmtState for AppState {
    fn user_info(&self, username: &str) -> Option<ferro_auth::users::UserInfo> {
        self.user_info(username)
    }

    fn admin_user(&self) -> &Option<String> {
        &self.admin_user
    }

    fn user_store(&self) -> &Arc<dyn ferro_auth::users::UserStoreTrait> {
        &self.user_store
    }

    fn db(&self) -> &Option<ferro_server_user_mgmt::DbHandle> {
        // DbHandle is the same type alias in both crates: Arc<Mutex<Connection>>
        &self.db
    }

    fn audit_log(&self) -> &Arc<dyn ferro_server_user_mgmt::AuditLog> {
        &self.user_mgmt_audit_adapter
    }

    fn push_notification_store(
        &self,
    ) -> &Option<Arc<tokio::sync::RwLock<ferro_server_integrations::push_notifications::PushNotificationStore>>> {
        &self.push_notification_store
    }

    fn push_notification_config(&self) -> &ferro_server_integrations::push_notifications::PushNotificationConfig {
        &self.push_notification_config
    }
}

// ---------------------------------------------------------------------------
// Composite trait implementations for crate decomposition
// ---------------------------------------------------------------------------

impl common::server_context::HasUptime for AppState {
    fn started_at(&self) -> std::time::Instant {
        self.started_at
    }
}

impl common::server_context::HasFavorites for AppState {
    fn list_favorites(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = Vec<String>> + Send + '_>> {
        Box::pin(async move { self.favorites.list().await })
    }
    fn add_favorite(&self, path: String) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + '_>> {
        Box::pin(async move { self.favorites.add(path).await })
    }
    fn remove_favorite(&self, path: &str) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + '_>> {
        let path = path.to_owned();
        Box::pin(async move { self.favorites.remove(&path).await })
    }
}

impl common::server_context::HasStorage for AppState {
    fn storage(&self) -> &Arc<dyn StorageEngine> {
        &self.storage
    }
}

impl common::server_context::HasLockManager for AppState {
    fn lock_manager(&self) -> &Arc<dyn LockManagerTrait> {
        &self.lock_manager
    }
}

impl common::server_context::HasBodyLimits for AppState {
    fn max_body_size(&self) -> u64 {
        self.max_body_size
    }
}

impl common::server_context::HasMaintenanceMode for AppState {
    fn maintenance_mode(&self) -> &Arc<std::sync::atomic::AtomicBool> {
        &self.maintenance_mode
    }
}

impl common::server_context::HasStartupState for AppState {
    fn startup_complete(&self) -> &Arc<std::sync::atomic::AtomicBool> {
        &self.startup_complete
    }
}

impl common::server_context::HasMetrics for AppState {
    fn request_count(&self) -> &Arc<std::sync::atomic::AtomicU64> {
        &self.request_count
    }
    fn storage_op_counts(&self) -> &Arc<[std::sync::atomic::AtomicU64; 6]> {
        &self.storage_op_counts
    }
}

impl common::server_context::HasExternalUrl for AppState {
    fn external_url(&self) -> &str {
        &self.external_url
    }
}

impl common::server_context::HasAdminCreds for AppState {
    fn admin_user(&self) -> Option<&str> {
        self.admin_user.as_deref()
    }
    fn admin_password(&self) -> Option<&str> {
        self.admin_password.as_deref()
    }
    fn admin_password_rotated(&self) -> &Arc<std::sync::atomic::AtomicBool> {
        &self.admin_password_rotated
    }
}

impl common::server_context::HasDataDir for AppState {
    fn data_dir(&self) -> Option<&str> {
        self.data_dir.as_deref()
    }
}

impl common::server_context::HasDedupConfig for AppState {
    fn dedup_enabled(&self) -> bool {
        self.dedup_enabled
    }
}

impl common::server_context::HasStreamingConfig for AppState {
    fn streaming_upload_threshold(&self) -> u64 {
        self.streaming_upload_threshold
    }
}

impl common::server_context::HasTrash for AppState {
    fn trash_dir(&self) -> Option<&str> {
        self.trash_dir.as_deref()
    }
    fn max_file_versions(&self) -> u64 {
        self.max_file_versions
    }
}

impl common::server_context::HasQuota for AppState {
    fn quota_bytes(&self) -> Option<u64> {
        self.quota_bytes
    }
    fn used_bytes(&self) -> &Arc<std::sync::atomic::AtomicU64> {
        &self.used_bytes
    }
    fn file_count(&self) -> &Arc<std::sync::atomic::AtomicU64> {
        &self.file_count
    }
}

impl common::server_context::HasWopi for AppState {
    fn wopi_token_secret(&self) -> &str {
        &self.wopi_token_secret
    }
    fn wopi_office_url(&self) -> &str {
        &self.wopi_office_url
    }
}

impl common::server_context::HasThumbnailConfig for AppState {
    fn thumbnail_size(&self) -> u32 {
        self.thumbnail_size
    }
}

impl common::server_context::HasRateLimitConfig for AppState {
    fn rate_limit_burst(&self) -> u32 {
        self.rate_limit_burst
    }
    fn rate_limit_refill(&self) -> u32 {
        self.rate_limit_refill
    }
    fn max_concurrent_requests(&self) -> usize {
        self.max_concurrent_requests
    }
}

impl common::server_context::HasSnapshotConfig for AppState {
    fn max_snapshot_versions(&self) -> usize {
        self.max_snapshot_versions
    }
}

impl common::server_context::HasStorageHealth for AppState {
    fn any_unhealthy(&self) -> bool {
        self.storage_health.any_unhealthy()
    }
}

// ---------------------------------------------------------------------------
// WebDavCoreState trait implementations
// ---------------------------------------------------------------------------

impl ferro_server_webdav_core::HasWasm for AppState {
    fn wasm_runtime(&self) -> Option<&Arc<WasmWorkerRuntime>> {
        self.wasm_runtime.as_ref()
    }
    fn wasm_dispatch_count(&self) -> &Arc<std::sync::atomic::AtomicU64> {
        &self.wasm_dispatch_count
    }
    fn wasm_error_count(&self) -> &Arc<std::sync::atomic::AtomicU64> {
        &self.wasm_error_count
    }
    fn wasm_fuel_total(&self) -> &Arc<std::sync::atomic::AtomicU64> {
        &self.wasm_fuel_total
    }
    fn recently_processed(&self) -> &DashSet<String> {
        &self.recently_processed
    }
}

impl ferro_server_webdav_core::HasSyncOps for AppState {
    fn sync_clock(&self) -> &Arc<std::sync::atomic::AtomicU64> {
        &self.sync_clock
    }
    fn record_sync_op(
        &self,
        op_type: ferro_server_webdav_core::WebdavOpType,
        path: &str,
        new_path: Option<&str>,
        size: u64,
        mime_type: Option<&str>,
        owner: &str,
        checksum: &str,
    ) {
        let mapped = match op_type {
            ferro_server_webdav_core::WebdavOpType::Create => crate::sync::ops::OpType::Create,
            ferro_server_webdav_core::WebdavOpType::Update => crate::sync::ops::OpType::Update,
            ferro_server_webdav_core::WebdavOpType::Delete => crate::sync::ops::OpType::Delete,
            ferro_server_webdav_core::WebdavOpType::Rename => crate::sync::ops::OpType::Rename,
        };
        self.record_sync_op(mapped, path, new_path, size, mime_type, owner, checksum);
    }
    fn bump_sync_clock(&self) {
        self.bump_sync_clock();
    }
}

impl ferro_server_webdav_core::HasOffline for AppState {
    fn is_online(&self) -> bool {
        self.connection_monitor.is_online()
    }
    fn offline_cache(&self) -> &Arc<tokio::sync::RwLock<ferro_offline::cache::ContentCache>> {
        &self.offline_cache
    }
    fn offline_queue(&self) -> &Option<Arc<ferro_offline::change_queue::SqliteChangeQueue>> {
        &self.offline_queue
    }
}

#[async_trait::async_trait]
impl ferro_server_webdav_core::HasEventDispatch for AppState {
    async fn dispatch_file_event(&self, event: ferro_server_webdav_core::WebdavFileEvent) {
        let ws_event = match event.op_type {
            "create" => ferro_server_api_core::ws::WsEvent::FileCreated {
                path: event.path,
                size: event.size.unwrap_or(0),
                owner: event.owner,
            },
            "update" => ferro_server_api_core::ws::WsEvent::FileUpdated {
                path: event.path,
                size: event.size.unwrap_or(0),
                owner: event.owner,
            },
            "delete" => ferro_server_api_core::ws::WsEvent::FileDeleted {
                path: event.path,
                owner: event.owner,
            },
            "move" => ferro_server_api_core::ws::WsEvent::FileMoved {
                from: event.path,
                to: event.new_path.unwrap_or_default(),
                owner: event.owner,
            },
            _ => return,
        };
        self.ws_manager.broadcast(&ws_event);
    }

    async fn fire_event_triggers(
        &self,
        event_type: ferro_server_webdav_core::WebdavEventType,
        path: &str,
        owner: &str,
    ) {
        let api_event_type = match event_type {
            ferro_server_webdav_core::WebdavEventType::FileUploaded => crate::event_triggers::EventType::FileUploaded,
            ferro_server_webdav_core::WebdavEventType::FileModified => crate::event_triggers::EventType::FileModified,
            ferro_server_webdav_core::WebdavEventType::FileDeleted => crate::event_triggers::EventType::FileDeleted,
        };
        crate::event_triggers::fire_event_triggers(self, api_event_type, path, owner).await;
    }

    async fn index_file_with_content(&self, metadata: &FileMetadata, content: &[u8]) {
        crate::indexer::index_file_with_content(self, metadata, content).await;
    }

    async fn remove_file_from_index(&self, path: &str) {
        if let Some(search_lock) = &self.search
            && let Ok(mut engine) = search_lock.try_write()
        {
            if let Err(e) = engine.remove(path) {
                warn!("Failed to remove {} from search index: {}", path, e);
            }
            if let Err(e) = engine.commit() {
                warn!("Failed to commit search index after removal: {}", e);
            }
        }
    }
}

impl ferro_server_webdav_core::HasWebDavStores for AppState {
    fn cas_store(&self) -> Option<&Arc<dyn ferro_core::cas::CasStore>> {
        self.cas_store.as_ref()
    }
    fn metadata_store(&self) -> Option<&Arc<dyn ferro_core::metadata::MetadataStore>> {
        self.metadata_store.as_ref()
    }
    fn thumbnail_cache_invalidate(&self, path: &str) {
        self.thumbnail_cache.invalidate(path);
    }
    fn load_worm_policies(&self) -> Vec<ferro_server_compliance::worm::WormPolicy> {
        crate::worm::load_policies(self)
    }
    fn is_worm_protected(&self, path: &str) -> bool {
        let policies = crate::worm::load_policies(self);
        crate::worm::is_worm_protected(path, &policies)
    }
    fn enforce_quota(
        &self,
        content_length: u64,
    ) -> impl std::future::Future<Output = Result<(), axum::response::Response>> + Send {
        let result = crate::quota::enforce_quota(self, content_length);
        async move { result.map_err(|e| *e) }
    }
    fn calendar_store(&self) -> &Arc<dyn ferro_dav::store::CalendarStore> {
        &self.calendar_store
    }
    fn address_book_store(&self) -> &Arc<dyn ferro_dav::store::AddressBookStore> {
        &self.address_book_store
    }
}

impl ferro_server_webdav_core::HasTrashStore for AppState {
    fn trash_store(&self) -> &ferro_server_webdav_core::trash::TrashStore {
        &self.trash_store
    }
}

impl ferro_server_webdav_core::WebDavCoreState for AppState {
    fn admin_user(&self) -> Option<&str> {
        self.admin_user.as_deref()
    }
}

// ---------------------------------------------------------------------------
// WatermarkState
// ---------------------------------------------------------------------------

impl ferro_server_content::watermark_api::WatermarkState for AppState {
    fn storage(&self) -> &Arc<dyn common::storage::StorageEngine> {
        &self.storage
    }

    fn db(&self) -> &Option<ferro_server_content::watermark_api::DbHandle> {
        &self.db
    }
}

// ---------------------------------------------------------------------------
// ComplianceState
// ---------------------------------------------------------------------------

impl ferro_server_compliance::ComplianceState for AppState {
    fn used_bytes(&self) -> &Arc<std::sync::atomic::AtomicU64> {
        &self.used_bytes
    }

    fn db(&self) -> &Option<ferro_server_compliance::DbHandle> {
        &self.db
    }

    fn retention_store(&self) -> &ferro_server_compliance::retention::RetentionStore {
        &self.retention_store
    }

    fn worm_store(&self) -> &ferro_server_compliance::worm::WormPolicyStore {
        &self.worm_store
    }

    fn dlp_store(&self) -> &ferro_server_compliance::dlp_api::DlpStore {
        &self.dlp_store
    }

    fn audit_log(&self) -> &Arc<dyn ferro_server_compliance::AuditLogTrait> {
        &self.compliance_audit_adapter
    }
}

// ---------------------------------------------------------------------------
// AdminState
// ---------------------------------------------------------------------------

impl ferro_server_admin_api::AdminState for AppState {
    fn started_at(&self) -> std::time::Instant {
        self.started_at
    }

    fn oidc_enabled(&self) -> bool {
        self.oidc.is_some()
    }

    fn admin_user_enabled(&self) -> bool {
        self.admin_user.is_some()
    }

    fn search_enabled(&self) -> bool {
        self.search.is_some()
    }

    fn cedar_enabled(&self) -> bool {
        self.cedar.is_some()
    }

    fn maintenance_mode(&self) -> &Arc<std::sync::atomic::AtomicBool> {
        &self.maintenance_mode
    }

    fn data_dir(&self) -> Option<&str> {
        self.data_dir.as_deref()
    }

    fn db(&self) -> &Option<ferro_server_admin_api::DbHandle> {
        &self.db
    }

    fn cas_store(&self) -> Option<&Arc<dyn ferro_core::cas::CasStore>> {
        self.cas_store.as_ref()
    }

    fn audit_log(&self) -> &Arc<dyn ferro_server_admin_api::AuditLogTrait> {
        &self.admin_audit_adapter
    }

    fn user_store(&self) -> &Arc<dyn ferro_auth::users::UserStoreTrait> {
        &self.user_store
    }

    fn share_store(&self) -> &Arc<dyn ferro_server_admin_api::AdminShareStoreTrait> {
        &self.admin_share_store
    }

    fn favorites(&self) -> &Arc<dyn ferro_server_admin_api::AdminFavoriteStoreTrait> {
        &self.admin_favorites_store
    }

    fn tags(&self) -> &Arc<dyn ferro_server_admin_api::AdminTagStoreTrait> {
        &self.admin_tags_store
    }

    fn branding_store(&self) -> &ferro_server_admin_api::branding::BrandingStore {
        &self.branding_store
    }

    fn gdpr_store(&self) -> &ferro_server_admin_api::gdpr::GdprStore {
        &self.gdpr_store
    }
}

// ---------------------------------------------------------------------------
// PluginState
// ---------------------------------------------------------------------------

impl ferro_server_plugins::PluginState for AppState {
    fn plugin_registry(&self) -> &Arc<DashMap<String, ferro_server_plugins::plugin_permissions::PluginManifest>> {
        &self.plugin_registry
    }

    fn workers_dir(&self) -> Option<&std::path::PathBuf> {
        self.workers_dir.as_ref()
    }

    fn wasm_runtime(&self) -> Option<&Arc<ferro_core::wasm::WasmWorkerRuntime>> {
        self.wasm_runtime.as_ref()
    }
}

// ---------------------------------------------------------------------------
// IntegrationsState
// ---------------------------------------------------------------------------

impl ferro_server_integrations::IntegrationsState for AppState {
    fn mail_store(&self) -> &ferro_server_integrations::mail_api::MailStore {
        &self.mail_store
    }
    fn push_notification_store(
        &self,
    ) -> &Option<Arc<tokio::sync::RwLock<ferro_server_integrations::push_notifications::PushNotificationStore>>> {
        &self.push_notification_store
    }
    fn push_notification_config(&self) -> &ferro_server_integrations::push_notifications::PushNotificationConfig {
        &self.push_notification_config
    }
    fn connection_monitor(&self) -> &Arc<ferro_offline::monitor::ConnectionMonitor> {
        &self.connection_monitor
    }
    fn offline_cache(&self) -> &Arc<tokio::sync::RwLock<ferro_offline::cache::ContentCache>> {
        &self.offline_cache
    }
    fn offline_queue(&self) -> &Option<Arc<ferro_offline::change_queue::SqliteChangeQueue>> {
        &self.offline_queue
    }
    fn storage(&self) -> &Arc<dyn common::storage::StorageEngine> {
        &self.storage
    }
    fn read_cache(&self) -> &Arc<ferro_server_integrations::read_cache::ReadCache> {
        &self.read_cache
    }
    fn remote_mounts(&self) -> &Arc<ferro_server_integrations::remote_mount::RemoteMountStore> {
        &self.remote_mounts
    }
}

// ---------------------------------------------------------------------------
// ApiCoreState
// ---------------------------------------------------------------------------

impl ferro_server_api_core::ApiCoreState for AppState {
    fn ws_manager(&self) -> &Arc<ferro_server_api_core::ws::WsManager> {
        &self.ws_manager
    }
    fn read_cache(&self) -> &Arc<ferro_server_integrations::read_cache::ReadCache> {
        &self.read_cache
    }
    fn webhooks(&self) -> &Arc<tokio::sync::RwLock<Vec<ferro_server_api_core::webhooks::WebhookConfig>>> {
        &self.webhooks
    }
    fn webhook_delivery_store(&self) -> &ferro_server_api_core::webhooks::WebhookDeliveryStore {
        &self.webhook_delivery_store
    }
    fn email_config(&self) -> &ferro_server_api_core::email::EmailConfig {
        &self.email_config
    }
    fn push_notification_store(
        &self,
    ) -> &Option<Arc<tokio::sync::RwLock<ferro_server_integrations::push_notifications::PushNotificationStore>>> {
        &self.push_notification_store
    }
    fn push_notification_config(&self) -> &ferro_server_integrations::push_notifications::PushNotificationConfig {
        &self.push_notification_config
    }
    fn event_bus(&self) -> &Arc<ferro_event_bus::EventBus> {
        &self.event_bus
    }
    fn wasm_runtime(&self) -> &Option<Arc<ferro_core::wasm::WasmWorkerRuntime>> {
        &self.wasm_runtime
    }
    fn workers_dir(&self) -> &Option<std::path::PathBuf> {
        &self.workers_dir
    }
    fn wasm_dispatch_count(&self) -> &Arc<std::sync::atomic::AtomicU64> {
        &self.wasm_dispatch_count
    }
    fn wasm_error_count(&self) -> &Arc<std::sync::atomic::AtomicU64> {
        &self.wasm_error_count
    }
    fn wasm_fuel_total(&self) -> &Arc<std::sync::atomic::AtomicU64> {
        &self.wasm_fuel_total
    }
    fn db(&self) -> &Option<crate::db::DbHandle> {
        &self.db
    }
    fn search(&self) -> &Option<Arc<tokio::sync::RwLock<ferro_core::search::SearchEngine>>> {
        &self.search
    }
    fn search_ranking_config(&self) -> &Arc<tokio::sync::RwLock<ferro_core::search::SearchRankingConfig>> {
        &self.search_ranking_config
    }
    fn ai_search(&self) -> &Option<Arc<dyn ferro_server_api_core::AiSearchBridgeTrait>> {
        &self.ai_search_bridge
    }
    fn lock_manager(&self) -> &Arc<dyn common::storage::LockManagerTrait> {
        &self.lock_manager
    }
    fn preferences(&self) -> &Arc<dyn crate::search::PreferenceStore> {
        &self.preferences
    }
}

// ---------------------------------------------------------------------------
// ProductivityState
// ---------------------------------------------------------------------------

impl ferro_server_productivity::ProductivityState for AppState {
    fn data_dir(&self) -> Option<&str> {
        self.data_dir.as_deref()
    }
    fn calendar_store(&self) -> &std::sync::Arc<dyn ferro_dav::store::CalendarStore> {
        &self.calendar_store
    }
    fn address_book_store(&self) -> &std::sync::Arc<dyn ferro_dav::store::AddressBookStore> {
        &self.address_book_store
    }
    fn task_store(&self) -> &ferro_server_productivity::tasks::TaskStore {
        &self.task_store
    }
}

// ---------------------------------------------------------------------------
// StorageUtilsState
// ---------------------------------------------------------------------------

impl ferro_server_storage_ops::StorageUtilsState for AppState {
    fn storage(&self) -> &Arc<dyn common::storage::StorageEngine> {
        &self.storage
    }
    fn data_dir(&self) -> Option<&str> {
        self.data_dir.as_deref()
    }
    fn thumbnail_cache(&self) -> &Arc<dyn ferro_server_storage_ops::ThumbnailCacheTrait> {
        &self.thumbnail_cache
    }
    fn thumbnail_size(&self) -> u32 {
        self.thumbnail_size
    }
    fn snapshot_store(&self) -> &Arc<ferro_server_storage_ops::snapshots::SnapshotStore> {
        &self.snapshot_store
    }
    fn storage_health(&self) -> &Arc<ferro_server_storage_ops::storage_health::StorageHealthMonitor> {
        &self.storage_health
    }
}

// ---------------------------------------------------------------------------
// InfraState
// ---------------------------------------------------------------------------

impl ferro_server_infra::InfraState for AppState {
    fn federation_secret(&self) -> &str {
        &self.federation_secret
    }
    fn external_url(&self) -> &str {
        &self.external_url
    }
    fn storage(&self) -> &Arc<dyn common::storage::StorageEngine> {
        &self.storage
    }
    fn activity_store(&self) -> &Arc<ferro_server_activitypub::store::ActivityStore> {
        &self.activity_store
    }
}

// ---------------------------------------------------------------------------
// AuditLogAdapter — bridges crate::audit::AuditLog to ferro_server_state::AuditLogTrait
// ---------------------------------------------------------------------------

pub struct AuditLogAdapter(pub Arc<crate::audit::AuditLog>);

pub fn convert_entry(e: crate::audit::AuditEntry) -> ferro_server_state::AuditEntry {
    ferro_server_state::AuditEntry {
        timestamp: e.timestamp,
        method: e.method,
        path: e.path,
        user: e.user,
        status: e.status,
        client_ip: e.client_ip,
        user_agent: e.user_agent,
        content_length: e.content_length,
    }
}

#[async_trait::async_trait]
impl ferro_server_state::AuditLogTrait for AuditLogAdapter {
    async fn log(&self, entry: ferro_server_state::AuditEntry) {
        self.0
            .log(crate::audit::AuditEntry {
                timestamp: entry.timestamp,
                method: entry.method,
                path: entry.path,
                user: entry.user,
                status: entry.status,
                client_ip: entry.client_ip,
                user_agent: entry.user_agent,
                content_length: entry.content_length,
            })
            .await;
    }

    async fn len(&self) -> usize {
        self.0.len().await
    }

    async fn recent(&self, n: usize) -> Vec<ferro_server_state::AuditEntry> {
        self.0.recent(n).await.into_iter().map(convert_entry).collect()
    }

    async fn recent_with_offset(&self, limit: usize, offset: usize) -> Vec<ferro_server_state::AuditEntry> {
        self.0
            .recent_with_offset(limit, offset)
            .await
            .into_iter()
            .map(convert_entry)
            .collect()
    }

    async fn entries(&self) -> Vec<ferro_server_state::AuditEntry> {
        self.0.entries().await.into_iter().map(convert_entry).collect()
    }
}

// ---------------------------------------------------------------------------
// ServerState for AppState
// ---------------------------------------------------------------------------

impl ferro_server_state::ServerState for AppState {
    fn storage(&self) -> &Arc<dyn common::storage::StorageEngine> {
        &self.storage
    }

    fn lock_manager(&self) -> &Arc<dyn common::storage::LockManagerTrait> {
        &self.lock_manager
    }

    fn db(&self) -> &Option<common::DbHandle> {
        &self.db
    }

    fn admin_user(&self) -> Option<&str> {
        self.admin_user.as_deref()
    }

    fn admin_password(&self) -> Option<&str> {
        self.admin_password.as_deref()
    }

    fn admin_password_rotated(&self) -> &Arc<std::sync::atomic::AtomicBool> {
        &self.admin_password_rotated
    }

    fn user_store(&self) -> &Arc<dyn ferro_auth::users::UserStoreTrait> {
        &self.user_store
    }

    fn api_key_store(&self) -> &Arc<dyn ferro_auth::api_keys::ApiKeyStoreTrait> {
        &self.api_key_store
    }

    fn search(&self) -> &Option<Arc<tokio::sync::RwLock<ferro_core::search::SearchEngine>>> {
        &self.search
    }

    fn preferences(&self) -> &Arc<dyn ferro_server_api_core::search::PreferenceStore> {
        &self.preferences
    }

    fn share_store(&self) -> &Arc<dyn ferro_server_sharing::shares::ShareStoreTrait> {
        &self.share_store
    }

    fn favorites(&self) -> &Arc<dyn ferro_server_sharing::favorites::FavoriteStore> {
        &self.favorites
    }

    fn tags(&self) -> &Arc<ferro_server_collaboration::tags::TagStore> {
        &self.tags
    }

    fn comments(&self) -> &Arc<ferro_server_collaboration::comments::CommentStore> {
        &self.comments
    }

    fn worm_store(&self) -> &ferro_server_compliance::worm::WormPolicyStore {
        &self.worm_store
    }

    fn retention_store(&self) -> &ferro_server_compliance::retention::RetentionStore {
        &self.retention_store
    }

    fn dlp_store(&self) -> &ferro_server_compliance::dlp_api::DlpStore {
        &self.dlp_store
    }

    fn snapshot_store(&self) -> &Arc<ferro_server_storage_ops::snapshots::SnapshotStore> {
        &self.snapshot_store
    }

    fn thumbnail_cache(&self) -> &Arc<dyn ferro_server_storage_ops::ThumbnailCacheTrait> {
        &self.thumbnail_cache
    }

    fn storage_health(&self) -> &Arc<ferro_server_storage_ops::storage_health::StorageHealthMonitor> {
        &self.storage_health
    }

    fn external_url(&self) -> &str {
        &self.external_url
    }

    fn max_body_size(&self) -> u64 {
        self.max_body_size
    }

    fn thumbnail_size(&self) -> u32 {
        self.thumbnail_size
    }

    fn data_dir(&self) -> Option<&str> {
        self.data_dir.as_deref()
    }

    fn max_file_versions(&self) -> u64 {
        self.max_file_versions
    }

    fn quota_bytes(&self) -> Option<u64> {
        self.quota_bytes
    }

    fn request_count(&self) -> &Arc<std::sync::atomic::AtomicU64> {
        &self.request_count
    }

    fn storage_op_counts(&self) -> &Arc<[std::sync::atomic::AtomicU64; 6]> {
        &self.storage_op_counts
    }

    fn maintenance_mode(&self) -> &Arc<std::sync::atomic::AtomicBool> {
        &self.maintenance_mode
    }

    fn startup_complete(&self) -> &Arc<std::sync::atomic::AtomicBool> {
        &self.startup_complete
    }

    fn audit_log(&self) -> &Arc<dyn ferro_server_state::AuditLogTrait> {
        &self.state_audit_adapter
    }

    fn wasm_runtime(&self) -> &Option<Arc<ferro_core::wasm::WasmWorkerRuntime>> {
        &self.wasm_runtime
    }

    fn search_ranking_config(&self) -> &Arc<tokio::sync::RwLock<ferro_core::search::SearchRankingConfig>> {
        &self.search_ranking_config
    }

    fn presigned_generator(&self) -> &Option<Arc<dyn ferro_core::presigned::PresignedUrlGenerator>> {
        &self.presigned_generator
    }

    fn ws_manager(&self) -> &Arc<ferro_server_api_core::ws::WsManager> {
        &self.ws_manager
    }

    fn calendar_store(&self) -> &Arc<dyn ferro_dav::store::CalendarStore> {
        &self.calendar_store
    }

    fn address_book_store(&self) -> &Arc<dyn ferro_dav::store::AddressBookStore> {
        &self.address_book_store
    }

    fn task_store(&self) -> &ferro_server_productivity::tasks::TaskStore {
        &self.task_store
    }

    fn cedar(&self) -> &Option<Arc<ferro_auth::cedar::CedarAuthorizer>> {
        &self.cedar
    }

    fn used_bytes(&self) -> u64 {
        self.used_bytes.load(std::sync::atomic::Ordering::Relaxed)
    }

    fn file_count(&self) -> u64 {
        self.file_count.load(std::sync::atomic::Ordering::Relaxed)
    }

    fn oidc(&self) -> &Option<Arc<ferro_auth::oidc::OidcValidator>> {
        &self.oidc
    }

    fn read_cache(&self) -> &Arc<ferro_server_integrations::read_cache::ReadCache> {
        &self.read_cache
    }

    fn health_checker(&self) -> &Arc<ferro_health::HealthChecker> {
        &self.health_checker
    }

    fn metadata_store(&self) -> &Option<Arc<dyn ferro_core::metadata::MetadataStore>> {
        &self.metadata_store
    }

    fn cas_store(&self) -> &Option<Arc<dyn ferro_core::cas::CasStore>> {
        &self.cas_store
    }

    fn started_at(&self) -> std::time::Instant {
        self.started_at
    }

    fn federation_secret(&self) -> &str {
        &self.federation_secret
    }

    fn activity_store(&self) -> &Arc<ferro_server_activitypub::store::ActivityStore> {
        &self.activity_store
    }

    fn tenant_rate_limit_store(&self) -> &Option<Arc<ferro_rate_limiter::tenant::TenantRateLimitStore>> {
        &self.tenant_rate_limit_store
    }

    fn tenant_rate_limiter(&self) -> &Option<Arc<ferro_rate_limiter::tenant::TenantAwareRateLimiter>> {
        &self.tenant_rate_limiter
    }

    fn selective_sync_store(&self) -> &Option<Arc<ferro_selective_sync::ProfileStore>> {
        &self.selective_sync_store
    }

    fn plugin_registry(
        &self,
    ) -> &Arc<dashmap::DashMap<String, ferro_server_plugins::plugin_permissions::PluginManifest>> {
        &self.plugin_registry
    }

    fn upload_store(
        &self,
    ) -> &Arc<tokio::sync::RwLock<std::collections::HashMap<String, ferro_server_state::ChunkedUpload>>> {
        &self.upload_store
    }

    fn event_bus(&self) -> &Arc<ferro_event_bus::EventBus> {
        &self.event_bus
    }

    fn request_duration_buckets(&self) -> &Arc<[std::sync::atomic::AtomicU64; 11]> {
        &self.request_duration_buckets
    }

    fn request_duration_sum_ms(&self) -> &std::sync::atomic::AtomicU64 {
        &self.request_duration_sum_ms
    }

    fn request_status_counts(&self) -> &Arc<[std::sync::atomic::AtomicU64; 4]> {
        &self.request_status_counts
    }

    fn wasm_dispatch_count(&self) -> &Arc<std::sync::atomic::AtomicU64> {
        &self.wasm_dispatch_count
    }

    fn wasm_error_count(&self) -> &Arc<std::sync::atomic::AtomicU64> {
        &self.wasm_error_count
    }

    fn wasm_fuel_total(&self) -> &Arc<std::sync::atomic::AtomicU64> {
        &self.wasm_fuel_total
    }

    fn auth_enabled(&self) -> bool {
        self.oidc.is_some()
    }

    fn wopi_office_url(&self) -> &str {
        &self.wopi_office_url
    }

    fn webhooks(&self) -> &Arc<tokio::sync::RwLock<Vec<ferro_server_api_core::webhooks::WebhookConfig>>> {
        &self.webhooks
    }

    fn webhook_delivery_store(&self) -> &ferro_server_api_core::webhooks::WebhookDeliveryStore {
        &self.webhook_delivery_store
    }

    fn email_config(&self) -> &ferro_server_api_core::email::EmailConfig {
        &self.email_config
    }

    fn push_notification_store(
        &self,
    ) -> &Option<Arc<tokio::sync::RwLock<ferro_server_integrations::push_notifications::PushNotificationStore>>> {
        &self.push_notification_store
    }

    fn push_notification_config(&self) -> &ferro_server_integrations::push_notifications::PushNotificationConfig {
        &self.push_notification_config
    }

    fn storage_circuit_breaker(&self) -> &ferro_server_resilience::NamedCircuitBreaker {
        &self.storage_circuit_breaker
    }

    fn auth_circuit_breaker(&self) -> &ferro_server_resilience::NamedCircuitBreaker {
        &self.auth_circuit_breaker
    }

    fn ldap_circuit_breaker(&self) -> &ferro_server_resilience::NamedCircuitBreaker {
        &self.ldap_circuit_breaker
    }

    fn bulkhead_pools(&self) -> &ferro_server_resilience::BulkheadPools {
        &self.bulkhead_pools
    }

    fn retry_policy(&self) -> &ferro_server_resilience::RetryPolicy {
        &self.retry_policy
    }

    fn slo_collector(&self) -> &std::sync::Arc<ferro_server_slo::SliCollector> {
        &self.slo_collector
    }

    fn slo_definitions(&self) -> &Vec<ferro_server_slo::SloDefinition> {
        &self.slo_definitions
    }
}
