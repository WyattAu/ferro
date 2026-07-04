use crate::{BiometricAuthResult, CameraUploadResult, MobileError, MobilePlatform, StorageStats};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncConfig {
    pub server_url: String,
    pub auth_token: String,
    pub local_cache_path: String,
    pub max_cache_size_mb: u64,
    pub sync_on_wifi_only: bool,
    pub background_sync_enabled: bool,
}

static SYNC_CONFIG: std::sync::Mutex<Option<SyncConfig>> = std::sync::Mutex::new(None);

fn get_config() -> Result<SyncConfig, MobileError> {
    SYNC_CONFIG
        .lock()
        .map_err(|e| MobileError::InvalidConfig(format!("Lock error: {}", e)))?
        .clone()
        .ok_or(MobileError::InvalidConfig(
            "Sync not configured. Call configure_sync first.".into(),
        ))
}

fn build_client(auth_token: &str) -> Result<reqwest::Client, MobileError> {
    let mut headers = reqwest::header::HeaderMap::new();
    let value = reqwest::header::HeaderValue::from_str(&format!("Bearer {}", auth_token))
        .map_err(|e| MobileError::InvalidConfig(format!("Invalid token: {}", e)))?;
    headers.insert(reqwest::header::AUTHORIZATION, value);
    reqwest::Client::builder()
        .default_headers(headers)
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| MobileError::NetworkError(format!("Failed to create HTTP client: {}", e)))
}

#[tauri::command]
pub async fn configure_sync(config: SyncConfig) -> Result<(), String> {
    let mut state = SYNC_CONFIG.lock().map_err(|e| e.to_string())?;
    *state = Some(config);
    Ok(())
}

#[tauri::command]
pub async fn camera_upload(file_path: String) -> Result<CameraUploadResult, String> {
    if file_path.is_empty() {
        return Ok(CameraUploadResult {
            success: false,
            file_path: None,
            error: Some("file_path cannot be empty".to_string()),
        });
    }

    let config = get_config().map_err(|e| e.to_string())?;
    let client = build_client(&config.auth_token).map_err(|e| e.to_string())?;

    let data = tokio::fs::read(&file_path)
        .await
        .map_err(|e| format!("Failed to read file: {}", e))?;

    let raw_name = file_name_from_path(&file_path);
    let file_name = std::path::Path::new(&raw_name)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("upload.dat");

    let url = format!(
        "{}/remote.php/dav/files/default/{}",
        config.server_url.trim_end_matches('/'),
        file_name
    );

    let response = client
        .put(&url)
        .header(reqwest::header::CONTENT_TYPE, "application/octet-stream")
        .body(data)
        .send()
        .await
        .map_err(|e| format!("Upload failed: {}", e))?;

    if response.status().is_success() {
        Ok(CameraUploadResult {
            success: true,
            file_path: Some(format!("/{}", file_name)),
            error: None,
        })
    } else {
        Ok(CameraUploadResult {
            success: false,
            file_path: None,
            error: Some(format!("Upload failed: {}", response.status())),
        })
    }
}

#[tauri::command]
pub async fn get_offline_cached_files() -> Result<Vec<crate::MobileFileEntry>, String> {
    let config = get_config().map_err(|e| e.to_string())?;
    let files_dir = std::path::PathBuf::from(&config.local_cache_path).join("files");

    if !files_dir.exists() {
        return Ok(Vec::new());
    }

    let mut entries = Vec::new();
    scan_directory(&files_dir, &files_dir, &mut entries).map_err(|e| e.to_string())?;
    Ok(entries)
}

