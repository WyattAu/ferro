use std::sync::Arc;

use chrono::Utc;
use dashmap::DashMap;

use crate::error::MarketplaceError;
use crate::plugin::{
    PluginId, PluginInstallation, PluginManifest, PluginMetadata, PluginReview, Version,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortField {
    Name,
    Downloads,
    Rating,
    Updated,
}

pub struct PluginRegistry {
    pub plugins: DashMap<PluginId, PluginMetadata>,
    pub installations: DashMap<PluginId, PluginInstallation>,
    pub reviews: DashMap<PluginId, Vec<PluginReview>>,
    current_abi: Arc<String>,
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: DashMap::new(),
            installations: DashMap::new(),
            reviews: DashMap::new(),
            current_abi: Arc::new("1".to_string()),
        }
    }

    pub fn with_abi(abi_version: impl Into<String>) -> Self {
        Self {
            plugins: DashMap::new(),
            installations: DashMap::new(),
            reviews: DashMap::new(),
            current_abi: Arc::new(abi_version.into()),
        }
    }

    pub fn register(&self, manifest: PluginManifest) -> Result<(), MarketplaceError> {
        let id = manifest.metadata.id.clone();
        if self.plugins.contains_key(&id) {
            return Err(MarketplaceError::AlreadyInstalled { id: id.to_string() });
        }
        self.plugins.insert(id.clone(), manifest.metadata);
        self.reviews.insert(id.clone(), Vec::new());
        Ok(())
    }

    pub fn unregister(&self, id: &PluginId) -> Result<(), MarketplaceError> {
        if self.plugins.remove(id).is_none() {
            return Err(MarketplaceError::PluginNotFound { id: id.to_string() });
        }
        self.installations.remove(id);
        self.reviews.remove(id);
        Ok(())
    }

    pub fn get(&self, id: &PluginId) -> Option<PluginMetadata> {
        self.plugins.get(id).map(|r| r.value().clone())
    }

    pub fn list(&self, tags: Option<&[String]>, sort_by: SortField) -> Vec<PluginMetadata> {
        let mut result: Vec<PluginMetadata> = if let Some(filter_tags) = tags {
            self.plugins
                .iter()
                .filter(|entry| {
                    let meta = entry.value();
                    meta.tags.iter().any(|t| filter_tags.contains(t))
                })
                .map(|entry| entry.value().clone())
                .collect()
        } else {
            self.plugins
                .iter()
                .map(|entry| entry.value().clone())
                .collect()
        };

        match sort_by {
            SortField::Name => result.sort_by_key(|a| a.name.clone()),
            SortField::Downloads => result.sort_by_key(|b| std::cmp::Reverse(b.downloads)),
            SortField::Rating => result.sort_by(|a, b| {
                b.rating
                    .partial_cmp(&a.rating)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
            SortField::Updated => result.sort_by_key(|b| std::cmp::Reverse(b.updated_at)),
        }
        result
    }

    pub fn search(&self, query: &str) -> Vec<PluginMetadata> {
        let query_lower = query.to_lowercase();
        self.plugins
            .iter()
            .filter(|entry| {
                let meta = entry.value();
                meta.name.to_lowercase().contains(&query_lower)
                    || meta.description.to_lowercase().contains(&query_lower)
                    || meta
                        .tags
                        .iter()
                        .any(|t| t.to_lowercase().contains(&query_lower))
            })
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub fn install(&self, id: &PluginId, version: &Version) -> Result<(), MarketplaceError> {
        let meta = self
            .plugins
            .get(id)
            .ok_or_else(|| MarketplaceError::PluginNotFound { id: id.to_string() })?
            .value()
            .clone();

        if meta.version != *version {
            return Err(MarketplaceError::VersionNotFound {
                plugin_id: id.to_string(),
                version: version.to_string(),
            });
        }

        if meta.abi_version != *self.current_abi {
            return Err(MarketplaceError::IncompatibleAbi {
                required: self.current_abi.to_string(),
                plugin: meta.abi_version,
            });
        }

        if self.installations.contains_key(id) {
            return Err(MarketplaceError::AlreadyInstalled { id: id.to_string() });
        }

        self.installations.insert(
            id.clone(),
            PluginInstallation {
                plugin_id: id.clone(),
                version: version.clone(),
                installed_at: Utc::now(),
                enabled: true,
            },
        );

        Ok(())
    }

    pub fn install_with_dependencies(
        &self,
        manifest: &PluginManifest,
        version: &Version,
    ) -> Result<(), MarketplaceError> {
        for dep in &manifest.dependencies {
            let meta = self.plugins.get(&dep.plugin_id).ok_or_else(|| {
                MarketplaceError::PluginNotFound {
                    id: dep.plugin_id.to_string(),
                }
            })?;
            if meta.value().version < dep.min_version {
                return Err(MarketplaceError::VersionNotFound {
                    plugin_id: dep.plugin_id.to_string(),
                    version: format!(">= {}", dep.min_version),
                });
            }
            if !self.installations.contains_key(&dep.plugin_id) {
                self.install(&dep.plugin_id, &meta.value().version)?;
            }
        }
        self.install(&manifest.metadata.id, version)
    }

    pub fn uninstall(&self, id: &PluginId) -> Result<(), MarketplaceError> {
        if self.installations.remove(id).is_none() {
            return Err(MarketplaceError::PluginNotFound { id: id.to_string() });
        }
        Ok(())
    }

    pub fn enable(&self, id: &PluginId) -> Result<(), MarketplaceError> {
        let mut inst = self
            .installations
            .get_mut(id)
            .ok_or_else(|| MarketplaceError::PluginNotFound { id: id.to_string() })?;
        inst.enabled = true;
        Ok(())
    }

    pub fn disable(&self, id: &PluginId) -> Result<(), MarketplaceError> {
        let mut inst = self
            .installations
            .get_mut(id)
            .ok_or_else(|| MarketplaceError::PluginNotFound { id: id.to_string() })?;
        inst.enabled = false;
        Ok(())
    }

    pub fn installed_plugins(&self) -> Vec<PluginInstallation> {
        self.installations
            .iter()
            .map(|r| r.value().clone())
            .collect()
    }

    pub fn add_review(&self, id: &PluginId, review: PluginReview) -> Result<(), MarketplaceError> {
        if !self.plugins.contains_key(id) {
            return Err(MarketplaceError::PluginNotFound { id: id.to_string() });
        }
        let mut reviews = self.reviews.get_mut(id).unwrap();
        reviews.push(review);
        Ok(())
    }

    pub fn average_rating(&self, id: &PluginId) -> Option<f32> {
        let reviews = self.reviews.get(id)?;
        if reviews.is_empty() {
            return None;
        }
        let sum: f32 = reviews.iter().map(|r| r.rating as f32).sum();
        Some(sum / reviews.len() as f32)
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use url::Url;

    use super::*;
    use crate::plugin::{PluginDependency, PluginReview};

    fn make_metadata(
        id: &str,
        name: &str,
        version: Version,
        abi: &str,
        tags: Vec<&str>,
    ) -> PluginMetadata {
        PluginMetadata {
            id: PluginId::from_str_unchecked(id),
            name: name.to_string(),
            description: format!("{} plugin", name),
            version,
            author: "test".to_string(),
            license: "MIT".to_string(),
            abi_version: abi.to_string(),
            homepage: None,
            repository: None,
            icon_url: None,
            capabilities: vec!["read".to_string()],
            tags: tags.into_iter().map(String::from).collect(),
            checksum: [0u8; 32],
            size_bytes: 1024,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            downloads: 0,
            rating: 0.0,
        }
    }

    fn make_manifest(
        id: &str,
        name: &str,
        version: Version,
        abi: &str,
        tags: Vec<&str>,
    ) -> PluginManifest {
        PluginManifest {
            metadata: make_metadata(id, name, version, abi, tags),
            wasm_url: Url::parse("https://example.com/plugin.wasm").unwrap(),
            dependencies: vec![],
        }
    }

    #[test]
    fn test_register_and_get() {
        let registry = PluginRegistry::new();
        let manifest = make_manifest(
            "00000000-0000-0000-0000-000000000001",
            "test-plugin",
            Version::new(1, 0, 0),
            "1",
            vec![],
        );
        let id = manifest.metadata.id.clone();

        registry.register(manifest).unwrap();
        let meta = registry.get(&id).unwrap();
        assert_eq!(meta.name, "test-plugin");
        assert_eq!(meta.version, Version::new(1, 0, 0));
    }

    #[test]
    fn test_duplicate_registration_error() {
        let registry = PluginRegistry::new();
        let manifest = make_manifest(
            "00000000-0000-0000-0000-000000000001",
            "test-plugin",
            Version::new(1, 0, 0),
            "1",
            vec![],
        );
        registry.register(manifest.clone()).unwrap();
        let err = registry.register(manifest).unwrap_err();
        assert!(matches!(err, MarketplaceError::AlreadyInstalled { .. }));
    }

    #[test]
    fn test_list_and_filter() {
        let registry = PluginRegistry::new();
        registry
            .register(make_manifest(
                "00000000-0000-0000-0000-000000000001",
                "alpha",
                Version::new(1, 0, 0),
                "1",
                vec!["storage", "fast"],
            ))
            .unwrap();
        registry
            .register(make_manifest(
                "00000000-0000-0000-0000-000000000002",
                "beta",
                Version::new(2, 0, 0),
                "1",
                vec!["network"],
            ))
            .unwrap();

        let all = registry.list(None, SortField::Name);
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].name, "alpha");

        let filtered = registry.list(Some(&["storage".to_string()]), SortField::Name);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "alpha");
    }

    #[test]
    fn test_search() {
        let registry = PluginRegistry::new();
        registry
            .register(make_manifest(
                "00000000-0000-0000-0000-000000000001",
                "storage-backend",
                Version::new(1, 0, 0),
                "1",
                vec!["storage", "s3"],
            ))
            .unwrap();
        registry
            .register(make_manifest(
                "00000000-0000-0000-0000-000000000002",
                "auth-provider",
                Version::new(1, 0, 0),
                "1",
                vec!["auth"],
            ))
            .unwrap();

        let results = registry.search("storage");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "storage-backend");
    }

    #[test]
    fn test_install_uninstall() {
        let registry = PluginRegistry::new();
        let version = Version::new(1, 0, 0);
        let manifest = make_manifest(
            "00000000-0000-0000-0000-000000000001",
            "test-plugin",
            version.clone(),
            "1",
            vec![],
        );
        let id = manifest.metadata.id.clone();

        registry.register(manifest).unwrap();
        registry.install(&id, &version).unwrap();
        assert_eq!(registry.installed_plugins().len(), 1);

        registry.uninstall(&id).unwrap();
        assert_eq!(registry.installed_plugins().len(), 0);
    }

    #[test]
    fn test_enable_disable() {
        let registry = PluginRegistry::new();
        let version = Version::new(1, 0, 0);
        let manifest = make_manifest(
            "00000000-0000-0000-0000-000000000001",
            "test-plugin",
            version.clone(),
            "1",
            vec![],
        );
        let id = manifest.metadata.id.clone();

        registry.register(manifest).unwrap();
        registry.install(&id, &version).unwrap();

        registry.disable(&id).unwrap();
        assert!(!registry.installed_plugins()[0].enabled);

        registry.enable(&id).unwrap();
        assert!(registry.installed_plugins()[0].enabled);
    }

    #[test]
    fn test_abi_compatibility_check() {
        let registry = PluginRegistry::new();
        let version = Version::new(1, 0, 0);
        let manifest = make_manifest(
            "00000000-0000-0000-0000-000000000001",
            "old-plugin",
            version.clone(),
            "2",
            vec![],
        );
        let id = manifest.metadata.id.clone();

        registry.register(manifest).unwrap();
        let err = registry.install(&id, &version).unwrap_err();
        assert!(matches!(err, MarketplaceError::IncompatibleAbi { .. }));
    }

    #[test]
    fn test_dependency_check() {
        let registry = PluginRegistry::new();
        let dep_version = Version::new(1, 0, 0);

        let dep_manifest = make_manifest(
            "00000000-0000-0000-0000-000000000001",
            "dep-plugin",
            dep_version.clone(),
            "1",
            vec![],
        );

        let main_manifest = PluginManifest {
            metadata: make_metadata(
                "00000000-0000-0000-0000-000000000002",
                "main-plugin",
                Version::new(1, 0, 0),
                "1",
                vec![],
            ),
            wasm_url: Url::parse("https://example.com/main.wasm").unwrap(),
            dependencies: vec![PluginDependency {
                plugin_id: PluginId::from_str_unchecked("00000000-0000-0000-0000-000000000001"),
                min_version: Version::new(1, 0, 0),
            }],
        };

        let main_version = main_manifest.metadata.version.clone();

        registry.register(dep_manifest).unwrap();
        registry.register(main_manifest.clone()).unwrap();
        registry
            .install_with_dependencies(&main_manifest, &main_version)
            .unwrap();

        assert_eq!(registry.installed_plugins().len(), 2);
    }

    #[test]
    fn test_reviews_and_rating() {
        let registry = PluginRegistry::new();
        let manifest = make_manifest(
            "00000000-0000-0000-0000-000000000001",
            "test-plugin",
            Version::new(1, 0, 0),
            "1",
            vec![],
        );
        let id = manifest.metadata.id.clone();

        registry.register(manifest).unwrap();

        let review1 = PluginReview::new("user1", 4, "good").unwrap();
        let review2 = PluginReview::new("user2", 5, "great").unwrap();

        registry.add_review(&id, review1).unwrap();
        registry.add_review(&id, review2).unwrap();

        let avg = registry.average_rating(&id).unwrap();
        assert_eq!(avg, 4.5);
    }

    #[test]
    fn test_uninstall_non_existent() {
        let registry = PluginRegistry::new();
        let id = PluginId::from_str_unchecked("00000000-0000-0000-0000-000000000001");
        let err = registry.uninstall(&id).unwrap_err();
        assert!(matches!(err, MarketplaceError::PluginNotFound { .. }));
    }
}
