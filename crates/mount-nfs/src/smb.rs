use crate::error::MountError;
use crate::traits::{
    BackendType, Credentials, FileMetadata, MountBackend, MountEntry, MountHandle, MountOptions,
    SpaceUsage,
};
use async_trait::async_trait;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct SmbConfig {
    pub server: String,
    pub share_name: String,
    pub credentials: Option<Credentials>,
    pub read_only: bool,
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

#[derive(Debug)]
pub struct SmbBackend {
    pub config: SmbConfig,
}

impl SmbBackend {
    pub fn new(config: SmbConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl MountBackend for SmbBackend {
    async fn mount(
        &self,
        remote_path: &str,
        local_path: &str,
        _options: &MountOptions,
    ) -> Result<MountHandle, MountError> {
        Ok(MountHandle::new(remote_path, local_path, BackendType::Smb))
    }

    async fn unmount(&self, _handle: &MountHandle) -> Result<(), MountError> {
        Ok(())
    }

    async fn read_dir(
        &self,
        _handle: &MountHandle,
        _path: &str,
    ) -> Result<Vec<MountEntry>, MountError> {
        Err(MountError::Unsupported {
            feature: "SMB read_dir requires platform FFI".to_string(),
        })
    }

    async fn read_file(
        &self,
        _handle: &MountHandle,
        _path: &str,
        _offset: u64,
        _length: u64,
    ) -> Result<Vec<u8>, MountError> {
        Err(MountError::Unsupported {
            feature: "SMB read_file requires platform FFI".to_string(),
        })
    }

    async fn metadata(
        &self,
        _handle: &MountHandle,
        _path: &str,
    ) -> Result<FileMetadata, MountError> {
        Err(MountError::Unsupported {
            feature: "SMB metadata requires platform FFI".to_string(),
        })
    }

    async fn space_usage(&self, _handle: &MountHandle) -> Result<SpaceUsage, MountError> {
        Err(MountError::Unsupported {
            feature: "SMB space_usage requires platform FFI".to_string(),
        })
    }

    fn backend_type(&self) -> BackendType {
        BackendType::Smb
    }
}
