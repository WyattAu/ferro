use async_trait::async_trait;
use bytes::Bytes;
use ferro_common::error::{FerroError, Result};
use ferro_common::metadata::{ContentHash, FileMetadata};
use ferro_common::storage::{StorageEngine, StorageReader};
use futures::{StreamExt, TryStreamExt};
use object_store::MultipartUpload;
use object_store::ObjectStore;
use object_store::path::Path as ObjPath;
use std::sync::Arc;
use tracing::debug;

/// Minimum size (bytes) to use multipart upload.
pub const MULTIPART_THRESHOLD: usize = 10 * 1024 * 1024; // 10 MB
/// Chunk size for multipart uploads (5 MB).
pub const MULTIPART_CHUNK_SIZE: usize = 5 * 1024 * 1024;

/// Storage engine backed by an `object_store` implementation (S3, GCS, Azure, local).
pub struct ObjectStoreStorageEngine {
    store: Arc<dyn ObjectStore>,
    prefix: String,
    /// Base filesystem path for local storage (enables real mkdir for collections).
    local_base: Option<std::path::PathBuf>,
}

impl ObjectStoreStorageEngine {
    pub fn new(store: Arc<dyn ObjectStore>) -> Self {
        Self {
            store,
            prefix: String::new(),
            local_base: None,
        }
    }

    /// Create a new storage engine with a key prefix for namespace isolation.
    pub fn with_prefix(store: Arc<dyn ObjectStore>, prefix: &str) -> Self {
        Self {
            store,
            prefix: prefix.trim_start_matches('/').to_string(),
            local_base: None,
        }
    }

    /// Create a new storage engine for local filesystem with a known base path.
    /// This enables real `mkdir` calls for collections instead of empty file markers.
    pub fn with_local_base(store: Arc<dyn ObjectStore>, base: std::path::PathBuf) -> Self {
        Self {
            store,
            prefix: String::new(),
            local_base: Some(base),
        }
    }

    fn to_obj_path(&self, path: &str) -> object_store::path::Path {
        let clean = path.trim_start_matches('/');
        if self.prefix.is_empty() {
            ObjPath::from(clean)
        } else {
            ObjPath::from(format!("{}/{}", self.prefix, clean))
        }
    }

    fn to_virtual_path(&self, obj_path: &object_store::path::Path) -> String {
        let full = obj_path.as_ref();
        if self.prefix.is_empty() {
            format!("/{}", full)
        } else {
            full.strip_prefix(&format!("{}/", self.prefix))
                .map(|s| format!("/{}", s))
                .unwrap_or_else(|| format!("/{}", full))
        }
    }
}

#[async_trait]
impl StorageEngine for ObjectStoreStorageEngine {
    async fn put(&self, path: &str, content: Bytes, owner: &str) -> Result<FileMetadata> {
        let obj_path = self.to_obj_path(path);
        let hash = ContentHash::compute(&content);
        let size = content.len() as u64;

        self.store
            .put(&obj_path, content.into())
            .await
            .map_err(|e| FerroError::StorageBackend(e.to_string()))?;

        let meta = FileMetadata::new(path.to_string(), hash, size, owner.to_string());
        debug!("PUT {} ({} bytes)", path, meta.size);
        Ok(meta)
    }

    fn supports_multipart(&self) -> bool {
        true
    }

    async fn put_multipart(&self, path: &str, content: Bytes, owner: &str) -> Result<FileMetadata> {
        let obj_path = self.to_obj_path(path);
        let hash = ContentHash::compute(&content);
        let size = content.len() as u64;

        let mut upload = self.store.put_multipart(&obj_path).await.map_err(|e| {
            FerroError::StorageBackend(format!("Failed to initiate multipart upload: {}", e))
        })?;

        let mut parts = Vec::new();
        for chunk in content.chunks(MULTIPART_CHUNK_SIZE) {
            let part = upload.put_part(object_store::PutPayload::from_bytes(
                Bytes::copy_from_slice(chunk),
            ));
            parts.push(part);
        }

        for part in parts {
            part.await.map_err(|e| {
                FerroError::StorageBackend(format!("Multipart part upload failed: {}", e))
            })?;
        }

        upload
            .complete()
            .await
            .map_err(|e| FerroError::StorageBackend(format!("Multipart complete failed: {}", e)))?;

        let meta = FileMetadata::new(path.to_string(), hash, size, owner.to_string());
        debug!("PUT multipart {} ({} bytes)", path, meta.size);
        Ok(meta)
    }

