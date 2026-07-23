use leptos::prelude::*;

/// Notes page with sidebar, editor, and folder/tag organization.
#[allow(unused_variables)]
#[component]
pub fn NotesPage() -> impl IntoView {
    let (notes, set_notes) = signal(Vec::<Note>::new());
    let (selected_id, set_selected_id) = signal(None::<String>);
    let (_loading, set_loading) = signal(true);
    let (search, _set_search) = signal(String::new());

    #[derive(Clone, Debug)]
    struct Note {
        id: String,
        title: String,
        content: String,
        folder: String,
        tags: Vec<String>,
        #[allow(dead_code)]
        updated_at: String,
    }

    Effect::new(move |_| {
        set_loading.set(true);
        #[cfg(target_arch = "wasm32")]
        {
            let set_n = set_notes;
            let set_l = set_loading;
            wasm_bindgen_futures::spawn_local(async move {
                let client = crate::api::ApiClient::from_env();
                match client.get::<serde_json::Value>("/api/v1/notes").await {
                    Ok(val) => {
                        if let Some(arr) = val.as_array() {
                            let items: Vec<Note> = arr
                                .iter()
                                .filter_map(|v| {
                                    Some(Note {
                                        id: v["id"].as_str()?.to_string(),
                                        title: v["title"].as_str().unwrap_or("Untitled").to_string(),
                                        content: v["content"].as_str().unwrap_or("").to_string(),
                                        folder: v["folder"].as_str().unwrap_or("").to_string(),
                                        tags: v["tags"]
                                            .as_array()
                                            .map(|a| a.iter().filter_map(|t| t.as_str().map(String::from)).collect())
                                            .unwrap_or_default(),
                                        updated_at: v["updated_at"].as_str().unwrap_or("").to_string(),
                                    })
                                })
                                .collect();
                            set_n.set(items);
                        }
                        set_l.set(false);
                    }
                    Err(e) => {
                        log::error!("Notes load failed: {}", e);
                        set_l.set(false);
                    }
                }
            });
        }
    });

    view! {
        <div class="flex h-full">
            // Sidebar
            <aside class="w-64 border-r border-[var(--color-border)] overflow-y-auto flex-shrink-0">
                <div class="p-3">
                    <input class="input w-full" type="text" placeholder="Search notes..." prop:value=move || search.get() />
                </div>
                <nav class="px-2">
                    {move || {
                        let q = search.get().to_lowercase();
                        notes.get().into_iter()
                            .filter(|n| q.is_empty() || n.title.to_lowercase().contains(&q))
                            .map(|n| {
                                let id = n.id.clone();
                                let title = n.title.clone();
                                let id2 = id.clone();
                                let sel = move || selected_id.get() == Some(id.clone());
                                view! {
                                    <button
                                        class=move || format!("w-full text-left px-3 py-2 rounded-md text-sm {}",
                                            if sel() { "bg-accent-subtle text-accent" } else { "hover:bg-sunken" })
                                        on:click=move |_| set_selected_id.set(Some(id2.clone()))
                                    >
                                        {title}
                                    </button>
                                }
                            }).collect_view()
                    }}
                </nav>
            </aside>

            // Editor
            <main class="flex-1 overflow-y-auto p-6">
                {move || {
                    match selected_id.get() {
                        Some(id) => {
                            let note = notes.get().iter().find(|n| n.id == id).cloned();
                            match note {
                                Some(n) => view! {
                                    <div>
                                        <h1 class="text-2xl font-bold mb-4">{n.title}</h1>
                                        <p class="text-secondary mb-2 text-sm">{format!("Folder: {} | Tags: {}", n.folder, n.tags.join(", "))}</p>
                                        <div class="prose max-w-none">
                                            <pre class="whitespace-pre-wrap">{n.content}</pre>
                                        </div>
                                    </div>
                                }.into_any(),
                                None => view! { <p class="text-secondary">"Note not found"</p> }.into_any(),
                            }
                        }
                        None => view! {
                            <div class="text-center text-secondary py-12">
                                <p class="text-lg">"Select a note to view"</p>
                            </div>
                        }.into_any(),
                    }
                }}
            </main>
        </div>
    }
}
