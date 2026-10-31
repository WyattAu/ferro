use leptos::prelude::*;
#[cfg(target_arch = "wasm32")]
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserInfo {
    pub sub: String,
    pub email: Option<String>,
    pub name: Option<String>,
}

#[allow(dead_code)] // Used by WASM runtime
const STORAGE_KEY: &str = "ferro_access_token";
#[allow(dead_code)] // Used by WASM runtime
const REFRESH_KEY: &str = "ferro_refresh_token";
#[allow(dead_code)] // Used by WASM runtime
const OIDC_LOGOUT_KEY: &str = "ferro_oidc_logout_url";
#[allow(dead_code)] // Used by WASM runtime
const BASIC_KEY: &str = "ferro_basic_auth";

#[derive(Clone)]
#[allow(dead_code)] // Used by WASM runtime
pub struct AuthState {
    access_token: ReadSignal<Option<String>>,
    set_access_token: WriteSignal<Option<String>>,
    user: ReadSignal<Option<UserInfo>>,
    set_user: WriteSignal<Option<UserInfo>>,
    auth_enabled: ReadSignal<bool>,
    set_auth_enabled: WriteSignal<bool>,
    loading: ReadSignal<bool>,
    set_loading: WriteSignal<bool>,
}

impl AuthState {
    #[allow(clippy::too_many_arguments)]
    fn new(
        access_token: ReadSignal<Option<String>>,
        set_access_token: WriteSignal<Option<String>>,
        user: ReadSignal<Option<UserInfo>>,
        set_user: WriteSignal<Option<UserInfo>>,
        auth_enabled: ReadSignal<bool>,
        set_auth_enabled: WriteSignal<bool>,
        loading: ReadSignal<bool>,
        set_loading: WriteSignal<bool>,
    ) -> Self {
        Self {
            access_token,
            set_access_token,
            user,
            set_user,
            auth_enabled,
            set_auth_enabled,
            loading,
            set_loading,
        }
    }

    pub fn access_token(&self) -> ReadSignal<Option<String>> {
        self.access_token
    }

    pub fn user(&self) -> ReadSignal<Option<UserInfo>> {
        self.user
    }

    pub fn auth_enabled(&self) -> ReadSignal<bool> {
        self.auth_enabled
    }

    pub fn loading(&self) -> ReadSignal<bool> {
        self.loading
    }

    pub fn is_authenticated(&self) -> bool {
        self.access_token.get_untracked().is_some()
    }

    pub fn get_access_token(&self) -> Option<String> {
        self.access_token.get_untracked()
    }
}

pub fn provide_auth_state() -> AuthState {
    let (access_token, set_access_token) = signal(None);
    let (user, set_user) = signal(None);
    let (auth_enabled, set_auth_enabled) = signal(false);
    let (loading, set_loading) = signal(true);

    let state = AuthState::new(
        access_token,
        set_access_token,
        user,
        set_user,
        auth_enabled,
        set_auth_enabled,
        loading,
        set_loading,
    );

    provide_context(state.clone());
    state
}

pub fn use_auth_state() -> AuthState {
    use_context::<AuthState>().expect("AuthState not provided")
}

#[cfg(target_arch = "wasm32")]
#[allow(dead_code)] // Used by WASM runtime
fn get_local_storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}

#[cfg(target_arch = "wasm32")]
fn read_stored_token() -> Option<String> {
    get_local_storage()?.get_item(STORAGE_KEY).ok()?
}

#[cfg(target_arch = "wasm32")]
fn store_token(token: &str) {
    if let Some(storage) = get_local_storage() {
        let _ = storage.set_item(STORAGE_KEY, token);
    }
}

#[cfg(target_arch = "wasm32")]
fn clear_stored_token() {
    if let Some(storage) = get_local_storage() {
        let _ = storage.remove_item(STORAGE_KEY);
        let _ = storage.remove_item(REFRESH_KEY);
    }
}

#[cfg(target_arch = "wasm32")]
fn store_refresh_token(token: &str) {
    if let Some(storage) = get_local_storage() {
        let _ = storage.set_item(REFRESH_KEY, token);
    }
}

#[cfg(target_arch = "wasm32")]
fn read_stored_refresh_token() -> Option<String> {
    get_local_storage()?.get_item(REFRESH_KEY).ok()?
}

#[cfg(target_arch = "wasm32")]
fn store_oidc_logout_url(url: &str) {
    if let Some(storage) = get_local_storage() {
        let _ = storage.set_item(OIDC_LOGOUT_KEY, url);
    }
}

#[cfg(target_arch = "wasm32")]
fn read_oidc_logout_url() -> Option<String> {
    get_local_storage()?.get_item(OIDC_LOGOUT_KEY).ok()?
}

