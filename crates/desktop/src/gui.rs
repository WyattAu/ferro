use tauri::{
    Emitter, Manager, State,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
};

#[cfg(target_os = "linux")]
use std::sync::mpsc;

#[cfg(all(feature = "tauri", feature = "android"))]
#[path = "android.rs"]
mod android;

use ferro_desktop::commands::DesktopState;
use ferro_desktop::commands::{ConfigResponse, MountStatusResponse, SaveConfigRequest};
use ferro_desktop::config::DesktopConfig;
use ferro_desktop::rclone::MountProgress;

#[cfg(feature = "mobile")]
use ferro_desktop::mobile_commands;

use serde::{Deserialize, Serialize};

/// CLI arguments passed from main() to gui::run().
/// These allow automated/headless testing without manual form input.
#[derive(Debug, Clone)]
pub struct CliArgs {
    /// Pre-configured server URL (e.g. http://localhost:8080).
    /// When set, the frontend auto-connects and skips the connect form.
    pub server_url: Option<String>,

    /// Pre-configured auth token (Bearer token or basic auth credentials).
    pub auth_token: Option<String>,

    /// Debug verbosity level (0=off, 1=debug desktop, 2=debug all).
    pub debug: u8,

    /// Auto-audit mode: navigate all pages, screenshot each, then exit.
    pub audit: bool,
}