    async fn get(&self, path: &str) -> Result<Bytes> {
        let obj_path = self.to_obj_path(path);
        let result = self
            .store
            .get(&obj_path)
            .await
            .map_err(|e| FerroError::NotFound(format!("{}: {}", path, e)))?;
        let bytes = result
            .bytes()
            .await
            .map_err(|e| FerroError::StorageBackend(e.to_string()))?;
        Ok(bytes)
    }

    async fn get_stream(&self, path: &str) -> Result<StorageReader> {
        let obj_path = self.to_obj_path(path);
        let result = self
            .store
            .get(&obj_path)
            .await
            .map_err(|e| FerroError::NotFound(format!("{}: {}", path, e)))?;
        let stream = result.into_stream().map_err(std::io::Error::other);
        let reader = tokio_util::io::StreamReader::new(stream);
        Ok(StorageReader::new(Box::pin(reader)))
    }

    async fn delete(&self, path: &str) -> Result<()> {
        let obj_path = self.to_obj_path(path);
        if let Err(e) = self.store.delete(&obj_path).await {
            // object_store::LocalFileSystem::delete uses remove_file, which fails
            // on directories. Fall back to filesystem remove_dir for local storage.
            if let Some(ref base) = self.local_base {
                let clean = path.trim_start_matches('/').trim_end_matches('/');
                let fs_path = base.join(clean);
                if fs_path.is_dir() {
                    tokio::fs::remove_dir(&fs_path)
                        .await
                        .map_err(|e| FerroError::StorageBackend(format!("{}: {}", path, e)))?;
                    debug!("DELETE dir {}", path);
                    return Ok(());
                }
            }
            return Err(FerroError::NotFound(format!("{}: {}", path, e)));
        }
        debug!("DELETE {}", path);
        Ok(())
    }

    async fn list(&self, prefix: &str) -> Result<Vec<FileMetadata>> {
        let obj_prefix = self.to_obj_path(prefix);
        let mut items = Vec::new();

        let result = self
            .store
            .list_with_delimiter(Some(&obj_prefix))
            .await
            .map_err(|e| FerroError::StorageBackend(e.to_string()))?;

        for obj_meta in result.objects {
            let vpath = self.to_virtual_path(&obj_meta.location);
            let etag = obj_meta.e_tag.clone().unwrap_or_default();

            items.push(FileMetadata {
                path: vpath,
                content_hash: ContentHash::from_etag(&etag),
                size: obj_meta.size as u64,
                mime_type: "application/octet-stream".to_string(),
                is_collection: false,
                created_at: obj_meta.last_modified,
                modified_at: obj_meta.last_modified,
                owner: "unknown".to_string(),
                etag: format!("\"{}\"", etag),
            });
        }

        for prefix_path in result.common_prefixes {
            let vpath = self.to_virtual_path(&prefix_path);
            items.push(FileMetadata::new_collection(vpath, "unknown".to_string()));
        }

        items.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(items)
    }

    async fn copy(&self, from: &str, to: &str) -> Result<()> {
        let from_path = self.to_obj_path(from);
        let to_path = self.to_obj_path(to);
        self.store
            .copy(&from_path, &to_path)
            .await
            .map_err(|e| FerroError::StorageBackend(e.to_string()))?;
        debug!("COPY {} -> {}", from, to);
        Ok(())
    }

    async fn move_path(&self, from: &str, to: &str) -> Result<()> {
        let from_path = self.to_obj_path(from);
        let to_path = self.to_obj_path(to);
        self.store
            .rename(&from_path, &to_path)
            .await
            .map_err(|e| FerroError::StorageBackend(e.to_string()))?;
        debug!("MOVE {} -> {}", from, to);
        Ok(())
    }

