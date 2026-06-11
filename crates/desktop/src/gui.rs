use tauri::{
    Emitter, Manager, State,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
};

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
}

impl CliArgs {
    /// Parse CLI args using a minimal parser (avoid full clap dep in tauri mode).
    pub fn parse() -> Self {
        let raw: Vec<String> = std::env::args().skip(1).collect();
        let mut server_url = None;
        let mut auth_token = None;
        let mut debug = 0u8;

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
                "--help" | "-h" => {
                    println!("Usage: ferro-desktop [OPTIONS]");
                    println!();
                    println!("Options:");
                    println!("  -s, --server-url <URL>    Server URL (auto-connects, skips form)");
                    println!("  -t, --auth-token <TOKEN>  Auth token (Bearer or user:pass)");
                    println!(
                        "  -d, --debug              Enable debug logging to /tmp/ferro-desktop.log"
                    );
                    println!("  -dd                      Verbose debug logging");
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
    let value = reqwest::header::HeaderValue::from_str(&format!("Bearer {}", token))
        .map_err(|e| format!("Invalid token: {}", e))?;
    headers.insert(reqwest::header::AUTHORIZATION, value);
    reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))
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

        let href_normalized = href.trim_end_matches('/');

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

        let prop = node
            .children()
            .find(|n| n.is_element() && n.tag_name().name() == "propstat")
            .and_then(|ps| {
                ps.children()
                    .find(|n| n.is_element() && n.tag_name().name() == "prop")
            });

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

async fn do_propfind(
    client: &reqwest::Client,
    base_url: &str,
    path: &str,
    depth: &str,
) -> Result<String, String> {
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
pub async fn list_directory(
    url: String,
    token: String,
    path: String,
    depth: Option<String>,
) -> Result<String, String> {
    let client = build_client(&token)?;
    let depth_val = depth.as_deref().unwrap_or("1");
    let xml = do_propfind(&client, &url, &path, depth_val).await?;
    let entries = parse_propfind_response(&xml, &path);
    serde_json::to_string(&entries).map_err(|e| format!("Serialization failed: {}", e))
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
pub async fn put_file(
    url: String,
    token: String,
    path: String,
    data: Vec<u8>,
) -> Result<(), String> {
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
    let stripped = url
        .trim_start_matches("http://")
        .trim_start_matches("https://");
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
            .and_then(|ps| {
                ps.children()
                    .find(|n| n.is_element() && n.tag_name().name() == "prop")
            })
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
async fn cmd_get_mount_status(
    state: State<'_, DesktopState>,
) -> Result<MountStatusResponse, String> {
    Ok(state.get_mount_status().await)
}

#[tauri::command]
async fn cmd_get_config(state: State<'_, DesktopState>) -> Result<ConfigResponse, String> {
    Ok(state.get_config().await)
}

#[tauri::command]
async fn cmd_save_config(
    state: State<'_, DesktopState>,
    request: SaveConfigRequest,
) -> Result<(), String> {
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
async fn cmd_show_notification(
    title: String,
    body: String,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
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

    #[cfg(all(feature = "sync", feature = "tauri"))]
    {
        let sync_status = state.get_sync_status().await;
        if sync_status.running {
            if sync_status.paused {
                tooltip = "Ferro - Sync Paused".to_string();
            } else if let Some(ref err) = sync_status.error {
                tooltip = format!("Ferro - Sync Error: {}", err);
            } else if let Some(ref summary) = sync_status.last_summary {
                tooltip = format!(
                    "Ferro - Synced: {} up, {} down",
                    summary.uploaded, summary.downloaded
                );
            } else {
                tooltip = "Ferro - Syncing...".to_string();
            }
        }
    }

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

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(state)
        .manage(cli_conn)
        .invoke_handler(tauri::generate_handler![
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
            list_directory,
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
        .setup(|app| {
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let show = MenuItem::with_id(app, "show", "Show Window", true, None::<&str>)?;
            let open_folder =
                MenuItem::with_id(app, "open_folder", "Open Folder", true, None::<&str>)?;
            let open_settings =
                MenuItem::with_id(app, "open_settings", "Settings", true, None::<&str>)?;
            let separator = PredefinedMenuItem::separator(app)?;

            #[cfg(all(feature = "sync", feature = "tauri"))]
            let sync_now = MenuItem::with_id(app, "sync_now", "Sync Now", true, None::<&str>)?;
            #[cfg(all(feature = "sync", feature = "tauri"))]
            let pause_sync =
                MenuItem::with_id(app, "pause_sync", "Pause Sync", true, None::<&str>)?;
            #[cfg(all(feature = "sync", feature = "tauri"))]
            let resume_sync =
                MenuItem::with_id(app, "resume_sync", "Resume Sync", true, None::<&str>)?;

            #[cfg(all(feature = "android", feature = "tauri"))]
            let share_file =
                MenuItem::with_id(app, "share_file", "Share File", true, None::<&str>)?;
            #[cfg(all(feature = "android", feature = "tauri"))]
            let open_in_files =
                MenuItem::with_id(app, "open_in_files", "Open in Files", true, None::<&str>)?;

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
                        tokio::spawn(async move {
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
                    #[cfg(all(feature = "sync", feature = "tauri"))]
                    "sync_now" => {
                        use tauri::Manager;
                        let handle = app.clone();
                        tokio::spawn(async move {
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
                        tokio::spawn(async move {
                            let _ = cmd_update_tray_tooltip(handle.clone(), handle.state()).await;
                        });
                    }
                    #[cfg(all(feature = "sync", feature = "tauri"))]
                    "resume_sync" => {
                        let state = app.state::<DesktopState>();
                        state.resume_sync();
                        let handle = app.clone();
                        tokio::spawn(async move {
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
            }

            // Auto-start sync if configured
            #[cfg(all(feature = "sync", feature = "tauri"))]
            {
                use tauri::Manager;
                let handle = app.handle().clone();
                tokio::spawn(async move {
                    let state = handle.state::<DesktopState>();
                    let config = state.config.read().await;
                    let should_start = config.sync_interval_secs > 0
                        && !config.username.is_empty()
                        && !config.password.is_empty();
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
        assert_eq!(
            extract_host_from_url("http://192.168.1.1:9090/"),
            "192.168.1.1"
        );
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
