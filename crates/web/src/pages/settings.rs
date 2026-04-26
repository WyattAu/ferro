use leptos::*;
use leptos_router::A;

use crate::api::{self, UserPreferences};
use crate::components::onboarding::reset_onboarding;
use crate::components::toast::ToastContext;

#[component]
pub fn SettingsPage() -> impl IntoView {
    let (prefs, set_prefs) = create_signal(UserPreferences {
        theme: "dark".to_string(),
        view_mode: "list".to_string(),
        sort_by: "name".to_string(),
        sort_order: "asc".to_string(),
        items_per_page: 50,
        show_hidden_files: false,
        language: "en".to_string(),
    });
    let (loading, set_loading) = create_signal(true);
    let (saving, set_saving) = create_signal(false);

    create_effect(move |_| {
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
                Ok(_) => ToastContext::success("Preferences saved"),
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

    let _on_show_hidden_toggle = move |ev: ev::Event| {
        let checked = event_target_checked(&ev);
        set_prefs.update(|p| p.show_hidden_files = checked);
    };

    let handle_reset_onboarding = move |_: ev::MouseEvent| {
        reset_onboarding();
        ToastContext::info("Onboarding tour has been reset. Reload the page to see it again.");
    };

    view! {
        <div class="min-h-screen bg-gray-100 dark:bg-gray-900 flex flex-col">
            <a href="#main-content" class="sr-only focus:not-sr-only focus:absolute focus:top-2 focus:left-2 focus:z-50 focus:px-4 focus:py-2 focus:bg-blue-600 focus:text-white focus:rounded">"Skip to main content"</a>

            <header class="bg-white dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700 px-6 py-3 shadow-sm">
                <div class="flex items-center justify-between max-w-7xl mx-auto">
                    <div class="flex items-center gap-3">
                        <A href="/ui/" class="flex items-center gap-2 no-underline">
                            <div class="w-8 h-8 bg-blue-600 rounded-lg flex items-center justify-center">
                                <span class="text-white font-bold text-sm">"F"</span>
                            </div>
                            <div>
                                <h1 class="text-lg font-bold text-gray-900 dark:text-gray-100 leading-none">"Ferro"</h1>
                                <span class="text-xs text-gray-500 dark:text-gray-400">"Settings"</span>
                            </div>
                        </A>
                    </div>
                    <A
                        href="/ui/"
                        class="px-3 py-1.5 text-sm text-gray-600 dark:text-gray-400 hover:text-gray-800 dark:hover:text-gray-200 no-underline rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500"
                    >
                        "Back to Files"
                    </A>
                </div>
            </header>

            <main id="main-content" class="flex-1 max-w-2xl w-full mx-auto bg-white dark:bg-gray-800 shadow-sm my-4 rounded-xl overflow-hidden">
                {move || loading.get().then(|| view! {
                    <div class="px-6 py-12 text-center text-gray-500 dark:text-gray-400" role="status" aria-live="polite">
                        <div class="animate-spin w-8 h-8 border-2 border-blue-600 border-t-transparent rounded-full mx-auto mb-3"></div>
                        "Loading preferences..."
                    </div>
                })}

                {move || (!loading.get()).then(|| view! {
                    <div class="p-6 space-y-6">
                        <h2 class="text-xl font-semibold text-gray-900 dark:text-gray-100">"Preferences"</h2>

                        <div class="space-y-5">
                            <fieldset>
                                <legend class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">"Theme"</legend>
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
                                        <span class="text-sm text-gray-700 dark:text-gray-300">"Light"</span>
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
                                        <span class="text-sm text-gray-700 dark:text-gray-300">"Dark"</span>
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
                                        <span class="text-sm text-gray-700 dark:text-gray-300">"System"</span>
                                    </label>
                                </div>
                            </fieldset>

                            <fieldset>
                                <legend class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">"Default View"</legend>
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
                                        <span class="text-sm text-gray-700 dark:text-gray-300">"List"</span>
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
                                        <span class="text-sm text-gray-700 dark:text-gray-300">"Grid"</span>
                                    </label>
                                </div>
                            </fieldset>

                            <div>
                                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1" for="sort-by">"Default Sort"</label>
                                <select
                                    id="sort-by"
                                    class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-blue-500 text-sm"
                                    on:change=on_sort_by_change
                                >
                                    <option value="name" selected=move || prefs.with(|p| p.sort_by == "name")>"Name"</option>
                                    <option value="date" selected=move || prefs.with(|p| p.sort_by == "date")>"Date"</option>
                                    <option value="size" selected=move || prefs.with(|p| p.sort_by == "size")>"Size"</option>
                                </select>
                            </div>

                            <div>
                                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1" for="sort-order">"Sort Order"</label>
                                <select
                                    id="sort-order"
                                    class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-blue-500 text-sm"
                                    on:change=on_sort_order_change
                                >
                                    <option value="asc" selected=move || prefs.with(|p| p.sort_order == "asc")>"Ascending"</option>
                                    <option value="desc" selected=move || prefs.with(|p| p.sort_order == "desc")>"Descending"</option>
                                </select>
                            </div>

                            <div>
                                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1" for="items-per-page">"Items Per Page"</label>
                                <select
                                    id="items-per-page"
                                    class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-blue-500 text-sm"
                                    on:change=on_items_per_page_change
                                >
                                    <option value="25" selected=move || prefs.with(|p| p.items_per_page == 25)>"25"</option>
                                    <option value="50" selected=move || prefs.with(|p| p.items_per_page == 50)>"50"</option>
                                    <option value="100" selected=move || prefs.with(|p| p.items_per_page == 100)>"100"</option>
                                </select>
                            </div>

                            <div class="flex items-center justify-between">
                                <label class="text-sm font-medium text-gray-700 dark:text-gray-300" for="show-hidden">"Show Hidden Files"</label>
                                <button
                                    id="show-hidden"
                                    role="switch"
                                    aria-checked=move || prefs.with(|p| p.show_hidden_files)
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

                        <div class="pt-4 border-t border-gray-200 dark:border-gray-700">
                            <button
                                class="px-4 py-2 text-sm bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 dark:focus:ring-offset-gray-800 min-h-[44px]"
                                disabled=saving
                                on:click=save_prefs
                            >
                                {move || if saving.get() { "Saving..." } else { "Save" }}
                            </button>
                        </div>

                        <div class="pt-4 border-t border-gray-200 dark:border-gray-700">
                            <h3 class="text-sm font-medium text-gray-700 dark:text-gray-300 mb-3">"Onboarding"</h3>
                            <button
                                class="px-4 py-2 text-sm text-gray-600 dark:text-gray-400 hover:text-gray-800 dark:hover:text-gray-200 border border-gray-300 dark:border-gray-600 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 dark:focus:ring-offset-gray-800 min-h-[44px]"
                                on:click=handle_reset_onboarding
                            >
                                "Reset Onboarding Tour"
                            </button>
                            <p class="text-xs text-gray-400 dark:text-gray-500 mt-1">"Show the introductory tour again on next page load"</p>
                        </div>
                    </div>
                })}
            </main>
        </div>
    }
}
