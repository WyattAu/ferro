//! File Manager Overlay Integration
//!
//! Provides platform-specific overlay integrations for file managers:
//! - macOS: Finder Sync Extension for badge icons and context menus
//! - Windows: Explorer overlay shell extension for sync status icons
//! - Linux: File manager integration via D-Bus or custom icon themes
//!
//! # Architecture
//!
//! Each platform implements its own overlay system using native APIs:
//! - **macOS**: Uses `FileSync` framework with `FIFinderSyncProtocol`
//! - **Windows**: Uses `IOverlayIdentifier` COM interface via shell extension
//! - **Linux**: Uses D-Bus notifications and icon themes for file managers
//!
//! The overlay system displays sync status badges on files/folders and provides
//! quick-access context menu actions for sync operations.

use serde::{Deserialize, Serialize};

/// Configuration for macOS Finder Sync Extension overlay.
///
/// The Finder Sync Extension runs as a separate process and communicates
/// with the main Ferro app via XPC or shared memory.
///
/// # Architecture
///
/// ```text
/// ┌─────────────────────────────────────┐
/// │         Finder.app                   │
/// │  ┌───────────────────────────────┐  │
/// │  │   Sync Extension (badge)      │  │
/// │  └───────────┬───────────────────┘  │
/// └──────────────┼──────────────────────┘
///                │ XPC / Shared Memory
/// ┌──────────────┴──────────────────────┐
/// │      Ferro Desktop App              │
/// │  ┌───────────────────────────────┐  │
/// │  │   Sync Coordinator            │  │
/// │  └───────────────────────────────┘  │
/// └─────────────────────────────────────┘
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlaySyncConfig {
    /// Enable Finder badge icons for sync status
    pub enable_badges: bool,
    /// Enable context menu items in Finder
    pub enable_context_menu: bool,
    /// Paths to monitor for badge display
    pub watched_paths: Vec<String>,
    /// Badge style: "dot", "checkmark", or "sync"
    pub badge_style: BadgeStyle,
    /// Show sync progress in badge
    pub show_progress: bool,
    /// Enable toolbar button for quick sync
    pub enable_toolbar: bool,
    /// Notification settings for Finder events
    pub notifications: FinderNotificationConfig,
}

/// Badge visual style for Finder overlay.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BadgeStyle {
    /// Simple colored dot (green=synced, yellow=syncing, red=error)
    #[default]
    Dot,
    /// Checkmark for synced items
    Checkmark,
    /// Sync arrows animation
    Sync,
    /// Cloud icon
    Cloud,
}

/// Notification configuration for Finder Sync Extension.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinderNotificationConfig {
    /// Show notification when sync completes
    pub notify_on_sync_complete: bool,
    /// Show notification on sync errors
    pub notify_on_error: bool,
    /// Show notification for new shared files
    pub notify_on_share: bool,
    /// Sound name for notifications (NULL for default)
    pub notification_sound: Option<String>,
}

impl Default for FinderNotificationConfig {
    fn default() -> Self {
        Self {
            notify_on_sync_complete: true,
            notify_on_error: true,
            notify_on_share: true,
            notification_sound: None,
        }
    }
}

impl Default for OverlaySyncConfig {
    fn default() -> Self {
        Self {
            enable_badges: true,
            enable_context_menu: true,
            watched_paths: Vec::new(),
            badge_style: BadgeStyle::Dot,
            show_progress: true,
            enable_toolbar: false,
            notifications: FinderNotificationConfig::default(),
        }
    }
}

