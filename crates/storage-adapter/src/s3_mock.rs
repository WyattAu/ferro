use async_trait::async_trait;
use chrono::Utc;
use dashmap::DashMap;

use crate::backend::{BackendType, ObjectInfo, ObjectMetadata, StorageBackend};
use crate::error::StorageAdapterError;

#[derive(Debug, Clone)]
struct MultipartUpload {
    parts: Vec<Vec<u8>>,
}

#[derive(Debug)]
pub struct MockS3Backend {
    store: DashMap<String, (Vec<u8>, ObjectMetadata)>,
    multipart: DashMap<String, MultipartUpload>,
}

impl MockS3Backend {
    pub fn new() -> Self {
        Self {
            store: DashMap::new(),
            multipart: DashMap::new(),
        }
    }

    fn normalize_key(&self, bucket: &str, key: &str) -> String {
        if key.starts_with(&format!("{bucket}/")) {
            key.to_string()
        } else {
            format!("{bucket}/{key}")
        }
    }

    pub async fn put_small(
        &self,
        bucket: &str,
        key: &str,
        data: &[u8],
        metadata: &ObjectMetadata,
    ) -> Result<(), StorageAdapterError> {
        let path = self.normalize_key(bucket, key);
        self.store.insert(path, (data.to_vec(), metadata.clone()));
        Ok(())
    }

    pub async fn start_multipart(&self, bucket: &str, key: &str) -> String {
        let _path = self.normalize_key(bucket, key);
        let upload_id = uuid::Uuid::new_v4().to_string();
        self.multipart
            .insert(upload_id.clone(), MultipartUpload { parts: Vec::new() });
        upload_id
    }

    pub async fn upload_part(
        &self,
        upload_id: &str,
        data: &[u8],
    ) -> Result<(), StorageAdapterError> {
        let mut entry = self
            .multipart
            .get_mut(upload_id)
            .ok_or_else(|| StorageAdapterError::NotFound(format!("upload {upload_id}")))?;
        entry.parts.push(data.to_vec());
        Ok(())
    }

    pub async fn complete_multipart(
        &self,
        upload_id: &str,
        bucket: &str,
        key: &str,
        metadata: &ObjectMetadata,
    ) -> Result<(), StorageAdapterError> {
        let (_, entry) = self
            .multipart
            .remove(upload_id)
            .ok_or_else(|| StorageAdapterError::NotFound(format!("upload {upload_id}")))?;
        let combined: Vec<u8> = entry.parts.into_iter().flatten().collect();
        let path = self.normalize_key(bucket, key);
        self.store.insert(path, (combined, metadata.clone()));
        Ok(())
    }
}

impl Default for MockS3Backend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl StorageBackend for MockS3Backend {
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
        let mut results = Vec::new();
        for entry in self.store.iter() {
            if entry.key().starts_with(prefix) {
                let (data, meta) = entry.value();
                results.push(ObjectInfo {
                    path: entry.key().clone(),
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
        BackendType::S3
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_put_get_lifecycle() {
        let b = MockS3Backend::new();
        b.put("mybucket/key", b"val", &ObjectMetadata::new())
            .await
            .unwrap();
        assert_eq!(b.get("mybucket/key").await.unwrap(), b"val");
        b.delete("mybucket/key").await.unwrap();
        assert!(b.get("mybucket/key").await.is_err());
    }

    #[tokio::test]
    async fn test_put_small_bucket_key() {
        let b = MockS3Backend::new();
        b.put_small("b", "k", b"v", &ObjectMetadata::new())
            .await
            .unwrap();
        assert_eq!(b.get("b/k").await.unwrap(), b"v");
    }

    #[tokio::test]
    async fn test_multipart_upload() {
        let b = MockS3Backend::new();
        let uid = b.start_multipart("b", "multi").await;
        b.upload_part(&uid, b"part1").await.unwrap();
        b.upload_part(&uid, b"part2").await.unwrap();
        b.complete_multipart(&uid, "b", "multi", &ObjectMetadata::new())
            .await
            .unwrap();
        let data = b.get("b/multi").await.unwrap();
        assert_eq!(data, b"part1part2");
    }

    #[tokio::test]
    async fn test_list_by_prefix() {
        let b = MockS3Backend::new();
        b.put("bucket/a", b"1", &ObjectMetadata::new())
            .await
            .unwrap();
        b.put("bucket/b", b"2", &ObjectMetadata::new())
            .await
            .unwrap();
        b.put("other/x", b"3", &ObjectMetadata::new())
            .await
            .unwrap();
        let items = b.list("bucket/").await.unwrap();
        assert_eq!(items.len(), 2);
    }

    #[tokio::test]
    async fn test_exists() {
        let b = MockS3Backend::new();
        assert!(!b.exists("b/k").await.unwrap());
        b.put("b/k", b"", &ObjectMetadata::new()).await.unwrap();
        assert!(b.exists("b/k").await.unwrap());
    }

    #[tokio::test]
    async fn test_backend_type() {
        let b = MockS3Backend::new();
        assert_eq!(b.backend_type(), BackendType::S3);
    }
}
