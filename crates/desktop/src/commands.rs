use crate::config::DesktopConfig;
use crate::mount::MountService;
use crate::rclone::RcloneManager;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[cfg(feature = "sync")]
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(feature = "sync")]
use crate::sync::engine::SyncEngine;
#[cfg(feature = "sync")]
use crate::sync::types::SyncSummary;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg(feature = "sync")]
pub struct SyncStatusResponse {
    pub running: bool,
    pub paused: bool,
    pub last_summary: Option<SyncSummary>,
    pub error: Option<String>,
}

pub struct DesktopState {
    pub config: Arc<RwLock<DesktopConfig>>,
    pub mount_service: MountService,

    #[cfg(feature = "sync")]
    pub sync_engine: Arc<RwLock<Option<Arc<SyncEngine>>>>,
    #[cfg(feature = "sync")]
    pub sync_running: Arc<AtomicBool>,
    #[cfg(feature = "sync")]
    pub sync_paused: Arc<AtomicBool>,
    #[cfg(feature = "sync")]
    pub sync_shutdown: Arc<RwLock<tokio::sync::watch::Sender<bool>>>,
    #[cfg(feature = "sync")]
    pub last_sync_summary: Arc<RwLock<Option<SyncSummary>>>,
    #[cfg(feature = "sync")]
    pub sync_error: Arc<RwLock<Option<String>>>,
}

