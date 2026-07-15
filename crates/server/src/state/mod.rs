mod adapters;
mod builder;
mod db_init;
mod graphql;
mod health_impl;
pub(crate) mod traits;

use common::storage::LockManagerTrait;
use common::storage::StorageEngine;
use dashmap::{DashMap, DashSet};
use std::sync::Arc;

use ferro_auth::api_keys::InMemoryApiKeyStore;
use ferro_auth::cedar::CedarAuthorizer;
use ferro_auth::oidc::OidcValidator;
use ferro_core::search::SearchEngine;
use ferro_core::search::SearchRankingConfig;
use ferro_core::wasm::WasmWorkerRuntime;

use crate::audit::AuditLog;
use crate::users::{InMemoryUserStore, UserStoreTrait};

use crate::db::DbHandle;
use crate::favorites::FavoriteStore;
use crate::search::PreferenceStore;
use crate::shares::ShareStoreTrait;
use crate::sync::ops::SyncStore;

use ferro_server_storage_ops::snapshots::SnapshotStore;
use ferro_server_webdav_core::lock::LockManager;
use ferro_server_webdav_core::trash::TrashStore;

#[derive(Clone)]
pub struct AppState {
    pub storage: Arc<dyn StorageEngine>,
    pub lock_manager: Arc<dyn LockManagerTrait>,
    pub oidc: Option<Arc<OidcValidator>>,
    pub cedar: Option<Arc<CedarAuthorizer>>,
    pub search: Option<Arc<tokio::sync::RwLock<SearchEngine>>>,
    pub search_ranking_config: Arc<tokio::sync::RwLock<SearchRankingConfig>>,
    pub ai_search: Option<Arc<crate::ai_search::AiSearchBridge>>,
    /// AI search bridge as trait object for api-core compatibility.
    pub ai_search_bridge: Option<Arc<dyn ferro_server_api_core::AiSearchBridgeTrait>>,
    pub wasm_runtime: Option<Arc<WasmWorkerRuntime>>,
    pub workers_dir: Option<std::path::PathBuf>,
    pub metadata_store: Option<Arc<dyn ferro_core::metadata::MetadataStore>>,
    pub cas_store: Option<Arc<dyn ferro_core::cas::CasStore>>,
    pub presigned_generator: Option<Arc<dyn ferro_core::presigned::PresignedUrlGenerator>>,
    pub share_store: Arc<dyn ShareStoreTrait>,
    pub audit_log: Arc<AuditLog>,
    pub snapshot_store: Arc<SnapshotStore>,
    pub max_body_size: u64,
    pub external_url: String,
    pub wopi_token_secret: String,
    pub recently_processed: Arc<DashSet<String>>,
    pub wopi_office_url: String,
    pub admin_user: Option<String>,
    pub admin_password: Option<String>,
    /// Set to true when the admin password is changed at runtime via
    /// POST /api/auth/change-password, lifting default-password restrictions.
    pub admin_password_rotated: Arc<std::sync::atomic::AtomicBool>,
    /// When true, all write operations (PUT, DELETE, POST, PATCH, MKCOL, etc.)
    /// return 503 Service Unavailable. GET, HEAD, OPTIONS pass through.
    pub maintenance_mode: Arc<std::sync::atomic::AtomicBool>,
    pub started_at: std::time::Instant,
    pub favorites: Arc<dyn FavoriteStore>,
    pub trash: Arc<DashMap<String, crate::trash::TrashedEntry>>,
    pub trash_dir: Option<String>,
    pub trash_store: TrashStore,
    pub quota_bytes: Option<u64>,
    pub used_bytes: Arc<std::sync::atomic::AtomicU64>,
    pub file_count: Arc<std::sync::atomic::AtomicU64>,
    pub preferences: Arc<dyn PreferenceStore>,
    pub saved_search_store: ferro_server_api_core::search::SavedSearchStore,
    pub read_cache: Arc<ferro_server_integrations::read_cache::ReadCache>,
    pub request_count: Arc<std::sync::atomic::AtomicU64>,
    /// Request duration histogram: buckets for <1ms, <5ms, <10ms, <25ms, <50ms,
    /// <100ms, <250ms, <500ms, <1s, <5s, >=5s. Each bucket is an AtomicU64.
    pub request_duration_buckets: Arc<[std::sync::atomic::AtomicU64; 11]>,
    /// Cumulative sum of request durations in milliseconds (for Prometheus histogram _sum).
    pub request_duration_sum_ms: Arc<std::sync::atomic::AtomicU64>,
    /// Per-status-code request counters: index 0 = 2xx, 1 = 3xx, 2 = 4xx, 3 = 5xx.
    pub request_status_counts: Arc<[std::sync::atomic::AtomicU64; 4]>,
    /// Storage operation counters: index 0 = PUT, 1 = GET, 2 = DELETE, 3 = LIST, 4 = COPY, 5 = MOVE.
    pub storage_op_counts: Arc<[std::sync::atomic::AtomicU64; 6]>,
    pub sync_clock: Arc<std::sync::atomic::AtomicU64>,
    pub webhooks: Arc<tokio::sync::RwLock<Vec<ferro_server_api_core::webhooks::WebhookConfig>>>,
    pub thumbnail_size: u32,
    pub thumbnail_cache: Arc<dyn ferro_server_storage_ops::ThumbnailCacheTrait>,
    pub data_dir: Option<String>,
    pub user_store: Arc<dyn UserStoreTrait>,
    pub max_file_versions: u64,
    pub calendar_store: Arc<dyn ferro_dav::store::CalendarStore>,
    pub address_book_store: Arc<dyn ferro_dav::store::AddressBookStore>,
    /// NOTE: The following fields are in-memory only (`DashMap`, `AtomicU64`, etc.).
    /// Data stored in these fields is lost on restart. Use `--data-dir` to enable
    /// SQLite-backed persistence so that state survives restarts.
    pub webrtc_offers: Arc<ferro_server_webrtc::offers::OfferStore>,
    pub activity_store: Arc<ferro_server_activitypub::store::ActivityStore>,
    pub federation_secret: String,
    pub sync_store: Arc<SyncStore>,
    pub tags: Arc<ferro_server_collaboration::tags::TagStore>,
    pub comments: Arc<crate::comments::CommentStore>,
    pub idempotency_store: Arc<crate::idempotency::IdempotencyStore>,
    pub storage_health: Arc<ferro_server_storage_ops::storage_health::StorageHealthMonitor>,
    pub ws_manager: Arc<ferro_server_api_core::ws::WsManager>,
    pub collab_rooms: crate::collab_ws::CollabRoomManager,
    pub collab_audit_adapter: Arc<dyn ferro_server_collaboration::AuditLogTrait>,
    pub db: Option<DbHandle>,
    pub branding_store: ferro_server_admin_api::branding::BrandingStore,
    pub task_store: ferro_server_productivity::tasks::TaskStore,
    pub retention_store: ferro_server_compliance::retention::RetentionStore,
    pub dlp_store: ferro_server_compliance::dlp_api::DlpStore,
    pub watermark_db_store: ferro_server_content::watermark_api::WatermarkDbStore,
    pub guest_store: ferro_server_user_mgmt::guests::GuestStore,
    pub gdpr_store: ferro_server_admin_api::gdpr::GdprStore,
    pub upload_store: crate::upload::UploadStore,
    pub worm_store: ferro_server_compliance::worm::WormPolicyStore,
    pub mail_store: ferro_server_integrations::mail_api::MailStore,
    pub notification_prefs_store: crate::notification_prefs_api::NotificationPrefsStore,
    pub webhook_delivery_store: ferro_server_api_core::webhooks::WebhookDeliveryStore,
    pub auth_attempt_tracker: Arc<ferro_server_security_middleware::security::AuthAttemptTracker>,
    pub login_rate_limiter: Arc<ferro_server_security_middleware::security::LoginRateLimiter>,
    /// WASM worker dispatch counter (total executions).
    pub wasm_dispatch_count: Arc<std::sync::atomic::AtomicU64>,
    /// WASM worker error counter (failed executions).
    pub wasm_error_count: Arc<std::sync::atomic::AtomicU64>,
    /// WASM worker total fuel consumed across all executions.
    pub wasm_fuel_total: Arc<std::sync::atomic::AtomicU64>,
    /// Registry of loaded WASM plugins with capability declarations.
    pub plugin_registry: Arc<DashMap<String, ferro_server_plugins::plugin_permissions::PluginManifest>>,
    /// Whether the server has completed startup (CAS verification, DB init, etc.).
    /// Set to true after all startup checks pass in main.rs.
    pub startup_complete: Arc<std::sync::atomic::AtomicBool>,
    pub streaming_upload_threshold: u64,
    pub dedup_enabled: bool,
    pub email_config: ferro_server_api_core::email::EmailConfig,
    pub remote_mounts: Arc<ferro_server_integrations::remote_mount::RemoteMountStore>,
    pub ransomware_detector: Arc<crate::ransomware::RansomwareDetector>,
    pub group_store: Arc<dyn ferro_server_user_mgmt::groups::GroupStoreTrait>,
    pub file_request_store: Arc<dyn ferro_server_api_core::file_requests::FileRequestStoreTrait>,
    #[cfg(feature = "webauthn")]
    pub webauthn_store: Arc<tokio::sync::RwLock<crate::auth::webauthn::WebAuthnStore>>,
    /// Per-tenant rate limit configuration store.
    pub tenant_rate_limit_store: Option<Arc<ferro_rate_limiter::tenant::TenantRateLimitStore>>,
    /// Per-tenant rate limiter instance.
    pub tenant_rate_limiter: Option<Arc<ferro_rate_limiter::tenant::TenantAwareRateLimiter>>,
    /// Connection monitor for offline-first mode.
    pub connection_monitor: Arc<ferro_offline::monitor::ConnectionMonitor>,
    /// SQLite-backed change queue for offline write operations.
    pub offline_queue: Option<Arc<ferro_offline::change_queue::SqliteChangeQueue>>,
    /// In-memory content cache for offline reads.
    pub offline_cache: Arc<tokio::sync::RwLock<ferro_offline::cache::ContentCache>>,
    /// Reconciler for syncing queued changes on reconnection.
    pub offline_reconciler: Arc<ferro_offline::reconciler::Reconciler>,
    /// In-memory API key store for service-to-service and CLI authentication.
    pub api_key_store: Arc<dyn ferro_auth::api_keys::ApiKeyStoreTrait>,
    /// Federation token store for cross-instance API federation.
    /// Push notification store (SQLite-backed, optional).
    pub push_notification_store:
        Option<Arc<tokio::sync::RwLock<ferro_server_integrations::push_notifications::PushNotificationStore>>>,
    /// Push notification configuration (FCM/APNS keys).
    pub push_notification_config: ferro_server_integrations::push_notifications::PushNotificationConfig,
    /// NFS/SMB mount backend (from ferro-mount-nfs crate).
    pub mount_backend: Option<Arc<dyn ferro_mount_nfs::traits::MountBackend>>,
    /// Multi-tenant organization store.
    pub organization_store: Arc<dyn ferro_multi_tenant::organization::OrganizationStore>,
    /// Multi-tenant tenant store.
    pub tenant_store: Arc<dyn ferro_multi_tenant::tenant::TenantStore>,
    /// General-purpose timed cache for metadata and responses (from ferro-cache crate).
    pub metadata_cache: Option<Arc<ferro_cache::TimedCache<String, Vec<u8>>>>,
    /// Event bus for pub/sub event dispatch (from ferro-event-bus crate).
    pub event_bus: Arc<ferro_event_bus::EventBus>,
    /// Structured health checker with configurable probes (from ferro-health crate).
    pub health_checker: Arc<ferro_health::HealthChecker>,
    /// Selective sync profile store (SQLite-backed, optional).
    pub selective_sync_store: Option<Arc<ferro_selective_sync::ProfileStore>>,
    /// Rate limiter burst capacity.
    pub rate_limit_burst: u32,
    /// Rate limiter refill rate per second.
    pub rate_limit_refill: u32,
    /// Maximum concurrent in-flight requests.
    pub max_concurrent_requests: usize,
    /// Maximum number of snapshot versions to retain.
    pub max_snapshot_versions: usize,
    /// Adapter for compliance crate's audit log trait.
    pub compliance_audit_adapter: Arc<dyn ferro_server_compliance::AuditLogTrait>,
    /// Adapter for admin-api crate's audit log trait.
    pub admin_audit_adapter: Arc<dyn ferro_server_admin_api::AuditLogTrait>,
    /// Adapter for admin-api crate's share store trait.
    pub admin_share_store: Arc<dyn ferro_server_admin_api::AdminShareStoreTrait>,
    /// Adapter for admin-api crate's favorite store trait.
    pub admin_favorites_store: Arc<dyn ferro_server_admin_api::AdminFavoriteStoreTrait>,
    /// Adapter for admin-api crate's tag store trait.
    pub admin_tags_store: Arc<dyn ferro_server_admin_api::AdminTagStoreTrait>,
    /// Adapter for user-mgmt crate's audit log trait.
    pub user_mgmt_audit_adapter: Arc<dyn ferro_server_user_mgmt::AuditLog>,
    /// Adapter for ferro-server-state's AuditLogTrait.
    pub state_audit_adapter: Arc<dyn ferro_server_state::AuditLogTrait>,