impl CliArgs {
    /// Parse CLI args using a minimal parser (avoid full clap dep in tauri mode).
    pub fn parse() -> Self {
        let raw: Vec<String> = std::env::args().skip(1).collect();
        let mut server_url = None;
        let mut auth_token = None;
        let mut debug = 0u8;
        let mut audit = false;

        let mut i = 0;
        while i < raw.len() {
            match raw[i].as_str() {
                "--server-url" | "-s" => {
                    if i + 1 < raw.len() {
                        server_url = Some(raw[i + 1].clone());
                        i += 2;
                    } else {
                        eprintln!("--server-url requires a value");
                        std::process::exit(1);
                    }
                }
                "--auth-token" | "-t" => {
                    if i + 1 < raw.len() {
                        auth_token = Some(raw[i + 1].clone());
                        i += 2;
                    } else {
                        eprintln!("--auth-token requires a value");
                        std::process::exit(1);
                    }
                }
                "--debug" | "-d" => {
                    debug = 1;
                    i += 1;
                }
                "-dd" => {
                    debug = 2;
                    i += 1;
                }
                "--audit" => {
                    audit = true;
                    i += 1;
                }
                "--help" | "-h" => {
                    println!("Usage: ferro-desktop [OPTIONS]");
                    println!();
                    println!("Options:");
                    println!("  -s, --server-url <URL>    Server URL (auto-connects, skips form)");
                    println!("  -t, --auth-token <TOKEN>  Auth token (Bearer or user:pass)");
                    println!("  -d, --debug              Enable debug logging to /tmp/ferro-desktop.log");
                    println!("  -dd                      Verbose debug logging");
                    println!("  --audit                  Auto-audit: screenshot all pages, then exit");
                    println!("  -h, --help               Show this help");
                    std::process::exit(0);
                }
                other if other.starts_with('-') => {
                    eprintln!("Unknown option: {other}");
                    std::process::exit(1);
                }
                _ => {
                    i += 1;
                }
            }
        }

        Self {
            server_url,
            auth_token,
            debug,
            audit,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CliConnection {
    /// Server URL, if provided via CLI.
    pub server_url: Option<String>,
    /// Auth token, if provided via CLI.
    pub auth_token: Option<String>,
}

/// Capture a screenshot of the webview using webkit2gtk's native snapshot API.
#[tauri::command]
#[cfg(feature = "screenshot")]
async fn capture_screenshot(app_handle: tauri::AppHandle, path: String) -> Result<String, String> {
    use tauri::Manager;
    let w = app_handle.get_webview_window("main").ok_or("no main window")?;

    #[cfg(target_os = "linux")]
    {
        use webkit2gtk::WebViewExt;
        use webkit2gtk::{SnapshotOptions, SnapshotRegion};
        let (tx, rx) = std::sync::mpsc::channel();
        w.with_webview(move |webview| {
            let wv = webview.inner();
            wv.snapshot(
                SnapshotRegion::Visible,
                SnapshotOptions::NONE,
                None::<&gtk::gio::Cancellable>,
                move |result| {
                    match result {
                        Ok(surface) => {
                            // Snapshot returns a cairo::Surface which is actually an ImageSurface
                            // Use unsafe transmute since webkit2gtk always returns ImageSurface for snapshots
                            let img: cairo::ImageSurface = unsafe { std::mem::transmute(surface) };
                            let mut f = std::fs::File::create(&path).unwrap();
                            let _ = img.write_to_png(&mut f);
                            let _ = tx.send(format!("saved to {}", path));
                        }
                        Err(e) => {
                            let _ = tx.send(format!("snapshot error: {}", e));
                        }
                    }
                },
            );
        })
        .map_err(|e| format!("with_webview failed: {}", e))?;

        rx.recv_timeout(std::time::Duration::from_secs(5))
            .map_err(|e| format!("timeout: {}", e))
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = w;
        let _ = path;
        Err("screenshot only supported on Linux/webkit2gtk".to_string())
    }
}

/// Returns pre-configured connection info from CLI args.
/// The frontend calls this on init to auto-connect without showing the form.
#[tauri::command]
fn dump_layout(data: String) -> Result<String, String> {
    let path = std::env::temp_dir().join("ferro-layout.txt");
    std::fs::write(&path, &data).map_err(|e| e.to_string())?;
    Ok(data)
}

/// Returns pre-configured connection info from CLI args.
/// The frontend calls this on init to auto-connect without showing the form.
#[tauri::command]
fn get_cli_connection(state: State<'_, CliConnection>) -> CliConnection {
    state.inner().clone()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub is_dir: bool,
    pub modified: String,
    pub etag: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectInfo {
    pub server_name: String,
    pub root_files: u64,
}

fn build_client(token: &str) -> Result<reqwest::Client, String> {
    let mut headers = reqwest::header::HeaderMap::new();

    // Detect auth format:
    // - If token contains ":" it's user:pass (Basic auth) -- base64 encode it
    // - If token decodes from base64 to user:pass, use Basic as-is
    // - Otherwise treat as Bearer token
    let auth_header = if token.is_empty() {
        // No auth -- skip header
        String::new()
    } else if token.contains(':') {
        // Raw user:pass -- base64 encode as Basic
        const BASE64_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let input = token.as_bytes();
        let mut output = Vec::with_capacity((input.len() + 2) / 3 * 4);
        for chunk in input.chunks(3) {
            let b0 = chunk[0] as u32;
            let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
            let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
            let triple = (b0 << 16) | (b1 << 8) | b2;
            output.push(BASE64_CHARS[((triple >> 18) & 0x3F) as usize]);
            output.push(BASE64_CHARS[((triple >> 12) & 0x3F) as usize]);
            if chunk.len() > 1 {
                output.push(BASE64_CHARS[((triple >> 6) & 0x3F) as usize]);
            } else {
                output.push(b'=');
            }
            if chunk.len() > 2 {
                output.push(BASE64_CHARS[(triple & 0x3F) as usize]);
            } else {
                output.push(b'=');
            }
        }
        let encoded = String::from_utf8(output).unwrap_or_default();
        format!("Basic {encoded}")
    } else if let Some(decoded) = try_base64_decode(token) {
        if decoded.contains(':') {
            format!("Basic {token}")
        } else {
            format!("Bearer {token}")
        }
    } else {
        format!("Bearer {token}")
    };

    if !auth_header.is_empty() {
        let value =
            reqwest::header::HeaderValue::from_str(&auth_header).map_err(|e| format!("Invalid token: {}", e))?;
        headers.insert(reqwest::header::AUTHORIZATION, value);
    }
    reqwest::Client::builder()
        .no_proxy()
        .default_headers(headers)
        .timeout(std::time::Duration::from_secs(30))
        .connect_timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))
}

/// URL percent-decode a string (e.g. %20 -> space).
fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hi = hex_val(bytes[i + 1]);
            let lo = hex_val(bytes[i + 2]);
            if let (Some(h), Some(l)) = (hi, lo) {
                output.push(h * 16 + l);
                i += 3;
                continue;
            }
        }
        output.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(output).unwrap_or_else(|_| input.to_string())
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Try to decode a base64 string. Returns None if invalid.
fn try_base64_decode(input: &str) -> Option<String> {
    const BASE64_TABLE: [i8; 256] = {
        let mut table = [0i8; 256];
        let chars = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut i = 0;
        while i < 64 {
            table[chars[i] as usize] = i as i8;
            i += 1;
        }
        table[b'=' as usize] = -1;
        table
    };

    let input = input.as_bytes();
    let mut output = Vec::with_capacity(input.len() * 3 / 4);
    let mut buf: u32 = 0;
    let mut bits: u32 = 0;

    for &byte in input {
        let val = BASE64_TABLE[byte as usize];
        if val == -1 {
            // padding, flush remaining
            if bits >= 6 {
                output.push((buf >> (bits - 6)) as u8);
            }
            break;
        } else if val >= 0 {
            buf = (buf << 6) | (val as u32);
            bits += 6;
            if bits >= 8 {
                bits -= 8;
                output.push((buf >> bits) as u8);
            }
        }
    }

    String::from_utf8(output).ok()
}

fn parse_propfind_response(xml: &str, base_path: &str) -> Vec<FileEntry> {
    let document = match roxmltree::Document::parse(xml) {
        Ok(doc) => doc,
        Err(_) => return Vec::new(),
    };

    let base_normalized = base_path.trim_end_matches('/');
    let mut entries = Vec::new();

    for node in document.descendants() {
        if !node.is_element() || node.tag_name().name() != "response" {
            continue;
        }

        let href = node
            .children()
            .find(|n| n.is_element() && n.tag_name().name() == "href")
            .and_then(|n| n.text())
            .unwrap_or("");

        let href_decoded = percent_decode(href);
        let href_normalized = href_decoded.trim_end_matches('/');

        if href_normalized.is_empty() || href_normalized == base_normalized {
            continue;
        }

        let name = href_normalized
            .trim_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or("")
            .to_string();

        if name.is_empty() {
            continue;
        }

        // Use the decoded href as the path so the frontend gets clean paths
        let href = href_decoded.as_str();

        let prop = node
            .children()
            .find(|n| n.is_element() && n.tag_name().name() == "propstat")
            .and_then(|ps| ps.children().find(|n| n.is_element() && n.tag_name().name() == "prop"));

        let is_dir = prop.is_some_and(|p| {
            p.descendants()
                .any(|n| n.is_element() && n.tag_name().name() == "collection")
        });

        let size = prop
            .and_then(|p| {
                p.children()
                    .find(|n| n.is_element() && n.tag_name().name() == "getcontentlength")
                    .and_then(|n| n.text())
                    .and_then(|t| t.parse::<u64>().ok())
            })
            .unwrap_or(0);

        let modified = prop
            .and_then(|p| {
                p.children()
                    .find(|n| n.is_element() && n.tag_name().name() == "getlastmodified")
                    .and_then(|n| n.text())
                    .map(|t| t.to_string())
            })
            .unwrap_or_default();

        let etag = prop.and_then(|p| {
            p.children()
                .find(|n| n.is_element() && n.tag_name().name() == "getetag")
                .and_then(|n| n.text())
                .map(|t| t.to_string())
        });

        entries.push(FileEntry {
            name,
            path: href.to_string(),
            size,
            is_dir,
            modified,
            etag,
        });
    }

    // Deduplicate by decoded path (handles double-encoding like Shared%2520Projects)
    // First decode each path, then dedup keeping the first occurrence (cleaner name)
    let mut seen = std::collections::HashSet::new();
    entries.retain(|e| seen.insert(percent_decode(&e.path)));

    entries
}

const PROPFIND_BODY: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<D:propfind xmlns:D="DAV:">
  <D:prop>
    <D:resourcetype/>
    <D:getcontentlength/>
    <D:getlastmodified/>
    <D:getetag/>
    <D:displayname/>
  </D:prop>
</D:propfind>"#;

async fn do_propfind(client: &reqwest::Client, base_url: &str, path: &str, depth: &str) -> Result<String, String> {
    let url = format!("{}{}", base_url.trim_end_matches('/'), path);
    let response = client
        .request(
            reqwest::Method::from_bytes(b"PROPFIND").expect("valid HTTP method"),
            &url,
        )
        .header("Depth", depth)
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

#[tauri::command]
pub async fn list_directory(url: String, token: String, path: String, depth: Option<String>) -> Result<String, String> {
    let client = build_client(&token)?;
    let depth_val = depth.as_deref().unwrap_or("1");
    let xml = do_propfind(&client, &url, &path, depth_val).await?;
    let entries = parse_propfind_response(&xml, &path);
    serde_json::to_string(&entries).map_err(|e| format!("Serialization failed: {}", e))
}

/// REST-based file listing using GET /api/v1/files?path=.
/// Returns JSON matching the frontend's expected format (ListFilesResponse).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestFileEntry {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub is_collection: bool,
    pub mime_type: Option<String>,
    pub etag: Option<String>,
    pub content_hash: Option<String>,
    pub modified_at: Option<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestListFilesResponse {
    pub entries: Vec<RestFileEntry>,
}

#[tauri::command]
pub async fn list_files_rest(url: String, token: String, path: String) -> Result<String, String> {
    let client = build_client(&token)?;
    let base = url.trim_end_matches('/');
    let query_path = path.trim_start_matches('/');
    let api_url = format!("{}/api/v1/files?path=/{}", base, query_path);

    tracing::info!("[list_files_rest] GET {}", api_url);

    let response = client
        .get(&api_url)
        .header(reqwest::header::ACCEPT, "application/json")
        .send()
        .await
        .map_err(|e| {
            tracing::error!("[list_files_rest] request failed: {}", e);
            format!("GET /api/v1/files failed: {}", e)
        })?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        tracing::error!("[list_files_rest] HTTP {}: {}", status, body);
        return Err(format!("GET /api/v1/files returned {}: {}", status, body));
    }

    let body = response.text().await.map_err(|e| {
        tracing::error!("[list_files_rest] read body failed: {}", e);
        format!("Failed to read response: {}", e)
    })?;

    tracing::debug!("[list_files_rest] response ({} bytes)", body.len());

    // Parse the server response and normalize to frontend format
    let raw: serde_json::Value = serde_json::from_str(&body).map_err(|e| {
        tracing::error!(
            "[list_files_rest] parse failed: {} — body: {}",
            e,
            &body[..body.len().min(200)]
        );
        format!("JSON parse: {}", e)
    })?;

    // Extract entries from the response — server may return { entries: [...] } or just [...]
    let entries = if let Some(arr) = raw.get("entries").and_then(|v| v.as_array()) {
        arr.clone()
    } else if let Some(arr) = raw.as_array() {
        arr.clone()
    } else {
        tracing::warn!(
            "[list_files_rest] unexpected response shape: {}",
            &body[..body.len().min(200)]
        );
        vec![]
    };

    // Normalize each entry to frontend-compatible format
    let normalized_path = path.trim_end_matches('/');
    let mut normalized = Vec::with_capacity(entries.len());
    for entry in &entries {
        let name = entry.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let entry_path = entry.get("path").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let size = entry.get("size").and_then(|v| v.as_u64()).unwrap_or(0);
        let is_collection = entry
            .get("isCollection")
            .and_then(|v| v.as_bool())
            .or_else(|| entry.get("is_collection").and_then(|v| v.as_bool()))
            .unwrap_or(false);
        let mime_type = entry
            .get("mimeType")
            .and_then(|v| v.as_str())
            .or_else(|| entry.get("mime_type").and_then(|v| v.as_str()))
            .map(String::from);
        let etag = entry.get("etag").and_then(|v| v.as_str()).map(String::from);
        let modified_at = entry
            .get("modifiedAt")
            .and_then(|v| v.as_str())
            .or_else(|| entry.get("modified_at").and_then(|v| v.as_str()))
            .map(String::from);

        // Filter out the self-referential directory entry
        if entry_path.trim_end_matches('/') == normalized_path {
            continue;
        }

        normalized.push(RestFileEntry {
            name,
            path: entry_path,
            size,
            is_collection,
            mime_type,
            etag,
            content_hash: entry.get("contentHash").and_then(|v| v.as_str()).map(String::from),
            modified_at,
            created_at: entry
                .get("createdAt")
                .and_then(|v| v.as_str())
                .or_else(|| entry.get("created_at").and_then(|v| v.as_str()))
                .map(String::from),
        });
    }

    let result = RestListFilesResponse { entries: normalized };
    serde_json::to_string(&result).map_err(|e| format!("Serialization failed: {}", e))
}

#[tauri::command]
pub async fn get_file(url: String, token: String, path: String) -> Result<Vec<u8>, String> {
    let client = build_client(&token)?;
    let full_url = format!("{}{}", url.trim_end_matches('/'), path);
    let response = client
        .get(&full_url)
        .send()
        .await
        .map_err(|e| format!("GET request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("GET failed: {}", response.status()));
    }

    response
        .bytes()
        .await
        .map(|b| b.to_vec())
        .map_err(|e| format!("Failed to read body: {}", e))
}

#[tauri::command]
pub async fn put_file(url: String, token: String, path: String, data: Vec<u8>) -> Result<(), String> {
    let client = build_client(&token)?;
    let full_url = format!("{}{}", url.trim_end_matches('/'), path);
    let response = client
        .put(&full_url)
        .body(data)
        .send()
        .await
        .map_err(|e| format!("PUT request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("PUT failed: {}", response.status()));
    }

    Ok(())
}

#[tauri::command]
pub async fn create_directory(url: String, token: String, path: String) -> Result<(), String> {
    let client = build_client(&token)?;
    let full_url = format!("{}{}", url.trim_end_matches('/'), path);
    let response = client
        .request(
            reqwest::Method::from_bytes(b"MKCOL").expect("valid HTTP method"),
            &full_url,
        )
        .send()
        .await
        .map_err(|e| format!("MKCOL request failed: {}", e))?;

    if response.status().as_u16() != 201 {
        return Err(format!("MKCOL failed: {}", response.status()));
    }

    Ok(())
}

#[tauri::command]
pub async fn delete_item(url: String, token: String, path: String) -> Result<(), String> {
    let client = build_client(&token)?;
    let full_url = format!("{}{}", url.trim_end_matches('/'), path);
    let response = client
        .delete(&full_url)
        .send()
        .await
        .map_err(|e| format!("DELETE request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("DELETE failed: {}", response.status()));
    }

    Ok(())
}

#[tauri::command]
pub async fn move_item(url: String, token: String, from: String, to: String) -> Result<(), String> {
    let client = build_client(&token)?;
    let from_url = format!("{}{}", url.trim_end_matches('/'), from);
    let to_url = format!("{}{}", url.trim_end_matches('/'), to);
    let response = client
        .request(
            reqwest::Method::from_bytes(b"MOVE").expect("valid HTTP method"),
            &from_url,
        )
        .header("Destination", &to_url)
        .send()
        .await
        .map_err(|e| format!("MOVE request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("MOVE failed: {}", response.status()));
    }

    Ok(())
}

fn extract_host_from_url(url: &str) -> String {
    let stripped = url.trim_start_matches("http://").trim_start_matches("https://");
    let host = stripped.split('/').next().unwrap_or(stripped);
    let hostname = host.split(':').next().unwrap_or(host);
    if hostname.is_empty() {
        "Ferro".to_string()
    } else {
        hostname.to_string()
    }
}

fn parse_server_name_from_propfind(xml: &str, url: &str) -> String {
    let document = match roxmltree::Document::parse(xml) {
        Ok(doc) => doc,
        Err(_) => return extract_host_from_url(url),
    };

    let base_normalized = "/".trim_end_matches('/');

    for node in document.descendants() {
        if !node.is_element() || node.tag_name().name() != "response" {
            continue;
        }

        let href = node
            .children()
            .find(|n| n.is_element() && n.tag_name().name() == "href")
            .and_then(|n| n.text())
            .unwrap_or("");
        let href_normalized = href.trim_end_matches('/');

        if href_normalized != base_normalized {
            continue;
        }

        if let Some(prop) = node
            .children()
            .find(|n| n.is_element() && n.tag_name().name() == "propstat")
            .and_then(|ps| ps.children().find(|n| n.is_element() && n.tag_name().name() == "prop"))
        {
            for child in prop.children() {
                if child.is_element()
                    && child.tag_name().name() == "displayname"
                    && let Some(text) = child.text()
                    && !text.is_empty()
                {
                    return text.to_string();
                }
            }
        }
    }

    extract_host_from_url(url)
}

#[tauri::command]
pub async fn test_connection(url: String, token: String) -> Result<ConnectInfo, String> {
    if url.starts_with("http://") {
        let host_part = url.trim_start_matches("http://").split('/').next().unwrap_or("");
        let hostname = host_part.split(':').next().unwrap_or(host_part);
        if hostname != "localhost" && hostname != "127.0.0.1" && hostname != "::1" {
            return Err(
                "HTTPS recommended for non-localhost connections. Use https:// instead of http://.".to_string(),
            );
        }
    }

    let client = build_client(&token)?;
    let xml = do_propfind(&client, &url, "/", "1").await?;
    let entries = parse_propfind_response(&xml, "/");
    let server_name = parse_server_name_from_propfind(&xml, &url);
    Ok(ConnectInfo {
        server_name,
        root_files: entries.len() as u64,
    })
}

#[tauri::command]
async fn get_server_url(state: State<'_, DesktopState>) -> Result<String, String> {
    let config = state.config.read().await;
    Ok(config.server_url.clone())
}

#[tauri::command]
async fn cmd_mount(state: State<'_, DesktopState>) -> Result<String, String> {
    state.mount_drive().await
}

#[tauri::command]
async fn cmd_unmount(state: State<'_, DesktopState>) -> Result<String, String> {
    state.unmount_drive().await
}

#[tauri::command]
async fn cmd_get_mount_status(state: State<'_, DesktopState>) -> Result<MountStatusResponse, String> {
    Ok(state.get_mount_status().await)
}

#[tauri::command]
async fn cmd_get_config(state: State<'_, DesktopState>) -> Result<ConfigResponse, String> {
    Ok(state.get_config().await)
}

#[tauri::command]
async fn cmd_save_config(state: State<'_, DesktopState>, request: SaveConfigRequest) -> Result<(), String> {
    state.save_config(request).await
}

#[tauri::command]
async fn cmd_get_mount_progress(state: State<'_, DesktopState>) -> Result<MountProgress, String> {
    Ok(state.mount_service.progress().await)
}

#[tauri::command]
async fn cmd_open_path(path: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open: {}", e))?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open: {}", e))?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", &path])
            .spawn()
            .map_err(|e| format!("Failed to open: {}", e))?;
    }
    Ok(())
}

#[tauri::command]
async fn cmd_show_notification(title: String, body: String, app_handle: tauri::AppHandle) -> Result<(), String> {
    use tauri_plugin_notification::NotificationExt;
    app_handle
        .notification()
        .builder()
        .title(&title)
        .body(&body)
        .show()
        .map_err(|e| format!("Notification failed: {}", e))
}

#[tauri::command]
async fn cmd_default_mount_point() -> String {
    DesktopConfig::default_mount_point().display().to_string()
}

// ── Auto-Update Commands ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheckResult {
    pub update_available: bool,
    pub current_version: String,
    pub latest_version: String,
    pub download_url: String,
}

#[tauri::command]
async fn cmd_check_update(app_handle: tauri::AppHandle) -> Result<UpdateCheckResult, String> {
    use tauri_plugin_updater::UpdaterExt;

    let current_version = app_handle.package_info().version.to_string();

    let updater = app_handle
        .updater()
        .map_err(|e| format!("Updater unavailable: {}", e))?;

    match updater.check().await {
        Ok(Some(update)) => {
            let version = update.version.clone();
            let body = update.body.clone().unwrap_or_default();
            tracing::info!("Update available: {} (body: {})", version, body);
            Ok(UpdateCheckResult {
                update_available: true,
                current_version,
                latest_version: version,
                download_url: "https://github.com/WyattAu/ferro/releases/latest".to_string(),
            })
        }
        Ok(None) => {
            tracing::info!("No update available");
            Ok(UpdateCheckResult {
                update_available: false,
                current_version,
                latest_version: String::new(),
                download_url: String::new(),
            })
        }
        Err(e) => {
            tracing::warn!("Update check failed: {}", e);
            Err(format!("Update check failed: {}", e))
        }
    }
}

#[tauri::command]
async fn cmd_install_update(app_handle: tauri::AppHandle) -> Result<String, String> {
    use tauri_plugin_updater::UpdaterExt;

    let updater = app_handle
        .updater()
        .map_err(|e| format!("Updater unavailable: {}", e))?;

    match updater.check().await {
        Ok(Some(update)) => {
            let version = update.version.clone();
            tracing::info!("Installing update: {}", version);

            update
                .download_and_install(|_chunk_length, _content_length| {}, || {})
                .await
                .map_err(|e| format!("Install failed: {}", e))?;

            tracing::info!("Update installed, restarting app");
            app_handle.restart();
        }
        Ok(None) => {
            return Err("No update available".to_string());
        }
        Err(e) => {
            return Err(format!("Update check failed: {}", e));
        }
    }

    Ok("Update installed successfully".to_string())
}

// ── Windows Shell Integration Commands ──────────────────────────────

#[tauri::command]
async fn cmd_register_context_menu() -> Result<String, String> {
    ferro_desktop::shell_integration::register_context_menu()?;
    Ok("Context menu registered".to_string())
}

#[tauri::command]
async fn cmd_unregister_context_menu() -> Result<String, String> {
    ferro_desktop::shell_integration::unregister_context_menu()?;
    Ok("Context menu unregistered".to_string())
}

#[tauri::command]
async fn cmd_is_context_menu_registered() -> Result<bool, String> {
    Ok(ferro_desktop::shell_integration::is_registered())
}

#[tauri::command]
async fn cmd_register_autostart(exe_path: String) -> Result<String, String> {
    ferro_desktop::shell_integration::register_autostart(&exe_path)?;
    Ok("Autostart registered".to_string())
}

#[tauri::command]
async fn cmd_unregister_autostart() -> Result<String, String> {
    ferro_desktop::shell_integration::unregister_autostart()?;
    Ok("Autostart unregistered".to_string())
}

/// Save a screenshot of the webview to a PNG file.
/// Uses ImageMagick `import` as a cross-tool fallback for WebKitGTK
/// which lacks a stable screenshot API.
#[tauri::command]
fn take_screenshot(output_path: String) -> Result<String, String> {
    use std::process::Command;
    let out = Command::new("import")
        .args(["-window", "root", &output_path])
        .output()
        .map_err(|e| format!("failed to execute import: {e}"))?;
    if out.status.success() {
        Ok(format!("saved to {output_path}"))
    } else {
        Err(String::from_utf8_lossy(&out.stderr).to_string())
    }
}

#[tauri::command]
async fn cmd_update_tray_tooltip(
    app_handle: tauri::AppHandle,
    #[allow(unused)] state: State<'_, DesktopState>,
) -> Result<(), String> {
    #[allow(unused_mut)]
    let mut tooltip = "Ferro - File Storage".to_string();
    let mut update_available = false;

    #[cfg(all(feature = "sync", feature = "tauri"))]
    {
        let sync_status = state.get_sync_status().await;
        if sync_status.running {
            if sync_status.paused {
                tooltip = "Ferro - Sync Paused".to_string();
            } else if let Some(ref err) = sync_status.error {
                tooltip = format!("Ferro - Sync Error: {}", err);
            } else if let Some(ref summary) = sync_status.last_summary {
                tooltip = format!("Ferro - Synced: {} up, {} down", summary.uploaded, summary.downloaded);
            } else {
                tooltip = "Ferro - Syncing...".to_string();
            }
        }
    }

    // Updater disabled — plugin crashes when endpoint unreachable.
    // Update check re-enabled when release infrastructure is deployed.
    let _ = update_available; // suppress unused warning

    if let Some(tray) = app_handle.tray_by_id("main") {
        let _ = tray.set_tooltip(Some(&tooltip));
    }

    Ok(())
}

pub fn run(cli_args: CliArgs) -> Result<(), Box<dyn std::error::Error>> {
    let config = ferro_desktop::config::load_config_from_disk().unwrap_or_default();
    let state = DesktopState::new(config);

    // Build CLI connection info for the frontend.
    let cli_conn = CliConnection {
        server_url: cli_args.server_url.clone(),
        auth_token: cli_args.auth_token.clone(),
    };

    tracing::info!(?cli_conn, "CLI args for frontend");

    let audit_mode = cli_args.audit;

    // Patch index.html with the server URL BEFORE Tauri serves it.
    // This avoids the race condition where WASM loads before window.eval() runs.
    if let Some(ref url) = cli_args.server_url {
        let html_path = std::env::current_exe()?
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .join("frontend")
            .join("index.html");
        if let Ok(html) = std::fs::read_to_string(&html_path) {
            let patched = html.replace(
                "window.FERRO_SERVER_URL = window.FERRO_SERVER_URL || '';",
                &format!("window.FERRO_SERVER_URL = '{}';", url),
            );
            let _ = std::fs::write(&html_path, &patched);
            tracing::info!("Patched index.html with server URL: {}", url);
        }
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(state)
        .manage(cli_conn)
        .invoke_handler(tauri::generate_handler![
            dump_layout,
            #[cfg(feature = "screenshot")]
            capture_screenshot,
            get_cli_connection,
            get_server_url,
            cmd_mount,
            cmd_unmount,
            cmd_get_mount_status,
            cmd_get_config,
            cmd_save_config,
            cmd_get_mount_progress,
            cmd_open_path,
            cmd_show_notification,
            cmd_default_mount_point,
            cmd_register_context_menu,
            cmd_unregister_context_menu,
            cmd_is_context_menu_registered,
            cmd_register_autostart,
            cmd_unregister_autostart,
            cmd_update_tray_tooltip,
            cmd_check_update,
            cmd_install_update,
            list_directory,
            list_files_rest,
            get_file,
            put_file,
            create_directory,
            delete_item,
            move_item,
            test_connection,
            take_screenshot,
            #[cfg(feature = "sync")]
            ferro_desktop::tauri_commands::cmd_start_sync,
            #[cfg(feature = "sync")]
            ferro_desktop::tauri_commands::cmd_stop_sync,
            #[cfg(feature = "sync")]
            ferro_desktop::tauri_commands::cmd_pause_sync,
            #[cfg(feature = "sync")]
            ferro_desktop::tauri_commands::cmd_resume_sync,
            #[cfg(feature = "sync")]
            ferro_desktop::tauri_commands::cmd_sync_now,
            #[cfg(feature = "sync")]
            ferro_desktop::tauri_commands::cmd_get_sync_status,
            #[cfg(feature = "mobile")]
            mobile_commands::mobile_get_file_thumbnail,
            #[cfg(feature = "mobile")]
            mobile_commands::mobile_get_storage_stats,
            #[cfg(feature = "mobile")]
            mobile_commands::mobile_start_background_sync,
            #[cfg(feature = "mobile")]
            mobile_commands::mobile_stop_background_sync,
            #[cfg(feature = "mobile")]
            mobile_commands::mobile_get_offline_files,
            #[cfg(feature = "mobile")]
            mobile_commands::mobile_pin_file_offline,
            #[cfg(feature = "mobile")]
            mobile_commands::mobile_unpin_file_offline,
            #[cfg(feature = "mobile")]
            mobile_commands::mobile_get_sync_status,
            #[cfg(feature = "mobile")]
            mobile_commands::mobile_resolve_conflict,
            #[cfg(feature = "mobile")]
            mobile_commands::mobile_share_file,
            #[cfg(feature = "mobile")]
            mobile_commands::mobile_monitor_connectivity,
            #[cfg(feature = "mobile")]
            mobile_commands::mobile_register_push_notifications,
        ])
        .setup(move |app| {
            let audit_mode = audit_mode;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let show = MenuItem::with_id(app, "show", "Show Window", true, None::<&str>)?;
            let open_folder = MenuItem::with_id(app, "open_folder", "Open Folder", true, None::<&str>)?;
            let open_settings = MenuItem::with_id(app, "open_settings", "Settings", true, None::<&str>)?;
            let check_update = MenuItem::with_id(app, "check_update", "Check for Updates", true, None::<&str>)?;
            let separator = PredefinedMenuItem::separator(app)?;

            #[cfg(all(feature = "sync", feature = "tauri"))]
            let sync_now = MenuItem::with_id(app, "sync_now", "Sync Now", true, None::<&str>)?;
            #[cfg(all(feature = "sync", feature = "tauri"))]
            let pause_sync = MenuItem::with_id(app, "pause_sync", "Pause Sync", true, None::<&str>)?;
            #[cfg(all(feature = "sync", feature = "tauri"))]
            let resume_sync = MenuItem::with_id(app, "resume_sync", "Resume Sync", true, None::<&str>)?;

            #[cfg(all(feature = "android", feature = "tauri"))]
            let share_file = MenuItem::with_id(app, "share_file", "Share File", true, None::<&str>)?;
            #[cfg(all(feature = "android", feature = "tauri"))]
            let open_in_files = MenuItem::with_id(app, "open_in_files", "Open in Files", true, None::<&str>)?;

            #[cfg(all(feature = "sync", feature = "tauri", not(feature = "android")))]
            let menu = Menu::with_items(
                app,
                &[
                    &show,
                    &separator,
                    &sync_now,
                    &pause_sync,
                    &resume_sync,
                    &separator,
                    &open_folder,
                    &open_settings,
                    &check_update,
                    &separator,
                    &quit,
                ],
            )?;
            #[cfg(all(feature = "sync", feature = "tauri", feature = "android"))]
            let menu = Menu::with_items(
                app,
                &[
                    &show,
                    &separator,
                    &sync_now,
                    &pause_sync,
                    &resume_sync,
                    &separator,
                    &share_file,
                    &open_in_files,
                    &separator,
                    &open_folder,
                    &open_settings,
                    &check_update,
                    &separator,
                    &quit,
                ],
            )?;
            #[cfg(not(all(feature = "sync", feature = "tauri")))]
            let menu = Menu::with_items(
                app,
                &[
                    &show,
                    &separator,
                    &open_folder,
                    &open_settings,
                    &check_update,
                    &separator,
                    &quit,
                ],
            )?;

            let tray = TrayIconBuilder::new()
                .icon(
                    app.default_window_icon()
                        .cloned()
                        .ok_or("no default window icon configured")?,
                )
                .menu(&menu)
                .tooltip("Ferro - File Storage")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => {
                        app.exit(0);
                    }
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "open_folder" => {
                        let state = app.state::<DesktopState>();
                        let config = state.config.clone();
                        tauri::async_runtime::spawn(async move {
                            let config = config.read().await;
                            let mount_point = config.mount_point.display().to_string();
                            drop(config);
                            let _ = crate::gui::cmd_open_path(mount_point).await;
                        });
                    }
                    "open_settings" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                            let _ = window.emit("navigate-to-settings", ());
                        }
                    }
                    "check_update" => {
                        use tauri_plugin_notification::NotificationExt;
                        use tauri_plugin_updater::UpdaterExt;
                        let handle = app.clone();
                        tauri::async_runtime::spawn(async move {
                            let current_version = handle.package_info().version.to_string();
                            let updater = match handle.updater() {
                                Ok(u) => u,
                                Err(e) => {
                                    let _ = handle
                                        .notification()
                                        .builder()
                                        .title("Update Check Failed")
                                        .body(format!("Updater unavailable: {}", e))
                                        .show();
                                    return;
                                }
                            };
                            match updater.check().await {
                                Ok(Some(update)) => {
                                    let version = update.version.clone();
                                    let _ = handle
                                        .notification()
                                        .builder()
                                        .title("Update Available")
                                        .body(format!(
                                            "Version {} is available (current: {})",
                                            version, current_version
                                        ))
                                        .show();
                                    if let Some(window) = handle.get_webview_window("main") {
                                        let _ = window.emit("update-available", version);
                                    }
                                }
                                Ok(None) => {
                                    let _ = handle
                                        .notification()
                                        .builder()
                                        .title("No Updates")
                                        .body(format!("You are running the latest version ({})", current_version))
                                        .show();
                                }
                                Err(e) => {
                                    let _ = handle
                                        .notification()
                                        .builder()
                                        .title("Update Check Failed")
                                        .body(format!("{}", e))
                                        .show();
                                }
                            }
                        });
                    }
                    #[cfg(all(feature = "sync", feature = "tauri"))]
                    "sync_now" => {
                        use tauri::Manager;
                        let handle = app.clone();
                        tauri::async_runtime::spawn(async move {
                            let state = handle.state::<DesktopState>();
                            if let Err(e) = state.sync_now().await {
                                tracing::error!("manual sync failed: {}", e);
                            }
                            // Update tray tooltip after sync
                            let _ = cmd_update_tray_tooltip(handle.clone(), handle.state()).await;
                        });
                    }
                    #[cfg(all(feature = "sync", feature = "tauri"))]
                    "pause_sync" => {
                        let state = app.state::<DesktopState>();
                        state.pause_sync();
                        let handle = app.clone();
                        tauri::async_runtime::spawn(async move {
                            let _ = cmd_update_tray_tooltip(handle.clone(), handle.state()).await;
                        });
                    }
                    #[cfg(all(feature = "sync", feature = "tauri"))]
                    "resume_sync" => {
                        let state = app.state::<DesktopState>();
                        state.resume_sync();
                        let handle = app.clone();
                        tauri::async_runtime::spawn(async move {
                            let _ = cmd_update_tray_tooltip(handle.clone(), handle.state()).await;
                        });
                    }
                    #[cfg(all(feature = "android", feature = "tauri"))]
                    "share_file" => {
                        use tauri_plugin_notification::NotificationExt;
                        let _ = app
                            .notification()
                            .builder()
                            .title("Share")
                            .body("Share functionality coming soon")
                            .show();
                    }
                    #[cfg(all(feature = "android", feature = "tauri"))]
                    "open_in_files" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    _ => {}
                })
                .build(app)?;

