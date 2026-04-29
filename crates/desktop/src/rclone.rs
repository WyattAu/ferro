use crate::config::DesktopConfig;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::process::{Command as StdCommand, Stdio};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::RwLock;
use tracing::{debug, error, info};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MountProgress {
    pub bytes_transferred: u64,
    pub checks: u64,
    pub transfers: u64,
    pub errors: u64,
    pub speed_bytes_per_sec: f64,
    pub current_file: Option<String>,
    pub last_error: Option<String>,
    pub status: String,
}

impl MountProgress {
    pub fn running() -> Self {
        Self {
            status: "running".to_string(),
            ..Default::default()
        }
    }
}

/// Manages the rclone sidecar process lifecycle
pub struct RcloneManager {
    process: RwLock<Option<Child>>,
    config: DesktopConfig,
    progress: Arc<RwLock<MountProgress>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MountStatus {
    NotMounted,
    Mounting,
    Mounted { pid: u32 },
    Error { message: String },
    Unmounting,
}

impl RcloneManager {
    pub fn new(config: DesktopConfig) -> Self {
        Self {
            process: RwLock::new(None),
            config,
            progress: Arc::new(RwLock::new(MountProgress::default())),
        }
    }

    /// Get the current mount progress
    pub async fn progress(&self) -> MountProgress {
        self.progress.read().await.clone()
    }

    /// Check current mount status
    pub async fn status(&self) -> MountStatus {
        let mut guard = self.process.write().await;
        match guard.as_mut() {
            None => MountStatus::NotMounted,
            Some(child) => match child.try_wait() {
                Ok(Some(status)) => MountStatus::Error {
                    message: format!("rclone exited with status: {}", status),
                },
                Ok(None) => MountStatus::Mounted {
                    pid: child.id().unwrap_or(0),
                },
                Err(e) => MountStatus::Error {
                    message: format!("Failed to check process: {}", e),
                },
            },
        }
    }

    /// Mount the Ferro drive using rclone
    pub async fn mount(&self) -> Result<()> {
        let mut guard = self.process.write().await;

        if let Some(child) = guard.as_mut() {
            match child.try_wait() {
                Ok(None) => return Err(anyhow::anyhow!("Already mounted")),
                Ok(Some(_)) => {}
                Err(e) => return Err(anyhow::anyhow!("Cannot check process: {}", e)),
            }
        }

        let rclone_path: &std::path::Path = self
            .config
            .rclone_path
            .as_deref()
            .unwrap_or(std::path::Path::new("rclone"));
        let remote_url = self.config.rclone_remote_url();
        let mount_point = &self.config.mount_point;

        std::fs::create_dir_all(mount_point)?;

        info!(
            "Mounting Ferro drive: {} -> {}",
            remote_url,
            mount_point.display()
        );

        let mut child = Command::new(rclone_path)
            .args([
                "mount",
                &remote_url,
                &mount_point.to_string_lossy(),
                "--vfs-cache-mode",
                "full",
                "--vfs-cache-max-size",
                "10G",
                "--buffer-size",
                "32M",
                "--dir-cache-time",
                "60s",
                "--poll-interval",
                "30s",
                "--no-check-certificate",
                "--verbose",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to start rclone: {}. Is rclone installed?", e))?;

        let pid = child.id().unwrap_or(0);
        info!("rclone started with PID {}", pid);

        let progress_clone = self.progress.clone();
        let stderr = child.stderr.take().expect("stderr should be piped");
        let stdout = child.stdout.take().expect("stdout should be piped");

        tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let line = line.trim().to_string();
                if line.is_empty() {
                    continue;
                }

                let mut progress = progress_clone.write().await;

                if line.contains("Transferred:") {
                    progress.status = "running".to_string();
                    if let Some(before_speed) = line.split("Bytes/s,").next()
                        && let Some(speed_str) = before_speed.rsplit(' ').next()
                        && let Ok(speed) = speed_str.trim().parse::<f64>()
                    {
                        progress.speed_bytes_per_sec = speed * 1_000_000.0;
                    }
                    if let Some(errors_part) = line.split("Errors:").nth(1)
                        && let Some(err_count) = errors_part.split_whitespace().next()
                        && let Ok(err) = err_count.parse::<u64>()
                    {
                        progress.errors = err;
                    }
                } else if line.contains("ERROR:")
                    || line.contains("Fatal")
                    || line.contains("Failed")
                {
                    tracing::error!("rclone error: {}", line);
                    progress.last_error = Some(line);
                    progress.status = "error".to_string();
                } else if line.contains("NOTICE:") {
                    tracing::info!("rclone notice: {}", line);
                }
            }
            progress_clone.write().await.status = "exited".to_string();
        });

        let progress_clone2 = self.progress.clone();
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let line = line.trim().to_string();
                if line.is_empty() {
                    continue;
                }
                debug!("rclone: {}", line);

                if let Some(filename) = line.split(": Copied").nth(0) {
                    let mut progress = progress_clone2.write().await;
                    progress.current_file = Some(filename.trim().to_string());
                    progress.transfers += 1;
                }
            }
        });

        *guard = Some(child);

        Ok(())
    }

    /// Unmount the Ferro drive
    pub async fn unmount(&self) -> Result<()> {
        let mut guard = self.process.write().await;

        if let Some(mut child) = guard.take() {
            info!("Unmounting Ferro drive (PID: {})", child.id().unwrap_or(0));

            let _ = child.kill().await;

            match child.wait().await {
                Ok(status) => info!("rclone exited with status: {}", status),
                Err(e) => error!("Failed to wait for rclone: {}", e),
            }
        }

        let mount_point = &self.config.mount_point;
        #[cfg(target_os = "linux")]
        {
            let _ = StdCommand::new("fusermount")
                .args(["-u", &mount_point.to_string_lossy()])
                .output();
        }
        #[cfg(target_os = "macos")]
        {
            let _ = StdCommand::new("umount")
                .arg(&mount_point.to_string_lossy())
                .output();
        }

        let mut progress = self.progress.write().await;
        *progress = MountProgress::default();

        Ok(())
    }

    /// Check if rclone is available on the system
    pub fn check_rclone_available() -> Result<String> {
        let output = StdCommand::new("rclone")
            .args(["version"])
            .output()
            .map_err(|e| {
                anyhow::anyhow!(
                    "rclone not found: {}. Install it from https://rclone.org/install/",
                    e
                )
            })?;

        let version = String::from_utf8_lossy(&output.stdout);
        Ok(version.lines().next().unwrap_or("unknown").to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_rclone_available() {
        let result = RcloneManager::check_rclone_available();
        let _ = result;
    }
}
