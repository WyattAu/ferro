use async_trait::async_trait;

use crate::error::StorageAdapterError;
use crate::memory::InMemoryBackend;

#[derive(Debug, Clone)]
pub struct MountInfo {
    pub remote: String,
    pub mount_point: String,
    pub mounted: bool,
}

#[async_trait]
pub trait NfsBackend: Send + Sync {
    async fn mount(&self, remote: &str, mount_point: &str) -> Result<(), StorageAdapterError>;
    async fn unmount(&self, mount_point: &str) -> Result<(), StorageAdapterError>;
    async fn is_mounted(&self, mount_point: &str) -> bool;
    async fn list_mounts(&self) -> Vec<MountInfo>;
}

pub struct MockNfsBackend {
    inner: InMemoryBackend,
    mounts: dashmap::DashMap<String, String>,
}

impl MockNfsBackend {
    pub fn new() -> Self {
        Self {
            inner: InMemoryBackend::new(),
            mounts: dashmap::DashMap::new(),
        }
    }

    pub fn storage(&self) -> &InMemoryBackend {
        &self.inner
    }
}

impl Default for MockNfsBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NfsBackend for MockNfsBackend {
    async fn mount(&self, remote: &str, mount_point: &str) -> Result<(), StorageAdapterError> {
        if self.mounts.contains_key(mount_point) {
            return Err(StorageAdapterError::MountFailed(format!(
                "already mounted at {mount_point}"
            )));
        }
        self.mounts
            .insert(mount_point.to_string(), remote.to_string());
        Ok(())
    }

    async fn unmount(&self, mount_point: &str) -> Result<(), StorageAdapterError> {
        self.mounts.remove(mount_point).map(|_| ()).ok_or_else(|| {
            StorageAdapterError::MountFailed(format!("not mounted at {mount_point}"))
        })
    }

    async fn is_mounted(&self, mount_point: &str) -> bool {
        self.mounts.contains_key(mount_point)
    }

    async fn list_mounts(&self) -> Vec<MountInfo> {
        self.mounts
            .iter()
            .map(|e| MountInfo {
                remote: e.value().clone(),
                mount_point: e.key().clone(),
                mounted: true,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::StorageBackend;

    #[tokio::test]
    async fn test_mount_unmount() {
        let nfs = MockNfsBackend::new();
        nfs.mount("server:/export", "/mnt/data").await.unwrap();
        assert!(nfs.is_mounted("/mnt/data").await);
        nfs.unmount("/mnt/data").await.unwrap();
        assert!(!nfs.is_mounted("/mnt/data").await);
    }

    #[tokio::test]
    async fn test_mount_already_mounted() {
        let nfs = MockNfsBackend::new();
        nfs.mount("s:/e", "/mnt").await.unwrap();
        let result = nfs.mount("s:/e2", "/mnt").await;
        assert!(matches!(result, Err(StorageAdapterError::MountFailed(_))));
    }

    #[tokio::test]
    async fn test_unmount_not_mounted() {
        let nfs = MockNfsBackend::new();
        let result = nfs.unmount("/nope").await;
        assert!(matches!(result, Err(StorageAdapterError::MountFailed(_))));
    }

    #[tokio::test]
    async fn test_list_mounts() {
        let nfs = MockNfsBackend::new();
        nfs.mount("a:/x", "/mnt/a").await.unwrap();
        nfs.mount("b:/y", "/mnt/b").await.unwrap();
        let mounts = nfs.list_mounts().await;
        assert_eq!(mounts.len(), 2);
        assert!(mounts.iter().all(|m| m.mounted));
    }

    #[tokio::test]
    async fn test_storage_through_mount() {
        let nfs = MockNfsBackend::new();
        let meta = crate::backend::ObjectMetadata::new();
        nfs.storage().put("test", b"data", &meta).await.unwrap();
        assert_eq!(nfs.storage().get("test").await.unwrap(), b"data");
    }
}
