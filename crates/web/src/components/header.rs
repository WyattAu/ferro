use leptos::ev;
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::api::{SearchFilters, SearchResultEntry};
use crate::auth;
use crate::components::theme_toggle::ThemeToggle;
use crate::t;
use ferro_common::format::format_size;
use leptos_router::components::A;

const MAX_RECENT_SEARCHES: usize = 8;
const RECENT_SEARCHES_KEY: &str = "ferro_recent_searches";

fn load_recent_searches() -> Vec<String> {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                if let Ok(Some(json)) = storage.get_item(RECENT_SEARCHES_KEY) {
                    if let Ok(list) = serde_json::from_str::<Vec<String>>(&json) {
                        return list;
                    }
                }
            }
        }
    }
    Vec::new()
}

fn save_recent_search(query: &str) {
    if query.trim().is_empty() {
        return;
    }
    let mut searches = load_recent_searches();
    searches.retain(|s| s != query);
    searches.insert(0, query.to_string());
    searches.truncate(MAX_RECENT_SEARCHES);

    #[cfg(target_arch = "wasm32")]
    {
        if let Ok(json) = serde_json::to_string(&searches) {
            if let Some(window) = web_sys::window() {
                if let Ok(Some(storage)) = window.local_storage() {
                    let _ = storage.set_item(RECENT_SEARCHES_KEY, &json);
                }
            }
        }
    }
}

