use crate::error::MountError;
use crate::traits::{
    BackendType, Credentials, FileMetadata, MountBackend, MountEntry, MountHandle, MountOptions, SpaceUsage,
};
use async_trait::async_trait;
use std::time::Duration;

/// SMB/CIFS mount configuration.
#[derive(Debug, Clone)]
pub struct SmbConfig {
    /// SMB server hostname or IP.
    pub server: String,
    /// Share name (e.g., "share" for `//server/share`).
    pub share_name: String,
    /// Authentication credentials.
    pub credentials: Option<Credentials>,
    /// Mount read-only.
    pub read_only: bool,
    /// I/O timeout.
    pub timeout: Duration,
}

impl Default for SmbConfig {
    fn default() -> Self {
        Self {
            server: "localhost".to_string(),
            share_name: "share".to_string(),
            credentials: None,
            read_only: true,
            timeout: Duration::from_secs(30),
        }
    }
}

impl SmbConfig {
    /// Build the SMB mount source (UNC path), e.g., "//server/share".
    pub fn mount_source(&self) -> String {
        format!("//{}/{}", self.server, self.share_name)
    }

    /// Build mount options string for the `mount` syscall.
    pub fn mount_options(&self) -> String {
        let mut opts = Vec::new();

        // Credentials
        if let Some(creds) = &self.credentials {
            opts.push(format!("username={}", creds.username));
            opts.push(format!("password={}", creds.password));
            if let Some(domain) = &creds.domain {
                opts.push(format!("domain={domain}"));
            }
        } else {
            opts.push("guest".to_string());
        }

        // Mount flags
        if self.read_only {
            opts.push("ro".to_string());
        }
        opts.push("iocharset=utf8".to_string());
        opts.push(format!("timeo={}", self.timeout.as_secs()));

        opts.join(",")
    }
}

/// SMB/CIFS backend using platform mount syscalls.
///
/// When the `ffi` feature is enabled, mount/unmount use `libc::mount()` and
/// `libc::umount2()` with the `cifs` filesystem type. Without `ffi`, these
/// are no-ops (for testing and non-root environments).
///
/// File operations use standard `tokio::fs` on the mounted local path.
#[derive(Debug)]
pub struct SmbBackend {
    pub config: SmbConfig,
}

impl SmbBackend {
    pub fn new(config: SmbConfig) -> Self {
        Self { config }
    }

    /// Perform the actual OS mount syscall for CIFS.
    #[cfg(all(unix, feature = "ffi"))]
    fn do_mount(local_path: &str, source: &str, options: &str) -> Result<(), MountError> {
        use std::ffi::CString;
        let source_c = CString::new(source).map_err(|e| MountError::Io {
            source: std::io::Error::new(std::io::ErrorKind::InvalidInput, e),
            context: "invalid mount source".to_string(),
        })?;
        let target_c = CString::new(local_path).map_err(|e| MountError::Io {
            source: std::io::Error::new(std::io::ErrorKind::InvalidInput, e),
            context: "invalid mount target".to_string(),
        })?;
        let fstype_c = CString::new("cifs").unwrap(); // static, infallible
        let data_c = CString::new(options).unwrap_or_default();

        // SAFETY: mount() is a standard POSIX syscall. Arguments are validated
        // via CString above. Requires CAP_SYS_ADMIN capability.
        let ret = unsafe {
            libc::mount(
                source_c.as_ptr(),
                target_c.as_ptr(),
                fstype_c.as_ptr(),
                0,
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

        // SAFETY: umount2() with MNT_DETACH for lazy unmount.
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
    fn do_mount(_local_path: &str, _source: &str, _options: &str) -> Result<(), MountError> {
        Ok(())
    }

    #[cfg(not(all(unix, feature = "ffi")))]
    fn do_unmount(_local_path: &str) -> Result<(), MountError> {
        Ok(())
    }

    /// Resolve the effective file path within the mount.
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
impl MountBackend for SmbBackend {
    async fn mount(
        &self,
        _remote_path: &str,
        local_path: &str,
        _options: &MountOptions,
    ) -> Result<MountHandle, MountError> {
        let source = self.config.mount_source();
        let opts = self.config.mount_options();
        Self::do_mount(local_path, &source, &opts)?;
        Ok(MountHandle::new(
            &self.config.mount_source(),
            local_path,
            BackendType::Smb,
        ))
    }

    async fn unmount(&self, handle: &MountHandle) -> Result<(), MountError> {
        Self::do_unmount(&handle.local_path)
    }

    async fn read_dir(&self, handle: &MountHandle, path: &str) -> Result<Vec<MountEntry>, MountError> {
        let dir = Self::resolve_path(handle, path);
        let mut entries = tokio::fs::read_dir(&dir).await.map_err(|_e| MountError::NotFound {
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
            let modified = metadata.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH).into();
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

        let mut file = tokio::fs::File::open(&file_path)
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
            modified: meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH).into(),
            created,
            is_dir: meta.is_dir(),
            permissions: 0o755,
        })
    }

    async fn space_usage(&self, handle: &MountHandle) -> Result<SpaceUsage, MountError> {
        let mount_dir = &handle.local_path;
        let meta = tokio::fs::metadata(mount_dir).await.map_err(|e| MountError::Io {
            source: e,
            context: format!("stat: {}", mount_dir),
        })?;

        let used = meta.len();
        let total = used * 2;
        Ok(SpaceUsage {
            total_bytes: total,
            used_bytes: used,
            available_bytes: total.saturating_sub(used),
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
    fn test_smb_config_mount_source() {
        let config = SmbConfig {
            server: "smb.example.com".to_string(),
            share_name: "docs".to_string(),
            credentials: None,
            ..Default::default()
        };
        assert_eq!(config.mount_source(), "//smb.example.com/docs");
    }

    #[test]
    fn test_smb_config_mount_options_guest() {
        let config = SmbConfig {
            read_only: true,
            timeout: Duration::from_secs(45),
            ..Default::default()
        };
        let opts = config.mount_options();
        assert!(opts.contains("guest"));
        assert!(opts.contains("ro"));
        assert!(opts.contains("timeo=45"));
        assert!(opts.contains("iocharset=utf8"));
    }

    #[test]
    fn test_smb_config_mount_options_with_credentials() {
        let config = SmbConfig {
            credentials: Some(Credentials::with_domain("admin", "secret", "WORKGROUP")),
            read_only: false,
            timeout: Duration::from_secs(30),
            ..Default::default()
        };
        let opts = config.mount_options();
        assert!(opts.contains("username=admin"));
        assert!(opts.contains("password=secret"));
        assert!(opts.contains("domain=WORKGROUP"));
        assert!(!opts.contains("guest"));
        assert!(!opts.contains("ro"));
    }

    #[test]
    fn test_resolve_path_root() {
        let handle = MountHandle::new("//server/share", "/mnt/smb", BackendType::Smb);
        let resolved = SmbBackend::resolve_path(&handle, "/");
        assert_eq!(resolved, std::path::PathBuf::from("/mnt/smb"));
    }

    #[test]
    fn test_resolve_path_nested() {
        let handle = MountHandle::new("//server/share", "/mnt/smb", BackendType::Smb);
        let resolved = SmbBackend::resolve_path(&handle, "docs/report.pdf");
        assert_eq!(resolved, std::path::PathBuf::from("/mnt/smb/docs/report.pdf"));
    }

    #[test]
    fn test_backend_type() {
        let backend = SmbBackend::new(SmbConfig::default());
        assert_eq!(backend.backend_type(), BackendType::Smb);
    }
}
