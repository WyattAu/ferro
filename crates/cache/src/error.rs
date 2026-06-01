use thiserror::Error;

#[derive(Debug, Error)]
pub enum CacheError {
    #[error("cache capacity exceeded (entries: {entries}, max: {max_entries})")]
    CapacityExceeded { entries: usize, max_entries: usize },

    #[error("cache size exceeded ({size_bytes} bytes, max: {max_size_bytes} bytes)")]
    SizeExceeded {
        size_bytes: u64,
        max_size_bytes: u64,
    },

    #[error("serialization failed: {0}")]
    SerializationFailed(String),

    #[error("cache entry not found")]
    NotFound,
}
