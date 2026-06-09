use leptos::prelude::*;
use leptos_router::components::*;
use leptos_router::hooks::use_location;

use crate::api::ApiState;

#[component]
pub fn Sidebar(api: RwSignal<ApiState>) -> impl IntoView {
    let location = use_location();
    let is_connected = move || api.with(|a| a.is_connected());
    let server_url =
        move || api.with(|a| a.config.as_ref().map(|c| c.url.clone()).unwrap_or_default());

    let disconnect = Callback::new(move |_: ()| {
        api.update(|a| a.disconnect());
        crate::state::clear_connection();
    });

    let nav_items: Vec<(String, String, String)> = vec![
        ("/".into(), "Dashboard".into(), "dashboard".into()),
        ("/users".into(), "Users".into(), "users".into()),
        ("/storage".into(), "Storage".into(), "storage".into()),
        (
            "/monitoring".into(),
            "Monitoring".into(),
            "monitoring".into(),
        ),
        ("/settings".into(), "Settings".into(), "settings".into()),
        (
            "/federation".into(),
            "Federation".into(),
            "federation".into(),
        ),
        ("/webhooks".into(), "Webhooks".into(), "webhooks".into()),
        ("/audit".into(), "Audit Log".into(), "audit".into()),
    ];

    let nav_links: Vec<_> = nav_items
        .into_iter()
        .map(|(path, label, icon)| {
            let pathname = move || location.pathname.get();
            let p = path.clone();
            let active = Memo::new(move |_| {
                let current = pathname();
                if p == "/" {
                    current == "/"
                } else {
                    current.starts_with(&p)
                }
            });
            let nav_class = move || {
                if active.get() {
                    "nav-item nav-active".to_string()
                } else {
                    "nav-item".to_string()
                }
            };
            let ac = active;
            let icon_svg = svg_icon(&icon);
            view! {
                <A href=path attr:class=nav_class attr:aria-current=move || if ac.get() { Some("page") } else { None }>
                    <span class="nav-icon" aria-hidden="true">{icon_svg}</span>
                    <span class="nav-label font-display">{label}</span>
                </A>
            }
        })
        .collect();

    let conn = is_connected();

    view! {
        <aside class="sidebar" role="complementary" aria-label="Admin navigation sidebar">
            <div class="sidebar-header">
                <svg width="28" height="28" viewBox="0 0 28 28" fill="none" aria-hidden="true">
                    <rect width="28" height="28" rx="6" fill="#E85D04"/>
                    <path d="M8 14h12M14 8v12" stroke="white" stroke-width="2.5" stroke-linecap="round"/>
                </svg>
                <span class="sidebar-brand font-display">"Ferro Admin"</span>
            </div>

            <nav class="sidebar-nav" aria-label="Main navigation">{nav_links}</nav>

            <div class="sidebar-footer">
                <div class="sidebar-server" class:sidebar-connected=conn aria-live="polite">
                    <span class="server-status-dot" aria-hidden="true"></span>
                    <span class="server-url" title=server_url()>
                        {move || if is_connected() { "Connected" } else { "Disconnected" }}
                    </span>
                </div>
                <button class="sidebar-disconnect" on:click=move |_| disconnect.run(()) disabled=!conn aria-label="Disconnect from server">
                    "Disconnect"
                </button>
            </div>
        </aside>
    }
}

fn svg_icon(name: &str) -> AnyView {
    let d = match name {
        "dashboard" => "M3 3h7v7H3zM14 3h7v7h-7zM3 14h7v7H3zM14 14h7v7h-7z",
        "users" => {
            "M9 7a3 3 0 100-6 3 3 0 000 6zM17 7a3 3 0 100-6 3 3 0 000 6zM3 19c0-3.3 2.7-6 6-6s6 2.7 6 6M17 13c2.2 0 4 1.3 4 3"
        }
        "storage" => "M3 4h18v16H3zM3 10h18M3 15h18",
        "monitoring" => "M3 17l5-5 5 2 5-7 3 2M3 19h18",
        "settings" => {
            "M12 1v3M12 20v3M4.2 4.2l2.1 2.1M15.7 15.7l2.1 2.1M1 12h3M20 12h3M4.2 19.8l2.1-2.1M15.7 8.3l2.1-2.1"
        }
        "federation" => {
            "M12 5a2.5 2.5 0 100-5 2.5 2.5 0 000 5zM5 17a2.5 2.5 0 100-5 2.5 2.5 0 000 5zM19 17a2.5 2.5 0 100-5 2.5 2.5 0 000 5zM12 7.5L5 14.5M12 7.5l7 7"
        }
        "webhooks" => {
            "M18 5a2.5 2.5 0 100-5 2.5 2.5 0 000 5zM6 12a2.5 2.5 0 100-5 2.5 2.5 0 000 5zM18 19a2.5 2.5 0 100-5 2.5 2.5 0 000 5zM15.5 6.5L8.5 11M8.5 13l7 4.5"
        }
        "audit" => "M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8zM14 2v6h6M8 13h8M8 17h4",
        _ => "",
    };
    view! {
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <path d=d/>
        </svg>
    }.into_any()
}
