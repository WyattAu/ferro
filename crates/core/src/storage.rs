pub use ferro_common::storage::StorageEngine;

use async_trait::async_trait;
use bytes::Bytes;
use chrono::Utc;
use ferro_common::error::{FerroError, Result};
use ferro_common::metadata::{ContentHash, FileMetadata};
use ferro_common::path::normalize_path;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;

/// In-memory storage engine that combines content and metadata storage.
///
/// Implements the [`StorageEngine`] trait with path normalization, immediate-child
/// listing, and depth-limited recursive listing. Suitable for tests, benchmarks,
/// and single-instance servers.
#[derive(Debug, Clone)]
pub struct InMemoryStorageEngine {
    store: Arc<RwLock<HashMap<String, Bytes>>>,
    metadata: Arc<RwLock<HashMap<String, FileMetadata>>>,
}

impl InMemoryStorageEngine {
    /// Create a new empty in-memory storage engine.
    #[must_use]
    pub fn new() -> Self {
        Self {
            store: Arc::new(RwLock::new(HashMap::new())),
            metadata: Arc::new(RwLock::new(HashMap::new())),
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

        let mut store = self.store.write().await;
        let mut meta_map = self.metadata.write().await;

        store.insert(path.clone(), content);
        meta_map.insert(path.clone(), meta.clone());

        debug!("PUT {} ({} bytes, hash={})", path, meta.size, hash.as_str());
        Ok(meta)
    }

    async fn get(&self, path: &str) -> Result<Bytes> {
        let path = normalize_path(path).into_owned();
        let store = self.store.read().await;
        store.get(&path).cloned().ok_or_else(|| {
            #[cold]
            fn not_found(p: String) -> FerroError {
                FerroError::NotFound(p)
            }
            not_found(path)
        })
    }

    async fn delete(&self, path: &str) -> Result<()> {
        let path = normalize_path(path).into_owned();

        let meta_guard = self.metadata.read().await;
        if !meta_guard.contains_key(&path) {
            #[cold]
            fn not_found(p: String) -> FerroError {
                FerroError::NotFound(p)
            }
            return Err(not_found(path));
        }
        drop(meta_guard);

        self.store.write().await.remove(&path);
        self.metadata.write().await.remove(&path);

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

        let meta_guard = self.metadata.read().await;
        let mut items: Vec<FileMetadata> = meta_guard
            .values()
            .filter(|m| {
                if !m.path.starts_with(&prefix) || m.path == path.as_ref() {
                    return false;
                }
                let remaining = &m.path[prefix.len()..];
                !remaining.contains('/')
            })
            .cloned()
            .collect();

        items.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(items)
    }

    async fn copy(&self, src: &str, dst: &str) -> Result<()> {
        let src = normalize_path(src).into_owned();
        let dst = normalize_path(dst).into_owned();

        let mut store = self.store.write().await;
        let mut meta_map = self.metadata.write().await;

        let content = match store.get(&src).cloned() {
            Some(c) => c,
            None => {
                #[cold]
                fn not_found(p: String) -> FerroError {
                    FerroError::NotFound(p)
                }
                return Err(not_found(src));
            }
        };
        let mut meta = match meta_map.get(&src).cloned() {
            Some(m) => m,
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
        store.insert(dst.clone(), content);
        meta_map.insert(dst, meta);

        Ok(())
    }

    async fn move_path(&self, src: &str, dst: &str) -> Result<()> {
        let src = normalize_path(src).into_owned();
        let dst = normalize_path(dst).into_owned();

        let mut store = self.store.write().await;
        let mut meta_map = self.metadata.write().await;

        let content = store.remove(&src).ok_or_else(|| {
            #[cold]
            fn not_found(p: String) -> FerroError {
                FerroError::NotFound(p)
            }
            not_found(src.clone())
        })?;
        let mut meta = meta_map.remove(&src).ok_or_else(|| {
            #[cold]
            fn not_found(p: String) -> FerroError {
                FerroError::NotFound(p)
            }
            not_found(src.clone())
        })?;

        meta.path = dst.clone();
        meta.etag = format!("\"{}\"", meta.content_hash.as_str());
        meta.modified_at = Utc::now();

        debug!("MOVE {} -> {}", src, dst);
        store.insert(dst.clone(), content);
        meta_map.insert(dst, meta);

        Ok(())
    }

    async fn head(&self, path: &str) -> Result<FileMetadata> {
        let path = normalize_path(path).into_owned();
        let meta_map = self.metadata.read().await;
        meta_map.get(&path).cloned().ok_or_else(|| {
            #[cold]
            fn not_found(p: String) -> FerroError {
                FerroError::NotFound(p)
            }
            not_found(path)
        })
    }

    async fn exists(&self, path: &str) -> Result<bool> {
        let path = normalize_path(path).into_owned();
        let meta_map = self.metadata.read().await;
        Ok(meta_map.contains_key(&path))
    }

    async fn create_collection(&self, path: &str, owner: &str) -> Result<FileMetadata> {
        let path = normalize_path(path).into_owned();

        if self.metadata.read().await.contains_key(&path) {
            #[cold]
            fn already_exists(p: String) -> FerroError {
                FerroError::AlreadyExists(p)
            }
            return Err(already_exists(path));
        }

        let meta = FileMetadata::new_collection(path.clone(), owner.to_string());
        debug!("MKCOL {}", path);
        self.metadata.write().await.insert(path.clone(), meta.clone());
        self.store.write().await.insert(path, Bytes::new());
        Ok(meta)
    }

    async fn list_all(&self, path: &str, max_depth: u32) -> Result<Vec<FileMetadata>> {
        let path = normalize_path(path);
        let prefix = if path == "/" {
            "/".to_string()
        } else {
            format!("{}/", path.trim_end_matches('/'))
        };

        let meta_guard = self.metadata.read().await;
        let mut items: Vec<FileMetadata> = meta_guard
            .values()
            .filter(|m| {
                if !m.path.starts_with(&prefix) || m.path == path.as_ref() {
                    return false;
                }
                let remaining = &m.path[prefix.len()..];
                let depth = remaining.matches('/').count() as u32;
                depth < max_depth
            })
            .cloned()
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
