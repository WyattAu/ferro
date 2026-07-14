// --- Re-exports from extracted crates ---
pub use ferro_distributed::erasure_storage;
pub use ferro_server_admin_api::activity;
pub use ferro_server_admin_api::admin_api;
pub use ferro_server_admin_api::backup;
pub use ferro_server_admin_api::branding;
pub use ferro_server_admin_api::gdpr;
pub use ferro_server_api_core::email;
pub use ferro_server_api_core::event_triggers;
pub use ferro_server_api_core::events;
pub use ferro_server_api_core::search;
pub use ferro_server_api_core::webhooks;
pub use ferro_server_collaboration::chat_api;
pub use ferro_server_collaboration::tags;
pub use ferro_server_compliance::antivirus_api;
#[cfg(unix)]
pub use ferro_server_compliance::clamav;
pub use ferro_server_compliance::dlp_api;
pub use ferro_server_compliance::retention;
pub use ferro_server_compliance::worm;
pub use ferro_server_content::e2ee;
pub use ferro_server_content::encryption;
pub use ferro_server_content::ocr_engine;
pub use ferro_server_content::watermark_api;
pub use ferro_server_infra::api_federation;
pub use ferro_server_infra::federation_sync;
pub use ferro_server_infra::metadata_replication;
#[cfg(feature = "pg")]
pub use ferro_server_infra::pg_state;
#[cfg(feature = "redis")]
pub use ferro_server_infra::redis_lock;
pub use ferro_server_integrations::mail_api;
pub use ferro_server_integrations::offline_api;
pub use ferro_server_integrations::push_notifications;
pub use ferro_server_integrations::read_cache;
pub use ferro_server_integrations::remote_mount;
pub use ferro_server_plugins::plugin_marketplace_api;
pub use ferro_server_plugins::plugin_permissions;
pub use ferro_server_plugins::wasm_upload;
pub use ferro_server_plugins::workers;
pub use ferro_server_productivity::calendar as calendar_api;
pub use ferro_server_productivity::contacts as contacts_api;
pub use ferro_server_productivity::notes as notes_api;
pub use ferro_server_productivity::tasks as tasks_api;
pub use ferro_server_productivity::whiteboard as whiteboard_api;
pub use ferro_server_security_middleware::auth;
pub use ferro_server_security_middleware::request_id;
pub use ferro_server_security_middleware::security_headers;
pub mod security;
pub use ferro_server_storage_ops::dedup;
pub use ferro_server_storage_ops::range_get;
pub use ferro_server_storage_ops::snapshots;
pub use ferro_server_storage_ops::storage_health;
pub use ferro_server_storage_ops::streaming_upload;
pub use ferro_server_storage_ops::thumbnails;
pub use ferro_server_user_mgmt::account_api;
pub use ferro_server_user_mgmt::guests;
pub use ferro_server_user_mgmt::user_api;
pub use ferro_server_webdav_core::dav;
pub use ferro_server_webdav_core::lock;
pub use ferro_server_webdav_core::move_copy;
pub use ferro_server_webdav_core::trash;
pub use ferro_server_webdav_core::webdav;

// --- Local modules ---
pub mod ai_search;
pub mod api;
pub mod api_error;
pub mod api_keys_routes;
pub mod audit;
pub mod batch;
pub mod bulk;
pub mod cache;
pub mod collab_ws;
pub mod comments;
pub mod config;
pub mod conflict;
pub mod connection_pool;
pub mod dashboard;
pub mod db;
pub mod error;
pub mod favorites;
pub mod fs_util;
pub mod idempotency;
pub mod indexer;
pub mod integration;
pub mod json_logging;
#[cfg(feature = "ldap")]
pub mod ldap_auth;
pub mod link_analytics_api;
pub mod metrics;
pub mod notification_prefs_api;
pub mod object_store_backend;
pub mod ocr;
pub mod offline_wiring;
pub mod openapi;
pub mod profiler;
pub use ferro_server_productivity::photos as photos_api;
pub mod policies;
pub mod preferences;
pub mod presigned;
pub mod prometheus_metrics;
pub mod query_optimizer;
pub mod quota;
pub mod ransomware;
#[cfg(feature = "redis")]
pub mod redis_rate_limiter;
pub mod request_logging;
pub mod routes;
pub mod selective_sync_api;
pub mod shares;
pub mod shares_ext;
pub mod simple_auth;
pub mod storage;
pub mod streaming;
pub mod sync;
pub mod tenant_rate_limit_api;
pub mod thumbnail_cache;
pub mod totp_api;
pub mod triggers;
pub mod upload;
pub mod user_paths;
pub mod users;
#[cfg(feature = "webauthn")]
pub mod webauthn_api;
pub mod worker_runner;
pub mod xml;

// --- Extracted wiring modules ---
pub mod federation;
pub mod ws;

// --- Startup modules ---
pub mod cli;
pub mod startup;
pub mod tls;

// --- Core module ---
mod handlers;
mod state;

pub use routes::build_router;
pub use routes::build_router_with_static;
pub use state::AppState;

// Re-export handler functions
pub use handlers::{
    audit_handler, health_check, health_endpoint, liveness, readiness, startup, startup_impl, storage_stats,
};

use axum::Router;

pub fn make_app() -> Router {
    let state = AppState::in_memory().with_wopi_token_secret("test-wopi-secret-for-integration".to_string());
    build_router(state)
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

    #[tokio::test]
    async fn test_health_endpoint() {
        let app = build_router(AppState::in_memory());
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/healthz")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }

    #[tokio::test]
    async fn test_liveness_endpoint() {
        let app = build_router(AppState::in_memory());
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/healthz")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }

    #[tokio::test]
    async fn test_readiness_endpoint() {
        let app = build_router(AppState::in_memory());
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/readyz")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }

    #[tokio::test]
    async fn test_startup_endpoint() {
        let app = build_router(AppState::in_memory());
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/startupz")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        // Server starts in not-started state, so startup probe returns 503
        assert_eq!(response.status(), axum::http::StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn test_storage_stats_endpoint() {
        let app = build_router(AppState::in_memory());
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/v1/storage/stats")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }

    #[tokio::test]
    async fn test_audit_log_endpoint() {
        let app = build_router(AppState::in_memory());
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/v1/audit")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }

    #[tokio::test]
    async fn test_make_app() {
        let app = make_app();
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/healthz")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }

    #[tokio::test]
    async fn test_404_for_unknown_route() {
        let app = build_router(AppState::in_memory());
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/nonexistent")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_webdav_options() {
        let app = build_router(AppState::in_memory());
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("OPTIONS")
                    .uri("/dav/")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(response.status().is_success() || response.status() == axum::http::StatusCode::NOT_FOUND);
    }
}
