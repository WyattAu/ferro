use crate::db::DbHandle;
use crate::favorites::FavoriteStore;
use crate::search::PreferenceStore;
use crate::shares::ShareStoreTrait;
use crate::state::AppState;
use crate::users::UserStoreTrait;
use common::storage::LockManagerTrait;
use ferro_auth::cedar::CedarAuthorizer;
use ferro_auth::oidc::OidcValidator;
use ferro_core::search::SearchEngine;
use ferro_core::wasm::WasmWorkerRuntime;
use std::sync::Arc;

impl AppState {
    pub fn with_oidc(self, validator: OidcValidator) -> Self {
        let mut s = self;
        s.oidc = Some(Arc::new(validator));
        s
    }

    pub fn with_cedar(self, authorizer: CedarAuthorizer) -> Self {
        let mut s = self;
        s.cedar = Some(Arc::new(authorizer));
        s
    }

    pub fn with_search(self, engine: SearchEngine) -> Self {
        let mut s = self;
        s.search = Some(Arc::new(tokio::sync::RwLock::new(engine)));
        s
    }

    pub fn with_ai_search(self, bridge: crate::ai_search::AiSearchBridge) -> Self {
        let mut s = self;
        let bridge_arc = Arc::new(bridge);
        s.ai_search_bridge = Some(bridge_arc.clone());
        s.ai_search = Some(bridge_arc);
        s
    }

    pub fn with_wasm_runtime(self, runtime: WasmWorkerRuntime) -> Self {
        let mut s = self;
        s.wasm_runtime = Some(Arc::new(runtime));
        s
    }

    pub fn with_workers_dir(self, dir: std::path::PathBuf) -> Self {
        let mut s = self;
        s.workers_dir = Some(dir);
        s
    }

    pub fn with_metadata_store(self, store: Arc<dyn ferro_core::metadata::MetadataStore>) -> Self {
        let mut s = self;
        s.metadata_store = Some(store);
        s
    }

    pub fn with_cas_store(self, store: Arc<dyn ferro_core::cas::CasStore>) -> Self {
        let mut s = self;
        s.cas_store = Some(store);
        s
    }

    pub fn with_presigned_generator(self, generator: Arc<dyn ferro_core::presigned::PresignedUrlGenerator>) -> Self {
        let mut s = self;
        s.presigned_generator = Some(generator);
        s
    }

    pub fn with_max_body_size(self, max_body_size: u64) -> Self {
        let mut s = self;
        s.max_body_size = max_body_size;
        s
    }

    pub fn with_wopi_token_secret(self, secret: String) -> Self {
        let mut s = self;
        s.wopi_token_secret = secret;
        s
    }

    pub fn with_external_url(self, external_url: String) -> Self {
        let mut s = self;
        s.external_url = external_url;
        s
    }

    pub fn with_federation_secret(self, secret: String) -> Self {
        let mut s = self;
        s.federation_secret = secret;
        s
    }

    pub fn with_wopi_office_url(self, url: String) -> Self {
        let mut s = self;
        s.wopi_office_url = url;
        s
    }

    pub fn with_admin_user(self, user: Option<String>) -> Self {
        let mut s = self;
        s.admin_user = user;
        s
    }

    pub fn with_admin_password(self, password: Option<String>) -> Self {
        let mut s = self;
        s.admin_password = password;
        s
    }

    pub fn auth_enabled(&self) -> bool {
        self.oidc.is_some() || self.admin_user.is_some()
    }

    pub fn with_trash_dir(self, dir: String) -> Self {
        let mut s = self;
        s.trash_dir = Some(dir.clone());
        s.trash_store = s.trash_store.clone().with_trash_dir(dir);
        s
    }

    pub fn with_audit_persistence(self, persistence: Arc<ferro_core::persistence::SqlitePersistence>) -> Self {
        let mut s = self;
        s.audit_log = Arc::new(crate::audit::AuditLog::new().with_persistence(persistence));
        s
    }