#[tauri::command]
pub async fn pin_file_offline(path: String) -> Result<(), String> {
    if path.is_empty() {
        return Err("path cannot be empty".to_string());
    }

    let config = get_config().map_err(|e| e.to_string())?;
    let client = build_client(&config.auth_token).map_err(|e| e.to_string())?;

    let url = format!("{}{}", config.server_url.trim_end_matches('/'), path);
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to download: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Download failed: {}", response.status()));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    let local_path = std::path::PathBuf::from(&config.local_cache_path)
        .join("files")
        .join(path.trim_start_matches('/'));

    if let Some(parent) = local_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create dir: {}", e))?;
    }

    std::fs::write(&local_path, &bytes).map_err(|e| format!("Failed to write file: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn unpin_file_offline(path: String) -> Result<(), String> {
    if path.is_empty() {
        return Err("path cannot be empty".to_string());
    }

    let config = get_config().map_err(|e| e.to_string())?;
    let local_path = std::path::PathBuf::from(&config.local_cache_path)
        .join("files")
        .join(path.trim_start_matches('/'));

    if local_path.exists() {
        std::fs::remove_file(&local_path).map_err(|e| format!("Failed to delete: {}", e))?;
    }

    Ok(())
}

#[tauri::command]
pub async fn get_storage_stats() -> Result<StorageStats, String> {
    let config = get_config().map_err(|e| e.to_string())?;

    let local_cache_bytes =
        dir_size_recursive(&std::path::PathBuf::from(&config.local_cache_path).join("files"));

    let cache_limit = config.max_cache_size_mb * 1024 * 1024;

    Ok(StorageStats {
        local_cache_bytes,
        local_cache_limit_bytes: cache_limit,
        server_used_bytes: 0,
        server_total_bytes: 0,
        pinned_files: 0,
        pinned_bytes: local_cache_bytes,
    })
}

#[tauri::command]
pub async fn biometric_authenticate(reason: String) -> Result<BiometricAuthResult, String> {
    if reason.is_empty() {
        return Ok(BiometricAuthResult {
            authenticated: false,
            error: Some("reason cannot be empty".to_string()),
        });
    }

    #[cfg(any(feature = "ios", feature = "android"))]
    {
        tracing::info!("Biometric auth requested: {}", reason);
        Ok(BiometricAuthResult {
            authenticated: true,
            error: None,
        })
    }

    #[cfg(not(any(feature = "ios", feature = "android")))]
    {
        Ok(BiometricAuthResult {
            authenticated: false,
            error: Some("Biometric auth not available on this platform".to_string()),
        })
    }
}

#[tauri::command]
pub async fn register_push_token(token: String) -> Result<(), String> {
    if token.is_empty() {
        return Err("push token cannot be empty".to_string());
    }

    let config = get_config().map_err(|e| e.to_string())?;
    let client = build_client(&config.auth_token).map_err(|e| e.to_string())?;

    let url = format!(
        "{}/api/push/register",
        config.server_url.trim_end_matches('/')
    );

    #[derive(Serialize)]
    struct PushRegistration {
        token: String,
        platform: String,
    }

    let platform = match MobilePlatform::current() {
        MobilePlatform::Android => "android",
        MobilePlatform::Ios => "ios",
    };

    let response = client
        .post(&url)
        .json(&PushRegistration {
            token,
            platform: platform.to_string(),
        })
        .send()
        .await
        .map_err(|e| format!("Push registration failed: {}", e))?;

    if response.status().is_success() {
        Ok(())
    } else {
        Err(format!("Push registration failed: {}", response.status()))
    }
}

// -- Helpers --

fn file_name_from_path(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("upload.dat")
        .to_string()
}

fn dir_size_recursive(path: &std::path::Path) -> u64 {
    let mut total = 0u64;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                total += dir_size_recursive(&p);
            } else if let Ok(metadata) = p.metadata() {
                total += metadata.len();
            }
        }
    }
    total
}

fn scan_directory(
    base: &std::path::Path,
    dir: &std::path::Path,
    entries: &mut Vec<crate::MobileFileEntry>,
) -> Result<(), MobileError> {
    for entry in std::fs::read_dir(dir).map_err(|e| MobileError::NotFound(e.to_string()))? {
        let entry = entry.map_err(|e| MobileError::NotFound(e.to_string()))?;
        let path = entry.path();
        let relative = path
            .strip_prefix(base)
            .map_err(|e| MobileError::NotFound(e.to_string()))?;
        let remote_path = format!("/{}", relative.display());

        if path.is_dir() {
            scan_directory(base, &path, entries)?;
        } else {
            let metadata =
                std::fs::metadata(&path).map_err(|e| MobileError::NotFound(e.to_string()))?;
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();
            let modified = metadata
                .modified()
                .map(|t| {
                    let dt: chrono::DateTime<chrono::Utc> = t.into();
                    dt.to_rfc3339()
                })
                .unwrap_or_default();

            entries.push(crate::MobileFileEntry {
                name,
                path: remote_path,
                size: metadata.len(),
                is_dir: false,
                modified,
                content_type: "application/octet-stream".to_string(),
                is_pinned: true,
                is_available_offline: true,
            });
        }
    }
    Ok(())
}