#[cfg(target_arch = "wasm32")]
pub fn get_auth_header() -> Option<String> {
    if let Some(token) = read_stored_token() {
        return Some(format!("Bearer {}", token));
    }
    // Simple/basic auth mode: the whole "Basic dXNlcjpwYXNz" header value is
    // stored, so no credential revelation can happen here.
    read_stored_basic()
}

#[cfg(target_arch = "wasm32")]
fn read_stored_basic() -> Option<String> {
    get_local_storage()?.get_item(BASIC_KEY).ok()?
}

#[cfg(target_arch = "wasm32")]
pub fn store_basic_auth(header: &str) {
    if let Some(storage) = get_local_storage() {
        let _ = storage.set_item(BASIC_KEY, header);
    }
}

#[cfg(target_arch = "wasm32")]
pub fn clear_stored_basic() {
    if let Some(storage) = get_local_storage() {
        let _ = storage.remove_item(BASIC_KEY);
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn store_basic_auth(_header: &str) {}

#[cfg(not(target_arch = "wasm32"))]
pub fn clear_stored_basic() {}

/// Build a Basic authorization header per RFC 7617: `user:pass` in UTF-8,
/// base64-encoded. Username and password must not contain a colon; the
/// username is percent-rejected up front so header injection is impossible.
pub fn basic_auth_header(username: &str, password: &str) -> Result<String, String> {
    if username.contains(':') {
        return Err("Username must not contain ':".to_string());
    }
    if username.is_empty() && password.is_empty() {
        return Err("Username and password are required".to_string());
    }
    let creds = format!("{username}:{password}");
    Ok(format!("Basic {}", base64(creds.as_bytes())))
}

/// Minimal RFC 4648 standard base64 (with padding) — the web crate has no
/// base64 dependency and btoa() via js_sys requires a live document.
fn base64(input: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(ALPHABET[(n >> 18) as usize & 63] as char);
        out.push(ALPHABET[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            ALPHABET[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            ALPHABET[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

/// Mode of the login page's auth flow, shared between the config fetch and
/// the rendered view. Defaults to "oidc" (the historical behavior) until the
/// server config resolves. Kept in a thread_local so it survives view
/// rebuilds; the web runtime is single-threaded.
pub fn login_page_mode_signal() -> (ReadSignal<String>, WriteSignal<String>) {
    thread_local! {
        static MODE: std::cell::RefCell<Option<(ReadSignal<String>, WriteSignal<String>)>> =
            const { std::cell::RefCell::new(None) };
    }
    MODE.with_borrow_mut(|m| {
        if m.is_none() {
            *m = Some(signal("oidc".to_string()));
        }
        *m.as_ref().unwrap()
    })
}

#[cfg(target_arch = "wasm32")]
pub fn set_login_page_mode(mode: &str) {
    let mode = if mode.is_empty() { "oidc" } else { mode };
    let (_, write) = login_page_mode_signal();
    write.set(mode.to_string());
}

#[cfg(not(target_arch = "wasm32"))]
pub fn set_login_page_mode(_mode: &str) {}

/// Decode a `Basic <b64>` header back to the username (display purposes).
pub fn basic_auth_username(header: &str) -> Option<String> {
    let encoded = header.strip_prefix("Basic ")?;
    let bytes = base64_decode(encoded)?;
    let creds = String::from_utf8(bytes).ok()?;
    creds.split_once(':').map(|(u, _)| u.to_string())
}

/// Minimal RFC 4648 standard base64 decoder (padding required).
fn base64_decode(input: &str) -> Option<Vec<u8>> {
    fn value(c: u8) -> Option<u32> {
        match c {
            b'A'..=b'Z' => Some((c - b'A') as u32),
            b'a'..=b'z' => Some((c - b'a') as u32 + 26),
            b'0'..=b'9' => Some((c - b'0') as u32 + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }
    let input = input.trim_end_matches('=');
    let mut out = Vec::with_capacity(input.len() * 3 / 4);
    for chunk in input.as_bytes().chunks(4) {
        let mut n: u32 = 0;
        for (i, &c) in chunk.iter().enumerate() {
            n |= value(c)? << (18 - 6 * i);
        }
        out.push((n >> 16) as u8);
        if chunk.len() > 2 {
            out.push((n >> 8) as u8);
        }
        if chunk.len() > 3 {
            out.push(n as u8);
        }
    }
    Some(out)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn get_auth_header() -> Option<String> {
    None
}

/// Proactively refresh the access token before it expires.
/// Call this periodically (e.g., every 5 minutes) from init_auth.
#[cfg(target_arch = "wasm32")]
pub fn spawn_token_refresh(state: &AuthState) {
    let state = state.clone();
    leptos::task::spawn_local(async move {
        loop {
            // Wait 5 minutes between refresh attempts
            let promise = js_sys::Promise::new(&mut |resolve, _reject| {
                web_sys::window()
                    .unwrap()
                    .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 300_000);
            });
            let _ = wasm_bindgen_futures::JsFuture::from(promise).await;

            // Check if we have a refresh token
            let refresh_token = match read_stored_refresh_token() {
                Some(rt) => rt,
                None => continue,
            };

            // Try to refresh
            match crate::api::auth_refresh_token(&refresh_token).await {
                Ok(data) => {
                    if let Some(new_token) = data.get("access_token").and_then(|v| v.as_str()) {
                        store_token(new_token);
                        state.set_access_token.set(Some(new_token.to_string()));
                        web_sys::console::log_1(&"Token refreshed successfully".into());
                    }
                    // Store new refresh token if provided
                    if let Some(new_rt) = data.get("refresh_token").and_then(|v| v.as_str()) {
                        store_refresh_token(new_rt);
                    }
                }
                Err(e) => {
                    web_sys::console::warn_1(&format!("Token refresh failed: {}", e).into());
                    // If refresh fails, clear tokens and redirect to login
                    clear_stored_token();
                    state.set_access_token.set(None);
                    state.set_user.set(None);
                    if let Some(window) = web_sys::window() {
                        let _ = window.location().set_href("/ui/");
                    }
                    return;
                }
            }
        }
    });
}

#[cfg(not(target_arch = "wasm32"))]
pub fn spawn_token_refresh(_state: &AuthState) {}

#[cfg(target_arch = "wasm32")]
pub fn init_auth(state: &AuthState) {
    let token = read_stored_token();
    if token.is_some() {
        state.set_access_token.set(token);
    }

    let state = state.clone();
    spawn_local(async move {
        let token = state.access_token.get_untracked();

        if token.is_some() {
            match crate::api::fetch_json("/api/auth/info").await {
                Ok(data) => {
                    if let Some(sub) = data.get("sub").and_then(|v| v.as_str()) {
                        let user = UserInfo {
                            sub: sub.to_string(),
                            email: data.get("email").and_then(|v| v.as_str()).map(|s| s.to_string()),
                            name: data.get("name").and_then(|v| v.as_str()).map(|s| s.to_string()),
                        };
                        state.set_user.set(Some(user));
                    } else {
                        state.set_access_token.set(None);
                        clear_stored_token();
                    }
                }
                Err(_) => {
                    state.set_access_token.set(None);
                    clear_stored_token();
                }
            }
        }

        let mut auth_mode = "none".to_string();
        match crate::api::get_auth_config().await {
            Ok(config) => {
                auth_mode = config.auth_mode.clone();
                state.set_auth_enabled.set(config.configured);
                if !config.configured {
                    state.set_loading.set(false);
                }
            }
            Err(_) => {
                state.set_auth_enabled.set(false);
                state.set_loading.set(false);
            }
        }

        // Auth guard: when auth is enabled and the visitor is anonymous,
        // send them to the right login flow instead of rendering an empty
        // file browser. Login/callback pages are exempt to avoid loops.
        if state.auth_enabled.get_untracked() && state.access_token.get_untracked().is_none() {
            let path = web_sys::window()
                .map(|w| w.location().pathname().unwrap_or_default())
                .unwrap_or_default();
            let on_auth_page = path.starts_with("/ui/auth/login") || path.starts_with("/ui/auth/callback");
            if !on_auth_page {
                if auth_mode == "basic" {
                    // Basic mode: OIDC is not configured, so never bounce the
                    // visitor at /api/auth/login (it only serves an error).
                    // Stored basic credentials are verified here; on success
                    // the session resumes silently.
                    match read_stored_basic() {
                        Some(header) => {
                            // /api/auth/info is public (always "anonymous"
                            // here), so validate against an authenticated
                            // endpoint instead.
                            if crate::api::verify_basic_auth(&header).await {
                                let user = UserInfo {
                                    sub: basic_auth_username(&header).unwrap_or_else(|| "admin".to_string()),
                                    email: None,
                                    name: basic_auth_username(&header),
                                };
                                state.set_user.set(Some(user));
                            } else {
                                clear_stored_basic();
                                redirect_to_basic_login();
                                return;
                            }
                        }
                        None => {
                            redirect_to_basic_login();
                            return;
                        }
                    }
                } else {
                    match crate::api::auth_login().await {
                        Ok(resp) => {
                            if let Some(window) = web_sys::window() {
                                let _ = window.location().set_href(&resp.authorization_url);
                            }
                        }
                        Err(e) => {
                            web_sys::console::warn_1(&format!("Auth redirect failed: {}", e).into());
                            state.set_loading.set(false);
                        }
                    }
                }
                return;
            }
        }

        state.set_loading.set(false);
    });
}

#[cfg(target_arch = "wasm32")]
fn redirect_to_basic_login() {
    if let Some(window) = web_sys::window() {
        let current = window.location().pathname().unwrap_or_default();
        if !current.starts_with("/ui/auth/login") {
            let _ = window.location().set_href("/ui/auth/login");
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn init_auth(_state: &AuthState) {}

#[cfg(target_arch = "wasm32")]
pub fn start_login() {
    spawn_local(async move {
        match crate::api::auth_login().await {
            Ok(resp) => {
                if let Some(window) = web_sys::window() {
                    let location = window.location();
                    let _ = location.set_href(&resp.authorization_url);
                }
            }
            Err(e) => {
                web_sys::console::warn_1(&format!("Login failed: {}", e).into());
            }
        }
    });
}

/// Redirect the browser to the OIDC login. Called from anywhere an API call
/// returns 401 (e.g. expired session) so the user never stares at an empty
/// file browser.
#[cfg(target_arch = "wasm32")]
pub fn redirect_to_login() {
    start_login();
}

#[cfg(not(target_arch = "wasm32"))]
pub fn redirect_to_login() {}

#[cfg(not(target_arch = "wasm32"))]
pub fn start_login() {}

#[cfg(target_arch = "wasm32")]
pub fn handle_callback(state: &AuthState, code: &str, query_state: &str) {
    let code = code.to_string();
    let query_state = query_state.to_string();
    let state = state.clone();
    spawn_local(async move {
        match crate::api::auth_callback(&code, &query_state).await {
            Ok(resp) => {
                store_token(&resp.access_token);
                state.set_access_token.set(Some(resp.access_token));
                state.set_user.set(Some(resp.user));
                // Store refresh token if provided
                if let Some(rt) = &resp.refresh_token {
                    store_refresh_token(rt);
                }
                // Store OIDC logout URL for proper front-channel logout
                store_oidc_logout_url(&resp.logout_url);
                let redirect = if resp.redirect.is_empty() {
                    "/ui/".to_string()
                } else {
                    resp.redirect
                };
                if let Some(window) = web_sys::window() {
                    let location = window.location();
                    let _ = location.set_href(&redirect);
                }
            }
            Err(e) => {
                web_sys::console::warn_1(&format!("Auth callback failed: {}", e).into());
            }
        }
    });
}

#[cfg(not(target_arch = "wasm32"))]
pub fn handle_callback(_state: &AuthState, _code: &str, _query_state: &str) {}

#[cfg(target_arch = "wasm32")]
pub fn logout(state: &AuthState) {
    clear_stored_token();
    clear_stored_basic();
    state.set_access_token.set(None);
    state.set_user.set(None);
    // Redirect to OIDC end_session_endpoint for proper front-channel logout
    if let Some(logout_url) = read_oidc_logout_url() {
        if !logout_url.is_empty() {
            if let Some(window) = web_sys::window() {
                let location = window.location();
                let _ = location.set_href(&logout_url);
                return;
            }
        }
    }
    // Fallback: just redirect to home
    if let Some(window) = web_sys::window() {
        let location = window.location();
        let _ = location.set_href("/ui/");
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn logout(_state: &AuthState) {}

#[cfg(test)]
mod tests {
    use super::base64;
    use super::basic_auth_header;
    use super::basic_auth_username;

    #[test]
    fn test_base64() {
        assert_eq!(base64(b"admin"), "YWRtaW4=");
        assert_eq!(base64(b"ad"), "YWQ=");
        assert_eq!(base64(b"a"), "YQ==");
        assert_eq!(base64(b""), "");
        // utf-8 password
        assert_eq!(base64("pässword".as_bytes()), "cMOkc3N3b3Jk");
    }

    #[test]
    fn test_basic_username_roundtrip() {
        let header = basic_auth_header("admin", "päss:wörD").ok().unwrap();
        assert_eq!(basic_auth_username(&header).as_deref(), Some("admin"));
        assert_eq!(basic_auth_username("garbage"), None);
        assert_eq!(basic_auth_username("Basic !!!"), None);
    }

    #[test]
    fn test_basic_header() {
        assert_eq!(
            basic_auth_header("admin", "secret").ok(),
            Some(format!("Basic {}", base64(b"admin:secret")))
        );
        assert!(basic_auth_header("user:evil", "x").is_err());
        assert!(basic_auth_header("", "x").is_ok());
    }
}