            if let Some(window) = app.get_webview_window("main") {
                window.set_title("Ferro")?;
                let _ = window.show();
                let _ = window.set_focus();

                // Inject FERRO_SERVER_URL from CLI args — patch the HTML file directly
                // so the value is available before WASM module scripts execute.
                if let Some(conn) = app.try_state::<CliConnection>() {
                    if let Some(ref url) = conn.server_url {
                        // Patch the index.html file in-place
                        let html_path = std::env::current_exe()
                            .ok()
                            .and_then(|p| p.parent().map(|d| d.join("frontend/index.html")))
                            .or_else(|| {
                                // Fallback: look relative to manifest dir
                                Some(std::path::PathBuf::from("frontend/index.html"))
                            });
                        if let Some(path) = html_path {
                            if let Ok(html) = std::fs::read_to_string(&path) {
                                let patched = html.replace(
                                    "window.FERRO_SERVER_URL = window.FERRO_SERVER_URL || '';",
                                    &format!("window.FERRO_SERVER_URL = '{}';", url),
                                );
                                let _ = std::fs::write(&path, patched);
                                tracing::info!("Patched index.html with FERRO_SERVER_URL = {}", url);
                            }
                        }
                        // Also eval as fallback
                        let js = format!("window.FERRO_SERVER_URL = '{}';", url);
                        let _ = window.eval(&js);
                        tracing::info!("Injected FERRO_SERVER_URL = {}", url);
                    }
                }
            }

