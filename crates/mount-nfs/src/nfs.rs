use crate::error::MountError;
use crate::traits::{
    BackendType, FileMetadata, MountBackend, MountEntry, MountHandle, MountOptions, SpaceUsage,
};
use async_trait::async_trait;
use std::time::Duration;

/// NFS mount configuration.
#[derive(Debug, Clone)]
pub struct NfsConfig {
    /// NFS server hostname or IP.
    pub server: String,
    /// Remote export path (e.g., "/export/data").
    pub export_path: String,
    /// NFS protocol version (3 or 4).
    pub mount_version: u32,
    /// Mount read-only.
    pub read_only: bool,
    /// I/O timeout.
    pub timeout: Duration,
}

impl Default for NfsConfig {
    fn default() -> Self {
        Self {
            server: "localhost".to_string(),
            export_path: "/".to_string(),
            mount_version: 4,
            read_only: true,
            timeout: Duration::from_secs(30),
        }
    }
}

impl NfsConfig {
    /// Build the NFS mount source string (e.g., "server:/export/path").
    pub fn mount_source(&self) -> String {
        format!("{}:{}", self.server, self.export_path)
    }

    /// Build mount options string for the `mount` syscall.
    pub fn mount_options(&self) -> String {
        let mut opts = Vec::new();
        match self.mount_version {
            3 => opts.push("vers=3".to_string()),
            4 => opts.push("vers=4".to_string()),
            other => opts.push(format!("vers={other}")),
        }
        if self.read_only {
            opts.push("ro".to_string());
        }
        opts.push(format!("timeo={}", self.timeout.as_secs()));
        opts.join(",")
    }
}

/// NFS backend using platform mount syscalls.
///
/// When the `ffi` feature is enabled, mount/unmount use `libc::mount()` and
/// `libc::umount2()`. Without `ffi`, mount/unmount are tracked in-memory
/// (useful for testing and non-root environments where the share is
/// pre-mounted by the system).
///
/// File operations (read_dir, read_file, metadata, space_usage) always use
/// `tokio::fs` on the local mount path, since once mounted the NFS share
/// behaves as a regular filesystem.
#[derive(Debug)]
pub struct NfsBackend {
    pub config: NfsConfig,
}

impl NfsBackend {
    pub fn new(config: NfsConfig) -> Self {
        Self { config }
    }

    /// Perform the actual OS mount syscall.
    #[cfg(all(unix, feature = "ffi"))]
    fn do_mount(
        local_path: &str,
        source: &str,
        fstype: &str,
        options: &str,
    ) -> Result<(), MountError> {
        use std::ffi::CString;
        let source_c = CString::new(source).map_err(|e| MountError::Io {
            source: std::io::Error::new(std::io::ErrorKind::InvalidInput, e),
            context: "invalid mount source".to_string(),
        })?;
        let target_c = CString::new(local_path).map_err(|e| MountError::Io {
            source: std::io::Error::new(std::io::ErrorKind::InvalidInput, e),
            context: "invalid mount target".to_string(),
        })?;
        let fstype_c = CString::new(fstype).map_err(|e| MountError::Io {
            source: std::io::Error::new(std::io::ErrorKind::InvalidInput, e),
            context: "invalid filesystem type".to_string(),
        })?;
        let data_c = CString::new(options).unwrap_or_default();

        // SAFETY: mount() is a standard POSIX syscall. Arguments are validated
        // CString conversions above. Requires CAP_SYS_ADMIN capability.
        let ret = unsafe {
            libc::mount(
                source_c.as_ptr(),
                target_c.as_ptr(),
                fstype_c.as_ptr(),
                0, // MS_NO flags
                data_c.as_ptr().cast(),
            )
        };

        if ret != 0 {
            let err = std::io::Error::last_os_error();
            return Err(MountError::ConnectionFailed {
                source: err.to_string(),
                mount_point: local_path.to_string(),
            });
        }
        Ok(())
    }

