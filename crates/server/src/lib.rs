pub mod activity;
pub mod admin_api;
pub mod api;
pub mod api_error;
pub mod audit;
pub mod auth;
pub mod backup;
pub mod batch;
pub mod branding;
pub mod bulk;
#[cfg(unix)]
pub mod clamav;
pub mod comments;
pub mod config;
pub mod conflict;
pub mod dav;
pub mod db;
pub mod dedup;
pub mod e2ee;
pub mod email;
pub mod encryption;
pub mod error;
pub mod events;
pub mod favorites;
pub mod fs_util;
pub mod gdpr;
pub mod guests;
pub mod federation {
    pub use ferro_server_activitypub::FederationState;
    pub use ferro_server_activitypub::store::ActivityStore;
    pub use ferro_server_activitypub::*;

    use axum::extract::State;
    use axum::response::Response;

    fn fed_state(s: &crate::AppState) -> FederationState {
        FederationState {
            activity_store: s.activity_store.clone(),
            external_url: s.external_url.clone(),
            federation_secret: s.federation_secret.clone(),
        }
    }

    pub async fn get_actor(
        State(s): State<crate::AppState>,
        path: axum::extract::Path<String>,
    ) -> Response {
        ferro_server_activitypub::get_actor(State(fed_state(&s)), path).await
    }

    pub async fn nodeinfo(State(s): State<crate::AppState>) -> Response {
        ferro_server_activitypub::nodeinfo(State(fed_state(&s))).await
    }

    pub async fn inbox(
        State(s): State<crate::AppState>,
        req: axum::http::Request<axum::body::Body>,
    ) -> Response {
        ferro_server_activitypub::inbox(State(fed_state(&s)), req).await
    }

    pub async fn list_inbox(
        State(s): State<crate::AppState>,
        q: axum::extract::Query<std::collections::HashMap<String, String>>,
    ) -> Response {
        ferro_server_activitypub::list_inbox(State(fed_state(&s)), q).await
    }

    pub async fn list_outbox(
        State(s): State<crate::AppState>,
        q: axum::extract::Query<std::collections::HashMap<String, String>>,
    ) -> Response {
        ferro_server_activitypub::list_outbox(State(fed_state(&s)), q).await
    }

    pub async fn list_followers(
        State(s): State<crate::AppState>,
        path: axum::extract::Path<String>,
    ) -> Response {
        ferro_server_activitypub::list_followers(State(fed_state(&s)), path).await
    }

    pub async fn list_following(
        State(s): State<crate::AppState>,
        path: axum::extract::Path<String>,
    ) -> Response {
        ferro_server_activitypub::list_following(State(fed_state(&s)), path).await
    }

    pub async fn webfinger(
        State(s): State<crate::AppState>,
        q: axum::extract::Query<ferro_server_activitypub::webfinger::WebFingerQuery>,
    ) -> Response {
        ferro_server_activitypub::webfinger::webfinger(State(fed_state(&s)), q).await
    }

    pub async fn federated_share(
        State(s): State<crate::AppState>,
        body: axum::Json<ferro_server_activitypub::ShareRequest>,
    ) -> Response {
        ferro_server_activitypub::federated_share(State(fed_state(&s)), body).await
    }
}
pub mod event_triggers;
pub mod idempotency;
pub mod indexer;
pub mod integration;
pub mod json_logging;
#[cfg(feature = "ldap")]
pub mod ldap_auth;
pub mod lock;
pub mod metrics;
pub mod move_copy;
pub mod object_store_backend;
pub mod ocr;
pub mod openapi;
#[cfg(feature = "pg")]
pub mod pg_state;
pub mod plugin_permissions;
pub mod policies;
pub mod preferences;
pub mod presigned;
pub mod prometheus_metrics;
pub mod quota;
pub mod range_get;
pub mod ransomware;
pub mod rate_limit;
pub mod read_cache;
#[cfg(feature = "redis")]
pub mod redis_lock;
#[cfg(feature = "redis")]
pub mod redis_rate_limiter;
pub mod remote_mount;
pub mod request_id;
pub mod request_logging;
pub mod retention;
pub mod search;
pub mod security;
pub mod security_headers;
pub mod shares;
pub mod shares_ext;
pub mod simple_auth;
pub mod snapshots;
pub mod storage;
pub mod storage_health;
pub mod streaming_upload;
pub mod sync;
pub mod tags;
pub mod thumbnail_cache;
pub mod thumbnails;
pub mod totp_api;
pub mod trash;
pub mod triggers;
pub mod upload;
pub mod user_api;
pub mod user_paths;
pub mod users;
pub mod wasm_upload;
#[cfg(feature = "webauthn")]
pub mod webauthn_api;
pub mod webdav;
pub mod webhooks;
pub mod worker_runner;
pub mod workers;
pub mod worm;
pub mod ws;
pub mod xml;

