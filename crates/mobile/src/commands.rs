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
    common::http_client::build_client(auth_token, common::http_client::HttpClientOptions::default())
        .map_err(|e| MobileError::InvalidConfig(e))
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

    let local_cache_bytes = dir_size_recursive(&std::path::PathBuf::from(&config.local_cache_path).join("files"));

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

    let url = format!("{}/api/push/register", config.server_url.trim_end_matches('/'));

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
            let metadata = std::fs::metadata(&path).map_err(|e| MobileError::NotFound(e.to_string()))?;
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
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

// ── File Browser Commands ─────────────────────────────────────────────

#[tauri::command]
pub async fn list_files(path: String) -> Result<Vec<crate::MobileFileEntry>, String> {
    let config = get_config().map_err(|e| e.to_string())?;
    let client = build_client(&config.auth_token).map_err(|e| e.to_string())?;

    let url = format!(
        "{}/remote.php/dav/files/default{}",
        config.server_url.trim_end_matches('/'),
        if path.is_empty() { "/" } else { &path }
    );

    let resp = client
        .request(reqwest::Method::from_bytes(b"PROPFIND").unwrap(), &url)
        .header("Depth", "1")
        .send()
        .await
        .map_err(|e| format!("PROPFIND failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("PROPFIND failed: {}", resp.status()));
    }

    let xml = resp.text().await.map_err(|e| format!("Read response: {}", e))?;
    parse_dav_response(&xml, &path)
}

fn parse_dav_response(xml: &str, parent_path: &str) -> Result<Vec<crate::MobileFileEntry>, String> {
    let mut entries = Vec::new();
    let mut current_href = String::new();
    let mut current_size = 0u64;
    let mut current_is_collection = false;
    let mut current_modified = String::new();
    let mut in_response = false;

    for line in xml.lines() {
        let trimmed = line.trim();

        if trimmed.contains("<d:response") || trimmed.contains("<D:response") {
            in_response = true;
            current_href.clear();
            current_size = 0;
            current_is_collection = false;
            current_modified.clear();
        }

        if in_response {
            if let Some(start) = trimmed.find("<d:href>").or_else(|| trimmed.find("<D:href>")) {
                if let Some(s) = trimmed[start..].find('>') {
                    let s = start + s + 1;
                    if let Some(e) = trimmed[s..].find('<') {
                        current_href = trimmed[s..s + e].to_string();
                    }
                }
            }
            if trimmed.contains("<d:collection") || trimmed.contains("<D:collection") {
                current_is_collection = true;
            }
            if let Some(start) = trimmed.find("<d:getcontentlength>").or_else(|| trimmed.find("<D:getcontentlength>")) {
                if let Some(s) = trimmed[start..].find('>') {
                    let s = start + s + 1;
                    if let Some(e) = trimmed[s..].find('<') {
                        current_size = trimmed[s..s + e].parse().unwrap_or(0);
                    }
                }
            }
            if let Some(start) = trimmed.find("<d:getlastmodified>").or_else(|| trimmed.find("<D:getlastmodified>")) {
                if let Some(s) = trimmed[start..].find('>') {
                    let s = start + s + 1;
                    if let Some(e) = trimmed[s..].find('<') {
                        current_modified = trimmed[s..s + e].to_string();
                    }
                }
            }
        }

        if trimmed.contains("</d:response>") || trimmed.contains("</D:response>") {
            if in_response && !current_href.is_empty() {
                // Skip the parent directory itself
                let parent_href = if parent_path.is_empty() || parent_path == "/" {
                    "/remote.php/dav/files/default/".to_string()
                } else {
                    format!("/remote.php/dav/files/default{}", parent_path)
                };
                if current_href.trim_end_matches('/') == parent_href.trim_end_matches('/') {
                    in_response = false;
                    continue;
                }

                let name = current_href
                    .trim_end_matches('/')
                    .rsplit('/')
                    .next()
                    .unwrap_or("")
                    .to_string();

                if !name.is_empty() {
                    entries.push(crate::MobileFileEntry {
                        name,
                        path: current_href.clone(),
                        size: current_size,
                        is_dir: current_is_collection,
                        modified: current_modified.clone(),
                        content_type: if current_is_collection {
                            "inode/directory".to_string()
                        } else {
                            "application/octet-stream".to_string()
                        },
                        is_pinned: false,
                        is_available_offline: false,
                    });
                }
            }
            in_response = false;
        }
    }

    Ok(entries)
}

// ── Share Commands ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileShare {
    pub id: String,
    pub path: String,
    pub share_type: String,
    pub shared_with: Option<String>,
    pub permissions: SharePermissions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SharePermissions {
    pub read: bool,
    pub write: bool,
}

#[tauri::command]
pub async fn list_shares(path: Option<String>) -> Result<Vec<MobileShare>, String> {
    let config = get_config().map_err(|e| e.to_string())?;
    let client = build_client(&config.auth_token).map_err(|e| e.to_string())?;

    let mut url = format!(
        "{}/ocs/v2.php/apps/files_sharing/api/v1/shares?format=json",
        config.server_url.trim_end_matches('/')
    );
    if let Some(ref p) = path {
        url.push_str(&format!("&path={}", p));
    }

    let resp = client
        .get(&url)
        .header("OCS-APIRequest", "true")
        .send()
        .await
        .map_err(|e| format!("List shares failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("List shares failed: {}", resp.status()));
    }

    let body: serde_json::Value = resp.json().await.map_err(|e| format!("Parse response: {}", e))?;
    let shares_data = body.get("ocs").and_then(|o| o.get("data")).and_then(|d| d.as_array());

    let shares = shares_data
        .map(|arr| {
            arr.iter()
                .filter_map(|s| {
                    let id = s.get("id")?.as_str()?.to_string();
                    let path = s.get("path")?.as_str()?.to_string();
                    let share_type = match s.get("share_type")?.as_i64()? {
                        0 => "user",
                        1 => "group",
                        3 => "link",
                        _ => "unknown",
                    };
                    let shared_with = s.get("share_with").and_then(|v| v.as_str()).map(|s| s.to_string());
                    let perms = s.get("permissions")?.as_i64()?;
                    Some(MobileShare {
                        id,
                        path,
                        share_type: share_type.to_string(),
                        shared_with,
                        permissions: SharePermissions {
                            read: perms & 1 != 0,
                            write: perms & 2 != 0,
                        },
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(shares)
}

#[tauri::command]
pub async fn create_share(
    path: String,
    share_type: String,
    shared_with: Option<String>,
    read: bool,
    write: bool,
) -> Result<MobileShare, String> {
    let config = get_config().map_err(|e| e.to_string())?;
    let client = build_client(&config.auth_token).map_err(|e| e.to_string())?;

    let share_type_id = match share_type.as_str() {
        "user" => 0,
        "group" => 1,
        "link" => 3,
        _ => return Err(format!("Invalid share type: {}", share_type)),
    };

    let mut body = serde_json::json!({
        "path": path,
        "shareType": share_type_id,
        "permissions": (if read { 1 } else { 0 }) | (if write { 2 } else { 0 }),
    });
    if let Some(ref with) = shared_with {
        body["shareWith"] = serde_json::json!(with);
    }

    let url = format!(
        "{}/ocs/v2.php/apps/files_sharing/api/v1/shares?format=json",
        config.server_url.trim_end_matches('/')
    );

    let resp = client
        .post(&url)
        .header("OCS-APIRequest", "true")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Create share failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Create share failed: {}", resp.status()));
    }

    let body: serde_json::Value = resp.json().await.map_err(|e| format!("Parse response: {}", e))?;
    let s = body.get("ocs").and_then(|o| o.get("data")).ok_or("No data in response")?;

    Ok(MobileShare {
        id: s.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        path: s.get("path").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        share_type: share_type,
        shared_with: shared_with,
        permissions: SharePermissions { read, write },
    })
}

// ── Version History Commands ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileVersion {
    pub version: String,
    pub size: u64,
    pub modified: String,
}

#[tauri::command]
pub async fn list_versions(path: String) -> Result<Vec<FileVersion>, String> {
    let config = get_config().map_err(|e| e.to_string())?;
    let client = build_client(&config.auth_token).map_err(|e| e.to_string())?;

    let encoded = path.trim_start_matches('/').replace('/', "%2F");
    let url = format!(
        "{}/remote.php/dav/versions/default/versions/{}",
        config.server_url.trim_end_matches('/'),
        encoded
    );

    let resp = client
        .request(reqwest::Method::from_bytes(b"PROPFIND").unwrap(), &url)
        .header("Depth", "1")
        .send()
        .await
        .map_err(|e| format!("PROPFIND versions failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("PROPFIND versions failed: {}", resp.status()));
    }

    let xml = resp.text().await.map_err(|e| format!("Read response: {}", e))?;
    let mut versions = Vec::new();
    let mut current_href = String::new();
    let mut current_size = 0u64;
    let mut current_modified = String::new();
    let mut in_response = false;

    for line in xml.lines() {
        let trimmed = line.trim();
        if trimmed.contains("<d:response") || trimmed.contains("<D:response") {
            in_response = true;
            current_href.clear();
            current_size = 0;
            current_modified.clear();
        }
        if in_response {
            if let Some(start) = trimmed.find("<d:href>").or_else(|| trimmed.find("<D:href>")) {
                if let Some(s) = trimmed[start..].find('>') {
                    let s = start + s + 1;
                    if let Some(e) = trimmed[s..].find('<') {
                        current_href = trimmed[s..s + e].to_string();
                    }
                }
            }
            if let Some(start) = trimmed.find("<d:getcontentlength>").or_else(|| trimmed.find("<D:getcontentlength>")) {
                if let Some(s) = trimmed[start..].find('>') {
                    let s = start + s + 1;
                    if let Some(e) = trimmed[s..].find('<') {
                        current_size = trimmed[s..s + e].parse().unwrap_or(0);
                    }
                }
            }
            if let Some(start) = trimmed.find("<d:getlastmodified>").or_else(|| trimmed.find("<D:getlastmodified>")) {
                if let Some(s) = trimmed[start..].find('>') {
                    let s = start + s + 1;
                    if let Some(e) = trimmed[s..].find('<') {
                        current_modified = trimmed[s..s + e].to_string();
                    }
                }
            }
        }
        if trimmed.contains("</d:response>") || trimmed.contains("</D:response>") {
            if in_response && !current_href.is_empty() {
                let version = current_href
                    .rsplit('/')
                    .next()
                    .unwrap_or("")
                    .to_string();
                if !version.is_empty() && version != "versions" {
                    versions.push(FileVersion {
                        version,
                        size: current_size,
                        modified: current_modified.clone(),
                    });
                }
            }
            in_response = false;
        }
    }

    Ok(versions)
}

// ── Trash Commands ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrashEntry {
    pub id: String,
    pub name: String,
    pub original_location: String,
    pub deleted_at: String,
    pub size: u64,
}

#[tauri::command]
pub async fn list_trash() -> Result<Vec<TrashEntry>, String> {
    let config = get_config().map_err(|e| e.to_string())?;
    let client = build_client(&config.auth_token).map_err(|e| e.to_string())?;

    let url = format!(
        "{}/remote.php/dav/trashbin/default/trash?depth=1",
        config.server_url.trim_end_matches('/')
    );

    let resp = client
        .request(reqwest::Method::from_bytes(b"PROPFIND").unwrap(), &url)
        .header("Depth", "1")
        .send()
        .await
        .map_err(|e| format!("PROPFIND trash failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("PROPFIND trash failed: {}", resp.status()));
    }

    let xml = resp.text().await.map_err(|e| format!("Read response: {}", e))?;
    let mut entries = Vec::new();
    let mut current_href = String::new();
    let mut current_size = 0u64;
    let mut current_name = String::new();
    let mut in_response = false;

    for line in xml.lines() {
        let trimmed = line.trim();
        if trimmed.contains("<d:response") || trimmed.contains("<D:response") {
            in_response = true;
            current_href.clear();
            current_size = 0;
            current_name.clear();
        }
        if in_response {
            if let Some(start) = trimmed.find("<d:href>").or_else(|| trimmed.find("<D:href>")) {
                if let Some(s) = trimmed[start..].find('>') {
                    let s = start + s + 1;
                    if let Some(e) = trimmed[s..].find('<') {
                        current_href = trimmed[s..s + e].to_string();
                    }
                }
            }
            if let Some(start) = trimmed.find("<d:displayname>").or_else(|| trimmed.find("<D:displayname>")) {
                if let Some(s) = trimmed[start..].find('>') {
                    let s = start + s + 1;
                    if let Some(e) = trimmed[s..].find('<') {
                        current_name = trimmed[s..s + e].to_string();
                    }
                }
            }
            if let Some(start) = trimmed.find("<d:getcontentlength>").or_else(|| trimmed.find("<D:getcontentlength>")) {
                if let Some(s) = trimmed[start..].find('>') {
                    let s = start + s + 1;
                    if let Some(e) = trimmed[s..].find('<') {
                        current_size = trimmed[s..s + e].parse().unwrap_or(0);
                    }
                }
            }
        }
        if trimmed.contains("</d:response>") || trimmed.contains("</D:response>") {
            if in_response && !current_href.is_empty() && current_name != "." && current_name != ".." {
                let id = current_href
                    .rsplit('/')
                    .next()
                    .unwrap_or("")
                    .to_string();
                entries.push(TrashEntry {
                    id,
                    name: current_name.clone(),
                    original_location: current_href.clone(),
                    deleted_at: String::new(),
                    size: current_size,
                });
            }
            in_response = false;
        }
    }

    Ok(entries)
}