/// Configuration for Windows Explorer overlay shell extension.
///
/// The Explorer overlay uses a COM shell extension to render custom
/// icon overlays on files managed by Ferro.
///
/// # Architecture
///
/// ```text
/// ┌─────────────────────────────────────┐
/// │         Explorer.exe                │
/// │  ┌───────────────────────────────┐  │
/// │  │   Shell Extension (COM)       │  │
/// │  │   IOverlayIdentifier          │  │
/// │  └───────────┬───────────────────┘  │
/// └──────────────┼──────────────────────┘
///                │ COM / Named Pipes
/// ┌──────────────┴──────────────────────┐
/// │      Ferro Desktop App              │
/// │  ┌───────────────────────────────┐  │
/// │  │   Sync Coordinator            │  │
/// │  └───────────────────────────────┘  │
/// └─────────────────────────────────────┘
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplorerOverlayConfig {
    /// Enable icon overlay in Explorer
    pub enable_overlays: bool,
    /// Enable column handler for sync status details
    pub enable_column_handler: bool,
    /// Enable property sheet extension
    pub enable_property_sheet: bool,
    /// Paths to monitor for overlay display
    pub monitored_paths: Vec<String>,
    /// Overlay icon set configuration
    pub icon_set: OverlayIconSet,
    /// Maximum number of overlay icons (Windows limit is typically 15)
    pub max_overlay_icons: u32,
    /// Enable context menu integration
    pub enable_context_menu: bool,
    /// Auto-start with Windows
    pub auto_start: bool,
    /// Explorer restart notification
    pub notify_explorer_restart: bool,
}

/// Configuration for Windows Explorer overlay icon set.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlayIconSet {
    /// Icon for synced files (path to .ico file)
    pub synced_icon: String,
    /// Icon for syncing files
    pub syncing_icon: String,
    /// Icon for files with errors
    pub error_icon: String,
    /// Icon for files pending sync
    pub pending_icon: String,
    /// Icon for shared files
    pub shared_icon: String,
}

impl Default for OverlayIconSet {
    fn default() -> Self {
        Self {
            synced_icon: "%LOCALAPPDATA%\\Ferro\\icons\\synced.ico".to_string(),
            syncing_icon: "%LOCALAPPDATA%\\Ferro\\icons\\syncing.ico".to_string(),
            error_icon: "%LOCALAPPDATA%\\Ferro\\icons\\error.ico".to_string(),
            pending_icon: "%LOCALAPPDATA%\\Ferro\\icons\\pending.ico".to_string(),
            shared_icon: "%LOCALAPPDATA%\\Ferro\\icons\\shared.ico".to_string(),
        }
    }
}

impl Default for ExplorerOverlayConfig {
    fn default() -> Self {
        Self {
            enable_overlays: true,
            enable_column_handler: true,
            enable_property_sheet: true,
            monitored_paths: Vec::new(),
            icon_set: OverlayIconSet::default(),
            max_overlay_icons: 4,
            enable_context_menu: true,
            auto_start: true,
            notify_explorer_restart: false,
        }
    }
}

/// Configuration for Linux file manager integration.
///
/// Linux integration uses D-Bus for file manager communication and
/// standard icon themes for overlay icons.
///
/// # Architecture
///
/// ```text
/// ┌─────────────────────────────────────┐
/// │   Nautilus / Dolphin / Thunar       │
/// │  ┌───────────────────────────────┐  │
/// │  │   D-Bus Interface             │  │
/// │  └───────────┬───────────────────┘  │
/// └──────────────┼──────────────────────┘
///                │ D-Bus
/// ┌──────────────┴──────────────────────┐
/// │      Ferro Desktop App              │
/// │  ┌───────────────────────────────┐  │
///  │   D-Bus Service                 │  │
/// │  └───────────────────────────────┘  │
/// └─────────────────────────────────────┘
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinuxFilemanagerConfig {
    /// Enable D-Bus service for file manager integration
    pub enable_dbus_service: bool,
    /// Enable Nautilus extension (GNOME)
    pub enable_nautilus_extension: bool,
    /// Enable Dolphin service (KDE)
    pub enable_dolphin_service: bool,
    /// Enable Thunar custom actions (XFCE)
    pub enable_thunar_actions: bool,
    /// Icon theme name for overlay icons
    pub icon_theme_name: String,
    /// Paths to monitor
    pub watched_paths: Vec<String>,
    /// D-Bus service name
    pub dbus_service_name: String,
    /// Enable desktop notifications
    pub enable_notifications: bool,
}

impl Default for LinuxFilemanagerConfig {
    fn default() -> Self {
        Self {
            enable_dbus_service: true,
            enable_nautilus_extension: true,
            enable_dolphin_service: true,
            enable_thunar_actions: false,
            icon_theme_name: "ferro-sync".to_string(),
            watched_paths: Vec::new(),
            dbus_service_name: "com.ferro.DesktopSync".to_string(),
            enable_notifications: true,
        }
    }
}

