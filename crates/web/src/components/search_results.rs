use leptos::*;

use crate::api::SearchResultEntry;
use leptos_router::use_navigate;

#[component]
pub fn SearchResultsPanel(
    results: ReadSignal<Vec<SearchResultEntry>>,
    query: ReadSignal<String>,
    total: ReadSignal<usize>,
    searching: ReadSignal<bool>,
    on_close: Callback<()>,
) -> impl IntoView {
    let navigate = use_navigate();

    view! {
        <div class="bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg shadow-lg max-h-96 overflow-auto">
            <div class="px-4 py-2 border-b border-gray-200 dark:border-gray-700 flex items-center justify-between">
                <span class="text-sm text-gray-500 dark:text-gray-400">
                    {move || {
                        if searching.get() {
                            "Searching...".to_string()
                        } else {
                            let t = total.get();
                            let q = query.get();
                            if q.is_empty() {
                                String::new()
                            } else {
                                format!("{} result(s) for \"{}\"", t, q)
                            }
                        }
                    }}
                </span>
                <button
                    class="p-1 text-gray-400 hover:text-gray-600 dark:hover:text-gray-200 rounded"
                    on:click=move |_| on_close.call(())
                    attr:aria-label="Close results"
                >
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                    </svg>
                </button>
            </div>

            {move || {
                if searching.get() {
                    return view! {
                        <div class="px-4 py-8 text-center text-gray-500 dark:text-gray-400">
                            <div class="animate-spin w-6 h-6 border-2 border-blue-600 border-t-transparent rounded-full mx-auto mb-2"></div>
                            "Searching..."
                        </div>
                    }.into_any();
                }

                let items = results.get();
                let q = query.get();
                if q.is_empty() {
                    return view! {
                        <div class="px-4 py-6 text-center text-sm text-gray-500 dark:text-gray-400">
                            "Type a search query to find files"
                        </div>
                    }.into_any();
                }

                if items.is_empty() {
                    return view! {
                        <div class="px-4 py-6 text-center">
                            <svg class="w-12 h-12 mx-auto mb-3 text-gray-300 dark:text-gray-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
                            </svg>
                            <div class="text-sm text-gray-500 dark:text-gray-400">"No results found"</div>
                            <div class="text-xs text-gray-400 dark:text-gray-500 mt-1">"Try different keywords or remove filters"</div>
                        </div>
                    }.into_any();
                }

                let nav = navigate.clone();
                view! {
                    <div>
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
                                    class="block w-full text-left px-4 py-2.5 hover:bg-blue-50 dark:hover:bg-blue-900/20 border-b border-gray-100 dark:border-gray-700 last:border-0 cursor-pointer no-underline text-inherit transition-colors"
                                    href=format!("/files{}", parent)
                                >
                                    <div class="flex items-center gap-3">
                                        <svg class="w-4 h-4 text-gray-400 dark:text-gray-500 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                                        </svg>
                                        <div class="min-w-0 flex-1">
                                            <div class="text-sm font-medium text-gray-900 dark:text-gray-100 truncate">{result.name.clone()}</div>
                                            <div class="text-xs text-gray-500 dark:text-gray-400 truncate">{result.path.clone()}</div>
                                        </div>
                                        {result.snippet.as_ref().map(|s| view! {
                                            <span class="text-xs text-gray-400 dark:text-gray-500 max-w-[200px] truncate">{s.clone()}</span>
                                        })}
                                    </div>
                                </a>
                            }
                        }
                    </For>
                    </div>
                }.into_any()
            }}
        </div>
    }
}