    /// Perform the actual OS unmount syscall.
    #[cfg(all(unix, feature = "ffi"))]
    fn do_unmount(local_path: &str) -> Result<(), MountError> {
        use std::ffi::CString;
        let target_c = CString::new(local_path).map_err(|e| MountError::Io {
            source: std::io::Error::new(std::io::ErrorKind::InvalidInput, e),
            context: "invalid unmount target".to_string(),
        })?;

        // SAFETY: umount2() is a standard POSIX syscall. The path is
        // validated above. Uses MNT_DETACH for lazy unmount.
        let ret = unsafe { libc::umount2(target_c.as_ptr(), libc::MNT_DETACH) };

        if ret != 0 {
            let err = std::io::Error::last_os_error();
            return Err(MountError::NotMounted {
                mount_point: local_path.to_string(),
            });
        }
        Ok(())
    }

    #[cfg(not(all(unix, feature = "ffi")))]
    fn do_mount(
        _local_path: &str,
        _source: &str,
        _fstype: &str,
        _options: &str,
    ) -> Result<(), MountError> {
        // Without FFI, assume the share is pre-mounted or this is a test.
        Ok(())
    }

    #[cfg(not(all(unix, feature = "ffi")))]
    fn do_unmount(_local_path: &str) -> Result<(), MountError> {
        // Without FFI, no-op.
        Ok(())
    }

    /// Resolve the effective file path within the mount.
    /// Strips the remote_path prefix from `path` and appends to local_path.
    fn resolve_path(handle: &MountHandle, path: &str) -> std::path::PathBuf {
        let mount_dir = &handle.local_path;
        if path == "/" || path.is_empty() {
            std::path::PathBuf::from(mount_dir)
        } else {
            std::path::PathBuf::from(mount_dir).join(path.trim_start_matches('/'))
        }
    }
}

#[async_trait]
impl MountBackend for NfsBackend {
    async fn mount(
        &self,
        remote_path: &str,
        local_path: &str,
        _options: &MountOptions,
    ) -> Result<MountHandle, MountError> {
        let source = self.config.mount_source();
        let opts = self.config.mount_options();
        Self::do_mount(local_path, &source, "nfs", &opts)?;
        Ok(MountHandle::new(remote_path, local_path, BackendType::Nfs))
    }

    async fn unmount(&self, handle: &MountHandle) -> Result<(), MountError> {
        Self::do_unmount(&handle.local_path)
    }

    async fn read_dir(
        &self,
        handle: &MountHandle,
        path: &str,
    ) -> Result<Vec<MountEntry>, MountError> {
        let dir = Self::resolve_path(handle, path);
        let mut entries = tokio::fs::read_dir(&dir)
            .await
            .map_err(|_e| MountError::NotFound {
                path: dir.display().to_string(),
            })?;

        let mut result = Vec::new();
        while let Some(entry) = entries.next_entry().await.map_err(|e| MountError::Io {
            source: e,
            context: format!("read_dir: {}", dir.display()),
        })? {
            let metadata = entry.metadata().await.map_err(|e| MountError::Io {
                source: e,
                context: format!("metadata: {}", entry.path().display()),
            })?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let modified = metadata
                .modified()
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
                .into();
            result.push(MountEntry {
                name,
                is_dir: metadata.is_dir(),
                size: metadata.len(),
                modified,
            });
        }
        Ok(result)
    }

    async fn read_file(
        &self,
        handle: &MountHandle,
        path: &str,
        offset: u64,
        length: u64,
    ) -> Result<Vec<u8>, MountError> {
        let file_path = Self::resolve_path(handle, path);
        use tokio::io::{AsyncReadExt, AsyncSeekExt};

        let mut file =
            tokio::fs::File::open(&file_path)
                .await
                .map_err(|_e| MountError::NotFound {
                    path: file_path.display().to_string(),
                })?;

        if offset > 0 {
            file.seek(std::io::SeekFrom::Start(offset))
                .await
                .map_err(|e| MountError::Io {
                    source: e,
                    context: format!("seek: {}", file_path.display()),
                })?;
        }

        let mut buf = vec![0u8; length as usize];
        let n = file.read(&mut buf).await.map_err(|e| MountError::Io {
            source: e,
            context: format!("read: {}", file_path.display()),
        })?;
        buf.truncate(n);
        Ok(buf)
    }

