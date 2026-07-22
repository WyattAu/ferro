use leptos::prelude::*;

/// Authentication state.
#[derive(Clone)]
pub struct AuthState {
    pub token: ReadSignal<Option<String>>,
    set_token: WriteSignal<Option<String>>,
    pub user: ReadSignal<Option<UserInfo>>,
    set_user: WriteSignal<Option<UserInfo>>,
    pub loading: ReadSignal<bool>,
    set_loading: WriteSignal<bool>,
}

#[derive(Clone, Debug, Default)]
pub struct UserInfo {
    pub sub: String,
    pub email: Option<String>,
    pub name: Option<String>,
}

impl AuthState {
    pub fn is_authenticated(&self) -> bool {
        self.token.get().is_some()
    }

    pub fn set_token(&self, token: Option<String>) {
        self.set_token.set(token);
    }

    pub fn logout(&self) {
        self.set_token.set(None);
        self.set_user.set(None);
    }
}

/// Provide auth state to the component tree.
pub fn provide_auth() -> AuthState {
    let initial_token = read_stored_token();
    let (token, set_token) = signal(initial_token);
    let (user, set_user) = signal(None::<UserInfo>);
    let (loading, set_loading) = signal(true);

    let state = AuthState {
        token,
        set_token,
        user,
        set_user,
        loading,
        set_loading,
    };
    provide_context(state.clone());

    state
}

/// Get auth state from context.
pub fn use_auth() -> AuthState {
    use_context::<AuthState>().expect("AuthState not provided")
}

#[cfg(target_arch = "wasm32")]
fn read_stored_token() -> Option<String> {
    web_sys::window()
        .and_then(|w| w.local_storage().ok())
        .flatten()
        .and_then(|s| s.get_item("ferro_token").ok())
        .flatten()
        .filter(|t| !t.is_empty())
}

#[cfg(not(target_arch = "wasm32"))]
fn read_stored_token() -> Option<String> {
    None
}
