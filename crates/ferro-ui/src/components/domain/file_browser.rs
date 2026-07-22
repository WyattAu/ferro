use leptos::prelude::*;
use crate::api::endpoints::FileEntry;
use crate::components::primitives::Spinner;

/// File browser with list/grid views, breadcrumb, selection.
#[component]
pub fn FileBrowser() -> impl IntoView {
    let (entries, set_entries) = signal(Vec::<FileEntry>::new());
    let (loading, set_loading) = signal(true);
    let (current_path, set_current_path) = signal("/".to_string());
    let (view_mode, set_view_mode) = signal("list".to_string());
    let (selected, set_selected) = signal(std::collections::HashSet::<String>::new());
    let (error, set_error) = signal(None::<String>);

    // Load directory on mount and path change
    Effect::new(move |_| {
        let path = current_path.get();
        set_loading.set(true);
        set_error.set(None);

        #[cfg(target_arch = "wasm32")]
        {
            let set = set_entries;
            let set_l = set_loading;
            let set_e = set_error;
            let p = path.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let client = crate::api::ApiClient::new(crate::api::ApiClientConfig::default());
                match client.get::<crate::api::endpoints::ListFilesResponse>(
                    &format!("/api/v1/files?path={}", urlencoding::encode(&p)),
                ).await {
                    Ok(resp) => {
                        set.set(resp.entries);
                        set_l.set(false);
                    }
                    Err(e) => {
                        set_e.set(Some(e.to_string()));
                        set_l.set(false);
                    }
                }
            });
        }
    });

    let navigate = move |path: String| {
        set_current_path.set(path);
        set_selected.set(std::collections::HashSet::new());
    };

    let toggle_select = move |path: String| {
        set_selected.update(|s| {
            if s.contains(&path) {
                s.remove(&path);
            } else {
                s.insert(path);
            }
        });
    };

    let select_all = move |_| {
        let all: std::collections::HashSet<String> =
            entries.get().iter().map(|e| e.path.clone()).collect();
        set_selected.set(all);
    };

    let clear_selection = move |_| {
        set_selected.set(std::collections::HashSet::new());
    };

    view! {
        <div class="flex flex-col h-full">
            // Toolbar
            <div class="flex items-center gap-3 px-4 py-2 border-b border-[var(--color-border)]">
                <button class="btn btn-ghost btn-sm" on:click=move |_| navigate("/".to_string())>
                    "🏠 Home"
                </button>
                <Breadcrumb path=current_path navigate=Callback::new(navigate.clone()) />
                <div class="ml-auto flex items-center gap-2">
                    <button
                        class=move || format!("btn btn-ghost btn-sm {}", if view_mode.get() == "list" { "btn-primary" } else { "" })
                        on:click=move |_| set_view_mode.set("list".to_string())
                    >
                        "☰ List"
                    </button>
                    <button
                        class=move || format!("btn btn-ghost btn-sm {}", if view_mode.get() == "grid" { "btn-primary" } else { "" })
                        on:click=move |_| set_view_mode.set("grid".to_string())
                    >
                        "⊞ Grid"
                    </button>
                    <span class="text-sm text-secondary">
                        {move || format!("{} items", entries.get().len())}
                    </span>
                </div>
            </div>

            // Selection bar
            {move || {
                let count = selected.get().len();
                if count > 0 {
                    view! {
                        <div class="flex items-center gap-3 px-4 py-2 bg-accent-subtle border-b border-[var(--color-border)]">
                            <span class="text-sm font-medium">{format!("{} selected", count)}</span>
                            <button class="btn btn-ghost btn-sm" on:click=clear_selection>"Clear"</button>
                            <button class="btn btn-ghost btn-sm" on:click=select_all>"Select all"</button>
                        </div>
                    }.into_any()
                } else {
                    view! { <></> }.into_any()
                }
            }}

            // File list
            <div class="flex-1 overflow-y-auto">
                {move || {
                    if loading.get() {
                        view! {
                            <div class="p-8 text-center">
                                <Spinner />
                                <p class="text-secondary mt-2">"Loading..."</p>
                            </div>
                        }.into_any()
                    } else if let Some(err) = error.get() {
                        view! {
                            <div class="p-8 text-center text-danger">
                                <p>{format!("Error: {}", err)}</p>
                            </div>
                        }.into_any()
                    } else if entries.get().is_empty() {
                        view! {
                            <EmptyState dir_path=current_path.get() />
                        }.into_any()
                    } else if view_mode.get() == "grid" {
                        view! {
                            <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-6 gap-3 p-4">
                                {entries.get().into_iter().map(|entry| {
                                    let p = entry.path.clone();
                                    let is_sel = move || selected.get().contains(&p);
                                    view! {
                                        <FileGridItem
                                            entry=entry
                                            is_selected=is_sel
                                            on_click=Callback::new(move |path: String| toggle_select(path.clone()))
                                            on_navigate=Callback::new(navigate.clone())
                                        />
                                    }
                                }).collect_view()}
                            </div>
                        }.into_any()
                    } else {
                        view! {
                            <table class="table w-full">
                                <thead>
                                    <tr>
                                        <th class="w-10"></th>
                                        <th>"Name"</th>
                                        <th class="w-24">"Size"</th>
                                        <th class="w-40">"Modified"</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {entries.get().into_iter().map(|entry| {
                                        let p = entry.path.clone();
                                        let is_sel = move || selected.get().contains(&p);
                                        view! {
                                            <FileRow
                                                entry=entry
                                                is_selected=is_sel
                                                on_click=Callback::new(move |path: String| toggle_select(path.clone()))
                                                on_navigate=Callback::new(navigate.clone())
                                            />
                                        }
                                    }).collect_view()}
                                </tbody>
                            </table>
                        }.into_any()
                    }
                }}
            </div>
        </div>
    }
}