            // Auto-capture screenshot after page loads for debugging
            #[cfg(feature = "screenshot")]
            {
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(8)).await;
                    let path = std::env::temp_dir().join("ferro-screenshot.png");
                    match capture_screenshot(handle.clone(), path.to_str().unwrap().to_string()).await {
                        Ok(msg) => tracing::info!("Screenshot: {}", msg),
                        Err(e) => tracing::warn!("Screenshot failed: {}", e),
                    }
                });
            }

            // Auto-audit mode: navigate all pages, screenshot each, then exit
            #[cfg(feature = "screenshot")]
            if audit_mode {
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    use tauri::Manager;
                    tokio::time::sleep(std::time::Duration::from_secs(6)).await;

                    let pages = vec![
                        ("/", "01_home"),
                        ("/ui/notes", "02_notes"),
                        ("/ui/tasks", "03_tasks"),
                        ("/ui/calendar", "04_calendar"),
                        ("/ui/contacts", "05_contacts"),
                        ("/ui/chat", "06_chat"),
                        ("/ui/photos", "07_photos"),
                        ("/ui/trash", "08_trash"),
                        ("/ui/admin", "09_admin"),
                        ("/ui/settings", "10_settings"),
                    ];

                    let out_dir = std::env::temp_dir().join("ferro_audit");
                    let _ = std::fs::create_dir_all(&out_dir);

                    // Get window ID for X11 import
                    let wid = {
                        let output = std::process::Command::new("xdotool")
                            .args(["search", "--name", "Ferro"])
                            .output()
                            .ok();
                        output
                            .and_then(|o| String::from_utf8(o.stdout).ok())
                            .and_then(|s| s.lines().next().map(String::from))
                    };
                    tracing::info!("[audit] window ID: {:?}", wid);

                    for (route, name) in &pages {
                        tracing::info!("[audit] navigating to {} ({})", route, name);

                        // Navigate via JS eval
                        if let Some(window) = handle.get_webview_window("main") {
                            let js = if *route == "/" {
                                "window.location.href = '/ui/';".to_string()
                            } else {
                                format!("window.location.href = '{}';", route)
                            };
                            let _ = window.eval(&js);
                        }

                        // Wait for page to load and render
                        tokio::time::sleep(std::time::Duration::from_secs(3)).await;

                        // Capture screenshot using X11 import (works with WASM content)
                        let path = out_dir.join(format!("{}.png", name));
                        if let Some(ref wid) = wid {
                            let _ = std::process::Command::new("import")
                                .env("DISPLAY", ":0")
                                .args(["-window", wid.as_str(), path.to_str().unwrap()])
                                .output();
                        } else {
                            tracing::warn!("[audit] no window ID, using webkit snapshot fallback");
                            let _ = capture_screenshot(handle.clone(), path.to_str().unwrap().to_string()).await;
                        }

                        let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                        tracing::info!("[audit] {}: {} ({} bytes)", name, path.display(), size);
                    }

                    // Dark mode toggle
                    if let Some(window) = handle.get_webview_window("main") {
                        let _ = window.eval("document.querySelector('[aria-label=\"Toggle theme\"]')?.click();");
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    let path = out_dir.join("11_dark_mode.png");
                    if let Some(ref wid) = wid {
                        let _ = std::process::Command::new("import")
                            .env("DISPLAY", ":0")
                            .args(["-window", wid.as_str(), path.to_str().unwrap()])
                            .output();
                    }
                    let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                    tracing::info!("[audit] dark_mode: {} ({} bytes)", path.display(), size);

                    tracing::info!("[audit] COMPLETE — {} screenshots in {:?}", pages.len() + 1, out_dir);

                    // Exit app after audit
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    std::process::exit(0);
                });
            }
            #[cfg(not(feature = "screenshot"))]
            if audit_mode {
                eprintln!("--audit requires the 'screenshot' feature");
                std::process::exit(1);
            }

            // Layout diagnostic via Tauri IPC
            // Auto-start sync if configured
            #[cfg(all(feature = "sync", feature = "tauri"))]
            {
                use tauri::Manager;
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    let state = handle.state::<DesktopState>();
                    let config = state.config.read().await;
                    let should_start =
                        config.sync_interval_secs > 0 && !config.username.is_empty() && !config.password.is_empty();
                    drop(config);
                    if should_start {
                        // Give the app a moment to fully initialize
                        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                        if let Err(e) = state.start_sync().await {
                            tracing::warn!("auto-start sync failed: {}", e);
                        }
                    }
                });
            }

            let _ = tray;

            #[cfg(all(feature = "android", feature = "tauri"))]
            {
                android::register_notification_channels(app.handle());
                android::setup_file_provider(app.handle());
                tracing::info!("Android platform initialized");
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .map_err(|e| {
            tracing::error!("error while running tauri application: {e}");
            e
        })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_url_command() {
        use ferro_desktop::config::DesktopConfig;
        let url = DesktopConfig::default().server_url;
        assert!(url.starts_with("http"));
    }

    #[test]
    fn test_parse_propfind_simple() {
        let xml = r#"<?xml version="1.0"?>
<D:multistatus xmlns:D="DAV:">
  <D:response>
    <D:href>/file.txt</D:href>
    <D:propstat>
      <D:prop>
        <D:getcontentlength>1024</D:getcontentlength>
        <D:getlastmodified>Wed, 01 Jan 2024 00:00:00 GMT</D:getlastmodified>
      </D:prop>
      <D:status>HTTP/1.1 200 OK</D:status>
    </D:propstat>
  </D:response>
</D:multistatus>"#;
        let entries = parse_propfind_response(xml, "/");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "file.txt");
        assert_eq!(entries[0].size, 1024);
        assert!(!entries[0].is_dir);
        assert_eq!(entries[0].modified, "Wed, 01 Jan 2024 00:00:00 GMT");
    }

    #[test]
    fn test_parse_propfind_directory() {
        let xml = r#"<?xml version="1.0"?>
<D:multistatus xmlns:D="DAV:">
  <D:response>
    <D:href>/</D:href>
    <D:propstat>
      <D:prop>
        <D:resourcetype><D:collection/></D:resourcetype>
      </D:prop>
      <D:status>HTTP/1.1 200 OK</D:status>
    </D:propstat>
  </D:response>
  <D:response>
    <D:href>/docs/</D:href>
    <D:propstat>
      <D:prop>
        <D:resourcetype><D:collection/></D:resourcetype>
      </D:prop>
      <D:status>HTTP/1.1 200 OK</D:status>
    </D:propstat>
  </D:response>
  <D:response>
    <D:href>/readme.txt</D:href>
    <D:propstat>
      <D:prop>
        <D:getcontentlength>42</D:getcontentlength>
      </D:prop>
      <D:status>HTTP/1.1 200 OK</D:status>
    </D:propstat>
  </D:response>
</D:multistatus>"#;
        let entries = parse_propfind_response(xml, "/");
        assert_eq!(entries.len(), 2);
        assert!(entries[0].is_dir);
        assert_eq!(entries[0].name, "docs");
        assert!(!entries[1].is_dir);
        assert_eq!(entries[1].name, "readme.txt");
        assert_eq!(entries[1].size, 42);
    }

    #[test]
    fn test_parse_propfind_empty() {
        let xml = r#"<?xml version="1.0"?>
<D:multistatus xmlns:D="DAV:">
  <D:response>
    <D:href>/</D:href>
    <D:propstat>
      <D:prop>
        <D:resourcetype><D:collection/></D:resourcetype>
      </D:prop>
      <D:status>HTTP/1.1 200 OK</D:status>
    </D:propstat>
  </D:response>
</D:multistatus>"#;
        let entries = parse_propfind_response(xml, "/");
        assert!(entries.is_empty());
    }

    #[test]
    fn test_parse_propfind_with_etag() {
        let xml = r#"<?xml version="1.0"?>
<D:multistatus xmlns:D="DAV:">
  <D:response>
    <D:href>/notes.md</D:href>
    <D:propstat>
      <D:prop>
        <D:getcontentlength>256</D:getcontentlength>
        <D:getlastmodified>Thu, 02 Jan 2024 12:00:00 GMT</D:getlastmodified>
        <D:getetag>"abc123"</D:getetag>
      </D:prop>
      <D:status>HTTP/1.1 200 OK</D:status>
    </D:propstat>
  </D:response>
</D:multistatus>"#;
        let entries = parse_propfind_response(xml, "/");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].etag.as_deref(), Some("\"abc123\""));
        assert_eq!(entries[0].modified, "Thu, 02 Jan 2024 12:00:00 GMT");
    }

    #[test]
    fn test_parse_propfind_nested_path() {
        let xml = r#"<?xml version="1.0"?>
<D:multistatus xmlns:D="DAV:">
  <D:response>
    <D:href>/docs/</D:href>
    <D:propstat>
      <D:prop>
        <D:resourcetype><D:collection/></D:resourcetype>
      </D:prop>
      <D:status>HTTP/1.1 200 OK</D:status>
    </D:propstat>
  </D:response>
  <D:response>
    <D:href>/docs/file.txt</D:href>
    <D:propstat>
      <D:prop>
        <D:getcontentlength>100</D:getcontentlength>
      </D:prop>
      <D:status>HTTP/1.1 200 OK</D:status>
    </D:propstat>
  </D:response>
</D:multistatus>"#;
        let entries = parse_propfind_response(xml, "/docs");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "file.txt");
    }

    #[test]
    fn test_extract_host_from_url() {
        assert_eq!(extract_host_from_url("http://localhost:8080"), "localhost");
        assert_eq!(
            extract_host_from_url("https://my-server.example.com/path"),
            "my-server.example.com"
        );
        assert_eq!(extract_host_from_url("http://192.168.1.1:9090/"), "192.168.1.1");
    }

    #[test]
    fn test_parse_server_name_from_propfind_with_displayname() {
        let xml = r#"<?xml version="1.0"?>
<D:multistatus xmlns:D="DAV:">
  <D:response>
    <D:href>/</D:href>
    <D:propstat>
      <D:prop>
        <D:resourcetype><D:collection/></D:resourcetype>
        <D:displayname>My Ferro Server</D:displayname>
      </D:prop>
    </D:propstat>
  </D:response>
  <D:response>
    <D:href>/file.txt</D:href>
    <D:propstat>
      <D:prop>
        <D:getcontentlength>100</D:getcontentlength>
      </D:prop>
    </D:propstat>
  </D:response>
</D:multistatus>"#;
        let name = parse_server_name_from_propfind(xml, "http://example.com");
        assert_eq!(name, "My Ferro Server");
    }

    #[test]
    fn test_parse_server_name_from_propfind_fallback() {
        let xml = r#"<?xml version="1.0"?>
<D:multistatus xmlns:D="DAV:">
  <D:response>
    <D:href>/</D:href>
    <D:propstat>
      <D:prop>
        <D:resourcetype><D:collection/></D:resourcetype>
      </D:prop>
    </D:propstat>
  </D:response>
</D:multistatus>"#;
        let name = parse_server_name_from_propfind(xml, "http://myhost:8080");
        assert_eq!(name, "myhost");
    }

    #[test]
    fn test_parse_propfind_invalid_xml() {
        let xml = "not valid xml";
        let entries = parse_propfind_response(xml, "/");
        assert!(entries.is_empty());
    }
}
