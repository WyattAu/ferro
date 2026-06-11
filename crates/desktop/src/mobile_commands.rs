//! Mobile-specific Tauri commands for iOS Files Provider and Android SAF.
//!
//! These commands are invoked from the mobile frontend or native platform code.
//! Only compiled when the `mobile` feature is enabled.

use crate::mobile::{MobileConflictStrategy, MobilePlatform, MobileSyncConfig};
use serde::{Deserialize, Serialize};
use tauri::Emitter;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageStats {
    pub local_cache_bytes: u64,
    pub local_cache_limit_bytes: u64,
    pub server_used_bytes: u64,
    pub server_total_bytes: u64,
    pub pinned_files: u32,
    pub pinned_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileFileEntry {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub is_dir: bool,
    pub modified: String,
    pub content_type: String,
    pub is_pinned: bool,
    pub is_available_offline: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SyncStatus {
    Idle,
    Syncing,
    Error(String),
    Conflict,
    Paused,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectivityState {
    pub connected: bool,
    pub wifi: bool,
}

#[tauri::command]
pub async fn mobile_get_file_thumbnail(path: String, size: u32) -> Result<Vec<u8>, String> {
    let _ = (path, size);
    Ok(Vec::new())
}

#[tauri::command]
pub async fn mobile_get_storage_stats() -> Result<StorageStats, String> {
    Ok(StorageStats {
        local_cache_bytes: 0,
        local_cache_limit_bytes: 256 * 1024 * 1024,
        server_used_bytes: 0,
        server_total_bytes: 0,
        pinned_files: 0,
        pinned_bytes: 0,
    })
}

#[tauri::command]
pub async fn mobile_start_background_sync(
    platform: String,
    server_url: String,
    auth_token: String,
    local_cache_path: String,
    max_cache_size_mb: u64,
    sync_on_wifi_only: bool,
    sync_on_charging: bool,
    background_sync_enabled: bool,
) -> Result<(), String> {
    let mobile_platform = match platform.as_str() {
        "android" => MobilePlatform::Android,
        "ios" => MobilePlatform::Ios,
        other => return Err(format!("Unknown platform: {other}")),
    };

    let _config = MobileSyncConfig {
        platform: mobile_platform,
        server_url,
        auth_token,
        local_cache_path,
        max_cache_size_mb,
        sync_on_wifi_only,
        sync_on_charging,
        background_sync_enabled,
        conflict_strategy: MobileConflictStrategy::Skip,
    };

    Ok(())
}

#[tauri::command]
pub async fn mobile_stop_background_sync() -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub async fn mobile_get_offline_files() -> Result<Vec<MobileFileEntry>, String> {
    Ok(Vec::new())
}

#[tauri::command]
pub async fn mobile_pin_file_offline(path: String) -> Result<(), String> {
    if path.is_empty() {
        return Err("path cannot be empty".to_string());
    }
    Ok(())
}

#[tauri::command]
pub async fn mobile_unpin_file_offline(path: String) -> Result<(), String> {
    if path.is_empty() {
        return Err("path cannot be empty".to_string());
    }
    Ok(())
}

#[tauri::command]
pub async fn mobile_get_sync_status() -> Result<SyncStatus, String> {
    Ok(SyncStatus::Idle)
}

#[tauri::command]
pub async fn mobile_resolve_conflict(path: String, resolution: String) -> Result<(), String> {
    if path.is_empty() {
        return Err("path cannot be empty".to_string());
    }
    match resolution.as_str() {
        "keep_local" | "keep_remote" | "keep_both" => Ok(()),
        other => Err(format!(
            "Invalid resolution: {other}. Must be keep_local, keep_remote, or keep_both"
        )),
    }
}

#[tauri::command]
pub async fn mobile_share_file(path: String, share_type: String) -> Result<String, String> {
    if path.is_empty() {
        return Err("path cannot be empty".to_string());
    }
    match share_type.as_str() {
        "link" => Ok(format!(
            "https://ferro.app/share/{}",
            path.trim_start_matches('/')
        )),
        "native" => Ok("native_share_invoked".to_string()),
        other => Err(format!("Unknown share type: {other}")),
    }
}

#[tauri::command]
pub async fn mobile_monitor_connectivity(app: tauri::AppHandle) -> Result<(), String> {
    let state = ConnectivityState {
        connected: true,
        wifi: true,
    };
    app.emit("connectivity-change", &state)
        .map_err(|e| format!("Failed to emit connectivity event: {}", e))?;
    Ok(())
}

#[tauri::command]
pub async fn mobile_register_push_notifications(token: String) -> Result<(), String> {
    if token.is_empty() {
        return Err("push token cannot be empty".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_stats_serialization() {
        let stats = StorageStats {
            local_cache_bytes: 1024,
            local_cache_limit_bytes: 256 * 1024 * 1024,
            server_used_bytes: 512,
            server_total_bytes: 1024,
            pinned_files: 5,
            pinned_bytes: 2048,
        };
        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("localCacheBytes"));
        assert!(json.contains("pinnedFiles"));
    }

    #[test]
    fn test_sync_status_serialization() {
        let status = SyncStatus::Idle;
        assert_eq!(serde_json::to_string(&status).unwrap(), "\"idle\"");

        let status = SyncStatus::Error("network timeout".to_string());
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("network timeout"));
    }

    #[test]
    fn test_mobile_file_entry_serialization() {
        let entry = MobileFileEntry {
            name: "doc.pdf".to_string(),
            path: "/docs/doc.pdf".to_string(),
            size: 4096,
            is_dir: false,
            modified: "2024-01-01T00:00:00Z".to_string(),
            content_type: "application/pdf".to_string(),
            is_pinned: true,
            is_available_offline: true,
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("isPinned"));
        assert!(json.contains("isAvailableOffline"));
    }

    #[tokio::test]
    async fn test_mobile_pin_file_offline_empty_path() {
        let result = mobile_pin_file_offline(String::new()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mobile_pin_file_offline_valid() {
        let result = mobile_pin_file_offline("/docs/file.txt".to_string()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mobile_unpin_file_offline_empty_path() {
        let result = mobile_unpin_file_offline(String::new()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mobile_resolve_conflict_valid() {
        for resolution in &["keep_local", "keep_remote", "keep_both"] {
            let result =
                mobile_resolve_conflict("/file.txt".to_string(), resolution.to_string()).await;
            assert!(result.is_ok());
        }
    }

    #[tokio::test]
    async fn test_mobile_resolve_conflict_invalid() {
        let result =
            mobile_resolve_conflict("/file.txt".to_string(), "overwrite".to_string()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mobile_resolve_conflict_empty_path() {
        let result = mobile_resolve_conflict(String::new(), "keep_local".to_string()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mobile_share_file_link() {
        let result = mobile_share_file("/docs/file.txt".to_string(), "link".to_string()).await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("ferro.app/share/"));
    }

    #[tokio::test]
    async fn test_mobile_share_file_native() {
        let result = mobile_share_file("/docs/file.txt".to_string(), "native".to_string()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mobile_share_file_invalid_type() {
        let result = mobile_share_file("/docs/file.txt".to_string(), "invalid".to_string()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mobile_share_file_empty_path() {
        let result = mobile_share_file(String::new(), "link".to_string()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mobile_register_push_notifications_empty_token() {
        let result = mobile_register_push_notifications(String::new()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mobile_register_push_notifications_valid() {
        let result = mobile_register_push_notifications("device-token-abc123".to_string()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mobile_start_background_sync_invalid_platform() {
        let result = mobile_start_background_sync(
            "windows".to_string(),
            "https://example.com".to_string(),
            "token".to_string(),
            "/cache".to_string(),
            256,
            true,
            true,
            false,
        )
        .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown platform"));
    }

    #[tokio::test]
    async fn test_mobile_start_background_sync_android() {
        let result = mobile_start_background_sync(
            "android".to_string(),
            "https://example.com".to_string(),
            "token".to_string(),
            "/data/data/com.ferro.app/cache".to_string(),
            512,
            true,
            true,
            false,
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mobile_start_background_sync_ios() {
        let result = mobile_start_background_sync(
            "ios".to_string(),
            "https://example.com".to_string(),
            "token".to_string(),
            "/var/mobile/Library/Caches/com.ferro.app".to_string(),
            256,
            true,
            true,
            true,
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mobile_get_storage_stats() {
        let result = mobile_get_storage_stats().await;
        assert!(result.is_ok());
        let stats = result.unwrap();
        assert_eq!(stats.local_cache_bytes, 0);
        assert_eq!(stats.pinned_files, 0);
    }

    #[tokio::test]
    async fn test_mobile_get_offline_files() {
        let result = mobile_get_offline_files().await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_mobile_get_sync_status() {
        let result = mobile_get_sync_status().await;
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), SyncStatus::Idle));
    }

    #[tokio::test]
    async fn test_mobile_stop_background_sync() {
        let result = mobile_stop_background_sync().await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_connectivity_state_serialization() {
        let state = ConnectivityState {
            connected: true,
            wifi: false,
        };
        let json = serde_json::to_string(&state).unwrap();
        assert!(json.contains("\"connected\":true"));
        assert!(json.contains("\"wifi\":false"));
    }
}
