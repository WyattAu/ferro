//! OS-appropriate storage for sync credentials.
//!
//! Tokens and passwords are kept out of `desktop.json` in a file with
//! owner-only permissions (0600 on Unix). This avoids leaking credentials
//! in world-readable config backups while working identically on headless
//! servers, where an OS keyring daemon is usually unavailable.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Secrets {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
}

impl Secrets {
    pub fn is_empty(&self) -> bool {
        self.auth_token.as_deref().is_none_or(str::is_empty) && self.password.as_deref().is_none_or(str::is_empty)
    }
}

pub fn secrets_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("ferro")
        .join("secrets.json")
}

const KEYCHAIN_SERVICE: &str = "ferro-desktop";
const KEYCHAIN_USER: &str = "ferro";

fn load_keychain() -> Option<Secrets> {
    let entry = keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_USER).ok()?;
    let raw = entry.get_password().ok()?;
    serde_json::from_str(&raw).ok()
}

fn save_keychain(secrets: &Secrets) -> Result<(), String> {
    let entry = keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_USER).map_err(|e| e.to_string())?;
    let raw = serde_json::to_string(secrets).map_err(|e| e.to_string())?;
    entry.set_password(&raw).map_err(|e| e.to_string())
}

pub fn load_secrets() -> Secrets {
    // OS keychain first (macOS Keychain, Windows Credential Manager,
    // Linux Secret Service); owner-only file as fallback for headless
    // hosts without a keyring daemon.
    if let Some(secrets) = load_keychain()
        && !secrets.is_empty()
    {
        return secrets;
    }
    load_file()
}

fn load_file() -> Secrets {
    let data = std::fs::read_to_string(secrets_path()).unwrap_or_default();
    serde_json::from_str(&data).unwrap_or_default()
}

pub fn save_secrets(secrets: &Secrets) -> Result<(), String> {
    // Best effort: keychain primary, file fallback (kept in sync so
    // headless hosts without a keyring daemon keep working).
    let _ = save_keychain(secrets);
    save_file(secrets)
}

fn save_file(secrets: &Secrets) -> Result<(), String> {
    let path = secrets_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let data = serde_json::to_string_pretty(secrets).map_err(|e| e.to_string())?;
    std::fs::write(&path, data).map_err(|e| e.to_string())?;
    restrict_permissions(&path)?;
    Ok(())
}

#[cfg(unix)]
fn restrict_permissions(path: &std::path::Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).map_err(|e| e.to_string())
}

#[cfg(not(unix))]
fn restrict_permissions(_path: &std::path::Path) -> Result<(), String> {
    // Windows ACLs default new files to the creating user + admins;
    // explicit hardening is a future improvement.
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_by_default() {
        assert!(Secrets::default().is_empty());
        assert!(
            !Secrets {
                auth_token: Some("t".into()),
                password: None,
                refresh_token: None
            }
            .is_empty()
        );
    }
}
