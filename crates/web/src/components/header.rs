use leptos::*;

use crate::api::{SearchFilters, SearchResultEntry};
use crate::auth;
use crate::components::theme_toggle::ThemeToggle;
use leptos_router::A;

#[derive(Clone, Copy)]
pub struct HeaderState {
    trigger_search: ReadSignal<u32>,
    open_search: Callback<()>,
}

impl HeaderState {
    pub fn open_search(&self) {
        self.open_search.call(());
    }
}

pub fn provide_header_state() -> HeaderState {
    let (trigger_search, set_trigger_search) = create_signal(0u32);
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
    let (show_search, set_show_search) = create_signal(false);
    let (search_query, set_search_query) = create_signal(String::new());
    let (search_results, set_search_results) = create_signal::<Vec<SearchResultEntry>>(vec![]);
    let (searching, set_searching) = create_signal(false);
    let (search_total, set_search_total) = create_signal(0usize);
    let (filter_type, set_filter_type) = create_signal(String::new());
    let (filter_sort, set_filter_sort) = create_signal(String::new());
    let (quota_info, set_quota_info) = create_signal(None::<crate::api::QuotaInfo>);

    create_effect(move |_| {
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
        create_effect(move |_| {
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
        let ft = filter_type.get();
        let fs = filter_sort.get();
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
        spawn_local(async move {
            let mut debounce_timer: Option<js_sys::Number> = None;
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
                    let handler = wasm_bindgen::closure::Closure::<dyn Fn(ev::KeyboardEvent)>::new(
                        move |ev: web_sys::KeyboardEvent| {
                            let input: Option<web_sys::HtmlInputElement> =
                                ev.target().and_then(|t| {
                                    use wasm_bindgen::JsCast;
                                    t.dyn_into::<web_sys::HtmlInputElement>().ok()
                                });
                            if let Some(input) = input {
                                use wasm_bindgen::JsCast;
                                let func = cb.clone();
                                if let Some(prev) = debounce_timer {
                                    let _ =
                                        web_sys::window().unwrap().clear_timeout_with_handle(prev);
                                }
                                debounce_timer = Some(
                                    web_sys::window()
                                        .unwrap()
                                        .set_timeout_with_callback_and_timeout_and_arguments_0(
                                            func.unchecked_ref(),
                                            300,
                                        )
                                        .unwrap(),
                                );
                            }
                        },
                    );
                    let _ = document.add_event_listener_with_callback(
                        "input",
                        handler.as_ref().unchecked_ref(),
                    );
                    handler.forget();
                }
            }
        });
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
        <header class="surface border-b px-4 sm:px-6 py-3 shadow-concrete">
            <div class="flex items-center justify-between max-w-7xl mx-auto">
                <div class="flex items-center gap-3">
                    <A href="/" class="flex items-center gap-3 no-underline">
                        <div class="w-10 h-10 brutal-border flex items-center justify-center bg-white dark:bg-gray-800" style="font-family: var(--font-display);">
                            <span class="font-bold text-xl" style="color: var(--accent); letter-spacing: -0.03em;">"F"</span>
                        </div>
                        <div class="hidden sm:block">
                            <h1 class="font-mono font-bold text-xl leading-none" style="letter-spacing: -0.02em; color: var(--text-primary);">"Ferro"</h1>
                            <span class="text-label">"Storage Orchestrator"</span>
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
                        class="p-2 text-gray-500 hover:text-blue-600 hover:bg-blue-50 rounded transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 min-w-[44px] min-h-[44px] flex items-center justify-center no-underline"
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
                                        class="text-xs text-label hover:text-blue-600 transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 rounded"
                                        style="text-transform: uppercase; letter-spacing: 0.08em;"
                                        on:click=move |_| auth::logout(&auth_st)
                                    >
                                        "Sign out"
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
                                    "Sign in"
                                </a>
                            }.into_any()
                        }
                    }}
                </div>
            </div>

            {move || show_search.get().then(|| view! {
                <div class="border-t bg-gray-50 dark:bg-gray-900 px-6 py-3 max-w-7xl mx-auto slide-up">
                    <div class="flex items-center gap-2 mb-2">
                        <div class="relative flex-1">
                            <input
                                type="text"
                                id="header-search-input"
                                placeholder="Search files..."
                                class="w-full px-4 py-2 pl-10 border rounded bg-white dark:bg-gray-800 text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent font-mono"
                                prop:value=search_query
                                on:input=on_search_input
                                on:keydown=on_search_submit
                            />
                            <svg class="absolute left-3 top-2.5 w-4 h-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
                            </svg>
                        </div>
                        <button
                            class="p-2 text-gray-500 hover:text-blue-600 rounded transition-colors"
                            on:click=close_search
                            aria-label="Close search"
                        >
                            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                            </svg>
                        </button>
                    </div>

                    {move || show_search.get().then(|| view! {
                        <div class="flex items-center gap-2 mb-2">
                            <select
                                class="px-3 py-1 text-xs font-mono font-medium border rounded bg-white dark:bg-gray-800 text-gray-700 dark:text-gray-300 focus:outline-none focus:ring-2 focus:ring-blue-500 uppercase"
                                style="letter-spacing: 0.05em;"
                                on:change=on_type_change
                            >
                                <option value="" selected=move || filter_type.get().is_empty()>"All Types"</option>
                                <option value="file" selected=move || filter_type.get() == "file">"Files"</option>
                                <option value="folder" selected=move || filter_type.get() == "folder">"Folders"</option>
                            </select>
                            <select
                                class="px-3 py-1 text-xs font-mono font-medium border rounded bg-white dark:bg-gray-800 text-gray-700 dark:text-gray-300 focus:outline-none focus:ring-2 focus:ring-blue-500 uppercase"
                                style="letter-spacing: 0.05em;"
                                on:change=on_sort_change
                            >
                                <option value="" selected=move || filter_sort.get().is_empty()>"Relevance"</option>
                                <option value="name" selected=move || filter_sort.get() == "name">"Name"</option>
                                <option value="date" selected=move || filter_sort.get() == "date">"Date"</option>
                                <option value="size" selected=move || filter_sort.get() == "size">"Size"</option>
                            </select>
                        </div>
                    })}

                    {move || searching.get().then(|| view! {
                        <div class="text-sm font-mono text-gray-500">"Searching..."</div>
                    })}
                    {move || has_searched().then(|| view! {
                        <div class="text-xs font-mono text-gray-400 mb-1" style="letter-spacing: 0.05em;">
                            {move || format!("{} result(s)", search_total.get())}
                        </div>
                    })}
                    <SearchResultsList results=search_results query=search_query />
                </div>
            })}
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

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
    const TB: u64 = 1024 * GB;

    if bytes < KB {
        format!("{} B", bytes)
    } else if bytes < MB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else if bytes < GB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes < TB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else {
        format!("{:.1} TB", bytes as f64 / TB as f64)
    }
}

