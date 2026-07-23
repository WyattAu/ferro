use crate::components::primitives::Spinner;
use leptos::prelude::*;

/// Search bar with filters and results.
#[component]
pub fn SearchBar() -> impl IntoView {
    let (query, set_query) = signal(String::new());
    let (results, set_results) = signal(Vec::<crate::api::endpoints::FileEntry>::new());
    let (searching, set_searching) = signal(false);
    let (show_results, set_show_results) = signal(false);
    let _set_q = set_query;

    let _do_search = move |q: String| {
        if q.is_empty() {
            set_results.set(Vec::new());
            set_show_results.set(false);
            return;
        }

        set_searching.set(true);
        set_show_results.set(true);

        #[cfg(target_arch = "wasm32")]
        {
            let set_r = set_results;
            let set_s = set_searching;
            wasm_bindgen_futures::spawn_local(async move {
                let client = crate::api::ApiClient::from_env();
                match client
                    .get::<crate::api::endpoints::SearchResult>(&format!(
                        "/api/v1/search?q={}",
                        urlencoding::encode(&q)
                    ))
                    .await
                {
                    Ok(resp) => {
                        set_r.set(resp.entries);
                        set_s.set(false);
                    }
                    Err(e) => {
                        log::error!("Search failed: {}", e);
                        set_s.set(false);
                    }
                }
            });
        }
    };

    view! {
        <div class="relative">
            <div class="flex items-center gap-2">
                <input
                    class="input flex-1"
                    type="text"
                    placeholder="Search files..."
                    prop:value=move || query.get()
                />
                {move || if searching.get() {
                    view! { <Spinner /> }.into_any()
                } else {
                    ().into_any()
                }}
            </div>

            {move || {
                if show_results.get() && !results.get().is_empty() {
                    view! {
                        <div class="absolute top-full left-0 right-0 mt-1 bg-raised border border-[var(--color-border)] rounded-lg shadow-lg z-50 max-h-64 overflow-y-auto">
                            {results.get().into_iter().map(|entry| {
                                let name = entry.name.clone();
                                let p = entry.path.clone();
                                view! {
                                    <a
                                        class="block px-4 py-2 hover:bg-sunken cursor-pointer text-sm"
                                        href=format!("/ui/files{}", p)
                                    >
                                        <span class="font-medium">{name}</span>
                                        <span class="text-secondary ml-2 text-xs">{entry.path}</span>
                                    </a>
                                }
                            }).collect_view()}
                        </div>
                    }.into_any()
                } else {
                    ().into_any()
                }
            }}
        </div>
    }
}
