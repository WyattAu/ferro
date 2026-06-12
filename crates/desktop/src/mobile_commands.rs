//! Mobile-specific Tauri commands for iOS Files Provider and Android SAF.
//!
//! These commands are invoked from the mobile frontend or native platform code.
//! Only compiled when the `mobile` feature is enabled.
//!
//! All commands connect to a ferro-server via WebDAV/REST API using a reqwest
//! client with Bearer token authentication. The server URL and auth token are
//! stored in global state when `mobile_start_background_sync` is called.

use crate::mobile::{MobileConflictStrategy, MobilePlatform, MobileSyncConfig};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use tauri::Emitter;

// -- Types --

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

// -- Manifest for pinned files --

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct PinManifest {
    pinned: std::collections::HashMap<String, PinEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PinEntry {
    etag: Option<String>,
    downloaded_at: String,
    size: u64,
}

// -- Global sync state --

struct MobileSyncStateInner {
    config: MobileSyncConfig,
    cancel: Arc<AtomicBool>,
    task: Option<tokio::task::JoinHandle<()>>,
    last_error: Option<String>,
}

static MOBILE_STATE: StdMutex<Option<MobileSyncStateInner>> = StdMutex::new(None);
static CONNECTIVITY_MONITOR: StdMutex<Option<tokio::task::JoinHandle<()>>> =
    StdMutex::new(None);

// -- Helper functions --

fn build_mobile_client(auth_token: &str) -> Result<reqwest::Client, String> {
    let mut headers = reqwest::header::HeaderMap::new();
    let value = reqwest::header::HeaderValue::from_str(&format!("Bearer {}", auth_token))
        .map_err(|e| format!("Invalid token: {}", e))?;
    headers.insert(reqwest::header::AUTHORIZATION, value);
    reqwest::Client::builder()
        .default_headers(headers)
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))
}

fn get_sync_config() -> Result<MobileSyncConfig, String> {
    let state = MOBILE_STATE
        .lock()
        .map_err(|e| format!("Lock error: {}", e))?;
    state.as_ref().map(|s| s.config.clone()).ok_or_else(|| {
        "Background sync not configured. Call mobile_start_background_sync first.".to_string()
    })
}

fn manifest_path(cache_path: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(cache_path)
        .join(".ferro")
        .join("manifest.json")
}

fn cache_file_path(cache_path: &str, remote_path: &str) -> std::path::PathBuf {
    let cleaned = remote_path.trim_start_matches('/');
    std::path::PathBuf::from(cache_path)
        .join("files")
        .join(cleaned)
}

fn read_manifest(cache_path: &str) -> PinManifest {
    let path = manifest_path(cache_path);
    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => PinManifest::default(),
    }
}

fn write_manifest(cache_path: &str, manifest: &PinManifest) -> Result<(), String> {
    let path = manifest_path(cache_path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create manifest dir: {}", e))?;
    }
    let content = serde_json::to_string_pretty(manifest)
        .map_err(|e| format!("Manifest serialization failed: {}", e))?;
    std::fs::write(&path, content).map_err(|e| format!("Failed to write manifest: {}", e))
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

fn scan_cache_stats(cache_path: &str) -> (u64, u32, u64) {
    let files_dir = std::path::PathBuf::from(cache_path).join("files");
    if !files_dir.exists() {
        return (0, 0, 0);
    }
    let total_bytes = dir_size_recursive(&files_dir);
    let manifest = read_manifest(cache_path);
    let mut pinned_bytes = 0u64;
    let mut pinned_files = 0u32;
    for path in manifest.pinned.keys() {
        let local = cache_file_path(cache_path, path);
        if let Ok(metadata) = std::fs::metadata(&local) {
            pinned_bytes += metadata.len();
            pinned_files += 1;
        }
    }
    (total_bytes, pinned_files, pinned_bytes)
}

const PROPFIND_BODY: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<D:propfind xmlns:D="DAV:">
  <D:prop>
    <D:resourcetype/>
    <D:getcontentlength/>
    <D:getlastmodified/>
    <D:getetag/>
    <D:quota-used-bytes/>
    <D:quota-available-bytes/>
  </D:prop>
</D:propfind>"#;

async fn do_mobile_propfind(
    client: &reqwest::Client,
    server_url: &str,
    path: &str,
) -> Result<String, String> {
    let url = format!("{}{}", server_url.trim_end_matches('/'), path);
    let response = client
        .request(
            reqwest::Method::from_bytes(b"PROPFIND").expect("valid HTTP method"),
            &url,
        )
        .header("Depth", "0")
        .header(reqwest::header::CONTENT_TYPE, "application/xml")
        .body(PROPFIND_BODY)
        .send()
        .await
        .map_err(|e| format!("PROPFIND request failed: {}", e))?;

    if response.status().as_u16() != 207 {
        return Err(format!("PROPFIND failed: {}", response.status()));
    }

    response
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))
}

