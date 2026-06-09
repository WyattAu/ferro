use std::sync::Arc;

use crate::erasure::{ErasureCoder, ErasureConfig, ReedSolomonErasureCoder, Shard};
use async_trait::async_trait;
use bytes::Bytes;
use common::error::{FerroError, Result};
use common::metadata::{ContentHash, FileMetadata};
use common::storage::StorageEngine;
use tracing::{debug, info, warn};

const METADATA_SUFFIX: &str = ".erasure.meta.json";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ShardInfo {
    pub index: u8,
    pub backend_path: String,
    pub shard_path: String,
    pub checksum: String,
    pub is_parity: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ErasureFileMetadata {
    pub original_path: String,
    pub data_shards: usize,
    pub parity_shards: usize,
    pub original_size: u64,
    pub shard_infos: Vec<ShardInfo>,
    pub owner: String,
    pub created_at: String,
}

#[derive(Clone)]
pub struct ErasureStorageConfig {
    pub data_shards: usize,
    pub parity_shards: usize,
    pub shard_backends: Vec<String>,
}

impl Default for ErasureStorageConfig {
    fn default() -> Self {
        Self {
            data_shards: 4,
            parity_shards: 2,
            shard_backends: vec!["/tmp/ferro-shards".to_string()],
        }
    }
}

pub struct ErasureStorageEngine {
    inner: Arc<dyn StorageEngine>,
    coder: ReedSolomonErasureCoder,
    config: ErasureStorageConfig,
}

impl ErasureStorageEngine {
    pub fn new(inner: Arc<dyn StorageEngine>, config: ErasureStorageConfig) -> Self {
        let erasure_config = ErasureConfig {
            data_shards: config.data_shards,
            parity_shards: config.parity_shards,
            shard_size: 1024 * 1024,
        };
        let coder = ReedSolomonErasureCoder::new(erasure_config);
        Self {
            inner,
            coder,
            config,
        }
    }

    fn shard_path(&self, backend_idx: usize, file_key: &str, shard_idx: usize) -> String {
        format!(
            "{}/shards/{}/shard_{}",
            self.config.shard_backends[backend_idx % self.config.shard_backends.len()],
            file_key,
            shard_idx
        )
    }

    fn meta_path(&self, path: &str) -> String {
        format!("{}.erasure.meta", path)
    }

    fn file_key(path: &str) -> String {
        use sha2::Digest;
        let hash = sha2::Sha256::digest(path.as_bytes());
        hex::encode(hash)
    }

    async fn write_shard(
        &self,
        backend_idx: usize,
        file_key: &str,
        shard: &Shard,
    ) -> Result<ShardInfo> {
        let shard_path = self.shard_path(backend_idx, file_key, shard.index as usize);
        let shard_dir = std::path::Path::new(&shard_path)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        if !shard_dir.is_empty() {
            let _ = tokio::fs::create_dir_all(&shard_dir).await;
        }

        tokio::fs::write(&shard_path, &shard.data)
            .await
            .map_err(|e| FerroError::StorageBackend(format!("Failed to write shard: {}", e)))?;

        let shard_size = shard.data.len() as u64;
        debug!(
            "Wrote shard {} ({} bytes) to {}",
            shard.index, shard_size, shard_path
        );

        Ok(ShardInfo {
            index: shard.index,
            backend_path: self.config.shard_backends
                [backend_idx % self.config.shard_backends.len()]
            .clone(),
            shard_path,
            checksum: hex::encode(shard.checksum),
            is_parity: shard.is_parity,
        })
    }

    async fn read_shard(&self, shard_info: &ShardInfo) -> Result<Option<Shard>> {
        match tokio::fs::read(&shard_info.shard_path).await {
            Ok(data) => {
                let checksum: [u8; 32] = hex::decode(&shard_info.checksum)
                    .map_err(|e| FerroError::StorageBackend(format!("Invalid checksum: {}", e)))?
                    .try_into()
                    .map_err(|_| FerroError::StorageBackend("Invalid checksum length".into()))?;

                Ok(Some(Shard {
                    index: shard_info.index,
                    data,
                    is_parity: shard_info.is_parity,
                    checksum,
                }))
            }
            Err(e) => {
                warn!(
                    "Failed to read shard {} from {}: {}",
                    shard_info.index, shard_info.shard_path, e
                );
                Ok(None)
            }
        }
    }

    async fn store_metadata(&self, path: &str, erasure_meta: &ErasureFileMetadata) -> Result<()> {
        let meta_json = serde_json::to_vec_pretty(erasure_meta).map_err(|e| {
            FerroError::StorageBackend(format!("Failed to serialize metadata: {}", e))
        })?;
        self.inner
            .put(
                &self.meta_path(path),
                Bytes::from(meta_json),
                &erasure_meta.owner,
            )
            .await?;
        Ok(())
    }

    async fn load_metadata(&self, path: &str) -> Result<ErasureFileMetadata> {
        let data = self.inner.get(&self.meta_path(path)).await?;
        serde_json::from_slice(&data)
            .map_err(|e| FerroError::StorageBackend(format!("Invalid erasure metadata: {}", e)))
    }

    async fn delete_shards(&self, erasure_meta: &ErasureFileMetadata) {
        for shard_info in &erasure_meta.shard_infos {
            if let Err(e) = tokio::fs::remove_file(&shard_info.shard_path).await {
                warn!(
                    "Failed to delete shard {} at {}: {}",
                    shard_info.index, shard_info.shard_path, e
                );
            }
        }
    }
}

#[async_trait]
impl StorageEngine for ErasureStorageEngine {
    async fn head(&self, path: &str) -> Result<FileMetadata> {
        self.inner.head(path).await
    }

    async fn get(&self, path: &str) -> Result<Bytes> {
        let erasure_meta = match self.load_metadata(path).await {
            Ok(meta) => meta,
            Err(_) => {
                return self.inner.get(path).await;
            }
        };

        let mut shards: Vec<Option<Shard>> =
            Vec::with_capacity(erasure_meta.data_shards + erasure_meta.parity_shards);
        for shard_info in &erasure_meta.shard_infos {
            shards.push(self.read_shard(shard_info).await?);
        }

        let reconstructed = self
            .coder
            .decode(&shards)
            .map_err(|e| FerroError::StorageBackend(format!("Erasure decode failed: {}", e)))?;

        let original_size = erasure_meta.original_size as usize;
        let data = if reconstructed.len() > original_size {
            Bytes::copy_from_slice(&reconstructed[..original_size])
        } else {
            Bytes::from(reconstructed)
        };

        info!(
            "Reconstructed {} from {} shards ({} bytes)",
            path,
            erasure_meta.shard_infos.len(),
            data.len()
        );

        Ok(data)
    }

    async fn put(&self, path: &str, content: Bytes, owner: &str) -> Result<FileMetadata> {
        let file_key = Self::file_key(path);

        let shards = self
            .coder
            .encode(&content)
            .map_err(|e| FerroError::StorageBackend(format!("Erasure encode failed: {}", e)))?;

        let mut shard_infos = Vec::with_capacity(shards.len());
        for (i, shard) in shards.iter().enumerate() {
            let backend_idx = i % self.config.shard_backends.len();
            let shard_info = self.write_shard(backend_idx, &file_key, shard).await?;
            shard_infos.push(shard_info);
        }

        let erasure_meta = ErasureFileMetadata {
            original_path: path.to_string(),
            data_shards: self.config.data_shards,
            parity_shards: self.config.parity_shards,
            original_size: content.len() as u64,
            shard_infos,
            owner: owner.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        self.store_metadata(path, &erasure_meta).await?;

        let content_hash = ContentHash::compute(&content);
        let meta = FileMetadata::new(
            path.to_string(),
            content_hash,
            content.len() as u64,
            owner.to_string(),
        );

        info!(
            "Encoded {} into {} shards ({} bytes total)",
            path,
            erasure_meta.data_shards + erasure_meta.parity_shards,
            content.len()
        );

        Ok(meta)
    }

    async fn delete(&self, path: &str) -> Result<()> {
        match self.load_metadata(path).await {
            Ok(erasure_meta) => {
                self.delete_shards(&erasure_meta).await;
                self.inner.delete(&self.meta_path(path)).await?;
                info!("Deleted erasure-coded file and {} shards", path);
            }
            Err(_) => {
                self.inner.delete(path).await?;
            }
        }
        Ok(())
    }

    async fn list(&self, path: &str) -> Result<Vec<FileMetadata>> {
        let entries = self.inner.list(path).await?;
        let filtered: Vec<FileMetadata> = entries
            .into_iter()
            .filter(|m| !m.path.ends_with(METADATA_SUFFIX))
            .collect();
        Ok(filtered)
    }

    async fn copy(&self, from: &str, to: &str) -> Result<()> {
        let erasure_meta = self.load_metadata(from).await?;
        let mut new_shard_infos = Vec::with_capacity(erasure_meta.shard_infos.len());

        let new_file_key = Self::file_key(to);
        for shard_info in &erasure_meta.shard_infos {
            match self.read_shard(shard_info).await? {
                Some(shard) => {
                    let backend_idx = shard.index as usize;
                    let new_info = self.write_shard(backend_idx, &new_file_key, &shard).await?;
                    new_shard_infos.push(new_info);
                }
                None => {
                    return Err(FerroError::StorageBackend(format!(
                        "Failed to copy shard {}",
                        shard_info.index
                    )));
                }
            }
        }

        let new_meta = ErasureFileMetadata {
            original_path: to.to_string(),
            data_shards: erasure_meta.data_shards,
            parity_shards: erasure_meta.parity_shards,
            original_size: erasure_meta.original_size,
            shard_infos: new_shard_infos,
            owner: erasure_meta.owner.clone(),
            created_at: erasure_meta.created_at.clone(),
        };

        self.store_metadata(to, &new_meta).await?;
        Ok(())
    }

    async fn move_path(&self, from: &str, to: &str) -> Result<()> {
        self.copy(from, to).await?;
        self.delete(from).await?;
        Ok(())
    }

    async fn exists(&self, path: &str) -> Result<bool> {
        self.inner.exists(path).await
    }

    async fn create_collection(&self, path: &str, owner: &str) -> Result<FileMetadata> {
        self.inner.create_collection(path, owner).await
    }

    async fn list_all(&self, path: &str, max_depth: u32) -> Result<Vec<FileMetadata>> {
        let entries = self.inner.list_all(path, max_depth).await?;
        let filtered: Vec<FileMetadata> = entries
            .into_iter()
            .filter(|m| !m.path.ends_with(METADATA_SUFFIX))
            .collect();
        Ok(filtered)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tokio::sync::RwLock;

    struct MockStorageEngine {
        data: Arc<RwLock<HashMap<String, Bytes>>>,
        metadata: Arc<RwLock<HashMap<String, FileMetadata>>>,
    }

    impl MockStorageEngine {
        fn new() -> Self {
            Self {
                data: Arc::new(RwLock::new(HashMap::new())),
                metadata: Arc::new(RwLock::new(HashMap::new())),
            }
        }
    }

    #[async_trait]
    impl StorageEngine for MockStorageEngine {
        async fn head(&self, path: &str) -> Result<FileMetadata> {
            self.metadata
                .read()
                .await
                .get(path)
                .cloned()
                .ok_or_else(|| FerroError::NotFound(path.to_string()))
        }

        async fn get(&self, path: &str) -> Result<Bytes> {
            self.data
                .read()
                .await
                .get(path)
                .cloned()
                .ok_or_else(|| FerroError::NotFound(path.to_string()))
        }

        async fn put(&self, path: &str, content: Bytes, owner: &str) -> Result<FileMetadata> {
            let hash = ContentHash::compute(&content);
            let meta = FileMetadata::new(
                path.to_string(),
                hash,
                content.len() as u64,
                owner.to_string(),
            );
            self.data.write().await.insert(path.to_string(), content);
            self.metadata
                .write()
                .await
                .insert(path.to_string(), meta.clone());
            Ok(meta)
        }

        async fn delete(&self, path: &str) -> Result<()> {
            self.data.write().await.remove(path);
            self.metadata.write().await.remove(path);
            Ok(())
        }

        async fn list(&self, path: &str) -> Result<Vec<FileMetadata>> {
            let prefix = if path.ends_with('/') {
                path.to_string()
            } else {
                format!("{}/", path)
            };
            let meta = self.metadata.read().await;
            Ok(meta
                .keys()
                .filter(|k| k.starts_with(&prefix))
                .filter_map(|k| meta.get(k).cloned())
                .collect())
        }

        async fn copy(&self, from: &str, to: &str) -> Result<()> {
            let data = self.get(from).await?;
            let meta = self.head(from).await?;
            self.put(to, data, &meta.owner).await?;
            Ok(())
        }

        async fn move_path(&self, from: &str, to: &str) -> Result<()> {
            self.copy(from, to).await?;
            self.delete(from).await?;
            Ok(())
        }

        async fn exists(&self, path: &str) -> Result<bool> {
            Ok(self.data.read().await.contains_key(path))
        }

        async fn create_collection(&self, path: &str, owner: &str) -> Result<FileMetadata> {
            let meta = FileMetadata::new(
                path.to_string(),
                ContentHash::compute(&[]),
                0,
                owner.to_string(),
            );
            self.metadata
                .write()
                .await
                .insert(path.to_string(), meta.clone());
            Ok(meta)
        }

        async fn list_all(&self, path: &str, _max_depth: u32) -> Result<Vec<FileMetadata>> {
            self.list(path).await
        }
    }

    fn test_config() -> ErasureStorageConfig {
        let dir = tempfile::tempdir().unwrap();
        ErasureStorageConfig {
            data_shards: 4,
            parity_shards: 2,
            shard_backends: vec![dir.path().to_string_lossy().to_string()],
        }
    }

    #[tokio::test]
    async fn test_erasure_put_and_get() {
        let inner = Arc::new(MockStorageEngine::new());
        let config = test_config();
        let engine = ErasureStorageEngine::new(inner, config);

        let data = b"hello erasure coding world!".to_vec();
        engine
            .put("/test.txt", Bytes::from(data.clone()), "alice")
            .await
            .unwrap();

        let retrieved = engine.get("/test.txt").await.unwrap();
        assert_eq!(retrieved.as_ref(), data.as_slice());
    }

    #[tokio::test]
    async fn test_erasure_delete() {
        let inner = Arc::new(MockStorageEngine::new());
        let config = test_config();
        let engine = ErasureStorageEngine::new(inner, config);

        let data = b"delete me".to_vec();
        engine
            .put("/delete.txt", Bytes::from(data), "alice")
            .await
            .unwrap();

        engine.delete("/delete.txt").await.unwrap();
        assert!(engine.get("/delete.txt").await.is_err());
    }

    #[tokio::test]
    async fn test_erasure_list_filters_metadata() {
        let inner = Arc::new(MockStorageEngine::new());
        let config = test_config();
        let engine = ErasureStorageEngine::new(inner.clone(), config);

        engine
            .put("/file.txt", Bytes::from(b"data".to_vec()), "alice")
            .await
            .unwrap();

        let listing = engine.list("/").await.unwrap();
        assert!(listing.iter().all(|m| !m.path.ends_with(METADATA_SUFFIX)));
    }
}
