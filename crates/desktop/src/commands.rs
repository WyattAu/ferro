use crate::config::DesktopConfig;
use crate::mount::MountService;
use crate::rclone::RcloneManager;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MountStatusResponse {
    pub is_mounted: bool,
    pub status: String,
    pub server_url: String,
    pub mount_point: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigResponse {
    pub server_url: String,
    pub mount_point: String,
    pub auto_mount: bool,
    pub sync_interval_secs: u32,
    pub rclone_available: bool,
    pub rclone_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveConfigRequest {
    pub server_url: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub mount_point: Option<String>,
    pub auto_mount: Option<bool>,
    pub sync_interval_secs: Option<u32>,
}

pub struct DesktopState {
    pub config: Arc<RwLock<DesktopConfig>>,
    pub mount_service: MountService,
}

impl DesktopState {
    pub fn new(config: DesktopConfig) -> Self {
        let mount_service = MountService::new(config.clone());
        Self {
            config: Arc::new(RwLock::new(config)),
            mount_service,
        }
    }

    pub async fn mount_drive(&self) -> Result<String, String> {
        self.mount_service.mount().await.map_err(|e| e.to_string())?;
        Ok("mounted".to_string())
    }

    pub async fn unmount_drive(&self) -> Result<String, String> {
        self.mount_service.unmount().await.map_err(|e| e.to_string())?;
        Ok("unmounted".to_string())
    }

    pub async fn get_mount_status(&self) -> MountStatusResponse {
        let is_mounted = self.mount_service.is_mounted().await;
        let config = self.config.read().await;
        MountStatusResponse {
            is_mounted,
            status: if is_mounted { "connected".to_string() } else { "disconnected".to_string() },
            server_url: config.server_url.clone(),
            mount_point: config.mount_point.display().to_string(),
        }
    }

    pub async fn get_config(&self) -> ConfigResponse {
        let config = self.config.read().await;
        let rclone_available = RcloneManager::check_rclone_available().is_ok();
        let rclone_version = RcloneManager::check_rclone_available().ok();
        ConfigResponse {
            server_url: config.server_url.clone(),
            mount_point: config.mount_point.display().to_string(),
            auto_mount: config.auto_mount,
            sync_interval_secs: config.sync_interval_secs,
            rclone_available,
            rclone_version,
        }
    }

    pub async fn save_config(&self, request: SaveConfigRequest) -> Result<(), String> {
        let mut config = self.config.write().await;
        if let Some(url) = request.server_url { config.server_url = url; }
        if let Some(user) = request.username { config.username = user; }
        if let Some(pass) = request.password { config.password = pass; }
        if let Some(mp) = request.mount_point {
            config.mount_point = std::path::PathBuf::from(mp);
        }
        if let Some(am) = request.auto_mount { config.auto_mount = am; }
        if let Some(si) = request.sync_interval_secs { config.sync_interval_secs = si; }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_desktop_state_status() {
        let config = DesktopConfig::default();
        let state = DesktopState::new(config);
        let status = state.get_mount_status().await;
        assert!(!status.is_mounted);
        assert_eq!(status.status, "disconnected");
    }

    #[tokio::test]
    async fn test_desktop_state_config() {
        let config = DesktopConfig::default();
        let state = DesktopState::new(config);
        let cfg = state.get_config().await;
        assert_eq!(cfg.server_url, "http://localhost:8080");
        assert!(cfg.auto_mount);
    }

    #[tokio::test]
    async fn test_save_config() {
        let config = DesktopConfig::default();
        let state = DesktopState::new(config);
        let request = SaveConfigRequest {
            server_url: Some("http://example.com".to_string()),
            username: Some("test".to_string()),
            password: Some("pass".to_string()),
            mount_point: None,
            auto_mount: Some(false),
            sync_interval_secs: Some(30),
        };
        state.save_config(request).await.unwrap();
        let cfg = state.get_config().await;
        assert_eq!(cfg.server_url, "http://example.com");
        assert_eq!(cfg.sync_interval_secs, 30);
        assert!(!cfg.auto_mount);
    }
}
