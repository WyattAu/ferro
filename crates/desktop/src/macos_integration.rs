//! macOS native integration for the Ferro desktop application.
//!
//! This module provides:
//! - Finder Sync Extension configuration for status badges and toolbar
//! - Spotlight indexing hooks via `MDItem` integration
//! - Menu bar extra showing sync status
//! - Universal binary build configuration (x86_64 + aarch64)
//! - DMG installer configuration

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Finder Sync Extension
// ---------------------------------------------------------------------------

/// Configuration for the Finder Sync Extension.
///
/// The Finder Sync Extension provides:
/// - Green checkmarks on synced files
/// - Sync status in the Finder toolbar
/// - Badge overlays showing sync state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinderExtensionConfig {
    /// Bundle identifier for the Finder Sync Extension (e.g., "com.ferro.app.FinderSync").
    pub bundle_id: String,
    /// Path to the synced directory root.
    pub synced_folder_path: String,
    /// Whether to show status badges on files.
    pub show_status_badges: bool,
    /// Whether to show the toolbar button.
    pub show_toolbar_button: bool,
    /// Badge images for different sync states.
    pub badge_images: BadgeImages,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BadgeImages {
    /// Badge for files that are fully synced.
    pub synced: String,
    /// Badge for files currently syncing.
    pub syncing: String,
    /// Badge for files with sync errors.
    pub error: String,
    /// Badge for files pending upload.
    pub pending: String,
}

impl Default for FinderExtensionConfig {
    fn default() -> Self {
        Self {
            bundle_id: "com.ferro.app.FinderSync".to_string(),
            synced_folder_path: String::new(),
            show_status_badges: true,
            show_toolbar_button: true,
            badge_images: BadgeImages {
                synced: "BadgeSynced".to_string(),
                syncing: "BadgeSyncing".to_string(),
                error: "BadgeError".to_string(),
                pending: "BadgePending".to_string(),
            },
        }
    }
}

impl FinderExtensionConfig {
    /// Generate the Info.plist content for the Finder Sync Extension.
    pub fn info_plist(&self) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleIdentifier</key>
    <string>{}.FinderSync</string>
    <key>CFBundleName</key>
    <string>Ferro Finder Sync</string>
    <key>CFBundleDisplayName</key>
    <string>Ferro</string>
    <key>CFBundleVersion</key>
    <string>1</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0</string>
    <key>CFBundlePackageType</key>
    <string>XPC!</string>
    <key>NSExtension</key>
    <dict>
        <key>NSExtensionPointIdentifier</key>
        <string>com.apple.FinderSync</string>
        <key>NSExtensionPrincipalClass</key>
        <string>$(PRODUCT_MODULE_NAME).FinderSync</string>
    </dict>
</dict>
</plist>"#,
            self.bundle_id
        )
    }

    /// Generate the Swift stub for the Finder Sync Extension.
    pub fn swift_stub(&self) -> String {
        format!(
            r#"import Cocoa
import FinderSync

class FinderSync: FIFinderSync {{
    override init() {{
        super.init()
        let syncedPaths = ["{synced_folder}"]
        FIFinderSyncController.default().directoryURLs = Set(
            syncedPaths.map {{ URL(fileURLWithPath: $0) }}
        )
    }}

    override func beginObservingDirectory(at url: URL) {{
        // Start monitoring file changes in the given directory
        NSLog("Ferro: beginObservingDirectory at {{%@}}", url.path)
    }}

    override func endObservingDirectory(at url: URL) {{
        NSLog("Ferro: endObservingDirectory at {{%@}}", url.path)
    }}

    override func requestBadgeIdentifier(for url: URL) {{
        // Query the Ferro sync daemon for the badge state of the given URL
        NSLog("Ferro: requestBadgeIdentifier for {{%@}}", url.path)
    }}
}}
"#,
            synced_folder = self.synced_folder_path
        )
    }
}

// ---------------------------------------------------------------------------
// Spotlight Indexing Hooks
// ---------------------------------------------------------------------------

/// Hook into macOS Spotlight to index synced files.
///
/// Ferro can provide Spotlight integration by:
/// 1. Setting `kMDItemTextContent` and other MDItem attributes on synced files
/// 2. Providing a custom import plugin for file metadata
/// 3. Using `MDItemSetAttribute` for custom Ferro-specific attributes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotlightConfig {
    /// Whether Spotlight indexing is enabled.
    pub enabled: bool,
    /// Custom Spotlight attributes to set on synced files.
    pub custom_attributes: Vec<SpotlightAttribute>,
    /// File extensions to exclude from Spotlight indexing.
    pub excluded_extensions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotlightAttribute {
    /// The MDItem key (e.g., "kMDItemKeywords", "com_ferro_sync_status").
    pub key: String,
    /// The attribute value.
    pub value: String,
}

