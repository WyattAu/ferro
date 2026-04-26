use crate::config::DesktopConfig;
use crate::rclone::RcloneManager;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Mount service that manages the mount lifecycle
pub struct MountService {
    rclone: Arc<RcloneManager>,
    is_mounted: RwLock<bool>,
}

impl MountService {
    pub fn new(config: DesktopConfig) -> Self {
        Self {
            rclone: Arc::new(RcloneManager::new(config)),
            is_mounted: RwLock::new(false),
        }
    }

    pub async fn mount(&self) -> anyhow::Result<()> {
        self.rclone.mount().await?;
        *self.is_mounted.write().await = true;
        Ok(())
    }

    pub async fn unmount(&self) -> anyhow::Result<()> {
        self.rclone.unmount().await?;
        *self.is_mounted.write().await = false;
        Ok(())
    }

    pub async fn is_mounted(&self) -> bool {
        *self.is_mounted.read().await
    }

    pub fn rclone_manager(&self) -> &RcloneManager {
        &self.rclone
    }
}
