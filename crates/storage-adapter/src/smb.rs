use async_trait::async_trait;

use crate::error::StorageAdapterError;
use crate::memory::InMemoryBackend;

#[derive(Debug, Clone)]
pub struct SmbCredentials {
    pub username: String,
    pub password: String,
    pub domain: Option<String>,
}

#[async_trait]
pub trait SmbBackend: Send + Sync {
    async fn connect(
        &self,
        server: &str,
        share: &str,
        credentials: &SmbCredentials,
    ) -> Result<(), StorageAdapterError>;
    async fn disconnect(&self) -> Result<(), StorageAdapterError>;
    async fn list_shares(&self, server: &str) -> Result<Vec<String>, StorageAdapterError>;
    async fn is_connected(&self) -> bool;
}

pub struct MockSmbBackend {
    inner: InMemoryBackend,
    server: dashmap::DashMap<String, String>,
    shares: dashmap::DashMap<String, Vec<String>>,
}

impl MockSmbBackend {
    pub fn new() -> Self {
        Self {
            inner: InMemoryBackend::new(),
            server: dashmap::DashMap::new(),
            shares: dashmap::DashMap::new(),
        }
    }

    pub fn add_share(&self, server: &str, share: &str) {
        self.shares
            .entry(server.to_string())
            .or_default()
            .push(share.to_string());
    }

    pub fn storage(&self) -> &InMemoryBackend {
        &self.inner
    }
}

impl Default for MockSmbBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SmbBackend for MockSmbBackend {
    async fn connect(
        &self,
        server: &str,
        share: &str,
        _credentials: &SmbCredentials,
    ) -> Result<(), StorageAdapterError> {
        let key = format!("{server}/{share}");
        if self.server.contains_key(&key) {
            return Err(StorageAdapterError::ConnectionFailed(format!(
                "already connected to {key}"
            )));
        }
        self.server.insert(key, share.to_string());
        Ok(())
    }

    async fn disconnect(&self) -> Result<(), StorageAdapterError> {
        if self.server.is_empty() {
            return Err(StorageAdapterError::ConnectionFailed(
                "not connected".into(),
            ));
        }
        self.server.clear();
        Ok(())
    }

    async fn list_shares(&self, server: &str) -> Result<Vec<String>, StorageAdapterError> {
        self.shares
            .get(server)
            .map(|s| s.value().clone())
            .ok_or_else(|| {
                StorageAdapterError::ConnectionFailed(format!("server {server} not found"))
            })
    }

    async fn is_connected(&self) -> bool {
        !self.server.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::StorageBackend;

    #[tokio::test]
    async fn test_connect_disconnect() {
        let smb = MockSmbBackend::new();
        let creds = SmbCredentials {
            username: "user".into(),
            password: "pass".into(),
            domain: None,
        };
        smb.connect("server", "share", &creds).await.unwrap();
        assert!(smb.is_connected().await);
        smb.disconnect().await.unwrap();
        assert!(!smb.is_connected().await);
    }

    #[tokio::test]
    async fn test_connect_already_connected() {
        let smb = MockSmbBackend::new();
        let creds = SmbCredentials {
            username: "u".into(),
            password: "p".into(),
            domain: None,
        };
        smb.connect("s", "sh", &creds).await.unwrap();
        let result = smb.connect("s", "sh", &creds).await;
        assert!(matches!(result, Err(StorageAdapterError::ConnectionFailed(_))));
    }

    #[tokio::test]
    async fn test_disconnect_not_connected() {
        let smb = MockSmbBackend::new();
        let result = smb.disconnect().await;
        assert!(matches!(result, Err(StorageAdapterError::ConnectionFailed(_))));
    }

    #[tokio::test]
    async fn test_list_shares() {
        let smb = MockSmbBackend::new();
        smb.add_share("srv", "docs");
        smb.add_share("srv", "media");
        let shares = smb.list_shares("srv").await.unwrap();
        assert_eq!(shares.len(), 2);
        assert!(shares.contains(&"docs".to_string()));
        assert!(shares.contains(&"media".to_string()));
    }

    #[tokio::test]
    async fn test_list_shares_unknown_server() {
        let smb = MockSmbBackend::new();
        let result = smb.list_shares("unknown").await;
        assert!(matches!(result, Err(StorageAdapterError::ConnectionFailed(_))));
    }

    #[tokio::test]
    async fn test_storage_through_smb() {
        let smb = MockSmbBackend::new();
        let meta = crate::backend::ObjectMetadata::new();
        smb.storage().put("smb-file", b"hello", &meta).await.unwrap();
        assert_eq!(smb.storage().get("smb-file").await.unwrap(), b"hello");
    }
}