/// Breadcrumb navigation component.
#[component]
fn Breadcrumb(
    path: ReadSignal<String>,
    navigate: Callback<String>,
) -> impl IntoView {
    view! {
        <nav class="flex items-center gap-1 text-sm" aria-label="Breadcrumb">
            {move || {
                let p = path.get();
                let parts: Vec<&str> = p.trim_start_matches('/').split('/').filter(|s| !s.is_empty()).collect();
                let mut acc = String::new();
                let mut items = Vec::new();

                // Root
                items.push(view! {
                    <button class="nav-link" on:click=move |_| navigate.run("/".to_string())>"Root"</button>
                }.into_any());

                for part in &parts {
                    acc.push('/');
                    acc.push_str(part);
                    let full_path = acc.clone();
                    let name = part.to_string();
                    items.push(view! {
                        <span class="text-tertiary">"/"</span>
                    }.into_any());
                    items.push(view! {
                        <button class="nav-link" on:click=move |_| navigate.run(full_path.clone())>{name}</button>
                    }.into_any());
                }

                items.into_view()
            }}
        </nav>
    }
}

/// Empty state when directory has no files.
#[component]
fn EmptyState(#[prop(into)] dir_path: String) -> impl IntoView {
    let _ = dir_path;
    view! {
        <div class="p-12 text-center">
            <div class="text-6xl mb-4">"📁"</div>
            <h3 class="text-lg font-semibold mb-2">"This folder is empty"</h3>
            <p class="text-secondary mb-4">"Drop files here or upload your first file"</p>
        </div>
    }
}

/// File row in list view.
#[component]
fn FileRow(
    entry: FileEntry,
    #[prop(into)] is_selected: Signal<bool>,
    on_click: Callback<String>,
    on_navigate: Callback<String>,
) -> impl IntoView {
    let icon = if entry.is_collection { "📁" } else { file_icon(&entry.name) };
    let path = entry.path.clone();
    let path2 = entry.path.clone();
    let path3 = entry.path.clone();
    let is_dir = entry.is_collection;

    view! {
        <tr
            class=move || format!("cursor-pointer hover:bg-[var(--color-bg-sunken)] {}",
                if is_selected.get() { "bg-accent-subtle" } else { "" })
            on:click=move |_| {
                if is_dir {
                    on_navigate.run(path.clone());
                } else {
                    on_click.run(path.clone());
                }
            }
        >
            <td class="text-center">{icon}</td>
            <td class="font-medium truncate">{entry.name}</td>
            <td class="text-secondary text-sm">{if entry.is_collection { "--".to_string() } else { format_size(entry.size) }}</td>
            <td class="text-secondary text-sm whitespace-nowrap">{entry.modified_at}</td>
        </tr>
    }
}

/// File card in grid view.
#[component]
fn FileGridItem(
    entry: FileEntry,
    #[prop(into)] is_selected: Signal<bool>,
    on_click: Callback<String>,
    on_navigate: Callback<String>,
) -> impl IntoView {
    let icon = if entry.is_collection { "📁" } else { file_icon(&entry.name) };
    let path = entry.path.clone();
    let is_dir = entry.is_collection;

    view! {
        <div
            class=move || format!("card cursor-pointer transition-all hover:shadow-md {}",
                if is_selected.get() { "ring-2 ring-accent" } else { "" })
            on:click=move |_| {
                if is_dir {
                    on_navigate.run(path.clone());
                } else {
                    on_click.run(path.clone());
                }
            }
        >
            <div class="text-4xl text-center mb-2">{icon}</div>
            <p class="text-sm font-medium truncate text-center">{entry.name}</p>
            <p class="text-xs text-secondary text-center">
                {if entry.is_collection { "--".to_string() } else { format_size(entry.size) }}
            </p>
        </div>
    }
}

/// Format file size to human-readable.
fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

/// File type icon.
fn file_icon(name: &str) -> &'static str {
    let ext = name.rsplit('.').next().unwrap_or("");
    match ext {
        "pdf" => "📄",
        "doc" | "docx" => "📝",
        "xls" | "xlsx" => "📊",
        "ppt" | "pptx" => "📈",
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "svg" => "🖼️",
        "mp4" | "avi" | "mov" | "mkv" => "🎬",
        "mp3" | "wav" | "flac" | "ogg" => "🎵",
        "zip" | "tar" | "gz" | "7z" => "📦",
        "rs" | "py" | "js" | "ts" | "go" | "java" | "c" | "cpp" | "h" => "💻",
        "md" | "txt" | "log" => "📄",
        "html" | "css" | "json" | "toml" | "yaml" | "yml" => "🌐",
        _ => "📄",
    }
}
