pub mod traits;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64};

use common::storage::{LockManagerTrait, StorageEngine};

// ---------------------------------------------------------------------------
// AuditLogTrait — minimal trait for audit logging without depending on ferro-server
// ---------------------------------------------------------------------------

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

/// Minimal audit log trait that handlers depend on.
#[async_trait::async_trait]
#[allow(clippy::len_without_is_empty)]
pub trait AuditLogTrait: Send + Sync {
    async fn log(&self, entry: AuditEntry);
    async fn len(&self) -> usize;
    async fn recent(&self, n: usize) -> Vec<AuditEntry>;
    async fn recent_with_offset(&self, limit: usize, offset: usize) -> Vec<AuditEntry>;
}

// ---------------------------------------------------------------------------
// ServerState trait
// ---------------------------------------------------------------------------

/// Consolidated trait capturing the interface handlers actually use from AppState.
///
/// This trait allows extracted crates to depend on `ferro-server-state` instead of
/// `ferro-server`, breaking circular dependencies. Handler functions can be generic
/// over this trait:
///
/// ```ignore
/// async fn my_handler<S: ServerState>(State(state): State<S>) -> impl IntoResponse {
///     let storage = state.storage();
///     // ...
/// }
/// ```
///
/// `AppState` implements this trait, so existing code continues to work unchanged.
pub trait ServerState: Send + Sync + Clone + 'static {
    // -- Core storage --------------------------------------------------------

    fn storage(&self) -> &Arc<dyn StorageEngine>;
    fn lock_manager(&self) -> &Arc<dyn LockManagerTrait>;
    fn db(&self) -> &Option<common::DbHandle>;

    // -- Authentication ------------------------------------------------------

    fn admin_user(&self) -> Option<&str>;
    fn admin_password(&self) -> Option<&str>;
    fn admin_password_rotated(&self) -> &Arc<AtomicBool>;
    fn user_store(&self) -> &Arc<dyn ferro_auth::users::UserStoreTrait>;
    fn api_key_store(&self) -> &Arc<dyn ferro_auth::api_keys::ApiKeyStoreTrait>;

    // -- Search --------------------------------------------------------------

    fn search(&self) -> &Option<Arc<tokio::sync::RwLock<ferro_core::search::SearchEngine>>>;
    fn preferences(&self) -> &Arc<dyn ferro_server_api_core::search::PreferenceStore>;

    // -- Sharing -------------------------------------------------------------

    fn share_store(&self) -> &Arc<dyn ferro_server_sharing::shares::ShareStoreTrait>;
    fn favorites(&self) -> &Arc<dyn ferro_server_sharing::favorites::FavoriteStore>;

    // -- Collaboration -------------------------------------------------------

    fn tags(&self) -> &Arc<ferro_server_collaboration::tags::TagStore>;
    fn comments(&self) -> &Arc<ferro_server_collaboration::comments::CommentStore>;

    // -- Compliance ----------------------------------------------------------

    fn worm_store(&self) -> &ferro_server_compliance::worm::WormPolicyStore;
    fn retention_store(&self) -> &ferro_server_compliance::retention::RetentionStore;
    fn dlp_store(&self) -> &ferro_server_compliance::dlp_api::DlpStore;

    // -- Operations ----------------------------------------------------------

    fn snapshot_store(&self) -> &Arc<ferro_server_storage_ops::snapshots::SnapshotStore>;
    fn thumbnail_cache(&self) -> &Arc<dyn ferro_server_storage_ops::ThumbnailCacheTrait>;
    fn storage_health(&self) -> &Arc<ferro_server_storage_ops::storage_health::StorageHealthMonitor>;

    // -- Configuration -------------------------------------------------------

    fn external_url(&self) -> &str;
    fn max_body_size(&self) -> u64;
    fn thumbnail_size(&self) -> u32;
    fn data_dir(&self) -> Option<&str>;
    fn max_file_versions(&self) -> u64;
    fn quota_bytes(&self) -> Option<u64>;

    // -- Metrics -------------------------------------------------------------

    fn request_count(&self) -> &Arc<AtomicU64>;
    fn storage_op_counts(&self) -> &Arc<[AtomicU64; 6]>;
    fn maintenance_mode(&self) -> &Arc<AtomicBool>;
    fn startup_complete(&self) -> &Arc<AtomicBool>;

    // -- Audit ---------------------------------------------------------------

    fn audit_log(&self) -> &Arc<dyn AuditLogTrait>;

    // -- Features ------------------------------------------------------------

    fn wasm_runtime(&self) -> &Option<Arc<ferro_core::wasm::WasmWorkerRuntime>>;
    fn search_ranking_config(&self) -> &Arc<tokio::sync::RwLock<ferro_core::search::SearchRankingConfig>>;
}