fn highlight_matches(text: &str, query: &str) -> String {
    if query.is_empty() {
        return html_escape(text);
    }
    let lower_text = text.to_lowercase();
    let lower_query = query.to_lowercase();
    let mut result = String::with_capacity(text.len() * 2);
    let mut last_end = 0;

    for start in lower_text.match_indices(&lower_query).map(|(i, _)| i) {
        let end = start + query.len();
        result.push_str(&html_escape(&text[last_end..start]));
        result.push_str("<mark class=\"bg-yellow-200 dark:bg-yellow-800 rounded px-0.5\">");
        result.push_str(&html_escape(&text[start..end]));
        result.push_str("</mark>");
        last_end = end;
    }
    result.push_str(&html_escape(&text[last_end..]));
    result
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[derive(Clone, Copy)]
pub struct HeaderState {
    trigger_search: ReadSignal<u32>,
    open_search: Callback<()>,
}

impl HeaderState {
    pub fn open_search(&self) {
        self.open_search.run(());
    }
}

pub fn provide_header_state() -> HeaderState {
    let (trigger_search, set_trigger_search) = signal(0u32);
    let open_search = Callback::new(move |_| {
        set_trigger_search.update(|v| *v += 1);
    });
    let state = HeaderState {
        trigger_search,
        open_search,
    };
    provide_context(state);
    state
}

pub fn use_header_state() -> Option<HeaderState> {
    use_context::<HeaderState>()
}

#[component]
pub fn Header() -> impl IntoView {
    let auth_state = auth::use_auth_state();
    let header_state = use_header_state();
    let branding: Option<ReadSignal<Option<crate::api::BrandingConfig>>> = use_context();
    let (show_search, set_show_search) = signal(false);
    let (search_query, set_search_query) = signal(String::new());
    let (search_results, set_search_results) = signal::<Vec<SearchResultEntry>>(vec![]);
    let (searching, set_searching) = signal(false);
    let (search_total, set_search_total) = signal(0usize);
    let (filter_type, set_filter_type) = signal(String::new());
    let (filter_sort, set_filter_sort) = signal(String::new());
    let (filter_folder, set_filter_folder) = signal(String::new());
    let (quota_info, set_quota_info) = signal(None::<crate::api::QuotaInfo>);
    let (recent_searches, set_recent_searches) = signal(load_recent_searches());
    let (show_suggestions, set_show_suggestions) = signal(false);

    Effect::new(move |_| {
        spawn_local(async move {
            match crate::api::get_quota().await {
                Ok(info) => {
                    if !info.unlimited {
                        set_quota_info.set(Some(info));
                    }
                }
                Err(_) => set_quota_info.set(None),
            }
        });
    });

    let toggle_search = move |_: ev::MouseEvent| {
        set_show_search.update(|v| *v = !*v);
    };

    let close_search = move |_: ev::MouseEvent| {
        set_show_search.set(false);
    };

    if let Some(hs) = header_state {
        Effect::new(move |_| {
            let _ = hs.trigger_search.get();
            set_show_search.set(true);
            #[cfg(target_arch = "wasm32")]
            {
                let _ = set_timeout_with_handle(
                    move || {
                        if let Some(window) = web_sys::window() {
                            if let Some(doc) = window.document() {
                                if let Ok(Some(input)) = doc.query_selector("#header-search-input")
                                {
                                    use wasm_bindgen::JsCast;
                                    if let Ok(el) = input.dyn_into::<web_sys::HtmlInputElement>() {
                                        let _ = el.focus();
                                    }
                                }
                            }
                        }
                    },
                    std::time::Duration::from_millis(50),
                );
            }
        });
    }

    let do_search = move |query: String| {
        if query.is_empty() {
            set_search_results.set(vec![]);
            return;
        }
        set_searching.set(true);
        set_show_suggestions.set(false);
        save_recent_search(&query);
        set_recent_searches.set(load_recent_searches());
        let ft = filter_type.get();
        let fs = filter_sort.get();
        let ff = filter_folder.get();
        spawn_local(async move {
            let filters = SearchFilters {
                r#type: if ft.is_empty() { None } else { Some(ft) },
                sort: if fs.is_empty() { None } else { Some(fs) },
                mime_type: None,
            };
            match crate::api::search_files(&query, Some(&filters)).await {
                Ok(resp) => {
                    let results = if ff.is_empty() {
                        resp.results
                    } else {
                        resp.results
                            .into_iter()
                            .filter(|r| r.path.starts_with(&ff))
                            .collect()
                    };
                    set_search_total.set(results.len());
                    set_search_results.set(results);
                }
                Err(_) => {
                    set_search_results.set(vec![]);
                    set_search_total.set(0);
                }
            }
            set_searching.set(false);
        });
    };

    let on_search_submit = move |ev: ev::KeyboardEvent| {
        if ev.key() == "Enter" {
            let query = search_query.get();
            do_search(query);
        }
        if ev.key() == "Escape" {
            set_show_search.set(false);
        }
    };

    let on_search_input = move |ev: ev::Event| {
        let v = event_target_value(&ev);
        set_search_query.set(v.clone());
    };

    #[cfg(target_arch = "wasm32")]
    {
        let sq = search_query;
        let set_searching_sig = set_searching;
        let ft_sig = filter_type;
        let fs_sig = filter_sort;
        let debounce_timer = std::cell::RefCell::new(None::<i32>);
        let closure = wasm_bindgen::closure::Closure::<dyn Fn()>::new(move || {
            let query = sq.get();
            if query.is_empty() {
                set_searching_sig.set(false);
                return;
            }
            set_searching_sig.set(true);
            let ft = ft_sig.get();
            let fs = fs_sig.get();
            spawn_local(async move {
                let filters = SearchFilters {
                    r#type: if ft.is_empty() { None } else { Some(ft) },
                    sort: if fs.is_empty() { None } else { Some(fs) },
                    mime_type: None,
                };
                match crate::api::search_files(&query, Some(&filters)).await {
                    Ok(resp) => {
                        set_search_results.set(resp.results);
                        set_search_total.set(resp.total);
                    }
                    Err(_) => {
                        set_search_results.set(vec![]);
                        set_search_total.set(0);
                    }
                }
                set_searching_sig.set(false);
            });
        });
        let cb = closure.into_js_value();
        if let Some(window) = web_sys::window() {
            if let Some(document) = window.document() {
                use wasm_bindgen::JsCast;
                let handler = wasm_bindgen::closure::Closure::<dyn Fn(ev::KeyboardEvent)>::new(
                    move |ev: web_sys::KeyboardEvent| {
                        let input: Option<web_sys::HtmlInputElement> = ev
                            .target()
                            .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok());
                        if let Some(input) = input {
                            // Only fire search for the dedicated search input, not any text field
                            if input.id() != "header-search-input" {
                                return;
                            }
                            let func = cb.clone();
                            if let Some(prev) = *debounce_timer.borrow() {
                                let _ = web_sys::window()
                                    .expect("window must exist in browser context")
                                    .clear_timeout_with_handle(prev);
                            }
                            if let Ok(handle) = web_sys::window()
                                .expect("window must exist in browser context")
                                .set_timeout_with_callback_and_timeout_and_arguments_0(
                                    func.unchecked_ref(),
                                    300,
                                )
                            {
                                *debounce_timer.borrow_mut() = Some(handle);
                            }
                        }
                    },
                );
                let _ = document
                    .add_event_listener_with_callback("input", handler.as_ref().unchecked_ref());
                std::mem::forget(handler);
            }
        }
    }

    let on_type_change = move |ev: ev::Event| {
        set_filter_type.set(event_target_value(&ev));
        let query = search_query.get();
        do_search(query);
    };

    let on_sort_change = move |ev: ev::Event| {
        set_filter_sort.set(event_target_value(&ev));
        let query = search_query.get();
        do_search(query);
    };

    let has_searched = move || search_total.get() > 0 || !search_results.with(Vec::is_empty);

    view! {
        <header class="fixed top-0 left-0 right-0 w-full z-30 surface border-b px-2 sm:px-6 py-1.5 sm:py-3 shadow-concrete">
            <div class="flex items-center justify-between max-w-7xl mx-auto">
                <div class="flex items-center gap-3">
                    <A href="/" attr:class="flex items-center gap-3 no-underline">
                        {move || {
                            if let Some(url) = branding
                                .and_then(|s| s.get())
                                .and_then(|b| b.logo_url)
                            {
                                view! {
                                    <img src=url alt=t!("brand.logo_alt") class="w-10 h-10 object-contain" />
                                }
                                .into_any()
                            } else {
                                view! {
                                    <div class="w-10 h-10 brutal-border flex items-center justify-center bg-white dark:bg-gray-800" style="font-family: var(--font-display);">
                                        <span class="font-bold text-xl" style="color: var(--accent); letter-spacing: -0.03em;">{t!("brand.name")}</span>
                                    </div>
                                }
                                .into_any()
                            }
                        }}
                        <div class="hidden sm:block">
                            {move || {
                                let title = branding
                                    .and_then(|s| s.get())
                                    .map(|b| b.title)
                                    .unwrap_or_else(|| t!("brand.name").to_string());
                                view! {
                                    <h1 class="font-mono font-bold text-xl leading-none" style="letter-spacing: -0.02em; color: var(--text-primary);">{title}</h1>
                                    <span class="text-label">{t!("brand.tagline")}</span>
                                }
                            }}
                        </div>
                    </A>
                </div>

                <div class="flex items-center gap-2 sm:gap-3">
                    {move || quota_info.get().map(|info| view! {
                        <QuotaIndicator info />
                    })}

                    <button
                        class="p-2 text-gray-500 hover:text-blue-600 hover:bg-blue-50 rounded transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 min-w-[44px] min-h-[44px] flex items-center justify-center"
                        on:click=toggle_search
                        aria-label="Search files"
                    >
                        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
                        </svg>
                    </button>

                    <A
                        href="/ui/settings"
                        attr:class="p-2 text-gray-500 hover:text-blue-600 hover:bg-blue-50 rounded transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 min-w-[44px] min-h-[44px] flex items-center justify-center no-underline"
                        attr:aria-label="Settings"
                    >
                        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                        </svg>
                    </A>

                    <ThemeToggle />

                    {move || {
                        let auth_enabled = auth_state.auth_enabled().get();
                        let token = auth_state.access_token().get();
                        let user = auth_state.user().get();

                        if !auth_enabled {
                            return view! { <div class="hidden"></div> }.into_any();
                        }

                        if let (Some(_token), Some(user)) = (token, user) {
                            let display_name = user.name.unwrap_or_else(|| user.email.unwrap_or(user.sub));
                            let auth_st = auth_state.clone();
                            view! {
                                <div class="flex items-center gap-2 sm:gap-3">
                                    <div class="w-8 h-8 brutal-border flex items-center justify-center bg-white dark:bg-gray-800">
                                        <span class="font-mono font-bold text-sm" style="color: var(--accent);">
                                            {display_name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_else(|| "?".to_string())}
                                        </span>
                                    </div>
                                    <span class="font-mono font-medium text-sm hidden sm:inline" style="color: var(--text-primary);">{display_name}</span>
                                    <button
                                        class="text-xs text-label hover:text-blue-600 transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 rounded min-h-[44px]"
                                        style="text-transform: uppercase; letter-spacing: 0.08em;"
                                        on:click=move |_| auth::logout(&auth_st)
                                    >
                                        {t!("common.sign_out")}
                                    </button>
                                </div>
                            }.into_any()
                        } else {
                            view! {
                                <a
                                    href="/ui/auth/login"
                                    class="font-mono text-xs font-bold uppercase no-underline px-3 py-2 brutal-border hover:bg-blue-50 transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 rounded"
                                    style="letter-spacing: 0.08em; color: var(--accent);"
                                >
                                    {t!("common.sign_in")}
                                </a>
                            }.into_any()
                        }
                    }}
                </div>
            </div>

            <div style:display=move || if show_search.get() { "block" } else { "none" } class="border-t bg-gray-50 dark:bg-gray-900 px-6 py-3 max-w-7xl mx-auto slide-up">
                <div class="flex items-center gap-2 mb-2">
                    <label for="header-search-input" class="sr-only">{t!("search.aria_label")}</label>
                    <div class="relative flex-1">
                        <input
                            type="text"
                            id="header-search-input"
                            placeholder=t!("search.placeholder")
                            aria-label=t!("search.aria_label")
                            class="w-full px-4 py-2 pl-10 border rounded bg-white dark:bg-gray-800 text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent font-mono"
                            prop:value=search_query
                            on:input=on_search_input
                            on:keydown=on_search_submit
                            on:focus=move |_| {
                                if search_query.get().is_empty() {
                                    set_show_suggestions.set(true);
                                }
                            }
                            on:blur=move |_| {
                                // Delay to allow click on suggestion
                                #[cfg(target_arch = "wasm32")]
                                {
                                    let _ = set_timeout_with_handle(
                                        move || set_show_suggestions.set(false),
                                        std::time::Duration::from_millis(200),
                                    );
                                }
                            }
                        />
                        <svg class="absolute left-3 top-2.5 w-4 h-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
                        </svg>

                        // Search suggestions dropdown
                        {move || {
                            let queries = recent_searches.get();
                            let show = show_suggestions.get() && search_query.get().is_empty() && !queries.is_empty();
                            if !show {
                                return view! { <div class="hidden"></div> }.into_any();
                            }
                            view! {
                                <div class="absolute top-full left-0 right-0 mt-1 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded shadow-lg z-50 max-h-48 overflow-y-auto">
                                    <div class="px-3 py-1.5 text-xs font-mono text-gray-400 uppercase tracking-wider border-b border-gray-100 dark:border-gray-700">
                                        "Recent Searches"
                                    </div>
                                    {queries.into_iter().map(|q| {
                                        let query = q.clone();
                                        let set_q = set_search_query;
                                        let do_s = do_search;
                                        let set_show = set_show_suggestions;
                                        view! {
                                            <button
                                                class="w-full text-left px-3 py-2 text-sm font-mono text-gray-700 dark:text-gray-300 hover:bg-blue-50 dark:hover:bg-gray-700 transition-colors flex items-center gap-2"
                                                on:mousedown=move |ev| {
                                                    ev.prevent_default();
                                                    set_q.set(query.clone());
                                                    do_s(query.clone());
                                                    set_show.set(false);
                                                }
                                            >
                                                <svg class="w-3.5 h-3.5 text-gray-400 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
                                                </svg>
                                                <span class="truncate">{q}</span>
                                            </button>
                                        }
                                    }).collect::<Vec<_>>()}
                                </div>
                            }.into_any()
                        }}
                    </div>
                    <button
                        class="p-2 text-gray-500 hover:text-blue-600 rounded transition-colors min-w-[44px] min-h-[44px] flex items-center justify-center"
                        on:click=close_search
                        aria-label=t!("search.aria_close")
                    >
                        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                        </svg>
                    </button>
                </div>

                <div class="flex items-center gap-2 mb-2 flex-wrap">
                    <label for="search-filter-type" class="sr-only">{t!("search.filter_type")}</label>
                    <select
                        id="search-filter-type"
                        class="px-3 py-1 text-xs font-mono font-medium border rounded bg-white dark:bg-gray-800 text-gray-700 dark:text-gray-300 focus:outline-none focus:ring-2 focus:ring-blue-500 uppercase"
                        style="letter-spacing: 0.05em;"
                        aria-label=t!("search.filter_type")
                        on:change=on_type_change
                    >
                        <option value="" selected=move || filter_type.get().is_empty()>{t!("search.filter_all")}</option>
                        <option value="file" selected=move || filter_type.get() == "file">{t!("search.filter_files")}</option>
                        <option value="folder" selected=move || filter_type.get() == "folder">{t!("search.filter_folders")}</option>
                    </select>
                    <label for="search-filter-sort" class="sr-only">{t!("search.sort_by")}</label>
                    <select
                        id="search-filter-sort"
                        class="px-3 py-1 text-xs font-mono font-medium border rounded bg-white dark:bg-gray-800 text-gray-700 dark:text-gray-300 focus:outline-none focus:ring-2 focus:ring-blue-500 uppercase"
                        style="letter-spacing: 0.05em;"
                        aria-label=t!("search.sort_by")
                        on:change=on_sort_change
                    >
                        <option value="" selected=move || filter_sort.get().is_empty()>{t!("search.sort_relevance")}</option>
                        <option value="name" selected=move || filter_sort.get() == "name">{t!("search.sort_name")}</option>
                        <option value="date" selected=move || filter_sort.get() == "date">{t!("search.sort_date")}</option>
                        <option value="size" selected=move || filter_sort.get() == "size">{t!("search.sort_size")}</option>
                    </select>
                    <label for="search-filter-folder" class="sr-only">"Search in folder"</label>
                    <input
                        id="search-filter-folder"
                        type="text"
                        placeholder="/path/to/folder"
                        class="px-3 py-1 text-xs font-mono font-medium border rounded bg-white dark:bg-gray-800 text-gray-700 dark:text-gray-300 focus:outline-none focus:ring-2 focus:ring-blue-500"
                        style="letter-spacing: 0.05em; max-width: 180px;"
                        aria-label="Search in folder"
                        prop:value=filter_folder
                        on:input=move |ev| set_filter_folder.set(event_target_value(&ev))
                        on:change=move |_| {
                            let query = search_query.get();
                            do_search(query);
                        }
                    />
                </div>

                {move || searching.get().then(|| view! {
                    <div class="text-sm font-mono text-gray-500">{t!("common.searching")}</div>
                })}
                {move || has_searched().then(|| view! {
                    <div class="text-xs font-mono text-gray-400 mb-1" style="letter-spacing: 0.05em;" aria-live="polite">
                        {move || format!("{} results", search_total.get())}
                    </div>
                })}
                <SearchResultsList results=search_results query=search_query />
            </div>
        </header>
    }
}

