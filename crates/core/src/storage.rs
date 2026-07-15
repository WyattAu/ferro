pub use ferro_common::storage::StorageEngine;

use async_trait::async_trait;
use bytes::Bytes;
use chrono::Utc;
use dashmap::DashMap;
use ferro_common::error::{FerroError, Result};
use ferro_common::metadata::{ContentHash, FileMetadata};
use ferro_common::path::normalize_path;
use tracing::debug;

/// In-memory storage engine that combines content and metadata storage.
///
/// Implements the [`StorageEngine`] trait with path normalization, immediate-child
/// listing, and depth-limited recursive listing. Suitable for tests, benchmarks,
/// and single-instance servers.
#[derive(Debug, Clone)]
pub struct InMemoryStorageEngine {
    store: DashMap<String, Bytes>,
    metadata: DashMap<String, FileMetadata>,
}

impl InMemoryStorageEngine {
    /// Create a new empty in-memory storage engine.
    #[must_use]
    pub fn new() -> Self {
        Self {
            store: DashMap::new(),
            metadata: DashMap::new(),
        }
    }
}

impl Default for InMemoryStorageEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl StorageEngine for InMemoryStorageEngine {
    async fn put(&self, path: &str, content: Bytes, owner: &str) -> Result<FileMetadata> {
        let path = normalize_path(path).into_owned();
        let hash = ContentHash::compute(&content);
        let meta = FileMetadata::new(path.clone(), hash.clone(), content.len() as u64, owner.to_string());

        self.store.insert(path.clone(), content);
        self.metadata.insert(path.clone(), meta.clone());

        debug!("PUT {} ({} bytes, hash={})", path, meta.size, hash.as_str());
        Ok(meta)
    }

    async fn get(&self, path: &str) -> Result<Bytes> {
        let path = normalize_path(path).into_owned();
        self.store.get(&path).map(|d| d.value().clone()).ok_or_else(|| {
            #[cold]
            fn not_found(p: String) -> FerroError {
                FerroError::NotFound(p)
            }
            not_found(path)
        })
    }

    async fn delete(&self, path: &str) -> Result<()> {
        let path = normalize_path(path).into_owned();

        if !self.metadata.contains_key(&path) {
            #[cold]
            fn not_found(p: String) -> FerroError {
                FerroError::NotFound(p)
            }
            return Err(not_found(path));
        }

        self.store.remove(&path);
        self.metadata.remove(&path);

        debug!("DELETE {}", path);
        Ok(())
    }

    async fn list(&self, path: &str) -> Result<Vec<FileMetadata>> {
        let path = normalize_path(path);
        let prefix = if path == "/" {
            "/".to_string()
        } else {
            format!("{}/", path.trim_end_matches('/'))
        };

        let mut items: Vec<FileMetadata> = self
            .metadata
            .iter()
            .filter(|m| {
                if !m.path.starts_with(&prefix) || m.path == path.as_ref() {
                    return false;
                }
                let remaining = &m.path[prefix.len()..];
                !remaining.contains('/')
            })
            .map(|m| m.value().clone())
            .collect();

        items.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(items)
    }

    async fn copy(&self, src: &str, dst: &str) -> Result<()> {
        let src = normalize_path(src).into_owned();
        let dst = normalize_path(dst).into_owned();

        let content = match self.store.get(&src) {
            Some(c) => c.value().clone(),
            None => {
                #[cold]
                fn not_found(p: String) -> FerroError {
                    FerroError::NotFound(p)
                }
                return Err(not_found(src));
            }
        };
        let mut meta = match self.metadata.get(&src) {
            Some(m) => m.value().clone(),
            None => {
                #[cold]
                fn not_found(p: String) -> FerroError {
                    FerroError::NotFound(p)
                }
                return Err(not_found(src));
            }
        };

        meta.path = dst.clone();
        meta.etag = format!("\"{}\"", meta.content_hash.as_str());
        meta.modified_at = Utc::now();

        debug!("COPY {} -> {}", src, dst);
        self.store.insert(dst.clone(), content);
        self.metadata.insert(dst, meta);

        Ok(())
    }

    async fn move_path(&self, src: &str, dst: &str) -> Result<()> {
        let src = normalize_path(src).into_owned();
        let dst = normalize_path(dst).into_owned();

        let content = self
            .store
            .remove(&src)
            .ok_or_else(|| {
                #[cold]
                fn not_found(p: String) -> FerroError {
                    FerroError::NotFound(p)
                }
                not_found(src.clone())
            })?
            .1;
        let mut meta = self
            .metadata
            .remove(&src)
            .ok_or_else(|| {
                #[cold]
                fn not_found(p: String) -> FerroError {
                    FerroError::NotFound(p)
                }
                not_found(src.clone())
            })?
            .1;

        meta.path = dst.clone();
        meta.etag = format!("\"{}\"", meta.content_hash.as_str());
        meta.modified_at = Utc::now();

        debug!("MOVE {} -> {}", src, dst);
        self.store.insert(dst.clone(), content);
        self.metadata.insert(dst, meta);

        Ok(())
    }

    async fn head(&self, path: &str) -> Result<FileMetadata> {
        let path = normalize_path(path).into_owned();
        self.metadata.get(&path).map(|m| m.value().clone()).ok_or_else(|| {
            #[cold]
            fn not_found(p: String) -> FerroError {
                FerroError::NotFound(p)
            }
            not_found(path)
        })
    }

