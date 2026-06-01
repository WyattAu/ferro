use std::path::{Path, PathBuf};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tokio::fs;
use tokio::io::AsyncWriteExt;

use crate::backend::{BackendType, ObjectInfo, ObjectMetadata, StorageBackend};
use crate::error::StorageAdapterError;

fn validate_path(path: &str) -> Result<PathBuf, StorageAdapterError> {
    let path = Path::new(path);
    for component in path.components() {
        if let std::path::Component::ParentDir = component {
            return Err(StorageAdapterError::InvalidPath(
                "path traversal (..) is not allowed".into(),
            ));
        }
    }
    Ok(path.to_path_buf())
}

fn compute_etag(data: &[u8]) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

#[derive(Debug)]
pub struct LocalFsBackend {
    root: PathBuf,
}

impl LocalFsBackend {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn resolve(&self, path: &str) -> Result<PathBuf, StorageAdapterError> {
        let relative = validate_path(path)?;
        let resolved = self.root.join(relative);
        if !resolved.starts_with(&self.root) {
            return Err(StorageAdapterError::InvalidPath(
                "path escapes root directory".into(),
            ));
        }
        Ok(resolved)
    }
}

#[async_trait]
impl StorageBackend for LocalFsBackend {
    async fn get(&self, path: &str) -> Result<Vec<u8>, StorageAdapterError> {
        let resolved = self.resolve(path)?;
        fs::read(&resolved).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                StorageAdapterError::NotFound(path.to_string())
            } else if e.kind() == std::io::ErrorKind::PermissionDenied {
                StorageAdapterError::PermissionDenied(path.to_string())
            } else {
                StorageAdapterError::IoError(e)
            }
        })
    }

    async fn put(
        &self,
        path: &str,
        data: &[u8],
        _metadata: &ObjectMetadata,
    ) -> Result<(), StorageAdapterError> {
        let relative = validate_path(path)?;
        let resolved = self.root.join(&relative);
        if let Some(parent) = resolved.parent() {
            fs::create_dir_all(parent).await?;
        }
        let mut file = fs::File::create(&resolved).await?;
        file.write_all(data).await?;
        Ok(())
    }

    async fn delete(&self, path: &str) -> Result<(), StorageAdapterError> {
        let resolved = self.resolve(path)?;
        fs::remove_file(&resolved).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                StorageAdapterError::NotFound(path.to_string())
            } else {
                StorageAdapterError::IoError(e)
            }
        })
    }

    async fn exists(&self, path: &str) -> Result<bool, StorageAdapterError> {
        let resolved = self.resolve(path)?;
        Ok(fs::metadata(&resolved).await.is_ok())
    }

    async fn list(&self, prefix: &str) -> Result<Vec<ObjectInfo>, StorageAdapterError> {
        let relative = validate_path(prefix)?;
        let dir_path = self.root.join(&relative);
        if !dir_path.exists() {
            return Ok(Vec::new());
        }
        let mut entries = Vec::new();
        let mut read_dir = fs::read_dir(&dir_path).await?;
        while let Some(entry) = read_dir.next_entry().await? {
            let meta = entry.metadata().await?;
            if !meta.is_file() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            let full_path = if prefix.is_empty() {
                name
            } else {
                format!("{prefix}/{name}")
            };
            let data = fs::read(entry.path()).await.unwrap_or_default();
            entries.push(ObjectInfo {
                path: full_path,
                size: meta.len(),
                content_type: None,
                last_modified: Some(meta.modified().ok().map(DateTime::<Utc>::from).unwrap_or(Utc::now())),
                etag: Some(compute_etag(&data)),
                metadata: std::collections::HashMap::new(),
            });
        }
        Ok(entries)
    }

    async fn size(&self, path: &str) -> Result<u64, StorageAdapterError> {
        let resolved = self.resolve(path)?;
        let meta = fs::metadata(&resolved).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                StorageAdapterError::NotFound(path.to_string())
            } else {
                StorageAdapterError::IoError(e)
            }
        })?;
        Ok(meta.len())
    }

    async fn copy(&self, from: &str, to: &str) -> Result<(), StorageAdapterError> {
        let src = self.resolve(from)?;
        let relative = validate_path(to)?;
        let dst = self.root.join(&relative);
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent).await?;
        }
        fs::copy(&src, &dst).await?;
        Ok(())
    }

    async fn move_obj(&self, from: &str, to: &str) -> Result<(), StorageAdapterError> {
        self.copy(from, to).await?;
        self.delete(from).await
    }

    async fn metadata(&self, path: &str) -> Result<ObjectInfo, StorageAdapterError> {
        let resolved = self.resolve(path)?;
        let meta = fs::metadata(&resolved).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                StorageAdapterError::NotFound(path.to_string())
            } else {
                StorageAdapterError::IoError(e)
            }
        })?;
        let data = fs::read(&resolved).await.unwrap_or_default();
        Ok(ObjectInfo {
            path: path.to_string(),
            size: meta.len(),
            content_type: None,
            last_modified: Some(
                meta.modified()
                    .ok()
                    .map(DateTime::<Utc>::from)
                    .unwrap_or(Utc::now()),
            ),
            etag: Some(compute_etag(&data)),
            metadata: std::collections::HashMap::new(),
        })
    }

    fn backend_type(&self) -> BackendType {
        BackendType::Local
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn temp_backend() -> (LocalFsBackend, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let backend = LocalFsBackend::new(dir.path());
        (backend, dir)
    }

    #[tokio::test]
    async fn test_put_get_delete() {
        let (b, _dir) = temp_backend().await;
        let meta = ObjectMetadata::new();
        b.put("hello.txt", b"world", &meta).await.unwrap();
        let data = b.get("hello.txt").await.unwrap();
        assert_eq!(data, b"world");
        b.delete("hello.txt").await.unwrap();
        assert!(matches!(b.get("hello.txt").await, Err(StorageAdapterError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_exists() {
        let (b, _dir) = temp_backend().await;
        assert!(!b.exists("missing.txt").await.unwrap());
        b.put("present.txt", b"data", &ObjectMetadata::new()).await.unwrap();
        assert!(b.exists("present.txt").await.unwrap());
    }

    #[tokio::test]
    async fn test_list() {
        let (b, _dir) = temp_backend().await;
        let meta = ObjectMetadata::new();
        b.put("a/1.txt", b"1", &meta).await.unwrap();
        b.put("a/2.txt", b"22", &meta).await.unwrap();
        let items = b.list("a").await.unwrap();
        assert_eq!(items.len(), 2);
        let paths: Vec<&str> = items.iter().map(|i| i.path.as_str()).collect();
        assert!(paths.contains(&"a/1.txt"));
        assert!(paths.contains(&"a/2.txt"));
    }

    #[tokio::test]
    async fn test_path_traversal_rejected() {
        let (b, _dir) = temp_backend().await;
        let meta = ObjectMetadata::new();
        let result = b.put("../escape.txt", b"evil", &meta).await;
        assert!(matches!(result, Err(StorageAdapterError::InvalidPath(_))));
    }

    #[tokio::test]
    async fn test_size() {
        let (b, _dir) = temp_backend().await;
        b.put("sized.txt", b"12345", &ObjectMetadata::new()).await.unwrap();
        assert_eq!(b.size("sized.txt").await.unwrap(), 5);
    }

    #[tokio::test]
    async fn test_metadata() {
        let (b, _dir) = temp_backend().await;
        b.put("meta.txt", b"info", &ObjectMetadata::new()).await.unwrap();
        let info = b.metadata("meta.txt").await.unwrap();
        assert_eq!(info.path, "meta.txt");
        assert_eq!(info.size, 4);
        assert!(info.etag.is_some());
    }

    #[tokio::test]
    async fn test_copy() {
        let (b, _dir) = temp_backend().await;
        b.put("src.txt", b"copy", &ObjectMetadata::new()).await.unwrap();
        b.copy("src.txt", "dst.txt").await.unwrap();
        assert_eq!(b.get("dst.txt").await.unwrap(), b"copy");
    }

    #[tokio::test]
    async fn test_move_obj() {
        let (b, _dir) = temp_backend().await;
        b.put("m.txt", b"move", &ObjectMetadata::new()).await.unwrap();
        b.move_obj("m.txt", "m2.txt").await.unwrap();
        assert!(!b.exists("m.txt").await.unwrap());
        assert_eq!(b.get("m2.txt").await.unwrap(), b"move");
    }

    #[tokio::test]
    async fn test_backend_type() {
        let (b, _dir) = temp_backend().await;
        assert_eq!(b.backend_type(), BackendType::Local);
    }
}
