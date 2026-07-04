use common::metadata::FileMetadata;
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

use tracing::warn;

use crate::audit::AuditLog;
use crate::users::{InMemoryUserStore, UserStoreTrait};

use crate::db::DbHandle;
use crate::favorites::FavoriteStore;
use crate::search::PreferenceStore;
use crate::shares::ShareStoreTrait;
use crate::sync::ops::SyncStore;
use ferro_selective_sync::persistence::ProfileStore as SelectiveSyncProfileStore;

use ferro_server_storage_utils::snapshots::SnapshotStore;
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
    pub thumbnail_cache: Arc<dyn ferro_server_storage_utils::ThumbnailCacheTrait>,
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
    pub storage_health: Arc<ferro_server_storage_utils::storage_health::StorageHealthMonitor>,
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
    pub plugin_registry:
        Arc<DashMap<String, ferro_server_plugins::plugin_permissions::PluginManifest>>,
    /// Whether the server has completed startup (CAS verification, DB init, etc.).
    /// Set to true after all startup checks pass in main.rs.
    pub startup_complete: Arc<std::sync::atomic::AtomicBool>,
    pub streaming_upload_threshold: u64,
    pub dedup_enabled: bool,
    pub email_config: ferro_server_api_core::email::EmailConfig,
    pub remote_mounts: Arc<ferro_server_integrations::remote_mount::RemoteMountStore>,
    pub ransomware_detector: Arc<crate::ransomware::RansomwareDetector>,
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
    pub push_notification_store: Option<
        Arc<
            tokio::sync::RwLock<
                ferro_server_integrations::push_notifications::PushNotificationStore,
            >,
        >,
    >,
    /// Push notification configuration (FCM/APNS keys).
    pub push_notification_config:
        ferro_server_integrations::push_notifications::PushNotificationConfig,
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
}

impl AppState {
    pub fn new(storage: Arc<dyn StorageEngine>) -> Self {
        Self {
            storage,
            lock_manager: Arc::new(LockManager::new()),
            oidc: None,
            cedar: None,
            search: None,
            search_ranking_config: Arc::new(tokio::sync::RwLock::new(
                SearchRankingConfig::default(),
            )),
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
            read_cache: Arc::new(ferro_server_integrations::read_cache::ReadCache::default()),
            request_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            request_duration_buckets: Arc::new(std::array::from_fn(|_| {
                std::sync::atomic::AtomicU64::new(0)
            })),
            request_duration_sum_ms: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            request_status_counts: Arc::new(std::array::from_fn(|_| {
                std::sync::atomic::AtomicU64::new(0)
            })),
            storage_op_counts: Arc::new(std::array::from_fn(|_| {
                std::sync::atomic::AtomicU64::new(0)
            })),
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
            storage_health: Arc::new(
                ferro_server_storage_utils::storage_health::StorageHealthMonitor::new(),
            ),
            ws_manager: Arc::new(ferro_server_api_core::ws::WsManager::new()),
            collab_rooms: crate::collab_ws::CollabRoomManager::new(),
            collab_audit_adapter: Arc::new(CollaborationAuditLogAdapter(Arc::new(AuditLog::new()))),
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
            auth_attempt_tracker: Arc::new(
                ferro_server_security_middleware::security::AuthAttemptTracker::default(),
            ),
            login_rate_limiter: Arc::new(
                ferro_server_security_middleware::security::LoginRateLimiter::default(),
            ),
            wasm_dispatch_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            wasm_error_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            wasm_fuel_total: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            plugin_registry: Arc::new(DashMap::new()),
            startup_complete: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            streaming_upload_threshold:
                ferro_server_storage_ops::streaming_upload::DEFAULT_STREAMING_THRESHOLD,
            dedup_enabled: false,
            email_config: ferro_server_api_core::email::EmailConfig::default(),
            remote_mounts: Arc::new(
                ferro_server_integrations::remote_mount::RemoteMountStore::new(),
            ),
            ransomware_detector: Arc::new(crate::ransomware::RansomwareDetector::new(
                crate::ransomware::RansomwareConfig::default(),
            )),
            #[cfg(feature = "webauthn")]
            webauthn_store: Arc::new(tokio::sync::RwLock::new(
                crate::auth::webauthn::WebAuthnStore::new(),
            )),
            tenant_rate_limit_store: None,
            tenant_rate_limiter: None,
            connection_monitor: Arc::new(ferro_offline::monitor::ConnectionMonitor::new()),
            offline_queue: None,
            offline_cache: Arc::new(tokio::sync::RwLock::new(
                ferro_offline::cache::ContentCache::unlimited(),
            )),
            offline_reconciler: Arc::new(ferro_offline::reconciler::Reconciler::new()),
            api_key_store: Arc::new(InMemoryApiKeyStore::new())
                as Arc<dyn ferro_auth::api_keys::ApiKeyStoreTrait>,
            push_notification_store: None,
            push_notification_config:
                ferro_server_integrations::push_notifications::PushNotificationConfig::default(),
            mount_backend: None,
            organization_store: Arc::new(
                ferro_multi_tenant::organization::InMemoryOrganizationStore::new(),
            ),
            tenant_store: Arc::new(ferro_multi_tenant::tenant::InMemoryTenantStore::new()),
            metadata_cache: None,
            event_bus: Arc::new(ferro_event_bus::EventBus::new()),
            health_checker: {
                let checker = ferro_health::HealthChecker::new(env!("CARGO_PKG_VERSION"));
                let _ = checker.register(Box::new(ferro_health::MemoryProbe::new(90.0)));
                Arc::new(checker)
            },
            selective_sync_store: None,
            compliance_audit_adapter: Arc::new(AuditLogAdapter(Arc::new(AuditLog::new()))),
            admin_audit_adapter: Arc::new(AdminAuditLogAdapter(Arc::new(AuditLog::new()))),
            admin_share_store: Arc::new(AdminShareStoreAdapter(Arc::new(
                crate::shares::ShareStore::new(),
            ))),
            admin_favorites_store: Arc::new(AdminFavoriteStoreAdapter(Arc::new(
                crate::favorites::InMemoryFavoriteStore::new(),
            ))),
            admin_tags_store: Arc::new(AdminTagStoreAdapter(Arc::new(
                ferro_server_collaboration::tags::TagStore::new(),
            ))),
            user_mgmt_audit_adapter: Arc::new(UserMgmtAuditLogAdapter(Arc::new(AuditLog::new()))),
        }
    }

