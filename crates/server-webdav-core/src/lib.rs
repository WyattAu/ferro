pub mod dav;
pub mod lock;
pub mod move_copy;
pub mod trash;
pub mod webdav;

use std::sync::Arc;
use std::sync::atomic::AtomicU64;

use common::metadata::FileMetadata;
use dashmap::DashSet;

// ---------------------------------------------------------------------------
// WebDAV event types (local copies to avoid depending on server-api-core)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebdavEventType {
    FileUploaded,
    FileModified,
    FileDeleted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebdavOpType {
    Create,
    Update,
    Delete,
    Rename,
}

#[derive(Debug, Clone)]
pub struct WebdavFileEvent {
    pub op_type: &'static str,
    pub path: String,
    pub new_path: Option<String>,
    pub size: Option<u64>,
    pub mime_type: Option<String>,
    pub owner: String,
    pub etag: Option<String>,
    pub already_existed: bool,
}

// ---------------------------------------------------------------------------
// DbHandle type alias (matches ferro-server and ferro-server-api-core)
// ---------------------------------------------------------------------------

pub type DbHandle = Arc<std::sync::Mutex<rusqlite::Connection>>;

// ---------------------------------------------------------------------------
// Sub-traits for composable state access
// ---------------------------------------------------------------------------

/// WASM worker runtime access.
pub trait HasWasm: Send + Sync {
    fn wasm_runtime(&self) -> Option<&Arc<ferro_core::wasm::WasmWorkerRuntime>>;
    fn wasm_dispatch_count(&self) -> &Arc<AtomicU64>;
    fn wasm_error_count(&self) -> &Arc<AtomicU64>;
    fn wasm_fuel_total(&self) -> &Arc<AtomicU64>;
    fn recently_processed(&self) -> &DashSet<String>;
}

/// Sync clock and operation recording.
pub trait HasSyncOps: Send + Sync {
    fn sync_clock(&self) -> &Arc<AtomicU64>;
    fn record_sync_op(
        &self,
        op_type: WebdavOpType,
        path: &str,
        new_path: Option<&str>,
        size: u64,
        mime_type: Option<&str>,
        owner: &str,
        checksum: &str,
    );
    fn bump_sync_clock(&self);
}

/// Offline-first support (connection monitor, cache, queue).
pub trait HasOffline: Send + Sync {
    fn is_online(&self) -> bool;
    fn offline_cache(&self) -> &Arc<tokio::sync::RwLock<ferro_offline::cache::ContentCache>>;
    fn offline_queue(&self) -> &Option<Arc<ferro_offline::change_queue::SqliteChangeQueue>>;
}

/// Event dispatch, indexing, and trigger firing.
#[async_trait::async_trait]
pub trait HasEventDispatch: Send + Sync {
    async fn dispatch_file_event(&self, event: WebdavFileEvent);
    async fn fire_event_triggers(&self, event_type: WebdavEventType, path: &str, owner: &str);
    async fn index_file_with_content(&self, metadata: &FileMetadata, content: &[u8]);
    async fn remove_file_from_index(&self, path: &str);
}

/// CAS, metadata stores, thumbnail cache, WORM policies, quota enforcement.
pub trait HasWebDavStores: Send + Sync {
    fn cas_store(&self) -> Option<&Arc<dyn ferro_core::cas::CasStore>>;
    fn metadata_store(&self) -> Option<&Arc<dyn ferro_core::metadata::MetadataStore>>;
    fn thumbnail_cache_invalidate(&self, path: &str);
    fn load_worm_policies(&self) -> Vec<ferro_server_compliance::worm::WormPolicy>;
    fn is_worm_protected(&self, path: &str) -> bool;
    fn enforce_quota(
        &self,
        content_length: u64,
    ) -> impl std::future::Future<Output = Result<(), axum::response::Response>> + Send;
    fn calendar_store(&self) -> &Arc<dyn ferro_dav::store::CalendarStore>;
    fn address_book_store(&self) -> &Arc<dyn ferro_dav::store::AddressBookStore>;
}

/// Trash store access (for trash operations).
pub trait HasTrashStore: Send + Sync {
    fn trash_store(&self) -> &crate::trash::TrashStore;
}

/// Composite trait that webdav-core handlers are generic over.
///
/// Implements all sub-traits + common::server_context traits for full access.
pub trait WebDavCoreState:
    Clone
    + Send
    + Sync
    + 'static
    + common::server_context::HasStorage
    + common::server_context::HasLockManager
    + common::server_context::HasBodyLimits
    + common::server_context::HasDataDir
    + common::server_context::HasStreamingConfig
    + common::server_context::HasTrash
    + HasWasm
    + HasSyncOps
    + HasOffline
    + HasEventDispatch
    + HasWebDavStores
    + HasTrashStore
{
    fn admin_user(&self) -> Option<&str>;
}
