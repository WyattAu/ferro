use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("ferro")
        .join("desktop.json")
}

pub fn load_config_from_disk() -> Option<DesktopConfig> {
    let path = config_path();
    let data = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&data).ok()
}

pub fn save_config_to_disk(config: &DesktopConfig) -> Result<(), String> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let data = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
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