    pub fn in_memory() -> Self {
        Self::new(Arc::new(crate::storage::InMemoryStorageEngine::new()))
    }

    pub fn with_oidc(mut self, validator: OidcValidator) -> Self {
        self.oidc = Some(Arc::new(validator));
        self
    }

    pub fn with_cedar(mut self, authorizer: CedarAuthorizer) -> Self {
        self.cedar = Some(Arc::new(authorizer));
        self
    }

    pub fn with_search(mut self, engine: SearchEngine) -> Self {
        self.search = Some(Arc::new(tokio::sync::RwLock::new(engine)));
        self
    }

    pub fn with_ai_search(mut self, bridge: crate::ai_search::AiSearchBridge) -> Self {
        let bridge_arc = Arc::new(bridge);
        self.ai_search_bridge = Some(bridge_arc.clone());
        self.ai_search = Some(bridge_arc);
        self
    }

    pub fn with_wasm_runtime(mut self, runtime: WasmWorkerRuntime) -> Self {
        self.wasm_runtime = Some(Arc::new(runtime));
        self
    }

    pub fn with_workers_dir(mut self, dir: std::path::PathBuf) -> Self {
        self.workers_dir = Some(dir);
        self
    }

    pub fn with_metadata_store(
        mut self,
        store: Arc<dyn ferro_core::metadata::MetadataStore>,
    ) -> Self {
        self.metadata_store = Some(store);
        self
    }

    pub fn with_cas_store(mut self, store: Arc<dyn ferro_core::cas::CasStore>) -> Self {
        self.cas_store = Some(store);
        self
    }

    pub fn with_presigned_generator(
        mut self,
        generator: Arc<dyn ferro_core::presigned::PresignedUrlGenerator>,
    ) -> Self {
        self.presigned_generator = Some(generator);
        self
    }

    pub fn with_max_body_size(mut self, max_body_size: u64) -> Self {
        self.max_body_size = max_body_size;
        self
    }

    pub fn with_wopi_token_secret(mut self, secret: String) -> Self {
        self.wopi_token_secret = secret;
        self
    }

