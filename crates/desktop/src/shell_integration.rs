//! Windows Shell Integration
//!
//! Provides context menu entries and Explorer integration for Ferro on Windows.
//! Registers shell extensions via the Windows Registry so users can right-click
//! files/folders and perform sync operations.

#[cfg(target_os = "windows")]
mod windows {
    use std::process::Command;
    use tracing::{error, info};

    /// Registry key path for the Ferro shell extension.
    const REGISTRY_KEY: &str = r"Software\Classes\Directory\Background\shell\Ferro";
    const SHELL_VERB: &str = "Sync with Ferro";

    /// Register the Windows Explorer context menu entry for directories.
    /// This adds a "Sync with Ferro" option when right-clicking folder backgrounds.
    pub fn register_context_menu() -> Result<(), String> {
        // Create the shell verb key
        let status = Command::new("reg")
            .args([
                "add",
                REGISTRY_KEY,
                "/ve",
                "/t",
                "REG_SZ",
                "/d",
                SHELL_VERB,
                "/f",
            ])
            .status()
            .map_err(|e| format!("Failed to run reg add: {}", e))?;

        if !status.success() {
            return Err("Failed to register context menu key".to_string());
        }

        // Set the icon
        let icon_status = Command::new("reg")
            .args([
                "add",
                &format!("{}\\command", REGISTRY_KEY),
                "/ve",
                "/t",
                "REG_SZ",
                "/d",
                "\"%LOCALAPPDATA%\\Ferro\\ferro-desktop.exe\" sync \"%V\"",
                "/f",
            ])
            .status()
            .map_err(|e| format!("Failed to set command: {}", e))?;

        if !icon_status.success() {
            return Err("Failed to register context menu command".to_string());
        }

        // Add icon path
        let icon_path = std::env::var("LOCALAPPDATA")
            .map(|p| format!("{}\\Ferro\\icon.ico", p))
            .unwrap_or_default();

        if !icon_path.is_empty() {
            let _ = Command::new("reg")
                .args([
                    "add",
                    REGISTRY_KEY,
                    "/v",
                    "Icon",
                    "/t",
                    "REG_SZ",
                    "/d",
                    &icon_path,
                    "/f",
                ])
                .status();
        }

        info!("Windows context menu registered");
        Ok(())
    }

    /// Unregister the Windows Explorer context menu entry.
    pub fn unregister_context_menu() -> Result<(), String> {
        let status = Command::new("reg")
            .args(["delete", REGISTRY_KEY, "/f"])
            .status()
            .map_err(|e| format!("Failed to run reg delete: {}", e))?;

        if !status.success() {
            return Err("Failed to unregister context menu".to_string());
        }

        info!("Windows context menu unregistered");
        Ok(())
    }

    /// Check if the context menu is currently registered.
    pub fn is_registered() -> bool {
        Command::new("reg")
            .args(["query", REGISTRY_KEY])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// Open Windows Explorer at the specified path.
    pub fn open_in_explorer(path: &str) -> Result<(), String> {
        Command::new("explorer")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Failed to open Explorer: {}", e))?;
        Ok(())
    }

    /// Add Ferro to Windows startup (per-user) so the tray icon appears on login.
    pub fn register_autostart(exe_path: &str) -> Result<(), String> {
        let key = r"Software\Microsoft\Windows\CurrentVersion\Run";
        let status = Command::new("reg")
            .args([
                "add", key, "/v", "Ferro", "/t", "REG_SZ", "/d", exe_path, "/f",
            ])
            .status()
            .map_err(|e| format!("Failed to add autostart: {}", e))?;

        if !status.success() {
            return Err("Failed to register autostart".to_string());
        }

        info!("Windows autostart registered for {}", exe_path);
        Ok(())
    }

    /// Remove Ferro from Windows startup.
    pub fn unregister_autostart() -> Result<(), String> {
        let key = r"Software\Microsoft\Windows\CurrentVersion\Run";
        let status = Command::new("reg")
            .args(["delete", key, "/v", "Ferro", "/f"])
            .status()
            .map_err(|e| format!("Failed to remove autostart: {}", e))?;

        if !status.success() {
            return Err("Failed to unregister autostart".to_string());
        }

        info!("Windows autostart unregistered");
        Ok(())
    }
}

#[cfg(not(target_os = "windows"))]
mod windows {
    pub fn register_context_menu() -> Result<(), String> {
        Err("context menu registration is only supported on Windows".into())
    }
    pub fn unregister_context_menu() -> Result<(), String> {
        Err("context menu unregistration is only supported on Windows".into())
    }
    pub fn is_registered() -> bool {
        false
    }
    pub fn open_in_explorer(_path: &str) -> Result<(), String> {
        Err("Windows Explorer integration is only available on Windows".into())
    }
    pub fn register_autostart(_exe_path: &str) -> Result<(), String> {
        Err("autostart registration is only supported on Windows".into())
    }
    pub fn unregister_autostart() -> Result<(), String> {
        Err("autostart unregistration is only supported on Windows".into())
    }
}

pub use windows::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_non_windows_functions_return_error() {
        #[cfg(not(target_os = "windows"))]
        {
            assert!(register_context_menu().is_err());
            assert!(unregister_context_menu().is_err());
            assert!(!is_registered());
            assert!(open_in_explorer("/tmp").is_err());
            assert!(register_autostart("/tmp/ferro").is_err());
            assert!(unregister_autostart().is_err());
        }
    }
}
