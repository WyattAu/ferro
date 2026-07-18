use leptos::ev;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::A;

use crate::api::{self, UserPreferences};
use crate::components::navigation::NavigationSidebar;
use crate::components::onboarding::reset_onboarding;
use crate::components::toast::ToastContext;
use crate::t;

#[derive(Debug, Clone, PartialEq)]
enum SettingsTab {
    Account,
    Preferences,
    Notifications,
    Appearance,
    Sync,
}

#[component]
pub fn SettingsPage() -> impl IntoView {
    let (tab, set_tab) = signal(SettingsTab::Preferences);
    let (prefs, set_prefs) = signal(UserPreferences {
        theme: "dark".to_string(),
        view_mode: "list".to_string(),
        sort_by: "name".to_string(),
        sort_order: "asc".to_string(),
        items_per_page: 50,
        show_hidden_files: false,
        language: "en".to_string(),
    });
    let (loading, set_loading) = signal(true);
    let (saving, set_saving) = signal(false);

    // Account tab
    let (profile_name, set_profile_name) = signal(String::new());
    let (profile_email, set_profile_email) = signal(String::new());
    let (current_password, set_current_password) = signal(String::new());
    let (new_password, set_new_password) = signal(String::new());
    let (confirm_password, set_confirm_password) = signal(String::new());

    // Notifications tab
    let (notify_email_shares, set_notify_email_shares) = signal(true);
    let (notify_email_comments, set_notify_email_comments) = signal(true);
    let (notify_email_uploads, set_notify_email_uploads) = signal(false);
    let (notify_push_shares, set_notify_push_shares) = signal(true);
    let (notify_push_comments, set_notify_push_comments) = signal(false);
    let (notify_push_uploads, set_notify_push_uploads) = signal(false);

    // Appearance
    let (dark_mode, set_dark_mode) = signal(true);
    let (language, set_language) = signal("en".to_string());

    // Sync
    let (offline_enabled, set_offline_enabled) = signal(false);
    let (cache_size, set_cache_size) = signal(256_u64);

    Effect::new(move |_| {
        spawn_local(async move {
            if let Ok(p) = api::get_preferences().await {
                set_prefs.set(p.clone());
                set_dark_mode.set(p.theme == "dark");
                set_language.set(p.language);
            }
            set_loading.set(false);
        });
    });

    let save_prefs = move |_: ev::MouseEvent| {
        set_saving.set(true);
        let mut p = prefs.get();
        p.theme = if dark_mode.get() {
            "dark".to_string()
        } else {
            "light".to_string()
        };
        p.language = language.get();
        spawn_local(async move {
            match api::update_preferences(&p).await {
                Ok(_) => ToastContext::success(t!("toast.preferences_saved")),
                Err(e) => ToastContext::error(format!("Failed to save: {}", e)),
            }
            set_saving.set(false);
        });
    };

    let save_account = move |_: ev::MouseEvent| {
        let name = profile_name.get();
        let email = profile_email.get();
        spawn_local(async move {
            let body = serde_json::json!({ "name": name, "email": email });
            let _ = api::fetch_json_with_method("/api/user/profile", "PUT", Some(&body.to_string())).await;
            ToastContext::success(t!("toast.preferences_saved"));
        });
    };

    let change_password = move |_: ev::MouseEvent| {
        let current = current_password.get();
        let new_pw = new_password.get();
        let confirm = confirm_password.get();
        if new_pw != confirm {
            ToastContext::error(t!("settings.password_mismatch"));
            return;
        }
        spawn_local(async move {
            let body = serde_json::json!({ "current_password": current, "new_password": new_pw });
            let _ = api::fetch_json_with_method("/api/user/password", "PUT", Some(&body.to_string())).await;
            ToastContext::success(t!("toast.preferences_saved"));
        });
        set_current_password.set(String::new());
        set_new_password.set(String::new());
        set_confirm_password.set(String::new());
    };

    let handle_reset_onboarding = move |_: ev::MouseEvent| {
        reset_onboarding();
        ToastContext::info(t!("toast.onboarding_reset"));
    };

    let on_theme_change = move |ev: ev::Event| {
        let v = event_target_value(&ev);
        set_prefs.update(|p| p.theme = v.clone());
        set_dark_mode.set(v == "dark");
    };

    let on_view_mode_change = move |ev: ev::Event| {
        let v = event_target_value(&ev);
        set_prefs.update(|p| p.view_mode = v);
    };

    let on_sort_by_change = move |ev: ev::Event| {
        let v = event_target_value(&ev);
        set_prefs.update(|p| p.sort_by = v);
    };

    let on_sort_order_change = move |ev: ev::Event| {
        let v = event_target_value(&ev);
        set_prefs.update(|p| p.sort_order = v);
    };

    let on_items_per_page_change = move |ev: ev::Event| {
        let v = event_target_value(&ev);
        set_prefs.update(|p| {
            p.items_per_page = v.parse().unwrap_or(50);
        });
    };

    view! {
        <div class="h-screen flex flex-col bg-[var(--bg-base)]">
            <a href="#main-content" class="sr-only focus:not-sr-only focus:absolute focus:top-2 focus:left-2 focus:z-50 focus:px-4 focus:py-2 focus:bg-[var(--accent)] focus:text-[var(--text-on-accent)] focus:rounded">{t!("nav.skip_to_content")}</a>

            <header class="surface brutal-border border-b px-6 py-3 shadow-concrete">
                <div class="flex items-center justify-between max-w-7xl mx-auto">
                    <div class="flex items-center gap-3">
                        <A href="/ui/" attr:class="flex items-center gap-2 no-underline">
                            <div class="w-8 h-8 bg-transparent brutal-border rounded flex items-center justify-center font-display text-accent">
                                <span class="font-bold text-sm">{t!("brand.name")}</span>
                            </div>
                            <div>
                                <h1 class="text-lg font-bold font-mono text-[var(--text-primary)] leading-none">{t!("brand.name")}</h1>
                                <span class="text-label text-muted">{t!("settings.title")}</span>
                            </div>
                        </A>
                    </div>
                    <nav aria-label=t!("nav.back_to_files") class="flex items-center gap-2">
                        <A
                            href="/ui/"
                            attr:class="px-3 py-1.5 text-sm text-[var(--text-secondary)] hover:text-[var(--text-primary)] no-underline rounded hover:bg-[var(--bg-inset)] transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)]"
                        >
                            {t!("nav.back_to_files")}
                        </A>
                    </nav>
                </div>
            </header>

            <div class="flex-1 flex overflow-hidden">
                <NavigationSidebar />
                <main id="main-content" class="flex-1 overflow-auto p-6">
                    {/* Tab Navigation */}
                    <div class="flex flex-wrap items-center gap-1 mb-6">
                        <button on:click=move |_| set_tab.set(SettingsTab::Account) class=move || format!("px-4 py-2 text-sm font-medium rounded-lg transition-colors {}", if tab.get() == SettingsTab::Account { "bg-[var(--accent)] text-[var(--text-on-accent)]" } else { "text-[var(--text-secondary)] dark:text-[var(--text-tertiary)] hover:bg-[var(--interactive-hover)]" })>{t!("settings.tab_account")}</button>
                        <button on:click=move |_| set_tab.set(SettingsTab::Preferences) class=move || format!("px-4 py-2 text-sm font-medium rounded-lg transition-colors {}", if tab.get() == SettingsTab::Preferences { "bg-[var(--accent)] text-[var(--text-on-accent)]" } else { "text-[var(--text-secondary)] dark:text-[var(--text-tertiary)] hover:bg-[var(--interactive-hover)]" })>{t!("settings.tab_preferences")}</button>
                        <button on:click=move |_| set_tab.set(SettingsTab::Notifications) class=move || format!("px-4 py-2 text-sm font-medium rounded-lg transition-colors {}", if tab.get() == SettingsTab::Notifications { "bg-[var(--accent)] text-[var(--text-on-accent)]" } else { "text-[var(--text-secondary)] dark:text-[var(--text-tertiary)] hover:bg-[var(--interactive-hover)]" })>{t!("settings.tab_notifications")}</button>
                        <button on:click=move |_| set_tab.set(SettingsTab::Appearance) class=move || format!("px-4 py-2 text-sm font-medium rounded-lg transition-colors {}", if tab.get() == SettingsTab::Appearance { "bg-[var(--accent)] text-[var(--text-on-accent)]" } else { "text-[var(--text-secondary)] dark:text-[var(--text-tertiary)] hover:bg-[var(--interactive-hover)]" })>{t!("settings.tab_appearance")}</button>
                        <button on:click=move |_| set_tab.set(SettingsTab::Sync) class=move || format!("px-4 py-2 text-sm font-medium rounded-lg transition-colors {}", if tab.get() == SettingsTab::Sync { "bg-[var(--accent)] text-[var(--text-on-accent)]" } else { "text-[var(--text-secondary)] dark:text-[var(--text-tertiary)] hover:bg-[var(--interactive-hover)]" })>{t!("settings.tab_sync")}</button>
                    </div>

                    {move || loading.get().then(|| view! {
                        <div class="px-6 py-12 text-center text-[var(--text-tertiary)]" role="status" aria-live="polite">
                            <div class="animate-spin w-8 h-8 border-2 border-blue-600 border-t-transparent rounded-full mx-auto mb-3"></div>
                            {t!("settings.loading_prefs")}
                        </div>
                    })}

                    <div class="max-w-2xl w-full surface brutal-border shadow-concrete rounded-lg overflow-hidden">
                        {/* Account Tab */}
                        {move || (tab.get() == SettingsTab::Account && !loading.get()).then(|| view! {
                            <div class="p-6 space-y-6">
                                <h2 class="text-section font-mono text-[var(--text-primary)]">{t!("settings.section_account")}</h2>
                                <div class="space-y-4">
                                    <div>
                                        <label class="block text-label font-mono text-[var(--text-secondary)] mb-1" for="profile-name">{t!("settings.profile_name")}</label>
                                        <input id="profile-name" type="text" prop:value=move || profile_name.get() on:input=move |ev| set_profile_name.set(event_target_value(&ev)) class="w-full px-3 py-2 border rounded bg-[var(--bg-surface)] font-mono text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] text-sm" />
                                    </div>
                                    <div>
                                        <label class="block text-label font-mono text-[var(--text-secondary)] mb-1" for="profile-email">{t!("settings.profile_email")}</label>
                                        <input id="profile-email" type="email" prop:value=move || profile_email.get() on:input=move |ev| set_profile_email.set(event_target_value(&ev)) class="w-full px-3 py-2 border rounded bg-[var(--bg-surface)] font-mono text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] text-sm" />
                                    </div>
                                    <div class="pt-4 border-t border-[var(--border-default)]">
                                        <button on:click=save_account class="px-4 py-2 text-sm bg-[var(--accent)] text-[var(--text-on-accent)] brutal-border rounded-sm font-bold uppercase hover:bg-[var(--accent-hover)] disabled:opacity-50 disabled:cursor-not-allowed transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] min-h-[44px]">{t!("common.save")}</button>
                                    </div>
                                </div>
                                <div class="pt-4 border-t border-[var(--border-default)]">
                                    <h3 class="text-label font-mono text-[var(--text-secondary)] mb-3">{t!("settings.change_password")}</h3>
                                    <div class="space-y-3">
                                        <input type="password" placeholder=t!("settings.current_password") prop:value=move || current_password.get() on:input=move |ev| set_current_password.set(event_target_value(&ev)) class="w-full px-3 py-2 border rounded bg-[var(--bg-surface)] font-mono text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] text-sm" />
                                        <input type="password" placeholder=t!("settings.new_password") prop:value=move || new_password.get() on:input=move |ev| set_new_password.set(event_target_value(&ev)) class="w-full px-3 py-2 border rounded bg-[var(--bg-surface)] font-mono text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] text-sm" />
                                        <input type="password" placeholder=t!("settings.confirm_password") prop:value=move || confirm_password.get() on:input=move |ev| set_confirm_password.set(event_target_value(&ev)) class="w-full px-3 py-2 border rounded bg-[var(--bg-surface)] font-mono text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] text-sm" />
                                        <button on:click=change_password class="px-4 py-2 text-sm bg-[var(--accent)] text-[var(--text-on-accent)] brutal-border rounded-sm font-bold uppercase hover:bg-[var(--accent-hover)] transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] min-h-[44px]">{t!("settings.change_password")}</button>
                                    </div>
                                </div>
                            </div>
                        })}

                        {/* Preferences Tab */}
                        {move || (tab.get() == SettingsTab::Preferences && !loading.get()).then(|| view! {
                            <div class="p-6 space-y-6">
                                <h2 class="text-section font-mono text-[var(--text-primary)]">{t!("settings.section_prefs")}</h2>
                                <div class="space-y-5">
                                    <fieldset>
                                        <legend class="block text-label font-mono text-[var(--text-secondary)] mb-2">{t!("settings.default_view_label")}</legend>
                                        <div class="flex items-center gap-4">
                                            <label class="flex items-center gap-2 cursor-pointer">
                                                <input type="radio" name="view_mode" value="list" prop:checked=move || prefs.with(|p| p.view_mode == "list") on:change=on_view_mode_change aria-label="List view" class="text-[var(--accent)] focus:ring-[var(--border-focus)]" />
                                                <span class="text-sm text-[var(--text-secondary)]">{t!("settings.view_list")}</span>
                                            </label>
                                            <label class="flex items-center gap-2 cursor-pointer">
                                                <input type="radio" name="view_mode" value="grid" prop:checked=move || prefs.with(|p| p.view_mode == "grid") on:change=on_view_mode_change aria-label="Grid view" class="text-[var(--accent)] focus:ring-[var(--border-focus)]" />
                                                <span class="text-sm text-[var(--text-secondary)]">{t!("settings.view_grid")}</span>
                                            </label>
                                        </div>
                                    </fieldset>
                                    <div>
                                        <label class="block text-label font-mono text-[var(--text-secondary)] mb-1" for="sort-by">{t!("settings.default_sort_label")}</label>
                                        <select id="sort-by" class="w-full px-3 py-2 border rounded bg-[var(--bg-surface)] font-mono text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] text-sm" on:change=on_sort_by_change>
                                            <option value="name" selected=move || prefs.with(|p| p.sort_by == "name")>{t!("settings.sort_name")}</option>
                                            <option value="date" selected=move || prefs.with(|p| p.sort_by == "date")>{t!("settings.sort_date")}</option>
                                            <option value="size" selected=move || prefs.with(|p| p.sort_by == "size")>{t!("settings.sort_size")}</option>
                                        </select>
                                    </div>
                                    <div>
                                        <label class="block text-label font-mono text-[var(--text-secondary)] mb-1" for="sort-order">{t!("settings.sort_order_label")}</label>
                                        <select id="sort-order" class="w-full px-3 py-2 border rounded bg-[var(--bg-surface)] font-mono text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] text-sm" on:change=on_sort_order_change>
                                            <option value="asc" selected=move || prefs.with(|p| p.sort_order == "asc")>{t!("settings.sort_ascending")}</option>
                                            <option value="desc" selected=move || prefs.with(|p| p.sort_order == "desc")>{t!("settings.sort_descending")}</option>
                                        </select>
                                    </div>
                                    <div>
                                        <label class="block text-label font-mono text-[var(--text-secondary)] mb-1" for="items-per-page">{t!("settings.items_per_page_label")}</label>
                                        <select id="items-per-page" class="w-full px-3 py-2 border rounded bg-[var(--bg-surface)] font-mono text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] text-sm" on:change=on_items_per_page_change>
                                            <option value="25" selected=move || prefs.with(|p| p.items_per_page == 25)>"25"</option>
                                            <option value="50" selected=move || prefs.with(|p| p.items_per_page == 50)>"50"</option>
                                            <option value="100" selected=move || prefs.with(|p| p.items_per_page == 100)>"100"</option>
                                        </select>
                                    </div>
                                    <div class="flex items-center justify-between">
                                        <label class="text-label font-mono text-[var(--text-secondary)]" for="show-hidden">{t!("settings.show_hidden_label")}</label>
                                        <button
                                            id="show-hidden"
                                            role="switch"
                                            aria-checked=move || prefs.with(|p| p.show_hidden_files)
                                            aria-label=move || t!("settings.show_hidden_label")
                                            class=move || format!("relative inline-flex h-6 w-11 items-center rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] focus:ring-offset-2 dark:focus:ring-offset-[var(--bg-base)] {}", if prefs.with(|p| p.show_hidden_files) { "bg-[var(--accent)]" } else { "bg-[var(--border-subtle)] dark:bg-[var(--text-tertiary)]" })
                                            on:click=move |_| { let current = prefs.with(|p| p.show_hidden_files); set_prefs.update(|p| p.show_hidden_files = !current); }
                                        >
                                            <span class=move || format!("inline-block h-4 w-4 transform rounded-full bg-[var(--bg-surface)] transition-transform {}", if prefs.with(|p| p.show_hidden_files) { "translate-x-6" } else { "translate-x-1" })></span>
                                        </button>
                                    </div>
                                </div>
                                <div class="pt-4 border-t border-[var(--border-default)]">
                                    <button
                                        class="px-4 py-2 text-sm bg-[var(--accent)] text-[var(--text-on-accent)] brutal-border rounded-sm font-bold uppercase hover:bg-[var(--accent-hover)] disabled:opacity-50 disabled:cursor-not-allowed transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] min-h-[44px]"
                                        disabled=saving
                                        on:click=save_prefs
                                    >
                                        {move || if saving.get() { t!("common.saving") } else { t!("common.save") }}
                                    </button>
                                </div>
                            </div>
                        })}

                        {/* Notifications Tab */}
                        {move || (tab.get() == SettingsTab::Notifications && !loading.get()).then(|| view! {
                            <div class="p-6 space-y-6">
                                <h2 class="text-section font-mono text-[var(--text-primary)]">{t!("settings.section_notifications")}</h2>
                                <div class="space-y-4">
                                    <div class="grid grid-cols-3 gap-4 text-xs font-bold uppercase font-mono text-[var(--text-tertiary)]">
                                        <div></div>
                                        <div class="text-center">{t!("settings.email")}</div>
                                        <div class="text-center">{t!("settings.push")}</div>
                                    </div>
                                    {vec![
                                        ("shares", "settings.event_shares", notify_email_shares, set_notify_email_shares, notify_push_shares, set_notify_push_shares),
                                        ("comments", "settings.event_comments", notify_email_comments, set_notify_email_comments, notify_push_comments, set_notify_push_comments),
                                        ("uploads", "settings.event_uploads", notify_email_uploads, set_notify_email_uploads, notify_push_uploads, set_notify_push_uploads),
                                    ].into_iter().map(|(_key, label, email_val, set_email, push_val, set_push)| {
                                        let label_text = t!(label);
                                        view! {
                                            <div class="grid grid-cols-3 gap-4 items-center py-2 border-b border-[var(--border-subtle)]">
                                                <span class="text-sm font-mono text-[var(--text-secondary)]">{label_text}</span>
                                                <div class="flex justify-center">
                                                    <button
                                                        role="switch"
                                                        aria-checked=move || email_val.get()
                                                        aria-label=move || format!("{} email notifications", label_text)
                                                        class=move || format!("relative inline-flex h-6 w-11 items-center rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] {}", if email_val.get() { "bg-[var(--accent)]" } else { "bg-[var(--border-subtle)] dark:bg-[var(--text-tertiary)]" })
                                                        on:click=move |_| set_email.set(!email_val.get())
                                                    >
                                                        <span class=move || format!("inline-block h-4 w-4 transform rounded-full bg-[var(--bg-surface)] transition-transform {}", if email_val.get() { "translate-x-6" } else { "translate-x-1" })></span>
                                                    </button>
                                                </div>
                                                <div class="flex justify-center">
                                                    <button
                                                        role="switch"
                                                        aria-checked=move || push_val.get()
                                                        aria-label=move || format!("{} push notifications", label_text)
                                                        class=move || format!("relative inline-flex h-6 w-11 items-center rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] {}", if push_val.get() { "bg-[var(--accent)]" } else { "bg-[var(--border-subtle)] dark:bg-[var(--text-tertiary)]" })
                                                        on:click=move |_| set_push.set(!push_val.get())
                                                    >
                                                        <span class=move || format!("inline-block h-4 w-4 transform rounded-full bg-[var(--bg-surface)] transition-transform {}", if push_val.get() { "translate-x-6" } else { "translate-x-1" })></span>
                                                    </button>
                                                </div>
                                            </div>
                                        }
                                    }).collect::<Vec<_>>()}
                                </div>
                                <div class="pt-4 border-t border-[var(--border-default)]">
                                    <button on:click=save_prefs class="px-4 py-2 text-sm bg-[var(--accent)] text-[var(--text-on-accent)] brutal-border rounded-sm font-bold uppercase hover:bg-[var(--accent-hover)] transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] min-h-[44px]">{t!("common.save")}</button>
                                </div>
                            </div>
                        })}

                        {/* Appearance Tab */}
                        {move || (tab.get() == SettingsTab::Appearance && !loading.get()).then(|| view! {
                            <div class="p-6 space-y-6">
                                <h2 class="text-section font-mono text-[var(--text-primary)]">{t!("settings.section_appearance")}</h2>
                                <div class="space-y-5">
                                    <fieldset>
                                        <legend class="block text-label font-mono text-[var(--text-secondary)] mb-2">{t!("settings.theme_label")}</legend>
                                        <div class="flex items-center gap-4">
                                            <label class="flex items-center gap-2 cursor-pointer">
                                                <input type="radio" name="theme" value="light" prop:checked=move || !dark_mode.get() on:change=move |ev| { on_theme_change(ev); } aria-label="Light theme" class="text-[var(--accent)] focus:ring-[var(--border-focus)]" />
                                                <span class="text-sm text-[var(--text-secondary)]">{t!("settings.theme_light")}</span>
                                            </label>
                                            <label class="flex items-center gap-2 cursor-pointer">
                                                <input type="radio" name="theme" value="dark" prop:checked=move || dark_mode.get() on:change=move |ev| { on_theme_change(ev); } aria-label="Dark theme" class="text-[var(--accent)] focus:ring-[var(--border-focus)]" />
                                                <span class="text-sm text-[var(--text-secondary)]">{t!("settings.theme_dark")}</span>
                                            </label>
                                            <label class="flex items-center gap-2 cursor-pointer">
                                                <input type="radio" name="theme" value="system" prop:checked=move || prefs.with(|p| p.theme == "system") on:change=move |ev| { on_theme_change(ev); } aria-label="System theme" class="text-[var(--accent)] focus:ring-[var(--border-focus)]" />
                                                <span class="text-sm text-[var(--text-secondary)]">{t!("settings.theme_system")}</span>
                                            </label>
                                        </div>
                                    </fieldset>
                                    <div>
                                        <label class="block text-label font-mono text-[var(--text-secondary)] mb-1" for="language">{t!("settings.language")}</label>
                                        <select id="language" class="w-full px-3 py-2 border rounded bg-[var(--bg-surface)] font-mono text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] text-sm" prop:value=move || language.get() on:change=move |ev| set_language.set(event_target_value(&ev))>
                                            <option value="en">{t!("settings.lang_en")}</option>
                                            <option value="es">{t!("settings.lang_es")}</option>
                                            <option value="fr">{t!("settings.lang_fr")}</option>
                                            <option value="de">{t!("settings.lang_de")}</option>
                                            <option value="ja">{t!("settings.lang_ja")}</option>
                                        </select>
                                    </div>
                                </div>
                                <div class="pt-4 border-t border-[var(--border-default)]">
                                    <button on:click=save_prefs class="px-4 py-2 text-sm bg-[var(--accent)] text-[var(--text-on-accent)] brutal-border rounded-sm font-bold uppercase hover:bg-[var(--accent-hover)] transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] min-h-[44px]">{t!("common.save")}</button>
                                </div>
                            </div>
                        })}

                        {/* Sync Tab */}
                        {move || (tab.get() == SettingsTab::Sync && !loading.get()).then(|| view! {
                            <div class="p-6 space-y-6">
                                <h2 class="text-section font-mono text-[var(--text-primary)]">{t!("settings.section_sync")}</h2>
                                <div class="space-y-5">
                                    <div class="flex items-center justify-between">
                                        <div>
                                            <label class="text-label font-mono text-[var(--text-secondary)]">{t!("settings.offline_mode")}</label>
                                            <p class="text-xs text-[var(--text-tertiary)] mt-0.5">{t!("settings.offline_hint")}</p>
                                        </div>
                                        <button
                                            role="switch"
                                            aria-checked=move || offline_enabled.get()
                                            class=move || format!("relative inline-flex h-6 w-11 items-center rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] {}", if offline_enabled.get() { "bg-[var(--accent)]" } else { "bg-[var(--border-subtle)] dark:bg-[var(--text-tertiary)]" })
                                            on:click=move |_| set_offline_enabled.set(!offline_enabled.get())
                                        >
                                            <span class=move || format!("inline-block h-4 w-4 transform rounded-full bg-[var(--bg-surface)] transition-transform {}", if offline_enabled.get() { "translate-x-6" } else { "translate-x-1" })></span>
                                        </button>
                                    </div>
                                    <div>
                                        <label class="block text-label font-mono text-[var(--text-secondary)] mb-1" for="cache-size">{t!("settings.cache_size")} (MB)</label>
                                        <input id="cache-size" type="number" min="64" max="4096" step="64" prop:value=move || cache_size.get().to_string() on:input=move |ev| { if let Ok(v) = event_target_value(&ev).parse::<u64>() { set_cache_size.set(v); } } class="w-full px-3 py-2 border rounded bg-[var(--bg-surface)] font-mono text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] text-sm" />
                                    </div>
                                </div>
                                <div class="pt-4 border-t border-[var(--border-default)]">
                                    <button on:click=save_prefs class="px-4 py-2 text-sm bg-[var(--accent)] text-[var(--text-on-accent)] brutal-border rounded-sm font-bold uppercase hover:bg-[var(--accent-hover)] transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] min-h-[44px]">{t!("common.save")}</button>
                                </div>
                                <div class="pt-4 border-t border-[var(--border-default)]">
                                    <h3 class="text-label font-mono text-[var(--text-secondary)] mb-3">{t!("settings.section_onboarding")}</h3>
                                    <button
                                        class="px-4 py-2 text-sm text-[var(--text-secondary)] hover:text-[var(--text-primary)] border rounded brutal-border font-bold uppercase hover:bg-[var(--bg-inset)] transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] min-h-[44px]"
                                        on:click=handle_reset_onboarding
                                    >
                                        {t!("settings.reset_tour")}
                                    </button>
                                    <p class="text-xs text-[var(--text-tertiary)] mt-1">{t!("settings.reset_tour_hint")}</p>
                                </div>
                            </div>
                        })}
                    </div>
                </main>
            </div>
        </div>
    }
}
