pub mod activity;
pub mod admin_api;
pub mod api;
pub mod api_error;
pub mod audit;
pub mod auth;
pub mod backup;
pub mod batch;
pub mod bulk;
pub mod config;
pub mod conflict;
pub mod dav;
pub mod encryption;
pub mod error;
pub mod favorites;
pub mod federation;
pub mod graphql;
pub mod idempotency;
pub mod indexer;
pub mod json_logging;
#[cfg(feature = "ldap")]
pub mod ldap_auth;
pub mod lock;
pub mod metrics;
pub mod move_copy;
pub mod object_store_backend;
#[cfg(feature = "pg")]
pub mod pg_state;
pub mod policies;
pub mod preferences;
pub mod presigned;
pub mod prometheus_metrics;
pub mod quota;
pub mod rate_limit;
#[cfg(feature = "redis")]
pub mod redis_lock;
#[cfg(feature = "redis")]
pub mod redis_rate_limiter;
pub mod request_id;
pub mod request_logging;
pub mod search;
pub mod security_headers;
pub mod shares;
pub mod simple_auth;
pub mod snapshots;
pub mod storage;
pub mod storage_health;
pub mod sync;
pub mod tags;
pub mod thumbnails;
pub mod trash;
pub mod user_api;
pub mod user_paths;
pub mod users;
pub mod versioning;
pub mod wasm_upload;
pub mod webdav;
pub mod webhooks;
pub mod webrtc;
pub mod wopi;
pub mod ws;
pub mod worker_runner;
pub mod workers;
pub mod xml;

