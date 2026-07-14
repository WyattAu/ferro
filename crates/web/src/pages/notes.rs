use leptos::ev;
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::api;
use crate::components::header::{Header, provide_header_state};
use crate::components::markdown_editor::MarkdownEditor;
use crate::components::theme_toggle::provide_theme_state;
use crate::t;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NoteMeta {
    pub id: String,
    pub title: String,
    pub folder: String,
    pub tags: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Note {
    pub id: String,
    pub title: String,
    pub content: String,
    pub folder: String,
    pub tags: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq)]
enum SortBy {
    UpdatedAt,
    Title,
    CreatedAt,
}

#[component]
pub fn NotesPage() -> impl IntoView {
    provide_theme_state();
    provide_header_state();

    let (loading, set_loading) = signal(true);
    let (notes, set_notes) = signal(Vec::<NoteMeta>::new());
    let (_error_msg, set_error) = signal(String::new());
    let (search_query, set_search_query) = signal(String::new());
    let (selected_note_id, set_selected_note_id) = signal(None::<String>);
    let (selected_note, set_selected_note) = signal(None::<Note>);
    let (sort_by, set_sort_by) = signal(SortBy::UpdatedAt);
    let (sort_order, set_sort_order) = signal("desc".to_string());
    let (folder_filter, set_folder_filter) = signal(String::new());
    let (show_create_dialog, set_show_create_dialog) = signal(false);
    let (create_title, set_create_title) = signal(String::new());
    let (create_folder, set_create_folder) = signal(String::new());

    let fetch_notes = move || {
        set_loading.set(true);
        set_error.set(String::new());
        spawn_local(async move {
            let sort_str = match sort_by.get() {
                SortBy::UpdatedAt => "updated_at",
                SortBy::Title => "title",
                SortBy::CreatedAt => "created_at",
            };
            let mut url = format!("/api/notes?sort={}&order={}", sort_str, sort_order.get());
            let folder = folder_filter.get();
            if !folder.is_empty() {
                url.push_str(&format!("&folder={}", urlencoding(&folder)));
            }
            let q = search_query.get();
            if !q.is_empty() {
                url.push_str(&format!("&q={}", urlencoding(&q)));
            }
            match api::fetch_json(&url).await {
                Ok(val) => {
                    let notes_list = val
                        .get("notes")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| {
                                    Some(NoteMeta {
                                        id: v.get("id")?.as_str()?.to_string(),
                                        title: v.get("title").and_then(|t| t.as_str()).unwrap_or("").to_string(),
                                        folder: v.get("folder").and_then(|f| f.as_str()).unwrap_or("").to_string(),
                                        tags: v.get("tags").and_then(|t| t.as_str()).unwrap_or("").to_string(),
                                        created_at: v
                                            .get("created_at")
                                            .and_then(|c| c.as_str())
                                            .unwrap_or("")
                                            .to_string(),
                                        updated_at: v
                                            .get("updated_at")
                                            .and_then(|u| u.as_str())
                                            .unwrap_or("")
                                            .to_string(),
                                    })
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    set_notes.set(notes_list);
                    set_loading.set(false);
                }
                Err(e) => {
                    set_error.set(e);
                    set_loading.set(false);
                }
            }
        });
    };

    Effect::new(move |_| {
        let _ = sort_by.get();
        let _ = sort_order.get();
        let _ = folder_filter.get();
        fetch_notes();
    });

    let open_note = move |id: String| {
        let id_clone = id.clone();
        set_selected_note_id.set(Some(id.clone()));
        spawn_local(async move {
            match api::fetch_json(&format!("/api/notes/{}", id_clone)).await {
                Ok(val) => {
                    if let Ok(note) = serde_json::from_value::<Note>(val) {
                        set_selected_note.set(Some(note));
                    }
                }
                Err(e) => {
                    set_error.set(e);
                }
            }
        });
    };

    let create_note = move |_: ev::MouseEvent| {
        let title = create_title.get();
        let folder = create_folder.get();
        set_show_create_dialog.set(false);
        spawn_local(async move {
            let body = serde_json::json!({
                "title": title,
                "folder": folder,
                "content": "",
            });
            match api::fetch_json_with_method("/api/notes", "POST", Some(&body.to_string())).await {
                Ok(val) => {
                    if let Some(id) = val.get("id").and_then(|i| i.as_str()) {
                        open_note(id.to_string());
                    }
                    fetch_notes();
                }
                Err(e) => {
                    set_error.set(e);
                }
            }
        });
    };

    let update_note_content = move |content: String| {
        if let Some(ref note) = selected_note.get() {
            let note_id = note.id.clone();
            let title = note.title.clone();
            let folder = note.folder.clone();
            let tags = note.tags.clone();
            spawn_local(async move {
                let body = serde_json::json!({
                    "title": title,
                    "content": content,
                    "folder": folder,
                    "tags": tags,
                });
                let _ = api::fetch_json_with_method(&format!("/api/notes/{}", note_id), "PUT", Some(&body.to_string()))
                    .await;
            });
        }
    };

    let delete_note = move |id: String| {
        spawn_local(async move {
            let _ = api::fetch_json_with_method(&format!("/api/notes/{}", id), "DELETE", None).await;
            set_selected_note_id.set(None);
            set_selected_note.set(None);
            fetch_notes();
        });
    };

    let folders = move || {
        let mut f: Vec<String> = notes
            .get()
            .iter()
            .map(|n| n.folder.clone())
            .filter(|f| !f.is_empty())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        f.sort();
        f
    };

    view! {
        <div class="h-screen flex flex-col bg-gray-100 dark:bg-gray-900">
            <a href="#main-content" class="sr-only focus:not-sr-only focus:absolute focus:top-2 focus:left-2 focus:z-50 focus:px-4 focus:py-2 focus:bg-blue-600 focus:text-white focus:rounded">{t!("nav.skip_to_content")}</a>
            <Header />
            <div class="flex-1 overflow-hidden px-2 sm:px-4 pt-16">
                <main id="main-content" class="h-full flex">
                    // Sidebar
                    <div class="w-72 flex-shrink-0 flex flex-col border-r border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800">
                        // Sidebar header
                        <div class="p-3 border-b border-gray-200 dark:border-gray-700">
                            <div class="flex items-center justify-between mb-3">
                                <h2 class="text-lg font-bold font-mono text-gray-900 dark:text-white">{t!("notes.title")}</h2>
                                <button
                                    on:click=move |_: ev::MouseEvent| {
                                        set_create_title.set(String::new());
                                        set_create_folder.set(String::new());
                                        set_show_create_dialog.set(true);
                                    }
                                    class="p-1.5 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
                                    title="New Note"
                                >
                                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" /></svg>
                                </button>
                            </div>
                            // Search
                            <div class="relative">
                                <svg class="absolute left-2.5 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" /></svg>
                                <input
                                    type="text"
                                    placeholder="Search notes..."
                                    prop:value=move || search_query.get()
                                    on:input=move |ev| set_search_query.set(event_target_value(&ev))
                                    class="w-full pl-8 pr-3 py-1.5 text-sm border border-gray-300 dark:border-gray-600 rounded-lg bg-gray-50 dark:bg-gray-700 text-gray-900 dark:text-white placeholder-gray-400 focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                                />
                            </div>
                        </div>

                        // Sort controls
                        <div class="px-3 py-2 border-b border-gray-200 dark:border-gray-700 flex items-center gap-2">
                            <select
                                prop:value=move || match sort_by.get() {
                                    SortBy::UpdatedAt => "updated_at",
                                    SortBy::Title => "title",
                                    SortBy::CreatedAt => "created_at",
                                }
                                on:change=move |ev| {
                                    let val = event_target_value(&ev);
                                    set_sort_by.set(match val.as_str() {
                                        "title" => SortBy::Title,
                                        "created_at" => SortBy::CreatedAt,
                                        _ => SortBy::UpdatedAt,
                                    });
                                }
                                class="text-xs px-2 py-1 border border-gray-300 dark:border-gray-600 rounded bg-white dark:bg-gray-700 text-gray-700 dark:text-gray-300"
                            >
                                <option value="updated_at">Updated</option>
                                <option value="created_at">Created</option>
                                <option value="title">Title</option>
                            </select>
                            <button
                                on:click=move |_| {
                                    set_sort_order.update(|o| {
                                        *o = if *o == "desc" { "asc".to_string() } else { "desc".to_string() };
                                    });
                                }
                                class="p-1 text-gray-500 hover:text-gray-700 dark:hover:text-gray-300"
                            >
                                {move || if sort_order.get() == "desc" {
                                    view! { <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" /></svg> }.into_any()
                                } else {
                                    view! { <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 15l7-7 7 7" /></svg> }.into_any()
                                }}
                            </button>
                        </div>

                        // Folder filter
                        {move || {
                            let f = folders();
                            if f.is_empty() {
                                return {
                                    ().into_any()
                                };
                            }
                            view! {
                                <div class="px-3 py-2 border-b border-gray-200 dark:border-gray-700">
                                    <select
                                        prop:value=move || folder_filter.get()
                                        on:change=move |ev| set_folder_filter.set(event_target_value(&ev))
                                        class="w-full text-xs px-2 py-1 border border-gray-300 dark:border-gray-600 rounded bg-white dark:bg-gray-700 text-gray-700 dark:text-gray-300"
                                    >
                                        <option value="">All folders</option>
                                        {f.into_iter().map(|folder| {
                                            let folder_clone = folder.clone();
                                            view! { <option value={folder_clone}>{folder}</option> }
                                        }).collect::<Vec<_>>()}
                                    </select>
                                </div>
                            }.into_any()
                        }}

                        // Notes list
                        <div class="flex-1 overflow-y-auto">
                            {move || loading.get().then(|| view! {
                                <div class="flex items-center justify-center py-8">
                                    <div class="text-sm text-gray-500 font-mono">{t!("common.loading")}</div>
                                </div>
                            })}

                            <For
                                each=move || notes.get()
                                key=|n| n.id.clone()
                                let:note
                            >
                                {
                                    let note_id = note.id.clone();
                                    let is_selected = move || selected_note_id.get() == Some(note_id.clone());
                                    let note_clone = note.clone();
                                    view! {
                                        <div
                                            class=move || format!("px-3 py-2 cursor-pointer border-b border-gray-100 dark:border-gray-700/50 transition-colors {}",
                                                if is_selected() { "bg-blue-50 dark:bg-blue-900/20" } else { "hover:bg-gray-50 dark:hover:bg-gray-700/50" }
                                            )
                                            on:click=move |_: ev::MouseEvent| open_note(note_clone.id.clone())
                                        >
                                            <div class="text-sm font-medium text-gray-900 dark:text-white truncate">{note.title}</div>
                                            <div class="flex items-center gap-2 mt-1">
                                                {if !note.folder.is_empty() {
                                                    view! { <span class="text-xs text-blue-500 dark:text-blue-400">{note.folder}</span> }.into_any()
                                                } else {
                                                    ().into_any()
                                                }}
                                                <span class="text-xs text-gray-400">{note.updated_at[..10.min(note.updated_at.len())].to_string()}</span>
                                            </div>
                                            {if !note.tags.is_empty() {
                                                view! {
                                                    <div class="flex flex-wrap gap-1 mt-1">
                                                        {note.tags.split(',').filter(|t| !t.trim().is_empty()).map(|tag| {
                                                            view! { <span class="text-xs px-1.5 py-0.5 bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-400 rounded">{tag.trim()}</span> }
                                                        }).collect::<Vec<_>>()}
                                                    </div>
                                                }.into_any()
                                            } else {
                                                ().into_any()
                                            }}
                                        </div>
                                    }
                                }
                            </For>
                        </div>
                    </div>

                    // Main content area
                    <div class="flex-1 flex flex-col overflow-hidden">
                        {move || {
                            if let Some(ref note) = selected_note.get() {
                                let note_clone = note.clone();
                                let note_id_for_delete = note.id.clone();
                                let note_content = note_clone.content.clone();
                                let note_title = note_clone.title.clone();
                                let note_id = note_clone.id.clone();
                                let note_folder = note_clone.folder.clone();
                                let note_tags = note_clone.tags.clone();
                                let note_created = note_clone.created_at.clone();
                                let note_updated = note_clone.updated_at.clone();
                                let note_content_for_editor = note_content.clone();
                                view! {
                                    <div class="flex-1 flex flex-col overflow-hidden">
                                        // Note header
                                        <div class="px-6 py-4 border-b border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800">
                                            <div class="flex items-center justify-between">
                                                <div class="flex-1">
                                                    <input
                                                        type="text"
                                                        prop:value=move || note_title.clone()
                                                        on:change=move |ev| {
                                                            let new_title = event_target_value(&ev);
                                                            let nid = note_id.clone();
                                                            let c = note_content.clone();
                                                            let f = note_folder.clone();
                                                            let t = note_tags.clone();
                                                            spawn_local(async move {
                                                                let body = serde_json::json!({
                                                                    "title": new_title,
                                                                    "content": c,
                                                                    "folder": f,
                                                                    "tags": t,
                                                                });
                                                                let _ = api::fetch_json_with_method(
                                                                    &format!("/api/notes/{}", nid),
                                                                    "PUT",
                                                                    Some(&body.to_string()),
                                                                )
                                                                .await;
                                                                fetch_notes();
                                                            });
                                                        }
                                                        class="text-xl font-bold font-mono text-gray-900 dark:text-white bg-transparent border-none focus:outline-none focus:ring-0 w-full"
                                                    />
                                                    <div class="flex items-center gap-4 mt-2 text-xs text-gray-500">
                                                        <span>"Created: " {note_created[..10.min(note_created.len())].to_string()}</span>
                                                        <span>"Updated: " {note_updated[..10.min(note_updated.len())].to_string()}</span>
                                                    </div>
                                                </div>
                                                <div class="flex items-center gap-2 ml-4">
                                                    <button
                                                        on:click=move |_: ev::MouseEvent| delete_note(note_id_for_delete.clone())
                                                        class="p-2 text-red-500 hover:text-red-700 hover:bg-red-50 dark:hover:bg-red-900/20 rounded-lg transition-colors"
                                                        title="Delete note"
                                                    >
                                                        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" /></svg>
                                                    </button>
                                                </div>
                                            </div>
                                        </div>

                                        // Markdown editor
                                        <div class="flex-1 overflow-hidden">
                                        <MarkdownEditor
                                            initial_content=note_content_for_editor
                                            on_change=Box::new(update_note_content)
                                        />
                                        </div>
                                    </div>
                                }.into_any()
                            } else {
                                view! {
                                    <div class="flex-1 flex items-center justify-center bg-white dark:bg-gray-800">
                                        <div class="text-center">
                                            <svg class="w-16 h-16 mx-auto text-gray-300 dark:text-gray-600 mb-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" /></svg>
                                            <p class="text-gray-500">Select a note or create a new one</p>
                                        </div>
                                    </div>
                                }.into_any()
                            }
                        }}
                    </div>
                </main>
            </div>

            // Create note dialog
            {move || show_create_dialog.get().then(|| view! {
                <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50" on:click=move |_: ev::MouseEvent| set_show_create_dialog.set(false)>
                    <div class="bg-white dark:bg-gray-800 rounded-xl shadow-xl max-w-md w-full mx-4 p-6" on:click=move |e: ev::MouseEvent| e.stop_propagation()>
                        <h3 class="text-lg font-bold font-mono text-gray-900 dark:text-white mb-4">{t!("notes.new_note")}</h3>
                        <div class="space-y-4">
                            <div>
                                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">{t!("notes.title")}</label>
                                <input
                                    type="text"
                                    prop:value=move || create_title.get()
                                    on:input=move |ev| set_create_title.set(event_target_value(&ev))
                                    class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                                    placeholder="Note title"
                                />
                            </div>
                            <div>
                                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">{t!("notes.folder")}</label>
                                <input
                                    type="text"
                                    prop:value=move || create_folder.get()
                                    on:input=move |ev| set_create_folder.set(event_target_value(&ev))
                                    class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                                    placeholder="Folder (optional)"
                                />
                            </div>
                        </div>
                        <div class="flex items-center justify-end gap-3 mt-6">
                            <button
                                on:click=move |_: ev::MouseEvent| set_show_create_dialog.set(false)
                                class="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors"
                            >
                                {t!("common.cancel")}
                            </button>
                            <button
                                on:click=create_note
                                class="px-4 py-2 text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 rounded-lg transition-colors"
                            >
                                {t!("common.save")}
                            </button>
                        </div>
                    </div>
                </div>
            })}
        </div>
    }
}

fn urlencoding(s: &str) -> String {
    s.replace(' ', "%20")
        .replace('/', "%2F")
        .replace('&', "%26")
        .replace('?', "%3F")
        .replace('#', "%23")
}
