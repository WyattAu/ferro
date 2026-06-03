use tauri::{
    Manager,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    State,
};

use ferro_desktop::commands::DesktopState;
use ferro_desktop::config::DesktopConfig;
use ferro_desktop::commands::{ConfigResponse, MountStatusResponse, SaveConfigRequest};
use ferro_desktop::rclone::MountProgress;

use serde::{Deserialize, Serialize};

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

        let is_dir = prop.map_or(false, |p| {
            p.children()
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

#[tauri::command]
pub async fn test_connection(url: String, token: String) -> Result<ConnectInfo, String> {
    let client = build_client(&token)?;
    let xml = do_propfind(&client, &url, "/", "1").await?;
    let entries = parse_propfind_response(&xml, "/");
    Ok(ConnectInfo {
        server_name: "Ferro".to_string(),
        root_files: entries.len() as u64,
    })
}

#[tauri::command]
fn get_server_url() -> String {
    "http://localhost:8080".to_string()
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
async fn cmd_get_mount_progress() -> Result<MountProgress, String> {
    Ok(MountProgress::default())
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
    DesktopConfig::default_mount_point()
        .display()
        .to_string()
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let config = DesktopConfig::default();
    let state = DesktopState::new(config);

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_notification::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
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
            list_directory,
            get_file,
            put_file,
            create_directory,
            delete_item,
            move_item,
            test_connection,
        ])
        .setup(|app| {
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let show = MenuItem::with_id(app, "show", "Show Window", true, None::<&str>)?;

            #[cfg(all(feature = "sync", feature = "tauri"))]
            let sync_now = MenuItem::with_id(app, "sync_now", "Sync Now", true, None::<&str>)?;
            #[cfg(all(feature = "sync", feature = "tauri"))]
            let pause_sync =
                MenuItem::with_id(app, "pause_sync", "Pause Sync", true, None::<&str>)?;
            #[cfg(all(feature = "sync", feature = "tauri"))]
            let resume_sync =
                MenuItem::with_id(app, "resume_sync", "Resume Sync", true, None::<&str>)?;

            #[cfg(all(feature = "sync", feature = "tauri"))]
            let menu =
                Menu::with_items(app, &[&show, &sync_now, &pause_sync, &resume_sync, &quit])?;
            #[cfg(not(all(feature = "sync", feature = "tauri")))]
            let menu = Menu::with_items(app, &[&show, &quit])?;

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
                    #[cfg(all(feature = "sync", feature = "tauri"))]
                    "sync_now" => {
                        let state = app.state::<DesktopState>();
                        tokio::spawn(async move {
                            if let Err(e) = state.sync_now().await {
                                tracing::error!("manual sync failed: {}", e);
                            }
                        });
                    }
                    #[cfg(all(feature = "sync", feature = "tauri"))]
                    "pause_sync" => {
                        let state = app.state::<DesktopState>();
                        state.pause_sync();
                    }
                    #[cfg(all(feature = "sync", feature = "tauri"))]
                    "resume_sync" => {
                        let state = app.state::<DesktopState>();
                        state.resume_sync();
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
                let state = app.state::<DesktopState>();
                let handle = app.handle().clone();
                tokio::spawn(async move {
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
                    let _ = handle;
                });
            }

            let _ = tray;
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
        let url = get_server_url();
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
    fn test_parse_propfind_invalid_xml() {
        let xml = "not valid xml";
        let entries = parse_propfind_response(xml, "/");
        assert!(entries.is_empty());
    }
}