    pub fn with_snapshot_persistence(self, persistence: Arc<ferro_core::persistence::SqlitePersistence>) -> Self {
        let mut s = self;
        s.snapshot_store = Arc::new(
            ferro_server_storage_ops::snapshots::SnapshotStore::new(s.max_snapshot_versions)
                .with_persistence(persistence),
        );
        s
    }

    pub fn with_lock_manager(self, lock_manager: Arc<dyn LockManagerTrait>) -> Self {
        let mut s = self;
        s.lock_manager = lock_manager;
        s
    }

    pub fn with_share_store(self, share_store: Arc<dyn ShareStoreTrait>) -> Self {
        let mut s = self;
        s.share_store = share_store;
        s
    }

    pub fn with_favorites(self, favorites: Arc<dyn FavoriteStore>) -> Self {
        let mut s = self;
        s.favorites = favorites;
        s
    }

    pub fn with_preferences(self, preferences: Arc<dyn PreferenceStore>) -> Self {
        let mut s = self;
        s.preferences = preferences;
        s
    }

    pub fn with_data_dir(self, dir: String) -> Self {
        let mut s = self;
        s.data_dir = Some(dir);
        s
    }

    pub fn with_db(self, db: DbHandle) -> Self {
        crate::state::db_init::with_db(self, db)
    }

    pub fn with_user_store(self, store: Arc<dyn UserStoreTrait>) -> Self {
        let mut s = self;
        s.user_store = store;
        s
    }

    pub fn with_max_file_versions(self, max: u64) -> Self {
        let mut s = self;
        s.max_file_versions = max;
        s
    }

    pub fn with_streaming_upload_threshold(self, threshold: u64) -> Self {
        let mut s = self;
        s.streaming_upload_threshold = threshold;
        s
    }

    pub fn with_tenant_rate_limiting(self, store: Arc<ferro_rate_limiter::tenant::TenantRateLimitStore>) -> Self {
        let mut s = self;
        let limiter = Arc::new(ferro_rate_limiter::tenant::TenantAwareRateLimiter::new(store.clone()));
        s.tenant_rate_limit_store = Some(store);
        s.tenant_rate_limiter = Some(limiter);
        s
    }

    pub fn with_offline_queue(self, queue: Arc<ferro_offline::change_queue::SqliteChangeQueue>) -> Self {
        let mut s = self;
        s.offline_queue = Some(queue);
        s
    }

    pub fn with_push_notifications(
        self,
        store: ferro_server_integrations::push_notifications::PushNotificationStore,
        config: ferro_server_integrations::push_notifications::PushNotificationConfig,
    ) -> Self {
        let mut s = self;
        s.push_notification_store = Some(Arc::new(tokio::sync::RwLock::new(store)));
        s.push_notification_config = config;
        s
    }

    pub fn with_mount_backend(self, backend: Arc<dyn ferro_mount_nfs::traits::MountBackend>) -> Self {
        let mut s = self;
        s.mount_backend = Some(backend);
        s
    }

    pub fn with_organization_store(self, store: Arc<dyn ferro_multi_tenant::organization::OrganizationStore>) -> Self {
        let mut s = self;
        s.organization_store = store;
        s
    }

    pub fn with_tenant_store(self, store: Arc<dyn ferro_multi_tenant::tenant::TenantStore>) -> Self {
        let mut s = self;
        s.tenant_store = store;
        s
    }

    pub fn with_metadata_cache(self, cache: ferro_cache::TimedCache<String, Vec<u8>>) -> Self {
        let mut s = self;
        s.metadata_cache = Some(Arc::new(cache));
        s
    }

    pub fn with_offline_cache_size(self, max_size: u64) -> Self {
        let mut s = self;
        s.offline_cache = Arc::new(tokio::sync::RwLock::new(ferro_offline::cache::ContentCache::new(
            max_size,
        )));
        s
    }

    pub fn with_selective_sync_store(self, store: Arc<ferro_selective_sync::persistence::ProfileStore>) -> Self {
        let mut s = self;
        s.selective_sync_store = Some(store);
        s
    }
}
