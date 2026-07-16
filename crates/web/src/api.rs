use serde::{Deserialize, Serialize};

use crate::auth::UserInfo;
use crate::utils::{percent_decode, percent_encode};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthCallbackResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub user: UserInfo,
    pub redirect: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthLoginResponse {
    pub authorization_url: String,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: String,
    pub name: String,
    pub size: u64,
    pub is_collection: bool,
    pub mime_type: String,
    pub modified_at: String,
    pub etag: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListingResponse {
    pub entries: Vec<FileEntry>,
    pub current_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    pub login_url: Option<String>,
    pub configured: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResultEntry {
    pub path: String,
    pub name: String,
    pub score: f64,
    pub snippet: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub query: String,
    pub results: Vec<SearchResultEntry>,
    pub total: usize,
    pub limit: usize,
    pub offset: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    pub theme: String,
    pub view_mode: String,
    pub sort_by: String,
    pub sort_order: String,
    pub items_per_page: usize,
    pub show_hidden_files: bool,
    pub language: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockInfo {
    pub path: String,
    pub token: String,
    pub owner: String,
    pub depth: String,
    pub created_at: String,
    pub expires_at: String,
}

#[allow(dead_code)] // Used by WASM runtime
fn urlencoding(s: &str) -> String {
    percent_encode(s)
}

fn decode_xml_entities(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}

#[allow(dead_code)] // Used by WASM runtime
fn parse_propfind_xml(xml: &str) -> Vec<FileEntry> {
    let mut entries = Vec::new();

    // Process XML tag by tag rather than line by line.
    // The server emits single-line XML (no newlines between tags), so
    // line-based parsing fails. We extract all tags and text content in
    // document order by splitting on '<' (each fragment then starts
    // with either a tag name or text content).
    let fragments: Vec<&str> = xml.split('<').collect();

    let mut in_response = false;
    let mut current_href = String::new();
    let mut current_props: std::collections::HashMap<String, String> = std::collections::HashMap::new();

    let mut in_prop = false;
    let mut current_prop_name = String::new();
    let mut in_propstat = false;
    let mut current_text = String::new();

    for fragment in &fragments {
        let trimmed = fragment.trim();

        if trimmed.is_empty() {
            continue;
        }

        // Closing tags first (they start with '/')
        if let Some(rest) = trimmed.strip_prefix("/D:") {
            let tag_name = rest.split('>').next().unwrap_or(rest).trim();
            match tag_name {
                "response" => {
                    if in_response && !current_href.is_empty() {
                        let name = current_href.rsplit('/').next().unwrap_or("").to_string();
                        let is_collection = current_props
                            .get("resourcetype")
                            .map(|v| v.contains("collection"))
                            .unwrap_or(false);

                        entries.push(FileEntry {
                            path: current_href.clone(),
                            name,
                            size: current_props
                                .get("getcontentlength")
                                .and_then(|v| v.parse().ok())
                                .unwrap_or(0),
                            is_collection,
                            mime_type: current_props.get("getcontenttype").cloned().unwrap_or_default(),
                            modified_at: current_props.get("getlastmodified").cloned().unwrap_or_default(),
                            etag: current_props.get("getetag").cloned().unwrap_or_default(),
                        });
                    }
                    in_response = false;
                }
                "propstat" => {
                    in_propstat = false;
                }
                "prop" => {
                    in_prop = false;
                }
                _ => {
                    // Closing tag for a property -- save it
                    if in_prop && !current_prop_name.is_empty() {
                        current_props.insert(current_prop_name.clone(), current_text.clone());
                    }
                    current_prop_name.clear();
                    current_text.clear();
                }
            }
        } else if let Some(rest) = trimmed.strip_prefix("D:") {
            let tag_end = rest.find('>').unwrap_or(rest.len());
            let tag_name = rest[..tag_end].trim();
            // Strip trailing '/' for self-closing tags like "collection/"
            let tag_name = tag_name.trim_end_matches('/');
            match tag_name {
                "response" => {
                    in_response = true;
                    current_href.clear();
                    current_props.clear();
                }
                "href" => {
                    // Content is between '>' and next '<'
                    if let Some(after_gt) = rest.find('>') {
                        let content = &rest[after_gt + 1..];
                        let raw = content.trim();
                        // Decode XML entities first (e.g. &amp; -> &),
                        // then percent-decode URL encoding (e.g. %26 -> &).
                        current_href = percent_decode(&decode_xml_entities(raw));
                    }
                }
                "propstat" => {
                    in_propstat = true;
                    in_prop = false;
                }
                "prop" => {
                    if in_propstat {
                        in_prop = true;
                    }
                }
                _ => {
                    // Property opening tag within <D:prop>
                    if in_prop {
                        // Check if this is a self-closing tag (trailing '/' before '>')
                        let is_self_closing = fragment.trim().ends_with("/>");
                        let after_gt = rest.find('>').map(|i| &rest[i + 1..]).unwrap_or("");
                        let content = if let Some(end) = after_gt.find("</") {
                            after_gt[..end].trim()
                        } else {
                            after_gt.trim()
                        };

                        if is_self_closing {
                            // Self-closing tag (e.g. <D:collection/>) -- append its name
                            // as a marker to the current property value
                            if current_text.is_empty() {
                                current_text = tag_name.to_string();
                            } else {
                                current_text.push_str(&format!(" {}", tag_name));
                            }
                        } else {
                            current_prop_name = tag_name.to_string();
                            current_text = content.to_string();
                        }
                    }
                }
            }
        }
    }

    entries.sort_by(|a, b| match (a.is_collection, b.is_collection) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });

    entries
}

#[allow(dead_code)] // Used by WASM runtime
fn js_err(msg: &str, e: &wasm_bindgen::JsValue) -> String {
    let detail = e.as_string().unwrap_or_else(|| format!("{:?}", e));
    format!("{}: {}", msg, detail)
}

#[cfg(target_arch = "wasm32")]
fn with_auth_headers(headers: &web_sys::Headers) {
    if let Some(auth) = crate::auth::get_auth_header() {
        let _ = headers.set("Authorization", &auth);
    }
}

#[cfg(target_arch = "wasm32")]
fn make_opts_with_auth(method: &str) -> web_sys::RequestInit {
    let headers = web_sys::Headers::new().expect("Headers::new must succeed in browser context");
    with_auth_headers(&headers);
    let opts = web_sys::RequestInit::new();
    opts.set_method(method);
    opts.set_headers(&headers);
    opts
}

#[allow(dead_code)] // Used by WASM runtime
/// Base URL for the Ferro server. Defaults to same-origin.
/// When running in Android WebView, set window.FERRO_SERVER_URL before WASM loads.
fn get_server_base() -> String {
    let window = web_sys::window().unwrap();

    // Check window.FERRO_SERVER_URL first (set by Android HTML)
    if let Ok(val) = js_sys::Reflect::get(&window, &wasm_bindgen::JsValue::from_str("FERRO_SERVER_URL"))
        && let Some(url) = val.as_string()
        && !url.is_empty()
    {
        return url.trim_end_matches('/').to_string();
    }

    // Then check localStorage connection
    if let Ok(Some(base)) = window.local_storage()
        && let Ok(Some(conn)) = base.get_item("ferro_connection")
        && let Ok(obj) = serde_json::from_str::<serde_json::Value>(&conn)
        && let Some(url) = obj.get("url").and_then(|v| v.as_str())
    {
        return url.trim_end_matches('/').to_string();
    }

    // Default to same-origin
    String::new()
}

/// Fetch a URL and return the response body as plain text.
///
/// All call sites live behind `cfg(target_arch = "wasm32")` (the WASM client build), so this
/// helper appears unused when type-checking the crate against a non-wasm host target. The
/// `allow(dead_code)` attribute silences that false positive without affecting the WASM build.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
async fn fetch_text(url: &str, opts: &web_sys::RequestInit) -> Result<String, String> {
    let window = web_sys::window().ok_or("No window")?;

    // If URL is relative and we have a server base URL, make it absolute
    let full_url = if url.starts_with('/') {
        let base = get_server_base();
        if base.is_empty() {
            url.to_string()
        } else {
            format!("{}{}", base, url)
        }
    } else {
        url.to_string()
    };

    let request =
        web_sys::Request::new_with_str_and_init(&full_url, opts).map_err(|e| js_err("Request creation failed", &e))?;

    let resp: web_sys::Response = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| js_err("Fetch failed", &e))?
        .into();

    if !resp.ok() {
        return Err(format!("HTTP error: {}", resp.status()));
    }

    wasm_bindgen_futures::JsFuture::from(resp.text().map_err(|e| js_err("text() failed", &e))?)
        .await
        .map_err(|e| js_err("Response read failed", &e))?
        .as_string()
        .ok_or_else(|| "Response text conversion failed".to_string())
}

#[cfg(target_arch = "wasm32")]
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
async fn post_json_text(url: &str, body: &str, opts: &web_sys::RequestInit) -> Result<String, String> {
    let window = web_sys::window().ok_or("No window")?;

    let full_url = if url.starts_with('/') {
        let base = get_server_base();
        if base.is_empty() {
            url.to_string()
        } else {
            format!("{}{}", base, url)
        }
    } else {
        url.to_string()
    };

    opts.set_method("POST");
    let headers = web_sys::Headers::new().map_err(|e| js_err("Headers creation failed", &e))?;
    with_auth_headers(&headers);
    headers.set("Content-Type", "application/json").map_err(|e| js_err("Headers set Content-Type", &e))?;
    opts.set_headers(&headers);
    opts.set_body(&wasm_bindgen::JsValue::from_str(body));

    let request =
        web_sys::Request::new_with_str_and_init(&full_url, opts).map_err(|e| js_err("Request creation failed", &e))?;

    let resp: web_sys::Response = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| js_err("Fetch failed", &e))?
        .into();

    if !resp.ok() {
        return Err(format!("HTTP error: {}", resp.status()));
    }

    wasm_bindgen_futures::JsFuture::from(resp.text().map_err(|e| js_err("text() failed", &e))?)
        .await
        .map_err(|e| js_err("Response read failed", &e))?
        .as_string()
        .ok_or_else(|| "Response text conversion failed".to_string())
}

