pub mod handler;
pub mod lock;
pub mod move_copy;
pub mod xml_util;

pub use handler::handle_any;
pub use handler::sanitize_path;
pub use lock::DbHandle;
pub use lock::LockManager;

/// Re-export range_get types from storage-ops as a module alias.
pub mod range_get {
    pub use ferro_server_storage_ops::range_get::*;
}

pub use range_get::*;
pub use xml_util::*;

use axum::http::Method;
use axum::response::Response;
use bytes::Bytes;
use common::storage::LockManagerTrait;
use common::storage::StorageEngine;
use std::sync::Arc;

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

#[async_trait::async_trait]
pub trait WebdavAppState: Clone + Send + Sync + 'static {
    fn storage(&self) -> &Arc<dyn StorageEngine>;
    fn lock_manager(&self) -> &Arc<dyn LockManagerTrait>;

    fn max_body_size(&self) -> u64;
    fn max_file_versions(&self) -> u64;
    fn data_dir(&self) -> Option<String>;
    fn admin_user(&self) -> Option<String>;

    fn is_online(&self) -> bool;
    fn offline_cache(&self) -> &Arc<tokio::sync::RwLock<ferro_offline::cache::ContentCache>>;
    fn offline_queue(&self) -> &Option<Arc<ferro_offline::change_queue::SqliteChangeQueue>>;

    fn sync_clock(&self) -> &Arc<std::sync::atomic::AtomicU64>;
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

    fn cas_store(&self) -> Option<&Arc<dyn ferro_core::cas::CasStore>>;
    fn metadata_store(&self) -> Option<&Arc<dyn ferro_core::metadata::MetadataStore>>;

    fn enforce_quota(&self, content_len: u64) -> Option<Response>;
    fn is_worm_protected(&self, path: &str) -> bool;
    fn verify_content_type(&self, declared: &str, body: &[u8]) -> Option<String>;

    fn dispatch_wasm_workers(&self, path: &str);
    fn thumbnail_cache_invalidate(&self, path: &str);

    async fn dispatch_post_op(&self, event: WebdavFileEvent);
    async fn fire_event_triggers(&self, event_type: WebdavEventType, path: &str, owner: &str);
    fn index_file_with_content(&self, meta: &common::metadata::FileMetadata, content: &[u8]);
    fn remove_file_from_index(&self, path: &str);

    async fn dispatch_caldav(&self, method: &Method, path: &str, body: Bytes) -> Response;
    async fn dispatch_carddav(&self, method: &Method, path: &str, body: Bytes) -> Response;
}
