pub use ferro_common::storage::StorageEngine;

use async_trait::async_trait;
use bytes::Bytes;
use chrono::Utc;
use ferro_common::error::{FerroError, Result};
use ferro_common::metadata::{ContentHash, FileMetadata};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;

/// In-memory storage engine that combines content and metadata storage.
pub struct InMemoryStorageEngine {
    store: Arc<RwLock<HashMap<String, Bytes>>>,
    metadata: Arc<RwLock<HashMap<String, FileMetadata>>>,
}

impl InMemoryStorageEngine {
    /// Create a new empty in-memory storage engine.
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
        let hash = ContentHash::compute(&content);
        let meta = FileMetadata::new(
            path.to_string(),
            hash.clone(),
            content.len() as u64,
            owner.to_string(),
        );

        let mut store = self.store.write().await;
        let mut meta_map = self.metadata.write().await;

        store.insert(path.to_string(), content);
        meta_map.insert(path.to_string(), meta.clone());

        debug!("PUT {} ({} bytes, hash={})", path, meta.size, hash.as_str());
        Ok(meta)
    }

    async fn get(&self, path: &str) -> Result<Bytes> {
        let store = self.store.read().await;
        store
            .get(path)
            .cloned()
            .ok_or_else(|| FerroError::NotFound(path.to_string()))
    }

    async fn delete(&self, path: &str) -> Result<()> {
        let mut store = self.store.write().await;
        let mut meta_map = self.metadata.write().await;

        store
            .remove(path)
            .ok_or_else(|| FerroError::NotFound(path.to_string()))?;
        meta_map.remove(path);

        debug!("DELETE {}", path);
        Ok(())
    }

    async fn list(&self, prefix: &str) -> Result<Vec<FileMetadata>> {
        let meta_map = self.metadata.read().await;
        let results: Vec<FileMetadata> = meta_map
            .values()
            .filter(|m| m.path.starts_with(prefix))
            .cloned()
            .collect();
        Ok(results)
    }

    async fn copy(&self, src: &str, dst: &str) -> Result<()> {
        // TOCTOU-safe: perform existence check AND extraction under a single write lock
        let mut store = self.store.write().await;
        let mut meta_map = self.metadata.write().await;

        let content = match store.get(src).cloned() {
            Some(c) => c,
            None => return Err(FerroError::NotFound(src.to_string())),
        };
        let mut meta = match meta_map.get(src).cloned() {
            Some(m) => m,
            None => return Err(FerroError::NotFound(src.to_string())),
        };

        meta.path = dst.to_string();
        meta.modified_at = Utc::now();

        store.insert(dst.to_string(), content);
        meta_map.insert(dst.to_string(), meta);

        debug!("COPY {} -> {}", src, dst);
        Ok(())
    }

    async fn move_path(&self, src: &str, dst: &str) -> Result<()> {
        let mut store = self.store.write().await;
        let mut meta_map = self.metadata.write().await;

        let content = store
            .remove(src)
            .ok_or_else(|| FerroError::NotFound(src.to_string()))?;
        let mut meta = meta_map
            .remove(src)
            .ok_or_else(|| FerroError::NotFound(src.to_string()))?;

        meta.path = dst.to_string();
        meta.modified_at = Utc::now();

        store.insert(dst.to_string(), content);
        meta_map.insert(dst.to_string(), meta);

        debug!("MOVE {} -> {}", src, dst);
        Ok(())
    }

    async fn head(&self, path: &str) -> Result<FileMetadata> {
        let meta_map = self.metadata.read().await;
        meta_map
            .get(path)
            .cloned()
            .ok_or_else(|| FerroError::NotFound(path.to_string()))
    }

    async fn exists(&self, path: &str) -> Result<bool> {
        let meta_map = self.metadata.read().await;
        Ok(meta_map.contains_key(path))
    }

    async fn create_collection(&self, path: &str, owner: &str) -> Result<FileMetadata> {
        let meta = FileMetadata::new_collection(path.to_string(), owner.to_string());
        self.metadata
            .write()
            .await
            .insert(path.to_string(), meta.clone());
        self.store
            .write()
            .await
            .insert(path.to_string(), Bytes::new());
        debug!("MKCOL {}", path);
        Ok(meta)
    }

    async fn list_all(&self, prefix: &str, max_depth: u32) -> Result<Vec<FileMetadata>> {
        let meta_map = self.metadata.read().await;
        let base = if prefix == "/" {
            ""
        } else {
            prefix.trim_end_matches('/')
        };
        let base_len = base.len();
        let results: Vec<FileMetadata> = meta_map
            .values()
            .filter(|m| {
                if !m.path.starts_with(base) || m.path == base || m.path == prefix {
                    return false;
                }
                let relative = &m.path[base_len..].trim_start_matches('/');
                let depth = relative.matches('/').count() as u32;
                depth < max_depth
            })
            .cloned()
            .collect();
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_put_get() {
        let engine = InMemoryStorageEngine::new();
        let content = Bytes::from("hello world");

        engine
            .put("/test.txt", content.clone(), "user1")
            .await
            .unwrap();
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
        engine
            .put("/test.txt", Bytes::from("hello"), "user1")
            .await
            .unwrap();
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
        engine
            .put("/docs/a.txt", Bytes::from("a"), "user1")
            .await
            .unwrap();
        engine
            .put("/docs/b.txt", Bytes::from("b"), "user1")
            .await
            .unwrap();
        engine
            .put("/other/c.txt", Bytes::from("c"), "user1")
            .await
            .unwrap();

        let docs = engine.list("/docs").await.unwrap();
        assert_eq!(docs.len(), 2);
    }

    #[tokio::test]
    async fn test_copy() {
        let engine = InMemoryStorageEngine::new();
        engine
            .put("/src.txt", Bytes::from("hello"), "user1")
            .await
            .unwrap();

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
    async fn test_get_not_found() {
        let engine = InMemoryStorageEngine::new();
        let result = engine.get("/nonexistent.txt").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_put_overwrite() {
        let engine = InMemoryStorageEngine::new();
        engine
            .put("/test.txt", Bytes::from("v1"), "user1")
            .await
            .unwrap();
        engine
            .put("/test.txt", Bytes::from("v2"), "user1")
            .await
            .unwrap();

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
