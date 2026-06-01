use std::collections::HashMap;

#[cfg(feature = "smb")]
use std::time::Duration;

use async_trait::async_trait;
#[cfg(feature = "smb")]
use chrono::{DateTime, Utc};

#[cfg(feature = "smb")]
use crate::backend::{BackendType, ObjectInfo, ObjectMetadata, StorageBackend};
#[cfg(feature = "smb")]
use crate::error::StorageAdapterError;

pub fn normalize_to_smb(path: &str) -> String {
    let path = path.replace('\\', "/");
    let path = path.trim_start_matches('/').to_string();
    path.trim_end_matches('/').to_string()
}

#[cfg(feature = "smb")]
fn filetime_to_datetime(ft: smb2::pack::FileTime) -> Option<DateTime<Utc>> {
    ft.to_system_time()
        .map(DateTime::<Utc>::from)
}

#[cfg(feature = "smb")]
fn map_smb_error(e: smb2::Error, context: &str) -> StorageAdapterError {
    match e.kind() {
        smb2::ErrorKind::NotFound => StorageAdapterError::NotFound(context.to_string()),
        smb2::ErrorKind::AccessDenied => {
            StorageAdapterError::PermissionDenied(context.to_string())
        }
        smb2::ErrorKind::DiskFull => StorageAdapterError::QuotaExceeded(context.to_string()),
        smb2::ErrorKind::ConnectionLost => {
            StorageAdapterError::ConnectionFailed(context.to_string())
        }
        smb2::ErrorKind::AuthRequired => {
            StorageAdapterError::PermissionDenied(context.to_string())
        }
        _ => StorageAdapterError::IoError(std::io::Error::other(e.to_string())),
    }
}

#[cfg(feature = "smb")]
pub struct Smb2StorageBackend {
    client: tokio::sync::Mutex<smb2::SmbClient>,
    #[allow(dead_code)]
    share_name: String,
    tree: tokio::sync::Mutex<smb2::Tree>,
}

#[cfg(feature = "smb")]
impl Smb2StorageBackend {
    pub async fn new(
        server: &str,
        share: &str,
        username: &str,
        password: &str,
    ) -> Result<Self, StorageAdapterError> {
        Self::with_domain(server, share, username, password, "").await
    }

    pub async fn with_domain(
        server: &str,
        share: &str,
        username: &str,
        password: &str,
        domain: &str,
    ) -> Result<Self, StorageAdapterError> {
        let addr = if server.contains(':') {
            server.to_string()
        } else {
            format!("{server}:445")
        };
        let config = smb2::ClientConfig {
            addr,
            timeout: Duration::from_secs(30),
            username: username.to_string(),
            password: password.to_string(),
            domain: domain.to_string(),
            auto_reconnect: true,
            compression: true,
            dfs_enabled: true,
            dfs_target_overrides: HashMap::new(),
        };
        let mut client = smb2::SmbClient::connect(config)
            .await
            .map_err(|e| StorageAdapterError::ConnectionFailed(e.to_string()))?;
        let tree = client
            .connect_share(share)
            .await
            .map_err(|e| StorageAdapterError::ConnectionFailed(e.to_string()))?;
        Ok(Self {
            client: tokio::sync::Mutex::new(client),
            tree: tokio::sync::Mutex::new(tree),
            share_name: share.to_string(),
        })
    }
}

#[cfg(feature = "smb")]
#[async_trait]
impl StorageBackend for Smb2StorageBackend {
    async fn get(&self, path: &str) -> Result<Vec<u8>, StorageAdapterError> {
        let smb_path = normalize_to_smb(path);
        if smb_path.is_empty() {
            return Err(StorageAdapterError::InvalidPath(
                "root directory is not a file".into(),
            ));
        }
        let ctx = format!("get {path}");
        let mut client = self.client.lock().await;
        let tree = self.tree.lock().await;
        tree.read_file(client.connection_mut(), &smb_path)
            .await
            .map_err(|e| map_smb_error(e, &ctx))
    }