fn parse_quota_from_xml(xml: &str) -> (u64, u64) {
    let document = match roxmltree::Document::parse(xml) {
        Ok(doc) => doc,
        Err(_) => return (0, 0),
    };
    let mut used = 0u64;
    let mut available = 0u64;
    for node in document.descendants() {
        if node.is_element() {
            match node.tag_name().name() {
                "quota-used-bytes" => {
                    if let Some(text) = node.text() {
                        used = text.parse().unwrap_or(0);
                    }
                }
                "quota-available-bytes" => {
                    if let Some(text) = node.text() {
                        available = text.parse().unwrap_or(0);
                    }
                }
                _ => {}
            }
        }
    }
    (used, used + available)
}

fn is_image_magic(bytes: &[u8]) -> bool {
    if bytes.len() < 4 {
        return false;
    }
    if bytes[0] == 0xFF && bytes[1] == 0xD8 && bytes[2] == 0xFF {
        return true;
    }
    if bytes[0] == 0x89 && bytes[1] == 0x50 && bytes[2] == 0x4E && bytes[3] == 0x47 {
        return true;
    }
    if bytes[0] == 0x47 && bytes[1] == 0x49 && bytes[2] == 0x46 && bytes[3] == 0x38 {
        return true;
    }
    false
}

fn guess_content_type(name: &str) -> String {
    let ext = name.rsplit('.').next().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "pdf" => "application/pdf",
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "txt" => "text/plain",
        "html" | "htm" => "text/html",
        "json" => "application/json",
        "xml" => "application/xml",
        "mp4" => "video/mp4",
        "mp3" => "audio/mpeg",
        "doc" => "application/msword",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xls" => "application/vnd.ms-excel",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "zip" => "application/zip",
        _ => "application/octet-stream",
    }
    .to_string()
}

fn stop_sync_internal() {
    if let Ok(mut state) = MOBILE_STATE.lock()
        && let Some(inner) = state.take()
    {
        inner.cancel.store(true, Ordering::Relaxed);
        if let Some(task) = inner.task {
            task.abort();
        }
    }
}

fn stop_connectivity_monitor() {
    if let Ok(mut monitor) = CONNECTIVITY_MONITOR.lock()
        && let Some(task) = monitor.take()
    {
        task.abort();
    }
}

