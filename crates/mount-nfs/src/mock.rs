use crate::error::MountError;
use crate::traits::{
    BackendType, FileMetadata, MountBackend, MountEntry, MountHandle, MountOptions, SpaceUsage,
};
use async_trait::async_trait;
use chrono::Utc;
use dashmap::DashMap;

#[derive(Debug)]
pub struct MockBackend {
    store: DashMap<String, Vec<(String, Vec<u8>)>>,
}

impl MockBackend {
    pub fn new() -> Self {
        Self {
            store: DashMap::new(),
        }
    }

    pub fn add_file(&self, path: &str, name: &str, content: Vec<u8>) {
        self.store
            .entry(path.to_string())
            .or_default()
            .push((name.to_string(), content));
    }
}

impl Default for MockBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MountBackend for MockBackend {
    async fn mount(
        &self,
        remote_path: &str,
        local_path: &str,
        _options: &MountOptions,
    ) -> Result<MountHandle, MountError> {
        if self.store.contains_key(remote_path) {
            Ok(MountHandle::new(remote_path, local_path, BackendType::WebDav))
        } else {
            Err(MountError::NotFound {
                path: remote_path.to_string(),
            })
        }
    }

    async fn unmount(&self, handle: &MountHandle) -> Result<(), MountError> {
        if self.store.remove(&handle.remote_path).is_some() {
            Ok(())
        } else {
            Err(MountError::NotMounted {
                mount_point: handle.local_path.clone(),
            })
        }
    }

    async fn read_dir(
        &self,
        _handle: &MountHandle,
        path: &str,
    ) -> Result<Vec<MountEntry>, MountError> {
        let entries = self.store.get(path).ok_or_else(|| MountError::NotFound {
            path: path.to_string(),
        })?;

        Ok(entries
            .iter()
            .map(|(name, content)| MountEntry {
                name: name.clone(),
                is_dir: false,
                size: content.len() as u64,
                modified: Utc::now(),
            })
            .collect())
    }

    async fn read_file(
        &self,
        _handle: &MountHandle,
        path: &str,
        offset: u64,
        length: u64,
    ) -> Result<Vec<u8>, MountError> {
        let (dir, file_name) = path.rsplit_once('/').ok_or_else(|| MountError::NotFound {
            path: path.to_string(),
        })?;

        let entries = self.store.get(dir).ok_or_else(|| MountError::NotFound {
            path: path.to_string(),
        })?;

        for (name, content) in entries.iter() {
            if name == file_name {
                let start = offset as usize;
                if start >= content.len() {
                    return Ok(Vec::new());
                }
                let end = ((offset + length) as usize).min(content.len());
                return Ok(content[start..end].to_vec());
            }
        }

        Err(MountError::NotFound {
            path: path.to_string(),
        })
    }

    async fn metadata(
        &self,
        handle: &MountHandle,
        path: &str,
    ) -> Result<FileMetadata, MountError> {
        let entries = self.store.get(path).ok_or_else(|| MountError::NotMounted {
            mount_point: handle.local_path.clone(),
        })?;

        let now = Utc::now();
        Ok(FileMetadata {
            size: entries.iter().map(|(_, c)| c.len() as u64).sum(),
            modified: now,
            created: now,
            is_dir: true,
            permissions: 0o755,
        })
    }

    async fn space_usage(&self, _handle: &MountHandle) -> Result<SpaceUsage, MountError> {
        let mut used: u64 = 0;
        for entry in self.store.iter() {
            for (_, content) in entry.value().iter() {
                used += content.len() as u64;
            }
        }

        let total = used * 2;
        Ok(SpaceUsage {
            total_bytes: total,
            used_bytes: used,
            available_bytes: total - used,
        })
    }

    fn backend_type(&self) -> BackendType {
        BackendType::WebDav
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mount_unmount() {
        let backend = MockBackend::new();
        backend.add_file("/data", "file1.txt", b"hello".to_vec());

        let handle = backend
            .mount("/data", "/mnt/data", &MountOptions::default())
            .await
            .unwrap();

        assert_eq!(handle.remote_path, "/data");
        assert_eq!(handle.local_path, "/mnt/data");
        assert_eq!(handle.backend_type, BackendType::WebDav);

        backend.unmount(&handle).await.unwrap();
    }

    #[tokio::test]
    async fn test_read_dir_lists_files() {
        let backend = MockBackend::new();
        backend.add_file("/docs", "readme.md", b"# Docs".to_vec());
        backend.add_file("/docs", "notes.txt", b"Some notes".to_vec());

        let handle = backend
            .mount("/docs", "/mnt/docs", &MountOptions::default())
            .await
            .unwrap();

        let entries = backend.read_dir(&handle, "/docs").await.unwrap();
        assert_eq!(entries.len(), 2);

        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"readme.md"));
        assert!(names.contains(&"notes.txt"));
    }

    #[tokio::test]
    async fn test_read_file_with_offset_and_length() {
        let backend = MockBackend::new();
        let content = b"Hello, World!";
        backend.add_file("/data", "test.txt", content.to_vec());

        let handle = backend
            .mount("/data", "/mnt/data", &MountOptions::default())
            .await
            .unwrap();

        let result = backend
            .read_file(&handle, "/data/test.txt", 7, 5)
            .await
            .unwrap();
        assert_eq!(result, b"World");
    }

    #[tokio::test]
    async fn test_read_file_offset_beyond_content() {
        let backend = MockBackend::new();
        backend.add_file("/data", "small.txt", b"hi".to_vec());

        let handle = backend
            .mount("/data", "/mnt/data", &MountOptions::default())
            .await
            .unwrap();

        let result = backend
            .read_file(&handle, "/data/small.txt", 100, 10)
            .await
            .unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_nested_directories() {
        let backend = MockBackend::new();
        backend.add_file("/parent/child", "deep.txt", b"nested content".to_vec());

        let handle = backend
            .mount("/parent/child", "/mnt/nested", &MountOptions::default())
            .await
            .unwrap();

        let entries = backend.read_dir(&handle, "/parent/child").await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "deep.txt");
        assert_eq!(entries[0].size, 14);
    }

    #[tokio::test]
    async fn test_read_nonexistent_file_returns_not_found() {
        let backend = MockBackend::new();

        let result = backend
            .mount("/nope", "/mnt/nope", &MountOptions::default())
            .await;
        assert!(result.is_err());
        match result.unwrap_err() {
            MountError::NotFound { path } => assert_eq!(path, "/nope"),
            other => panic!("expected NotFound, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_unmount_then_read_returns_not_mounted() {
        let backend = MockBackend::new();
        backend.add_file("/tmp", "file.txt", b"data".to_vec());

        let handle = backend
            .mount("/tmp", "/mnt/tmp", &MountOptions::default())
            .await
            .unwrap();

        backend.unmount(&handle).await.unwrap();

        let result = backend.read_dir(&handle, "/tmp").await;
        assert!(result.is_err());
        match result.unwrap_err() {
            MountError::NotFound { .. } => {}
            other => panic!("expected NotFound after unmount, got: {:?}", other),
        }
    }
}
