use async_trait::async_trait;
use bytes::Bytes;
use ferro_common::metadata::ContentHash;
use ferro_common::error::{FerroError, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;

/// Content-addressable storage trait for deduplicated blob storage.
#[async_trait]
pub trait CasStore: Send + Sync {
    async fn put_content(&self, content: Bytes) -> Result<ContentHash>;
    async fn get_content(&self, hash: &ContentHash) -> Result<Bytes>;
    async fn exists(&self, hash: &ContentHash) -> Result<bool>;
    async fn dedup_check(&self, hash: &ContentHash) -> Result<bool>;
    async fn content_count(&self) -> usize;
}

/// In-memory CAS store backed by a hash map.
pub struct InMemoryCasStore {
    content: Arc<RwLock<HashMap<String, Bytes>>>,
}

impl InMemoryCasStore {
    /// Create a new empty in-memory CAS store.
    pub fn new() -> Self {
        Self {
            content: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryCasStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CasStore for InMemoryCasStore {
    async fn put_content(&self, content: Bytes) -> Result<ContentHash> {
        let hash = ContentHash::compute(&content);
        let hash_key = hash.as_str().to_string();

        let mut store = self.content.write().await;
        if store.contains_key(&hash_key) {
            debug!("DEDUP: content {} already exists", &hash_key[..16]);
        } else {
            store.insert(hash_key.clone(), content);
            debug!("CAS PUT: stored content {}", &hash_key[..16]);
        }

        Ok(hash)
    }

    async fn get_content(&self, hash: &ContentHash) -> Result<Bytes> {
        let store = self.content.read().await;
        store
            .get(hash.as_str())
            .cloned()
            .ok_or_else(|| FerroError::NotFound(format!("content hash {}", hash.as_str())))
    }

    async fn exists(&self, hash: &ContentHash) -> Result<bool> {
        let store = self.content.read().await;
        Ok(store.contains_key(hash.as_str()))
    }

    async fn dedup_check(&self, hash: &ContentHash) -> Result<bool> {
        self.exists(hash).await
    }

    async fn content_count(&self) -> usize {
        let store = self.content.read().await;
        store.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cas_dedup() {
        let cas = InMemoryCasStore::new();
        let content = Bytes::from("hello world");

        let hash1 = cas.put_content(content.clone()).await.unwrap();
        let hash2 = cas.put_content(content.clone()).await.unwrap();

        assert_eq!(hash1, hash2);
        assert_eq!(cas.content_count().await, 1);
    }

    #[tokio::test]
    async fn test_cas_different_content() {
        let cas = InMemoryCasStore::new();
        let content1 = Bytes::from("hello");
        let content2 = Bytes::from("world");

        let hash1 = cas.put_content(content1).await.unwrap();
        let hash2 = cas.put_content(content2).await.unwrap();

        assert_ne!(hash1, hash2);
        assert_eq!(cas.content_count().await, 2);
    }

    #[tokio::test]
    async fn test_cas_roundtrip() {
        let cas = InMemoryCasStore::new();
        let content = Bytes::from("test content");

        let hash = cas.put_content(content.clone()).await.unwrap();
        let retrieved = cas.get_content(&hash).await.unwrap();

        assert_eq!(content, retrieved);
    }

    #[tokio::test]
    async fn test_cas_not_found() {
        let cas = InMemoryCasStore::new();
        let hash = ContentHash::new("0".repeat(64));

        let result = cas.get_content(&hash).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_cas_exists() {
        let cas = InMemoryCasStore::new();
        let content = Bytes::from("exists");
        let hash = cas.put_content(content).await.unwrap();

        assert!(cas.exists(&hash).await.unwrap());
        assert!(cas.dedup_check(&hash).await.unwrap());
    }

    #[tokio::test]
    async fn test_cas_not_exists() {
        let cas = InMemoryCasStore::new();
        let hash = ContentHash::new("a".repeat(64));

        assert!(!cas.exists(&hash).await.unwrap());
    }
}