    // --- Resilience ---
    /// Circuit breaker for storage backend calls.
    pub storage_circuit_breaker: ferro_server_resilience::NamedCircuitBreaker,
    /// Circuit breaker for OIDC/auth validation calls.
    pub auth_circuit_breaker: ferro_server_resilience::NamedCircuitBreaker,
    /// Circuit breaker for LDAP calls.
    pub ldap_circuit_breaker: ferro_server_resilience::NamedCircuitBreaker,
    /// Bulkhead pools for subsystem isolation.
    pub bulkhead_pools: ferro_server_resilience::BulkheadPools,
    /// Default retry policy for transient operations.
    pub retry_policy: ferro_server_resilience::RetryPolicy,
    /// SLO/SLI error budget collector.
    pub slo_collector: Arc<ferro_server_slo::SliCollector>,
    /// SLO definitions for the server.
    pub slo_definitions: Vec<ferro_server_slo::SloDefinition>,

    // --- FIPS 140-2/3 ---
    /// FIPS 140-2/3 runtime validator.
    pub fips_validator: Option<Arc<ferro_server_fips::FipsValidator>>,
    /// Key hierarchy manager for FIPS-compliant key wrapping.
    pub key_manager: Option<Arc<tokio::sync::RwLock<ferro_server_fips::KeyManager>>>,
}

