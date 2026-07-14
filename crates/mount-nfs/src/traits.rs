use crate::error::MountError;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::Duration;
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackendType {
    Nfs,
    Smb,
    WebDav,
}

impl fmt::Display for BackendType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Nfs => write!(f, "nfs"),
            Self::Smb => write!(f, "smb"),
            Self::WebDav => write!(f, "webdav"),
        }
    }
}

#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct Credentials {
    pub username: String,
    pub password: String,
    pub domain: Option<String>,
}

impl std::fmt::Debug for Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Credentials")
            .field("username", &self.username)
            .field("password", &"[REDACTED]")
            .field("domain", &self.domain)
            .finish()
    }
}

impl Credentials {
    pub fn new(username: &str, password: &str) -> Self {
        Self {
            username: username.to_string(),
            password: password.to_string(),
            domain: None,
        }
    }

    pub fn with_domain(username: &str, password: &str, domain: &str) -> Self {
        Self {
            username: username.to_string(),
            password: password.to_string(),
            domain: Some(domain.to_string()),
        }
    }
}

pub struct Secret<T: fmt::Display>(pub T);

impl<T: fmt::Display> fmt::Display for Secret<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[REDACTED]")
    }
}

impl fmt::Debug for Secret<&str> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

impl fmt::Debug for Secret<String> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

#[derive(Debug, Clone)]
pub struct MountOptions {
    pub read_only: bool,
    pub version: Option<String>,
    pub timeout: Duration,
    pub credentials: Option<Credentials>,
}

impl Default for MountOptions {
    fn default() -> Self {
        Self {
            read_only: false,
            version: None,
            timeout: Duration::from_secs(30),
            credentials: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MountHandle {
    pub id: String,
    pub remote_path: String,
    pub local_path: String,
    pub backend_type: BackendType,
    pub mounted_at: DateTime<Utc>,
}

impl MountHandle {
    pub fn new(remote_path: &str, local_path: &str, backend_type: BackendType) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            remote_path: remote_path.to_string(),
            local_path: local_path.to_string(),
            backend_type,
            mounted_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MountEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub size: u64,
    pub modified: DateTime<Utc>,
    pub created: DateTime<Utc>,
    pub is_dir: bool,
    pub permissions: u32,
}

#[derive(Debug, Clone)]
pub struct SpaceUsage {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
}

#[async_trait]
pub trait MountBackend: Send + Sync {
    async fn mount(
        &self,
        remote_path: &str,
        local_path: &str,
        options: &MountOptions,
    ) -> Result<MountHandle, MountError>;

    async fn unmount(&self, handle: &MountHandle) -> Result<(), MountError>;

    async fn read_dir(&self, handle: &MountHandle, path: &str) -> Result<Vec<MountEntry>, MountError>;

    async fn read_file(
        &self,
        handle: &MountHandle,
        path: &str,
        offset: u64,
        length: u64,
    ) -> Result<Vec<u8>, MountError>;

    async fn metadata(&self, handle: &MountHandle, path: &str) -> Result<FileMetadata, MountError>;

    async fn space_usage(&self, handle: &MountHandle) -> Result<SpaceUsage, MountError>;

    fn backend_type(&self) -> BackendType;
}