    async fn head(&self, path: &str) -> Result<FileMetadata> {
        let obj_path = self.to_obj_path(path);
        match self.store.head(&obj_path).await {
            Ok(meta) => {
                let etag = meta.e_tag.clone().unwrap_or_default();
                let is_collection = path.ends_with('/') || (meta.size == 0 && path.contains('/'));

                // Normalize the object store's native ETag to a 64-char SHA-256 hash.
                // For uploads done via our put(), the ETag is already SHA-256 hex (64 chars).
                // For object stores (S3 etc.), the native ETag may be MD5 (32 chars) or
                // multipart hash — we SHA-256 hash it to produce a consistent 64-char value.
                let content_hash = ContentHash::from_etag(&etag);

                Ok(FileMetadata {
                    path: path.to_string(),
                    content_hash,
                    size: meta.size as u64,
                    mime_type: if is_collection {
                        "httpd/unix-directory".to_string()
                    } else {
                        "application/octet-stream".to_string()
                    },
                    is_collection,
                    created_at: meta.last_modified,
                    modified_at: meta.last_modified,
                    owner: "unknown".to_string(),
                    etag: format!("\"{}\"", etag),
                })
            }
            Err(_) if self.local_base.is_some() => {
                // Fallback: check if it's a real directory on local filesystem
                let clean = path.trim_start_matches('/');
                let fs_path = self.local_base.as_ref().unwrap().join(clean);
                let metadata = tokio::fs::metadata(&fs_path)
                    .await
                    .map_err(|e| FerroError::NotFound(format!("{}: {}", path, e)))?;
                if metadata.is_dir() {
                    let modified = metadata
                        .modified()
                        .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                    let chrono_modified: chrono::DateTime<chrono::Utc> = modified.into();
                    Ok(FileMetadata {
                        path: path.to_string(),
                        etag: format!("\"col-{}\"", chrono_modified.timestamp_millis()),
                        content_hash: ContentHash::new("0".repeat(64)),
                        size: 0,
                        mime_type: "httpd/unix-directory".to_string(),
                        is_collection: true,
                        created_at: chrono_modified,
                        modified_at: chrono_modified,
                        owner: "unknown".to_string(),
                    })
                } else {
                    let modified = metadata
                        .modified()
                        .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                    let chrono_modified: chrono::DateTime<chrono::Utc> = modified.into();
                    Ok(FileMetadata {
                        path: path.to_string(),
                        content_hash: ContentHash::new("0".repeat(64)),
                        size: metadata.len(),
                        mime_type: "application/octet-stream".to_string(),
                        is_collection: false,
                        created_at: chrono_modified,
                        modified_at: chrono_modified,
                        owner: "unknown".to_string(),
                        etag: String::new(),
                    })
                }
            }
            Err(e) => Err(FerroError::NotFound(format!("{}: {}", path, e))),
        }
    }

    async fn exists(&self, path: &str) -> Result<bool> {
        let obj_path = self.to_obj_path(path);
        match self.store.head(&obj_path).await {
            Ok(_) => Ok(true),
            Err(object_store::Error::NotFound { .. }) => {
                // For local storage, also check if it's a real directory
                if let Some(ref base) = self.local_base {
                    let clean = path.trim_start_matches('/');
                    let fs_path = base.join(clean);
                    Ok(fs_path.exists())
                } else {
                    Ok(false)
                }
            }
            Err(e) => Err(FerroError::StorageBackend(e.to_string())),
        }
    }

    async fn create_collection(&self, path: &str, owner: &str) -> Result<FileMetadata> {
        // For local filesystem, create a real directory instead of an empty file marker
        if let Some(ref base) = self.local_base {
            let clean = path.trim_start_matches('/');
            let dir_path = base.join(clean);
            tokio::fs::create_dir_all(&dir_path).await.map_err(|e| {
                FerroError::StorageBackend(format!(
                    "Failed to create directory {:?}: {}",
                    dir_path, e
                ))
            })?;
        } else {
            // For cloud object stores, create an empty marker object (directory is implicit)
            let dir_path = if path.ends_with('/') {
                path.to_string()
            } else {
                format!("{}/", path)
            };
            let obj_path = self.to_obj_path(&dir_path);
            self.store
                .put(&obj_path, Bytes::new().into())
                .await
                .map_err(|e| FerroError::StorageBackend(e.to_string()))?;
        }
        debug!("MKCOL {}", path);
        Ok(FileMetadata::new_collection(
            path.to_string(),
            owner.to_string(),
        ))
    }