#[component]
fn SearchResultsList(
    results: ReadSignal<Vec<SearchResultEntry>>,
    query: ReadSignal<String>,
) -> impl IntoView {
    let navigate = leptos_router::use_navigate();
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
                        <div class="font-mono font-semibold text-sm text-gray-500">"No files match your search"</div>
                        <div class="text-xs text-gray-400 mt-1">"Check spelling, try different keywords, or remove filters"</div>
                    </div>
                }.into_any();
            }
            let nav = navigate.clone();
            view! {
                <div class="surface brutal-border shadow-xl max-h-64 overflow-auto rounded-lg">
                    <For
                        each=move || results.get()
                        key=|r| r.path.clone()
                        let:result
                    >
                        {
                            let _n = nav.clone();
                            let dir_path = result.path.clone();
                            let parent = dir_path.rfind('/').map(|i| &dir_path[..i]).unwrap_or("/");
                            view! {
                                <a
                                    class="block w-full text-left px-4 py-2 hover:bg-blue-50 border-b border-gray-100 last:border-0 cursor-pointer no-underline text-inherit transition-colors"
                                    href=format!("/files{}", parent)
                                >
                                    <div class="flex items-center gap-2">
                                        <svg class="w-4 h-4 text-gray-400 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                                        </svg>
                                        <div class="min-w-0">
                                            <div class="text-sm font-semibold font-mono truncate">{result.name.clone()}</div>
                                            <div class="text-xs text-gray-500 font-mono truncate">{result.path.clone()}</div>
                                        </div>
                                    </div>
                                    {result.snippet.as_ref().map(|s| view! {
                                        <div class="text-xs text-gray-500 mt-0.5 ml-6 truncate">{s.clone()}</div>
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