#[component]
fn QuotaIndicator(info: crate::api::QuotaInfo) -> impl IntoView {
    let used_str = format_size(info.used_bytes);
    let quota_str = format_size(info.quota_bytes);
    let percent = info.used_percent;
    let is_over_90 = percent > 90.0;
    let bar_color = if is_over_90 {
        "bg-red-500"
    } else {
        "bg-blue-500"
    };
    let text_color = if is_over_90 {
        "text-red-600 dark:text-red-400"
    } else {
        "text-gray-500 dark:text-gray-400"
    };

    view! {
        <div class="hidden md:flex items-center gap-2 font-mono text-xs" style="letter-spacing: 0.03em;">
            <div class="w-28 h-3 bg-gray-200 dark:bg-gray-700 rounded-none overflow-hidden brutal-border" title=move || format!("{}% used", percent as u32)>
                <div
                    class=move || format!("h-full transition-all {}", bar_color)
                    style=move || format!("width: {}%;", percent.min(100.0))
                ></div>
            </div>
            <span class=text_color style="font-weight: 600;">
                {move || format!("{} / {} ({}%)", used_str, quota_str, percent as u32)}
            </span>
        </div>
    }
}

#[component]
fn SearchResultsList(
    results: ReadSignal<Vec<SearchResultEntry>>,
    query: ReadSignal<String>,
) -> impl IntoView {
    view! {
        {move || {
            let empty = results.with(Vec::is_empty);
            let q = query.get();
            if empty && q.is_empty() {
                return view! { <div class="hidden"></div> }.into_any();
            }
            if empty {
                return view! {
                    <div class="py-6 text-center">
                        <svg class="w-12 h-12 mx-auto mb-3 text-gray-300 dark:text-gray-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
                        </svg>
                        <div class="font-mono font-semibold text-sm text-gray-500">{t!("search.no_results")}</div>
                        <div class="text-xs text-gray-400 mt-1">{t!("search.no_results_hint")}</div>
                    </div>
                }.into_any();
            }
            view! {
                <div class="surface brutal-border shadow-xl max-h-64 overflow-auto rounded-lg" role="listbox" aria-label=t!("search.aria_results")>
                    <For
                        each=move || results.get()
                        key=|r| r.path.clone()
                        let:result
                    >
                        {
                            let dir_path = result.path.clone();
                            let parent = dir_path.rfind('/').map(|i| &dir_path[..i]).unwrap_or("/");
                            let highlighted_name = highlight_matches(&result.name, &q);
                            let highlighted_path = highlight_matches(&result.path, &q);
                            view! {
                                <a
                                    class="block w-full text-left px-4 py-2 hover:bg-blue-50 border-b border-gray-100 last:border-0 cursor-pointer no-underline text-inherit transition-colors"
                                    href=format!("/ui/files{}", parent)
                                >
                                    <div class="flex items-center gap-2">
                                        <svg class="w-4 h-4 text-gray-400 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                                        </svg>
                                        <div class="min-w-0">
                                            <div class="text-sm font-semibold font-mono truncate" inner_html=highlighted_name></div>
                                            <div class="text-xs text-gray-500 font-mono truncate" inner_html=highlighted_path></div>
                                        </div>
                                    </div>
                                    {result.snippet.as_ref().map(|s| {
                                        let highlighted_snippet = highlight_matches(s, &q);
                                        view! {
                                            <div class="text-xs text-gray-500 mt-0.5 ml-6 truncate" inner_html=highlighted_snippet></div>
                                        }
                                    })}
                                </a>
                            }
                        }
                    </For>
                </div>
            }.into_any()
        }}
    }
}
