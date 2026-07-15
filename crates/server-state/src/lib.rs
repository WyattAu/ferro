pub mod traits;

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64};

use common::storage::{LockManagerTrait, StorageEngine};

/// A single chunked upload session.
#[derive(Debug, Clone)]
pub struct ChunkedUpload {
    pub path: String,
    pub chunk_size: usize,
    pub received_chunks: HashMap<usize, Vec<u8>>,
    pub total_chunks: Option<usize>,
    pub created_at: std::time::Instant,
}

/// A single audit log entry.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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

#[async_trait::async_trait]
#[allow(clippy::len_without_is_empty)]
pub trait AuditLogTrait: Send + Sync {
    async fn log(&self, entry: AuditEntry);
    async fn len(&self) -> usize;
    async fn recent(&self, n: usize) -> Vec<AuditEntry>;
    async fn recent_with_offset(&self, limit: usize, offset: usize) -> Vec<AuditEntry>;
    async fn entries(&self) -> Vec<AuditEntry>;
}

pub trait ServerState: Send + Sync + Clone + 'static {
    fn storage(&self) -> &Arc<dyn StorageEngine>;
    fn lock_manager(&self) -> &Arc<dyn LockManagerTrait>;
    fn db(&self) -> &Option<common::DbHandle>;
    fn admin_user(&self) -> Option<&str>;
    fn admin_password(&self) -> Option<&str>;
    fn admin_password_rotated(&self) -> &Arc<AtomicBool>;
    fn user_store(&self) -> &Arc<dyn ferro_auth::users::UserStoreTrait>;
    fn api_key_store(&self) -> &Arc<dyn ferro_auth::api_keys::ApiKeyStoreTrait>;
    fn search(&self) -> &Option<Arc<tokio::sync::RwLock<ferro_core::search::SearchEngine>>>;
    fn preferences(&self) -> &Arc<dyn ferro_server_api_core::search::PreferenceStore>;
    fn share_store(&self) -> &Arc<dyn ferro_server_sharing::shares::ShareStoreTrait>;
    fn favorites(&self) -> &Arc<dyn ferro_server_sharing::favorites::FavoriteStore>;
    fn tags(&self) -> &Arc<ferro_server_collaboration::tags::TagStore>;
    fn comments(&self) -> &Arc<ferro_server_collaboration::comments::CommentStore>;
    fn worm_store(&self) -> &ferro_server_compliance::worm::WormPolicyStore;
    fn retention_store(&self) -> &ferro_server_compliance::retention::RetentionStore;
    fn dlp_store(&self) -> &ferro_server_compliance::dlp_api::DlpStore;
    fn snapshot_store(&self) -> &Arc<ferro_server_storage_ops::snapshots::SnapshotStore>;
    fn thumbnail_cache(&self) -> &Arc<dyn ferro_server_storage_ops::ThumbnailCacheTrait>;
    fn storage_health(&self) -> &Arc<ferro_server_storage_ops::storage_health::StorageHealthMonitor>;
    fn external_url(&self) -> &str;
    fn max_body_size(&self) -> u64;
    fn thumbnail_size(&self) -> u32;
    fn data_dir(&self) -> Option<&str>;
    fn max_file_versions(&self) -> u64;
    fn quota_bytes(&self) -> Option<u64>;
    fn request_count(&self) -> &Arc<AtomicU64>;
    fn storage_op_counts(&self) -> &Arc<[AtomicU64; 6]>;
    fn maintenance_mode(&self) -> &Arc<AtomicBool>;
    fn startup_complete(&self) -> &Arc<AtomicBool>;
    fn audit_log(&self) -> &Arc<dyn AuditLogTrait>;
    fn wasm_runtime(&self) -> &Option<Arc<ferro_core::wasm::WasmWorkerRuntime>>;
    fn search_ranking_config(&self) -> &Arc<tokio::sync::RwLock<ferro_core::search::SearchRankingConfig>>;
    fn presigned_generator(&self) -> &Option<Arc<dyn ferro_core::presigned::PresignedUrlGenerator>>;
    fn ws_manager(&self) -> &Arc<ferro_server_api_core::ws::WsManager>;
    fn calendar_store(&self) -> &Arc<dyn ferro_dav::store::CalendarStore>;
    fn address_book_store(&self) -> &Arc<dyn ferro_dav::store::AddressBookStore>;
    fn task_store(&self) -> &ferro_server_productivity::tasks::TaskStore;
    fn cedar(&self) -> &Option<Arc<ferro_auth::cedar::CedarAuthorizer>>;
    fn used_bytes(&self) -> u64;
    fn file_count(&self) -> u64;

    // Auth
    fn oidc(&self) -> &Option<Arc<ferro_auth::oidc::OidcValidator>>;

    // Cache/Health
    fn read_cache(&self) -> &Arc<ferro_server_integrations::read_cache::ReadCache>;
    fn health_checker(&self) -> &Arc<ferro_health::HealthChecker>;
    fn metadata_store(&self) -> &Option<Arc<dyn ferro_core::metadata::MetadataStore>>;
    fn cas_store(&self) -> &Option<Arc<dyn ferro_core::cas::CasStore>>;
    fn started_at(&self) -> std::time::Instant;

    // Upload store
    fn upload_store(&self) -> &Arc<tokio::sync::RwLock<HashMap<String, ChunkedUpload>>>;

    // Event bus
    fn event_bus(&self) -> &Arc<ferro_event_bus::EventBus>;

    // Prometheus metrics
    fn request_duration_buckets(&self) -> &Arc<[AtomicU64; 11]>;
    fn request_duration_sum_ms(&self) -> &AtomicU64;
    fn request_status_counts(&self) -> &Arc<[AtomicU64; 4]>;

    // WASM metrics
    fn wasm_dispatch_count(&self) -> &Arc<AtomicU64>;
    fn wasm_error_count(&self) -> &Arc<AtomicU64>;
    fn wasm_fuel_total(&self) -> &Arc<AtomicU64>;

    // Federation
    fn federation_secret(&self) -> &str;
    fn activity_store(&self) -> &Arc<ferro_server_activitypub::store::ActivityStore>;

    // Rate limiting
    fn tenant_rate_limit_store(&self) -> &Option<Arc<ferro_rate_limiter::tenant::TenantRateLimitStore>>;
    fn tenant_rate_limiter(&self) -> &Option<Arc<ferro_rate_limiter::tenant::TenantAwareRateLimiter>>;

    // Selective sync
    fn selective_sync_store(&self) -> &Option<Arc<ferro_selective_sync::ProfileStore>>;

    // Plugins
    fn plugin_registry(
        &self,
    ) -> &Arc<dashmap::DashMap<String, ferro_server_plugins::plugin_permissions::PluginManifest>>;

    // Config helpers
    fn auth_enabled(&self) -> bool;
    fn wopi_office_url(&self) -> &str;

    // Webhooks
    fn webhooks(&self) -> &Arc<tokio::sync::RwLock<Vec<ferro_server_api_core::webhooks::WebhookConfig>>>;
    fn webhook_delivery_store(&self) -> &ferro_server_api_core::webhooks::WebhookDeliveryStore;

    // Email
    fn email_config(&self) -> &ferro_server_api_core::email::EmailConfig;

    // Push notifications
    fn push_notification_store(
        &self,
    ) -> &Option<Arc<tokio::sync::RwLock<ferro_server_integrations::push_notifications::PushNotificationStore>>>;
    fn push_notification_config(&self) -> &ferro_server_integrations::push_notifications::PushNotificationConfig;
}
