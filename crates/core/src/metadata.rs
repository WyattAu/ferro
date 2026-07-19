use async_trait::async_trait;
use dashmap::DashMap;
use ferro_common::error::{FerroError, Result};
use ferro_common::metadata::FileMetadata;
use tracing::debug;

/// Trait for storing and retrieving file metadata.
#[async_trait]
pub trait MetadataStore: Send + Sync {
    async fn get(&self, path: &str) -> Result<FileMetadata>;
    async fn put(&self, metadata: FileMetadata) -> Result<()>;
    async fn delete(&self, path: &str) -> Result<()>;
    async fn list(&self, prefix: &str) -> Result<Vec<FileMetadata>>;
    async fn exists(&self, path: &str) -> Result<bool>;
}

/// In-memory metadata store backed by a DashMap for lock-free concurrent access.
#[derive(Debug, Default)]
pub struct InMemoryMetadataStore {
    data: DashMap<String, FileMetadata>,
}

impl InMemoryMetadataStore {
    /// Create a new empty in-memory metadata store.
    #[must_use]
    pub fn new() -> Self {
        Self { data: DashMap::new() }
    }
}

#[async_trait]
impl MetadataStore for InMemoryMetadataStore {
    async fn get(&self, path: &str) -> Result<FileMetadata> {
        self.data
            .get(path)
            .map(|entry| entry.value().clone())
            .ok_or_else(|| FerroError::NotFound(path.to_string()))
    }

    async fn put(&self, metadata: FileMetadata) -> Result<()> {
        debug!("META PUT: {}", metadata.path);
        self.data.insert(metadata.path.clone(), metadata);
        Ok(())
    }

    async fn delete(&self, path: &str) -> Result<()> {
        self.data
            .remove(path)
            .ok_or_else(|| FerroError::NotFound(path.to_string()))?;
        debug!("META DELETE: {}", path);
        Ok(())
    }

    async fn list(&self, prefix: &str) -> Result<Vec<FileMetadata>> {
        Ok(self
            .data
            .iter()
            .filter(|m| m.value().path.starts_with(prefix))
            .map(|m| m.value().clone())
            .collect())
    }

    async fn exists(&self, path: &str) -> Result<bool> {
        Ok(self.data.contains_key(path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferro_common::metadata::ContentHash;

    #[tokio::test]
    async fn test_metadata_crud() {
        let store = InMemoryMetadataStore::new();
        let hash = ContentHash::new("a".repeat(64)).expect("valid hardcoded hash");
        let meta = FileMetadata::new("/test.txt".to_string(), hash, 42, "user1".to_string());

        assert!(!store.exists("/test.txt").await.unwrap());
        store.put(meta.clone()).await.unwrap();
        assert!(store.exists("/test.txt").await.unwrap());

        let retrieved = store.get("/test.txt").await.unwrap();
        assert_eq!(retrieved.path, "/test.txt");
        assert_eq!(retrieved.size, 42);

        store.delete("/test.txt").await.unwrap();
        assert!(!store.exists("/test.txt").await.unwrap());
    }

    #[tokio::test]
    async fn test_metadata_list() {
        let store = InMemoryMetadataStore::new();
        let hash = ContentHash::new("a".repeat(64)).expect("valid hardcoded hash");

        store
            .put(FileMetadata::new(
                "/docs/a.txt".to_string(),
                hash.clone(),
                10,
                "user1".to_string(),
            ))
            .await
            .unwrap();
        store
            .put(FileMetadata::new(
                "/docs/b.txt".to_string(),
                hash.clone(),
                20,
                "user1".to_string(),
            ))
            .await
            .unwrap();
        store
            .put(FileMetadata::new(
                "/other/c.txt".to_string(),
                hash.clone(),
                30,
                "user1".to_string(),
            ))
            .await
            .unwrap();

        let docs = store.list("/docs").await.unwrap();
        assert_eq!(docs.len(), 2);
    }

    #[tokio::test]
    async fn test_metadata_not_found() {
        let store = InMemoryMetadataStore::new();
        let result = store.get("/nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_metadata_update() {
        let store = InMemoryMetadataStore::new();
        let hash1 = ContentHash::new("a".repeat(64)).expect("valid hardcoded hash");
        let hash2 = ContentHash::new("b".repeat(64)).expect("valid hardcoded hash");

        store
            .put(FileMetadata::new(
                "/test.txt".to_string(),
                hash1,
                10,
                "user1".to_string(),
            ))
            .await
            .unwrap();

        store
            .put(FileMetadata::new(
                "/test.txt".to_string(),
                hash2,
                20,
                "user2".to_string(),
            ))
            .await
            .unwrap();

        let retrieved = store.get("/test.txt").await.unwrap();
        assert_eq!(retrieved.size, 20);
        assert_eq!(retrieved.owner, "user2");
    }
}
