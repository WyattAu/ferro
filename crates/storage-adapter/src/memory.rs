use async_trait::async_trait;
use chrono::Utc;
use dashmap::DashMap;

use crate::backend::{BackendType, ObjectInfo, ObjectMetadata, StorageBackend};
use crate::error::StorageAdapterError;

#[derive(Debug)]
pub struct InMemoryBackend {
    store: DashMap<String, (Vec<u8>, ObjectMetadata)>,
}

impl InMemoryBackend {
    pub fn new() -> Self {
        Self {
            store: DashMap::new(),
        }
    }
}

impl Default for InMemoryBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl StorageBackend for InMemoryBackend {
    async fn get(&self, path: &str) -> Result<Vec<u8>, StorageAdapterError> {
        self.store
            .get(path)
            .map(|r| r.value().0.clone())
            .ok_or_else(|| StorageAdapterError::NotFound(path.to_string()))
    }

    async fn put(
        &self,
        path: &str,
        data: &[u8],
        metadata: &ObjectMetadata,
    ) -> Result<(), StorageAdapterError> {
        let mut meta = metadata.clone();
        meta.last_modified = Some(Utc::now());
        self.store.insert(path.to_string(), (data.to_vec(), meta));
        Ok(())
    }

    async fn delete(&self, path: &str) -> Result<(), StorageAdapterError> {
        self.store
            .remove(path)
            .map(|_| ())
            .ok_or_else(|| StorageAdapterError::NotFound(path.to_string()))
    }

    async fn exists(&self, path: &str) -> Result<bool, StorageAdapterError> {
        Ok(self.store.contains_key(path))
    }

    async fn list(&self, prefix: &str) -> Result<Vec<ObjectInfo>, StorageAdapterError> {
        let prefix = if prefix.ends_with('/') {
            prefix.to_string()
        } else {
            format!("{prefix}/")
        };
        let mut results = Vec::new();
        for entry in self.store.iter() {
            let key = entry.key();
            if key.starts_with(&prefix) {
                let (data, meta) = entry.value();
                results.push(ObjectInfo {
                    path: key.clone(),
                    size: data.len() as u64,
                    content_type: meta.content_type.clone(),
                    last_modified: meta.last_modified,
                    etag: meta.etag.clone(),
                    metadata: meta.custom_headers.clone(),
                });
            }
        }
        results.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(results)
    }

    async fn size(&self, path: &str) -> Result<u64, StorageAdapterError> {
        self.store
            .get(path)
            .map(|r| r.value().0.len() as u64)
            .ok_or_else(|| StorageAdapterError::NotFound(path.to_string()))
    }

    async fn copy(&self, from: &str, to: &str) -> Result<(), StorageAdapterError> {
        let entry = self
            .store
            .get(from)
            .ok_or_else(|| StorageAdapterError::NotFound(from.to_string()))?;
        self.store.insert(to.to_string(), entry.value().clone());
        Ok(())
    }

    async fn move_obj(&self, from: &str, to: &str) -> Result<(), StorageAdapterError> {
        let entry = self
            .store
            .remove(from)
            .ok_or_else(|| StorageAdapterError::NotFound(from.to_string()))?;
        self.store.insert(to.to_string(), entry.1);
        Ok(())
    }

    async fn metadata(&self, path: &str) -> Result<ObjectInfo, StorageAdapterError> {
        self.store
            .get(path)
            .map(|r| {
                let (data, meta) = r.value();
                ObjectInfo {
                    path: path.to_string(),
                    size: data.len() as u64,
                    content_type: meta.content_type.clone(),
                    last_modified: meta.last_modified,
                    etag: meta.etag.clone(),
                    metadata: meta.custom_headers.clone(),
                }
            })
            .ok_or_else(|| StorageAdapterError::NotFound(path.to_string()))
    }

    fn backend_type(&self) -> BackendType {
        BackendType::Memory
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::task::JoinSet;

    #[tokio::test]
    async fn test_put_get_delete() {
        let b = InMemoryBackend::new();
        b.put("k", b"v", &ObjectMetadata::new()).await.unwrap();
        assert_eq!(b.get("k").await.unwrap(), b"v");
        b.delete("k").await.unwrap();
        assert!(b.get("k").await.is_err());
    }

    #[tokio::test]
    async fn test_exists() {
        let b = InMemoryBackend::new();
        assert!(!b.exists("x").await.unwrap());
        b.put("x", b"", &ObjectMetadata::new()).await.unwrap();
        assert!(b.exists("x").await.unwrap());
    }

    #[tokio::test]
    async fn test_list_with_prefix() {
        let b = InMemoryBackend::new();
        b.put("docs/a.txt", b"a", &ObjectMetadata::new()).await.unwrap();
        b.put("docs/b.txt", b"b", &ObjectMetadata::new()).await.unwrap();
        b.put("other/c.txt", b"c", &ObjectMetadata::new()).await.unwrap();
        let items = b.list("docs").await.unwrap();
        assert_eq!(items.len(), 2);
    }

    #[tokio::test]
    async fn test_size() {
        let b = InMemoryBackend::new();
        b.put("s", b"hello", &ObjectMetadata::new()).await.unwrap();
        assert_eq!(b.size("s").await.unwrap(), 5);
    }

    #[tokio::test]
    async fn test_copy() {
        let b = InMemoryBackend::new();
        b.put("a", b"data", &ObjectMetadata::new()).await.unwrap();
        b.copy("a", "b").await.unwrap();
        assert_eq!(b.get("b").await.unwrap(), b"data");
    }

    #[tokio::test]
    async fn test_move_obj() {
        let b = InMemoryBackend::new();
        b.put("src", b"m", &ObjectMetadata::new()).await.unwrap();
        b.move_obj("src", "dst").await.unwrap();
        assert!(b.get("src").await.is_err());
        assert_eq!(b.get("dst").await.unwrap(), b"m");
    }

    #[tokio::test]
    async fn test_metadata_returns_info() {
        let b = InMemoryBackend::new();
        let meta = ObjectMetadata::new().with_content_type("text/plain").with_etag("abc");
        b.put("info", b"data", &meta).await.unwrap();
        let info = b.metadata("info").await.unwrap();
        assert_eq!(info.size, 4);
        assert_eq!(info.content_type.as_deref(), Some("text/plain"));
    }

    #[tokio::test]
    async fn test_concurrent_access() {
        let b = Arc::new(InMemoryBackend::new());
        let mut set = JoinSet::new();
        for i in 0..100u32 {
            let b = Arc::clone(&b);
            set.spawn(async move {
                let key = format!("k/{i}");
                let val = i.to_le_bytes();
                b.put(&key, &val, &ObjectMetadata::new()).await.unwrap();
                let got = b.get(&key).await.unwrap();
                assert_eq!(got, val);
            });
        }
        while let Some(res) = set.join_next().await {
            res.unwrap();
        }
    }

    #[tokio::test]
    async fn test_backend_type() {
        let b = InMemoryBackend::new();
        assert_eq!(b.backend_type(), BackendType::Memory);
    }
}
