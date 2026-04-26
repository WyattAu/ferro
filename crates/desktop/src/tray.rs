use crate::rclone::MountStatus;

/// Tray menu actions (these will be wrapped by Tauri commands)
#[derive(Debug, Clone)]
pub enum TrayAction {
    Mount,
    Unmount,
    OpenBrowser,
    OpenFolder,
    ShowStatus,
    Quit,
}

/// Get the status text for the tray icon tooltip
pub fn status_tooltip(status: &MountStatus) -> &'static str {
    match status {
        MountStatus::NotMounted => "Ferro: Not mounted",
        MountStatus::Mounting => "Ferro: Mounting...",
        MountStatus::Mounted { .. } => "Ferro: Connected",
        MountStatus::Error { .. } => "Ferro: Error",
        MountStatus::Unmounting => "Ferro: Unmounting...",
    }
}