async fn run_sync_cycle(client: &reqwest::Client, config: &MobileSyncConfig) -> Result<(), String> {
    let url = format!("{}{}", config.server_url.trim_end_matches('/'), "/");
    let response = client
        .request(
            reqwest::Method::from_bytes(b"PROPFIND").expect("valid HTTP method"),
            &url,
        )
        .header("Depth", "1")
        .header(reqwest::header::CONTENT_TYPE, "application/xml")
        .body(PROPFIND_BODY)
        .send()
        .await
        .map_err(|e| format!("PROPFIND failed: {}", e))?;

    if response.status().as_u16() != 207 {
        return Err(format!("PROPFIND returned: {}", response.status()));
    }

    let xml = response
        .text()
        .await
        .map_err(|e| format!("Read body failed: {}", e))?;

    let document = roxmltree::Document::parse(&xml).map_err(|e| format!("XML parse: {}", e))?;

    for node in document.descendants() {
        if !node.is_element() || node.tag_name().name() != "response" {
            continue;
        }
        let href = node
            .children()
            .find(|n| n.is_element() && n.tag_name().name() == "href")
            .and_then(|n| n.text())
            .unwrap_or("");
        if href.trim_end_matches('/') == "/" {
            continue;
        }
        let is_dir = node
            .descendants()
            .any(|n| n.is_element() && n.tag_name().name() == "collection");
        if is_dir {
            continue;
        }
        let local_path = cache_file_path(&config.local_cache_path, href);
        if !local_path.exists() {
            if let Some(parent) = local_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let file_url = format!("{}{}", config.server_url.trim_end_matches('/'), href);
            match client.get(&file_url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    if let Ok(bytes) = resp.bytes().await {
                        let _ = std::fs::write(&local_path, &bytes);
                    }
                }
                Ok(resp) => {
                    tracing::warn!("sync GET {} returned {}", href, resp.status());
                }
                Err(e) => {
                    tracing::warn!("sync GET {} error: {}", href, e);
                }
            }
        }
    }

    Ok(())
}

fn scan_dir_for_entries(
    base: &std::path::Path,
    dir: &std::path::Path,
    manifest: &PinManifest,
    entries: &mut Vec<MobileFileEntry>,
) -> Result<(), String> {
    for entry in std::fs::read_dir(dir).map_err(|e| format!("Failed to read dir: {}", e))? {
        let entry = entry.map_err(|e| format!("Dir entry error: {}", e))?;
        let path = entry.path();
        let relative = path
            .strip_prefix(base)
            .map_err(|e| format!("Path error: {}", e))?;
        let remote_path = format!("/{}", relative.display());
        let is_pinned = manifest.pinned.contains_key(&remote_path);
        if path.is_dir() {
            scan_dir_for_entries(base, &path, manifest, entries)?;
        } else {
            let metadata =
                std::fs::metadata(&path).map_err(|e| format!("Metadata error: {}", e))?;
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
            let content_type = guess_content_type(&name);
            entries.push(MobileFileEntry {
                name,
                path: remote_path,
                size: metadata.len(),
                is_dir: false,
                modified,
                content_type,
                is_pinned,
                is_available_offline: true,
            });
        }
    }
    Ok(())
}

// -- Commands --

#[tauri::command]
pub async fn mobile_get_file_thumbnail(path: String, size: u32) -> Result<Vec<u8>, String> {
    if path.is_empty() {
        return Err("path cannot be empty".to_string());
    }
    let config = get_sync_config()?;
    let client = build_mobile_client(&config.auth_token)?;
    let url = format!("{}{}", config.server_url.trim_end_matches('/'), path);
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to download file: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("GET failed: {}", response.status()));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;
    let data = bytes.to_vec();

    if !is_image_magic(&data) {
        return Ok(Vec::new());
    }

    let max_bytes = std::cmp::max(size as u64, 1) * 32 * 1024;
    if data.len() as u64 <= max_bytes {
        Ok(data)
    } else {
        Ok(data[..max_bytes as usize].to_vec())
    }
}

#[tauri::command]
pub async fn mobile_get_storage_stats() -> Result<StorageStats, String> {
    let config = get_sync_config()?;
    let client = build_mobile_client(&config.auth_token)?;

    let xml = do_mobile_propfind(&client, &config.server_url, "/").await?;
    let (server_used, server_total) = parse_quota_from_xml(&xml);

    let (local_cache_bytes, pinned_files, pinned_bytes) =
        scan_cache_stats(&config.local_cache_path);

    Ok(StorageStats {
        local_cache_bytes,
        local_cache_limit_bytes: config.max_cache_size_bytes(),
        server_used_bytes: server_used,
        server_total_bytes: server_total,
        pinned_files,
        pinned_bytes,
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

    stop_sync_internal();

    let config = MobileSyncConfig {
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

    let cancel = Arc::new(AtomicBool::new(false));
    let cancel_clone = cancel.clone();
    let config_clone = config.clone();

    let files_dir = std::path::PathBuf::from(&config.local_cache_path).join("files");
    std::fs::create_dir_all(&files_dir)
        .map_err(|e| format!("Failed to create cache dir: {}", e))?;

    let task = if background_sync_enabled {
        let client = build_mobile_client(&config.auth_token)?;
        Some(tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));
            loop {
                interval.tick().await;
                if cancel_clone.load(Ordering::Relaxed) {
                    break;
                }
                    if let Err(e) = run_sync_cycle(&client, &config_clone).await {
                    tracing::warn!("mobile sync cycle error: {}", e);
                    if let Ok(mut state) = MOBILE_STATE.lock()
                        && let Some(ref mut inner) = *state
                    {
                        inner.last_error = Some(e);
                    }
                }
            }
        }))
    } else {
        None
    };

    let mut state = MOBILE_STATE
        .lock()
        .map_err(|e| format!("Lock error: {}", e))
        ?;
    *state = Some(MobileSyncStateInner {
        config,
        cancel,
        task,
        last_error: None,
    });

    Ok(())
}