#[cfg(target_arch = "wasm32")]
pub async fn list_files(path: &str) -> Result<ListingResponse, String> {
    let headers = web_sys::Headers::new().map_err(|e| js_err("Headers creation failed", &e))?;
    headers
        .set("Depth", "1")
        .map_err(|e| js_err("Headers set failed", &e))?;
    with_auth_headers(&headers);

    let opts = web_sys::RequestInit::new();
    opts.set_method("PROPFIND");
    opts.set_headers(&headers);

    web_sys::console::log_1(&format!("list_files: calling fetch for path={}", path).into());
    let text = fetch_text(path, &opts).await?;
    web_sys::console::log_1(&format!("list_files: got response, length={}", text.len()).into());
    let mut entries = parse_propfind_xml(&text);
    web_sys::console::log_1(&format!("list_files: parsed {} entries", entries.len()).into());
    // Filter out the self-referential directory entry (PROPFIND Depth:1
    // always includes the requested directory itself as the first response).
    let normalized = path.trim_end_matches('/');
    entries.retain(|e| e.path.trim_end_matches('/') != normalized);
    web_sys::console::log_1(&format!("list_files: after filter {} entries", entries.len()).into());

    Ok(ListingResponse {
        entries,
        current_path: path.to_string(),
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn list_files(path: &str) -> Result<ListingResponse, String> {
    Ok(ListingResponse {
        entries: vec![],
        current_path: path.to_string(),
    })
}

#[cfg(target_arch = "wasm32")]
pub async fn upload_file(path: &str, content: &[u8]) -> Result<(), String> {
    let window = web_sys::window().ok_or("No window")?;

    let array = js_sys::Uint8Array::new_with_length(content.len() as u32);
    array.copy_from(content);

    let opts = make_opts_with_auth("PUT");
    opts.set_body(&array.buffer());

    let request =
        web_sys::Request::new_with_str_and_init(path, &opts).map_err(|e| js_err("Request creation failed", &e))?;

    let _ = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| js_err("Fetch failed", &e))?;

    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn upload_file(_path: &str, _content: &[u8]) -> Result<(), String> {
    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub async fn delete_file(path: &str) -> Result<(), String> {
    let window = web_sys::window().ok_or("No window")?;

    let opts = make_opts_with_auth("DELETE");

    let request =
        web_sys::Request::new_with_str_and_init(path, &opts).map_err(|e| js_err("Request creation failed", &e))?;

    let resp: web_sys::Response = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| js_err("Fetch failed", &e))?
        .into();

    if !resp.ok() {
        return Err(format!("Delete failed: HTTP {}", resp.status()));
    }

    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn delete_file(_path: &str) -> Result<(), String> {
    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub async fn create_directory(path: &str) -> Result<(), String> {
    let window = web_sys::window().ok_or("No window")?;

    let opts = make_opts_with_auth("MKCOL");

    let request =
        web_sys::Request::new_with_str_and_init(path, &opts).map_err(|e| js_err("Request creation failed", &e))?;

    let _ = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| js_err("Fetch failed", &e))?;

    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn create_directory(_path: &str) -> Result<(), String> {
    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub async fn get_auth_config() -> Result<AuthResponse, String> {
    let opts = make_opts_with_auth("GET");

    let text = fetch_text("/api/config", &opts).await?;
    let config: serde_json::Value = serde_json::from_str(&text).unwrap_or_default();

    Ok(AuthResponse {
        login_url: config
            .get("auth_enabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
            .then(|| "/api/auth/login".to_string()),
        configured: config.get("auth_enabled").and_then(|v| v.as_bool()).unwrap_or(false),
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn get_auth_config() -> Result<AuthResponse, String> {
    Ok(AuthResponse {
        login_url: None,
        configured: false,
    })
}

#[cfg(target_arch = "wasm32")]
pub async fn search_files(query: &str, filters: Option<&SearchFilters>) -> Result<SearchResponse, String> {
    let mut url = format!("/api/search?q={}&limit=50", urlencoding(query));
    if let Some(f) = filters {
        if let Some(ref t) = f.r#type {
            url.push_str(&format!("&type={}", t));
        }
        if let Some(ref s) = f.sort {
            url.push_str(&format!("&sort={}", s));
        }
        if let Some(ref m) = f.mime_type {
            url.push_str(&format!("&mime_type={}", urlencoding(m)));
        }
    }

    let opts = make_opts_with_auth("GET");

    let text = fetch_text(&url, &opts).await?;
    let result: SearchResponse = serde_json::from_str(&text).map_err(|e| format!("JSON parse failed: {}", e))?;

    Ok(result)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn search_files(_query: &str, _filters: Option<&SearchFilters>) -> Result<SearchResponse, String> {
    Ok(SearchResponse {
        query: _query.to_string(),
        results: vec![],
        total: 0,
        limit: 50,
        offset: 0,
    })
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchFilters {
    pub r#type: Option<String>,
    pub sort: Option<String>,
    pub mime_type: Option<String>,
}

#[cfg(target_arch = "wasm32")]
pub async fn download_file(path: &str) -> Result<(), String> {
    use wasm_bindgen::JsCast;
    let window = web_sys::window().ok_or("No window")?;
    let document = window.document().ok_or("No document")?;

    let opts = make_opts_with_auth("GET");

    let request =
        web_sys::Request::new_with_str_and_init(path, &opts).map_err(|e| js_err("Request creation failed", &e))?;

    let resp: web_sys::Response = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| js_err("Fetch failed", &e))?
        .into();

    if !resp.ok() {
        return Err(format!("Download failed: {}", resp.status()));
    }

    let blob: web_sys::Blob =
        wasm_bindgen_futures::JsFuture::from(resp.blob().map_err(|e| js_err("blob() failed", &e))?)
            .await
            .map_err(|e| js_err("Blob creation failed", &e))?
            .into();

    let blob_url =
        web_sys::Url::create_object_url_with_blob(&blob).map_err(|e| js_err("Object URL creation failed", &e))?;

    let anchor: web_sys::HtmlAnchorElement = document
        .create_element("a")
        .map_err(|e| js_err("Element creation failed", &e))?
        .dyn_into()
        .map_err(|e| js_err("Cast failed", &e))?;

    let name = path.rsplit('/').next().unwrap_or("download");
    anchor.set_href(&blob_url);
    anchor.set_download(name);
    anchor.click();

    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn download_file(_path: &str) -> Result<(), String> {
    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub async fn fetch_json(url: &str) -> Result<serde_json::Value, String> {
    let opts = make_opts_with_auth("GET");

    let text = fetch_text(url, &opts).await?;
    serde_json::from_str(&text).map_err(|e| format!("JSON error: {}", e))
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn fetch_json(_url: &str) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({}))
}

#[cfg(target_arch = "wasm32")]
pub async fn auth_login() -> Result<AuthLoginResponse, String> {
    let url = "/api/auth/login?redirect=/ui/";
    let opts = make_opts_with_auth("GET");
    let text = fetch_text(url, &opts).await?;
    serde_json::from_str(&text).map_err(|e| format!("JSON parse failed: {}", e))
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn auth_login() -> Result<AuthLoginResponse, String> {
    Ok(AuthLoginResponse {
        authorization_url: String::new(),
        state: String::new(),
    })
}

#[cfg(target_arch = "wasm32")]
pub async fn auth_callback(code: &str, state: &str) -> Result<AuthCallbackResponse, String> {
    let url = format!(
        "/api/auth/callback?code={}&state={}",
        urlencoding(code),
        urlencoding(state)
    );
    let opts = make_opts_with_auth("GET");
    let text = fetch_text(&url, &opts).await?;
    serde_json::from_str(&text).map_err(|e| format!("JSON parse failed: {}", e))
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn auth_callback(_code: &str, _state: &str) -> Result<AuthCallbackResponse, String> {
    Ok(AuthCallbackResponse {
        access_token: String::new(),
        token_type: String::new(),
        expires_in: 0,
        user: UserInfo::default(),
        redirect: String::new(),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateShareResponse {
    pub token: String,
    pub url: String,
    pub path: String,
    pub expires_at: String,
}

#[cfg(target_arch = "wasm32")]
pub async fn create_share(
    path: &str,
    password: Option<&str>,
    expires_in_hours: Option<u32>,
) -> Result<CreateShareResponse, String> {
    let window = web_sys::window().ok_or("No window")?;

    let body = serde_json::json!({
        "path": path,
        "password": password,
        "expires_in_hours": expires_in_hours.unwrap_or(168),
    });

    let headers = web_sys::Headers::new().map_err(|e| js_err("Headers creation failed", &e))?;
    headers
        .set("Content-Type", "application/json")
        .map_err(|e| js_err("Headers set failed", &e))?;
    with_auth_headers(&headers);

    let opts = web_sys::RequestInit::new();
    opts.set_method("POST");
    opts.set_headers(&headers);
    opts.set_body(&wasm_bindgen::JsValue::from_str(&body.to_string()));

    let request = web_sys::Request::new_with_str_and_init("/api/shares", &opts)
        .map_err(|e| js_err("Request creation failed", &e))?;

    let resp: web_sys::Response = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| js_err("Fetch failed", &e))?
        .into();

    if !resp.ok() {
        return Err(format!("HTTP error: {}", resp.status()));
    }

    let text = wasm_bindgen_futures::JsFuture::from(resp.text().map_err(|e| js_err("text() failed", &e))?)
        .await
        .map_err(|e| js_err("Response read failed", &e))?
        .as_string()
        .ok_or_else(|| "Response text conversion failed".to_string())?;

    serde_json::from_str(&text).map_err(|e| format!("JSON parse failed: {}", e))
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn create_share(
    _path: &str,
    _password: Option<&str>,
    _expires_in_hours: Option<u32>,
) -> Result<CreateShareResponse, String> {
    Ok(CreateShareResponse {
        token: String::new(),
        url: String::new(),
        path: String::new(),
        expires_at: String::new(),
    })
}

#[cfg(target_arch = "wasm32")]
pub async fn get_file_content(path: &str) -> Result<String, String> {
    let opts = make_opts_with_auth("GET");
    let text = fetch_text(path, &opts).await?;
    if text.len() > 102_400 {
        Ok(text[..102_400].to_string())
    } else {
        Ok(text)
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn get_file_content(_path: &str) -> Result<String, String> {
    Ok(String::new())
}

#[cfg(target_arch = "wasm32")]
pub async fn list_favorites() -> Result<Vec<String>, String> {
    let opts = make_opts_with_auth("GET");
    let text = fetch_text("/api/favorites", &opts).await?;
    let val: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("JSON parse failed: {}", e))?;
    let paths = val
        .get("paths")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();
    Ok(paths)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn list_favorites() -> Result<Vec<String>, String> {
    Ok(vec![])
}

#[cfg(target_arch = "wasm32")]
pub async fn add_favorite(path: &str) -> Result<(), String> {
    let window = web_sys::window().ok_or("No window")?;
    let body = serde_json::json!({ "path": path });
    let headers = web_sys::Headers::new().map_err(|e| js_err("Headers creation failed", &e))?;
    headers
        .set("Content-Type", "application/json")
        .map_err(|e| js_err("Headers set failed", &e))?;
    with_auth_headers(&headers);
    let opts = web_sys::RequestInit::new();
    opts.set_method("PUT");
    opts.set_headers(&headers);
    opts.set_body(&wasm_bindgen::JsValue::from_str(&body.to_string()));
    let request = web_sys::Request::new_with_str_and_init("/api/favorites", &opts)
        .map_err(|e| js_err("Request creation failed", &e))?;
    let resp: web_sys::Response = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| js_err("Fetch failed", &e))?
        .into();
    if !resp.ok() {
        return Err(format!("HTTP error: {}", resp.status()));
    }
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn add_favorite(_path: &str) -> Result<(), String> {
    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub async fn remove_favorite(path: &str) -> Result<(), String> {
    let window = web_sys::window().ok_or("No window")?;
    let body = serde_json::json!({ "path": path });
    let headers = web_sys::Headers::new().map_err(|e| js_err("Headers creation failed", &e))?;
    headers
        .set("Content-Type", "application/json")
        .map_err(|e| js_err("Headers set failed", &e))?;
    with_auth_headers(&headers);
    let opts = web_sys::RequestInit::new();
    opts.set_method("DELETE");
    opts.set_headers(&headers);
    opts.set_body(&wasm_bindgen::JsValue::from_str(&body.to_string()));
    let request = web_sys::Request::new_with_str_and_init("/api/favorites", &opts)
        .map_err(|e| js_err("Request creation failed", &e))?;
    let resp: web_sys::Response = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| js_err("Fetch failed", &e))?
        .into();
    if !resp.ok() {
        return Err(format!("HTTP error: {}", resp.status()));
    }
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn remove_favorite(_path: &str) -> Result<(), String> {
    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub async fn list_recent_files() -> Result<Vec<FileEntry>, String> {
    let opts = make_opts_with_auth("GET");
    let text = fetch_text("/api/recent", &opts).await?;
    let val: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("JSON parse failed: {}", e))?;
    let files = val
        .get("files")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .map(|v| FileEntry {
                    path: v.get("path").and_then(|p| p.as_str()).unwrap_or("").to_string(),
                    name: v
                        .get("path")
                        .and_then(|p| p.as_str())
                        .unwrap_or("")
                        .rsplit('/')
                        .next()
                        .unwrap_or("")
                        .to_string(),
                    size: 0,
                    is_collection: false,
                    mime_type: String::new(),
                    modified_at: v.get("timestamp").and_then(|t| t.as_str()).unwrap_or("").to_string(),
                    etag: String::new(),
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(files)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn list_recent_files() -> Result<Vec<FileEntry>, String> {
    Ok(vec![])
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrashedEntry {
    pub original_path: String,
    pub deleted_at: String,
    pub size: u64,
    pub mime_type: String,
}

#[cfg(target_arch = "wasm32")]
pub async fn list_trash() -> Result<Vec<TrashedEntry>, String> {
    let opts = make_opts_with_auth("GET");
    let text = fetch_text("/api/trash", &opts).await?;
    let val: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("JSON parse failed: {}", e))?;
    let entries = val
        .get("entries")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| {
                    Some(TrashedEntry {
                        original_path: v
                            .get("original_path")
                            .and_then(|p| p.as_str())
                            .unwrap_or("")
                            .to_string(),
                        deleted_at: v.get("deleted_at").and_then(|d| d.as_str()).unwrap_or("").to_string(),
                        size: v.get("size").and_then(|s| s.as_u64()).unwrap_or(0),
                        mime_type: v.get("mime_type").and_then(|m| m.as_str()).unwrap_or("").to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(entries)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn list_trash() -> Result<Vec<TrashedEntry>, String> {
    Ok(vec![])
}

#[cfg(target_arch = "wasm32")]
pub async fn restore_trash(path: &str) -> Result<(), String> {
    let window = web_sys::window().ok_or("No window")?;
    let body = serde_json::json!({ "original_path": path });
    let headers = web_sys::Headers::new().map_err(|e| js_err("Headers creation failed", &e))?;
    headers
        .set("Content-Type", "application/json")
        .map_err(|e| js_err("Headers set failed", &e))?;
    with_auth_headers(&headers);
    let opts = web_sys::RequestInit::new();
    opts.set_method("POST");
    opts.set_headers(&headers);
    opts.set_body(&wasm_bindgen::JsValue::from_str(&body.to_string()));
    let request = web_sys::Request::new_with_str_and_init("/api/trash/restore", &opts)
        .map_err(|e| js_err("Request creation failed", &e))?;
    let resp: web_sys::Response = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| js_err("Fetch failed", &e))?
        .into();
    if !resp.ok() {
        return Err(format!("HTTP error: {}", resp.status()));
    }
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn restore_trash(_path: &str) -> Result<(), String> {
    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub async fn purge_trash(path: &str) -> Result<(), String> {
    let window = web_sys::window().ok_or("No window")?;
    let body = serde_json::json!({ "original_path": path });
    let headers = web_sys::Headers::new().map_err(|e| js_err("Headers creation failed", &e))?;
    headers
        .set("Content-Type", "application/json")
        .map_err(|e| js_err("Headers set failed", &e))?;
    with_auth_headers(&headers);
    let opts = web_sys::RequestInit::new();
    opts.set_method("DELETE");
    opts.set_headers(&headers);
    opts.set_body(&wasm_bindgen::JsValue::from_str(&body.to_string()));
    let request = web_sys::Request::new_with_str_and_init("/api/trash/purge", &opts)
        .map_err(|e| js_err("Request creation failed", &e))?;
    let resp: web_sys::Response = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| js_err("Fetch failed", &e))?
        .into();
    if !resp.ok() {
        return Err(format!("HTTP error: {}", resp.status()));
    }
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn purge_trash(_path: &str) -> Result<(), String> {
    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub async fn empty_trash() -> Result<(), String> {
    let window = web_sys::window().ok_or("No window")?;
    let opts = make_opts_with_auth("DELETE");
    let request = web_sys::Request::new_with_str_and_init("/api/trash/empty", &opts)
        .map_err(|e| js_err("Request creation failed", &e))?;
    let resp: web_sys::Response = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| js_err("Fetch failed", &e))?
        .into();
    if !resp.ok() {
        return Err(format!("HTTP error: {}", resp.status()));
    }
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn empty_trash() -> Result<(), String> {
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkDeleteResponse {
    pub succeeded: Vec<String>,
    pub failed: Vec<serde_json::Value>,
    pub total_requested: usize,
}

#[cfg(target_arch = "wasm32")]
pub async fn bulk_delete(paths: &[String]) -> Result<BulkDeleteResponse, String> {
    let window = web_sys::window().ok_or("No window")?;
    let body = serde_json::json!({ "paths": paths });
    let headers = web_sys::Headers::new().map_err(|e| js_err("Headers creation failed", &e))?;
    headers
        .set("Content-Type", "application/json")
        .map_err(|e| js_err("Headers set failed", &e))?;
    with_auth_headers(&headers);
    let opts = web_sys::RequestInit::new();
    opts.set_method("POST");
    opts.set_headers(&headers);
    opts.set_body(&wasm_bindgen::JsValue::from_str(&body.to_string()));
    let request = web_sys::Request::new_with_str_and_init("/api/bulk/delete", &opts)
        .map_err(|e| js_err("Request creation failed", &e))?;
    let resp: web_sys::Response = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| js_err("Fetch failed", &e))?
        .into();
    if !resp.ok() {
        return Err(format!("HTTP error: {}", resp.status()));
    }
    let text = wasm_bindgen_futures::JsFuture::from(resp.text().map_err(|e| js_err("text() failed", &e))?)
        .await
        .map_err(|e| js_err("Response read failed", &e))?
        .as_string()
        .ok_or_else(|| "Response text conversion failed".to_string())?;
    serde_json::from_str(&text).map_err(|e| format!("JSON parse failed: {}", e))
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn bulk_delete(_paths: &[String]) -> Result<BulkDeleteResponse, String> {
    Ok(BulkDeleteResponse {
        succeeded: vec![],
        failed: vec![],
        total_requested: 0,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaInfo {
    pub used_bytes: u64,
    pub quota_bytes: u64,
    pub used_percent: f64,
    pub file_count: u64,
    pub unlimited: bool,
}

#[cfg(target_arch = "wasm32")]
pub async fn get_quota() -> Result<QuotaInfo, String> {
    let opts = make_opts_with_auth("GET");
    let text = fetch_text("/api/quota", &opts).await?;
    serde_json::from_str(&text).map_err(|e| format!("JSON parse failed: {}", e))
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn get_quota() -> Result<QuotaInfo, String> {
    Ok(QuotaInfo {
        used_bytes: 0,
        quota_bytes: 0,
        used_percent: 0.0,
        file_count: 0,
        unlimited: true,
    })
}

#[cfg(target_arch = "wasm32")]
pub async fn move_file(source: &str, destination: &str) -> Result<(), String> {
    let window = web_sys::window().ok_or("No window")?;
    let body = serde_json::json!({
        "source": source,
        "destination": destination,
    });
    let headers = web_sys::Headers::new().map_err(|e| js_err("Headers creation failed", &e))?;
    headers
        .set("Content-Type", "application/json")
        .map_err(|e| js_err("Headers set failed", &e))?;
    with_auth_headers(&headers);
    let opts = web_sys::RequestInit::new();
    opts.set_method("POST");
    opts.set_headers(&headers);
    opts.set_body(&wasm_bindgen::JsValue::from_str(&body.to_string()));
    let request = web_sys::Request::new_with_str_and_init("/api/files/move", &opts)
        .map_err(|e| js_err("Request creation failed", &e))?;
    let resp: web_sys::Response = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| js_err("Fetch failed", &e))?
        .into();
    if !resp.ok() {
        return Err(format!("HTTP error: {}", resp.status()));
    }
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn move_file(_source: &str, _destination: &str) -> Result<(), String> {
    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub async fn copy_file(source: &str, destination: &str) -> Result<(), String> {
    let window = web_sys::window().ok_or("No window")?;
    let body = serde_json::json!({
        "source": source,
        "destination": destination,
    });
    let headers = web_sys::Headers::new().map_err(|e| js_err("Headers creation failed", &e))?;
    headers
        .set("Content-Type", "application/json")
        .map_err(|e| js_err("Headers set failed", &e))?;
    with_auth_headers(&headers);
    let opts = web_sys::RequestInit::new();
    opts.set_method("POST");
    opts.set_headers(&headers);
    opts.set_body(&wasm_bindgen::JsValue::from_str(&body.to_string()));
    let request = web_sys::Request::new_with_str_and_init("/api/files/copy", &opts)
        .map_err(|e| js_err("Request creation failed", &e))?;
    let resp: web_sys::Response = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| js_err("Fetch failed", &e))?
        .into();
    if !resp.ok() {
        return Err(format!("HTTP error: {}", resp.status()));
    }
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn copy_file(_source: &str, _destination: &str) -> Result<(), String> {
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityEntry {
    pub action: String,
    pub path: String,
    pub size: Option<u64>,
    pub timestamp: String,
    pub user: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityResponse {
    pub entries: Vec<ActivityEntry>,
    pub total: usize,
}

#[cfg(target_arch = "wasm32")]
pub async fn get_activity(limit: u32, offset: u32) -> Result<ActivityResponse, String> {
    let url = format!("/api/activity?limit={}&offset={}", limit, offset);
    let opts = make_opts_with_auth("GET");
    let text = fetch_text(&url, &opts).await?;
    serde_json::from_str(&text).map_err(|e| format!("JSON parse failed: {}", e))
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn get_activity(_limit: u32, _offset: u32) -> Result<ActivityResponse, String> {
    Ok(ActivityResponse {
        entries: vec![],
        total: 0,
    })
}

#[cfg(target_arch = "wasm32")]
pub async fn get_preferences() -> Result<UserPreferences, String> {
    let opts = make_opts_with_auth("GET");
    let text = fetch_text("/api/preferences", &opts).await?;
    serde_json::from_str(&text).map_err(|e| format!("JSON parse failed: {}", e))
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn get_preferences() -> Result<UserPreferences, String> {
    Ok(UserPreferences {
        theme: "dark".to_string(),
        view_mode: "list".to_string(),
        sort_by: "name".to_string(),
        sort_order: "asc".to_string(),
        items_per_page: 50,
        show_hidden_files: false,
        language: "en".to_string(),
    })
}

#[cfg(target_arch = "wasm32")]
pub async fn update_preferences(prefs: &UserPreferences) -> Result<UserPreferences, String> {
    let window = web_sys::window().ok_or("No window")?;
    let body = serde_json::to_string(prefs).map_err(|e| format!("Serialize failed: {}", e))?;
    let headers = web_sys::Headers::new().map_err(|e| js_err("Headers creation failed", &e))?;
    headers
        .set("Content-Type", "application/json")
        .map_err(|e| js_err("Headers set failed", &e))?;
    with_auth_headers(&headers);
    let opts = web_sys::RequestInit::new();
    opts.set_method("PUT");
    opts.set_headers(&headers);
    opts.set_body(&wasm_bindgen::JsValue::from_str(&body));
    let request = web_sys::Request::new_with_str_and_init("/api/preferences", &opts)
        .map_err(|e| js_err("Request creation failed", &e))?;
    let resp: web_sys::Response = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| js_err("Fetch failed", &e))?
        .into();
    if !resp.ok() {
        return Err(format!("HTTP error: {}", resp.status()));
    }
    let text = wasm_bindgen_futures::JsFuture::from(resp.text().map_err(|e| js_err("text() failed", &e))?)
        .await
        .map_err(|e| js_err("Response read failed", &e))?
        .as_string()
        .ok_or_else(|| "Response text conversion failed".to_string())?;
    serde_json::from_str(&text).map_err(|e| format!("JSON parse failed: {}", e))
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn update_preferences(prefs: &UserPreferences) -> Result<UserPreferences, String> {
    Ok(prefs.clone())
}

#[cfg(target_arch = "wasm32")]
pub async fn list_locks() -> Result<Vec<LockInfo>, String> {
    let opts = make_opts_with_auth("GET");
    let text = fetch_text("/api/locks", &opts).await?;
    let val: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("JSON parse failed: {}", e))?;
    let locks = val
        .get("locks")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| {
                    Some(LockInfo {
                        path: v.get("path").and_then(|p| p.as_str()).unwrap_or("").to_string(),
                        token: v.get("token").and_then(|t| t.as_str()).unwrap_or("").to_string(),
                        owner: v.get("owner").and_then(|o| o.as_str()).unwrap_or("").to_string(),
                        depth: v.get("depth").and_then(|d| d.as_str()).unwrap_or("").to_string(),
                        created_at: v.get("created_at").and_then(|c| c.as_str()).unwrap_or("").to_string(),
                        expires_at: v.get("expires_at").and_then(|e| e.as_str()).unwrap_or("").to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(locks)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn list_locks() -> Result<Vec<LockInfo>, String> {
    Ok(vec![])
}

#[cfg(target_arch = "wasm32")]
pub async fn force_unlock(path: &str) -> Result<(), String> {
    let window = web_sys::window().ok_or("No window")?;
    let body = serde_json::json!({ "path": path });
    let headers = web_sys::Headers::new().map_err(|e| js_err("Headers creation failed", &e))?;
    headers
        .set("Content-Type", "application/json")
        .map_err(|e| js_err("Headers set failed", &e))?;
    with_auth_headers(&headers);
    let opts = web_sys::RequestInit::new();
    opts.set_method("POST");
    opts.set_headers(&headers);
    opts.set_body(&wasm_bindgen::JsValue::from_str(&body.to_string()));
    let request = web_sys::Request::new_with_str_and_init("/api/locks/force-unlock", &opts)
        .map_err(|e| js_err("Request creation failed", &e))?;
    let resp: web_sys::Response = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| js_err("Fetch failed", &e))?
        .into();
    if !resp.ok() {
        return Err(format!("HTTP error: {}", resp.status()));
    }
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn force_unlock(_path: &str) -> Result<(), String> {
    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub fn request_notification_permission() {
    let _ = js_sys::eval("Notification.requestPermission()");
}

#[cfg(not(target_arch = "wasm32"))]
pub fn request_notification_permission() {}

#[cfg(target_arch = "wasm32")]
pub fn show_notification(title: &str, body: &str) {
    // Sanitize: strip any characters that could break out of JS string literals
    let sanitize = |s: &str| -> String {
        s.chars()
            .map(|c| match c {
                '\\' => "\\\\".to_string(),
                '\'' => "\\'".to_string(),
                '"' => "\\\"".to_string(),
                '\n' => "\\n".to_string(),
                '\r' => "\\r".to_string(),
                c if (c as u32) < 0x20 => format!("\\u{:04x}", c as u32),
                c => c.to_string(),
            })
            .collect()
    };
    let safe_title = sanitize(title);
    let safe_body = sanitize(body);
    let _ = js_sys::eval(&format!(
        "if (typeof Notification !== 'undefined' && Notification.permission === 'granted') {{ new Notification('{safe_title}', {{ body: '{safe_body}' }}); }}"
    ));
}

#[cfg(not(target_arch = "wasm32"))]
pub fn show_notification(_title: &str, _body: &str) {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrandingConfig {
    pub logo_url: Option<String>,
    pub primary_color: String,
    pub title: String,
    pub favicon_url: Option<String>,
    pub custom_css: Option<String>,
}

impl Default for BrandingConfig {
    fn default() -> Self {
        Self {
            logo_url: None,
            primary_color: "#3b82f6".to_string(),
            title: "Ferro".to_string(),
            favicon_url: None,
            custom_css: None,
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub async fn fetch_branding() -> Result<BrandingConfig, String> {
    let opts = make_opts_with_auth("GET");
    let text = fetch_text("/api/branding", &opts).await?;
    serde_json::from_str(&text).map_err(|e| format!("JSON parse failed: {}", e))
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn fetch_branding() -> Result<BrandingConfig, String> {
    Ok(BrandingConfig::default())
}

// ---------------------------------------------------------------------------
// Version History API
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileVersion {
    pub id: u64,
    pub path: String,
    pub size: u64,
    pub content_hash: String,
    pub modified_at: String,
    pub author: String,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionsResponse {
    pub versions: Vec<FileVersion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffLine {
    #[serde(rename = "type")]
    pub type_: String,
    pub content: String,
    pub old_line: Option<usize>,
    pub new_line: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffStats {
    pub additions: usize,
    pub deletions: usize,
    pub unchanged: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffResponse {
    pub from_version: String,
    pub to_version: String,
    pub is_binary: bool,
    pub lines: Vec<DiffLine>,
    pub stats: DiffStats,
}

#[cfg(target_arch = "wasm32")]
pub async fn list_versions(path: &str) -> Result<VersionsResponse, String> {
    let url = format!("/api/files/{}/versions", urlencoding(path.trim_start_matches('/')));
    let opts = make_opts_with_auth("GET");
    let text = fetch_text(&url, &opts).await?;
    serde_json::from_str(&text).map_err(|e| format!("JSON parse failed: {}", e))
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn list_versions(_path: &str) -> Result<VersionsResponse, String> {
    Ok(VersionsResponse { versions: vec![] })
}

#[cfg(target_arch = "wasm32")]
pub async fn get_version_content(path: &str, version_id: u64) -> Result<Vec<u8>, String> {
    let url = format!(
        "/api/files/{}/versions/{}",
        urlencoding(path.trim_start_matches('/')),
        version_id
    );
    let window = web_sys::window().ok_or("No window")?;
    let opts = make_opts_with_auth("GET");
    let request =
        web_sys::Request::new_with_str_and_init(&url, &opts).map_err(|e| js_err("Request creation failed", &e))?;
    let resp: web_sys::Response = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| js_err("Fetch failed", &e))?
        .into();
    if !resp.ok() {
        return Err(format!("HTTP error: {}", resp.status()));
    }
    let array_buffer =
        wasm_bindgen_futures::JsFuture::from(resp.array_buffer().map_err(|e| js_err("array_buffer() failed", &e))?)
            .await
            .map_err(|e| js_err("ArrayBuffer read failed", &e))?;
    let uint8 = js_sys::Uint8Array::new(&array_buffer);
    Ok(uint8.to_vec())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn get_version_content(_path: &str, _version_id: u64) -> Result<Vec<u8>, String> {
    Ok(vec![])
}

#[cfg(target_arch = "wasm32")]
pub async fn restore_version(path: &str, version_id: u64) -> Result<(), String> {
    let content = get_version_content(path, version_id).await?;
    upload_file(path, &content).await
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn restore_version(_path: &str, _version_id: u64) -> Result<(), String> {
    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub async fn diff_versions(path: &str, from: u64, to: u64) -> Result<DiffResponse, String> {
    let url = format!(
        "/api/files/{}/diff?from={}&to={}",
        urlencoding(path.trim_start_matches('/')),
        from,
        to
    );
    let opts = make_opts_with_auth("GET");
    let text = fetch_text(&url, &opts).await?;
    serde_json::from_str(&text).map_err(|e| format!("JSON parse failed: {}", e))
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn diff_versions(_path: &str, _from: u64, _to: u64) -> Result<DiffResponse, String> {
    Ok(DiffResponse {
        from_version: String::new(),
        to_version: String::new(),
        is_binary: false,
        lines: vec![],
        stats: DiffStats {
            additions: 0,
            deletions: 0,
            unchanged: 0,
        },
    })
}

// ---------------------------------------------------------------------------
// Transcode API
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscodeRequest {
    pub source_path: String,
    pub target_format: String,
    pub quality: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscodeResponse {
    pub id: String,
    pub status: String,
    pub output_path: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscodeJob {
    pub id: String,
    pub source_path: String,
    pub target_format: String,
    pub quality: String,
    pub output_path: String,
    pub status: String,
    pub progress: f64,
    pub created_at: String,
    pub completed_at: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscodeJobsResponse {
    pub jobs: Vec<TranscodeJob>,
    pub total: usize,
}

#[cfg(target_arch = "wasm32")]
pub async fn start_transcode(req: &TranscodeRequest) -> Result<TranscodeResponse, String> {
    let body = serde_json::to_string(req).map_err(|e| format!("Serialize failed: {}", e))?;
    let opts = make_opts_with_auth("POST");
    let text = post_json_text("/api/transcode", &body, &opts).await?;
    serde_json::from_str(&text).map_err(|e| format!("JSON parse failed: {}", e))
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn start_transcode(_req: &TranscodeRequest) -> Result<TranscodeResponse, String> {
    Ok(TranscodeResponse {
        id: String::new(),
        status: "pending".to_string(),
        output_path: String::new(),
        message: "Not available in native mode".to_string(),
    })
}

#[cfg(target_arch = "wasm32")]
pub async fn get_transcode_status(id: &str) -> Result<TranscodeJob, String> {
    let url = format!("/api/transcode/{}/status", id);
    let opts = make_opts_with_auth("GET");
    let text = fetch_text(&url, &opts).await?;
    serde_json::from_str(&text).map_err(|e| format!("JSON parse failed: {}", e))
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn get_transcode_status(_id: &str) -> Result<TranscodeJob, String> {
    Ok(TranscodeJob {
        id: String::new(),
        source_path: String::new(),
        target_format: String::new(),
        quality: String::new(),
        output_path: String::new(),
        status: "unknown".to_string(),
        progress: 0.0,
        created_at: String::new(),
        completed_at: None,
        error: Some("Not available in native mode".to_string()),
    })
}

#[cfg(target_arch = "wasm32")]
pub async fn list_transcode_jobs() -> Result<TranscodeJobsResponse, String> {
    let opts = make_opts_with_auth("GET");
    let text = fetch_text("/api/transcode", &opts).await?;
    serde_json::from_str(&text).map_err(|e| format!("JSON parse failed: {}", e))
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn list_transcode_jobs() -> Result<TranscodeJobsResponse, String> {
    Ok(TranscodeJobsResponse {
        jobs: vec![],
        total: 0,
    })
}

// ---------------------------------------------------------------------------
// Smart Collections API
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartCollection {
    pub id: String,
    pub name: String,
    pub rules: Vec<serde_json::Value>,
    pub auto_update: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartCollectionsResponse {
    pub collections: Vec<SmartCollection>,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSmartCollectionRequest {
    pub name: String,
    pub rules: Vec<serde_json::Value>,
    pub auto_update: bool,
}

#[cfg(target_arch = "wasm32")]
pub async fn list_smart_collections() -> Result<SmartCollectionsResponse, String> {
    let opts = make_opts_with_auth("GET");
    let text = fetch_text("/api/smart-collections", &opts).await?;
    serde_json::from_str(&text).map_err(|e| format!("JSON parse failed: {}", e))
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn list_smart_collections() -> Result<SmartCollectionsResponse, String> {
    Ok(SmartCollectionsResponse {
        collections: vec![],
        total: 0,
    })
}

#[cfg(target_arch = "wasm32")]
pub async fn create_smart_collection(req: &CreateSmartCollectionRequest) -> Result<SmartCollection, String> {
    let body = serde_json::to_string(req).map_err(|e| format!("Serialize failed: {}", e))?;
    let opts = make_opts_with_auth("POST");
    let text = post_json_text("/api/smart-collections", &body, &opts).await?;
    let resp: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("JSON parse failed: {}", e))?;
    serde_json::from_value(resp["collection"].clone())
        .map_err(|e| format!("JSON parse failed: {}", e))
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn create_smart_collection(_req: &CreateSmartCollectionRequest) -> Result<SmartCollection, String> {
    Ok(SmartCollection {
        id: String::new(),
        name: String::new(),
        rules: vec![],
        auto_update: false,
        created_at: String::new(),
        updated_at: String::new(),
    })
}

#[cfg(target_arch = "wasm32")]
pub async fn delete_smart_collection(id: &str) -> Result<(), String> {
    let url = format!("/api/smart-collections/{}", id);
    let opts = make_opts_with_auth("DELETE");
    let window = web_sys::window().ok_or("No window")?;
    let full_url = if url.starts_with('/') {
        let base = get_server_base();
        if base.is_empty() {
            url.clone()
        } else {
            format!("{}{}", base, url)
        }
    } else {
        url.clone()
    };
    let req = web_sys::Request::new_with_str_and_init(&full_url, &opts)
        .map_err(|e| js_err("Request creation failed", &e))?;
    let resp: web_sys::Response = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&req))
        .await
        .map_err(|e| js_err("Fetch failed", &e))?
        .into();
    if resp.ok() {
        Ok(())
    } else {
        Err(format!("HTTP error: {}", resp.status()))
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn delete_smart_collection(_id: &str) -> Result<(), String> {
    Ok(())
}

// ---------------------------------------------------------------------------
// Dashboard API
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardRecentFile {
    pub path: String,
    pub modified_at: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardSharedFile {
    pub token: String,
    pub path: String,
    pub expires_at: String,
    pub download_count: u32,
    pub created_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardResponse {
    pub storage_used: u64,
    pub storage_total: u64,
    pub file_count: u64,
    pub recent_files: Vec<DashboardRecentFile>,
    pub shared_files: Vec<DashboardSharedFile>,
    pub activity: Vec<ActivityEntry>,
}

#[cfg(target_arch = "wasm32")]
pub async fn get_dashboard() -> Result<DashboardResponse, String> {
    let opts = make_opts_with_auth("GET");
    let text = fetch_text("/api/dashboard", &opts).await?;
    serde_json::from_str(&text).map_err(|e| format!("JSON parse failed: {}", e))
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn get_dashboard() -> Result<DashboardResponse, String> {
    Ok(DashboardResponse {
        storage_used: 0,
        storage_total: 0,
        file_count: 0,
        recent_files: vec![],
        shared_files: vec![],
        activity: vec![],
    })
}

// ---------------------------------------------------------------------------
// Share management API
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareListItem {
    pub token: String,
    pub url: String,
    pub path: String,
    pub expires_at: String,
    pub max_downloads: Option<u32>,
    pub download_count: u32,
    pub created_by: String,
    pub allow_download: Option<bool>,
    pub allow_upload: Option<bool>,
}

#[cfg(target_arch = "wasm32")]
pub async fn list_shares() -> Result<Vec<ShareListItem>, String> {
    let opts = make_opts_with_auth("GET");
    let text = fetch_text("/api/shares", &opts).await?;
    let val: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("JSON parse failed: {}", e))?;
    let items = val
        .get("shares")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| {
                    Some(ShareListItem {
                        token: v.get("token").and_then(|t| t.as_str()).unwrap_or("").to_string(),
                        url: v.get("url").and_then(|u| u.as_str()).unwrap_or("").to_string(),
                        path: v.get("path").and_then(|p| p.as_str()).unwrap_or("").to_string(),
                        expires_at: v.get("expires_at").and_then(|e| e.as_str()).unwrap_or("").to_string(),
                        max_downloads: v.get("max_downloads").and_then(|d| d.as_u64()).map(|d| d as u32),
                        download_count: v.get("download_count").and_then(|d| d.as_u64()).unwrap_or(0) as u32,
                        created_by: v.get("created_by").and_then(|u| u.as_str()).unwrap_or("").to_string(),
                        allow_download: v.get("allow_download").and_then(|d| d.as_bool()),
                        allow_upload: v.get("allow_upload").and_then(|u| u.as_bool()),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(items)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn list_shares() -> Result<Vec<ShareListItem>, String> {
    Ok(vec![])
}

#[cfg(target_arch = "wasm32")]
pub async fn delete_share(token: &str) -> Result<(), String> {
    let window = web_sys::window().ok_or("No window")?;
    let opts = make_opts_with_auth("DELETE");
    let url = format!("/api/shares/{}", urlencoding(token));
    let request =
        web_sys::Request::new_with_str_and_init(&url, &opts).map_err(|e| js_err("Request creation failed", &e))?;
    let resp: web_sys::Response = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| js_err("Fetch failed", &e))?
        .into();
    if !resp.ok() {
        return Err(format!("Delete share failed: HTTP {}", resp.status()));
    }
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn delete_share(_token: &str) -> Result<(), String> {
    Ok(())
}

// ---------------------------------------------------------------------------
// Update Check API
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub update_available: bool,
    pub download_url: String,
}

#[cfg(target_arch = "wasm32")]
pub async fn check_for_updates() -> Result<UpdateInfo, String> {
    let current_version = env!("CARGO_PKG_VERSION").to_string();

    let url = "https://api.github.com/repos/WyattAu/ferro/releases/latest";
    let window = web_sys::window().ok_or("No window")?;
    let headers = web_sys::Headers::new().map_err(|e| js_err("Headers creation failed", &e))?;
    headers
        .set("Accept", "application/vnd.github.v3+json")
        .map_err(|e| js_err("Headers set failed", &e))?;

    let opts = web_sys::RequestInit::new();
    opts.set_method("GET");
    opts.set_headers(&headers);

    let request =
        web_sys::Request::new_with_str_and_init(url, &opts).map_err(|e| js_err("Request creation failed", &e))?;

    let resp: web_sys::Response = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| js_err("Fetch failed", &e))?
        .into();

    if !resp.ok() {
        return Err(format!("HTTP error: {}", resp.status()));
    }

    let text = wasm_bindgen_futures::JsFuture::from(resp.text().map_err(|e| js_err("text() failed", &e))?)
        .await
        .map_err(|e| js_err("Response read failed", &e))?
        .as_string()
        .ok_or_else(|| "Response text conversion failed".to_string())?;

    let val: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("JSON parse failed: {}", e))?;

    let latest_version = val
        .get("tag_name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim_start_matches('v')
        .to_string();

    let download_url = val
        .get("html_url")
        .and_then(|v| v.as_str())
        .unwrap_or("https://github.com/WyattAu/ferro/releases/latest")
        .to_string();

    let update_available = if latest_version.is_empty() {
        false
    } else {
        latest_version != current_version
    };

    Ok(UpdateInfo {
        current_version,
        latest_version,
        update_available,
        download_url,
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn check_for_updates() -> Result<UpdateInfo, String> {
    Ok(UpdateInfo {
        current_version: env!("CARGO_PKG_VERSION").to_string(),
        latest_version: String::new(),
        update_available: false,
        download_url: String::new(),
    })
}

// ---------------------------------------------------------------------------
// Calendar & Contacts REST API helpers
// ---------------------------------------------------------------------------

#[cfg(target_arch = "wasm32")]
pub async fn fetch_json_with_method(url: &str, method: &str, body: Option<&str>) -> Result<serde_json::Value, String> {
    let window = web_sys::window().ok_or("No window")?;
    let headers = web_sys::Headers::new().map_err(|e| js_err("Headers creation failed", &e))?;
    headers
        .set("Content-Type", "application/json")
        .map_err(|e| js_err("Headers set failed", &e))?;
    with_auth_headers(&headers);

    let opts = web_sys::RequestInit::new();
    opts.set_method(method);
    opts.set_headers(&headers);

    if let Some(body_str) = body {
        opts.set_body(&wasm_bindgen::JsValue::from_str(body_str));
    }

    let request =
        web_sys::Request::new_with_str_and_init(url, &opts).map_err(|e| js_err("Request creation failed", &e))?;

    let resp: web_sys::Response = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| js_err("Fetch failed", &e))?
        .into();

    if !resp.ok() {
        return Err(format!("HTTP error: {}", resp.status()));
    }

    let text = wasm_bindgen_futures::JsFuture::from(resp.text().map_err(|e| js_err("text() failed", &e))?)
        .await
        .map_err(|e| js_err("Response read failed", &e))?
        .as_string()
        .ok_or_else(|| "Response text conversion failed".to_string())?;

    if text.is_empty() {
        Ok(serde_json::json!({}))
    } else {
        serde_json::from_str(&text).map_err(|e| format!("JSON parse failed: {}", e))
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn fetch_json_with_method(
    _url: &str,
    _method: &str,
    _body: Option<&str>,
) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_urlencoding_safe_chars() {
        assert_eq!(urlencoding("abcABC123-_.~"), "abcABC123-_.~");
    }

    #[test]
    fn test_urlencoding_special_chars() {
        let encoded = urlencoding("hello world");
        assert_eq!(encoded, "hello%20world");

        let encoded = urlencoding("/");
        assert_eq!(encoded, "%2F");

        let encoded = urlencoding("a+b=c");
        assert!(encoded.contains("%2B"));
        assert!(encoded.contains("%3D"));
    }

    #[test]
    fn test_urlencoding_empty() {
        assert_eq!(urlencoding(""), "");
    }

    #[test]
    fn test_parse_propfind_xml_empty() {
        let entries = parse_propfind_xml("");
        assert!(entries.is_empty());
    }

    #[test]
    fn test_parse_propfind_xml_extracts_href_only() {
        let xml = "<D:multistatus xmlns:D=\"DAV:\">\n\
  <D:response>\n\
    <D:href>/files/test.txt</D:href>\n\
    <D:propstat>\n\
      <D:prop>\n\
      </D:prop>\n\
    </D:propstat>\n\
  </D:response>\n\
</D:multistatus>\n";
        let entries = parse_propfind_xml(xml);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "test.txt");
        assert_eq!(entries[0].path, "/files/test.txt");
        assert_eq!(entries[0].size, 0);
    }

    #[test]
    fn test_parse_propfind_xml_empty_multistatus() {
        let xml = "<D:multistatus xmlns:D=\"DAV:\">\n\
</D:multistatus>\n";
        let entries = parse_propfind_xml(xml);
        assert!(entries.is_empty());
    }

    #[test]
    fn test_parse_propfind_xml_missing_props() {
        let xml = r#"
<D:multistatus xmlns:D="DAV:">
  <D:response>
    <D:href>/files/test.txt</D:href>
    <D:propstat>
      <D:prop>
      </D:prop>
    </D:propstat>
  </D:response>
</D:multistatus>
"#;
        let entries = parse_propfind_xml(xml);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].size, 0);
        assert_eq!(entries[0].mime_type, "");
        assert_eq!(entries[0].etag, "");
    }

    #[test]
    fn test_parse_propfind_xml_single_line() {
        // The server emits single-line XML; the parser must handle it.
        let xml = "<?xml version=\"1.0\" encoding=\"utf-8\"?><D:multistatus xmlns:D=\"DAV:\"><D:response><D:href>/</D:href><D:propstat><D:prop><D:getlastmodified>Tue, 19 May 2026 04:47:47 GMT</D:getlastmodified><D:getcontentlength>0</D:getcontentlength><D:getetag>\"col-1\"</D:getetag><D:getcontenttype>httpd/unix-directory</D:getcontenttype><D:resourcetype><D:collection/></D:resourcetype></D:prop><D:status>HTTP/1.1 200 OK</D:status></D:propstat></D:response><D:response><D:href>/test.txt</D:href><D:propstat><D:prop><D:getlastmodified>Tue, 19 May 2026 04:47:47 GMT</D:getlastmodified><D:getcontentlength>5</D:getcontentlength><D:getetag>\"abc\"</D:getetag><D:getcontenttype>application/octet-stream</D:getcontenttype><D:resourcetype></D:resourcetype></D:prop><D:status>HTTP/1.1 200 OK</D:status></D:propstat></D:response></D:multistatus>";
        let entries = parse_propfind_xml(xml);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].path, "/");
        assert!(entries[0].is_collection);
        assert_eq!(entries[1].path, "/test.txt");
        assert_eq!(entries[1].name, "test.txt");
        assert_eq!(entries[1].size, 5);
        assert!(!entries[1].is_collection);
    }

    #[test]
    fn test_search_filters_default() {
        let filters = SearchFilters::default();
        assert!(filters.r#type.is_none());
        assert!(filters.sort.is_none());
        assert!(filters.mime_type.is_none());
    }

    #[test]
    fn test_file_entry_serde_roundtrip() {
        let entry = FileEntry {
            path: "/files/test.txt".to_string(),
            name: "test.txt".to_string(),
            size: 2048,
            is_collection: false,
            mime_type: "text/plain".to_string(),
            modified_at: "2025-01-01T00:00:00Z".to_string(),
            etag: "\"etag\"".to_string(),
        };
        let json = serde_json::to_string(&entry).unwrap();
        let parsed: FileEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.path, entry.path);
        assert_eq!(parsed.size, entry.size);
    }

    #[test]
    fn test_listing_response_serde() {
        let resp = ListingResponse {
            entries: vec![],
            current_path: "/".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: ListingResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.current_path, "/");
    }

    #[test]
    fn test_user_preferences_serde() {
        let prefs = UserPreferences {
            theme: "dark".to_string(),
            view_mode: "grid".to_string(),
            sort_by: "name".to_string(),
            sort_order: "asc".to_string(),
            items_per_page: 100,
            show_hidden_files: true,
            language: "en".to_string(),
        };
        let json = serde_json::to_string(&prefs).unwrap();
        let parsed: UserPreferences = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.theme, "dark");
        assert_eq!(parsed.items_per_page, 100);
        assert!(parsed.show_hidden_files);
    }

    #[test]
    fn test_quota_info_serde() {
        let quota = QuotaInfo {
            used_bytes: 1024,
            quota_bytes: 4096,
            used_percent: 25.0,
            file_count: 10,
            unlimited: false,
        };
        let json = serde_json::to_string(&quota).unwrap();
        let parsed: QuotaInfo = serde_json::from_str(&json).unwrap();
        assert!(!parsed.unlimited);
        assert_eq!(parsed.used_bytes, 1024);
    }

    #[test]
    fn test_search_response_serde() {
        let resp = SearchResponse {
            query: "test".to_string(),
            results: vec![SearchResultEntry {
                path: "/files/test.txt".to_string(),
                name: "test.txt".to_string(),
                score: 1.0,
                snippet: Some("...test...".to_string()),
            }],
            total: 1,
            limit: 50,
            offset: 0,
        };
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: SearchResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.total, 1);
        assert_eq!(parsed.results[0].score, 1.0);
    }
}
