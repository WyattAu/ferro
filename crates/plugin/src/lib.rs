use async_trait::async_trait;
use std::collections::HashMap;

/// Plugin error
#[derive(Debug)]
pub enum PluginError {
    NotFound,
    AlreadyLoaded,
    LoadFailed(String),
    InitFailed(String),
    ShutdownFailed(String),
    ConfigError(String),
}

impl std::fmt::Display for PluginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PluginError::NotFound => write!(f, "Plugin not found"),
            PluginError::AlreadyLoaded => write!(f, "Plugin already loaded"),
            PluginError::LoadFailed(msg) => write!(f, "Load failed: {}", msg),
            PluginError::InitFailed(msg) => write!(f, "Init failed: {}", msg),
            PluginError::ShutdownFailed(msg) => write!(f, "Shutdown failed: {}", msg),
            PluginError::ConfigError(msg) => write!(f, "Config error: {}", msg),
        }
    }
}

impl std::error::Error for PluginError {}

/// Plugin trait
#[async_trait]
pub trait Plugin: Send + Sync {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    async fn initialize(&self, config: &PluginConfig) -> Result<(), PluginError>;
    async fn shutdown(&self) -> Result<(), PluginError>;
}

/// Plugin configuration
#[derive(Debug, Clone)]
pub struct PluginConfig {
    pub name: String,
    pub version: String,
    pub enabled: bool,
    pub settings: HashMap<String, serde_json::Value>,
}

/// Plugin manager
pub struct PluginManager {
    plugins: Vec<Box<dyn Plugin>>,
    configs: HashMap<String, PluginConfig>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
            configs: HashMap::new(),
        }
    }

    /// Register a plugin
    pub fn register(&mut self, plugin: Box<dyn Plugin>, config: PluginConfig) -> Result<(), PluginError> {
        let name = plugin.name().to_string();

        if self.plugins.iter().any(|p| p.name() == name) {
            return Err(PluginError::AlreadyLoaded);
        }

        self.configs.insert(name, config);
        self.plugins.push(plugin);

        Ok(())
    }

    /// Unregister a plugin
    pub fn unregister(&mut self, name: &str) -> Result<(), PluginError> {
        if let Some(pos) = self.plugins.iter().position(|p| p.name() == name) {
            self.plugins.remove(pos);
            self.configs.remove(name);
            Ok(())
        } else {
            Err(PluginError::NotFound)
        }
    }

    /// Get a plugin
    pub fn get(&self, name: &str) -> Option<&dyn Plugin> {
        self.plugins.iter().find(|p| p.name() == name).map(|p| p.as_ref())
    }

    /// List all plugins
    pub fn list(&self) -> Vec<&dyn Plugin> {
        self.plugins.iter().map(|p| p.as_ref()).collect()
    }

    /// Initialize all plugins
    pub async fn initialize_all(&self) -> Result<(), PluginError> {
        for plugin in &self.plugins {
            let name = plugin.name();
            if let Some(config) = self.configs.get(name)
                && config.enabled
            {
                plugin.initialize(config).await?;
            }
        }
        Ok(())
    }

    /// Shutdown all plugins
    pub async fn shutdown_all(&self) -> Result<(), PluginError> {
        for plugin in &self.plugins {
            plugin.shutdown().await?;
        }
        Ok(())
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestPlugin;

    #[async_trait]
    impl Plugin for TestPlugin {
        fn name(&self) -> &str {
            "test-plugin"
        }

        fn version(&self) -> &str {
            "1.0.0"
        }

        async fn initialize(&self, _config: &PluginConfig) -> Result<(), PluginError> {
            Ok(())
        }

        async fn shutdown(&self) -> Result<(), PluginError> {
            Ok(())
        }
    }

    #[test]
    fn test_plugin_registration() {
        let mut manager = PluginManager::new();
        let config = PluginConfig {
            name: "test-plugin".to_string(),
            version: "1.0.0".to_string(),
            enabled: true,
            settings: HashMap::new(),
        };

        manager.register(Box::new(TestPlugin), config).unwrap();

        let plugin = manager.get("test-plugin");
        assert!(plugin.is_some());
        assert_eq!(plugin.unwrap().name(), "test-plugin");
    }

    #[test]
    fn test_plugin_unregistration() {
        let mut manager = PluginManager::new();
        let config = PluginConfig {
            name: "test-plugin".to_string(),
            version: "1.0.0".to_string(),
            enabled: true,
            settings: HashMap::new(),
        };

        manager.register(Box::new(TestPlugin), config).unwrap();
        manager.unregister("test-plugin").unwrap();

        let plugin = manager.get("test-plugin");
        assert!(plugin.is_none());
    }

    #[test]
    fn test_duplicate_registration() {
        let mut manager = PluginManager::new();
        let config = PluginConfig {
            name: "test-plugin".to_string(),
            version: "1.0.0".to_string(),
            enabled: true,
            settings: HashMap::new(),
        };

        manager.register(Box::new(TestPlugin), config.clone()).unwrap();
        let result = manager.register(Box::new(TestPlugin), config);
        assert!(result.is_err());
    }

    #[test]
    fn test_unregister_not_found() {
        let mut manager = PluginManager::new();
        let result = manager.unregister("nonexistent");
        assert!(result.is_err());
    }
}
