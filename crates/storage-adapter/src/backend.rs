use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::StorageAdapterError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BackendType {
    Local,
    S3,
    Memory,
    Nfs,
    Smb,
    Composite,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ObjectMetadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_encoding: Option<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub custom_headers: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub etag: Option<String>,
    #[serde(default, with = "chrono::serde::ts_milliseconds_option")]
    pub last_modified: Option<DateTime<Utc>>,
}

impl ObjectMetadata {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_content_type(mut self, ct: impl Into<String>) -> Self {
        self.content_type = Some(ct.into());
        self
    }

    pub fn with_etag(mut self, etag: impl Into<String>) -> Self {
        self.etag = Some(etag.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectInfo {
    pub path: String,
    pub size: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    #[serde(default, with = "chrono::serde::ts_milliseconds_option")]
    pub last_modified: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub etag: Option<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, String>,
}

#[async_trait]
pub trait StorageBackend: Send + Sync {
    async fn get(&self, path: &str) -> Result<Vec<u8>, StorageAdapterError>;
    async fn put(
        &self,
        path: &str,
        data: &[u8],
        metadata: &ObjectMetadata,
    ) -> Result<(), StorageAdapterError>;
    async fn delete(&self, path: &str) -> Result<(), StorageAdapterError>;
    async fn exists(&self, path: &str) -> Result<bool, StorageAdapterError>;
    async fn list(&self, prefix: &str) -> Result<Vec<ObjectInfo>, StorageAdapterError>;
    async fn size(&self, path: &str) -> Result<u64, StorageAdapterError>;
    async fn copy(&self, from: &str, to: &str) -> Result<(), StorageAdapterError>;
    async fn move_obj(&self, from: &str, to: &str) -> Result<(), StorageAdapterError>;
    async fn metadata(&self, path: &str) -> Result<ObjectInfo, StorageAdapterError>;
    fn backend_type(&self) -> BackendType;
}