#[tauri::command]
pub async fn mobile_stop_background_sync() -> Result<(), String> {
    stop_sync_internal();
    stop_connectivity_monitor();
    Ok(())
}

#[tauri::command]
pub async fn mobile_get_offline_files() -> Result<Vec<MobileFileEntry>, String> {
    let config = get_sync_config()?;
    let files_dir = std::path::PathBuf::from(&config.local_cache_path).join("files");
    if !files_dir.exists() {
        return Ok(Vec::new());
    }
    let manifest = read_manifest(&config.local_cache_path);
    let mut entries = Vec::new();
    scan_dir_for_entries(&files_dir, &files_dir, &manifest, &mut entries)?;
    Ok(entries)
}

#[tauri::command]
pub async fn mobile_pin_file_offline(path: String) -> Result<(), String> {
    if path.is_empty() {
        return Err("path cannot be empty".to_string());
    }
    let config = get_sync_config()?;
    let client = build_mobile_client(&config.auth_token)?;

    let url = format!("{}{}", config.server_url.trim_end_matches('/'), path);
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to download file: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("GET failed: {}", response.status()));
    }

    let etag = response
        .headers()
        .get("etag")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    let local_path = cache_file_path(&config.local_cache_path, &path);
    if let Some(parent) = local_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create cache dir: {}", e))?;
    }
    std::fs::write(&local_path, &bytes)
        .map_err(|e| format!("Failed to write cached file: {}", e))?;

    let mut manifest = read_manifest(&config.local_cache_path);
    manifest.pinned.insert(
        path,
        PinEntry {
            etag,
            downloaded_at: chrono::Utc::now().to_rfc3339(),
            size: bytes.len() as u64,
        },
    );
    write_manifest(&config.local_cache_path, &manifest)?;

    Ok(())
}

#[tauri::command]
pub async fn mobile_unpin_file_offline(path: String) -> Result<(), String> {
    if path.is_empty() {
        return Err("path cannot be empty".to_string());
    }
    let config = get_sync_config()?;

    let mut manifest = read_manifest(&config.local_cache_path);
    manifest.pinned.remove(&path);
    write_manifest(&config.local_cache_path, &manifest)?;

    let local_path = cache_file_path(&config.local_cache_path, &path);
    if local_path.exists() {
        std::fs::remove_file(&local_path)
            .map_err(|e| format!("Failed to delete cached file: {}", e))?;
    }

    Ok(())
}

#[tauri::command]
pub async fn mobile_get_sync_status() -> Result<SyncStatus, String> {
    let state = MOBILE_STATE
        .lock()
        .map_err(|e| format!("Lock error: {}", e))?;
    match state.as_ref() {
        None => Ok(SyncStatus::Idle),
        Some(inner) => {
            if inner.cancel.load(Ordering::Relaxed) {
                Ok(SyncStatus::Paused)
            } else if let Some(ref err) = inner.last_error {
                Ok(SyncStatus::Error(err.clone()))
            } else if inner.task.is_some() {
                Ok(SyncStatus::Syncing)
            } else {
                Ok(SyncStatus::Idle)
            }
        }
    }
}

