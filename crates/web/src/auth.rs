use leptos::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserInfo {
    pub sub: String,
    pub email: Option<String>,
    pub name: Option<String>,
}

#[allow(dead_code)] // Used by WASM runtime
const STORAGE_KEY: &str = "ferro_access_token";

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
        self.access_token.get().is_some()
    }

    pub fn get_access_token(&self) -> Option<String> {
        self.access_token.get()
    }
}

pub fn provide_auth_state() -> AuthState {
    let (access_token, set_access_token) = create_signal(None);
    let (user, set_user) = create_signal(None);
    let (auth_enabled, set_auth_enabled) = create_signal(false);
    let (loading, set_loading) = create_signal(true);

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
    }
}

#[cfg(target_arch = "wasm32")]
pub fn get_auth_header() -> Option<String> {
    read_stored_token().map(|t| format!("Bearer {}", t))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn get_auth_header() -> Option<String> {
    None
}

#[cfg(target_arch = "wasm32")]
pub fn init_auth(state: &AuthState) {
    let token = read_stored_token();
    if token.is_some() {
        state.set_access_token.set(token);
    }

    let state = state.clone();
    spawn_local(async move {
        let token = state.access_token.get();

        if token.is_some() {
            match crate::api::fetch_json("/api/auth/info").await {
                Ok(data) => {
                    if let Some(sub) = data.get("sub").and_then(|v| v.as_str()) {
                        let user = UserInfo {
                            sub: sub.to_string(),
                            email: data
                                .get("email")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                            name: data
                                .get("name")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
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

        if !state.auth_enabled.get() {
            state.set_loading.set(false);
        } else {
            state.set_loading.set(false);
        }
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
                web_sys::console::log_1(&format!("Login failed: {}", e).into());
            }
        }
    });
}

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
                web_sys::console::log_1(&format!("Auth callback failed: {}", e).into());
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
    if let Some(window) = web_sys::window() {
        let location = window.location();
        let _ = location.set_href("/ui/");
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn logout(_state: &AuthState) {}
