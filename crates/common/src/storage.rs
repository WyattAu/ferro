use async_trait::async_trait;
use bytes::Bytes;
use crate::metadata::FileMetadata;
use crate::error::Result;

/// Core storage engine trait — all backends must implement this.
/// This is the single source of truth for storage operations used by
/// the server's WebDAV handler, API routes, and background workers.
#[async_trait]
pub trait StorageEngine: Send + Sync {
    /// Retrieve metadata for a file or collection.
    async fn head(&self, path: &str) -> Result<FileMetadata>;

    /// Read the raw bytes of a file.
    async fn get(&self, path: &str) -> Result<Bytes>;

    /// Write bytes to a path, returning the new metadata.
    async fn put(&self, path: &str, content: Bytes, owner: &str) -> Result<FileMetadata>;

    /// Delete a file or empty collection.
    async fn delete(&self, path: &str) -> Result<()>;

    /// List immediate children (depth-1) of a collection.
    async fn list(&self, path: &str) -> Result<Vec<FileMetadata>>;

    /// Copy a file or collection.
    async fn copy(&self, from: &str, to: &str) -> Result<()>;

    /// Move/rename a file or collection.
    async fn move_path(&self, from: &str, to: &str) -> Result<()>;

    /// Check if a path exists.
    async fn exists(&self, path: &str) -> Result<bool>;

    /// Create a collection (directory).
    async fn create_collection(&self, path: &str, owner: &str) -> Result<FileMetadata>;

    /// List all descendants recursively (used by PROPFIND depth:infinity).
    /// Implementations should apply a max_depth guard to prevent DoS.
    async fn list_all(&self, path: &str, max_depth: u32) -> Result<Vec<FileMetadata>>;
}
