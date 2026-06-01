//! Configuration integration.
//!
//! Provides helpers for loading and validating server configuration.

use ferro_config_manager::{ConfigLoader, FerroConfig};

pub fn load_server_config(path: Option<&str>) -> FerroConfig {
    ConfigLoader::load_merged(path).unwrap_or_else(|_| ConfigLoader::load_with_defaults())
}

pub fn load_default_config() -> FerroConfig {
    ConfigLoader::load_with_defaults()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_default_config() {
        let config = load_default_config();
        assert!(config.server.host.len() > 0);
    }

    #[test]
    fn test_load_config_missing_file() {
        let config = load_server_config(Some("/nonexistent/path.toml"));
        assert!(config.server.host.len() > 0);
    }
}