/// Unified overlay configuration for all platforms.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlayConfig {
    /// Enable overlay integration globally
    pub enabled: bool,
    /// macOS Finder Sync configuration
    #[cfg(target_os = "macos")]
    pub macos: OverlaySyncConfig,
    /// Windows Explorer overlay configuration
    #[cfg(target_os = "windows")]
    pub windows: ExplorerOverlayConfig,
    /// Linux file manager configuration
    #[cfg(target_os = "linux")]
    pub linux: LinuxFilemanagerConfig,
    /// Global settings
    pub global: OverlayGlobalConfig,
}

/// Global overlay settings that apply across all platforms.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlayGlobalConfig {
    /// Auto-enable overlay on first run
    pub auto_enable: bool,
    /// Show overlay only for synced folders (not all files)
    pub synced_only: bool,
    /// Minimum sync status change interval (ms) to prevent icon flicker
    pub min_update_interval_ms: u64,
    /// Log overlay operations for debugging
    pub debug_logging: bool,
}

impl Default for OverlayGlobalConfig {
    fn default() -> Self {
        Self {
            auto_enable: true,
            synced_only: true,
            min_update_interval_ms: 500,
            debug_logging: false,
        }
    }
}

impl Default for OverlayConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            #[cfg(target_os = "macos")]
            macos: OverlaySyncConfig::default(),
            #[cfg(target_os = "windows")]
            windows: ExplorerOverlayConfig::default(),
            #[cfg(target_os = "linux")]
            linux: LinuxFilemanagerConfig::default(),
            global: OverlayGlobalConfig::default(),
        }
    }
}

/// Sync status to display as overlay badge/icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncStatus {
    /// File is fully synced with server
    Synced,
    /// File is currently being synced
    Syncing,
    /// File has sync conflict
    Conflict,
    /// File has sync error
    Error,
    /// File is pending sync (queued)
    Pending,
    /// File is not being synced (excluded)
    Excluded,
}

/// Overlay badge information for a file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlayBadge {
    /// Sync status
    pub status: SyncStatus,
    /// Progress percentage (0-100) for syncing state
    pub progress: Option<u8>,
    /// Last sync timestamp
    pub last_sync: Option<chrono::DateTime<chrono::Utc>>,
    /// Conflict details if status is Conflict
    pub conflict_info: Option<ConflictInfo>,
}

/// Information about a sync conflict.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictInfo {
    /// Local version timestamp
    pub local_modified: chrono::DateTime<chrono::Utc>,
    /// Remote version timestamp
    pub remote_modified: chrono::DateTime<chrono::Utc>,
    /// Path to remote version
    pub remote_path: Option<String>,
}

/// Overlay manager trait for platform-specific implementations.
pub trait OverlayManager: Send + Sync {
    /// Initialize the overlay system
    fn initialize(&mut self) -> Result<(), OverlayError>;

    /// Update badge for a specific path
    fn update_badge(&self, path: &str, badge: OverlayBadge) -> Result<(), OverlayError>;

    /// Remove badge from a path
    fn remove_badge(&self, path: &str) -> Result<(), OverlayError>;

    /// Refresh all badges
    fn refresh_all(&self) -> Result<(), OverlayError>;

    /// Shutdown the overlay system
    fn shutdown(&mut self) -> Result<(), OverlayError>;
}

/// Errors that can occur in overlay operations.
#[derive(Debug, thiserror::Error)]
pub enum OverlayError {
    #[error("overlay not supported on this platform")]
    Unsupported,

    #[error("overlay initialization failed: {0}")]
    InitFailed(String),

    #[error("badge update failed: {0}")]
    BadgeUpdateFailed(String),

    #[error("communication error: {0}")]
    CommunicationError(String),

    #[error("permission denied: {0}")]
    PermissionDenied(String),

    #[error("resource limit exceeded: {0}")]
    ResourceLimit(String),
}

/// Stub implementation of OverlayManager for unsupported platforms.
pub struct StubOverlayManager;