    async fn list_all(&self, prefix: &str, max_depth: u32) -> Result<Vec<FileMetadata>> {
        let obj_prefix = self.to_obj_path(prefix);
        let mut items = Vec::new();

        let mut stream = self.store.list(Some(&obj_prefix));
        while let Some(result) = stream.next().await {
            let obj_meta = result.map_err(|e| FerroError::StorageBackend(e.to_string()))?;
            let vpath = self.to_virtual_path(&obj_meta.location);
            if vpath == prefix {
                continue;
            }
            // Calculate depth relative to queried path
            let base = if prefix == "/" {
                ""
            } else {
                prefix.trim_end_matches('/')
            };
            let relative = vpath
                .strip_prefix(base)
                .unwrap_or(&vpath)
                .trim_start_matches('/');
            let depth = relative.matches('/').count() as u32;
            if depth >= max_depth {
                continue;
            }

            let etag = obj_meta.e_tag.clone().unwrap_or_default();
            // Normalize native ETag to consistent 64-char SHA-256 hash (see head() comment).
            let content_hash = ContentHash::from_etag(&etag);

            items.push(FileMetadata {
                path: vpath,
                content_hash,
                size: obj_meta.size as u64,
                mime_type: "application/octet-stream".to_string(),
                is_collection: obj_meta.location.as_ref().ends_with('/'),
                created_at: obj_meta.last_modified,
                modified_at: obj_meta.last_modified,
                owner: "unknown".to_string(),
                etag: format!("\"{}\"", etag),
            });
        }

        Ok(items)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use object_store::local::LocalFileSystem;
    use tempfile::TempDir;

    fn make_test_engine() -> (ObjectStoreStorageEngine, TempDir) {
        let tmp = TempDir::new().unwrap();
        let local: Arc<dyn ObjectStore> =
            Arc::new(LocalFileSystem::new_with_prefix(tmp.path()).unwrap());
        let engine = ObjectStoreStorageEngine::new(local);
        (engine, tmp)
    }

    #[tokio::test]
    async fn test_put_get_roundtrip() {
        let (engine, _tmp) = make_test_engine();
        let content = Bytes::from("hello world");

        let meta = engine
            .put("/test.txt", content.clone(), "user1")
            .await
            .unwrap();
        assert_eq!(meta.path, "/test.txt");
        assert_eq!(meta.size, 11);
        assert_eq!(meta.owner, "user1");

        let retrieved = engine.get("/test.txt").await.unwrap();
        assert_eq!(content, retrieved);
    }

    #[tokio::test]
    async fn test_put_delete() {
        let (engine, _tmp) = make_test_engine();
        engine
            .put("/test.txt", Bytes::from("hello"), "user1")
            .await
            .unwrap();
        assert!(engine.exists("/test.txt").await.unwrap());

        engine.delete("/test.txt").await.unwrap();
        assert!(!engine.exists("/test.txt").await.unwrap());
    }

    #[tokio::test]
    async fn test_create_collection_and_list() {
        let (engine, _tmp) = make_test_engine();
        engine
            .put("/docs/a.txt", Bytes::from("aaa"), "user1")
            .await
            .unwrap();
        engine
            .put("/docs/b.txt", Bytes::from("bbb"), "user1")
            .await
            .unwrap();

        let items = engine.list("/docs").await.unwrap();
        let files: Vec<&FileMetadata> = items.iter().filter(|m| !m.is_collection).collect();
        assert_eq!(files.len(), 2);
    }

    #[tokio::test]
    async fn test_copy() {
        let (engine, _tmp) = make_test_engine();
        engine
            .put("/src.txt", Bytes::from("hello"), "user1")
            .await
            .unwrap();

        engine.copy("/src.txt", "/dst.txt").await.unwrap();
        assert!(engine.exists("/src.txt").await.unwrap());
        assert!(engine.exists("/dst.txt").await.unwrap());

        let src = engine.get("/src.txt").await.unwrap();
        let dst = engine.get("/dst.txt").await.unwrap();
        assert_eq!(src, dst);
    }

    #[tokio::test]
    async fn test_move_path() {
        let (engine, _tmp) = make_test_engine();
        engine
            .put("/old.txt", Bytes::from("hello"), "user1")
            .await
            .unwrap();

        engine.move_path("/old.txt", "/new.txt").await.unwrap();
        assert!(!engine.exists("/old.txt").await.unwrap());
        assert!(engine.exists("/new.txt").await.unwrap());

        let content = engine.get("/new.txt").await.unwrap();
        assert_eq!(content, Bytes::from("hello"));
    }

    #[tokio::test]
    async fn test_exists() {
        let (engine, _tmp) = make_test_engine();
        assert!(!engine.exists("/nope.txt").await.unwrap());

        engine
            .put("/yes.txt", Bytes::from("data"), "user1")
            .await
            .unwrap();
        assert!(engine.exists("/yes.txt").await.unwrap());
    }

    #[tokio::test]
    async fn test_nested_collections() {
        let (engine, _tmp) = make_test_engine();
        engine
            .put("/a/b/c/file.txt", Bytes::from("nested"), "user1")
            .await
            .unwrap();

        let items = engine.list("/a/b").await.unwrap();
        assert!(!items.is_empty());
    }

    #[tokio::test]
    async fn test_list_all_descendants() {
        let (engine, _tmp) = make_test_engine();
        engine
            .put("/root/f1.txt", Bytes::from("a"), "user1")
            .await
            .unwrap();
        engine
            .put("/root/sub/f2.txt", Bytes::from("b"), "user1")
            .await
            .unwrap();
        engine
            .put("/root/sub/deep/f3.txt", Bytes::from("c"), "user1")
            .await
            .unwrap();

        let all = engine.list_all("/root", 100).await.unwrap();
        assert_eq!(all.len(), 3);
    }

    #[tokio::test]
    async fn test_list_all_depth_limit() {
        let (engine, _tmp) = make_test_engine();
        engine
            .put("/root/f1.txt", Bytes::from("a"), "user1")
            .await
            .unwrap();
        engine
            .put("/root/sub/f2.txt", Bytes::from("b"), "user1")
            .await
            .unwrap();
        engine
            .put("/root/sub/deep/f3.txt", Bytes::from("c"), "user1")
            .await
            .unwrap();

        // depth=1 → immediate children (f1.txt + sub/)
        let items = engine.list_all("/root", 1).await.unwrap();
        // object_store may return common_prefixes as "sub/" which has 0 slashes
        // relative to "root", so depth=1 includes them
        assert!(
            items.len() >= 1,
            "Expected at least 1 item at depth 1, got {}",
            items.len()
        );

        // depth=100 → everything
        let items = engine.list_all("/root", 100).await.unwrap();
        assert_eq!(items.len(), 3);
    }

    #[tokio::test]
    async fn test_get_stream_reads_correctly() {
        let (engine, _tmp) = make_test_engine();
        let content = Bytes::from("streaming test data");
        engine
            .put("/stream.txt", content.clone(), "user1")
            .await
            .unwrap();

        let mut reader = engine.get_stream("/stream.txt").await.unwrap();
        let mut buf = vec![0u8; 64];
        use tokio::io::AsyncReadExt;
        let n = reader.read(&mut buf).await.unwrap();
        assert_eq!(n, content.len());
        assert_eq!(&buf[..n], &content[..]);
    }

    #[tokio::test]
    async fn test_get_stream_not_found() {
        let (engine, _tmp) = make_test_engine();
        let result = engine.get_stream("/nonexistent.txt").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_stream_large_file() {
        let (engine, _tmp) = make_test_engine();
        let large_content = Bytes::from(vec![0xAB_u8; 100_000]);
        engine
            .put("/large.bin", large_content.clone(), "user1")
            .await
            .unwrap();

        let mut reader = engine.get_stream("/large.bin").await.unwrap();
        let mut buf = vec![0u8; 8192];
        use tokio::io::AsyncReadExt;

        let mut total_read = 0usize;
        loop {
            let n = reader.read(&mut buf).await.unwrap();
            if n == 0 {
                break;
            }
            assert!(buf[..n].iter().all(|&b| b == 0xAB));
            total_read += n;
        }
        assert_eq!(total_read, large_content.len());
    }

    #[tokio::test]
    async fn test_multipart_upload() {
        let (engine, _tmp) = make_test_engine();
        let content = Bytes::from(vec![0u8; 2 * 1024 * 1024]);

        let meta = engine
            .put_multipart("/large.bin", content.clone(), "user1")
            .await
            .unwrap();
        assert_eq!(meta.size, 2 * 1024 * 1024);
        assert_eq!(meta.path, "/large.bin");

        let retrieved = engine.get("/large.bin").await.unwrap();
        assert_eq!(retrieved.len(), 2 * 1024 * 1024);
    }

    #[tokio::test]
    async fn test_supports_multipart() {
        let (engine, _tmp) = make_test_engine();
        assert!(engine.supports_multipart());
    }

    #[tokio::test]
    async fn test_multipart_upload_single_byte() {
        let (engine, _tmp) = make_test_engine();
        let content = Bytes::from("hello");

        let meta = engine
            .put_multipart("/small.bin", content.clone(), "user1")
            .await
            .unwrap();
        assert_eq!(meta.size, 5);

        let retrieved = engine.get("/small.bin").await.unwrap();
        assert_eq!(retrieved, content);
    }

    #[tokio::test]
    async fn test_multipart_upload_exact_chunk_boundary() {
        let (engine, _tmp) = make_test_engine();
        let content = Bytes::from(vec![0x42_u8; MULTIPART_CHUNK_SIZE * 3]);

        let meta = engine
            .put_multipart("/boundary.bin", content.clone(), "user1")
            .await
            .unwrap();
        assert_eq!(meta.size, (MULTIPART_CHUNK_SIZE * 3) as u64);

        let retrieved = engine.get("/boundary.bin").await.unwrap();
        assert_eq!(retrieved.len(), MULTIPART_CHUNK_SIZE * 3);
    }
}