impl Default for SpotlightConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            custom_attributes: vec![
                SpotlightAttribute {
                    key: "com.ferro.syncStatus".to_string(),
                    value: "synced".to_string(),
                },
                SpotlightAttribute {
                    key: "com.ferro.lastSyncTime".to_string(),
                    value: String::new(),
                },
            ],
            excluded_extensions: vec!["db".to_string(), "db-journal".to_string(), "tmp".to_string()],
        }
    }
}

impl SpotlightConfig {
    /// Generate mdimporter Info.plist for the custom Spotlight attributes.
    pub fn mdimporter_plist(&self) -> String {
        let mut attributes = String::new();
        for attr in &self.custom_attributes {
            attributes.push_str(&format!(
                r#"
        <dict>
            <key>CFBundleAttributeKey</key>
            <string>{}</string>
            <key>CFBundleAttributeName</key>
            <string>{}</string>
            <key>CFBundleAttributeType</key>
            <string>String</string>
            <key>CFBundleAttributeDefaultValue</key>
            <string></string>
        </dict>"#,
                attr.key, attr.key
            ));
        }

        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleExecutable</key>
    <string>$(EXECUTABLE_NAME)</string>
    <key>CFBundleIdentifier</key>
    <string>com.ferro.app.mdimporter</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>FerroImporter</string>
    <key>CFBundlePackageType</key>
    <string>BNDL</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0</string>
    <key>CFBundleVersion</key>
    <string>1</string>
    <key>NSPlugin</key>
    <dict>
        <key>Extensions</key>
        <array>{}</array>
        <key>NSExtensionPointName</key>
        <string>com.apple.metadata-importer</string>
        <key>NSExtensionPointVersion</key>
        <string>1.0</string>
    </dict>
</dict>
</plist>"#,
            attributes
        )
    }

    /// Check if a file extension should be excluded from indexing.
    pub fn should_index(&self, path: &str) -> bool {
        if !self.enabled {
            return false;
        }
        path.rsplit('.')
            .next()
            .map(|ext| !self.excluded_extensions.contains(&ext.to_lowercase()))
            .unwrap_or(true)
    }
}

// ---------------------------------------------------------------------------
// Menu Bar Extra
// ---------------------------------------------------------------------------

/// Status displayed in the macOS menu bar.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SyncStatusBarState {
    /// All files are synced.
    Synced,
    /// Files are currently being synced.
    Syncing { progress: f32 },
    /// An error occurred during sync.
    Error { message: String },
    /// The app is offline.
    Offline,
    /// Sync is paused by the user.
    Paused,
}

impl SyncStatusBarState {
    pub fn icon_name(&self) -> &'static str {
        match self {
            SyncStatusBarState::Synced => "StatusBarSynced",
            SyncStatusBarState::Syncing { .. } => "StatusBarSyncing",
            SyncStatusBarState::Error { .. } => "StatusBarError",
            SyncStatusBarState::Offline => "StatusBarOffline",
            SyncStatusBarState::Paused => "StatusBarPaused",
        }
    }

    pub fn tooltip(&self) -> String {
        match self {
            SyncStatusBarState::Synced => "Ferro: All files synced".to_string(),
            SyncStatusBarState::Syncing { progress } => {
                format!("Ferro: Syncing... ({:.0}%)", progress * 100.0)
            }
            SyncStatusBarState::Error { message } => {
                format!("Ferro: Sync error - {}", message)
            }
            SyncStatusBarState::Offline => "Ferro: Offline".to_string(),
            SyncStatusBarState::Paused => "Ferro: Sync paused".to_string(),
        }
    }
}

/// Menu bar configuration for the Ferro macOS app.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenuBarConfig {
    /// Whether to show the menu bar icon.
    pub show_icon: bool,
    /// Whether to show sync progress percentage.
    pub show_progress: bool,
    /// Click action for the menu bar icon.
    pub click_action: MenuBarClickAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MenuBarClickAction {
    /// Open the main Ferro window.
    OpenWindow,
    /// Show a dropdown menu with sync status.
    ShowMenu,
    /// Toggle sync pause.
    ToggleSync,
}

impl Default for MenuBarConfig {
    fn default() -> Self {
        Self {
            show_icon: true,
            show_progress: true,
            click_action: MenuBarClickAction::ShowMenu,
        }
    }
}

// ---------------------------------------------------------------------------
// Universal Binary Build Configuration
// ---------------------------------------------------------------------------

/// Build configuration for creating a universal macOS binary (x86_64 + aarch64).
#[derive(Debug, Clone)]
pub struct UniversalBinaryConfig {
    /// x86_64 target triple.
    pub x86_64_target: String,
    /// aarch64 (Apple Silicon) target triple.
    pub aarch64_target: String,
    /// Output path for the universal binary.
    pub output_path: String,
    /// Whether to strip debug symbols from the universal binary.
    pub strip: bool,
    /// Minimum macOS version for deployment target.
    pub min_macos_version: String,
}

