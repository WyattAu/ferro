pub mod email;
pub mod event_triggers;
pub mod events;
pub mod search;
pub mod webhooks;
pub mod ws;

use std::sync::Arc;

pub type DbHandle = Arc<std::sync::Mutex<rusqlite::Connection>>;

// ---------------------------------------------------------------------------
// ApiError (local copy matching ferro-server's api_error module)
// ---------------------------------------------------------------------------

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

pub struct ApiError;

impl ApiError {
    pub fn respond(status: StatusCode, code: &str, message: impl Into<String>) -> Response {
        let body = axum::Json(serde_json::json!({
            "error": message.into(),
            "error_code": code,
        }));
        (status, body).into_response()
    }

    pub fn bad_request(code: &str, message: impl Into<String>) -> Response {
        Self::respond(StatusCode::BAD_REQUEST, code, message)
    }

    pub fn not_found(code: &str, message: impl Into<String>) -> Response {
        Self::respond(StatusCode::NOT_FOUND, code, message)
    }

    pub fn internal(code: &str, message: impl Into<String>) -> Response {
        Self::respond(StatusCode::INTERNAL_SERVER_ERROR, code, message)
    }

    pub const INTERNAL_ERROR: &str = "INTERNAL_ERROR";
    pub const NOT_FOUND: &str = "NOT_FOUND";
    pub const BAD_REQUEST: &str = "BAD_REQUEST";
}

// ---------------------------------------------------------------------------
// AiSearchBridge trait (implemented by server's AiSearchBridge)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct SemanticSearchResult {
    pub id: String,
    pub path: String,
    pub score: f32,
    pub metadata: serde_json::Value,
}

#[async_trait::async_trait]
pub trait AiSearchBridgeTrait: Send + Sync {
    fn is_available(&self) -> bool;
    fn semantic_search(
        &self,
        query: &str,
        limit: usize,
        min_similarity: Option<f32>,
    ) -> Result<Vec<SemanticSearchResult>, String>;
}

// ---------------------------------------------------------------------------
// ApiCoreState trait
// ---------------------------------------------------------------------------

/// Trait abstracting the AppState fields needed by api-core modules.
/// Implemented in ferro-server's lib.rs.
pub trait ApiCoreState: Clone + Send + Sync + 'static {
    // events
    fn ws_manager(&self) -> &Arc<ws::WsManager>;
    fn read_cache(&self) -> &Arc<ferro_server_integrations::read_cache::ReadCache>;
    fn webhooks(&self) -> &Arc<tokio::sync::RwLock<Vec<webhooks::WebhookConfig>>>;
    fn webhook_delivery_store(&self) -> &webhooks::WebhookDeliveryStore;
    fn email_config(&self) -> &email::EmailConfig;
    fn push_notification_store(
        &self,
    ) -> &Option<
        Arc<
            tokio::sync::RwLock<
                ferro_server_integrations::push_notifications::PushNotificationStore,
            >,
        >,
    >;
    fn push_notification_config(
        &self,
    ) -> &ferro_server_integrations::push_notifications::PushNotificationConfig;
    fn event_bus(&self) -> &Arc<ferro_event_bus::EventBus>;

    // event_triggers
    fn wasm_runtime(&self) -> &Option<Arc<ferro_core::wasm::WasmWorkerRuntime>>;
    fn workers_dir(&self) -> &Option<std::path::PathBuf>;
    fn wasm_dispatch_count(&self) -> &Arc<std::sync::atomic::AtomicU64>;
    fn wasm_error_count(&self) -> &Arc<std::sync::atomic::AtomicU64>;
    fn wasm_fuel_total(&self) -> &Arc<std::sync::atomic::AtomicU64>;

    // webhooks
    fn db(&self) -> &Option<DbHandle>;

    // search
    fn search(&self) -> &Option<Arc<tokio::sync::RwLock<ferro_core::search::SearchEngine>>>;
    fn search_ranking_config(
        &self,
    ) -> &Arc<tokio::sync::RwLock<ferro_core::search::SearchRankingConfig>>;
    fn ai_search(&self) -> &Option<Arc<dyn AiSearchBridgeTrait>>;
    fn lock_manager(&self) -> &Arc<dyn common::storage::LockManagerTrait>;
    fn preferences(&self) -> &Arc<dyn search::PreferenceStore>;
}

// ---------------------------------------------------------------------------
// Re-exports for intra-group access
// ---------------------------------------------------------------------------

pub use ferro_server_integrations::push_notifications;
pub use ferro_server_security::security::validate_url;