use axum::Router;
use axum::body::Body;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::http::{Request, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::any;
use common::storage::LockManagerTrait;
use common::storage::StorageEngine;
use dashmap::{DashMap, DashSet};
use lock::LockManager;
use std::sync::Arc;
use tower::limit::ConcurrencyLimitLayer;
use tower_http::compression::CompressionLayer;
use tower_http::services::{ServeDir, ServeFile};

use auth::cedar::CedarAuthorizer;
use auth::oidc::OidcValidator;
use ferro_core::search::SearchEngine;
use ferro_core::wasm::WasmWorkerRuntime;

use audit::AuditLog;
use snapshots::SnapshotStore;
use trash::TrashedEntry;
use users::{InMemoryUserStore, UserStoreTrait};

use db::DbHandle;
use favorites::FavoriteStore;
use search::PreferenceStore;
use shares::ShareStoreTrait;
use sync::ops::SyncStore;

#[derive(Clone)]
pub struct AppState {
    pub storage: Arc<dyn StorageEngine>,
    pub lock_manager: Arc<dyn LockManagerTrait>,
    pub oidc: Option<Arc<OidcValidator>>,
    pub cedar: Option<Arc<CedarAuthorizer>>,
    pub search: Option<Arc<tokio::sync::RwLock<SearchEngine>>>,
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
    pub trash: Arc<DashMap<String, TrashedEntry>>,
    pub trash_dir: Option<String>,
    pub quota_bytes: Option<u64>,
    pub used_bytes: Arc<std::sync::atomic::AtomicU64>,
    pub file_count: Arc<std::sync::atomic::AtomicU64>,
    pub preferences: Arc<dyn PreferenceStore>,
    pub read_cache: Arc<read_cache::ReadCache>,
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
    pub webhooks: Arc<tokio::sync::RwLock<Vec<webhooks::WebhookConfig>>>,
    pub thumbnail_size: u32,
    pub thumbnail_cache: Arc<thumbnail_cache::ThumbnailCache>,
    pub data_dir: Option<String>,
    pub user_store: Arc<dyn UserStoreTrait>,
    pub max_file_versions: u64,
    pub calendar_store: Arc<dyn ferro_dav::store::CalendarStore>,
    pub address_book_store: Arc<dyn ferro_dav::store::AddressBookStore>,
    /// NOTE: The following fields are in-memory only (`DashMap`, `AtomicU64`, etc.).
    /// Data stored in these fields is lost on restart. Use `--data-dir` to enable
    /// SQLite-backed persistence so that state survives restarts.
    pub webrtc_offers: Arc<ferro_server_webrtc::offers::OfferStore>,
    pub activity_store: Arc<federation::store::ActivityStore>,
    pub federation_secret: String,
    pub sync_store: Arc<SyncStore>,
    pub tags: Arc<tags::TagStore>,
    pub comments: Arc<comments::CommentStore>,
    pub idempotency_store: Arc<idempotency::IdempotencyStore>,
    pub storage_health: Arc<storage_health::StorageHealthMonitor>,
    pub ws_manager: Arc<ws::WsManager>,
    pub db: Option<DbHandle>,
    pub upload_store: upload::UploadStore,
    pub auth_attempt_tracker: Arc<security::AuthAttemptTracker>,
    pub login_rate_limiter: Arc<security::LoginRateLimiter>,
    /// WASM worker dispatch counter (total executions).
    pub wasm_dispatch_count: Arc<std::sync::atomic::AtomicU64>,
    /// WASM worker error counter (failed executions).
    pub wasm_error_count: Arc<std::sync::atomic::AtomicU64>,
    /// WASM worker total fuel consumed across all executions.
    pub wasm_fuel_total: Arc<std::sync::atomic::AtomicU64>,
    /// Registry of loaded WASM plugins with capability declarations.
    pub plugin_registry: Arc<DashMap<String, plugin_permissions::PluginManifest>>,
    /// Whether the server has completed startup (CAS verification, DB init, etc.).
    /// Set to true after all startup checks pass in main.rs.
    pub startup_complete: Arc<std::sync::atomic::AtomicBool>,
    pub streaming_upload_threshold: u64,
    pub dedup_enabled: bool,
    pub email_config: email::EmailConfig,
    pub remote_mounts: Arc<remote_mount::RemoteMountStore>,
    pub ransomware_detector: Arc<ransomware::RansomwareDetector>,
    #[cfg(feature = "webauthn")]
    pub webauthn_store: Arc<tokio::sync::RwLock<crate::auth::webauthn::WebAuthnStore>>,
}

impl AppState {
    pub fn new(storage: Arc<dyn StorageEngine>) -> Self {
        Self {
            storage,
            lock_manager: Arc::new(LockManager::new()),
            oidc: None,
            cedar: None,
            search: None,
            wasm_runtime: None,
            workers_dir: None,
            metadata_store: None,
            cas_store: None,
            presigned_generator: None,
            share_store: Arc::new(shares::ShareStore::new()),
            audit_log: Arc::new(AuditLog::new()),
            snapshot_store: Arc::new(SnapshotStore::new(50)),
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
            favorites: Arc::new(favorites::InMemoryFavoriteStore::new()),
            trash: Arc::new(DashMap::new()),
            trash_dir: None,
            quota_bytes: None,
            used_bytes: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            file_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            preferences: Arc::new(search::InMemoryPreferenceStore::new()),
            read_cache: Arc::new(read_cache::ReadCache::default()),
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
            thumbnail_cache: Arc::new(thumbnail_cache::ThumbnailCache::noop()),
            user_store: Arc::new(InMemoryUserStore::new()),
            max_file_versions: 10,
            calendar_store: Arc::new(ferro_dav::store::InMemoryCalendarStore::new()),
            address_book_store: Arc::new(ferro_dav::store::InMemoryAddressBookStore::new()),
            webrtc_offers: Arc::new(ferro_server_webrtc::offers::OfferStore::new()),
            activity_store: Arc::new(federation::store::ActivityStore::new()),
            federation_secret: String::new(),
            sync_store: Arc::new(SyncStore::new()),
            tags: Arc::new(tags::TagStore::new()),
            comments: Arc::new(comments::CommentStore::new()),
            idempotency_store: Arc::new(idempotency::IdempotencyStore::new()),
            storage_health: Arc::new(storage_health::StorageHealthMonitor::new()),
            ws_manager: Arc::new(ws::WsManager::new()),
            db: None,
            upload_store: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            auth_attempt_tracker: Arc::new(security::AuthAttemptTracker::default()),
            login_rate_limiter: Arc::new(security::LoginRateLimiter::default()),
            wasm_dispatch_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            wasm_error_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            wasm_fuel_total: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            plugin_registry: Arc::new(DashMap::new()),
            startup_complete: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            streaming_upload_threshold: streaming_upload::DEFAULT_STREAMING_THRESHOLD,
            dedup_enabled: false,
            email_config: email::EmailConfig::default(),
            remote_mounts: Arc::new(remote_mount::RemoteMountStore::new()),
            ransomware_detector: Arc::new(ransomware::RansomwareDetector::new(
                ransomware::RansomwareConfig::default(),
            )),
            #[cfg(feature = "webauthn")]
            webauthn_store: Arc::new(tokio::sync::RwLock::new(
                crate::auth::webauthn::WebAuthnStore::new(),
            )),
        }
    }

    pub fn in_memory() -> Self {
        Self::new(Arc::new(storage::InMemoryStorageEngine::new()))
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
        self.trash_dir = Some(dir);
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
        self.snapshot_store = Arc::new(SnapshotStore::new(50).with_persistence(persistence));
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

        let user_store = users::InMemoryUserStore::new().with_db(db.clone());
        if let Ok(users) = users::InMemoryUserStore::load_all_from_db(&conn) {
            for user in users {
                user_store.load_from_db(user);
            }
        }
        self.user_store = Arc::new(user_store);

        let share_store = shares::ShareStore::new().with_db(db.clone());
        if let Ok(loaded) = shares::ShareStore::load_all_from_db(&conn) {
            share_store.load_links_blocking(loaded);
        }
        self.share_store = Arc::new(share_store);

        let fav_store = favorites::InMemoryFavoriteStore::new().with_db(db.clone());
        if let Ok(paths) = favorites::InMemoryFavoriteStore::load_all_from_db(&conn) {
            for path in paths {
                fav_store.favorites.insert(path);
            }
        }
        self.favorites = Arc::new(fav_store);

        let tags_store = tags::TagStore::new().with_db(db.clone());
        if let Err(e) = tags_store.load_all_from_db(&conn) {
            tracing::warn!(error = %e, "failed to load tags from database");
        }
        self.tags = Arc::new(tags_store);

        let comments_store = comments::CommentStore::new().with_db(db.clone());
        self.comments = Arc::new(comments_store);

        let sync_store = sync::ops::SyncStore::new().with_db(db.clone());
        if let Err(e) = sync_store.load_all_from_db(&conn) {
            tracing::warn!(error = %e, "failed to load sync ops from database");
        }
        self.sync_store = Arc::new(sync_store);
        let activity_store = federation::store::ActivityStore::new().with_db(db.clone());
        if let Err(e) = activity_store.load_all_from_db(&conn) {
            tracing::warn!(error = %e, "failed to load activity store from database");
        }
        self.activity_store = Arc::new(activity_store);

        if let Ok(entries) = trash::load_trash_from_db(&conn) {
            for entry in entries {
                self.trash.insert(entry.original_path.clone(), entry);
            }
        }
        let lock_mgr = lock::LockManager::new().with_db(db.clone());
        if let Err(e) = lock_mgr.load_all_from_db(&conn) {
            tracing::warn!(error = %e, "failed to load locks from database");
        }
        self.lock_manager = Arc::new(lock_mgr);
        let remote_mounts = remote_mount::RemoteMountStore::new().with_db_handle(db.clone());
        if let Err(e) = remote_mounts.load_all_from_db(&conn) {
            tracing::warn!(error = %e, "failed to load remote mounts from database");
        }
        self.remote_mounts = Arc::new(remote_mounts);

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

    pub fn record_sync_op(
        &self,
        op_type: sync::ops::OpType,
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
        self.sync_store.record_op(sync::ops::SyncOp {
            id: op_id,
            site_id: "local".to_string(),
            clock: sync::clock::VectorClock::new("local").with_counter(clock),
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

    pub fn user_info(&self, username: &str) -> Option<users::UserInfo> {
        match self.user_store.get_user_by_username_blocking(username) {
            Ok(u) if u.is_active() => Some(users::UserInfo::from(&u)),
            _ => {
                if self.admin_user.as_deref() == Some(username) {
                    Some(users::UserInfo {
                        user_id: "admin".to_string(),
                        username: username.to_string(),
                        role: users::UserRole::Admin,
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

pub fn make_app() -> Router {
    let state = AppState::in_memory()
        .with_wopi_token_secret("test-wopi-secret-for-integration".to_string());
    build_router(state)
}

pub fn build_router(state: AppState) -> Router {
    build_router_with_static(state, None, "*", "v1")
}

fn api_routes(
    state: &AppState,
    webrtc_offers: Arc<ferro_server_webrtc::offers::OfferStore>,
) -> Router<AppState> {
    Router::new()
        .route("/auth/info", axum::routing::get(api::auth_info))
        .route("/auth/login", axum::routing::get(api::auth_login))
        .route("/auth/callback", axum::routing::get(api::auth_callback))
        .route(
            "/auth/refresh",
            axum::routing::post(api::auth_refresh_token),
        )
        .route(
            "/auth/change-password",
            axum::routing::post(api::auth_change_password),
        )
        // TOTP two-factor authentication
        .route(
            "/auth/totp/setup",
            axum::routing::post(totp_api::totp_setup),
        )
        .route(
            "/auth/totp/enable",
            axum::routing::post(totp_api::totp_enable),
        )
        .route(
            "/auth/totp/disable",
            axum::routing::post(totp_api::totp_disable),
        )
        .route(
            "/auth/totp/status",
            axum::routing::get(totp_api::totp_status),
        )
        // WebAuthn/FIDO2 authentication (G-04)
        .merge({
            #[cfg(feature = "webauthn")]
            {
                axum::Router::new()
                    .route(
                        "/auth/webauthn/register/begin",
                        axum::routing::post(webauthn_api::webauthn_register_begin),
                    )
                    .route(
                        "/auth/webauthn/register/finish",
                        axum::routing::post(webauthn_api::webauthn_register_finish),
                    )
                    .route(
                        "/auth/webauthn/login/begin",
                        axum::routing::post(webauthn_api::webauthn_login_begin),
                    )
                    .route(
                        "/auth/webauthn/login/finish",
                        axum::routing::post(webauthn_api::webauthn_login_finish),
                    )
            }
            #[cfg(not(feature = "webauthn"))]
            {
                axum::Router::new()
            }
        })
        .route("/search", axum::routing::get(search::handle_search))
        .route(
            "/workers",
            axum::routing::get(workers::list_workers).post(workers::register_worker),
        )
        .route(
            "/workers/upload",
            axum::routing::post(wasm_upload::upload_wasm_module),
        )
        .route(
            "/workers/modules/{filename}",
            axum::routing::delete(wasm_upload::delete_wasm_module),
        )
        .route(
            "/workers/modules",
            axum::routing::get(wasm_upload::list_wasm_modules),
        )
        .route(
            "/plugins",
            axum::routing::get(plugin_permissions::list_plugins),
        )
        .route(
            "/policies",
            axum::routing::get(policies::list_policies)
                .post(policies::add_policy)
                .delete(policies::delete_policy),
        )
        .route("/config", axum::routing::get(config::get_server_config))
        .route(
            "/branding",
            axum::routing::get(branding::get_public_branding),
        )
        .route("/files", axum::routing::get(api::list_files))
        .route("/files/mkdir", axum::routing::post(api::mkdir))
        .route("/files/move", axum::routing::post(move_copy::move_file))
        .route("/files/copy", axum::routing::post(move_copy::copy_file))
        .route("/upload-url", axum::routing::get(presigned::get_upload_url))
        .route(
            "/download-url",
            axum::routing::get(presigned::get_download_url),
        )
        .route(
            "/shares",
            axum::routing::get(shares::list_shares).post(shares::create_share),
        )
        .route(
            "/shares/:token",
            axum::routing::delete(shares::delete_share),
        )
        .route("/audit", axum::routing::get(audit_handler))
        .route("/storage/stats", axum::routing::get(storage_stats))
        .route(
            "/snapshots",
            axum::routing::get(snapshots::list_snapshots).post(snapshots::create_snapshot),
        )
        .route(
            "/snapshots/:id",
            axum::routing::delete(snapshots::delete_snapshot_by_id),
        )
        .route(
            "/snapshots/:id/restore",
            axum::routing::post(snapshots::restore_snapshot),
        )
        .route(
            "/favorites",
            axum::routing::get(favorites::list_favorites)
                .put(favorites::add_favorite)
                .delete(favorites::remove_favorite),
        )
        .route("/recent", axum::routing::get(favorites::list_recent))
        .route("/trash", axum::routing::get(trash::list_trash))
        .route("/trash/{path}", axum::routing::delete(trash::move_to_trash))
        .route("/trash/restore", axum::routing::post(trash::restore_trash))
        .route("/trash/purge", axum::routing::delete(trash::purge_trash))
        .route("/trash/empty", axum::routing::delete(trash::empty_trash))
        .route("/bulk/delete", axum::routing::post(bulk::bulk_delete))
        .route("/batch/copy", axum::routing::post(batch::batch_copy))
        .route("/batch/move", axum::routing::post(batch::batch_move))
        .route(
            "/fed/share",
            axum::routing::post(federation::federated_share),
        )
        .route(
            "/files/encrypt",
            axum::routing::post(encryption::encrypt_file),
        )
        .route(
            "/files/decrypt",
            axum::routing::post(encryption::decrypt_file),
        )
        .route("/e2ee/encrypt", axum::routing::post(e2ee::e2ee_encrypt))
        .route(
            "/e2ee/key/generate",
            axum::routing::post(e2ee::e2ee_key_generate),
        )
        .route("/quota", axum::routing::get(quota::get_quota))
        .route("/activity", axum::routing::get(activity::get_activity))
        .route("/tags", axum::routing::get(tags::list_tags))
        .route(
            "/tags/{path}",
            axum::routing::get(tags::get_tags).post(tags::add_tags),
        )
        .route(
            "/tags/{path}/{tag}",
            axum::routing::delete(tags::remove_tag),
        )
        .route("/tags/search", axum::routing::get(tags::search_by_tag))
        .route(
            "/comments",
            axum::routing::get(comments::list_comments_handler)
                .post(comments::create_comment_handler),
        )
        .route(
            "/comments/:id",
            axum::routing::put(comments::update_comment_handler)
                .delete(comments::delete_comment_handler),
        )
        .route(
            "/comments/:id/resolve",
            axum::routing::post(comments::resolve_comment_handler),
        )
        .route(
            "/health/storage",
            axum::routing::get(storage_health::storage_health_handler),
        )
        .route(
            "/thumbnail/*path",
            axum::routing::get(thumbnails::get_thumbnail),
        )
        .route(
            "/preferences",
            axum::routing::get(search::handle_get_preferences)
                .put(search::handle_update_preferences),
        )
        .route("/locks", axum::routing::get(search::handle_list_locks))
        .route(
            "/locks/force-unlock",
            axum::routing::post(search::handle_force_unlock),
        )
        .route(
            "/locks/{token}",
            axum::routing::delete(search::handle_unlock_by_token),
        )
        .route("/admin/stats", axum::routing::get(admin_api::admin_stats))
        .route(
            "/admin/storage",
            axum::routing::get(admin_api::admin_storage),
        )
        .route(
            "/admin/storage/stats",
            axum::routing::get(admin_api::admin_storage_stats),
        )
        .route("/admin/audit", axum::routing::get(admin_api::admin_audit))
        .route(
            "/admin/audit/summary",
            axum::routing::get(admin_api::admin_audit_summary),
        )
        .route(
            "/admin/maintenance",
            axum::routing::get(admin_api::admin_maintenance).post(admin_api::admin_maintenance),
        )
        .route(
            "/admin/backup/:id",
            axum::routing::delete(backup::delete_backup),
        )
        .route("/admin/backup", axum::routing::post(backup::create_backup))
        .route("/admin/backups", axum::routing::get(backup::list_backups))
        .route(
            "/admin/integrity",
            axum::routing::get(backup::audit_integrity),
        )
        .route(
            "/admin/audit-chain",
            axum::routing::get(backup::audit_chain_verify),
        )
        .route(
            "/admin/restore",
            axum::routing::post(backup::restore_backup),
        )
        .route(
            "/admin/webhooks/:id",
            axum::routing::delete(webhooks::delete_webhook),
        )
        .route(
            "/admin/webhooks",
            axum::routing::post(webhooks::create_webhook).get(webhooks::list_webhooks),
        )
        .route(
            "/admin/users",
            axum::routing::post(user_api::create_user).get(admin_api::admin_list_users),
        )
        .route(
            "/admin/users/{id}",
            axum::routing::get(admin_api::admin_get_user)
                .put(user_api::update_user)
                .delete(admin_api::admin_delete_user),
        )
        .route(
            "/admin/users/{id}/reset-password",
            axum::routing::post(user_api::reset_password),
        )
        .route(
            "/admin/users/{id}/role",
            axum::routing::put(admin_api::admin_set_user_role),
        )
        // Branding (G-09)
        .route(
            "/admin/branding",
            axum::routing::get(branding::get_branding)
                .put(branding::update_branding)
                .delete(branding::reset_branding),
        )
        // Guest accounts (G-10)
        .route(
            "/admin/guests",
            axum::routing::post(guests::create_guest).get(guests::list_guests),
        )
        .route(
            "/admin/guests/{id}",
            axum::routing::delete(guests::revoke_guest),
        )
        // Data retention policies (G-23)
        .route(
            "/admin/retention/policies",
            axum::routing::get(retention::list_policies).post(retention::create_policy),
        )
        .route(
            "/admin/retention/policies/{id}",
            axum::routing::delete(retention::delete_policy),
        )
        .route(
            "/admin/retention/execute",
            axum::routing::post(retention::execute_policies),
        )
        // WORM policies
        .route(
            "/admin/worm/policies",
            axum::routing::get(worm::list_policies).post(worm::create_policy),
        )
        .route(
            "/admin/worm/policies/{id}",
            axum::routing::delete(worm::delete_policy),
        )
        // GDPR compliance (G-13)
        .route("/admin/gdpr", axum::routing::get(gdpr::list_gdpr_requests))
        .route(
            "/admin/users/{id}/export",
            axum::routing::post(gdpr::request_data_export).get(admin_api::admin_export_user_data),
        )
        .route(
            "/admin/users/{id}/data",
            axum::routing::delete(admin_api::admin_erase_user_data),
        )
        // Event triggers (G-16)
        .route(
            "/admin/triggers",
            axum::routing::post(event_triggers::create_event_trigger)
                .get(event_triggers::list_event_triggers),
        )
        .route(
            "/admin/triggers/{id}",
            axum::routing::delete(event_triggers::delete_event_trigger),
        )
        .route(
            "/admin/triggers/{id}/toggle",
            axum::routing::post(event_triggers::toggle_event_trigger),
        )
        // Extended shares (G-24, G-25)
        .route(
            "/shares/ext",
            axum::routing::post(shares_ext::create_share_ext),
        )
        .route(
            "/users/me",
            axum::routing::get(user_api::get_current_user).put(user_api::update_current_user),
        )
        .nest(
            "",
            ferro_server_versioning::routes().layer(axum::Extension(
                ferro_server_versioning::VersioningState {
                    data_dir: state.data_dir.clone(),
                    admin_user: state.admin_user.clone(),
                    storage: state.storage.clone(),
                    max_file_versions: state.max_file_versions,
                },
            )),
        )
        .nest(
            "/webrtc",
            ferro_server_webrtc::routes(ferro_server_webrtc::WebRtcState {
                offers: webrtc_offers,
            }),
        )
        .route(
            "/graphql",
            axum::routing::get(ferro_graphql::graphql_playground)
                .post(ferro_graphql::graphql_handler),
        )
        .route(
            "/sync/events",
            axum::routing::get(sync::events::sync_events),
        )
        .route("/sync/delta", axum::routing::get(sync::events::sync_delta))
        .route(
            "/sync/status",
            axum::routing::get(sync::events::sync_status),
        )
        // Block sync protocol
        .route(
            "/sync/blocks/manifest",
            axum::routing::get(sync::blocks::get_manifest),
        )
        .route(
            "/sync/blocks/upload",
            axum::routing::post(sync::blocks::upload_blocks),
        )
        .route(
            "/sync/blocks/check",
            axum::routing::get(sync::blocks::check_blocks),
        )
        .route(
            "/sync/blocks/assemble",
            axum::routing::post(sync::blocks::assemble_file),
        )
        .route(
            "/sync/blocks/{hash}",
            axum::routing::get(sync::blocks::get_block),
        )
        .route("/ws", axum::routing::get(ws::ws_handler))
        .route("/upload/init", axum::routing::post(upload::init_upload))
        .route(
            "/upload/:upload_id/chunk/:chunk_index",
            axum::routing::put(upload::upload_chunk),
        )
        .route(
            "/upload/:upload_id/complete",
            axum::routing::post(upload::complete_upload),
        )
        .route(
            "/upload/:upload_id",
            axum::routing::delete(upload::cancel_upload),
        )
        .route("/uploads", axum::routing::get(upload::list_uploads))
        .merge(Router::from(openapi::swagger_ui()))
}

pub fn build_router_with_static(
    state: AppState,
    static_dir: Option<&str>,
    cors_allowed_origins: &str,
    api_version: &str,
) -> Router {
    let request_counter = state.request_count.clone();
    let duration_buckets = state.request_duration_buckets.clone();
    let duration_sum_ms = state.request_duration_sum_ms.clone();
    let status_counts = state.request_status_counts.clone();
    let storage_op_counts = state.storage_op_counts.clone();
    let auth_enabled = state.auth_enabled();
    let oidc = state.oidc.clone();
    let cedar = state.cedar.clone();
    let auth_layer = axum::middleware::from_fn(move |req, next| {
        let fut: std::pin::Pin<
            Box<dyn std::future::Future<Output = axum::response::Response> + Send>,
        > = if auth_enabled {
            Box::pin(auth::oidc::auth_middleware(oidc.clone(), req, next))
        } else {
            let mut req = req;
            req.extensions_mut()
                .insert(common::auth::Claims::anonymous());
            Box::pin(next.run(req))
        };
        fut
    });

    let cedar_layer = axum::middleware::from_fn(move |req, next| {
        Box::pin(auth::cedar::cedar_middleware(cedar.clone(), req, next))
    });

    let admin_user = state.admin_user.clone();
    let admin_password = state.admin_password.clone();
    let admin_password_for_default_check = admin_password.clone();
    let admin_password_rotated = state.admin_password_rotated.clone();
    let user_store = state.user_store.clone();
    let simple_auth_layer =
        axum::middleware::from_fn(move |req: axum::http::Request<Body>, next: Next| {
            simple_auth::simple_auth_middleware(
                req,
                admin_user.clone(),
                admin_password.clone(),
                user_store.clone(),
                next,
            )
        });

    // Enforce password change when default password is in use.
    // This runs AFTER simple_auth, so we know the request passed authentication.
    let default_password_layer =
        axum::middleware::from_fn(move |req: axum::http::Request<Body>, next: Next| {
            let pw = admin_password_for_default_check.clone();
            let rotated = admin_password_rotated.clone();
            async move {
                if !rotated.load(std::sync::atomic::Ordering::Relaxed)
                    && let Some(ref pw_val) = pw
                    && security::is_default_password(pw_val)
                {
                    let path = req.uri().path();
                    if !security::is_password_change_allowed_path(path) {
                        return Ok::<_, std::convert::Infallible>(
                            security::response_require_password_change(),
                        );
                    }
                }
                Ok(next.run(req).await)
            }
        });

    let maintenance_mode = state.maintenance_mode.clone();
    let maintenance_layer = axum::middleware::from_fn(
        move |req: axum::http::Request<Body>, next: Next| {
            let flag = maintenance_mode.clone();
            async move {
                if flag.load(std::sync::atomic::Ordering::Relaxed) {
                    let method = req.method();
                    let path = req.uri().path();
                    // Allow read operations and the maintenance toggle endpoint.
                    let is_read = matches!(method.as_str(), "GET" | "HEAD" | "OPTIONS");
                    // Allow the admin maintenance toggle even during maintenance.
                    let is_maintenance_toggle = path == "/api/admin/maintenance";
                    if !is_read && !is_maintenance_toggle {
                        return Ok::<_, std::convert::Infallible>(
                            crate::api_error::ApiError::service_unavailable(
                                crate::api_error::ApiError::MAINTENANCE_MODE,
                                "Server is in maintenance mode. Write operations are temporarily disabled.",
                            ),
                        );
                    }
                }
                Ok(next.run(req).await)
            }
        },
    );

    let cors_origins = cors_allowed_origins.to_string();
    if cors_origins == "*" {
        tracing::warn!(
            "SECURITY WARNING: CORS is configured to allow all origins ('*'). \
             This is appropriate for development but should be restricted in production."
        );
    }
    let cors_auth_enabled = state.auth_enabled();
    if cors_origins == "*" && cors_auth_enabled {
        tracing::error!(
            "CORS allowed origins is '*' while auth is enabled -- \
             set a specific origin in production to prevent credential theft"
        );
    }
    let cors_layer = axum::middleware::from_fn(move |req: Request<Body>, next: Next| {
        let allowed = cors_origins.clone();
        async move {
            if req.headers().contains_key("origin") {
                let origin_value = if allowed == "*" {
                    axum::http::HeaderValue::from_static("*")
                } else {
                    let req_origin = req
                        .headers()
                        .get("origin")
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("");
                    let origin_str = if allowed.split(',').any(|o| o.trim() == req_origin) {
                        req_origin
                    } else {
                        ""
                    };
                    match axum::http::HeaderValue::from_str(origin_str) {
                        Ok(v) if !origin_str.is_empty() => v,
                        _ => {
                            return (StatusCode::FORBIDDEN, "CORS origin not allowed")
                                .into_response();
                        }
                    }
                };

                if req.method() == axum::http::Method::OPTIONS {
                    let mut headers = axum::http::HeaderMap::new();
                    headers.insert("Access-Control-Allow-Origin", origin_value);
                    headers.insert("Access-Control-Allow-Methods", axum::http::HeaderValue::from_static(
                        "GET, POST, PUT, DELETE, PATCH, OPTIONS, PROPFIND, MKCOL, COPY, MOVE, LOCK, UNLOCK, PROPPATCH"
                    ));
                    headers.insert("Access-Control-Allow-Headers", axum::http::HeaderValue::from_static(
                        "Content-Type, Authorization, Depth, Destination, If, If-Match, If-None-Match, Lock-Token, Overwrite"
                    ));
                    headers.insert(
                        "Access-Control-Max-Age",
                        axum::http::HeaderValue::from_static("86400"),
                    );
                    return (StatusCode::NO_CONTENT, headers, "").into_response();
                }

                let mut response = next.run(req).await;
                response
                    .headers_mut()
                    .insert("Access-Control-Allow-Origin", origin_value);
                response.headers_mut().insert(
                    "Access-Control-Expose-Headers",
                    axum::http::HeaderValue::from_static("ETag, Content-Length, DAV, Lock-Token"),
                );
                response
            } else {
                next.run(req).await
            }
        }
    });

    let rate_limiter = Arc::new(rate_limit::RateLimiter::new(
        rate_limit::RateLimiterConfig {
            max_requests: 10_000,
            window: std::time::Duration::from_secs(60),
        },
    ));
    let rate_limit_layer =
        axum::middleware::from_fn(move |req: axum::http::Request<Body>, next: Next| {
            let limiter = rate_limiter.clone();
            async move {
                let client_ip = req
                    .headers()
                    .get("x-forwarded-for")
                    .and_then(|v: &axum::http::HeaderValue| v.to_str().ok())
                    .and_then(|s: &str| s.split(',').next())
                    .map(|s: &str| s.trim().to_string())
                    .unwrap_or_else(|| "unknown".to_string());

                if limiter.check(&client_ip).await {
                    next.run(req).await
                } else {
                    api_error::ApiError::too_many_requests(
                        api_error::ApiError::RATE_LIMITED,
                        "Rate limit exceeded",
                    )
                }
            }
        });

    let versioned_api_path = format!("/api/{}", api_version);
    let api_version_for_header = api_version.to_string();
    let deprecation_layer = axum::middleware::from_fn(
        move |req: axum::extract::Request, next: axum::middleware::Next| {
            let ver = api_version_for_header.clone();
            async move {
                let mut response = next.run(req).await;
                response.headers_mut().insert(
                    axum::http::HeaderName::from_static("deprecation"),
                    axum::http::HeaderValue::from_static("true"),
                );
                response.headers_mut().insert(
                    axum::http::HeaderName::from_static("sunset"),
                    axum::http::HeaderValue::from_static("Sat, 01 May 2027 00:00:00 GMT"),
                );
                let link = format!("</api/{}>; rel=\"successor-version\"", ver);
                let header_value = axum::http::HeaderValue::from_str(&link)
                    .unwrap_or_else(|_| axum::http::HeaderValue::from_static("invalid-version"));
                response
                    .headers_mut()
                    .insert(axum::http::header::LINK, header_value);
                response
            }
        },
    );

    let router = Router::new()
        .route("/", any(webdav::handle_any))
        .route("/.well-known/ferro", axum::routing::get(health_check))
        .route("/healthz", axum::routing::get(liveness))
        .route("/readyz", axum::routing::get(readiness))
        .route("/startupz", axum::routing::get(startup))
        .route(
            "/s/:token",
            axum::routing::get(shares::serve_share).post(shares::handle_share_upload),
        )
        // Remote mount management
        .route(
            "/admin/mounts",
            axum::routing::get(remote_mount::list_mounts).post(remote_mount::create_mount),
        )
        .route(
            "/admin/mounts/{id}",
            axum::routing::delete(remote_mount::delete_mount),
        )
        .route(
            "/admin/mounts/{id}/test",
            axum::routing::get(remote_mount::test_mount),
        )
        // Extended share endpoints (G-24, G-25)
        .route(
            "/s/:token/upload",
            axum::routing::post(shares_ext::upload_to_share),
        )
        .route(
            "/s/:token/uploads",
            axum::routing::get(shares_ext::list_share_uploads),
        )
        .route(
            "/s/:token/view",
            axum::routing::get(shares_ext::serve_view_share),
        )
        .nest(
            "/wopi",
            ferro_server_wopi::routes::<AppState>().layer(axum::Extension(
                ferro_server_wopi::WopiState {
                    storage: state.storage.clone(),
                    lock_manager: state.lock_manager.clone(),
                    wopi_token_secret: state.wopi_token_secret.clone(),
                    wopi_office_url: state.wopi_office_url.clone(),
                },
            )),
        )
        .nest(
            "/hosting",
            ferro_server_wopi::discovery_route::<AppState>().layer(axum::Extension(
                ferro_server_wopi::WopiState {
                    storage: state.storage.clone(),
                    lock_manager: state.lock_manager.clone(),
                    wopi_token_secret: state.wopi_token_secret.clone(),
                    wopi_office_url: state.wopi_office_url.clone(),
                },
            )),
        )
        .route("/metrics", axum::routing::get(metrics::metrics_handler))
        .route(
            "/metrics/prometheus",
            axum::routing::get(prometheus_metrics::prometheus_metrics_handler),
        )
        .route(
            "/.well-known/webfinger",
            axum::routing::get(federation::webfinger),
        )
        .route(
            "/fed/actor/{username}",
            axum::routing::get(federation::get_actor),
        )
        .route(
            "/fed/actor/{username}/followers",
            axum::routing::get(federation::list_followers),
        )
        .route(
            "/fed/actor/{username}/following",
            axum::routing::get(federation::list_following),
        )
        .route(
            "/fed/inbox",
            axum::routing::post(federation::inbox).get(federation::list_inbox),
        )
        .route("/fed/outbox", axum::routing::get(federation::list_outbox))
        .route("/fed/nodeinfo", axum::routing::get(federation::nodeinfo))
        .nest(
            &versioned_api_path,
            api_routes(&state, state.webrtc_offers.clone()),
        )
        .nest(
            "/api",
            api_routes(&state, state.webrtc_offers.clone()).layer(deprecation_layer),
        )
        // CalDAV and CardDAV routes (registered before /*path catch-all)
        .route("/dav/cal", axum::routing::options(dav::caldav_options))
        .route(
            "/dav/cal/",
            axum::routing::get(dav::caldav_list).put(dav::caldav_create),
        )
        .route(
            "/dav/cal/{calendar}",
            axum::routing::delete(dav::caldav_delete),
        )
        .route(
            "/dav/cal/{calendar}/",
            axum::routing::get(dav::caldav_props),
        )
        .route(
            "/dav/cal/{calendar}/{uid}.ics",
            axum::routing::get(dav::caldav_get_event)
                .put(dav::caldav_put_event)
                .delete(dav::caldav_delete_event),
        )
        .route("/dav/card", axum::routing::options(dav::carddav_options))
        .route(
            "/dav/card/",
            axum::routing::get(dav::carddav_list).put(dav::carddav_create),
        )
        .route(
            "/dav/card/{book}",
            axum::routing::delete(dav::carddav_delete),
        )
        .route("/dav/card/{book}/", axum::routing::get(dav::carddav_props))
        .route(
            "/dav/card/{book}/{uid}.vcf",
            axum::routing::get(dav::carddav_get_contact)
                .put(dav::carddav_put_contact)
                .delete(dav::carddav_delete_contact),
        )
        .route(
            "/remote/*path",
            axum::routing::any(remote_mount::proxy_remote_mount),
        )
        // Combined fallback: dispatches REST file content requests vs WebDAV.
        //
        // matchit 0.7.3 does not allow catch-all parameters ({*path}) inside
        // nested routes, and .nest("/api/v1", ...) prevents a top-level
        // /api/v1/files/{*path} from matching (the nested router claims the
        // /api/v1 prefix). Using fallback() ensures we run after all route
        // matching, and we dispatch based on path prefix.
        .fallback(api_and_webdav_fallback)
        .layer(rate_limit_layer)
        .layer(cedar_layer)
        .layer(auth_layer)
        .layer(simple_auth_layer)
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            guests::guest_expiry_middleware,
        ))
        .layer(default_password_layer)
        .layer(maintenance_layer)
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            security::auth_guard_middleware,
        ))
        .layer(cors_layer)
        .layer(axum::middleware::from_fn(request_id::request_id_middleware))
        .layer(axum::middleware::from_fn(
            move |req: Request<Body>, next: Next| {
                let counter = request_counter.clone();
                let buckets = duration_buckets.clone();
                let statuses = status_counts.clone();
                let storage_ops = storage_op_counts.clone();
                let sum = duration_sum_ms.clone();
                request_logging::request_logging_middleware(
                    counter,
                    buckets,
                    sum,
                    statuses,
                    Some(storage_ops),
                    req,
                    next,
                )
            },
        ))
        .layer(axum::middleware::from_fn(
            security_headers::security_headers_middleware,
        ))
        .layer(axum::middleware::from_fn(
            security_headers::panic_handler_middleware,
        ))
        .layer(CompressionLayer::new())
        .layer(axum::extract::DefaultBodyLimit::max(
            state.max_body_size as usize,
        ))
        // Cap concurrent in-flight requests to prevent the tokio runtime and
        // storage backend from being overwhelmed. Excess connections queue in
        // the kernel listen backlog instead of competing for resources.
        .layer(ConcurrencyLimitLayer::new(128))
        // Reject requests with both Content-Length and Transfer-Encoding
        // to prevent HTTP request smuggling (CL-TE / TE-CL desync).
        .layer(axum::middleware::from_fn(
            security::smuggling_rejection_middleware,
        ))
        .with_state(state.clone())
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            audit::audit_middleware,
        ));

    let schema = ferro_graphql::build_schema(state.graphql_context());
    let mut router = router.layer(axum::Extension(schema));

    if let Some(dir) = static_dir {
        let static_dir_path = std::path::Path::new(dir);
        tracing::info!("Serving static web assets from {:?}", static_dir_path);
        let serve_dir = ServeDir::new(static_dir_path)
            .fallback(ServeFile::new(static_dir_path.join("index.html")));
        router = router.nest_service("/ui", serve_dir);
    }

    router
}

pub async fn liveness() -> impl IntoResponse {
    (StatusCode::OK, "ok")
}

/// GET /startupz — Kubernetes-style startup probe.
/// Returns 200 once the server has completed all startup checks
/// (storage reachability, CAS verification, DB init). Returns 503 until then.
pub async fn startup(State(state): State<AppState>) -> Response {
    use std::sync::atomic::Ordering;
    if state.startup_complete.load(Ordering::Relaxed) {
        (
            StatusCode::OK,
            axum::Json(serde_json::json!({"status": "ok"})),
        )
            .into_response()
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            axum::Json(serde_json::json!({"status": "starting"})),
        )
            .into_response()
    }
}

pub async fn readiness(State(state): State<AppState>) -> Response {
    let mut subsystems = serde_json::Map::new();
    let mut healthy = true;

    let storage_ok = state.storage.list("/").await.is_ok();
    subsystems.insert(
        "storage".to_string(),
        serde_json::json!(if storage_ok { "ok" } else { "error" }),
    );
    if !storage_ok {
        healthy = false;
    }

    subsystems.insert(
        "metadata".to_string(),
        serde_json::json!(if state.metadata_store.is_some() {
            "persistent"
        } else {
            "in-memory"
        }),
    );

    // Check SQLite database reachability if configured.
    let db_ok = match &state.db {
        Some(db) => db
            .lock()
            .ok()
            .and_then(|conn| conn.execute_batch("SELECT 1;").ok())
            .is_some(),
        None => true, // No DB configured, not a failure.
    };
    subsystems.insert(
        "database".to_string(),
        serde_json::json!(if db_ok { "ok" } else { "error" }),
    );
    if !db_ok {
        healthy = false;
    }

    // Check search index readiness if configured.
    let search_ok = match &state.search {
        Some(search) => search.try_read().is_ok(),
        None => true, // No search configured, not a failure.
    };
    subsystems.insert(
        "search".to_string(),
        serde_json::json!(if search_ok { "ok" } else { "error" }),
    );
    if !search_ok {
        healthy = false;
    }

    let status = if healthy { "ok" } else { "degraded" };
    let code = if healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    let body = serde_json::json!({
        "status": status,
        "subsystems": subsystems,
    });
    (code, axum::Json(body)).into_response()
}

/// Combined fallback that dispatches REST API file content requests to the
/// REST handler and everything else to the WebDAV handler.
///
/// This is necessary because matchit 0.7.3 does not support catch-all
/// parameters (`{*path}`) inside nested routes, and `.nest("/api/v1", ...)`
/// prevents a top-level `/api/v1/files/{*path}` from matching paths that
/// start with `/api/v1/`. Using `fallback()` ensures we run after all
/// matchit route matching is complete.
pub async fn api_and_webdav_fallback(
    method: axum::http::Method,
    uri: axum::http::Uri,
    State(state): State<AppState>,
    headers: HeaderMap,
    body: axum::body::Body,
) -> Response {
    let path_str = uri.path().to_string();
    // Check for both /api/v1/files/ and /api/files/ (deprecated) prefixes
    let rest = path_str
        .strip_prefix("/api/v1/files/")
        .or_else(|| path_str.strip_prefix("/api/files/"));

    if let Some(file_path) = rest
        && !file_path.is_empty()
    {
        // ----------------------------------------------------------------
        // Versioning API: intercept before the generic file-content handler.
        //
        // matchit `{path}` only captures a single segment, so nested paths
        // like /api/v1/files/docs/test.txt/versions never match the
        // versioning routes registered via .nest(""). They fall through to
        // this fallback, which previously treated them as file content
        // requests. We check for the /versions and /diff suffixes here.
        // ----------------------------------------------------------------

        // GET|DELETE /files/{*path}/versions/{version_id}
        if let Some(idx) = file_path.rfind("/versions/") {
            let filepath = &file_path[..idx];
            let after = &file_path[idx + "/versions/".len()..];
            if !filepath.is_empty()
                && let Ok(vid) = after.parse::<u64>()
            {
                let ver_state = ferro_server_versioning::VersioningState {
                    data_dir: state.data_dir.clone(),
                    admin_user: state.admin_user.clone(),
                    storage: state.storage.clone(),
                    max_file_versions: state.max_file_versions,
                };
                return match method {
                    axum::http::Method::GET => {
                        ferro_server_versioning::get_version(
                            axum::Extension(ver_state),
                            axum::extract::Path((filepath.to_string(), vid)),
                        )
                        .await
                    }
                    axum::http::Method::DELETE => {
                        ferro_server_versioning::delete_version(
                            axum::Extension(ver_state),
                            axum::extract::Path((filepath.to_string(), vid)),
                        )
                        .await
                    }
                    _ => (
                        axum::http::StatusCode::METHOD_NOT_ALLOWED,
                        "Method not allowed",
                    )
                        .into_response(),
                };
            }
        }

        // GET|POST /files/{*path}/versions
        if let Some(filepath) = file_path.strip_suffix("/versions")
            && !filepath.is_empty()
            && matches!(method, axum::http::Method::GET | axum::http::Method::POST)
        {
            let ver_state = ferro_server_versioning::VersioningState {
                data_dir: state.data_dir.clone(),
                admin_user: state.admin_user.clone(),
                storage: state.storage.clone(),
                max_file_versions: state.max_file_versions,
            };
            return match method {
                axum::http::Method::GET => {
                    ferro_server_versioning::list_versions(
                        axum::Extension(ver_state),
                        axum::extract::Path(filepath.to_string()),
                    )
                    .await
                }
                axum::http::Method::POST => {
                    ferro_server_versioning::create_version(
                        axum::Extension(ver_state),
                        axum::extract::Path(filepath.to_string()),
                    )
                    .await
                }
                _ => StatusCode::METHOD_NOT_ALLOWED.into_response(),
            };
        }

        // GET /files/{*path}/diff
        if let Some(filepath) = file_path.strip_suffix("/diff")
            && !filepath.is_empty()
            && method == axum::http::Method::GET
        {
            let ver_state = ferro_server_versioning::VersioningState {
                data_dir: state.data_dir.clone(),
                admin_user: state.admin_user.clone(),
                storage: state.storage.clone(),
                max_file_versions: state.max_file_versions,
            };
            let params: std::collections::HashMap<String, String> = uri
                .query()
                .map(|q| {
                    q.split('&')
                        .filter_map(|p| {
                            let mut parts = p.splitn(2, '=');
                            Some((parts.next()?.to_string(), parts.next()?.to_string()))
                        })
                        .collect()
                })
                .unwrap_or_default();
            return ferro_server_versioning::diff_versions(
                axum::Extension(ver_state),
                axum::extract::Path(filepath.to_string()),
                axum::extract::Query(ferro_server_versioning::DiffParams {
                    from: params.get("from").cloned().unwrap_or_default(),
                    to: params.get("to").cloned().unwrap_or_default(),
                }),
            )
            .await;
        }

        // File content handler (original behavior)
        let body_bytes = match http_body_util::BodyExt::collect(body).await {
            Ok(collected) => collected.to_bytes(),
            Err(_) => {
                return (
                    axum::http::StatusCode::BAD_REQUEST,
                    "Failed to read request body",
                )
                    .into_response();
            }
        };
        return api::files_content_handler(
            method,
            uri,
            State(state),
            headers,
            Some(axum::extract::Path(file_path.to_string())),
            body_bytes,
        )
        .await;
    }
    // Fall through to WebDAV handler
    webdav::handle_any(method, uri, State(state), None, headers, body).await
}

pub async fn health_check(State(state): State<AppState>) -> Response {
    let mut subsystems = serde_json::Map::new();
    let mut healthy = true;

    let storage_ok = state.storage.list("/").await.is_ok();
    subsystems.insert(
        "storage".to_string(),
        serde_json::json!(if storage_ok { "ok" } else { "error" }),
    );
    if !storage_ok {
        healthy = false;
    }

    subsystems.insert(
        "metadata".to_string(),
        serde_json::json!(if state.metadata_store.is_some() {
            "persistent"
        } else {
            "in-memory"
        }),
    );

    subsystems.insert(
        "wasm".to_string(),
        serde_json::json!(if state.wasm_runtime.is_some() {
            "ok"
        } else {
            "disabled"
        }),
    );

    subsystems.insert(
        "search".to_string(),
        serde_json::json!(if state.search.is_some() {
            "ok"
        } else {
            "disabled"
        }),
    );

    subsystems.insert(
        "auth".to_string(),
        serde_json::json!(if state.oidc.is_some() {
            "configured"
        } else {
            "disabled"
        }),
    );

    subsystems.insert(
        "cas".to_string(),
        serde_json::json!(if state.cas_store.is_some() {
            "enabled"
        } else {
            "disabled"
        }),
    );

    let status = if healthy { "ok" } else { "degraded" };
    let code = if healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    let body = serde_json::json!({
        "status": status,
        "version": env!("CARGO_PKG_VERSION"),
        "uptime_seconds": state.started_at.elapsed().as_secs(),
        "subsystems": subsystems,
    });
    (code, axum::Json(body)).into_response()
}

pub async fn audit_handler(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Response {
    let limit: usize = params
        .get("limit")
        .and_then(|v| v.parse().ok())
        .unwrap_or(100);
    let offset: usize = params
        .get("offset")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let total = state.audit_log.len().await;
    let entries = state.audit_log.recent_with_offset(limit, offset).await;
    (
        axum::http::StatusCode::OK,
        axum::Json(serde_json::json!({
            "entries": entries,
            "total": total,
            "limit": limit,
            "offset": offset,
        })),
    )
        .into_response()
}

pub async fn storage_stats(State(state): State<AppState>) -> Response {
    let mut file_count = 0u64;
    let mut total_size = 0u64;
    let mut collection_count = 0u64;

    if let Ok(entries) = state.storage.list_all("/", 1000).await {
        for meta in &entries {
            if meta.is_collection {
                collection_count += 1;
            } else {
                file_count += 1;
                total_size += meta.size;
            }
        }
    }

    let cas_stats: serde_json::Value = if let Some(cas) = &state.cas_store {
        serde_json::json!({
            "enabled": true,
            "content_blocks": cas.content_count().await,
        })
    } else {
        serde_json::json!({"enabled": false})
    };

    (
        axum::http::StatusCode::OK,
        axum::Json(serde_json::json!({
            "files": file_count,
            "collections": collection_count,
            "total_bytes": total_size,
            "cas": cas_stats,
            "metadata_store": state.metadata_store.is_some(),
        })),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_deprecation_headers_on_legacy_api() {
        let app = build_router(AppState::in_memory());

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/config")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        assert_eq!(response.headers().get("deprecation").unwrap(), "true");
        assert_eq!(
            response.headers().get("sunset").unwrap(),
            "Sat, 01 May 2027 00:00:00 GMT"
        );
        assert_eq!(
            response.headers().get("link").unwrap(),
            "</api/v1>; rel=\"successor-version\""
        );
    }

    #[tokio::test]
    async fn test_no_deprecation_headers_on_versioned_api() {
        let app = build_router(AppState::in_memory());

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/v1/config")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        assert!(response.headers().get("deprecation").is_none());
        assert!(response.headers().get("sunset").is_none());
    }

    #[tokio::test]
    async fn test_versioned_api_returns_same_response() {
        let legacy_resp = build_router(AppState::in_memory())
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/config")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let v1_resp = build_router(AppState::in_memory())
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/v1/config")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let legacy_bytes = legacy_resp.into_body().collect().await.unwrap().to_bytes();
        let v1_bytes = v1_resp.into_body().collect().await.unwrap().to_bytes();
        let legacy_json: serde_json::Value = serde_json::from_slice(&legacy_bytes).unwrap();
        let v1_json: serde_json::Value = serde_json::from_slice(&v1_bytes).unwrap();
        assert_eq!(legacy_json, v1_json);
    }
}
