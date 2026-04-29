use bytes::Bytes;
use common::error::FerroError;
use common::error::Result;
use common::metadata::{ContentHash, FileMetadata};
use common::path::normalize_path;
use common::storage::StorageEngine;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;

/// Server-side in-memory storage engine with path normalization.
#[derive(Clone)]
pub struct InMemoryStorageEngine {
    data: Arc<RwLock<DashMap<String, Bytes>>>,
    metadata: Arc<RwLock<DashMap<String, FileMetadata>>>,
}

impl InMemoryStorageEngine {
    /// Create a new empty in-memory storage engine.
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(DashMap::new())),
            metadata: Arc::new(RwLock::new(DashMap::new())),
        }
    }
}

impl Default for InMemoryStorageEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl StorageEngine for InMemoryStorageEngine {
    async fn head(&self, path: &str) -> Result<FileMetadata> {
        let path = normalize_path(path);
        let meta = self
            .metadata
            .read()
            .await
            .get(&path)
            .map(|m| m.value().clone())
            .ok_or_else(|| FerroError::NotFound(path.to_string()))?;
        Ok(meta)
    }

    async fn get(&self, path: &str) -> Result<Bytes> {
        let path = normalize_path(path);
        let data = self
            .data
            .read()
            .await
            .get(&path)
            .map(|d| d.value().clone())
            .ok_or_else(|| FerroError::NotFound(path.to_string()))?;
        Ok(data)
    }

    async fn put(&self, path: &str, content: Bytes, owner: &str) -> Result<FileMetadata> {
        let path = normalize_path(path);
        let content_hash = ContentHash::compute(&content);
        let size = content.len() as u64;

        let meta = FileMetadata::new(path.clone(), content_hash, size, owner.to_string());

        self.data.write().await.insert(path.clone(), content);
        self.metadata
            .write()
            .await
            .insert(path.clone(), meta.clone());

        debug!("PUT {} ({} bytes)", path, size);
        Ok(meta)
    }

    async fn delete(&self, path: &str) -> Result<()> {
        let path = normalize_path(path);

        let meta_guard = self.metadata.read().await;
        if !meta_guard.contains_key(&path) {
            return Err(FerroError::NotFound(path.to_string()));
        }
        drop(meta_guard);

        self.data.write().await.remove(&path);
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
            .iter()
            .filter_map(|entry| {
                let key = entry.key();
                if key.starts_with(&prefix) && key != &path {
                    let remaining = &key[prefix.len()..];
                    if !remaining.contains('/') {
                        return Some(entry.value().clone());
                    }
                }
                None
            })
            .collect();

        items.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(items)
    }

    async fn exists(&self, path: &str) -> Result<bool> {
        let path = normalize_path(path);
        Ok(self.metadata.read().await.contains_key(&path))
    }

    async fn copy(&self, from: &str, to: &str) -> Result<()> {
        let from = normalize_path(from);
        let to = normalize_path(to);

        let data_guard = self.data.read().await;
        let meta_guard = self.metadata.read().await;

        let content = data_guard
            .get(&from)
            .map(|d| d.value().clone())
            .ok_or_else(|| FerroError::NotFound(from.to_string()))?;
        let mut meta = meta_guard
            .get(&from)
            .map(|m| m.value().clone())
            .ok_or_else(|| FerroError::NotFound(from.to_string()))?;

        drop(data_guard);
        drop(meta_guard);

        meta.path = to.clone();
        meta.etag = format!("\"{}\"", meta.content_hash.as_str());
        meta.modified_at = chrono::Utc::now();

        self.data.write().await.insert(to.clone(), content);
        self.metadata.write().await.insert(to.clone(), meta);

        debug!("COPY {} -> {}", from, to);
        Ok(())
    }

    async fn move_path(&self, from: &str, to: &str) -> Result<()> {
        let from = normalize_path(from);
        let to = normalize_path(to);

        let data_guard = self.data.write().await;
        let meta_guard = self.metadata.write().await;

        let content = data_guard
            .remove(&from)
            .map(|(_, v)| v)
            .ok_or_else(|| FerroError::NotFound(from.to_string()))?;
        let mut meta = meta_guard
            .remove(&from)
            .map(|(_, v)| v)
            .ok_or_else(|| FerroError::NotFound(from.to_string()))?;

        meta.path = to.clone();
        meta.etag = format!("\"{}\"", meta.content_hash.as_str());
        meta.modified_at = chrono::Utc::now();

        data_guard.insert(to.clone(), content);
        meta_guard.insert(to.clone(), meta);

        debug!("MOVE {} -> {}", from, to);
        Ok(())
    }