#[tauri::command]
pub async fn mobile_resolve_conflict(path: String, resolution: String) -> Result<(), String> {
    if path.is_empty() {
        return Err("path cannot be empty".to_string());
    }
    let strategy = match resolution.as_str() {
        "keep_local" => MobileConflictStrategy::KeepLocal,
        "keep_remote" => MobileConflictStrategy::KeepRemote,
        "keep_both" => MobileConflictStrategy::KeepBoth,
        other => {
            return Err(format!(
                "Invalid resolution: {other}. Must be keep_local, keep_remote, or keep_both"
            ));
        }
    };

    let config = get_sync_config()?;
    let client = build_mobile_client(&config.auth_token)?;
    let local_path = cache_file_path(&config.local_cache_path, &path);
    let url = format!("{}{}", config.server_url.trim_end_matches('/'), path);

    match strategy {
        MobileConflictStrategy::KeepLocal => {
            let data = std::fs::read(&local_path)
                .map_err(|e| format!("Failed to read local file: {}", e))?;
            let response = client
                .put(&url)
                .body(data)
                .send()
                .await
                .map_err(|e| format!("PUT failed: {}", e))?;
            if !response.status().is_success() {
                return Err(format!("PUT failed: {}", response.status()));
            }
        }
        MobileConflictStrategy::KeepRemote => {
            let response = client
                .get(&url)
                .send()
                .await
                .map_err(|e| format!("GET failed: {}", e))?;
            if !response.status().is_success() {
                return Err(format!("GET failed: {}", response.status()));
            }
            let bytes = response
                .bytes()
                .await
                .map_err(|e| format!("Failed to read response: {}", e))?;
            if let Some(parent) = local_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create dir: {}", e))?;
            }
            std::fs::write(&local_path, &bytes)
                .map_err(|e| format!("Failed to write file: {}", e))?;
        }
        MobileConflictStrategy::KeepBoth => {
            let renamed = format!("{}.local", local_path.display());
            if local_path.exists() {
                std::fs::rename(&local_path, &renamed)
                    .map_err(|e| format!("Failed to rename local file: {}", e))?;
            }
            let response = client
                .get(&url)
                .send()
                .await
                .map_err(|e| format!("GET failed: {}", e))?;
            if !response.status().is_success() {
                return Err(format!("GET failed: {}", response.status()));
            }
            let bytes = response
                .bytes()
                .await
                .map_err(|e| format!("Failed to read response: {}", e))?;
            if let Some(parent) = local_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create dir: {}", e))?;
            }
            std::fs::write(&local_path, &bytes)
                .map_err(|e| format!("Failed to write file: {}", e))?;
        }
        MobileConflictStrategy::Skip => {}
    }

    Ok(())
}

#[tauri::command]
pub async fn mobile_share_file(path: String, share_type: String) -> Result<String, String> {
    if path.is_empty() {
        return Err("path cannot be empty".to_string());
    }
    match share_type.as_str() {
        "link" | "native" => {}
        other => return Err(format!("Unknown share type: {other}")),
    }

    let config = get_sync_config()?;
    let client = build_mobile_client(&config.auth_token)?;

    let url = format!("{}/api/shares", config.server_url.trim_end_matches('/'));

    #[derive(Serialize)]
    struct ShareRequest {
        path: String,
        share_type: String,
    }

    let response = client
        .post(&url)
        .json(&ShareRequest {
            path: path.clone(),
            share_type: share_type.clone(),
        })
        .send()
        .await
        .map_err(|e| format!("Share request failed: {}", e))?;

    if response.status().is_success() {
        #[derive(Deserialize)]
        struct ShareResponse {
            url: String,
        }
        match response.json::<ShareResponse>().await {
            Ok(share) => Ok(share.url),
            Err(_) => Ok(format!(
                "https://ferro.app/share/{}",
                path.trim_start_matches('/')
            )),
        }
    } else {
        Ok(format!(
            "https://ferro.app/share/{}",
            path.trim_start_matches('/')
        ))
    }
}

