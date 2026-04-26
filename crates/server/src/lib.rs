pub mod activity;
pub mod admin_api;
pub mod api;
pub mod api_error;
pub mod audit;
pub mod auth;
pub mod bulk;
pub mod config;
pub mod conflict;
pub mod error;
pub mod favorites;
pub mod indexer;
pub mod lock;
pub mod metrics;
pub mod move_copy;
pub mod object_store_backend;
pub mod policies;
pub mod preferences;
pub mod presigned;
pub mod quota;
pub mod rate_limit;
pub mod request_id;
pub mod request_logging;
pub mod search;
pub mod security_headers;
pub mod shares;
pub mod simple_auth;
pub mod snapshots;
pub mod storage;
pub mod trash;
pub mod user_paths;
pub mod webdav;
pub mod wasm_upload;
pub mod worker_runner;
pub mod workers;
pub mod wopi;
pub mod xml;

use axum::body::Body;
use axum::extract::State;
use axum::http::{Request, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::any;
use axum::Router;
use common::storage::StorageEngine;
use dashmap::{DashMap, DashSet};
use lock::LockManager;
use std::sync::Arc;
use tower_http::compression::CompressionLayer;
use tower_http::services::{ServeDir, ServeFile};

use auth::oidc::OidcValidator;
use auth::cedar::CedarAuthorizer;
use ferro_core::search::SearchEngine;
use ferro_core::wasm::WasmWorkerRuntime;

use audit::AuditLog;
use shares::ShareStore;
use snapshots::SnapshotStore;
use trash::TrashedEntry;
use search::UserPreferences;

#[derive(Clone)]
pub struct AppState {
    pub storage: Arc<dyn StorageEngine>,
    pub lock_manager: Arc<LockManager>,
    pub oidc: Option<Arc<OidcValidator>>,
    pub cedar: Option<Arc<CedarAuthorizer>>,
    pub search: Option<Arc<tokio::sync::RwLock<SearchEngine>>>,
    pub wasm_runtime: Option<Arc<WasmWorkerRuntime>>,
    pub workers_dir: Option<std::path::PathBuf>,
    pub metadata_store: Option<Arc<dyn ferro_core::metadata::MetadataStore>>,
    pub cas_store: Option<Arc<dyn ferro_core::cas::CasStore>>,
    pub presigned_generator: Option<Arc<dyn ferro_core::presigned::PresignedUrlGenerator>>,
    pub share_store: Arc<ShareStore>,
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
    pub favorites: Arc<DashSet<String>>,
    pub trash: Arc<DashMap<String, TrashedEntry>>,
    pub quota_bytes: Option<u64>,
    pub used_bytes: Arc<std::sync::atomic::AtomicU64>,
    pub file_count: Arc<std::sync::atomic::AtomicU64>,
    pub preferences: Arc<tokio::sync::RwLock<UserPreferences>>,
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
            share_store: Arc::new(ShareStore::new()),
            audit_log: Arc::new(AuditLog::new()),
            snapshot_store: Arc::new(SnapshotStore::new(50)),
            max_body_size: 1024 * 1024 * 1024, // 1 GB default
            external_url: "http://localhost:8080".to_string(),
            wopi_token_secret: "ferro-wopi-token-secret-change-me".to_string(),
            recently_processed: Arc::new(DashSet::new()),
            wopi_office_url: String::new(),
            admin_user: None,
            admin_password: None,
            started_at: std::time::Instant::now(),
            favorites: Arc::new(DashSet::new()),
            trash: Arc::new(DashMap::new()),
            quota_bytes: None,
            used_bytes: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            file_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            preferences: Arc::new(tokio::sync::RwLock::new(UserPreferences::default())),
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

    pub fn with_metadata_store(mut self, store: Arc<dyn ferro_core::metadata::MetadataStore>) -> Self {
        self.metadata_store = Some(store);
        self
    }

    pub fn with_cas_store(mut self, store: Arc<dyn ferro_core::cas::CasStore>) -> Self {
        self.cas_store = Some(store);
        self
    }

    pub fn with_presigned_generator(mut self, generator: Arc<dyn ferro_core::presigned::PresignedUrlGenerator>) -> Self {
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
}

pub fn make_app() -> Router {
    let state = AppState::in_memory();
    build_router(state)
}

pub fn build_router(state: AppState) -> Router {
    build_router_with_static(state, None)
}

pub fn build_router_with_static(state: AppState, static_dir: Option<&str>) -> Router {
    let auth_enabled = state.auth_enabled();
    let oidc = state.oidc.clone();
    let cedar = state.cedar.clone();
    let auth_layer = axum::middleware::from_fn(move |req, next| {
        let fut: std::pin::Pin<Box<dyn std::future::Future<Output = axum::response::Response> + Send>> = if auth_enabled {
            Box::pin(auth::oidc::auth_middleware(oidc.clone(), req, next))
        } else {
            let mut req = req;
            req.extensions_mut().insert(common::auth::Claims::anonymous());
            Box::pin(next.run(req))
        };
        fut
    });

    let cedar_layer = axum::middleware::from_fn(move |req, next| {
        Box::pin(auth::cedar::cedar_middleware(cedar.clone(), req, next))
    });

    let admin_user = state.admin_user.clone();
    let admin_password = state.admin_password.clone();
    let simple_auth_layer = axum::middleware::from_fn(move |req: axum::http::Request<Body>, next: Next| {
        simple_auth::simple_auth_middleware(req, admin_user.clone(), admin_password.clone(), next)
    });

    let rate_limiter = Arc::new(rate_limit::RateLimiter::new(rate_limit::RateLimiterConfig {
        max_requests: 10_000, // High limit to avoid false positives in normal use
        window: std::time::Duration::from_secs(60),
    }));
    let rate_limit_layer = axum::middleware::from_fn(move |req: axum::http::Request<Body>, next: Next| {
        let limiter = rate_limiter.clone();
        async move {
            // Extract client IP (prefer X-Forwarded-For, fall back to connect info)
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

    let mut router = Router::new()
        .route("/", any(webdav::handle_any))
        .route("/*path", any(webdav::handle_any))
        .route("/.well-known/ferro", axum::routing::get(health_check))
        .route("/api/auth/info", axum::routing::get(api::auth_info))
        .route("/api/auth/login", axum::routing::get(api::auth_login))
        .route("/api/auth/callback", axum::routing::get(api::auth_callback))
        .route("/api/search", axum::routing::get(search::handle_search))
        .route("/api/workers", axum::routing::get(workers::list_workers).post(workers::register_worker))
        .route("/api/workers/upload", axum::routing::post(wasm_upload::upload_wasm_module))
        .route("/api/workers/modules", axum::routing::get(wasm_upload::list_wasm_modules))
        .route("/api/workers/modules/{filename}", axum::routing::delete(wasm_upload::delete_wasm_module))
        .route("/api/policies", axum::routing::get(policies::list_policies).post(policies::add_policy).delete(policies::delete_policy))
        .route("/api/config", axum::routing::get(config::get_server_config))
        .route("/api/upload-url", axum::routing::get(presigned::get_upload_url))
        .route("/api/download-url", axum::routing::get(presigned::get_download_url))
        .route("/api/shares", axum::routing::get(shares::list_shares).post(shares::create_share))
        .route("/api/shares/:token", axum::routing::delete(shares::delete_share))
        .route("/s/:token", axum::routing::get(shares::serve_share))
        .route("/api/audit", axum::routing::get(audit_handler))
        .route("/api/storage/stats", axum::routing::get(storage_stats))
        .route("/api/snapshots", axum::routing::get(snapshots::list_snapshots).post(snapshots::create_snapshot))
        .route("/api/snapshots/:id", axum::routing::delete(snapshots::delete_snapshot_by_id))
        .route("/api/snapshots/:id/restore", axum::routing::post(snapshots::restore_snapshot))
        .route("/wopi/files/*path", axum::routing::get(wopi::wopi_get).post(wopi::wopi_post))
        .route("/wopi/files/{path}/token", axum::routing::post(wopi::wopi_issue_token))
        .route("/hosting/discovery", axum::routing::get(wopi::wopi_discovery))
        .route("/api/admin/stats", axum::routing::get(admin_api::admin_stats))
        .route("/api/admin/storage", axum::routing::get(admin_api::admin_storage))
        .route("/api/admin/audit", axum::routing::get(admin_api::admin_audit))
        .route("/api/favorites", axum::routing::get(favorites::list_favorites).put(favorites::add_favorite).delete(favorites::remove_favorite))
        .route("/api/recent", axum::routing::get(favorites::list_recent))
        .route("/api/trash", axum::routing::get(trash::list_trash))
        .route("/api/trash/{path}", axum::routing::delete(trash::move_to_trash))
        .route("/api/trash/restore", axum::routing::post(trash::restore_trash))
        .route("/api/trash/purge", axum::routing::delete(trash::purge_trash))
        .route("/api/trash/empty", axum::routing::delete(trash::empty_trash))
        .route("/api/bulk/delete", axum::routing::post(bulk::bulk_delete))
        .route("/api/files/move", axum::routing::post(move_copy::move_file))
        .route("/api/files/copy", axum::routing::post(move_copy::copy_file))
        .route("/api/quota", axum::routing::get(quota::get_quota))
        .route("/api/activity", axum::routing::get(activity::get_activity))
        .route("/api/preferences", axum::routing::get(search::handle_get_preferences).put(search::handle_update_preferences))
        .route("/api/locks", axum::routing::get(search::handle_list_locks))
        .route("/api/locks/force-unlock", axum::routing::post(search::handle_force_unlock))
        .route("/api/locks/{token}", axum::routing::delete(search::handle_unlock_by_token))
        .route("/metrics", axum::routing::get(metrics::metrics_handler))
        .layer(rate_limit_layer) // Rate limiting (before auth, after Cedar)
        .layer(cedar_layer)      // Cedar authorization (after auth)
        .layer(auth_layer)       // OIDC authentication
        .layer(simple_auth_layer)
        .layer(axum::middleware::from_fn(cors_middleware))
        .layer(axum::middleware::from_fn(request_id::request_id_middleware))
        .layer(axum::middleware::from_fn(request_logging::request_logging_middleware))
        .layer(axum::middleware::from_fn(security_headers::security_headers_middleware))
        .layer(CompressionLayer::new())
        .with_state(state);

    // If a static directory is provided, serve the web frontend SPA.
    // Static assets (.js, .wasm) are served directly; everything else
    // falls back to index.html for client-side hash-based routing.
    if let Some(dir) = static_dir {
        let static_dir_path = std::path::Path::new(dir);
        tracing::info!("Serving static web assets from {:?}", static_dir_path);
        let serve_dir = ServeDir::new(static_dir_path)
            .fallback(ServeFile::new(static_dir_path.join("index.html")));
        // Nest under /ui so it doesn't conflict with WebDAV routes
        router = router.nest_service("/ui", serve_dir);
    }

    router
}

pub async fn health_check(State(state): State<AppState>) -> Response {
    let status = "ok";
    let version = env!("CARGO_PKG_VERSION");
    let uptime = state.started_at.elapsed().as_secs();

    let body = serde_json::json!({
        "status": status,
        "version": version,
        "uptime_seconds": uptime,
        "subsystems": {
            "storage": "ok",
            "auth": if state.oidc.is_some() { "configured" } else { "disabled" },
            "search": if state.search.is_some() { "ok" } else { "disabled" },
            "wasm": if state.wasm_runtime.is_some() { "ok" } else { "disabled" },
            "metadata": if state.metadata_store.is_some() { "persistent" } else { "in-memory" },
            "cas": if state.cas_store.is_some() { "enabled" } else { "disabled" },
        }
    });
    (StatusCode::OK, axum::Json(body)).into_response()
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
    (axum::http::StatusCode::OK, axum::Json(serde_json::json!({
        "entries": entries,
        "total": total,
        "limit": limit,
        "offset": offset,
    }))).into_response()
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

    (axum::http::StatusCode::OK, axum::Json(serde_json::json!({
        "files": file_count,
        "collections": collection_count,
        "total_bytes": total_size,
        "cas": cas_stats,
        "metadata_store": state.metadata_store.is_some(),
    }))).into_response()
}

/// Conditional CORS middleware: only applies CORS headers when the request
/// has an `Origin` header (i.e., is a cross-origin request). Same-origin
/// requests (including WebDAV OPTIONS without Origin) pass through untouched.
async fn cors_middleware(req: Request<Body>, next: Next) -> Response {
    if req.headers().contains_key("origin") {
        // For CORS preflight (OPTIONS with Origin), return preflight response
        if req.method() == axum::http::Method::OPTIONS {
            let mut headers = axum::http::HeaderMap::new();
            headers.insert("Access-Control-Allow-Origin", axum::http::HeaderValue::from_static("*"));
            headers.insert("Access-Control-Allow-Methods", axum::http::HeaderValue::from_static(
                "GET, POST, PUT, DELETE, PATCH, OPTIONS, PROPFIND, MKCOL, COPY, MOVE, LOCK, UNLOCK, PROPPATCH"
            ));
            headers.insert("Access-Control-Allow-Headers", axum::http::HeaderValue::from_static(
                "Content-Type, Authorization, Depth, Destination, If, If-Match, If-None-Match, Lock-Token, Overwrite"
            ));
            headers.insert("Access-Control-Max-Age", axum::http::HeaderValue::from_static("86400"));
            return (StatusCode::NO_CONTENT, headers, "").into_response();
        }

        let mut response = next.run(req).await;
        response.headers_mut().insert(
            "Access-Control-Allow-Origin",
            axum::http::HeaderValue::from_static("*"),
        );
        response.headers_mut().insert(
            "Access-Control-Expose-Headers",
            axum::http::HeaderValue::from_static("ETag, Content-Length, DAV, Lock-Token"),
        );
        response
    } else {
        next.run(req).await
    }
}