    async fn exists(&self, path: &str) -> Result<bool> {
        let path = normalize_path(path).into_owned();
        Ok(self.metadata.contains_key(&path))
    }

    async fn create_collection(&self, path: &str, owner: &str) -> Result<FileMetadata> {
        let path = normalize_path(path).into_owned();

        if self.metadata.contains_key(&path) {
            #[cold]
            fn already_exists(p: String) -> FerroError {
                FerroError::AlreadyExists(p)
            }
            return Err(already_exists(path));
        }

        let meta = FileMetadata::new_collection(path.clone(), owner.to_string());
        debug!("MKCOL {}", path);
        self.metadata.insert(path.clone(), meta.clone());
        self.store.insert(path, Bytes::new());
        Ok(meta)
    }

    async fn list_all(&self, path: &str, max_depth: u32) -> Result<Vec<FileMetadata>> {
        let path = normalize_path(path);
        let prefix = if path == "/" {
            "/".to_string()
        } else {
            format!("{}/", path.trim_end_matches('/'))
        };

        let mut items: Vec<FileMetadata> = self
            .metadata
            .iter()
            .filter(|m| {
                if !m.path.starts_with(&prefix) || m.path == path.as_ref() {
                    return false;
                }
                let remaining = &m.path[prefix.len()..];
                let depth = remaining.matches('/').count() as u32;
                depth < max_depth
            })
            .map(|m| m.value().clone())
            .collect();

        // Hard cap on total results to prevent DoS (must match webdav::MAX_PROPFIND_DEPTH)
        items.truncate(100);

        items.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(items)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_put_get() {
        let engine = InMemoryStorageEngine::new();
        let content = Bytes::from("hello world");

        engine.put("/test.txt", content.clone(), "user1").await.unwrap();
        let retrieved = engine.get("/test.txt").await.unwrap();
        assert_eq!(content, retrieved);
    }

    #[tokio::test]
    async fn test_put_get_metadata() {
        let engine = InMemoryStorageEngine::new();
        let content = Bytes::from("hello world");

        let meta = engine.put("/test.txt", content, "user1").await.unwrap();
        assert_eq!(meta.path, "/test.txt");
        assert_eq!(meta.size, 11);
        assert_eq!(meta.owner, "user1");

        let head = engine.head("/test.txt").await.unwrap();
        assert_eq!(head.path, "/test.txt");
        assert_eq!(head.size, 11);
    }

    #[tokio::test]
    async fn test_delete() {
        let engine = InMemoryStorageEngine::new();
        engine.put("/test.txt", Bytes::from("hello"), "user1").await.unwrap();
        assert!(engine.exists("/test.txt").await.unwrap());

        engine.delete("/test.txt").await.unwrap();
        assert!(!engine.exists("/test.txt").await.unwrap());
    }

    #[tokio::test]
    async fn test_delete_not_found() {
        let engine = InMemoryStorageEngine::new();
        let result = engine.delete("/nonexistent.txt").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list() {
        let engine = InMemoryStorageEngine::new();
        engine.put("/docs/a.txt", Bytes::from("a"), "user1").await.unwrap();
        engine.put("/docs/b.txt", Bytes::from("b"), "user1").await.unwrap();
        engine.put("/other/c.txt", Bytes::from("c"), "user1").await.unwrap();

        let docs = engine.list("/docs").await.unwrap();
        assert_eq!(docs.len(), 2);
    }

    #[tokio::test]
    async fn test_copy() {
        let engine = InMemoryStorageEngine::new();
        engine.put("/src.txt", Bytes::from("hello"), "user1").await.unwrap();

        engine.copy("/src.txt", "/dst.txt").await.unwrap();
        assert!(engine.exists("/src.txt").await.unwrap());
        assert!(engine.exists("/dst.txt").await.unwrap());

        let src_content = engine.get("/src.txt").await.unwrap();
        let dst_content = engine.get("/dst.txt").await.unwrap();
        assert_eq!(src_content, dst_content);
    }

    #[tokio::test]
    async fn test_move_path() {
        let engine = InMemoryStorageEngine::new();
        engine.put("/old.txt", Bytes::from("hello"), "user1").await.unwrap();

        engine.move_path("/old.txt", "/new.txt").await.unwrap();
        assert!(!engine.exists("/old.txt").await.unwrap());
        assert!(engine.exists("/new.txt").await.unwrap());

        let content = engine.get("/new.txt").await.unwrap();
        assert_eq!(content, Bytes::from("hello"));
    }

    #[tokio::test]
    async fn test_get_not_found() {
        let engine = InMemoryStorageEngine::new();
        let result = engine.get("/nonexistent.txt").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_put_overwrite() {
        let engine = InMemoryStorageEngine::new();
        engine.put("/test.txt", Bytes::from("v1"), "user1").await.unwrap();
        engine.put("/test.txt", Bytes::from("v2"), "user1").await.unwrap();

        let content = engine.get("/test.txt").await.unwrap();
        assert_eq!(content, Bytes::from("v2"));
    }

    #[tokio::test]
    async fn test_create_collection() {
        let engine = InMemoryStorageEngine::new();
        let meta = engine.create_collection("/docs", "user1").await.unwrap();
        assert!(meta.is_collection);
        assert!(engine.exists("/docs").await.unwrap());
    }
}