    pub fn with_external_url(mut self, external_url: String) -> Self {
        self.external_url = external_url;
        self
    }

    pub fn with_federation_secret(mut self, secret: String) -> Self {
        self.federation_secret = secret;
        self
    }

    pub fn with_wopi_office_url(mut self, url: String) -> Self {
        self.wopi_office_url = url;
        self
    }

    pub fn with_admin_user(mut self, user: Option<String>) -> Self {
        self.admin_user = user;
        self
    }

    pub fn with_admin_password(mut self, password: Option<String>) -> Self {
        self.admin_password = password;
        self
    }

    pub fn auth_enabled(&self) -> bool {
        self.oidc.is_some() || self.admin_user.is_some()
    }

    pub fn with_trash_dir(mut self, dir: String) -> Self {
        self.trash_dir = Some(dir.clone());
        self.trash_store = self.trash_store.clone().with_trash_dir(dir);
        self
    }

    pub fn with_audit_persistence(
        mut self,
        persistence: Arc<ferro_core::persistence::SqlitePersistence>,
    ) -> Self {
        self.audit_log = Arc::new(AuditLog::new().with_persistence(persistence));
        self
    }

    pub fn with_snapshot_persistence(
        mut self,
        persistence: Arc<ferro_core::persistence::SqlitePersistence>,
    ) -> Self {
        self.snapshot_store =
            Arc::new(SnapshotStore::new(self.max_snapshot_versions).with_persistence(persistence));
        self
    }

    pub fn with_lock_manager(mut self, lock_manager: Arc<dyn LockManagerTrait>) -> Self {
        self.lock_manager = lock_manager;
        self
    }

    pub fn with_share_store(mut self, share_store: Arc<dyn ShareStoreTrait>) -> Self {
        self.share_store = share_store;
        self
    }

    pub fn with_favorites(mut self, favorites: Arc<dyn FavoriteStore>) -> Self {
        self.favorites = favorites;
        self
    }

    pub fn with_preferences(mut self, preferences: Arc<dyn PreferenceStore>) -> Self {
        self.preferences = preferences;
        self
    }

    pub fn with_data_dir(mut self, dir: String) -> Self {
        self.data_dir = Some(dir);
        self
    }

    pub fn with_db(mut self, db: DbHandle) -> Self {
        self.db = Some(db.clone());
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());

        let user_store = crate::users::InMemoryUserStore::new().with_db(db.clone());
        if let Ok(users) = crate::users::InMemoryUserStore::load_all_from_db(&conn) {
            for user in users {
                user_store.load_from_db(user);
            }
        }
        self.user_store = Arc::new(user_store);

        let share_store = crate::shares::ShareStore::new().with_db(db.clone());
        if let Ok(loaded) = crate::shares::ShareStore::load_all_from_db(&conn) {
            share_store.load_links_blocking(loaded);
        }
        self.share_store = Arc::new(share_store);

        let fav_store = crate::favorites::InMemoryFavoriteStore::new().with_db(db.clone());
        if let Ok(paths) = crate::favorites::InMemoryFavoriteStore::load_all_from_db(&conn) {
            for path in paths {
                fav_store.favorites.insert(path);
            }
        }
        self.favorites = Arc::new(fav_store);

        let tags_store = ferro_server_collaboration::tags::TagStore::new().with_db(db.clone());
        if let Err(e) = tags_store.load_all_from_db(&conn) {
            tracing::warn!(error = %e, "failed to load tags from database");
        }
        self.tags = Arc::new(tags_store);

        let comments_store = crate::comments::CommentStore::new().with_db(db.clone());
        self.comments = Arc::new(comments_store);

        let sync_store = crate::sync::ops::SyncStore::new().with_db(db.clone());
        if let Err(e) = sync_store.load_all_from_db(&conn) {
            tracing::warn!(error = %e, "failed to load sync ops from database");
        }
        self.sync_store = Arc::new(sync_store);
        let activity_store =
            ferro_server_activitypub::store::ActivityStore::new().with_db(db.clone());
        if let Err(e) = activity_store.load_all_from_db(&conn) {
            tracing::warn!(error = %e, "failed to load activity store from database");
        }
        self.activity_store = Arc::new(activity_store);