    async fn create_collection(&self, path: &str, owner: &str) -> Result<FileMetadata> {
        let path = normalize_path(path);

        if self.metadata.read().await.contains_key(&path) {
            return Err(FerroError::AlreadyExists(path.to_string()));
        }

        let meta = FileMetadata::new_collection(path.clone(), owner.to_string());
        self.data.write().await.insert(path.clone(), Bytes::new());
        self.metadata
            .write()
            .await
            .insert(path.clone(), meta.clone());

        debug!("MKCOL {}", path);
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
            .iter()
            .filter_map(|entry| {
                let key = entry.key();
                if key.starts_with(&prefix) && key != &path {
                    // Calculate depth relative to the queried path
                    let remaining = &key[prefix.len()..];
                    let depth = remaining.matches('/').count() as u32;
                    if depth < max_depth {
                        return Some(entry.value().clone());
                    }
                }
                None
            })
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
    async fn test_put_get_delete() {
        let engine = InMemoryStorageEngine::new();

        engine
            .put("/hello.txt", Bytes::from("hello"), "user1")
            .await
            .unwrap();

        let meta = engine.head("/hello.txt").await.unwrap();
        assert_eq!(meta.size, 5);

        let content = engine.get("/hello.txt").await.unwrap();
        assert_eq!(&content[..], b"hello");

        engine.delete("/hello.txt").await.unwrap();
        assert!(engine.exists("/hello.txt").await.unwrap() == false);
    }

    #[tokio::test]
    async fn test_list() {
        let engine = InMemoryStorageEngine::new();

        engine.create_collection("/docs", "user1").await.unwrap();
        engine
            .put("/docs/a.txt", Bytes::from("a"), "user1")
            .await
            .unwrap();
        engine
            .put("/docs/b.txt", Bytes::from("b"), "user1")
            .await
            .unwrap();

        let items = engine.list("/docs").await.unwrap();
        assert_eq!(items.len(), 2);
    }

    #[tokio::test]
    async fn test_copy() {
        let engine = InMemoryStorageEngine::new();

        engine
            .put("/original.txt", Bytes::from("data"), "user1")
            .await
            .unwrap();
        engine.copy("/original.txt", "/copy.txt").await.unwrap();

        let content = engine.get("/copy.txt").await.unwrap();
        assert_eq!(&content[..], b"data");

        assert!(engine.exists("/original.txt").await.unwrap());
    }

    #[tokio::test]
    async fn test_move_path() {
        let engine = InMemoryStorageEngine::new();

        engine
            .put("/source.txt", Bytes::from("data"), "user1")
            .await
            .unwrap();
        engine.move_path("/source.txt", "/dest.txt").await.unwrap();

        assert!(engine.exists("/source.txt").await.unwrap() == false);
        let content = engine.get("/dest.txt").await.unwrap();
        assert_eq!(&content[..], b"data");
    }

    #[tokio::test]
    async fn test_not_found() {
        let engine = InMemoryStorageEngine::new();
        assert!(engine.head("/missing").await.is_err());
        assert!(engine.get("/missing").await.is_err());
        assert!(engine.delete("/missing").await.is_err());
    }

    #[tokio::test]
    async fn test_list_all_with_depth_limit() {
        let engine = InMemoryStorageEngine::new();
        engine.create_collection("/root", "user1").await.unwrap();
        engine
            .create_collection("/root/sub", "user1")
            .await
            .unwrap();
        engine
            .create_collection("/root/sub/deep", "user1")
            .await
            .unwrap();
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

        // depth=1 should get root/* (sub, f1.txt) — 2 items
        let items = engine.list_all("/root", 1).await.unwrap();
        assert_eq!(items.len(), 2);

        // depth=2 should get root/* and root/*/* (sub, f1.txt, deep, f2.txt) — 4 items
        let items = engine.list_all("/root", 2).await.unwrap();
        assert_eq!(items.len(), 4);

        // depth=100 should get everything except /root itself (5 items)
        let items = engine.list_all("/root", 100).await.unwrap();
        assert_eq!(items.len(), 5);
    }
}
