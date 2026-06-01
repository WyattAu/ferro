use dashmap::DashMap;

use crate::error::MarketplaceError;
use crate::plugin::{PluginId, PluginManifest, PluginMetadata, Version};

#[allow(async_fn_in_trait)]
pub trait PluginRepository {
    async fn fetch_manifest(&self, id: &PluginId) -> Result<PluginManifest, MarketplaceError>;
    async fn list_available(&self) -> Result<Vec<PluginMetadata>, MarketplaceError>;
    async fn download_wasm(
        &self,
        id: &PluginId,
        version: &Version,
    ) -> Result<Vec<u8>, MarketplaceError>;
    async fn publish(&self, manifest: PluginManifest) -> Result<(), MarketplaceError>;
}

pub struct MockPluginRepository {
    manifests: DashMap<PluginId, PluginManifest>,
    wasm_binaries: DashMap<String, Vec<u8>>,
}

impl MockPluginRepository {
    pub fn new() -> Self {
        Self {
            manifests: DashMap::new(),
            wasm_binaries: DashMap::new(),
        }
    }

    pub fn with_wasm(self, id: &PluginId, version: &Version, data: Vec<u8>) -> Self {
        let key = format!("{}:{}", id, version);
        self.wasm_binaries.insert(key, data);
        self
    }
}

impl Default for MockPluginRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginRepository for MockPluginRepository {
    async fn fetch_manifest(&self, id: &PluginId) -> Result<PluginManifest, MarketplaceError> {
        self.manifests
            .get(id)
            .map(|r| r.value().clone())
            .ok_or_else(|| MarketplaceError::PluginNotFound { id: id.to_string() })
    }

    async fn list_available(&self) -> Result<Vec<PluginMetadata>, MarketplaceError> {
        Ok(self
            .manifests
            .iter()
            .map(|r| r.value().metadata.clone())
            .collect())
    }

    async fn download_wasm(
        &self,
        id: &PluginId,
        version: &Version,
    ) -> Result<Vec<u8>, MarketplaceError> {
        let key = format!("{}:{}", id, version);
        self.wasm_binaries
            .get(&key)
            .map(|r| r.value().clone())
            .ok_or_else(|| MarketplaceError::VersionNotFound {
                plugin_id: id.to_string(),
                version: version.to_string(),
            })
    }

    async fn publish(&self, manifest: PluginManifest) -> Result<(), MarketplaceError> {
        let id = manifest.metadata.id.clone();
        self.manifests.insert(id, manifest);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use url::Url;

    use super::*;
    use crate::plugin::PluginMetadata;

    fn make_manifest(id: &str, name: &str) -> PluginManifest {
        PluginManifest {
            metadata: PluginMetadata {
                id: PluginId::from_str_unchecked(id),
                name: name.to_string(),
                description: format!("{} plugin", name),
                version: Version::new(1, 0, 0),
                author: "test".to_string(),
                license: "MIT".to_string(),
                abi_version: "1".to_string(),
                homepage: None,
                repository: None,
                icon_url: None,
                capabilities: vec!["read".to_string()],
                tags: vec![],
                checksum: [0u8; 32],
                size_bytes: 1024,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                downloads: 0,
                rating: 0.0,
            },
            wasm_url: Url::parse("https://example.com/plugin.wasm").unwrap(),
            dependencies: vec![],
        }
    }

    #[tokio::test]
    async fn test_fetch_manifest() {
        let repo = MockPluginRepository::new();
        let manifest = make_manifest("00000000-0000-0000-0000-000000000001", "test");
        let id = manifest.metadata.id.clone();

        repo.publish(manifest.clone()).await.unwrap();
        let fetched = repo.fetch_manifest(&id).await.unwrap();
        assert_eq!(fetched.metadata.name, "test");
    }

    #[tokio::test]
    async fn test_list_available() {
        let repo = MockPluginRepository::new();
        repo.publish(make_manifest(
            "00000000-0000-0000-0000-000000000001",
            "alpha",
        ))
        .await
        .unwrap();
        repo.publish(make_manifest(
            "00000000-0000-0000-0000-000000000002",
            "beta",
        ))
        .await
        .unwrap();

        let list = repo.list_available().await.unwrap();
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn test_download() {
        let id = PluginId::from_str_unchecked("00000000-0000-0000-0000-000000000001");
        let version = Version::new(1, 0, 0);
        let wasm_data = vec![0x00, 0x61, 0x73, 0x6d];

        let repo = MockPluginRepository::new().with_wasm(&id, &version, wasm_data.clone());

        let downloaded = repo.download_wasm(&id, &version).await.unwrap();
        assert_eq!(downloaded, wasm_data);
    }

    #[tokio::test]
    async fn test_publish() {
        let repo = MockPluginRepository::new();
        let manifest = make_manifest("00000000-0000-0000-0000-000000000001", "test");
        let id = manifest.metadata.id.clone();

        repo.publish(manifest).await.unwrap();
        assert!(repo.fetch_manifest(&id).await.is_ok());
    }

    #[tokio::test]
    async fn test_not_found_error() {
        let repo = MockPluginRepository::new();
        let id = PluginId::from_str_unchecked("00000000-0000-0000-0000-000000000001");

        let err = repo.fetch_manifest(&id).await.unwrap_err();
        assert!(matches!(err, MarketplaceError::PluginNotFound { .. }));
    }
}
