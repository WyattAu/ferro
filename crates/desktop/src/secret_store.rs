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
}

impl Secrets {
    pub fn is_empty(&self) -> bool {
        self.auth_token.as_deref().map_or(true, str::is_empty)
            && self.password.as_deref().map_or(true, str::is_empty)
    }
}

pub fn secrets_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("ferro")
        .join("secrets.json")
}

pub fn load_secrets() -> Secrets {
    let data = std::fs::read_to_string(secrets_path()).unwrap_or_default();
    serde_json::from_str(&data).unwrap_or_default()
}

pub fn save_secrets(secrets: &Secrets) -> Result<(), String> {
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
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
        .map_err(|e| e.to_string())
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
        assert!(!Secrets { auth_token: Some("t".into()), password: None }.is_empty());
    }
}
