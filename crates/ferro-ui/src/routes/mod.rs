use leptos::prelude::*;
use leptos_router::components::*;
use leptos_router::path;

use crate::components::domain::admin::AdminPage;
use crate::components::domain::calendar::CalendarPage;
use crate::components::domain::chat::ChatPage;
use crate::components::domain::contacts::ContactsPage;
use crate::components::domain::file_browser::FileBrowser;
use crate::components::domain::notes::NotesPage;
use crate::components::domain::photos::PhotosPage;
use crate::components::domain::settings::SettingsPage;
use crate::components::domain::tasks::TasksPage;
use crate::components::domain::trash::TrashPage;
use crate::components::infrastructure::error_boundary::ErrorBoundary;
use crate::components::infrastructure::toast::ToastProvider;
use crate::stores::auth::provide_auth;
use crate::stores::theme::provide_theme;
use crate::styles::inject_styles;

/// Call Tauri v2 IPC from WASM.
#[cfg(target_arch = "wasm32")]
async fn tauri_invoke(cmd: &str, args: &serde_json::Value) -> Result<String, String> {
    use wasm_bindgen::JsCast;
    let window = web_sys::window().ok_or("no window")?;
    let tauri = js_sys::Reflect::get(&window, &wasm_bindgen::JsValue::from_str("__TAURI__"))
        .map_err(|_| "no __TAURI__".to_string())?;
    let core = js_sys::Reflect::get(&tauri, &wasm_bindgen::JsValue::from_str("core"))
        .map_err(|_| "no __TAURI__.core".to_string())?;
    let invoke = js_sys::Reflect::get(&core, &wasm_bindgen::JsValue::from_str("invoke"))
        .map_err(|_| "no __TAURI__.core.invoke".to_string())?;
    let invoke_fn: js_sys::Function = invoke.dyn_into().map_err(|_| "invoke not a function".to_string())?;
    // Parse JSON string into a JS object (Tauri expects an object, not a string)
    let args_js = js_sys::JSON::parse(&serde_json::to_string(args).unwrap_or_default())
        .map_err(|e| format!("JSON parse: {:?}", e))?;
    let result = invoke_fn
        .call2(&core, &wasm_bindgen::JsValue::from_str(cmd), &args_js)
        .map_err(|e| format!("invoke: {:?}", e))?;
    let promise: js_sys::Promise = result.dyn_into().map_err(|_| "not promise".to_string())?;
    let value = wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map_err(|e| format!("promise: {:?}", e))?;
    value.as_string().ok_or("result not string".to_string())
}

/// Root application component.
#[component]
pub fn App() -> impl IntoView {
    inject_styles();
    provide_theme();
    provide_auth();

    view! {
        <ErrorBoundary>
            <ToastProvider>
                <Router>
                    <Routes fallback=|| "Page not found".into_view()>
                        <Route path=path!("/") view=HomePage />
                        <Route path=path!("/ui/") view=HomePage />
                        <Route path=path!("/ui/files") view=HomePage />
                        <Route path=path!("/ui/notes") view=NotesPage />
                        <Route path=path!("/ui/tasks") view=TasksPage />
                        <Route path=path!("/ui/calendar") view=CalendarPage />
                        <Route path=path!("/ui/contacts") view=ContactsPage />
                        <Route path=path!("/ui/chat") view=ChatPage />
                        <Route path=path!("/ui/photos") view=PhotosPage />
                        <Route path=path!("/ui/settings") view=SettingsPage />
                        <Route path=path!("/ui/admin") view=AdminPage />
                        <Route path=path!("/ui/trash") view=TrashPage />
                    </Routes>
                </Router>
            </ToastProvider>
        </ErrorBoundary>
    }
}

/// Home / file browser page.
#[component]
fn HomePage() -> impl IntoView {
    let (server_url, set_server_url) = signal(String::new());

    // Get server URL from window.FERRO_SERVER_URL (set by inline script in index.html)
    #[cfg(target_arch = "wasm32")]
    {
        let set = set_server_url;
        wasm_bindgen_futures::spawn_local(async move {
            // Use window.__TAURI__ to get the URL from state
            let url = tauri_invoke("get_cli_connection", &serde_json::json!({}))
                .await
                .ok()
                .and_then(|json| serde_json::from_str::<serde_json::Value>(&json).ok())
                .and_then(|conn| conn.get("serverUrl").and_then(|v| v.as_str()).map(String::from))
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "http://127.0.0.1:13000".to_string());
            set.set(url);
        });
    }

    view! {
        <div class="shell">
            <AppHeader />
            <main class="shell-content" style="padding:0;">
                {move || {
                    let url = server_url.get();
                    if url.is_empty() {
                        view! { <div class="p-8 text-center text-secondary">"Connecting..."</div> }.into_any()
                    } else {
                        view! { <FileBrowser server_url=url /> }.into_any()
                    }
                }}
            </main>
        </div>
    }
}

/// Shared app header with navigation.
#[component]
fn AppHeader() -> impl IntoView {
    view! {
        <header class="shell-header">
            <a href="/ui/" class="text-xl font-bold tracking-tight">"⚡ Ferro"</a>
            <nav class="flex items-center gap-1 ml-6 overflow-x-auto">
                <a href="/ui/" class="nav-link">"Files"</a>
                <a href="/ui/notes" class="nav-link">"Notes"</a>
                <a href="/ui/tasks" class="nav-link">"Tasks"</a>
                <a href="/ui/calendar" class="nav-link">"Calendar"</a>
                <a href="/ui/contacts" class="nav-link">"Contacts"</a>
                <a href="/ui/chat" class="nav-link">"Chat"</a>
                <a href="/ui/photos" class="nav-link">"Photos"</a>
                <a href="/ui/trash" class="nav-link">"Trash"</a>
                <a href="/ui/admin" class="nav-link">"Admin"</a>
                <a href="/ui/settings" class="nav-link">"Settings"</a>
            </nav>
            <div class="ml-auto shrink-0">
                <ThemeToggle />
            </div>
        </header>
    }
}

/// Theme toggle button.
#[component]
fn ThemeToggle() -> impl IntoView {
    let theme = crate::stores::theme::use_theme();

    view! {
        <button
            class="btn btn-ghost btn-sm"
            on:click=move |_| theme.toggle()
            aria-label="Toggle theme"
        >
            {move || if theme.theme.get().is_dark() { "🌙" } else { "☀️" }}
        </button>
    }
}
