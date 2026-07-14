use crate::error::Result;
use crate::metadata::FileMetadata;
use crate::webdav::{LockDepth, LockInfo, LockScope};
use async_trait::async_trait;
use bytes::Bytes;
use std::io::Cursor;
use std::pin::Pin;
use tokio::io::{AsyncRead, ReadBuf};

#[doc = "An async reader wrapping Bytes for streaming file content."]
pub struct StorageReader {
    inner: Pin<Box<dyn AsyncRead + Send>>,
}

impl StorageReader {
    /// Create a new storage reader wrapping an async read stream.
    #[must_use]
    pub fn new(inner: Pin<Box<dyn AsyncRead + Send>>) -> Self {
        Self { inner }
    }
}

impl AsyncRead for StorageReader {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        self.inner.as_mut().poll_read(cx, buf)
    }
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_storage_reader_from_bytes() {
        let data = Bytes::from("hello world");
        let reader = StorageReader::new(Box::pin(Cursor::new(data.clone())));
        let mut buf = vec![0u8; 64];

        use tokio::io::AsyncReadExt;
        let reader = &mut Box::pin(reader);
        let n = reader.read(&mut buf).await.unwrap();
        assert_eq!(n, 11);
        assert_eq!(&buf[..n], b"hello world");
    }

    #[tokio::test]
    async fn test_storage_reader_eof() {
        let data = Bytes::from("hi");
        let mut reader = StorageReader::new(Box::pin(Cursor::new(data)));
        let mut buf = vec![0u8; 64];

        use tokio::io::AsyncReadExt;
        let n1 = reader.read(&mut buf).await.unwrap();
        assert_eq!(n1, 2);

        let n2 = reader.read(&mut buf).await.unwrap();
        assert_eq!(n2, 0);
    }

    #[tokio::test]
    async fn test_storage_reader_large_buffer() {
        let data = Bytes::from("abc");
        let mut reader = StorageReader::new(Box::pin(Cursor::new(data)));
        let mut buf = vec![0u8; 2];

        use tokio::io::AsyncReadExt;
        let n1 = reader.read(&mut buf).await.unwrap();
        assert_eq!(n1, 2);
        assert_eq!(&buf[..n1], b"ab");

        let n2 = reader.read(&mut buf).await.unwrap();
        assert_eq!(n2, 1);
        assert_eq!(&buf[..n2], b"c");
    }

    #[tokio::test]
    async fn test_storage_reader_empty() {
        let data = Bytes::new();
        let mut reader = StorageReader::new(Box::pin(Cursor::new(data)));
        let mut buf = vec![0u8; 64];

        use tokio::io::AsyncReadExt;
        let n = reader.read(&mut buf).await.unwrap();
        assert_eq!(n, 0);
    }

    #[tokio::test]
    async fn test_storage_reader_large_data() {
        let data = Bytes::from(vec![42u8; 1024]);
        let mut reader = StorageReader::new(Box::pin(Cursor::new(data)));
        let mut buf = vec![0u8; 2048];

        use tokio::io::AsyncReadExt;
        let n = reader.read(&mut buf).await.unwrap();
        assert_eq!(n, 1024);
        assert!(buf[..1024].iter().all(|&b| b == 42));
    }

    #[tokio::test]
    async fn test_storage_reader_partial_read() {
        let data = Bytes::from("hello world");
        let mut reader = StorageReader::new(Box::pin(Cursor::new(data)));
        let mut buf = vec![0u8; 5];

        use tokio::io::AsyncReadExt;
        let n1 = reader.read(&mut buf).await.unwrap();
        assert_eq!(n1, 5);
        assert_eq!(&buf, b"hello");

        let n2 = reader.read(&mut buf).await.unwrap();
        assert_eq!(n2, 5);
        assert_eq!(&buf, b" worl");

        let n3 = reader.read(&mut buf).await.unwrap();
        assert_eq!(n3, 1);
        assert_eq!(buf[0], b'd');
    }
}

/// Trait for managing `WebDAV` locks across the server.
#[async_trait]
pub trait LockManagerTrait: Send + Sync {
    async fn check_lock(&self, path: &str) -> Option<LockInfo>;
    async fn check_lock_for_write(&self, path: &str) -> Result<()>;
    async fn acquire_lock(
        &self,
        path: &str,
        principal: &str,
        scope: LockScope,
        depth: LockDepth,
        timeout_secs: Option<u32>,
    ) -> Result<LockInfo>;
    async fn release_lock(&self, token: &str) -> Result<()>;
    async fn refresh_lock(&self, token: &str, timeout_secs: Option<u32>) -> Result<LockInfo>;
    async fn all_locks(&self) -> Vec<LockInfo>;
    async fn cleanup_all_expired(&self) {}
}

/// Core storage engine trait — all backends must implement this.
/// This is the single source of truth for storage operations used by
/// the server's `WebDAV` handler, API routes, and background workers.
#[async_trait]
pub trait StorageEngine: Send + Sync {
    /// Retrieve metadata for a file or collection.
    async fn head(&self, path: &str) -> Result<FileMetadata>;

    /// Read the raw bytes of a file.
    async fn get(&self, path: &str) -> Result<Bytes>;

    /// Stream a file's contents as an `AsyncRead` without loading the entire file into memory.
    /// Default implementation wraps the full `get()` result in a cursor.
    /// Backends should override this for true streaming (e.g., file I/O, S3 ranged GET).
    async fn get_stream(&self, path: &str) -> Result<StorageReader> {
        let data = self.get(path).await?;
        Ok(StorageReader::new(Box::pin(Cursor::new(data))))
    }

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
    /// Implementations should apply a `max_depth` guard to prevent `DoS`.
    async fn list_all(&self, path: &str, max_depth: u32) -> Result<Vec<FileMetadata>>;

    /// Upload a large file using multipart upload. Default: falls back to `put()`.
    /// Backends should override for efficient large file uploads.
    async fn put_multipart(&self, path: &str, content: Bytes, owner: &str) -> Result<FileMetadata> {
        self.put(path, content, owner).await
    }

    /// Stream a file upload from an `AsyncRead` without loading the entire file into memory.
    /// Default implementation reads all bytes and calls `put()`.
    /// Backends should override for true streaming (e.g., S3 multipart upload from reader).
    async fn put_stream(
        &self,
        path: &str,
        reader: Pin<Box<dyn tokio::io::AsyncRead + Send>>,
        size: u64,
        owner: &str,
    ) -> Result<FileMetadata> {
        use tokio::io::AsyncReadExt;
        let mut buf = Vec::with_capacity(size as usize);
        reader
            .take(size)
            .read_to_end(&mut buf)
            .await
            .map_err(|e| crate::error::FerroError::StorageBackend(format!("Stream read error: {e}")))?;
        self.put(path, Bytes::from(buf), owner).await
    }

    /// Check if streaming upload is supported.
    fn supports_put_stream(&self) -> bool {
        false
    }

    /// Check if multipart upload is supported.
    fn supports_multipart(&self) -> bool {
        false
    }
}
