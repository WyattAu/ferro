pub mod api;
pub mod batch;
pub mod dedup;
pub mod quota;
pub mod range_get;
pub mod snapshots;
pub mod storage_health;
pub mod streaming;
pub mod streaming_upload;
pub mod thumbnail_cache;
pub mod thumbnails;

use common::storage::StorageEngine;
use std::sync::Arc;

pub trait ThumbnailCacheTrait: Send + Sync {
    fn get(&self, path: &str) -> Option<(Vec<u8>, String)>;
    fn put(&self, path: &str, data: Vec<u8>, mime: &str);
    fn invalidate(&self, path: &str);
}

pub trait StorageUtilsState: Clone + Send + Sync + 'static {
    fn storage(&self) -> &Arc<dyn StorageEngine>;
    fn data_dir(&self) -> Option<&str>;
    fn thumbnail_cache(&self) -> &Arc<dyn ThumbnailCacheTrait>;
    fn thumbnail_size(&self) -> u32;
    fn snapshot_store(&self) -> &Arc<snapshots::SnapshotStore>;
    fn storage_health(&self) -> &Arc<storage_health::StorageHealthMonitor>;
}