    async fn put(
        &self,
        path: &str,
        data: &[u8],
        _metadata: &ObjectMetadata,
    ) -> Result<(), StorageAdapterError> {
        let smb_path = normalize_to_smb(path);
        if smb_path.is_empty() {
            return Err(StorageAdapterError::InvalidPath(
                "cannot write to root directory".into(),
            ));
        }
        let ctx = format!("put {path}");
        let mut client = self.client.lock().await;
        let tree = self.tree.lock().await;
        tree.write_file(client.connection_mut(), &smb_path, data)
            .await
            .map_err(|e| map_smb_error(e, &ctx))?;
        Ok(())
    }

    async fn delete(&self, path: &str) -> Result<(), StorageAdapterError> {
        let smb_path = normalize_to_smb(path);
        if smb_path.is_empty() {
            return Err(StorageAdapterError::InvalidPath(
                "cannot delete root directory".into(),
            ));
        }
        let ctx = format!("delete {path}");
        let mut client = self.client.lock().await;
        let tree = self.tree.lock().await;
        let result = tree.delete_file(client.connection_mut(), &smb_path).await;
        if let Err(e) = result {
            if matches!(e.kind(), smb2::ErrorKind::IsADirectory) {
                tree.delete_directory(client.connection_mut(), &smb_path)
                    .await
                    .map_err(|e2| map_smb_error(e2, &ctx))?;
            } else {
                return Err(map_smb_error(e, &ctx));
            }
        }
        Ok(())
    }

    async fn exists(&self, path: &str) -> Result<bool, StorageAdapterError> {
        let smb_path = normalize_to_smb(path);
        let mut client = self.client.lock().await;
        let tree = self.tree.lock().await;
        match tree.stat(client.connection_mut(), &smb_path).await {
            Ok(_) => Ok(true),
            Err(e) => {
                if matches!(e.kind(), smb2::ErrorKind::NotFound) {
                    Ok(false)
                } else {
                    Err(map_smb_error(e, &format!("exists {path}")))
                }
            }
        }
    }

    async fn list(&self, prefix: &str) -> Result<Vec<ObjectInfo>, StorageAdapterError> {
        let smb_prefix = normalize_to_smb(prefix);
        let ctx = format!("list {prefix}");
        let mut client = self.client.lock().await;
        let tree = self.tree.lock().await;
        let dir_path = if smb_prefix.is_empty() || smb_prefix.ends_with('/') {
            smb_prefix
        } else {
            match tree.stat(client.connection_mut(), &smb_prefix).await {
                Ok(info) if info.is_directory => {
                    let mut p = smb_prefix.clone();
                    p.push('/');
                    p
                }
                _ => {
                    return Err(StorageAdapterError::InvalidPath(format!(
                        "prefix '{prefix}' is not a directory"
                    )));
                }
            }
        };
        let entries = tree
            .list_directory(client.connection_mut(), &dir_path)
            .await
            .map_err(|e| map_smb_error(e, &ctx))?;
        let mut results = Vec::new();
        for entry in entries {
            if entry.name == "." || entry.name == ".." {
                continue;
            }
            let full_path = if dir_path.is_empty() || dir_path == "/" {
                format!("/{prefix}{}", entry.name)
            } else {
                format!("/{dir_path}{}", entry.name)
            };
            results.push(ObjectInfo {
                path: full_path,
                size: entry.size,
                last_modified: filetime_to_datetime(entry.modified),
                content_type: None,
                etag: None,
                metadata: HashMap::new(),
            });
        }
        Ok(results)
    }

    async fn size(&self, path: &str) -> Result<u64, StorageAdapterError> {
        let smb_path = normalize_to_smb(path);
        let ctx = format!("size {path}");
        let mut client = self.client.lock().await;
        let tree = self.tree.lock().await;
        let info = tree
            .stat(client.connection_mut(), &smb_path)
            .await
            .map_err(|e| map_smb_error(e, &ctx))?;
        Ok(info.size)
    }

    async fn copy(&self, from: &str, to: &str) -> Result<(), StorageAdapterError> {
        let data = self.get(from).await?;
        let meta = ObjectMetadata::new();
        self.put(to, &data, &meta).await
    }