impl AppState {
    pub fn new(storage: Arc<dyn StorageEngine>) -> Self {
        Self {
            storage,
            lock_manager: Arc::new(LockManager::new()),
            oidc: None,
            cedar: None,
            search: None,
            search_ranking_config: Arc::new(tokio::sync::RwLock::new(SearchRankingConfig::default())),
            ai_search: None,
            ai_search_bridge: None,
            wasm_runtime: None,
            workers_dir: None,
            metadata_store: None,
            cas_store: None,
            presigned_generator: None,
            share_store: Arc::new(crate::shares::ShareStore::new()),
            audit_log: Arc::new(AuditLog::new()),
            snapshot_store: Arc::new(SnapshotStore::new(50)),
            rate_limit_burst: 10_000,
            rate_limit_refill: 166,
            max_concurrent_requests: 128,
            max_snapshot_versions: 50,
            max_body_size: 1024 * 1024 * 1024,
            external_url: "http://localhost:8080".to_string(),
            wopi_token_secret: String::new(),
            recently_processed: Arc::new(DashSet::new()),
            wopi_office_url: String::new(),
            admin_user: None,
            admin_password: None,
            admin_password_rotated: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            maintenance_mode: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            started_at: std::time::Instant::now(),
            favorites: Arc::new(crate::favorites::InMemoryFavoriteStore::new()),
            trash: Arc::new(DashMap::new()),
            trash_dir: None,
            trash_store: TrashStore::new(),
            quota_bytes: None,
            used_bytes: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            file_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            preferences: Arc::new(crate::search::InMemoryPreferenceStore::new()),
            saved_search_store: ferro_server_api_core::search::SavedSearchStore::new(),
            read_cache: Arc::new(ferro_server_integrations::read_cache::ReadCache::default()),
            request_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            request_duration_buckets: Arc::new(std::array::from_fn(|_| std::sync::atomic::AtomicU64::new(0))),
            request_duration_sum_ms: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            request_status_counts: Arc::new(std::array::from_fn(|_| std::sync::atomic::AtomicU64::new(0))),
            storage_op_counts: Arc::new(std::array::from_fn(|_| std::sync::atomic::AtomicU64::new(0))),
            sync_clock: Arc::new(std::sync::atomic::AtomicU64::new(1)),
            webhooks: Arc::new(tokio::sync::RwLock::new(Vec::new())),
            data_dir: None,
            thumbnail_size: 256,
            thumbnail_cache: Arc::new(crate::thumbnail_cache::ThumbnailCache::noop()),
            user_store: Arc::new(InMemoryUserStore::new()),
            max_file_versions: 10,
            calendar_store: Arc::new(ferro_dav::store::InMemoryCalendarStore::new()),
            address_book_store: Arc::new(ferro_dav::store::InMemoryAddressBookStore::new()),
            webrtc_offers: Arc::new(ferro_server_webrtc::offers::OfferStore::new()),
            activity_store: Arc::new(ferro_server_activitypub::store::ActivityStore::new()),
            federation_secret: String::new(),
            sync_store: Arc::new(SyncStore::new()),
            tags: Arc::new(ferro_server_collaboration::tags::TagStore::new()),
            comments: Arc::new(crate::comments::CommentStore::new()),
            idempotency_store: Arc::new(crate::idempotency::IdempotencyStore::new()),
            storage_health: Arc::new(ferro_server_storage_ops::storage_health::StorageHealthMonitor::new()),
            ws_manager: Arc::new(ferro_server_api_core::ws::WsManager::new()),
            collab_rooms: crate::collab_ws::CollabRoomManager::new(),
            collab_audit_adapter: Arc::new(adapters::CollaborationAuditLogAdapter(Arc::new(AuditLog::new()))),
            db: None,
            branding_store: ferro_server_admin_api::branding::BrandingStore::new(),
            task_store: ferro_server_productivity::tasks::TaskStore::new(),
            retention_store: ferro_server_compliance::retention::RetentionStore::new(),
            dlp_store: ferro_server_compliance::dlp_api::DlpStore::new(),
            watermark_db_store: ferro_server_content::watermark_api::WatermarkDbStore::new(),
            guest_store: ferro_server_user_mgmt::guests::GuestStore::new(),
            gdpr_store: ferro_server_admin_api::gdpr::GdprStore::new(),
            upload_store: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            worm_store: ferro_server_compliance::worm::WormPolicyStore::new(),
            mail_store: ferro_server_integrations::mail_api::MailStore::new(),
            notification_prefs_store: crate::notification_prefs_api::NotificationPrefsStore::new(),
            webhook_delivery_store: ferro_server_api_core::webhooks::WebhookDeliveryStore::new(),
            auth_attempt_tracker: Arc::new(ferro_server_security_middleware::security::AuthAttemptTracker::default()),
            login_rate_limiter: Arc::new(ferro_server_security_middleware::security::LoginRateLimiter::default()),
            wasm_dispatch_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            wasm_error_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            wasm_fuel_total: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            plugin_registry: Arc::new(DashMap::new()),
            startup_complete: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            streaming_upload_threshold: ferro_server_storage_ops::streaming_upload::DEFAULT_STREAMING_THRESHOLD,
            dedup_enabled: false,
            email_config: ferro_server_api_core::email::EmailConfig::default(),
            remote_mounts: Arc::new(ferro_server_integrations::remote_mount::RemoteMountStore::new()),
            ransomware_detector: Arc::new(crate::ransomware::RansomwareDetector::new(
                crate::ransomware::RansomwareConfig::default(),
            )),
            group_store: Arc::new(ferro_server_user_mgmt::groups::GroupStore::new()),
            file_request_store: Arc::new(ferro_server_api_core::file_requests::FileRequestStore::new()),
            #[cfg(feature = "webauthn")]
            webauthn_store: Arc::new(tokio::sync::RwLock::new(crate::auth::webauthn::WebAuthnStore::new())),
            tenant_rate_limit_store: None,
            tenant_rate_limiter: None,
            connection_monitor: Arc::new(ferro_offline::monitor::ConnectionMonitor::new()),
            offline_queue: None,
            offline_cache: Arc::new(tokio::sync::RwLock::new(ferro_offline::cache::ContentCache::unlimited())),
            offline_reconciler: Arc::new(ferro_offline::reconciler::Reconciler::new()),
            api_key_store: Arc::new(InMemoryApiKeyStore::new()) as Arc<dyn ferro_auth::api_keys::ApiKeyStoreTrait>,
            push_notification_store: None,
            push_notification_config: ferro_server_integrations::push_notifications::PushNotificationConfig::default(),
            mount_backend: None,
            organization_store: Arc::new(ferro_multi_tenant::organization::InMemoryOrganizationStore::new()),
            tenant_store: Arc::new(ferro_multi_tenant::tenant::InMemoryTenantStore::new()),
            metadata_cache: None,
            event_bus: Arc::new(ferro_event_bus::EventBus::new()),
            health_checker: {
                let checker = ferro_health::HealthChecker::new(env!("CARGO_PKG_VERSION"));
                let _ = checker.register(Box::new(ferro_health::MemoryProbe::new(90.0)));
                Arc::new(checker)
            },
            selective_sync_store: None,
            compliance_audit_adapter: Arc::new(adapters::AuditLogAdapter(Arc::new(AuditLog::new()))),
            admin_audit_adapter: Arc::new(adapters::AdminAuditLogAdapter(Arc::new(AuditLog::new()))),
            admin_share_store: Arc::new(adapters::AdminShareStoreAdapter(Arc::new(
                crate::shares::ShareStore::new(),
            ))),
            admin_favorites_store: Arc::new(adapters::AdminFavoriteStoreAdapter(Arc::new(
                crate::favorites::InMemoryFavoriteStore::new(),
            ))),
            admin_tags_store: Arc::new(adapters::AdminTagStoreAdapter(Arc::new(
                ferro_server_collaboration::tags::TagStore::new(),
            ))),
            user_mgmt_audit_adapter: Arc::new(adapters::UserMgmtAuditLogAdapter(Arc::new(AuditLog::new()))),
            state_audit_adapter: Arc::new(traits::AuditLogAdapter(Arc::new(AuditLog::new()))),

            // Resilience defaults
            storage_circuit_breaker: ferro_server_resilience::NamedCircuitBreaker::new(
                "storage",
                ferro_server_resilience::CircuitBreakerConfig::default(),
            ),
            auth_circuit_breaker: ferro_server_resilience::NamedCircuitBreaker::new(
                "auth",
                ferro_server_resilience::CircuitBreakerConfig::default(),
            ),
            ldap_circuit_breaker: ferro_server_resilience::NamedCircuitBreaker::new(
                "ldap",
                ferro_server_resilience::CircuitBreakerConfig::default(),
            ),
            bulkhead_pools: ferro_server_resilience::BulkheadPools::new(
                ferro_server_resilience::BulkheadConfig::default(),
            ),
            retry_policy: ferro_server_resilience::RetryPolicy::default(),
            slo_collector: Arc::new(ferro_server_slo::SliCollector::new()),
            slo_definitions: ferro_server_slo::default_slos(),

            // FIPS 140-2/3 defaults (disabled)
            fips_validator: None,
            key_manager: None,
        }
    }

