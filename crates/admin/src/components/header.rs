use leptos::prelude::*;
use leptos_router::hooks::use_location;

use crate::api::ApiState;

#[component]
pub fn Header(api: RwSignal<ApiState>) -> impl IntoView {
    let location = use_location();
    let is_connected = move || api.with(|a| a.is_connected());

    let page_title = move || {
        let path = location.pathname.get();
        match path.as_str() {
            "/" => "Dashboard",
            "/users" => "User Management",
            "/storage" => "Storage",
            "/monitoring" => "Monitoring",
            "/settings" => "Settings",
            "/federation" => "Federation",
            "/webhooks" => "Webhooks",
            "/plugins" => "Plugin Marketplace",
            "/audit" => "Audit Log",
            "/login" => "Connect to Server",
            _ => "Ferro Admin",
        }
        .to_string()
    };

    let conn = is_connected();

    view! {
        <header class="admin-header" role="banner">
            <div class="header-left">
                <h1 class="header-title font-display text-accent">{page_title}</h1>
            </div>
            <div class="header-right">
                <div class="connection-status" class:status-connected=conn aria-live="polite" role="status">
                    <span class="status-dot" aria-hidden="true"></span>
                    <span class="status-text">
                        {move || if is_connected() { "Connected" } else { "Disconnected" }}
                    </span>
                </div>
                <button class="header-btn" on:click=move |_| {
                    if let Some(w) = web_sys::window() {
                        let _ = w.location().reload();
                    }
                } title="Refresh page" aria-label="Refresh page">
                    <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true">
                        <path d="M13.5 2.5v4h-4M2.5 13.5v-4h4M2.5 5.5A5.5 5.5 0 0113 3M13.5 10.5a5.5 5.5 0 01-10.5 2.5"/>
                    </svg>
                </button>
            </div>
        </header>
    }
}