impl Default for UniversalBinaryConfig {
    fn default() -> Self {
        Self {
            x86_64_target: "x86_64-apple-darwin".to_string(),
            aarch64_target: "aarch64-apple-darwin".to_string(),
            output_path: "target/universal-apple-darwin/ferro-desktop".to_string(),
            strip: true,
            min_macos_version: "12.0".to_string(),
        }
    }
}

impl UniversalBinaryConfig {
    /// Generate the cargo build commands for each architecture.
    pub fn build_commands(&self) -> Vec<String> {
        vec![
            format!("cargo build --release --target {} --features tauri", self.x86_64_target),
            format!(
                "cargo build --release --target {} --features tauri",
                self.aarch64_target
            ),
            format!(
                "lipo -create target/{}/release/ferro-desktop target/{}/release/ferro-desktop -output {}",
                self.x86_64_target, self.aarch64_target, self.output_path
            ),
            format!("install_name_tool -id @rpath/ferro-desktop {}", self.output_path),
        ]
    }

    /// Generate the build script content for CI.
    pub fn ci_build_script(&self) -> String {
        let commands = self.build_commands();
        let mut script = String::from("#!/bin/bash\nset -euo pipefail\n\n");
        script.push_str("# Universal macOS binary build script\n\n");
        for cmd in &commands {
            script.push_str(&format!("{}\n\n", cmd));
        }
        if self.strip {
            script.push_str(&format!("strip -x {}\n", self.output_path));
        }
        script.push_str(&format!("echo \"Universal binary created at {}\"\n", self.output_path));
        script
    }
}

// ---------------------------------------------------------------------------
// DMG Installer Configuration
// ---------------------------------------------------------------------------

/// Configuration for generating the DMG installer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DmgConfig {
    /// Volume name shown when DMG is mounted.
    pub volume_name: String,
    /// Width of the DMG window.
    pub window_width: f64,
    /// Height of the DMG window.
    pub window_height: f64,
    /// Position of the Applications symlink (x, y).
    pub applications_position: (f64, f64),
    /// Position of the app icon (x, y).
    pub app_position: (f64, f64),
    /// Background image path (optional).
    pub background_image: Option<String>,
    /// Whether to accept any license agreement.
    pub accept_eula: bool,
    /// Signing identity for code signing.
    pub codesign_identity: Option<String>,
    /// Notarization Apple ID (optional).
    pub notarize_apple_id: Option<String>,
    /// Notarization team ID (optional).
    pub notarize_team_id: Option<String>,
    /// Notarization password keychain profile.
    pub notarize_password: Option<String>,
}

impl Default for DmgConfig {
    fn default() -> Self {
        Self {
            volume_name: "Ferro".to_string(),
            window_width: 660.0,
            window_height: 400.0,
            applications_position: (180.0, 170.0),
            app_position: (480.0, 170.0),
            background_image: None,
            accept_eula: false,
            codesign_identity: None,
            notarize_apple_id: None,
            notarize_team_id: None,
            notarize_password: None,
        }
    }
}

impl DmgConfig {
    /// Generate the create-dmg command for building the DMG installer.
    pub fn create_dmg_command(&self, app_path: &str, dmg_output: &str) -> String {
        let mut cmd = format!(
            "create-dmg \
                --volname \"{}\" \
                --window-pos {} {} \
                --window-size {} {} \
                --app-drop-link {} {} \
                --icon-positions {} {} \
                \"{}\" \
                \"{}\"",
            self.volume_name,
            self.applications_position.0 as u64,
            self.applications_position.1 as u64,
            self.window_width as u64,
            self.window_height as u64,
            self.applications_position.0,
            self.applications_position.1,
            self.app_position.0,
            self.app_position.1,
            app_path,
            dmg_output,
        );

        if let Some(ref bg) = self.background_image {
            cmd.push_str(&format!(" --background \"{}\"", bg));
        }

        if self.accept_eula {
            cmd.push_str(" --no-internet-enable");
        }

        cmd
    }

    /// Generate the notarization command using `xcrun notarytool`.
    pub fn notarize_command(&self, dmg_path: &str) -> Option<String> {
        let apple_id = self.notarize_apple_id.as_ref()?;
        let team_id = self.notarize_team_id.as_ref()?;
        let password = self.notarize_password.as_ref()?;

        Some(format!(
            "xcrun notarytool submit \"{}\" \
                --apple-id \"{}\" \
                --team-id \"{}\" \
                --password \"{}\" \
                --wait",
            dmg_path, apple_id, team_id, password
        ))
    }

