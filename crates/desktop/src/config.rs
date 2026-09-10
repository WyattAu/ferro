use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("ferro")
        .join("desktop.json")
}

/// Load the stored config, falling back to defaults (never None).
pub fn load_from_disk_or_default() -> DesktopConfig {
    load_config_from_disk().unwrap_or_default()
}

pub fn load_config_from_disk() -> Option<DesktopConfig> {
    let path = config_path();
    let data = std::fs::read_to_string(&path).ok()?;
    let mut config: DesktopConfig = serde_json::from_str(&data).ok()?;
    // Overlay the owner-only secrets file; it wins over legacy
    // plaintext values still present in desktop.json.
    let secrets = crate::secret_store::load_secrets();
    if let Some(token) = secrets.auth_token
        && !token.is_empty()
    {
        config.auth_token = Some(token);
    }
    if let Some(password) = secrets.password
        && !password.is_empty()
    {
        config.password = password;
    }
    // Environment always wins (containers, one-shot CLI runs).
    if let Ok(token) = std::env::var("FERRO_AUTH_TOKEN")
        && !token.is_empty()
    {
        config.auth_token = Some(token);
    }
    if let Ok(password) = std::env::var("FERRO_PASSWORD")
        && !password.is_empty()
    {
        config.password = password;
    }
    Some(config)
}

pub fn save_config_to_disk(config: &DesktopConfig) -> Result<(), String> {
    // Secrets go to the owner-only file, never to desktop.json.
    crate::secret_store::save_secrets(&crate::secret_store::Secrets {
        auth_token: config.auth_token.clone(),
        refresh_token: config.refresh_token.clone(),
        password: if config.password.is_empty() {
            None
        } else {
            Some(config.password.clone())
        },
    })?;
    let mut scrubbed = config.clone();
    scrubbed.auth_token = None;
    scrubbed.refresh_token = None;
    scrubbed.password = String::new();
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let data = serde_json::to_string_pretty(&scrubbed).map_err(|e| e.to_string())?;
    std::fs::write(&path, data).map_err(|e| e.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesktopConfig {
    /// Ferro server URL
    pub server_url: String,
    /// Username for authentication
    pub username: String,
    /// Password or API token
    pub password: String,
    /// Bearer token (OIDC access token) used by the sync engine. When set,
    /// takes precedence over username/password basic auth.
    #[serde(default)]
    pub auth_token: Option<String>,
    /// Refresh token for the sync engine's silent re-auth.
    #[serde(default)]
    pub refresh_token: Option<String>,
    /// Local mount point
    pub mount_point: PathBuf,
    /// rclone binary path (auto-detected if empty)
    pub rclone_path: Option<PathBuf>,
    /// Auto-mount on login
    pub auto_mount: bool,
    /// Sync interval in seconds (0 = manual only)
    pub sync_interval_secs: u32,
}

impl Default for DesktopConfig {
    fn default() -> Self {
        Self {
            server_url: "http://localhost:8080".to_string(),
            username: String::new(),
            password: String::new(),
            auth_token: None,
            refresh_token: None,
            mount_point: Self::default_mount_point(),
            rclone_path: None,
            auto_mount: true,
            sync_interval_secs: 0,
        }
    }
}

impl DesktopConfig {
    pub fn default_mount_point() -> PathBuf {
        #[cfg(target_os = "windows")]
        {
            PathBuf::from("Z:\\")
        }
        #[cfg(target_os = "macos")]
        {
            PathBuf::from("/Volumes/Ferro")
        }
        #[cfg(target_os = "linux")]
        {
            PathBuf::from("/mnt/ferro")
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            PathBuf::from("/tmp/ferro")
        }
    }

    /// Build rclone remote URL for WebDAV
    pub fn rclone_remote_url(&self) -> String {
        format!(
            "webdav://{}:{}@{}/",
            self.username,
            self.password,
            self.server_url.trim_end_matches('/'),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = DesktopConfig::default();
        assert_eq!(config.server_url, "http://localhost:8080");
        assert!(config.auto_mount);
        assert!(!config.mount_point.as_os_str().is_empty());
    }

    #[test]
    fn test_rclone_remote_url() {
        let config = DesktopConfig {
            server_url: "http://localhost:8080".to_string(),
            username: "admin".to_string(),
            password: "secret".to_string(),
            ..Default::default()
        };
        let url = config.rclone_remote_url();
        assert!(url.contains("admin:secret@"));
        assert!(url.contains("localhost:8080"));
    }
}
