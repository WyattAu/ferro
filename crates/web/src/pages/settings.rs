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

    // Pre-extract all t!() calls for the top-level view! block
    let lbl_skip_to_content = t!("nav.skip_to_content");
    let lbl_brand_name = t!("brand.name");
    let lbl_title = t!("settings.title");
    let lbl_back_to_files = t!("nav.back_to_files");
    let lbl_tab_account = t!("settings.tab_account");
    let lbl_tab_preferences = t!("settings.tab_preferences");
    let lbl_tab_notifications = t!("settings.tab_notifications");
    let lbl_tab_appearance = t!("settings.tab_appearance");
    let lbl_tab_sync = t!("settings.tab_sync");
    let lbl_loading_prefs = t!("settings.loading_prefs");
    let lbl_section_account = t!("settings.section_account");
    let lbl_profile_name = t!("settings.profile_name");
    let lbl_profile_email = t!("settings.profile_email");
    let lbl_save = t!("common.save");
    let lbl_change_password = t!("settings.change_password");
    let lbl_current_password = t!("settings.current_password");
    let lbl_new_password = t!("settings.new_password");
    let lbl_confirm_password = t!("settings.confirm_password");
    let lbl_section_prefs = t!("settings.section_prefs");
    let lbl_default_view_label = t!("settings.default_view_label");
    let lbl_view_list = t!("settings.view_list");
    let lbl_view_grid = t!("settings.view_grid");
    let lbl_default_sort_label = t!("settings.default_sort_label");
    let lbl_sort_name = t!("settings.sort_name");
    let lbl_sort_date = t!("settings.sort_date");
    let lbl_sort_size = t!("settings.sort_size");
    let lbl_sort_order_label = t!("settings.sort_order_label");
    let lbl_sort_ascending = t!("settings.sort_ascending");
    let lbl_sort_descending = t!("settings.sort_descending");
    let lbl_items_per_page_label = t!("settings.items_per_page_label");
    let lbl_show_hidden_label = t!("settings.show_hidden_label");
    let lbl_saving = t!("common.saving");
    let lbl_section_notifications = t!("settings.section_notifications");
    let lbl_email = t!("settings.email");
    let lbl_push = t!("settings.push");
    let lbl_section_appearance = t!("settings.section_appearance");
    let lbl_theme_label = t!("settings.theme_label");
    let lbl_theme_light = t!("settings.theme_light");
    let lbl_theme_dark = t!("settings.theme_dark");
    let lbl_theme_system = t!("settings.theme_system");
    let lbl_language = t!("settings.language");
    let lbl_lang_en = t!("settings.lang_en");
    let lbl_lang_es = t!("settings.lang_es");
    let lbl_lang_fr = t!("settings.lang_fr");
    let lbl_lang_de = t!("settings.lang_de");
    let lbl_lang_ja = t!("settings.lang_ja");
    let lbl_section_sync = t!("settings.section_sync");
    let lbl_offline_mode = t!("settings.offline_mode");
    let lbl_offline_hint = t!("settings.offline_hint");
    let lbl_cache_size = t!("settings.cache_size");
    let lbl_section_onboarding = t!("settings.section_onboarding");
    let lbl_reset_tour = t!("settings.reset_tour");
    let lbl_reset_tour_hint = t!("settings.reset_tour_hint");
    let lbl_event_shares = t!("settings.event_shares");
    let lbl_event_comments = t!("settings.event_comments");
    let lbl_event_uploads = t!("settings.event_uploads");

    view! {
        <div class="h-screen flex flex-col bg-[var(--bg-base)]">
            <a href="#main-content" class="sr-only focus:not-sr-only focus:absolute focus:top-2 focus:left-2 focus:z-50 focus:px-4 focus:py-2 focus:bg-[var(--accent)] focus:text-[var(--text-on-accent)] focus:rounded">{lbl_skip_to_content}</a>

            <header class="surface brutal-border border-b px-6 py-3 shadow-concrete">
                <div class="flex items-center justify-between max-w-7xl mx-auto">
                    <div class="flex items-center gap-3">
                        <A href="/ui/" attr:class="flex items-center gap-2 no-underline">
                            <div class="w-8 h-8 bg-transparent brutal-border rounded flex items-center justify-center font-display text-accent">
                                <span class="font-bold text-sm">{lbl_brand_name}</span>
                            </div>
                            <div>
                                <h1 class="text-lg font-bold font-mono text-[var(--text-primary)] leading-none">{lbl_brand_name}</h1>
                                <span class="text-label text-muted">{lbl_title}</span>
                            </div>
                        </A>
                    </div>
                    <nav aria-label=lbl_back_to_files class="flex items-center gap-2">
                        <A
                            href="/ui/"
                            attr:class="px-3 py-1.5 text-sm text-[var(--text-secondary)] hover:text-[var(--text-primary)] no-underline rounded hover:bg-[var(--bg-inset)] transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)]"
                        >
                            {lbl_back_to_files}
                        </A>
                    </nav>
                </div>
            </header>

            <div class="flex-1 flex overflow-hidden">
                <NavigationSidebar />
                <main id="main-content" class="flex-1 overflow-auto p-6">
                    {/* Tab Navigation */}
                    <div class="flex flex-wrap items-center gap-1 mb-6" role="tablist" aria-label="Settings tabs">
                        <button role="tab" aria-selected=move || tab.get() == SettingsTab::Account aria-controls="panel-account" id="tab-account" on:click=move |_| set_tab.set(SettingsTab::Account) class=move || format!("px-4 py-2 text-sm font-medium rounded-lg transition-colors {}", if tab.get() == SettingsTab::Account { "bg-[var(--accent)] text-[var(--text-on-accent)]" } else { "text-[var(--text-secondary)] dark:text-[var(--text-tertiary)] hover:bg-[var(--interactive-hover)]" })>{lbl_tab_account}</button>
                        <button role="tab" aria-selected=move || tab.get() == SettingsTab::Preferences aria-controls="panel-preferences" id="tab-preferences" on:click=move |_| set_tab.set(SettingsTab::Preferences) class=move || format!("px-4 py-2 text-sm font-medium rounded-lg transition-colors {}", if tab.get() == SettingsTab::Preferences { "bg-[var(--accent)] text-[var(--text-on-accent)]" } else { "text-[var(--text-secondary)] dark:text-[var(--text-tertiary)] hover:bg-[var(--interactive-hover)]" })>{lbl_tab_preferences}</button>
                        <button role="tab" aria-selected=move || tab.get() == SettingsTab::Notifications aria-controls="panel-notifications" id="tab-notifications" on:click=move |_| set_tab.set(SettingsTab::Notifications) class=move || format!("px-4 py-2 text-sm font-medium rounded-lg transition-colors {}", if tab.get() == SettingsTab::Notifications { "bg-[var(--accent)] text-[var(--text-on-accent)]" } else { "text-[var(--text-secondary)] dark:text-[var(--text-tertiary)] hover:bg-[var(--interactive-hover)]" })>{lbl_tab_notifications}</button>
                        <button role="tab" aria-selected=move || tab.get() == SettingsTab::Appearance aria-controls="panel-appearance" id="tab-appearance" on:click=move |_| set_tab.set(SettingsTab::Appearance) class=move || format!("px-4 py-2 text-sm font-medium rounded-lg transition-colors {}", if tab.get() == SettingsTab::Appearance { "bg-[var(--accent)] text-[var(--text-on-accent)]" } else { "text-[var(--text-secondary)] dark:text-[var(--text-tertiary)] hover:bg-[var(--interactive-hover)]" })>{lbl_tab_appearance}</button>
                        <button role="tab" aria-selected=move || tab.get() == SettingsTab::Sync aria-controls="panel-sync" id="tab-sync" on:click=move |_| set_tab.set(SettingsTab::Sync) class=move || format!("px-4 py-2 text-sm font-medium rounded-lg transition-colors {}", if tab.get() == SettingsTab::Sync { "bg-[var(--accent)] text-[var(--text-on-accent)]" } else { "text-[var(--text-secondary)] dark:text-[var(--text-tertiary)] hover:bg-[var(--interactive-hover)]" })>{lbl_tab_sync}</button>
                    </div>

                    {move || loading.get().then(|| {
                        let msg = lbl_loading_prefs;
                        view! {
                            <div class="px-6 py-12 text-center text-[var(--text-tertiary)]" role="status" aria-live="polite">
                                <div class="animate-spin w-8 h-8 border-2 border-blue-600 border-t-transparent rounded-full mx-auto mb-3"></div>
                                {msg}
                            </div>
                        }
                    })}

                    <div class="max-w-2xl w-full surface brutal-border shadow-concrete rounded-lg overflow-hidden">
                        {/* Account Tab */}
                        {move || (tab.get() == SettingsTab::Account && !loading.get()).then(|| {
                            let s_account = lbl_section_account;
                            let s_pname = lbl_profile_name;
                            let s_pemail = lbl_profile_email;
                            let s_save = lbl_save;
                            let s_chpw = lbl_change_password;
                            let s_cpw = lbl_current_password;
                            let s_npw = lbl_new_password;
                            let s_cpw2 = lbl_confirm_password;
                            view! {
                                <div id="panel-account" role="tabpanel" aria-labelledby="tab-account" class="p-6 space-y-6">
                                    <h2 class="text-section font-mono text-[var(--text-primary)]">{s_account}</h2>
                                    <div class="space-y-4">
                                        <div>
                                            <label class="block text-label font-mono text-[var(--text-secondary)] mb-1" for="profile-name">{s_pname}</label>
                                            <input id="profile-name" type="text" prop:value=move || profile_name.get() on:input=move |ev| set_profile_name.set(event_target_value(&ev)) class="w-full px-3 py-2 border rounded bg-[var(--bg-surface)] font-mono text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] text-sm" />
                                        </div>
                                        <div>
                                            <label class="block text-label font-mono text-[var(--text-secondary)] mb-1" for="profile-email">{s_pemail}</label>
                                            <input id="profile-email" type="email" prop:value=move || profile_email.get() on:input=move |ev| set_profile_email.set(event_target_value(&ev)) class="w-full px-3 py-2 border rounded bg-[var(--bg-surface)] font-mono text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] text-sm" />
                                        </div>
                                        <div class="pt-4 border-t border-[var(--border-default)]">
                                            <button on:click=save_account class="px-4 py-2 text-sm bg-[var(--accent)] text-[var(--text-on-accent)] brutal-border rounded-sm font-bold uppercase hover:bg-[var(--accent-hover)] disabled:opacity-50 disabled:cursor-not-allowed transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] min-h-[44px]">{s_save}</button>
                                        </div>
                                    </div>
                                    <div class="pt-4 border-t border-[var(--border-default)]">
                                        <h3 class="text-label font-mono text-[var(--text-secondary)] mb-3">{s_chpw}</h3>
                                        <div class="space-y-3">
                                            <input type="password" placeholder=s_cpw prop:value=move || current_password.get() on:input=move |ev| set_current_password.set(event_target_value(&ev)) class="w-full px-3 py-2 border rounded bg-[var(--bg-surface)] font-mono text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] text-sm" />
                                            <input type="password" placeholder=s_npw prop:value=move || new_password.get() on:input=move |ev| set_new_password.set(event_target_value(&ev)) class="w-full px-3 py-2 border rounded bg-[var(--bg-surface)] font-mono text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] text-sm" />
                                            <input type="password" placeholder=s_cpw2 prop:value=move || confirm_password.get() on:input=move |ev| set_confirm_password.set(event_target_value(&ev)) class="w-full px-3 py-2 border rounded bg-[var(--bg-surface)] font-mono text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] text-sm" />
                                            <button on:click=change_password class="px-4 py-2 text-sm bg-[var(--accent)] text-[var(--text-on-accent)] brutal-border rounded-sm font-bold uppercase hover:bg-[var(--accent-hover)] transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] min-h-[44px]">{s_chpw}</button>
                                        </div>
                                    </div>
                                </div>
                            }
                        })}

                        {/* Preferences Tab */}
                        {move || (tab.get() == SettingsTab::Preferences && !loading.get()).then(|| {
                            let s_prefs = lbl_section_prefs;
                            let s_dvl = lbl_default_view_label;
                            let s_vl = lbl_view_list;
                            let s_vg = lbl_view_grid;
                            let s_dsl = lbl_default_sort_label;
                            let s_sn = lbl_sort_name;
                            let s_sd = lbl_sort_date;
                            let s_ss = lbl_sort_size;
                            let s_sol = lbl_sort_order_label;
                            let s_sa = lbl_sort_ascending;
                            let s_sde = lbl_sort_descending;
                            let s_ippl = lbl_items_per_page_label;
                            let s_shl = lbl_show_hidden_label;
                            let s_sav = lbl_save;
                            let s_sav2 = lbl_saving;
                            view! {
                                <div id="panel-preferences" role="tabpanel" aria-labelledby="tab-preferences" class="p-6 space-y-6">
                                    <h2 class="text-section font-mono text-[var(--text-primary)]">{s_prefs}</h2>
                                    <div class="space-y-5">
                                        <fieldset>
                                            <legend class="block text-label font-mono text-[var(--text-secondary)] mb-2">{s_dvl}</legend>
                                            <div class="flex items-center gap-4">
                                                <label class="flex items-center gap-2 cursor-pointer">
                                                    <input type="radio" name="view_mode" value="list" prop:checked=move || prefs.with(|p| p.view_mode == "list") on:change=on_view_mode_change aria-label="List view" class="text-[var(--accent)] focus:ring-[var(--border-focus)]" />
                                                    <span class="text-sm text-[var(--text-secondary)]">{s_vl}</span>
                                                </label>
                                                <label class="flex items-center gap-2 cursor-pointer">
                                                    <input type="radio" name="view_mode" value="grid" prop:checked=move || prefs.with(|p| p.view_mode == "grid") on:change=on_view_mode_change aria-label="Grid view" class="text-[var(--accent)] focus:ring-[var(--border-focus)]" />
                                                    <span class="text-sm text-[var(--text-secondary)]">{s_vg}</span>
                                                </label>
                                            </div>
                                        </fieldset>
                                        <div>
                                            <label class="block text-label font-mono text-[var(--text-secondary)] mb-1" for="sort-by">{s_dsl}</label>
                                            <select id="sort-by" class="w-full px-3 py-2 border rounded bg-[var(--bg-surface)] font-mono text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] text-sm" on:change=on_sort_by_change>
                                                <option value="name" selected=move || prefs.with(|p| p.sort_by == "name")>{s_sn}</option>
                                                <option value="date" selected=move || prefs.with(|p| p.sort_by == "date")>{s_sd}</option>
                                                <option value="size" selected=move || prefs.with(|p| p.sort_by == "size")>{s_ss}</option>
                                            </select>
                                        </div>
                                        <div>
                                            <label class="block text-label font-mono text-[var(--text-secondary)] mb-1" for="sort-order">{s_sol}</label>
                                            <select id="sort-order" class="w-full px-3 py-2 border rounded bg-[var(--bg-surface)] font-mono text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] text-sm" on:change=on_sort_order_change>
                                                <option value="asc" selected=move || prefs.with(|p| p.sort_order == "asc")>{s_sa}</option>
                                                <option value="desc" selected=move || prefs.with(|p| p.sort_order == "desc")>{s_sde}</option>
                                            </select>
                                        </div>
                                        <div>
                                            <label class="block text-label font-mono text-[var(--text-secondary)] mb-1" for="items-per-page">{s_ippl}</label>
                                            <select id="items-per-page" class="w-full px-3 py-2 border rounded bg-[var(--bg-surface)] font-mono text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] text-sm" on:change=on_items_per_page_change>
                                                <option value="25" selected=move || prefs.with(|p| p.items_per_page == 25)>"25"</option>
                                                <option value="50" selected=move || prefs.with(|p| p.items_per_page == 50)>"50"</option>
                                                <option value="100" selected=move || prefs.with(|p| p.items_per_page == 100)>"100"</option>
                                            </select>
                                        </div>
                                        <div class="flex items-center justify-between">
                                            <label class="text-label font-mono text-[var(--text-secondary)]" for="show-hidden">{s_shl}</label>
                                            <button
                                                id="show-hidden"
                                                role="switch"
                                                aria-checked=move || prefs.with(|p| p.show_hidden_files)
                                                aria-label=move || lbl_show_hidden_label
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
                                            {move || if saving.get() { s_sav2 } else { s_sav }}
                                        </button>
                                    </div>
                                </div>
                            }
                        })}

                        {/* Notifications Tab */}
                        {move || (tab.get() == SettingsTab::Notifications && !loading.get()).then(|| {
                            let s_notif = lbl_section_notifications;
                            let s_email = lbl_email;
                            let s_push = lbl_push;
                            let s_save = lbl_save;
                            let s_shares = lbl_event_shares;
                            let s_comments = lbl_event_comments;
                            let s_uploads = lbl_event_uploads;
                            view! {
                                <div id="panel-notifications" role="tabpanel" aria-labelledby="tab-notifications" class="p-6 space-y-6">
                                    <h2 class="text-section font-mono text-[var(--text-primary)]">{s_notif}</h2>
                                    <div class="space-y-4">
                                        <div class="grid grid-cols-3 gap-4 text-xs font-bold uppercase font-mono text-[var(--text-tertiary)]">
                                            <div></div>
                                            <div class="text-center">{s_email}</div>
                                            <div class="text-center">{s_push}</div>
                                        </div>
                                        {vec![
                                            ("shares", s_shares, notify_email_shares, set_notify_email_shares, notify_push_shares, set_notify_push_shares),
                                            ("comments", s_comments, notify_email_comments, set_notify_email_comments, notify_push_comments, set_notify_push_comments),
                                            ("uploads", s_uploads, notify_email_uploads, set_notify_email_uploads, notify_push_uploads, set_notify_push_uploads),
                                        ].into_iter().map(|(_key, label_text, email_val, set_email, push_val, set_push)| {
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
                                        <button on:click=save_prefs class="px-4 py-2 text-sm bg-[var(--accent)] text-[var(--text-on-accent)] brutal-border rounded-sm font-bold uppercase hover:bg-[var(--accent-hover)] transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] min-h-[44px]">{s_save}</button>
                                    </div>
                                </div>
                            }
                        })}

                        {/* Appearance Tab */}
                        {move || (tab.get() == SettingsTab::Appearance && !loading.get()).then(|| {
                            let s_appear = lbl_section_appearance;
                            let s_tl = lbl_theme_label;
                            let s_tlight = lbl_theme_light;
                            let s_tdark = lbl_theme_dark;
                            let s_tsys = lbl_theme_system;
                            let s_lang = lbl_language;
                            let s_en = lbl_lang_en;
                            let s_es = lbl_lang_es;
                            let s_fr = lbl_lang_fr;
                            let s_de = lbl_lang_de;
                            let s_ja = lbl_lang_ja;
                            let s_save = lbl_save;
                            view! {
                                <div id="panel-appearance" role="tabpanel" aria-labelledby="tab-appearance" class="p-6 space-y-6">
                                    <h2 class="text-section font-mono text-[var(--text-primary)]">{s_appear}</h2>
                                    <div class="space-y-5">
                                        <fieldset>
                                            <legend class="block text-label font-mono text-[var(--text-secondary)] mb-2">{s_tl}</legend>
                                            <div class="flex items-center gap-4">
                                                <label class="flex items-center gap-2 cursor-pointer">
                                                    <input type="radio" name="theme" value="light" prop:checked=move || !dark_mode.get() on:change=move |ev| { on_theme_change(ev); } aria-label="Light theme" class="text-[var(--accent)] focus:ring-[var(--border-focus)]" />
                                                    <span class="text-sm text-[var(--text-secondary)]">{s_tlight}</span>
                                                </label>
                                                <label class="flex items-center gap-2 cursor-pointer">
                                                    <input type="radio" name="theme" value="dark" prop:checked=move || dark_mode.get() on:change=move |ev| { on_theme_change(ev); } aria-label="Dark theme" class="text-[var(--accent)] focus:ring-[var(--border-focus)]" />
                                                    <span class="text-sm text-[var(--text-secondary)]">{s_tdark}</span>
                                                </label>
                                                <label class="flex items-center gap-2 cursor-pointer">
                                                    <input type="radio" name="theme" value="system" prop:checked=move || prefs.with(|p| p.theme == "system") on:change=move |ev| { on_theme_change(ev); } aria-label="System theme" class="text-[var(--accent)] focus:ring-[var(--border-focus)]" />
                                                    <span class="text-sm text-[var(--text-secondary)]">{s_tsys}</span>
                                                </label>
                                            </div>
                                        </fieldset>
                                        <div>
                                            <label class="block text-label font-mono text-[var(--text-secondary)] mb-1" for="language">{s_lang}</label>
                                            <select id="language" class="w-full px-3 py-2 border rounded bg-[var(--bg-surface)] font-mono text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] text-sm" prop:value=move || language.get() on:change=move |ev| set_language.set(event_target_value(&ev))>
                                                <option value="en">{s_en}</option>
                                                <option value="es">{s_es}</option>
                                                <option value="fr">{s_fr}</option>
                                                <option value="de">{s_de}</option>
                                                <option value="ja">{s_ja}</option>
                                            </select>
                                        </div>
                                    </div>
                                    <div class="pt-4 border-t border-[var(--border-default)]">
                                        <button on:click=save_prefs class="px-4 py-2 text-sm bg-[var(--accent)] text-[var(--text-on-accent)] brutal-border rounded-sm font-bold uppercase hover:bg-[var(--accent-hover)] transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] min-h-[44px]">{s_save}</button>
                                    </div>
                                </div>
                            }
                        })}

                        {/* Sync Tab */}
                        {move || (tab.get() == SettingsTab::Sync && !loading.get()).then(|| {
                            let s_sync = lbl_section_sync;
                            let s_offline = lbl_offline_mode;
                            let s_offhint = lbl_offline_hint;
                            let s_csize = lbl_cache_size;
                            let s_save = lbl_save;
                            let s_onboard = lbl_section_onboarding;
                            let s_rtour = lbl_reset_tour;
                            let s_rhint = lbl_reset_tour_hint;
                            view! {
                                <div id="panel-sync" role="tabpanel" aria-labelledby="tab-sync" class="p-6 space-y-6">
                                    <h2 class="text-section font-mono text-[var(--text-primary)]">{s_sync}</h2>
                                    <div class="space-y-5">
                                        <div class="flex items-center justify-between">
                                            <div>
                                                <label class="text-label font-mono text-[var(--text-secondary)]">{s_offline}</label>
                                                <p class="text-xs text-[var(--text-tertiary)] mt-0.5">{s_offhint}</p>
                                            </div>
                                            <button
                                                role="switch"
                                                aria-checked=move || offline_enabled.get()
                                                aria-label="Offline mode"
                                                class=move || format!("relative inline-flex h-6 w-11 items-center rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] {}", if offline_enabled.get() { "bg-[var(--accent)]" } else { "bg-[var(--border-subtle)] dark:bg-[var(--text-tertiary)]" })
                                                on:click=move |_| set_offline_enabled.set(!offline_enabled.get())
                                            >
                                                <span class=move || format!("inline-block h-4 w-4 transform rounded-full bg-[var(--bg-surface)] transition-transform {}", if offline_enabled.get() { "translate-x-6" } else { "translate-x-1" })></span>
                                            </button>
                                        </div>
                                        <div>
                                            <label class="block text-label font-mono text-[var(--text-secondary)] mb-1" for="cache-size">{s_csize} (MB)</label>
                                            <input id="cache-size" type="number" min="64" max="4096" step="64" prop:value=move || cache_size.get().to_string() on:input=move |ev| { if let Ok(v) = event_target_value(&ev).parse::<u64>() { set_cache_size.set(v); } } class="w-full px-3 py-2 border rounded bg-[var(--bg-surface)] font-mono text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] text-sm" />
                                        </div>
                                    </div>
                                    <div class="pt-4 border-t border-[var(--border-default)]">
                                        <button on:click=save_prefs class="px-4 py-2 text-sm bg-[var(--accent)] text-[var(--text-on-accent)] brutal-border rounded-sm font-bold uppercase hover:bg-[var(--accent-hover)] transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] min-h-[44px]">{s_save}</button>
                                    </div>
                                    <div class="pt-4 border-t border-[var(--border-default)]">
                                        <h3 class="text-label font-mono text-[var(--text-secondary)] mb-3">{s_onboard}</h3>
                                        <button
                                            class="px-4 py-2 text-sm text-[var(--text-secondary)] hover:text-[var(--text-primary)] border rounded brutal-border font-bold uppercase hover:bg-[var(--bg-inset)] transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] min-h-[44px]"
                                            on:click=handle_reset_onboarding
                                        >
                                            {s_rtour}
                                        </button>
                                        <p class="text-xs text-[var(--text-tertiary)] mt-1">{s_rhint}</p>
                                    </div>
                                </div>
                            }
                        })}
                    </div>
                </main>
            </div>
        </div>
    }
}
