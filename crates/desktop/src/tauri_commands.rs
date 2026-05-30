//! Tauri command wrappers for the desktop client.
//!
//! When `tauri` feature is enabled, these become `#[tauri::command]` handlers
//! that can be invoked from the Tauri frontend (JS/TS).
//!
//! Without the feature, they're plain async functions usable in tests and
//! the CLI-only mode.

use crate::commands::{ConfigResponse, DesktopState, MountStatusResponse, SaveConfigRequest};
use crate::rclone::MountProgress;

#[cfg(feature = "tauri")]
use tauri::State;

/// Mount the Ferro drive via rclone.
#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn cmd_mount(state: State<'_, DesktopState>) -> Result<String, String> {
    state.mount_drive().await
}

#[cfg(not(feature = "tauri"))]
pub async fn cmd_mount(state: &DesktopState) -> Result<String, String> {
    state.mount_drive().await
}

/// Unmount the Ferro drive.
#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn cmd_unmount(state: State<'_, DesktopState>) -> Result<String, String> {
    state.unmount_drive().await
}

#[cfg(not(feature = "tauri"))]
pub async fn cmd_unmount(state: &DesktopState) -> Result<String, String> {
    state.unmount_drive().await
}

/// Get current mount status.
#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn cmd_get_mount_status(
    state: State<'_, DesktopState>,
) -> Result<MountStatusResponse, String> {
    Ok(state.get_mount_status().await)
}

#[cfg(not(feature = "tauri"))]
pub async fn cmd_get_mount_status(state: &DesktopState) -> MountStatusResponse {
    state.get_mount_status().await
}

/// Get current configuration.
#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn cmd_get_config(state: State<'_, DesktopState>) -> Result<ConfigResponse, String> {
    Ok(state.get_config().await)
}

#[cfg(not(feature = "tauri"))]
pub async fn cmd_get_config(state: &DesktopState) -> ConfigResponse {
    state.get_config().await
}

/// Save configuration changes.
#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn cmd_save_config(
    state: State<'_, DesktopState>,
    request: SaveConfigRequest,
) -> Result<(), String> {
    state.save_config(request).await
}

#[cfg(not(feature = "tauri"))]
pub async fn cmd_save_config(
    state: &DesktopState,
    request: SaveConfigRequest,
) -> Result<(), String> {
    state.save_config(request).await
}

/// Get current mount progress (bytes transferred, speed, errors).
#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn cmd_get_mount_progress() -> Result<MountProgress, String> {
    Ok(MountProgress::default())
}

#[cfg(not(feature = "tauri"))]
pub async fn cmd_get_mount_progress(_state: &DesktopState) -> MountProgress {
    MountProgress::default()
}

/// Open a file or folder in the system file manager.
#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn cmd_open_path(path: String) -> Result<(), String> {
    open_path_inner(&path)
}

#[cfg(not(feature = "tauri"))]
pub async fn cmd_open_path(path: String) -> Result<(), String> {
    open_path_inner(&path)
}

fn open_path_inner(path: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Failed to open: {}", e))?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Failed to open: {}", e))?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", path])
            .spawn()
            .map_err(|e| format!("Failed to open: {}", e))?;
    }
    Ok(())
}

/// Show a notification in the system tray / notification center.
#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn cmd_show_notification(
    title: String,
    body: String,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    use tauri_plugin_notification::NotificationExt;
    app_handle
        .notification()
        .title(&title)
        .body(&body)
        .show()
        .map_err(|e| format!("Notification failed: {}", e))
}

/// Get the system mount point for the current platform.
#[cfg(feature = "tauri")]
#[tauri::command]
pub async fn cmd_default_mount_point() -> String {
    crate::config::DesktopConfig::default_mount_point()
        .display()
        .to_string()
}

#[cfg(not(feature = "tauri"))]
pub async fn cmd_default_mount_point() -> String {
    crate::config::DesktopConfig::default_mount_point()
        .display()
        .to_string()
}

/// Standalone mount — usable without Tauri runtime.
pub async fn standalone_mount(state: &DesktopState) -> Result<String, String> {
    state.mount_drive().await
}

/// Standalone unmount — usable without Tauri runtime.
pub async fn standalone_unmount(state: &DesktopState) -> Result<String, String> {
    state.unmount_drive().await
}

// ── Sync Commands ──────────────────────────────────────────────────

#[cfg(all(feature = "tauri", feature = "sync"))]
#[tauri::command]
pub async fn cmd_start_sync(state: State<'_, DesktopState>) -> Result<(), String> {
    state.start_sync().await
}

#[cfg(all(feature = "tauri", feature = "sync"))]
#[tauri::command]
pub async fn cmd_stop_sync(state: State<'_, DesktopState>) -> Result<(), String> {
    state.stop_sync().await
}

#[cfg(all(feature = "tauri", feature = "sync"))]
#[tauri::command]
pub async fn cmd_pause_sync(state: State<'_, DesktopState>) -> Result<(), String> {
    state.pause_sync();
    Ok(())
}

#[cfg(all(feature = "tauri", feature = "sync"))]
#[tauri::command]
pub async fn cmd_resume_sync(state: State<'_, DesktopState>) -> Result<(), String> {
    state.resume_sync();
    Ok(())
}

#[cfg(all(feature = "tauri", feature = "sync"))]
#[tauri::command]
pub async fn cmd_sync_now(state: State<'_, DesktopState>) -> Result<String, String> {
    let summary = state.sync_now().await?;
    serde_json::to_string(&summary).map_err(|e| e.to_string())
}

#[cfg(all(feature = "tauri", feature = "sync"))]
#[tauri::command]
pub async fn cmd_get_sync_status(
    state: State<'_, DesktopState>,
) -> Result<crate::commands::SyncStatusResponse, String> {
    Ok(state.get_sync_status().await)
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_default_mount_point() {
        let point = crate::config::DesktopConfig::default_mount_point();
        assert!(!point.as_os_str().is_empty());
    }
}