use axum::Router;
use axum::body::Body;
use axum::extract::State;
use axum::http::{Request, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::any;
use common::storage::StorageEngine;
use dashmap::{DashMap, DashSet};
use lock::{LockManager, LockManagerTrait};
use std::sync::Arc;
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
    pub started_at: std::time::Instant,
    pub favorites: Arc<dyn FavoriteStore>,
    pub trash: Arc<DashMap<String, TrashedEntry>>,
    pub trash_dir: Option<String>,
    pub quota_bytes: Option<u64>,
    pub used_bytes: Arc<std::sync::atomic::AtomicU64>,
    pub file_count: Arc<std::sync::atomic::AtomicU64>,
    pub preferences: Arc<dyn PreferenceStore>,
    pub request_count: Arc<std::sync::atomic::AtomicU64>,
    pub webhooks: Arc<tokio::sync::RwLock<Vec<webhooks::WebhookConfig>>>,
    pub thumbnail_size: u32,
    pub data_dir: Option<String>,
    pub user_store: Arc<dyn UserStoreTrait>,
    pub max_file_versions: u64,
    pub calendar_store: Arc<dyn ferro_dav::store::CalendarStore>,
    pub address_book_store: Arc<dyn ferro_dav::store::AddressBookStore>,
    pub webrtc_offers: Arc<webrtc::offers::OfferStore>,
    pub activity_store: Arc<federation::store::ActivityStore>,
    pub sync_store: Arc<SyncStore>,
    pub tags: Arc<tags::TagStore>,
    pub idempotency_store: Arc<idempotency::IdempotencyStore>,
    pub storage_health: Arc<storage_health::StorageHealthMonitor>,
    pub ws_manager: Arc<ws::WsManager>,
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
            wopi_token_secret: "ferro-wopi-token-secret-change-me".to_string(),
            recently_processed: Arc::new(DashSet::new()),
            wopi_office_url: String::new(),
            admin_user: None,
            admin_password: None,
            started_at: std::time::Instant::now(),
            favorites: Arc::new(favorites::InMemoryFavoriteStore::new()),
            trash: Arc::new(DashMap::new()),
            trash_dir: None,
            quota_bytes: None,
            used_bytes: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            file_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            preferences: Arc::new(search::InMemoryPreferenceStore::new()),
            request_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            webhooks: Arc::new(tokio::sync::RwLock::new(Vec::new())),
            data_dir: None,
            thumbnail_size: 256,
            user_store: Arc::new(InMemoryUserStore::new()),
            max_file_versions: 10,
            calendar_store: Arc::new(ferro_dav::store::InMemoryCalendarStore::new()),
            address_book_store: Arc::new(ferro_dav::store::InMemoryAddressBookStore::new()),
            webrtc_offers: Arc::new(webrtc::offers::OfferStore::new()),
            activity_store: Arc::new(federation::store::ActivityStore::new()),
            sync_store: Arc::new(SyncStore::new()),
            tags: Arc::new(tags::TagStore::new()),
            idempotency_store: Arc::new(idempotency::IdempotencyStore::new()),
            storage_health: Arc::new(storage_health::StorageHealthMonitor::new()),
            ws_manager: Arc::new(ws::WsManager::new()),
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

    pub fn with_user_store(mut self, store: Arc<dyn UserStoreTrait>) -> Self {
        self.user_store = store;
        self
    }

    pub fn with_max_file_versions(mut self, max: u64) -> Self {
        self.max_file_versions = max;
        self
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

pub fn make_app() -> Router {
    let state = AppState::in_memory();
    build_router(state)
}

pub fn build_router(state: AppState) -> Router {
    build_router_with_static(state, None, "*")
}

pub fn build_router_with_static(
    state: AppState,
    static_dir: Option<&str>,
    cors_allowed_origins: &str,
) -> Router {
    let request_counter = state.request_count.clone();
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

    let cors_origins = cors_allowed_origins.to_string();
    let cors_auth_enabled = state.auth_enabled();
    if cors_origins == "*" && cors_auth_enabled {
        tracing::warn!(
            "CORS allowed origins is '*' while auth is enabled — restrict in production"
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

    let router = Router::new()
        .route("/", any(webdav::handle_any))
        .route("/.well-known/ferro", axum::routing::get(health_check))
        .route("/healthz", axum::routing::get(liveness))
        .route("/readyz", axum::routing::get(readiness))
        .route("/api/auth/info", axum::routing::get(api::auth_info))
        .route("/api/auth/login", axum::routing::get(api::auth_login))
        .route("/api/auth/callback", axum::routing::get(api::auth_callback))
        .route("/api/search", axum::routing::get(search::handle_search))
        .route(
            "/api/workers",
            axum::routing::get(workers::list_workers).post(workers::register_worker),
        )
        .route(
            "/api/workers/upload",
            axum::routing::post(wasm_upload::upload_wasm_module),
        )
        .route(
            "/api/workers/modules/{filename}",
            axum::routing::delete(wasm_upload::delete_wasm_module),
        )
        .route(
            "/api/workers/modules",
            axum::routing::get(wasm_upload::list_wasm_modules),
        )
        .route(
            "/api/policies",
            axum::routing::get(policies::list_policies)
                .post(policies::add_policy)
                .delete(policies::delete_policy),
        )
        .route("/api/config", axum::routing::get(config::get_server_config))
        .route(
            "/api/upload-url",
            axum::routing::get(presigned::get_upload_url),
        )
        .route(
            "/api/download-url",
            axum::routing::get(presigned::get_download_url),
        )
        .route(
            "/api/shares",
            axum::routing::get(shares::list_shares).post(shares::create_share),
        )
        .route(
            "/api/shares/:token",
            axum::routing::delete(shares::delete_share),
        )
        .route("/s/:token", axum::routing::get(shares::serve_share))
        .route("/api/audit", axum::routing::get(audit_handler))
        .route("/api/storage/stats", axum::routing::get(storage_stats))
        .route(
            "/api/snapshots",
            axum::routing::get(snapshots::list_snapshots).post(snapshots::create_snapshot),
        )
        .route(
            "/api/snapshots/:id",
            axum::routing::delete(snapshots::delete_snapshot_by_id),
        )
        .route(
            "/api/snapshots/:id/restore",
            axum::routing::post(snapshots::restore_snapshot),
        )
        .route(
            "/wopi/files/*path",
            axum::routing::get(wopi::wopi_get).post(wopi::wopi_post),
        )
        .route(
            "/wopi/files/{path}/token",
            axum::routing::post(wopi::wopi_issue_token),
        )
        .route(
            "/hosting/discovery",
            axum::routing::get(wopi::wopi_discovery),
        )
        .route(
            "/api/favorites",
            axum::routing::get(favorites::list_favorites)
                .put(favorites::add_favorite)
                .delete(favorites::remove_favorite),
        )
        .route("/api/recent", axum::routing::get(favorites::list_recent))
        .route("/api/trash", axum::routing::get(trash::list_trash))
        .route(
            "/api/trash/{path}",
            axum::routing::delete(trash::move_to_trash),
        )
        .route(
            "/api/trash/restore",
            axum::routing::post(trash::restore_trash),
        )
        .route(
            "/api/trash/purge",
            axum::routing::delete(trash::purge_trash),
        )
        .route(
            "/api/trash/empty",
            axum::routing::delete(trash::empty_trash),
        )
        .route("/api/bulk/delete", axum::routing::post(bulk::bulk_delete))
        .route("/api/batch/copy", axum::routing::post(batch::batch_copy))
        .route("/api/batch/move", axum::routing::post(batch::batch_move))
        .route("/api/files/move", axum::routing::post(move_copy::move_file))
        .route("/api/files/copy", axum::routing::post(move_copy::copy_file))
        .route(
            "/api/files/encrypt",
            axum::routing::post(encryption::encrypt_file),
        )
        .route(
            "/api/files/decrypt",
            axum::routing::post(encryption::decrypt_file),
        )
        .route("/api/quota", axum::routing::get(quota::get_quota))
        .route("/api/activity", axum::routing::get(activity::get_activity))
        .route("/api/tags", axum::routing::get(tags::list_tags))
        .route("/api/tags/{path}", axum::routing::get(tags::get_tags).post(tags::add_tags))
        .route("/api/tags/{path}/{tag}", axum::routing::delete(tags::remove_tag))
        .route("/api/tags/search", axum::routing::get(tags::search_by_tag))
        .route("/api/health/storage", axum::routing::get(storage_health::storage_health_handler))
        .route(
            "/api/thumbnail/*path",
            axum::routing::get(thumbnails::get_thumbnail),
        )
        .route(
            "/api/preferences",
            axum::routing::get(search::handle_get_preferences)
                .put(search::handle_update_preferences),
        )
        .route("/api/locks", axum::routing::get(search::handle_list_locks))
        .route(
            "/api/locks/force-unlock",
            axum::routing::post(search::handle_force_unlock),
        )
        .route(
            "/api/locks/{token}",
            axum::routing::delete(search::handle_unlock_by_token),
        )
        .route("/metrics", axum::routing::get(metrics::metrics_handler))
        .route(
            "/metrics/prometheus",
            axum::routing::get(prometheus_metrics::prometheus_metrics_handler),
        )
        .route(
            "/api/admin/stats",
            axum::routing::get(admin_api::admin_stats),
        )
        .route(
            "/api/admin/storage",
            axum::routing::get(admin_api::admin_storage),
        )
        .route(
            "/api/admin/audit",
            axum::routing::get(admin_api::admin_audit),
        )
        .route(
            "/api/admin/backup/:id",
            axum::routing::delete(backup::delete_backup),
        )
        .route(
            "/api/admin/backup",
            axum::routing::post(backup::create_backup),
        )
        .route(
            "/api/admin/backups",
            axum::routing::get(backup::list_backups),
        )
        .route(
            "/api/admin/restore",
            axum::routing::post(backup::restore_backup),
        )
        .route(
            "/api/admin/webhooks/:id",
            axum::routing::delete(webhooks::delete_webhook),
        )
        .route(
            "/api/admin/webhooks",
            axum::routing::post(webhooks::create_webhook).get(webhooks::list_webhooks),
        )
        .route(
            "/api/admin/users",
            axum::routing::post(user_api::create_user).get(user_api::list_users),
        )
        .route(
            "/api/admin/users/{id}",
            axum::routing::get(user_api::get_user)
                .put(user_api::update_user)
                .delete(user_api::delete_user),
        )
        .route(
            "/api/admin/users/{id}/reset-password",
            axum::routing::post(user_api::reset_password),
        )
        .route(
            "/api/users/me",
            axum::routing::get(user_api::get_current_user).put(user_api::update_current_user),
        )
        .route(
            "/api/files/{path}/versions",
            axum::routing::get(versioning::list_versions).post(versioning::create_version),
        )
        .route(
            "/api/files/{path}/versions/{version_id}",
            axum::routing::get(versioning::get_version).delete(versioning::delete_version),
        )
        .route(
            "/api/files/{path}/diff",
            axum::routing::get(versioning::diff_versions),
        )
        .route(
            "/.well-known/webfinger",
            axum::routing::get(federation::webfinger::webfinger),
        )
        .route(
            "/fed/actor/:username",
            axum::routing::get(federation::get_actor),
        )
        .route(
            "/fed/actor/:username/followers",
            axum::routing::get(federation::list_followers),
        )
        .route(
            "/fed/actor/:username/following",
            axum::routing::get(federation::list_following),
        )
        .route(
            "/fed/inbox",
            axum::routing::post(federation::inbox).get(federation::list_inbox),
        )
        .route("/fed/outbox", axum::routing::get(federation::list_outbox))
        .route("/fed/nodeinfo", axum::routing::get(federation::nodeinfo))
        .route(
            "/api/fed/share",
            axum::routing::post(federation::federated_share),
        )
        .route(
            "/api/webrtc/offer",
            axum::routing::post(webrtc::signaling::create_offer),
        )
        .route(
            "/api/webrtc/offer/:session_id",
            axum::routing::get(webrtc::signaling::get_offer),
        )
        .route(
            "/api/webrtc/offer/:session_id/answer",
            axum::routing::post(webrtc::signaling::submit_answer),
        )
        .route(
            "/api/webrtc/offer/:session_id/ice",
            axum::routing::post(webrtc::signaling::add_ice_candidate),
        )
        .route(
            "/api/webrtc/offer/:session_id/poll",
            axum::routing::get(webrtc::signaling::poll_answer),
        )
        .route(
            "/api/graphql",
            axum::routing::get(graphql::graphql_playground).post(graphql::graphql_handler),
        )
        .route(
            "/api/sync/events",
            axum::routing::get(sync::events::sync_events),
        )
        .route(
            "/api/sync/delta",
            axum::routing::get(sync::events::sync_delta),
        )
        .route(
            "/api/sync/status",
            axum::routing::get(sync::events::sync_status),
        )
        .route("/api/ws", axum::routing::get(ws::ws_handler))
        .route("/*path", any(webdav::handle_any))
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
        .layer(rate_limit_layer)
        .layer(cedar_layer)
        .layer(auth_layer)
        .layer(simple_auth_layer)
        .layer(cors_layer)
        .layer(axum::middleware::from_fn(request_id::request_id_middleware))
        .layer(axum::middleware::from_fn(
            move |req: Request<Body>, next: Next| {
                let counter = request_counter.clone();
                request_logging::request_logging_middleware(counter, req, next)
            },
        ))
        .layer(axum::middleware::from_fn(
            security_headers::security_headers_middleware,
        ))
        .layer(CompressionLayer::new())
        .with_state(state.clone());

    let schema = graphql::build_schema(state);
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