    /// Generate the stapling command for the notarized DMG.
    pub fn staple_command(&self, dmg_path: &str) -> String {
        format!("xcrun stapler staple \"{}\"", dmg_path)
    }
}

// ---------------------------------------------------------------------------
// Integration: Wire into AppState
// ---------------------------------------------------------------------------

/// Create a `FinderExtensionConfig` with the given synced folder.
pub fn finder_sync_config(synced_folder: &str) -> FinderExtensionConfig {
    let mut config = FinderExtensionConfig::default();
    config.synced_folder_path = synced_folder.to_string();
    config
}

/// Create a `SpotlightConfig` for the given data directory.
pub fn spotlight_config(data_dir: &str) -> SpotlightConfig {
    let mut config = SpotlightConfig::default();
    // Exclude Ferro internal files from Spotlight
    config.excluded_extensions.extend(vec![
        "db-wal".to_string(),
        "db-shm".to_string(),
        "db-journal".to_string(),
    ]);
    config
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_finder_sync_info_plist() {
        let config = FinderExtensionConfig {
            synced_folder_path: "/Users/test/Documents".to_string(),
            ..Default::default()
        };
        let plist = config.info_plist();
        assert!(plist.contains("com.ferro.app.FinderSync.FinderSync"));
        assert!(plist.contains("com.apple.FinderSync"));
    }

    #[test]
    fn test_finder_sync_swift_stub() {
        let config = FinderExtensionConfig {
            synced_folder_path: "/Users/test/Documents".to_string(),
            ..Default::default()
        };
        let swift = config.swift_stub();
        assert!(swift.contains("/Users/test/Documents"));
        assert!(swift.contains("FIFinderSync"));
    }

    #[test]
    fn test_spotlight_should_index() {
        let config = SpotlightConfig::default();
        assert!(config.should_index("file.txt"));
        assert!(config.should_index("photo.jpg"));
        assert!(!config.should_index("data.db"));
        assert!(!config.should_index("file.tmp"));
    }

    #[test]
    fn test_spotlight_disabled() {
        let config = SpotlightConfig {
            enabled: false,
            ..Default::default()
        };
        assert!(!config.should_index("anything.txt"));
    }

    #[test]
    fn test_sync_status_bar_state() {
        let synced = SyncStatusBarState::Synced;
        assert_eq!(synced.icon_name(), "StatusBarSynced");
        assert!(synced.tooltip().contains("synced"));

        let syncing = SyncStatusBarState::Syncing { progress: 0.5 };
        assert_eq!(syncing.icon_name(), "StatusBarSyncing");
        assert!(syncing.tooltip().contains("50%"));

        let error = SyncStatusBarState::Error {
            message: "timeout".to_string(),
        };
        assert_eq!(error.icon_name(), "StatusBarError");
        assert!(error.tooltip().contains("timeout"));
    }

    #[test]
    fn test_universal_binary_build_commands() {
        let config = UniversalBinaryConfig::default();
        let cmds = config.build_commands();
        assert_eq!(cmds.len(), 4);
        assert!(cmds[0].contains("x86_64-apple-darwin"));
        assert!(cmds[1].contains("aarch64-apple-darwin"));
        assert!(cmds[2].contains("lipo -create"));
    }

    #[test]
    fn test_dmg_config_command() {
        let config = DmgConfig::default();
        let cmd = config.create_dmg_command("/path/to/Ferro.app", "/path/to/Ferro.dmg");
        assert!(cmd.contains("create-dmg"));
        assert!(cmd.contains("--volname \"Ferro\""));
        assert!(cmd.contains("/path/to/Ferro.app"));
        assert!(cmd.contains("/path/to/Ferro.dmg"));
    }

    #[test]
    fn test_dmg_notarize_command() {
        let config = DmgConfig {
            notarize_apple_id: Some("user@example.com".to_string()),
            notarize_team_id: Some("TEAM123".to_string()),
            notarize_password: Some("@keychain:notary".to_string()),
            ..Default::default()
        };
        let cmd = config.notarize_command("/path/to/Ferro.dmg").unwrap();
        assert!(cmd.contains("xcrun notarytool submit"));
        assert!(cmd.contains("--apple-id \"user@example.com\""));
        assert!(cmd.contains("--team-id \"TEAM123\""));
    }

    #[test]
    fn test_dmg_notarize_no_credentials() {
        let config = DmgConfig::default();
        assert!(config.notarize_command("/path/to/Ferro.dmg").is_none());
    }

    #[test]
    fn test_dmg_staple_command() {
        let config = DmgConfig::default();
        let cmd = config.staple_command("/path/to/Ferro.dmg");
        assert_eq!(cmd, "xcrun stapler staple \"/path/to/Ferro.dmg\"");
    }
}
