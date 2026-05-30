use crate::rclone::MountStatus;

/// Tray menu actions (used by Tauri command handlers).
#[derive(Debug, Clone)]
pub enum TrayAction {
    Mount,
    Unmount,
    OpenBrowser,
    OpenFolder,
    ShowStatus,
    SyncNow,
    PauseSync,
    ResumeSync,
    Quit,
}

/// Get the tooltip for the tray icon based on mount status.
pub fn status_tooltip(status: &MountStatus) -> &'static str {
    match status {
        MountStatus::NotMounted => "Ferro: Not mounted",
        MountStatus::Mounting => "Ferro: Mounting...",
        MountStatus::Mounted { .. } => "Ferro: Connected",
        MountStatus::Error { .. } => "Ferro: Error",
        MountStatus::Unmounting => "Ferro: Unmounting...",
    }
}

/// Get the sync-specific tooltip suffix.
pub fn sync_tooltip_suffix(syncing: bool, paused: bool) -> &'static str {
    match (syncing, paused) {
        (true, _) => " | Syncing",
        (_, true) => " | Sync paused",
        _ => "",
    }
}