    async fn move_obj(&self, from: &str, to: &str) -> Result<(), StorageAdapterError> {
        let smb_from = normalize_to_smb(from);
        let smb_to = normalize_to_smb(to);
        if smb_from.is_empty() || smb_to.is_empty() {
            return Err(StorageAdapterError::InvalidPath(
                "cannot move root directory".into(),
            ));
        }
        let ctx = format!("move {from} -> {to}");
        let mut client = self.client.lock().await;
        let tree = self.tree.lock().await;
        tree.rename(client.connection_mut(), &smb_from, &smb_to)
            .await
            .map_err(|e| map_smb_error(e, &ctx))?;
        Ok(())
    }

    async fn metadata(&self, path: &str) -> Result<ObjectInfo, StorageAdapterError> {
        let smb_path = normalize_to_smb(path);
        let ctx = format!("metadata {path}");
        let mut client = self.client.lock().await;
        let tree = self.tree.lock().await;
        let info = tree
            .stat(client.connection_mut(), &smb_path)
            .await
            .map_err(|e| map_smb_error(e, &ctx))?;
        Ok(ObjectInfo {
            path: path.to_string(),
            size: info.size,
            last_modified: filetime_to_datetime(info.modified),
            content_type: None,
            etag: None,
            metadata: HashMap::new(),
        })
    }

    fn backend_type(&self) -> BackendType {
        BackendType::Smb
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_leading_slash() {
        assert_eq!(normalize_to_smb("/docs/file.txt"), "docs/file.txt");
    }

    #[test]
    fn test_normalize_root() {
        assert_eq!(normalize_to_smb("/"), "");
    }

    #[test]
    fn test_normalize_double_slash() {
        assert_eq!(
            normalize_to_smb("//server/share/file.txt"),
            "server/share/file.txt"
        );
    }

    #[test]
    fn test_normalize_trailing_slash() {
        assert_eq!(normalize_to_smb("/docs/"), "docs");
    }

    #[test]
    fn test_normalize_no_slash() {
        assert_eq!(normalize_to_smb("docs/file.txt"), "docs/file.txt");
    }

    #[test]
    fn test_normalize_backslash() {
        assert_eq!(normalize_to_smb("\\docs\\file.txt"), "docs/file.txt");
    }

    #[test]
    fn test_normalize_mixed_separators() {
        assert_eq!(normalize_to_smb("/docs\\sub/file.txt"), "docs/sub/file.txt");
    }

    #[cfg(feature = "smb")]
    #[test]
    fn test_map_smb_error_not_found() {
        use smb2::types::Command;
        use smb2::types::status::NtStatus;
        let err = smb2::Error::Protocol {
            status: NtStatus::NO_SUCH_FILE,
            command: Command::Create,
        };
        let mapped = map_smb_error(err, "get /foo");
        assert!(matches!(mapped, StorageAdapterError::NotFound(_)));
    }

    #[cfg(feature = "smb")]
    #[test]
    fn test_map_smb_error_access_denied() {
        use smb2::types::Command;
        use smb2::types::status::NtStatus;
        let err = smb2::Error::Protocol {
            status: NtStatus::ACCESS_DENIED,
            command: Command::Read,
        };
        let mapped = map_smb_error(err, "get /secret");
        assert!(matches!(
            mapped,
            StorageAdapterError::PermissionDenied(_)
        ));
    }

    #[cfg(feature = "smb")]
    #[test]
    fn test_map_smb_error_disk_full() {
        use smb2::types::Command;
        use smb2::types::status::NtStatus;
        let err = smb2::Error::Protocol {
            status: NtStatus::DISK_FULL,
            command: Command::Write,
        };
        let mapped = map_smb_error(err, "put /big");
        assert!(matches!(mapped, StorageAdapterError::QuotaExceeded(_)));
    }

    #[cfg(feature = "smb")]
    #[test]
    fn test_map_smb_error_connection_lost() {
        let err = smb2::Error::Disconnected;
        let mapped = map_smb_error(err, "read /x");
        assert!(matches!(
            mapped,
            StorageAdapterError::ConnectionFailed(_)
        ));
    }

    #[cfg(feature = "smb")]
    #[test]
    fn test_map_smb_error_other() {
        let err = smb2::Error::Timeout;
        let mapped = map_smb_error(err, "op");
        assert!(matches!(mapped, StorageAdapterError::IoError(_)));
    }
}
