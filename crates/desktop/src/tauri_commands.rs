//! Tauri sync command wrappers for the desktop client.

#[cfg(all(feature = "tauri", feature = "sync"))]
use crate::commands::DesktopState;

#[cfg(all(feature = "tauri", feature = "sync"))]
use tauri::State;

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