impl OverlayManager for StubOverlayManager {
    fn initialize(&mut self) -> Result<(), OverlayError> {
        tracing::info!("stub overlay manager initialized (no-op)");
        Ok(())
    }

    fn update_badge(&self, _path: &str, _badge: OverlayBadge) -> Result<(), OverlayError> {
        Ok(())
    }

    fn remove_badge(&self, _path: &str) -> Result<(), OverlayError> {
        Ok(())
    }

    fn refresh_all(&self) -> Result<(), OverlayError> {
        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), OverlayError> {
        Ok(())
    }
}

/// Create the appropriate overlay manager for the current platform.
pub fn create_overlay_manager(config: &OverlayConfig) -> Box<dyn OverlayManager> {
    if !config.enabled {
        return Box::new(StubOverlayManager);
    }

    #[cfg(target_os = "macos")]
    {
        return Box::new(MacosOverlayManager::new(&config.macos));
    }

    #[cfg(target_os = "windows")]
    {
        return Box::new(WindowsOverlayManager::new(&config.windows));
    }

    #[cfg(target_os = "linux")]
    {
        Box::new(LinuxOverlayManager::new(&config.linux))
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        Box::new(StubOverlayManager)
    }
}

#[cfg(target_os = "macos")]
pub struct MacosOverlayManager {
    config: OverlaySyncConfig,
    initialized: bool,
}

#[cfg(target_os = "macos")]
impl MacosOverlayManager {
    pub fn new(config: &OverlaySyncConfig) -> Self {
        Self {
            config: config.clone(),
            initialized: false,
        }
    }
}

#[cfg(target_os = "macos")]
impl OverlayManager for MacosOverlayManager {
    fn initialize(&mut self) -> Result<(), OverlayError> {
        tracing::info!("initializing macOS Finder Sync overlay");
        // TODO: Implement Finder Sync Extension launch and XPC connection
        self.initialized = true;
        Ok(())
    }

    fn update_badge(&self, _path: &str, _badge: OverlayBadge) -> Result<(), OverlayError> {
        if !self.initialized {
            return Err(OverlayError::InitFailed("not initialized".to_string()));
        }
        // TODO: Send badge update via XPC to Finder Sync Extension
        Ok(())
    }

    fn remove_badge(&self, _path: &str) -> Result<(), OverlayError> {
        // TODO: Remove badge via XPC
        Ok(())
    }

    fn refresh_all(&self) -> Result<(), OverlayError> {
        // TODO: Refresh all badges
        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), OverlayError> {
        tracing::info!("shutting down macOS Finder Sync overlay");
        // TODO: Disconnect XPC and terminate Finder Sync Extension
        self.initialized = false;
        Ok(())
    }
}

#[cfg(target_os = "windows")]
pub struct WindowsOverlayManager {
    config: ExplorerOverlayConfig,
    initialized: bool,
}

#[cfg(target_os = "windows")]
impl WindowsOverlayManager {
    pub fn new(config: &ExplorerOverlayConfig) -> Self {
        Self {
            config: config.clone(),
            initialized: false,
        }
    }
}

#[cfg(target_os = "windows")]
impl OverlayManager for WindowsOverlayManager {
    fn initialize(&mut self) -> Result<(), OverlayError> {
        tracing::info!("initializing Windows Explorer overlay");
        // TODO: Register COM shell extension
        self.initialized = true;
        Ok(())
    }

    fn update_badge(&self, _path: &str, _badge: OverlayBadge) -> Result<(), OverlayError> {
        if !self.initialized {
            return Err(OverlayError::InitFailed("not initialized".to_string()));
        }
        // TODO: Update overlay icon via COM interface
        Ok(())
    }

    fn remove_badge(&self, _path: &str) -> Result<(), OverlayError> {
        // TODO: Remove overlay via COM
        Ok(())
    }

    fn refresh_all(&self) -> Result<(), OverlayError> {
        // TODO: Refresh overlays
        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), OverlayError> {
        tracing::info!("shutting down Windows Explorer overlay");
        // TODO: Unregister COM shell extension
        self.initialized = false;
        Ok(())
    }
}