        if let Ok(entries) = ferro_server_webdav_core::trash::load_trash_from_db(&conn) {
            for entry in entries {
                self.trash.insert(entry.original_path.clone(), entry);
            }
        }
        self.trash_store = self.trash_store.clone().with_db(db.clone());
        self.trash_store.load_from_db();
        let push_store =
            ferro_server_integrations::push_notifications::PushNotificationStore::new(db.clone());
        if let Err(e) = push_store.init_table() {
            tracing::warn!(error = %e, "failed to init push_notifications table");
        }
        self.push_notification_store = Some(Arc::new(tokio::sync::RwLock::new(push_store)));

        // Initialize device store
        let device_store = ferro_server_user_mgmt::account_api::DeviceStore::new(db.clone());
        if let Err(e) = device_store.init_table() {
            tracing::warn!(error = %e, "failed to init devices table");
        }

        let lock_mgr = ferro_server_webdav_core::lock::LockManager::new().with_db(db.clone());
        if let Err(e) = lock_mgr.load_all_from_db(&conn) {
            tracing::warn!(error = %e, "failed to load locks from database");
        }
        self.lock_manager = Arc::new(lock_mgr);
        let remote_mounts = ferro_server_integrations::remote_mount::RemoteMountStore::new()
            .with_db_handle(db.clone());
        if let Err(e) = remote_mounts.load_all_from_db(&conn) {
            tracing::warn!(error = %e, "failed to load remote mounts from database");
        }
        self.remote_mounts = Arc::new(remote_mounts);

        self.branding_store =
            ferro_server_admin_api::branding::BrandingStore::new().with_db(db.clone());
        self.task_store = ferro_server_productivity::tasks::TaskStore::new().with_db(db.clone());
        self.retention_store =
            ferro_server_compliance::retention::RetentionStore::new().with_db(db.clone());
        self.dlp_store = ferro_server_compliance::dlp_api::DlpStore::new().with_db(db.clone());
        self.watermark_db_store =
            ferro_server_content::watermark_api::WatermarkDbStore::new().with_db(db.clone());
        self.guest_store = ferro_server_user_mgmt::guests::GuestStore::new().with_db(db.clone());
        self.gdpr_store = ferro_server_admin_api::gdpr::GdprStore::new().with_db(db.clone());
        self.worm_store = ferro_server_compliance::worm::WormPolicyStore::new().with_db(db.clone());
        self.mail_store = ferro_server_integrations::mail_api::MailStore::new().with_db(db.clone());
        self.notification_prefs_store =
            crate::notification_prefs_api::NotificationPrefsStore::new().with_db(db.clone());
        // Rebuild admin-api adapters with DB-backed stores
        self.admin_audit_adapter = Arc::new(AdminAuditLogAdapter(self.audit_log.clone()));
        self.admin_share_store = Arc::new(AdminShareStoreAdapter(self.share_store.clone()));
        self.admin_favorites_store = Arc::new(AdminFavoriteStoreAdapter(self.favorites.clone()));
        self.admin_tags_store = Arc::new(AdminTagStoreAdapter(self.tags.clone()));
        self.collab_audit_adapter = Arc::new(CollaborationAuditLogAdapter(self.audit_log.clone()));
        self.user_mgmt_audit_adapter = Arc::new(UserMgmtAuditLogAdapter(self.audit_log.clone()));
        if let Err(e) = self.notification_prefs_store.init_table() {
            tracing::warn!(error = %e, "failed to init notification_prefs table");
        }
        self.webhook_delivery_store =
            ferro_server_api_core::webhooks::WebhookDeliveryStore::new().with_db(db.clone());

