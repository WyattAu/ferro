use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_navigate;

use crate::api::ApiState;
use crate::state::save_connection;

#[component]
pub fn LoginPage(api: RwSignal<ApiState>) -> impl IntoView {
    let (server_url, set_server_url) = signal(String::new());
    let (token, set_token) = signal(String::new());
    let (error, set_error) = signal(None::<String>);
    let (loading, set_loading) = signal(false);
    let has_saved = api.with(|a| a.is_connected());

    let _handle_connect = move |_: leptos::ev::MouseEvent| {
        let url = server_url.get();
        let tok = token.get();
        if url.trim().is_empty() {
            set_error.set(Some("Server URL is required".to_string()));
            return;
        }
        if tok.trim().is_empty() {
            set_error.set(Some("Admin token is required".to_string()));
            return;
        }
        set_loading.set(true);
        set_error.set(None);
        let url_val = url.trim().to_string();
        let token_val = tok.trim().to_string();
        spawn_local(async move {
            let mut test_api = api.get_untracked();
            let u = url_val.clone();
            let t = token_val.clone();
            test_api.connect(u, t);
            match test_api.test_connection().await {
                Ok(_) => {
                    save_connection(&crate::api::AdminConnectionConfig {
                        url: url_val.clone(),
                        token: token_val.clone(),
                    });
                    api.update(|a| a.connect(url_val, token_val));
                    set_error.set(None);
                    use leptos_router::hooks::use_navigate;
                    let navigate = use_navigate();
                    navigate("/", Default::default());
                }
                Err(e) => set_error.set(Some(format!("Connection failed: {}", e))),
            }
            set_loading.set(false);
        });
    };

    view! {
        <div class="login-page">
            <div class="login-card surface brutal-border">
                <div class="login-header">
                    <svg width="48" height="48" viewBox="0 0 48 48" fill="none" aria-hidden="true">
                        <rect width="48" height="48" rx="10" fill="#E85D04"/>
                        <path d="M14 24h20M24 14v20" stroke="white" stroke-width="4" stroke-linecap="round"/>
                    </svg>
                    <h1 class="login-title font-display text-accent">"Ferro Admin"</h1>
                    <p class="login-subtitle">"Connect to your Ferro server to manage it"</p>
                </div>

                <form class="login-form" on:submit=move |ev| ev.prevent_default() aria-label="Server connection form">
                    <div class="form-group">
                        <label class="form-label" for="server-url">"Server URL"</label>
                        <input id="server-url" type="url" class="form-input" placeholder="https://ferro.example.com" prop:value=server_url on:input=move |ev| set_server_url.set(event_target_value(&ev)) aria-required="true" />
                    </div>
                    <div class="form-group">
                        <label class="form-label" for="admin-token">"Admin Token"</label>
                        <input id="admin-token" type="password" class="form-input" placeholder="Enter your admin token or password" prop:value=token on:input=move |ev| set_token.set(event_target_value(&ev)) aria-required="true" />
                    </div>
                    <div aria-live="assertive">
                        {move || error.get().map(|e| view! { <div class="form-error" role="alert">{e}</div> })}
                    </div>
                    <button type="submit" class="btn btn-primary btn-block" disabled=loading aria-label=move || if loading.get() { "Connecting to server" } else { "Connect to server" }>
                        {move || if loading.get() { "Connecting..." } else { "Connect" }}
                    </button>
                    {has_saved.then(|| view! {
                        <button type="button" class="btn btn-secondary btn-block" on:click=move |_| {
                            let navigate = use_navigate();
                            navigate("/", Default::default());
                        } aria-label="Go to dashboard">"Go to Dashboard"</button>
                    })}
                </form>
                <div class="login-footer">
                    <p>"The admin panel connects to your Ferro server via its REST API."</p>
                    <p>"Your credentials are stored locally in your browser."</p>
                </div>
            </div>
        </div>
    }
}