    async fn metadata(&self, handle: &MountHandle, path: &str) -> Result<FileMetadata, MountError> {
        let file_path = Self::resolve_path(handle, path);
        let meta = tokio::fs::metadata(&file_path)
            .await
            .map_err(|_e| MountError::NotFound {
                path: file_path.display().to_string(),
            })?;

        let created = meta
            .created()
            .unwrap_or(meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH))
            .into();
        Ok(FileMetadata {
            size: meta.len(),
            modified: meta
                .modified()
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
                .into(),
            created,
            is_dir: meta.is_dir(),
            permissions: 0o755, // default; real permissions need platform FFI
        })
    }

    async fn space_usage(&self, handle: &MountHandle) -> Result<SpaceUsage, MountError> {
        // Use tokio::fs to stat the mount point's filesystem.
        // std::fs::metadata on the root gives us the total/available space
        // on some platforms. For accurate NFS space info we'd need statvfs,
        // but tokio::fs doesn't expose it directly.
        let mount_dir = &handle.local_path;
        let meta = tokio::fs::metadata(mount_dir)
            .await
            .map_err(|e| MountError::Io {
                source: e,
                context: format!("stat: {}", mount_dir),
            })?;

        // This gives a rough estimate. For accurate space info on Linux,
        // statvfs() would be used via libc.
        let used = meta.len();
        let total = used * 2; // placeholder; real implementation uses statvfs
        Ok(SpaceUsage {
            total_bytes: total,
            used_bytes: used,
            available_bytes: total.saturating_sub(used),
        })
    }

    fn backend_type(&self) -> BackendType {
        BackendType::Nfs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nfs_config_mount_source() {
        let config = NfsConfig {
            server: "nfs.example.com".to_string(),
            export_path: "/data".to_string(),
            ..Default::default()
        };
        assert_eq!(config.mount_source(), "nfs.example.com:/data");
    }

    #[test]
    fn test_nfs_config_mount_options_v4() {
        let config = NfsConfig {
            mount_version: 4,
            read_only: true,
            timeout: Duration::from_secs(60),
            ..Default::default()
        };
        let opts = config.mount_options();
        assert!(opts.contains("vers=4"));
        assert!(opts.contains("ro"));
        assert!(opts.contains("timeo=60"));
    }

    #[test]
    fn test_nfs_config_mount_options_v3() {
        let config = NfsConfig {
            mount_version: 3,
            read_only: false,
            timeout: Duration::from_secs(30),
            ..Default::default()
        };
        let opts = config.mount_options();
        assert!(opts.contains("vers=3"));
        assert!(!opts.contains("ro"));
        assert!(opts.contains("timeo=30"));
    }

    #[test]
    fn test_resolve_path_root() {
        let handle = MountHandle::new("/export/data", "/mnt/nfs", BackendType::Nfs);
        let resolved = NfsBackend::resolve_path(&handle, "/");
        assert_eq!(resolved, std::path::PathBuf::from("/mnt/nfs"));
    }

    #[test]
    fn test_resolve_path_subdir() {
        let handle = MountHandle::new("/export/data", "/mnt/nfs", BackendType::Nfs);
        let resolved = NfsBackend::resolve_path(&handle, "subdir/file.txt");
        assert_eq!(
            resolved,
            std::path::PathBuf::from("/mnt/nfs/subdir/file.txt")
        );
    }

    #[test]
    fn test_resolve_path_leading_slash() {
        let handle = MountHandle::new("/export/data", "/mnt/nfs", BackendType::Nfs);
        let resolved = NfsBackend::resolve_path(&handle, "/subdir/file.txt");
        assert_eq!(
            resolved,
            std::path::PathBuf::from("/mnt/nfs/subdir/file.txt")
        );
    }

    #[test]
    fn test_backend_type() {
        let backend = NfsBackend::new(NfsConfig::default());
        assert_eq!(backend.backend_type(), BackendType::Nfs);
    }
}