impl DesktopState {
    pub fn new(config: DesktopConfig) -> Self {
        let mount_service = MountService::new(config.clone());
        Self {
            config: Arc::new(RwLock::new(config)),
            mount_service,

            #[cfg(feature = "sync")]
            sync_engine: Arc::new(RwLock::new(None)),
            #[cfg(feature = "sync")]
            sync_running: Arc::new(AtomicBool::new(false)),
            #[cfg(feature = "sync")]
            sync_paused: Arc::new(AtomicBool::new(false)),
            #[cfg(feature = "sync")]
            sync_shutdown: Arc::new(RwLock::new({
                let (tx, _) = tokio::sync::watch::channel(false);
                tx
            })),
            #[cfg(feature = "sync")]
            last_sync_summary: Arc::new(RwLock::new(None)),
            #[cfg(feature = "sync")]
            sync_error: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn mount_drive(&self) -> Result<String, String> {
        self.mount_service
            .mount()
            .await
            .map_err(|e| e.to_string())?;
        Ok("mounted".to_string())
    }

    pub async fn unmount_drive(&self) -> Result<String, String> {
        self.mount_service
            .unmount()
            .await
            .map_err(|e| e.to_string())?;
        Ok("unmounted".to_string())
    }

    pub async fn get_mount_status(&self) -> MountStatusResponse {
        let is_mounted = self.mount_service.is_mounted().await;
        let config = self.config.read().await;
        MountStatusResponse {
            is_mounted,
            status: if is_mounted {
                "connected".to_string()
            } else {
                "disconnected".to_string()
            },
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
        if let Some(url) = request.server_url {
            config.server_url = url;
        }
        if let Some(user) = request.username {
            config.username = user;
        }
        if let Some(pass) = request.password {
            config.password = pass;
        }
        if let Some(mp) = request.mount_point {
            config.mount_point = std::path::PathBuf::from(mp);
        }
        if let Some(am) = request.auto_mount {
            config.auto_mount = am;
        }
        if let Some(si) = request.sync_interval_secs {
            config.sync_interval_secs = si;
        }
        crate::config::save_config_to_disk(&config)?;
        Ok(())
    }

    /// Start the sync engine. Creates the engine from current config and
    /// spawns a background periodic sync task.
    #[cfg(feature = "sync")]
    pub async fn start_sync(&self) -> Result<(), String> {
        if self.sync_running.load(Ordering::Relaxed) {
            return Ok(());
        }

        let config = self.config.read().await;
        if config.username.is_empty() || config.password.is_empty() {
            return Err("sync requires username and password".to_string());
        }

        let sync_config = crate::sync::engine::SyncConfig {
            local_path: config.mount_point.clone(),
            remote_path: "/".to_string(),
            server_url: config.server_url.clone(),
            username: config.username.clone(),
            password: config.password.clone(),
            ..Default::default()
        };

        let engine = SyncEngine::new(sync_config).map_err(|e| e.to_string())?;
        drop(config);

        self.sync_running.store(true, Ordering::Relaxed);
        self.sync_paused.store(false, Ordering::Relaxed);

        // Reset shutdown channel
        let (shutdown_tx, _shutdown_rx) = tokio::sync::watch::channel(false);
        *self.sync_shutdown.write().await = shutdown_tx;

        let engine = Arc::new(engine);
        self.sync_engine.write().await.replace(engine.clone());

        let running = self.sync_running.clone();
        let error = self.last_sync_summary.clone(); // intentionally kept for future summary storage
        let err_log = self.sync_error.clone();

        // Spawn initial sync
        let engine_initial = engine.clone();
        tokio::spawn(async move {
            if let Err(e) = engine_initial.sync().await {
                let _ = err_log.write().await.insert(e.to_string());
            }
            // Note: running is not set to false after initial sync; periodic loop manages lifecycle
            let _ = running;
            let _ = error;
        });

        // Spawn periodic sync loop
        let running_loop = self.sync_running.clone();
        let paused_loop = self.sync_paused.clone();
        let mut shutdown = self.sync_shutdown.read().await.subscribe();
        let error_loop = self.sync_error.clone();
        let interval_secs = self.config.read().await.sync_interval_secs as u64;

        if interval_secs > 0 {
            tokio::spawn(async move {
                let mut interval =
                    tokio::time::interval(std::time::Duration::from_secs(interval_secs.max(10)));
                loop {
                    tokio::select! {
                        _ = interval.tick() => {
                            if !running_loop.load(Ordering::Relaxed) {
                                break;
                            }
                            if paused_loop.load(Ordering::Relaxed) {
                                continue;
                            }
                            if let Err(e) = engine.sync().await {
                                *error_loop.write().await = Some(e.to_string());
                            }
                        }
                        _ = shutdown.changed() => {
                            break;
                        }
                    }
                }
            });
        }

        tracing::info!("sync engine started");
        Ok(())
    }

    /// Stop the sync engine gracefully.
    #[cfg(feature = "sync")]
    pub async fn stop_sync(&self) -> Result<(), String> {
        self.sync_running.store(false, Ordering::Relaxed);
        let _ = self.sync_shutdown.read().await.send(true);
        *self.sync_engine.write().await = None;
        *self.last_sync_summary.write().await = None;
        *self.sync_error.write().await = None;
        tracing::info!("sync engine stopped");
        Ok(())
    }

    /// Pause the sync engine (periodic syncs skip).
    #[cfg(feature = "sync")]
    pub fn pause_sync(&self) {
        self.sync_paused.store(true, Ordering::Relaxed);
        tracing::info!("sync paused");
    }

    /// Resume the sync engine after pause.
    #[cfg(feature = "sync")]
    pub fn resume_sync(&self) {
        self.sync_paused.store(false, Ordering::Relaxed);
        tracing::info!("sync resumed");
    }

    /// Trigger an immediate one-shot sync cycle.
    #[cfg(feature = "sync")]
    pub async fn sync_now(&self) -> Result<SyncSummary, String> {
        let engine_lock = self.sync_engine.read().await;
        let engine = engine_lock
            .as_ref()
            .ok_or_else(|| "sync engine not running".to_string())?;
        engine.sync().await.map_err(|e| e.to_string())
    }

    /// Get the current sync status.
    #[cfg(feature = "sync")]
    pub async fn get_sync_status(&self) -> SyncStatusResponse {
        SyncStatusResponse {
            running: self.sync_running.load(Ordering::Relaxed),
            paused: self.sync_paused.load(Ordering::Relaxed),
            last_summary: self.last_sync_summary.read().await.clone(),
            error: self.sync_error.read().await.clone(),
        }
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

    #[cfg(feature = "sync")]
    #[test]
    fn test_pause_resume() {
        let config = DesktopConfig::default();
        let state = DesktopState::new(config);
        assert!(!state.sync_paused.load(Ordering::Relaxed));
        state.pause_sync();
        assert!(state.sync_paused.load(Ordering::Relaxed));
        state.resume_sync();
        assert!(!state.sync_paused.load(Ordering::Relaxed));
    }
}