#[cfg(target_os = "linux")]
pub struct LinuxOverlayManager {
    _config: LinuxFilemanagerConfig,
    initialized: bool,
}

#[cfg(target_os = "linux")]
impl LinuxOverlayManager {
    pub fn new(config: &LinuxFilemanagerConfig) -> Self {
        Self {
            _config: config.clone(),
            initialized: false,
        }
    }
}

#[cfg(target_os = "linux")]
impl OverlayManager for LinuxOverlayManager {
    fn initialize(&mut self) -> Result<(), OverlayError> {
        tracing::info!("initializing Linux file manager overlay");
        // TODO: Register D-Bus service
        self.initialized = true;
        Ok(())
    }

    fn update_badge(&self, _path: &str, _badge: OverlayBadge) -> Result<(), OverlayError> {
        if !self.initialized {
            return Err(OverlayError::InitFailed("not initialized".to_string()));
        }
        // TODO: Emit D-Bus signal for badge update
        Ok(())
    }

    fn remove_badge(&self, _path: &str) -> Result<(), OverlayError> {
        // TODO: Emit D-Bus signal for badge removal
        Ok(())
    }

    fn refresh_all(&self) -> Result<(), OverlayError> {
        // TODO: Refresh all badges
        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), OverlayError> {
        tracing::info!("shutting down Linux file manager overlay");
        // TODO: Unregister D-Bus service
        self.initialized = false;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_finder_sync_config_default() {
        let config = OverlaySyncConfig::default();
        assert!(config.enable_badges);
        assert!(config.enable_context_menu);
        assert!(config.watched_paths.is_empty());
        assert_eq!(config.badge_style, BadgeStyle::Dot);
    }

    #[test]
    fn test_explorer_overlay_config_default() {
        let config = ExplorerOverlayConfig::default();
        assert!(config.enable_overlays);
        assert!(config.enable_column_handler);
        assert!(config.max_overlay_icons <= 15);
    }

    #[test]
    fn test_linux_filemanager_config_default() {
        let config = LinuxFilemanagerConfig::default();
        assert!(config.enable_dbus_service);
        assert!(config.enable_nautilus_extension);
    }

    #[test]
    fn test_overlay_config_default() {
        let config = OverlayConfig::default();
        assert!(config.enabled);
    }

    #[test]
    fn test_stub_overlay_manager() {
        let mut manager = StubOverlayManager;
        assert!(manager.initialize().is_ok());
        assert!(
            manager
                .update_badge(
                    "/test",
                    OverlayBadge {
                        status: SyncStatus::Synced,
                        progress: None,
                        last_sync: None,
                        conflict_info: None,
                    }
                )
                .is_ok()
        );
        assert!(manager.remove_badge("/test").is_ok());
        assert!(manager.refresh_all().is_ok());
        assert!(manager.shutdown().is_ok());
    }

    #[test]
    fn test_sync_status_serialization() {
        let status = SyncStatus::Syncing;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"syncing\"");
        let de: SyncStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(de, SyncStatus::Syncing);
    }

    #[test]
    fn test_overlay_badge_serialization() {
        let badge = OverlayBadge {
            status: SyncStatus::Conflict,
            progress: None,
            last_sync: None,
            conflict_info: Some(ConflictInfo {
                local_modified: chrono::Utc::now(),
                remote_modified: chrono::Utc::now(),
                remote_path: Some("/remote/file.txt".to_string()),
            }),
        };
        let json = serde_json::to_string(&badge).unwrap();
        let de: OverlayBadge = serde_json::from_str(&json).unwrap();
        assert_eq!(de.status, SyncStatus::Conflict);
        assert!(de.conflict_info.is_some());
    }

    #[test]
    fn test_overlay_error_display() {
        let err = OverlayError::InitFailed("test failure".to_string());
        assert!(err.to_string().contains("test failure"));
    }

    #[test]
    fn test_create_overlay_manager_disabled() {
        let config = OverlayConfig {
            enabled: false,
            ..Default::default()
        };
        let mut manager = create_overlay_manager(&config);
        assert!(manager.initialize().is_ok());
        assert!(manager.shutdown().is_ok());
    }
}
