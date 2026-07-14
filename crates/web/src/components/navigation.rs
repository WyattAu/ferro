use leptos::prelude::*;
use leptos_router::components::A;

use crate::components::theme_toggle::ThemeToggle;
use crate::t;

#[component]
pub fn NavigationSidebar() -> impl IntoView {
    let (mobile_open, set_mobile_open) = signal(false);

    let nav_items = vec![
        (
            "/ui/dashboard",
            "nav.dashboard",
            "M3 12l2-2m0 0l7-7 7 7M5 10v10a1 1 0 001 1h3m10-11l2 2m-2-2v10a1 1 0 01-1 1h-3m-6 0a1 1 0 001-1v-4a1 1 0 011-1h2a1 1 0 011 1v4a1 1 0 001 1m-6 0h6",
        ),
        (
            "/ui/files/",
            "nav.files",
            "M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z",
        ),
        (
            "/ui/calendar",
            "nav.calendar",
            "M8 7V3m8 4V3m-9 8h10M5 21h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z",
        ),
        (
            "/ui/contacts",
            "nav.contacts",
            "M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0z",
        ),
        (
            "/ui/notes",
            "nav.notes",
            "M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z",
        ),
        (
            "/ui/tasks",
            "nav.tasks",
            "M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2m-6 9l2 2 4-4",
        ),
        (
            "/ui/chat",
            "nav.chat",
            "M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z",
        ),
        (
            "/ui/photos",
            "nav.photos",
            "M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z",
        ),
        (
            "/ui/mail",
            "nav.mail",
            "M3 8l7.89 5.26a2 2 0 002.22 0L21 8M5 19h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z",
        ),
        (
            "/ui/whiteboard",
            "nav.whiteboard",
            "M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z",
        ),
        (
            "/ui/analytics",
            "nav.analytics",
            "M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z",
        ),
        (
            "/ui/admin",
            "nav.admin",
            "M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z M15 12a3 3 0 11-6 0 3 3 0 016 0z",
        ),
        (
            "/ui/settings",
            "nav.settings",
            "M12 6V4m0 2a2 2 0 100 4m0-4a2 2 0 110 4m-6 8a2 2 0 100-4m0 4a2 2 0 110-4m0 4v2m0-6V4m6 6v10m6-2a2 2 0 100-4m0 4a2 2 0 110-4m0 4v2m0-6V4",
        ),
        (
            "/ui/trash",
            "nav.trash",
            "M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16",
        ),
    ];

    let close_mobile = move |_| set_mobile_open.set(false);

    view! {
        // Skip navigation link
        <a href="#main-content" class="skip-link">
            {t!("nav.skip_to_content")}
        </a>

        // Mobile hamburger button
        <button
            class="lg:hidden fixed top-3 left-3 z-50 p-2 rounded-md bg-[var(--bg-surface)] border border-[var(--border-default)] shadow-md text-[var(--text-primary)] focus-ring min-w-[44px] min-h-[44px] flex items-center justify-center"
            on:click=move |_| set_mobile_open.update(|v| *v = !*v)
            aria-label="Toggle navigation"
            aria-expanded=move || mobile_open.get()
        >
            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                {move || if mobile_open.get() {
                    view! { <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" /> }.into_any()
                } else {
                    view! { <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 12h16M4 18h16" /> }.into_any()
                }}
            </svg>
        </button>

        // Mobile overlay
        {move || if mobile_open.get() {
            view! {
                <div
                    class="fixed inset-0 z-30 bg-[var(--overlay)] lg:hidden"
                    on:click=close_mobile
                    aria-hidden="true"
                />
            }.into_any()
        } else {
            ().into_any()
        }}

        // Desktop sidebar (always visible on lg+)
        <nav
            class="hidden lg:flex w-56 shrink-0 bg-[var(--bg-surface)] border-r border-[var(--border-default)] overflow-y-auto flex-col h-screen sticky top-0"
            aria-label=t!("nav.main")
        >
            <SidebarContent nav_items=nav_items.clone() />
        </nav>

        // Mobile sidebar (slide-in drawer)
        <nav
            class=move || {
                let base = "fixed top-0 left-0 z-40 w-64 h-full bg-[var(--bg-surface)] border-r border-[var(--border-default)] overflow-y-auto flex flex-col lg:hidden transform transition-transform duration-200 ease-in-out";
                if mobile_open.get() {
                    format!("{} translate-x-0", base)
                } else {
                    format!("{} -translate-x-full", base)
                }
            }
            aria-label=t!("nav.main")
        >
            // Close button inside mobile nav
            <div class="flex items-center justify-between px-4 py-3 border-b border-[var(--border-default)]">
                <span class="text-sm font-semibold text-[var(--text-primary)]">{t!("app.name")}</span>
                <button
                    class="p-1.5 rounded-md text-[var(--text-tertiary)] hover:text-[var(--text-primary)] hover:bg-[var(--interactive-hover)] focus-ring min-w-[44px] min-h-[44px] flex items-center justify-center"
                    on:click=close_mobile
                    aria-label="Close navigation"
                >
                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                    </svg>
                </button>
            </div>
            <SidebarContent nav_items=nav_items.clone() />
        </nav>
    }
}

#[component]
fn SidebarContent(nav_items: Vec<(&'static str, &'static str, &'static str)>) -> impl IntoView {
    view! {
        <div class="flex flex-col flex-1 py-3">
            <For
                each=move || nav_items.clone()
                key=|item| item.0.to_string()
                let:item
            >
                {
                    let (href, label_key, icon_path) = item;
                    view! {
                        <A
                            href=href
                            attr:class="flex items-center gap-3 px-4 py-2.5 mx-2 text-sm font-medium text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--interactive-hover)] rounded-lg transition-colors no-underline"
                        >
                            <svg class="w-5 h-5 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d=icon_path />
                            </svg>
                            {t!(label_key)}
                        </A>
                    }
                }
            </For>
        </div>

        // Bottom section: theme toggle
        <div class="px-4 py-3 border-t border-[var(--border-default)]">
            <ThemeToggle />
        </div>
    }
}

/// Top header bar with search, notifications, and user menu.
#[component]
pub fn TopHeader() -> impl IntoView {
    view! {
        <header class="h-14 shrink-0 bg-[var(--bg-surface)] border-b border-[var(--border-default)] flex items-center px-4 lg:px-6 gap-4">
            // Mobile: spacer for hamburger button
            <div class="w-10 lg:hidden" />

            // Search bar
            <div class="flex-1 max-w-md">
                <div class="relative">
                    <svg class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-[var(--text-tertiary)]" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
                    </svg>
                    <input
                        type="search"
                        placeholder=t!("nav.search_placeholder")
                        aria-label=t!("nav.search_placeholder")
                        class="w-full pl-10 pr-4 py-2 text-sm rounded-lg border border-[var(--border-default)] bg-[var(--bg-surface-sunken)] text-[var(--text-primary)] placeholder-[var(--text-tertiary)] focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] focus:border-[var(--border-focus)] transition-colors min-h-[40px]"
                    />
                </div>
            </div>

            // Right side: theme toggle + notifications
            <div class="flex items-center gap-2">
                <ThemeToggle />

                // Notifications bell
                <button
                    class="p-2 rounded-md text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--interactive-hover)] transition-colors focus-ring min-w-[44px] min-h-[44px] flex items-center justify-center relative"
                    aria-label="Notifications"
                >
                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 17h5l-1.405-1.405A2.032 2.032 0 0118 14.158V11a6.002 6.002 0 00-4-5.659V5a2 2 0 10-4 0v.341C7.67 6.165 6 8.388 6 11v3.159c0 .538-.214 1.055-.595 1.436L4 17h5m6 0v1a3 3 0 11-6 0v-1m6 0H9" />
                    </svg>
                </button>
            </div>
        </header>
    }
}
