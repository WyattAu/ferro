use crate::error::MountError;
use crate::traits::{
    BackendType, FileMetadata, MountBackend, MountEntry, MountHandle, MountOptions, SpaceUsage,
};
use async_trait::async_trait;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct NfsConfig {
    pub server: String,
    pub export_path: String,
    pub mount_version: u32,
    pub read_only: bool,
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

#[derive(Debug)]
pub struct NfsBackend {
    pub config: NfsConfig,
}

impl NfsBackend {
    pub fn new(config: NfsConfig) -> Self {
        Self { config }
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
        Ok(MountHandle::new(remote_path, local_path, BackendType::Nfs))
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
            feature: "NFS read_dir requires platform FFI".to_string(),
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
            feature: "NFS read_file requires platform FFI".to_string(),
        })
    }

    async fn metadata(
        &self,
        _handle: &MountHandle,
        _path: &str,
    ) -> Result<FileMetadata, MountError> {
        Err(MountError::Unsupported {
            feature: "NFS metadata requires platform FFI".to_string(),
        })
    }

    async fn space_usage(&self, _handle: &MountHandle) -> Result<SpaceUsage, MountError> {
        Err(MountError::Unsupported {
            feature: "NFS space_usage requires platform FFI".to_string(),
        })
    }

    fn backend_type(&self) -> BackendType {
        BackendType::Nfs
    }
}