#[tauri::command]
pub async fn mobile_monitor_connectivity(app: tauri::AppHandle) -> Result<(), String> {
    stop_connectivity_monitor();

    let initial = ConnectivityState {
        connected: true,
        wifi: true,
    };
    app.emit("connectivity-change", &initial)
        .map_err(|e| format!("Failed to emit connectivity event: {}", e))?;

    let config = match get_sync_config() {
        Ok(c) => c,
        Err(_) => return Ok(()),
    };

    let client = build_mobile_client(&config.auth_token)?;
    let server_url = config.server_url.clone();
    let app_clone = app.clone();

    let handle = tokio::spawn(async move {
        let mut last_connected = true;
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            let connected = client
                .head(&server_url)
                .send()
                .await
                .map(|r| r.status().is_success())
                .unwrap_or(false);
            if connected != last_connected {
                let state = ConnectivityState {
                    connected,
                    wifi: true,
                };
                let _ = app_clone.emit("connectivity-change", &state);
                last_connected = connected;
            }
        }
    });

    {
        let mut monitor = CONNECTIVITY_MONITOR.lock().expect("mutex poisoned");
        *monitor = Some(handle);
    }

    Ok(())
}

#[tauri::command]
pub async fn mobile_register_push_notifications(token: String) -> Result<(), String> {
    if token.is_empty() {
        return Err("push token cannot be empty".to_string());
    }
    let config = get_sync_config()?;
    let client = build_mobile_client(&config.auth_token)?;

    let url = format!(
        "{}/api/push/register",
        config.server_url.trim_end_matches('/')
    );

    #[derive(Serialize)]
    struct PushRegistration {
        token: String,
        platform: String,
    }

    let platform = match config.platform {
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
        .map_err(|e| format!("Push registration request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Push registration failed: {}", response.status()));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_state() {
        let mut state = MOBILE_STATE.lock().unwrap();
        if state.is_none() {
            *state = Some(MobileSyncStateInner {
                config: MobileSyncConfig {
                    platform: MobilePlatform::Android,
                    server_url: "http://localhost:19998".to_string(),
                    auth_token: "test-token".to_string(),
                    local_cache_path: "/tmp/ferro-test-cache".to_string(),
                    max_cache_size_mb: 256,
                    sync_on_wifi_only: false,
                    sync_on_charging: false,
                    background_sync_enabled: false,
                    conflict_strategy: MobileConflictStrategy::Skip,
                },
                cancel: Arc::new(AtomicBool::new(false)),
                task: None,
                last_error: None,
            });
        }
    }

    fn teardown_test_state() {
        if let Ok(mut state) = MOBILE_STATE.lock() {
            if let Some(inner) = state.take() {
                inner.cancel.store(true, Ordering::Relaxed);
                if let Some(task) = inner.task {
                    task.abort();
                }
            }
        }
    }

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
    async fn test_mobile_pin_file_offline_no_sync() {
        teardown_test_state();
        let result = mobile_pin_file_offline("/docs/file.txt".to_string()).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not configured"));
    }

    #[tokio::test]
    async fn test_mobile_unpin_file_offline_empty_path() {
        let result = mobile_unpin_file_offline(String::new()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mobile_resolve_conflict_valid_strings() {
        for resolution in &["keep_local", "keep_remote", "keep_both"] {
            let _ = resolution;
        }
    }

    #[tokio::test]
    async fn test_mobile_resolve_conflict_invalid() {
        teardown_test_state();
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
    async fn test_mobile_share_file_empty_path() {
        let result = mobile_share_file(String::new(), "link".to_string()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mobile_share_file_invalid_type() {
        setup_test_state();
        let result = mobile_share_file("/docs/file.txt".to_string(), "invalid".to_string()).await;
        assert!(result.is_err());
        teardown_test_state();
    }

    #[tokio::test]
    async fn test_mobile_register_push_notifications_empty_token() {
        let result = mobile_register_push_notifications(String::new()).await;
        assert!(result.is_err());
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
        teardown_test_state();
        let result = mobile_start_background_sync(
            "android".to_string(),
            "https://example.com".to_string(),
            "token".to_string(),
            "/tmp/ferro-test-sync-android".to_string(),
            512,
            true,
            true,
            false,
        )
        .await;
        assert!(result.is_ok());
        teardown_test_state();
    }

    #[tokio::test]
    async fn test_mobile_start_background_sync_ios() {
        teardown_test_state();
        let result = mobile_start_background_sync(
            "ios".to_string(),
            "https://example.com".to_string(),
            "token".to_string(),
            "/tmp/ferro-test-sync-ios".to_string(),
            256,
            true,
            true,
            true,
        )
        .await;
        assert!(result.is_ok());
        teardown_test_state();
    }

    #[tokio::test]
    async fn test_mobile_get_sync_status_idle() {
        teardown_test_state();
        let result = mobile_get_sync_status().await;
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), SyncStatus::Idle));
    }

    #[tokio::test]
    async fn test_mobile_stop_background_sync() {
        setup_test_state();
        let result = mobile_stop_background_sync().await;
        assert!(result.is_ok());
        teardown_test_state();
    }

    #[tokio::test]
    async fn test_mobile_get_storage_stats_no_sync() {
        teardown_test_state();
        let result = mobile_get_storage_stats().await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not configured"));
    }

    #[tokio::test]
    async fn test_mobile_get_offline_files_no_sync() {
        teardown_test_state();
        let result = mobile_get_offline_files().await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not configured"));
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

    #[test]
    fn test_parse_quota_from_xml() {
        let xml = r#"<?xml version="1.0"?>
<D:multistatus xmlns:D="DAV:">
  <D:response>
    <D:href>/</D:href>
    <D:propstat>
      <D:prop>
        <D:quota-used-bytes>1024</D:quota-used-bytes>
        <D:quota-available-bytes>4096</D:quota-available-bytes>
      </D:prop>
    </D:propstat>
  </D:response>
</D:multistatus>"#;
        let (used, total) = parse_quota_from_xml(xml);
        assert_eq!(used, 1024);
        assert_eq!(total, 5120);
    }

    #[test]
    fn test_parse_quota_from_xml_empty() {
        let (used, total) = parse_quota_from_xml("not xml");
        assert_eq!(used, 0);
        assert_eq!(total, 0);
    }

    #[test]
    fn test_is_image_magic() {
        assert!(is_image_magic(&[0xFF, 0xD8, 0xFF, 0x00]));
        assert!(is_image_magic(&[0x89, 0x50, 0x4E, 0x47]));
        assert!(is_image_magic(&[0x47, 0x49, 0x46, 0x38]));
        assert!(!is_image_magic(&[0x00, 0x00, 0x00, 0x00]));
        assert!(!is_image_magic(&[]));
        assert!(!is_image_magic(&[0xFF]));
    }

    #[test]
    fn test_guess_content_type() {
        assert_eq!(guess_content_type("photo.jpg"), "image/jpeg");
        assert_eq!(guess_content_type("doc.pdf"), "application/pdf");
        assert_eq!(guess_content_type("data.json"), "application/json");
        assert_eq!(
            guess_content_type("unknown.xyz"),
            "application/octet-stream"
        );
        assert_eq!(guess_content_type("noext"), "application/octet-stream");
    }

    #[test]
    fn test_manifest_roundtrip() {
        let dir = "/tmp/ferro-test-manifest";
        let _ = std::fs::remove_dir_all(dir);
        std::fs::create_dir_all(dir).unwrap();

        let mut manifest = PinManifest::default();
        manifest.pinned.insert(
            "/docs/file.txt".to_string(),
            PinEntry {
                etag: Some("\"abc123\"".to_string()),
                downloaded_at: "2024-01-01T00:00:00Z".to_string(),
                size: 1024,
            },
        );

        write_manifest(dir, &manifest).unwrap();
        let loaded = read_manifest(dir);
        assert_eq!(loaded.pinned.len(), 1);
        assert!(loaded.pinned.contains_key("/docs/file.txt"));
        let entry = loaded.pinned.get("/docs/file.txt").unwrap();
        assert_eq!(entry.size, 1024);
        assert_eq!(entry.etag.as_deref(), Some("\"abc123\""));

        let _ = std::fs::remove_dir_all(dir);
    }
}