        let selective_sync_path = match &self.data_dir {
            Some(dir) => format!("{}/selective_sync.db", dir),
            None => ":memory:".to_string(),
        };
        let selective_sync_conn = match rusqlite::Connection::open(&selective_sync_path) {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(error = %e, "failed to open selective sync database");
                return self;
            }
        };
        match SelectiveSyncProfileStore::new(selective_sync_conn) {
            Ok(store) => {
                self.selective_sync_store = Some(Arc::new(store));
            }
            Err(e) => {
                tracing::warn!(error = %e, "failed to initialize selective sync store");
            }
        }

        self
    }

    pub fn with_user_store(mut self, store: Arc<dyn UserStoreTrait>) -> Self {
        self.user_store = store;
        self
    }

    pub fn with_max_file_versions(mut self, max: u64) -> Self {
        self.max_file_versions = max;
        self
    }

    pub fn with_streaming_upload_threshold(mut self, threshold: u64) -> Self {
        self.streaming_upload_threshold = threshold;
        self
    }

    pub fn with_tenant_rate_limiting(
        mut self,
        store: Arc<ferro_rate_limiter::tenant::TenantRateLimitStore>,
    ) -> Self {
        let limiter = Arc::new(ferro_rate_limiter::tenant::TenantAwareRateLimiter::new(
            store.clone(),
        ));
        self.tenant_rate_limit_store = Some(store);
        self.tenant_rate_limiter = Some(limiter);
        self
    }

    pub fn with_offline_queue(
        mut self,
        queue: Arc<ferro_offline::change_queue::SqliteChangeQueue>,
    ) -> Self {
        self.offline_queue = Some(queue);
        self
    }

    pub fn with_push_notifications(
        mut self,
        store: ferro_server_integrations::push_notifications::PushNotificationStore,
        config: ferro_server_integrations::push_notifications::PushNotificationConfig,
    ) -> Self {
        self.push_notification_store = Some(Arc::new(tokio::sync::RwLock::new(store)));
        self.push_notification_config = config;
        self
    }

    pub fn with_mount_backend(
        mut self,
        backend: Arc<dyn ferro_mount_nfs::traits::MountBackend>,
    ) -> Self {
        self.mount_backend = Some(backend);
        self
    }

    pub fn with_organization_store(
        mut self,
        store: Arc<dyn ferro_multi_tenant::organization::OrganizationStore>,
    ) -> Self {
        self.organization_store = store;
        self
    }

    pub fn with_tenant_store(
        mut self,
        store: Arc<dyn ferro_multi_tenant::tenant::TenantStore>,
    ) -> Self {
        self.tenant_store = store;
        self
    }

    pub fn with_metadata_cache(mut self, cache: ferro_cache::TimedCache<String, Vec<u8>>) -> Self {
        self.metadata_cache = Some(Arc::new(cache));
        self
    }

    pub fn with_offline_cache_size(mut self, max_size: u64) -> Self {
        self.offline_cache = Arc::new(tokio::sync::RwLock::new(
            ferro_offline::cache::ContentCache::new(max_size),
        ));
        self
    }

    pub fn with_selective_sync_store(mut self, store: Arc<SelectiveSyncProfileStore>) -> Self {
        self.selective_sync_store = Some(store);
        self
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
        self.sync_clock
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
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
        self.sync_clock
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
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

// ---------------------------------------------------------------------------
// GraphQL context builder
// ---------------------------------------------------------------------------

impl AppState {
    /// Build a [`ferro_graphql::GraphQLContext`] from this state.
    pub fn graphql_context(&self) -> ferro_graphql::GraphQLContext {
        let storage = self.storage.clone();
        let share_store = self.share_store.clone();
        let audit_log = self.audit_log.clone();
        let storage2 = storage.clone();
        let storage3 = storage.clone();
        let storage4 = storage.clone();
        ferro_graphql::GraphQLContext {
            list_files: Box::new(move |prefix: &str| {
                let storage = storage.clone();
                let prefix = prefix.to_string();
                Box::pin(async move { storage.list(&prefix).await.map_err(|e| e.to_string()) })
            }),
            head_file: Box::new(move |path: &str| {
                let storage = storage2.clone();
                let path = path.to_string();
                Box::pin(async move { storage.head(&path).await.map_err(|e| e.to_string()) })
            }),
            create_collection: Box::new(move |path: &str, owner: &str| {
                let storage = storage3.clone();
                let path = path.to_string();
                let owner = owner.to_string();
                Box::pin(async move {
                    storage
                        .create_collection(&path, &owner)
                        .await
                        .map_err(|e| e.to_string())
                })
            }),
            delete_file: Box::new(move |path: &str| {
                let storage = storage4.clone();
                let path = path.to_string();
                Box::pin(async move { storage.delete(&path).await.map_err(|e| e.to_string()) })
            }),
            list_shares: Box::new(move || {
                let share_store = share_store.clone();
                Box::pin(async move {
                    share_store
                        .list()
                        .await
                        .into_iter()
                        .map(|l| ferro_graphql::ShareEntry {
                            token: l.token,
                            path: l.path,
                            expires_at: l.expires_at.to_string(),
                            password_protected: l.password.is_some(),
                            max_downloads: l.max_downloads,
                            download_count: l.download_count,
                            created_by: l.created_by,
                        })
                        .collect()
                })
            }),
            recent_audit: Box::new(move |limit: usize, offset: usize| {
                let audit_log = audit_log.clone();
                Box::pin(async move {
                    audit_log
                        .recent_with_offset(limit, offset)
                        .await
                        .into_iter()
                        .map(|e| ferro_graphql::AuditEntry {
                            method: e.method,
                            path: e.path,
                            user: e.user,
                            status: e.status,
                            timestamp: e.timestamp,
                        })
                        .collect()
                })
            }),
            current_user: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Adapter structs
// ---------------------------------------------------------------------------

/// Adapter to bridge the server's AuditLog to the collaboration crate's AuditLogTrait.
struct CollaborationAuditLogAdapter(Arc<AuditLog>);

#[async_trait::async_trait]
impl ferro_server_collaboration::AuditLogTrait for CollaborationAuditLogAdapter {
    async fn log(&self, entry: ferro_server_collaboration::AuditEntry) {
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
}

/// Adapter to bridge the server's AuditLog to the user-mgmt crate's AuditLog trait.
struct UserMgmtAuditLogAdapter(Arc<AuditLog>);

impl ferro_server_user_mgmt::AuditLog for UserMgmtAuditLogAdapter {
    fn log(
        &self,
        entry: ferro_server_user_mgmt::AuditEntry,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + '_>> {
        let audit = self.0.clone();
        Box::pin(async move {
            audit
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
        })
    }
}

/// Adapter to bridge the server's AuditLog to the compliance crate's AuditLogTrait.
struct AuditLogAdapter(Arc<AuditLog>);

#[async_trait::async_trait]
impl ferro_server_compliance::AuditLogTrait for AuditLogAdapter {
    async fn log(&self, entry: ferro_server_compliance::AuditEntry) {
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
}

/// Adapter to bridge the server's AuditLog to the admin crate's AuditLogTrait.
struct AdminAuditLogAdapter(Arc<AuditLog>);

#[async_trait::async_trait]
impl ferro_server_admin_api::AuditLogTrait for AdminAuditLogAdapter {
    async fn log(&self, entry: ferro_server_admin_api::AuditEntry) {
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

    async fn entries(&self) -> Vec<ferro_server_admin_api::AuditEntry> {
        self.0
            .recent_with_offset(10000, 0)
            .await
            .into_iter()
            .map(|e| ferro_server_admin_api::AuditEntry {
                timestamp: e.timestamp,
                method: e.method,
                path: e.path,
                user: e.user,
                status: e.status,
                client_ip: e.client_ip,
                user_agent: e.user_agent,
                content_length: e.content_length,
            })
            .collect()
    }

    async fn verify_chain(&self) -> Option<serde_json::Value> {
        None
    }
}

/// Adapter to bridge the server's ShareStoreTrait to the admin crate's AdminShareStoreTrait.
struct AdminShareStoreAdapter(Arc<dyn crate::shares::ShareStoreTrait>);

#[async_trait::async_trait]
impl ferro_server_admin_api::AdminShareStoreTrait for AdminShareStoreAdapter {
    async fn list(&self) -> Vec<ferro_server_admin_api::AdminShareLink> {
        self.0
            .list()
            .await
            .into_iter()
            .map(|s| ferro_server_admin_api::AdminShareLink {
                token: s.token,
                path: s.path,
                expires_at: s.expires_at.to_rfc3339(),
                max_downloads: s.max_downloads,
                download_count: s.download_count,
                created_by: s.created_by,
                allow_download: s.allow_download,
                allow_upload: s.allow_upload,
            })
            .collect()
    }

    async fn delete(&self, token: &str) -> bool {
        self.0.delete(token).await
    }
}

/// Adapter to bridge the server's FavoriteStore to the admin crate's AdminFavoriteStoreTrait.
struct AdminFavoriteStoreAdapter(Arc<dyn crate::favorites::FavoriteStore>);

#[async_trait::async_trait]
impl ferro_server_admin_api::AdminFavoriteStoreTrait for AdminFavoriteStoreAdapter {
    async fn list(&self) -> Vec<String> {
        self.0.list().await
    }

    async fn remove(&self, path: &str) {
        self.0.remove(path).await
    }
}

/// Adapter to bridge the server's TagStore to the admin crate's AdminTagStoreTrait.
struct AdminTagStoreAdapter(Arc<ferro_server_collaboration::tags::TagStore>);

impl ferro_server_admin_api::AdminTagStoreTrait for AdminTagStoreAdapter {
    fn all_tags(&self) -> Vec<(String, Vec<String>)> {
        self.0
            .entries
            .iter()
            .map(|entry| {
                let (path, tags) = entry.pair();
                (path.clone(), tags.iter().cloned().collect())
            })
            .collect()
    }

    fn all_tag_pairs(&self) -> Vec<(String, String)> {
        self.0
            .entries
            .iter()
            .flat_map(|entry| {
                let (path, tags) = entry.pair();
                tags.iter()
                    .map(|tag| (path.clone(), tag.clone()))
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    fn remove_tag(&self, path: &str, tag: &str) -> bool {
        self.0.remove_tag(path, tag)
    }
}

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
    ) -> &Option<
        Arc<
            tokio::sync::RwLock<
                ferro_server_integrations::push_notifications::PushNotificationStore,
            >,
        >,
    > {
        &self.push_notification_store
    }

    fn push_notification_config(
        &self,
    ) -> &ferro_server_integrations::push_notifications::PushNotificationConfig {
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
    fn list_favorites(
        &self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Vec<String>> + Send + '_>> {
        Box::pin(async move { self.favorites.list().await })
    }
    fn add_favorite(
        &self,
        path: String,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + '_>> {
        Box::pin(async move { self.favorites.add(path).await })
    }
    fn remove_favorite(
        &self,
        path: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + '_>> {
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
            ferro_server_webdav_core::WebdavEventType::FileUploaded => {
                crate::event_triggers::EventType::FileUploaded
            }
            ferro_server_webdav_core::WebdavEventType::FileModified => {
                crate::event_triggers::EventType::FileModified
            }
            ferro_server_webdav_core::WebdavEventType::FileDeleted => {
                crate::event_triggers::EventType::FileDeleted
            }
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
    fn plugin_registry(
        &self,
    ) -> &Arc<DashMap<String, ferro_server_plugins::plugin_permissions::PluginManifest>> {
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
    ) -> &Option<
        Arc<
            tokio::sync::RwLock<
                ferro_server_integrations::push_notifications::PushNotificationStore,
            >,
        >,
    > {
        &self.push_notification_store
    }
    fn push_notification_config(
        &self,
    ) -> &ferro_server_integrations::push_notifications::PushNotificationConfig {
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
    fn webhooks(
        &self,
    ) -> &Arc<tokio::sync::RwLock<Vec<ferro_server_api_core::webhooks::WebhookConfig>>> {
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
    ) -> &Option<
        Arc<
            tokio::sync::RwLock<
                ferro_server_integrations::push_notifications::PushNotificationStore,
            >,
        >,
    > {
        &self.push_notification_store
    }
    fn push_notification_config(
        &self,
    ) -> &ferro_server_integrations::push_notifications::PushNotificationConfig {
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
    fn search_ranking_config(
        &self,
    ) -> &Arc<tokio::sync::RwLock<ferro_core::search::SearchRankingConfig>> {
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

impl ferro_server_storage_utils::StorageUtilsState for AppState {
    fn storage(&self) -> &Arc<dyn common::storage::StorageEngine> {
        &self.storage
    }
    fn data_dir(&self) -> Option<&str> {
        self.data_dir.as_deref()
    }
    fn thumbnail_cache(&self) -> &Arc<dyn ferro_server_storage_utils::ThumbnailCacheTrait> {
        &self.thumbnail_cache
    }
    fn thumbnail_size(&self) -> u32 {
        self.thumbnail_size
    }
    fn snapshot_store(&self) -> &Arc<ferro_server_storage_utils::snapshots::SnapshotStore> {
        &self.snapshot_store
    }
    fn storage_health(
        &self,
    ) -> &Arc<ferro_server_storage_utils::storage_health::StorageHealthMonitor> {
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
