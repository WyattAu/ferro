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
    read_stored_token().map(|t| format!("Bearer {}", t))
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

        match crate::api::get_auth_config().await {
            Ok(config) => {
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

        // Auth guard: when OIDC auth is enabled and the visitor is anonymous,
        // redirect to the Keycloak login instead of rendering an empty file
        // browser. Login/callback pages are exempt to avoid redirect loops.
        if state.auth_enabled.get_untracked() && state.access_token.get_untracked().is_none() {
            let path = web_sys::window()
                .map(|w| w.location().pathname().unwrap_or_default())
                .unwrap_or_default();
            let on_auth_page = path.starts_with("/ui/auth/login") || path.starts_with("/ui/auth/callback");
            if !on_auth_page {
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
                return;
            }
        }

        state.set_loading.set(false);
    });
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
