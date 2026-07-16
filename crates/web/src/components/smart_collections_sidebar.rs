use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::api;
use crate::components::toast::ToastContext;

/// Sidebar panel showing smart collections.
/// Self-contained: loads its own data when opened.
#[component]
pub fn SmartCollectionsSidebar(
    /// Whether the sidebar is visible.
    open: ReadSignal<bool>,
    /// Setter for sidebar visibility.
    set_open: WriteSignal<bool>,
) -> impl IntoView {
    let (collections, set_collections) = signal(Vec::<api::SmartCollection>::new());
    let (loading, set_loading) = signal(false);
    let (show_create, set_show_create) = signal(false);
    let (new_name, set_new_name) = signal(String::new());

    let load_collections = move || {
        set_loading.set(true);
        spawn_local(async move {
            match api::list_smart_collections().await {
                Ok(resp) => {
                    set_collections.set(resp.collections);
                    set_loading.set(false);
                }
                Err(_) => {
                    set_collections.set(vec![]);
                    set_loading.set(false);
                }
            }
        });
    };

    // Reload collections whenever the sidebar opens.
    Effect::new(move |_| {
        if open.get() {
            load_collections();
        }
    });

    let create_collection = move || {
        let name = new_name.get();
        if name.trim().is_empty() {
            return;
        }
        spawn_local(async move {
            let req = api::CreateSmartCollectionRequest {
                name: name.clone(),
                rules: vec![serde_json::json!({"file_type": {"mime_pattern": "*"}})],
                auto_update: true,
            };
            match api::create_smart_collection(&req).await {
                Ok(_collection) => {
                    ToastContext::success(format!("Smart collection '{}' created", name));
                    set_new_name.set(String::new());
                    set_show_create.set(false);
                    load_collections();
                }
                Err(e) => {
                    ToastContext::error(format!("Failed to create collection: {}", e));
                }
            }
        });
    };

    let delete_collection = move |id: String, name: String| {
        spawn_local(async move {
            match api::delete_smart_collection(&id).await {
                Ok(()) => {
                    ToastContext::info(format!("Deleted collection '{}'", name));
                    load_collections();
                }
                Err(e) => {
                    ToastContext::error(format!("Failed to delete collection: {}", e));
                }
            }
        });
    };

    view! {
        {move || open.get().then(|| view! {
            <div class="w-72 brutal-border border-l surface overflow-y-auto transition-all duration-200">
                <div class="px-4 py-3 border-b border-[var(--border-default)] flex items-center justify-between">
                    <h3 class="text-label font-mono text-[var(--text-primary)]">"Smart Collections"</h3>
                    <button
                        class="p-1 text-[var(--text-tertiary)] hover:text-[var(--text-secondary)] rounded focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] min-w-[44px] min-h-[44px] flex items-center justify-center"
                        on:click=move |_| set_open.set(false)
                        aria-label="Close smart collections"
                    >
                        <svg class="w-4 h-4" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" /></svg>
                    </button>
                </div>

                // Create button
                <div class="px-4 py-2 border-b border-[var(--border-default)]">
                    {move || if show_create.get() {
                        view! {
                            <div class="flex gap-2">
                                <input
                                    type="text"
                                    class="flex-1 px-2 py-1 text-sm bg-[var(--bg-inset)] border border-[var(--border)] rounded font-mono text-[var(--text-primary)] focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)]"
                                    placeholder="Collection name"
                                    prop:value=move || new_name.get()
                                    on:input=move |ev| set_new_name.set(event_target_value(&ev))
                                    on:keydown=move |ev: web_sys::KeyboardEvent| {
                                        if ev.key() == "Enter" {
                                            create_collection();
                                        } else if ev.key() == "Escape" {
                                            set_show_create.set(false);
                                            set_new_name.set(String::new());
                                        }
                                    }
                                />
                                <button
                                    class="px-2 py-1 text-xs bg-[var(--accent)] text-[var(--text-on-accent)] rounded hover:bg-[var(--accent-hover)] font-bold"
                                    on:click=move |_| create_collection()
                                >
                                    "Add"
                                </button>
                            </div>
                        }.into_any()
                    } else {
                        view! {
                            <button
                                class="w-full px-3 py-1.5 text-xs text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-inset)] rounded transition-colors font-mono flex items-center gap-1"
                                on:click=move |_| set_show_create.set(true)
                            >
                                <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
                                </svg>
                                "New Smart Collection"
                            </button>
                        }.into_any()
                    }}
                </div>

                // Collections list
                <div class="p-4 space-y-2">
                    {move || loading.get().then(|| view! {
                        <div class="text-center py-4">
                            <div class="animate-spin w-6 h-6 border-2 border-[var(--accent)] border-t-transparent rounded-full mx-auto"></div>
                            <p class="text-xs text-[var(--text-tertiary)] mt-2">"Loading..."</p>
                        </div>
                    })}
                    {move || {
                        let cols = collections.get();
                        if cols.is_empty() && !loading.get() {
                            view! {
                                <div class="text-center py-8">
                                    <svg class="w-12 h-12 mx-auto mb-3 text-[var(--text-tertiary)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
                                    </svg>
                                    <p class="text-sm text-[var(--text-tertiary)]">"No smart collections"</p>
                                    <p class="text-xs text-[var(--text-tertiary)] mt-1">"Create one to auto-organize files"</p>
                                </div>
                            }.into_any()
                        } else {
                            view! {
                                <For
                                    each=move || collections.get()
                                    key=|c| c.id.clone()
                                    let:collection
                                >
                                    {
                                        let coll_id = collection.id.clone();
                                        let coll_name = collection.name.clone();
                                        let rules_count = collection.rules.len();
                                        view! {
                                            <div class="flex items-center justify-between p-2 rounded hover:bg-[var(--bg-inset)] group">
                                                <div class="min-w-0">
                                                    <div class="text-sm font-mono text-[var(--text-primary)] truncate">
                                                        {coll_name.clone()}
                                                    </div>
                                                    <div class="text-xs text-[var(--text-tertiary)]">
                                                        {rules_count} " rule(s)"
                                                        {if collection.auto_update { " \u{2022} auto" } else { "" }}
                                                    </div>
                                                </div>
                                                <button
                                                    class="opacity-0 group-hover:opacity-100 p-1 text-[var(--text-tertiary)] hover:text-[var(--danger)] rounded transition-opacity"
                                                    on:click=move |_| delete_collection(coll_id.clone(), coll_name.clone())
                                                    aria-label="Delete collection"
                                                >
                                                    <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                                                    </svg>
                                                </button>
                                            </div>
                                        }
                                    }
                                </For>
                            }.into_any()
                        }
                    }}
                </div>
            </div>
        })}
    }
}