    pub fn in_memory() -> Self {
        Self::new(Arc::new(crate::storage::InMemoryStorageEngine::new()))
    }

    pub fn record_sync_op(
        &self,
        op_type: crate::sync::ops::OpType,
        path: &str,
        new_path: Option<&str>,
        size: u64,
        mime_type: Option<&str>,
        owner: &str,
        checksum: &str,
    ) {
        self.sync_clock.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let (op_id, clock) = self.sync_store.next_op_id();
        self.sync_store.record_op(crate::sync::ops::SyncOp {
            id: op_id,
            site_id: "local".to_string(),
            clock: crate::sync::clock::VectorClock::new("local").with_counter(clock),
            r#type: op_type,
            path: path.to_string(),
            new_path: new_path.map(|s| s.to_string()),
            size,
            mime_type: mime_type.map(|s| s.to_string()),
            owner: owner.to_string(),
            checksum: checksum.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        });
    }

    pub fn bump_sync_clock(&self) {
        self.sync_clock.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn user_info(&self, username: &str) -> Option<crate::users::UserInfo> {
        match self.user_store.get_user_by_username_blocking(username) {
            Ok(u) if u.is_active() => Some(crate::users::UserInfo::from(&u)),
            _ => {
                if self.admin_user.as_deref() == Some(username) {
                    Some(crate::users::UserInfo {
                        user_id: "admin".to_string(),
                        username: username.to_string(),
                        role: crate::users::UserRole::Admin,
                    })
                } else {
                    None
                }
            }
        }
    }
}
