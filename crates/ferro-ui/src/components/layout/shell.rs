use leptos::prelude::*;

/// Application shell layout.
#[component]
pub fn Shell(children: Children) -> impl IntoView {
    view! {
        <div class="shell">
            {children()}
        </div>
    }
}

/// Fixed header bar.
#[component]
pub fn Header(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
    let class_str = format!("shell-header {}", class);
    view! {
        <header class=class_str>
            {children()}
        </header>
    }
}

/// Sidebar navigation panel.
#[component]
pub fn Sidebar(
    #[prop(optional)] open: bool,
    #[prop(into, optional)] class: String,
    children: Children,
) -> impl IntoView {
    let class_str = if open {
        format!("shell-sidebar {}", class)
    } else {
        format!("shell-sidebar hidden {}", class)
    };
    view! {
        <aside class=class_str>
            {children()}
        </aside>
    }
}

/// Main content area.
#[component]
pub fn ContentArea(#[prop(into, optional)] class: String, children: Children) -> impl IntoView {
    let class_str = format!("shell-content {}", class);
    view! {
        <main class=class_str>
            {children()}
        </main>
    }
}
