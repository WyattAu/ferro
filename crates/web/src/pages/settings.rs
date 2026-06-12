use leptos::ev;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::A;

use crate::api::{self, UserPreferences};
use crate::components::onboarding::reset_onboarding;
use crate::components::toast::ToastContext;
use crate::t;

#[component]
pub fn SettingsPage() -> impl IntoView {
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

    Effect::new(move |_| {
        spawn_local(async move {
            if let Ok(p) = api::get_preferences().await {
                set_prefs.set(p);
            }
            set_loading.set(false);
        });
    });

    let save_prefs = move |_: ev::MouseEvent| {
        set_saving.set(true);
        let p = prefs.get();
        spawn_local(async move {
            match api::update_preferences(&p).await {
                Ok(_) => ToastContext::success(t!("toast.preferences_saved")),
                Err(e) => ToastContext::error(format!("Failed to save: {}", e)),
            }
            set_saving.set(false);
        });
    };

    let on_theme_change = move |ev: ev::Event| {
        let v = event_target_value(&ev);
        set_prefs.update(|p| p.theme = v);
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

    // Toggle handler for show-hidden-files is wired inline in the view below

    let handle_reset_onboarding = move |_: ev::MouseEvent| {
        reset_onboarding();
        ToastContext::info(t!("toast.onboarding_reset"));
    };

    view! {
        <div class="min-h-screen bg-gray-100 dark:bg-gray-900 flex flex-col">
            <a href="#main-content" class="sr-only focus:not-sr-only focus:absolute focus:top-2 focus:left-2 focus:z-50 focus:px-4 focus:py-2 focus:bg-blue-600 focus:text-white focus:rounded brutal-border">{t!("nav.skip_to_content")}</a>

            <header class="surface brutal-border border-b px-6 py-3 shadow-concrete">
                <div class="flex items-center justify-between max-w-7xl mx-auto">
                    <div class="flex items-center gap-3">
                        <A href="/ui/" attr:class="flex items-center gap-2 no-underline">
                            <div class="w-8 h-8 bg-transparent brutal-border rounded flex items-center justify-center font-display text-accent">
                                <span class="font-bold text-sm">{t!("brand.name")}</span>
                            </div>
                            <div>
                                <h1 class="text-lg font-bold font-mono text-gray-900 leading-none">{t!("brand.name")}</h1>
                                <span class="text-label text-muted">{t!("settings.title")}</span>
                            </div>
                        </A>
                    </div>
                    <nav aria-label=t!("nav.back_to_files") class="flex items-center gap-2">
                        <A
                            href="/ui/"
                            attr:class="px-3 py-1.5 text-sm text-gray-600 hover:text-gray-800 no-underline rounded hover:bg-gray-100 transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500"
                        >
                            {t!("nav.back_to_files")}
                        </A>
                    </nav>
                </div>
            </header>

            <main id="main-content" class="flex-1 max-w-2xl w-full mx-auto surface brutal-border shadow-concrete my-4 rounded-lg overflow-hidden">
                {move || loading.get().then(|| view! {
                    <div class="px-6 py-12 text-center text-gray-500" role="status" aria-live="polite">
                        <div class="animate-spin w-8 h-8 border-2 border-blue-600 border-t-transparent rounded-full mx-auto mb-3"></div>
                        {t!("settings.loading_prefs")}
                    </div>
                })}

                {move || (!loading.get()).then(|| view! {
                    <div class="p-6 space-y-6">
                        <h2 class="text-section font-mono text-gray-900">{t!("settings.section_prefs")}</h2>

                        <div class="space-y-5">
                            <fieldset>
                                <legend class="block text-label font-mono text-gray-700 mb-2">{t!("settings.theme_label")}</legend>
                                <div class="flex items-center gap-4">
                                    <label class="flex items-center gap-2 cursor-pointer">
                                        <input
                                            type="radio"
                                            name="theme"
                                            value="light"
                                            prop:checked=move || prefs.with(|p| p.theme == "light")
                                            on:change=on_theme_change
                                            class="text-blue-600 focus:ring-blue-500"
                                        />
                                        <span class="text-sm text-gray-700">{t!("settings.theme_light")}</span>
                                    </label>
                                    <label class="flex items-center gap-2 cursor-pointer">
                                        <input
                                            type="radio"
                                            name="theme"
                                            value="dark"
                                            prop:checked=move || prefs.with(|p| p.theme == "dark")
                                            on:change=on_theme_change
                                            class="text-blue-600 focus:ring-blue-500"
                                        />
                                        <span class="text-sm text-gray-700">{t!("settings.theme_dark")}</span>
                                    </label>
                                    <label class="flex items-center gap-2 cursor-pointer">
                                        <input
                                            type="radio"
                                            name="theme"
                                            value="system"
                                            prop:checked=move || prefs.with(|p| p.theme == "system")
                                            on:change=on_theme_change
                                            class="text-blue-600 focus:ring-blue-500"
                                        />
                                        <span class="text-sm text-gray-700">{t!("settings.theme_system")}</span>
                                    </label>
                                </div>
                            </fieldset>

                            <fieldset>
                                <legend class="block text-label font-mono text-gray-700 mb-2">{t!("settings.default_view_label")}</legend>
                                <div class="flex items-center gap-4">
                                    <label class="flex items-center gap-2 cursor-pointer">
                                        <input
                                            type="radio"
                                            name="view_mode"
                                            value="list"
                                            prop:checked=move || prefs.with(|p| p.view_mode == "list")
                                            on:change=on_view_mode_change
                                            class="text-blue-600 focus:ring-blue-500"
                                        />
                                        <span class="text-sm text-gray-700">{t!("settings.view_list")}</span>
                                    </label>
                                    <label class="flex items-center gap-2 cursor-pointer">
                                        <input
                                            type="radio"
                                            name="view_mode"
                                            value="grid"
                                            prop:checked=move || prefs.with(|p| p.view_mode == "grid")
                                            on:change=on_view_mode_change
                                            class="text-blue-600 focus:ring-blue-500"
                                        />
                                        <span class="text-sm text-gray-700">{t!("settings.view_grid")}</span>
                                    </label>
                                </div>
                            </fieldset>

                            <div>
                                <label class="block text-label font-mono text-gray-700 mb-1" for="sort-by">{t!("settings.default_sort_label")}</label>
                                <select
                                    id="sort-by"
                                    class="w-full px-3 py-2 border rounded bg-white dark:bg-gray-800 font-mono text-gray-900 focus:outline-none focus:ring-2 focus:ring-blue-500 text-sm"
                                    on:change=on_sort_by_change
                                >
                                    <option value="name" selected=move || prefs.with(|p| p.sort_by == "name")>{t!("settings.sort_name")}</option>
                                    <option value="date" selected=move || prefs.with(|p| p.sort_by == "date")>{t!("settings.sort_date")}</option>
                                    <option value="size" selected=move || prefs.with(|p| p.sort_by == "size")>{t!("settings.sort_size")}</option>
                                </select>
                            </div>

                            <div>
                                <label class="block text-label font-mono text-gray-700 mb-1" for="sort-order">{t!("settings.sort_order_label")}</label>
                                <select
                                    id="sort-order"
                                    class="w-full px-3 py-2 border rounded bg-white dark:bg-gray-800 font-mono text-gray-900 focus:outline-none focus:ring-2 focus:ring-blue-500 text-sm"
                                    on:change=on_sort_order_change
                                >
                                    <option value="asc" selected=move || prefs.with(|p| p.sort_order == "asc")>{t!("settings.sort_ascending")}</option>
                                    <option value="desc" selected=move || prefs.with(|p| p.sort_order == "desc")>{t!("settings.sort_descending")}</option>
                                </select>
                            </div>

                            <div>
                                <label class="block text-label font-mono text-gray-700 mb-1" for="items-per-page">{t!("settings.items_per_page_label")}</label>
                                <select
                                    id="items-per-page"
                                    class="w-full px-3 py-2 border rounded bg-white dark:bg-gray-800 font-mono text-gray-900 focus:outline-none focus:ring-2 focus:ring-blue-500 text-sm"
                                    on:change=on_items_per_page_change
                                >
                                    <option value="25" selected=move || prefs.with(|p| p.items_per_page == 25)>"25"</option>
                                    <option value="50" selected=move || prefs.with(|p| p.items_per_page == 50)>"50"</option>
                                    <option value="100" selected=move || prefs.with(|p| p.items_per_page == 100)>"100"</option>
                                </select>
                            </div>

                            <div class="flex items-center justify-between">
                                <label class="text-label font-mono text-gray-700" for="show-hidden">{t!("settings.show_hidden_label")}</label>
                                <button
                                    id="show-hidden"
                                    role="switch"
                                    aria-checked=move || prefs.with(|p| p.show_hidden_files)
                                    aria-label=move || t!("settings.show_hidden_label")
                                    class=move || format!(
                                        "relative inline-flex h-6 w-11 items-center rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 dark:focus:ring-offset-gray-800 {}",
                                        if prefs.with(|p| p.show_hidden_files) { "bg-blue-600" } else { "bg-gray-200 dark:bg-gray-600" }
                                    )
                                    on:click=move |_| {
                                        let current = prefs.with(|p| p.show_hidden_files);
                                        set_prefs.update(|p| p.show_hidden_files = !current);
                                    }
                                >
                                    <span
                                        class=move || format!(
                                            "inline-block h-4 w-4 transform rounded-full bg-white transition-transform {}",
                                            if prefs.with(|p| p.show_hidden_files) { "translate-x-6" } else { "translate-x-1" }
                                        )
                                    ></span>
                                </button>
                            </div>
                        </div>

                        <div class="pt-4 border-t border-gray-200">
                            <button
                                class="px-4 py-2 text-sm bg-blue-600 text-white brutal-border rounded-sm font-bold uppercase hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 dark:focus:ring-offset-gray-800 min-h-[44px]"
                                disabled=saving
                                on:click=save_prefs
                            >
                                {move || if saving.get() { t!("common.saving") } else { t!("common.save") }}
                            </button>
                        </div>

                        <div class="pt-4 border-t border-gray-200">
                            <h3 class="text-label font-mono text-gray-700 mb-3">{t!("settings.section_onboarding")}</h3>
                            <button
                                class="px-4 py-2 text-sm text-gray-600 hover:text-gray-800 border rounded brutal-border font-bold uppercase hover:bg-gray-50 transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 dark:focus:ring-offset-gray-800 min-h-[44px]"
                                on:click=handle_reset_onboarding
                            >
                                {t!("settings.reset_tour")}
                            </button>
                            <p class="text-xs text-gray-400 mt-1">{t!("settings.reset_tour_hint")}</p>
                        </div>
                    </div>
                })}
            </main>
        </div>
    }
}
