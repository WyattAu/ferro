use leptos::ev;
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::api;
use crate::components::header::{Header, provide_header_state};
use crate::components::theme_toggle::provide_theme_state;
use crate::t;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Photo {
    pub id: String,
    pub path: String,
    pub name: String,
    pub size: u64,
    pub mime_type: String,
    pub taken_at: Option<String>,
    pub modified_at: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Album {
    pub id: String,
    pub name: String,
    pub description: String,
    pub photo_paths: Vec<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExifData {
    pub camera_make: Option<String>,
    pub camera_model: Option<String>,
    pub date_taken: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[derive(Debug, Clone, PartialEq)]
enum ViewMode {
    Timeline,
    Grid,
    Albums,
}

#[component]
pub fn PhotosPage() -> impl IntoView {
    provide_theme_state();
    provide_header_state();

    let (loading, set_loading) = signal(true);
    let (photos, set_photos) = signal(Vec::<Photo>::new());
    let (albums, set_albums) = signal(Vec::<Album>::new());
    let (view_mode, set_view_mode) = signal(ViewMode::Grid);
    let (selected_photo, set_selected_photo) = signal(None::<Photo>);
    let (show_exif, set_show_exif) = signal(false);
    let (exif_data, set_exif_data) = signal(None::<ExifData>);
    let (date_start, set_date_start) = signal(String::new());
    let (date_end, set_date_end) = signal(String::new());
    let (_error_msg, set_error) = signal(String::new());
    let (show_create_album, set_show_create_album) = signal(false);
    let (new_album_name, set_new_album_name) = signal(String::new());
    let (new_album_description, set_new_album_description) = signal(String::new());

    let fetch_photos = move || {
        set_loading.set(true);
        spawn_local(async move {
            let mut url = "/api/photos".to_string();
            let mut params = Vec::new();
            let start = date_start.get();
            let end = date_end.get();
            if !start.is_empty() {
                params.push(format!("start={}", start));
            }
            if !end.is_empty() {
                params.push(format!("end={}", end));
            }
            if !params.is_empty() {
                url.push('?');
                url.push_str(&params.join("&"));
            }

            match api::fetch_json(&url).await {
                Ok(val) => {
                    let photos_list = val
                        .get("photos")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| {
                                    Some(Photo {
                                        id: v.get("id")?.as_str()?.to_string(),
                                        path: v.get("path").and_then(|p| p.as_str()).unwrap_or("").to_string(),
                                        name: v.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string(),
                                        size: v.get("size").and_then(|s| s.as_u64()).unwrap_or(0),
                                        mime_type: v
                                            .get("mime_type")
                                            .and_then(|m| m.as_str())
                                            .unwrap_or("")
                                            .to_string(),
                                        taken_at: v.get("taken_at").and_then(|t| t.as_str()).map(|s| s.to_string()),
                                        modified_at: v
                                            .get("modified_at")
                                            .and_then(|m| m.as_str())
                                            .unwrap_or("")
                                            .to_string(),
                                        width: v.get("width").and_then(|w| w.as_u64()).map(|v| v as u32),
                                        height: v.get("height").and_then(|h| h.as_u64()).map(|v| v as u32),
                                    })
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    set_photos.set(photos_list);
                    set_loading.set(false);
                }
                Err(e) => {
                    set_error.set(e);
                    set_loading.set(false);
                }
            }
        });
    };

    let fetch_albums = move || {
        spawn_local(async move {
            if let Ok(val) = api::fetch_json("/api/photos/albums").await {
                let albums_list = val
                    .get("albums")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| {
                                Some(Album {
                                    id: v.get("id")?.as_str()?.to_string(),
                                    name: v.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string(),
                                    description: v
                                        .get("description")
                                        .and_then(|d| d.as_str())
                                        .unwrap_or("")
                                        .to_string(),
                                    photo_paths: v
                                        .get("photo_paths")
                                        .and_then(|p| p.as_array())
                                        .map(|arr| {
                                            arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()
                                        })
                                        .unwrap_or_default(),
                                    created_at: v.get("created_at").and_then(|c| c.as_str()).unwrap_or("").to_string(),
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                set_albums.set(albums_list);
            }
        });
    };

    let fetch_exif = move |photo_path: String| {
        spawn_local(async move {
            match api::fetch_json(&format!("/api/photos/exif/{}", photo_path.trim_start_matches('/'))).await {
                Ok(val) => {
                    if let Ok(exif) = serde_json::from_value::<ExifData>(val) {
                        set_exif_data.set(Some(exif));
                    }
                }
                Err(e) => {
                    set_error.set(e);
                }
            }
        });
    };

    let open_lightbox = move |photo: Photo| {
        fetch_exif(photo.path.clone());
        set_selected_photo.set(Some(photo));
        set_show_exif.set(false);
    };

    let close_lightbox = move |_: ev::MouseEvent| {
        set_selected_photo.set(None);
        set_exif_data.set(None);
    };

    let create_album = move |_: ev::MouseEvent| {
        let name = new_album_name.get();
        if name.trim().is_empty() {
            return;
        }
        set_show_create_album.set(false);
        spawn_local(async move {
            let body = serde_json::json!({
                "name": name,
                "description": new_album_description.get(),
            });
            match api::fetch_json_with_method("/api/photos/albums", "POST", Some(&body.to_string())).await {
                Ok(_) => {
                    fetch_albums();
                }
                Err(e) => {
                    set_error.set(e);
                }
            }
        });
    };

    let group_by_date = move || -> Vec<(String, Vec<Photo>)> {
        let mut groups: std::collections::BTreeMap<String, Vec<Photo>> = std::collections::BTreeMap::new();
        for photo in photos.get() {
            let date = photo.modified_at[..10.min(photo.modified_at.len())].to_string();
            groups.entry(date).or_default().push(photo);
        }
        groups.into_iter().rev().collect()
    };

    Effect::new(move |_| {
        let _ = date_start.get();
        let _ = date_end.get();
        fetch_photos();
        fetch_albums();
    });

    view! {
        <div class="h-screen flex flex-col bg-[var(--bg-base)]">
            <a href="#main-content" class="sr-only focus:not-sr-only focus:absolute focus:top-2 focus:left-2 focus:z-50 focus:px-4 focus:py-2 focus:bg-[var(--accent)] focus:text-[var(--text-on-accent)] focus:rounded">{t!("nav.skip_to_content")}</a>
            <Header />
            <div class="flex-1 overflow-hidden px-2 sm:px-4 pt-16">
                <main id="main-content" class="h-full flex">
                    // Sidebar
                    <div class="w-72 flex-shrink-0 flex flex-col border-r border-[var(--border-default)] bg-[var(--bg-surface)]">
                        <div class="p-3 border-b border-[var(--border-default)]">
                            <div class="flex items-center justify-between mb-3">
                                <h2 class="text-lg font-bold font-mono text-[var(--text-primary)]">"Photos"</h2>
                                <button
                                    on:click=move |_: ev::MouseEvent| {
                                        set_new_album_name.set(String::new());
                                        set_new_album_description.set(String::new());
                                        set_show_create_album.set(true);
                                    }
                                    class="p-1.5 bg-[var(--accent)] text-[var(--text-on-accent)] rounded-lg hover:bg-[var(--accent-hover)] transition-colors"
                                    title="New Album"
                                >
                                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" /></svg>
                                </button>
                            </div>
                        </div>

                        // View mode toggle
                        <div class="px-3 py-2 border-b border-[var(--border-default)]">
                            <div class="flex gap-1 bg-[var(--bg-inset)] bg-[var(--bg-surface-raised)] rounded-lg p-1">
                                <button
                                    on:click=move |_| set_view_mode.set(ViewMode::Grid)
                                    class=move || format!("flex-1 px-3 py-1.5 text-xs font-medium rounded-md transition-colors {}",
                                        if view_mode.get() == ViewMode::Grid { "bg-[var(--bg-surface)] dark:bg-[var(--text-tertiary)] text-[var(--text-primary)] shadow" } else { "text-[var(--text-tertiary)] hover:text-[var(--text-secondary)] hover:text-[var(--text-secondary)]" }
                                    )
                                >
                                    "Grid"
                                </button>
                                <button
                                    on:click=move |_| set_view_mode.set(ViewMode::Timeline)
                                    class=move || format!("flex-1 px-3 py-1.5 text-xs font-medium rounded-md transition-colors {}",
                                        if view_mode.get() == ViewMode::Timeline { "bg-[var(--bg-surface)] dark:bg-[var(--text-tertiary)] text-[var(--text-primary)] shadow" } else { "text-[var(--text-tertiary)] hover:text-[var(--text-secondary)] hover:text-[var(--text-secondary)]" }
                                    )
                                >
                                    "Timeline"
                                </button>
                                <button
                                    on:click=move |_| set_view_mode.set(ViewMode::Albums)
                                    class=move || format!("flex-1 px-3 py-1.5 text-xs font-medium rounded-md transition-colors {}",
                                        if view_mode.get() == ViewMode::Albums { "bg-[var(--bg-surface)] dark:bg-[var(--text-tertiary)] text-[var(--text-primary)] shadow" } else { "text-[var(--text-tertiary)] hover:text-[var(--text-secondary)] hover:text-[var(--text-secondary)]" }
                                    )
                                >
                                    "Albums"
                                </button>
                            </div>
                        </div>

                        // Date range filter
                        <div class="px-3 py-2 border-b border-[var(--border-default)] space-y-2">
                            <div>
                                <label class="block text-xs text-[var(--text-tertiary)] mb-1">"From"</label>
                                <input
                                    type="date"
                                    prop:value=move || date_start.get()
                                    on:input=move |ev| set_date_start.set(event_target_value(&ev))
                                    class="w-full text-xs px-2 py-1 border border-[var(--border-default)] rounded bg-[var(--bg-surface)] text-[var(--text-secondary)]"
                                />
                            </div>
                            <div>
                                <label class="block text-xs text-[var(--text-tertiary)] mb-1">"To"</label>
                                <input
                                    type="date"
                                    prop:value=move || date_end.get()
                                    on:input=move |ev| set_date_end.set(event_target_value(&ev))
                                    class="w-full text-xs px-2 py-1 border border-[var(--border-default)] rounded bg-[var(--bg-surface)] text-[var(--text-secondary)]"
                                />
                            </div>
                        </div>

                        // Albums list
                        <div class="flex-1 overflow-y-auto">
                            <div class="px-3 py-2">
                                <h3 class="text-xs font-medium text-[var(--text-tertiary)] uppercase tracking-wider mb-2">"Albums"</h3>
                                <For
                                    each=move || albums.get()
                                    key=|a| a.id.clone()
                                    let:album
                                >
                                    {
                                        view! {
                                            <div class="px-2 py-1.5 rounded hover:bg-[var(--interactive-hover)] cursor-pointer transition-colors">
                                                <div class="text-sm text-[var(--text-secondary)]">{album.name}</div>
                                                <div class="text-xs text-[var(--text-tertiary)]">{album.photo_paths.len()}" photos"</div>
                                            </div>
                                        }
                                    }
                                </For>
                            </div>
                        </div>
                    </div>

                    // Main content area
                    <div class="flex-1 flex flex-col overflow-hidden bg-[var(--bg-surface)]">
                        {move || loading.get().then(|| view! {
                            <div class="flex items-center justify-center py-8">
                                <div class="text-sm text-[var(--text-tertiary)] font-mono">{t!("common.loading")}</div>
                            </div>
                        })}

                        {move || {
                            match view_mode.get() {
                                ViewMode::Grid => {
                                    view! {
                                        <div class="flex-1 overflow-y-auto p-4">
                                            <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 gap-4">
                                                <For
                                                    each=move || photos.get()
                                                    key=|p| p.id.clone()
                                                    let:photo
                                                >
                                                    {
                                                        let photo_clone = photo.clone();
                                                        view! {
                                                            <div
                                                                class="aspect-square rounded-lg overflow-hidden cursor-pointer hover:ring-2 hover:ring-blue-500 transition-all bg-[var(--bg-inset)] bg-[var(--bg-surface-raised)]"
                                                                on:click=move |_: ev::MouseEvent| open_lightbox(photo_clone.clone())
                                                            >
                                                                <img
                                                                    src=format!("/api/photos/thumbnail/{}", photo.path.trim_start_matches('/'))
                                                                    alt=photo.name.clone()
                                                                    class="w-full h-full object-cover"
                                                                    loading="lazy"
                                                                />
                                                            </div>
                                                        }
                                                    }
                                                </For>
                                            </div>
                                        </div>
                                    }.into_any()
                                }
                                ViewMode::Timeline => {
                                    let groups = group_by_date();
                                    view! {
                                        <div class="flex-1 overflow-y-auto p-4 space-y-8">
                                            {groups.into_iter().map(|(date, photos)| {
                                                view! {
                                                    <div>
                                                        <h3 class="text-sm font-medium text-[var(--text-tertiary)] dark:text-[var(--text-tertiary)] mb-3 sticky top-0 bg-[var(--bg-surface)] py-1">{date}</h3>
                                                        <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 gap-4">
                                                            {photos.into_iter().map(|photo| {
                                                                let photo_clone = photo.clone();
                                                                view! {
                                                                    <div
                                                                        class="aspect-square rounded-lg overflow-hidden cursor-pointer hover:ring-2 hover:ring-blue-500 transition-all bg-[var(--bg-inset)] bg-[var(--bg-surface-raised)]"
                                                                        on:click=move |_: ev::MouseEvent| open_lightbox(photo_clone.clone())
                                                                    >
                                                                        <img
                                                                            src=format!("/api/photos/thumbnail/{}", photo.path.trim_start_matches('/'))
                                                                            alt=photo.name.clone()
                                                                            class="w-full h-full object-cover"
                                                                            loading="lazy"
                                                                        />
                                                                    </div>
                                                                }
                                                            }).collect::<Vec<_>>()}
                                                        </div>
                                                    </div>
                                                }
                                            }).collect::<Vec<_>>()}
                                        </div>
                                    }.into_any()
                                }
                                ViewMode::Albums => {
                                    view! {
                                        <div class="flex-1 overflow-y-auto p-4">
                                            <div class="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 gap-4">
                                                <For
                                                    each=move || albums.get()
                                                    key=|a| a.id.clone()
                                                    let:album
                                                >
                                                    {
                                                        view! {
                                                            <div class="border border-[var(--border-default)] rounded-lg overflow-hidden hover:shadow-lg transition-shadow">
                                                                <div class="aspect-video bg-[var(--bg-inset)] bg-[var(--bg-surface-raised)] flex items-center justify-center">
                                                                    <svg class="w-12 h-12 text-[var(--text-tertiary)] dark:text-[var(--text-secondary)]" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" /></svg>
                                                                </div>
                                                                <div class="p-3">
                                                                    <div class="font-medium text-[var(--text-primary)]">{album.name}</div>
                                                                    <div class="text-sm text-[var(--text-tertiary)]">{album.photo_paths.len()}" photos"</div>
                                                                    {if !album.description.is_empty() {
                                                                        view! { <div class="text-xs text-[var(--text-tertiary)] mt-1 truncate">{album.description}</div> }.into_any()
                                                                    } else {
                                                                        ().into_any()
                                                                    }}
                                                                </div>
                                                            </div>
                                                        }
                                                    }
                                                </For>
                                            </div>
                                        </div>
                                    }.into_any()
                                }
                            }
                        }}
                    </div>
                </main>
            </div>

            // Lightbox
            {move || {
                if let Some(ref photo) = selected_photo.get() {
                    view! {
                        <div class="fixed inset-0 z-50 bg-black/90 flex items-center justify-center" on:click=close_lightbox>
                            <button
                                class="absolute top-4 right-4 text-[var(--text-on-accent)] hover:text-[var(--text-tertiary)] z-10"
                                on:click=close_lightbox
                            >
                                <svg class="w-8 h-8" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" /></svg>
                            </button>

                            <div class="max-w-4xl max-h-[80vh] mx-4" on:click=move |e: ev::MouseEvent| e.stop_propagation()>
                                <img
                                    src=format!("/api/photos/thumbnail/{}", photo.path.trim_start_matches('/'))
                                    alt=photo.name.clone()
                                    class="max-w-full max-h-[70vh] mx-auto rounded-lg shadow-2xl"
                                />

                                <div class="mt-4 flex items-center justify-between">
                                    <div class="text-[var(--text-on-accent)]">
                                        <div class="font-medium">{photo.name.clone()}</div>
                                        <div class="text-sm text-[var(--text-tertiary)]">{photo.modified_at[..10.min(photo.modified_at.len())].to_string()}</div>
                                    </div>
                                    <button
                                        on:click=move |_: ev::MouseEvent| {
                                            let current = show_exif.get();
                                            set_show_exif.set(!current);
                                        }
                                        class="px-3 py-1.5 bg-[var(--bg-surface)]/10 text-[var(--text-on-accent)] rounded-lg hover:bg-[var(--interactive-hover)]/20 transition-colors text-sm"
                                    >
                                        {move || if show_exif.get() { "Hide Info" } else { "Show Info" }}
                                    </button>
                                </div>

                                {move || {
                                    if show_exif.get() {
                                        if let Some(ref exif) = exif_data.get() {
                                            view! {
                                                <div class="mt-4 p-4 bg-[var(--bg-surface)]/10 rounded-lg text-[var(--text-on-accent)] text-sm space-y-2">
                                                    {if let Some(ref make) = exif.camera_make {
                                                        let make_clone = make.clone();
                                                        let model_clone = exif.camera_model.clone();
                                                        view! { <div>"Camera: " {make_clone} {if let Some(model) = model_clone { format!(" {}", model) } else { String::new() }}</div> }.into_any()
                                                    } else {
                                                        ().into_any()
                                                    }}
                                                    {if let Some(ref date) = exif.date_taken {
                                                        let date_clone = date.clone();
                                                        view! { <div>"Taken: " {date_clone}</div> }.into_any()
                                                    } else {
                                                        ().into_any()
                                                    }}
                                                    {if let (Some(lat), Some(lon)) = (exif.latitude, exif.longitude) {
                                                        view! { <div>"Location: " {format!("{:.6}, {:.6}", lat, lon)}</div> }.into_any()
                                                    } else {
                                                        ().into_any()
                                                    }}
                                                    {if let (Some(w), Some(h)) = (exif.width, exif.height) {
                                                        view! { <div>"Dimensions: " {w}" x "{h}</div> }.into_any()
                                                    } else {
                                                        ().into_any()
                                                    }}
                                                </div>
                                            }.into_any()
                                        } else {
                                            ().into_any()
                                        }
                                    } else {
                                        ().into_any()
                                    }
                                }}
                            </div>
                        </div>
                    }.into_any()
                } else {
                    ().into_any()
                }
            }}

            // Create album dialog
            {move || show_create_album.get().then(|| view! {
                <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50" on:click=move |_: ev::MouseEvent| set_show_create_album.set(false)>
                    <div class="bg-[var(--bg-surface)] rounded-xl shadow-xl max-w-md w-full mx-4 p-6" on:click=move |e: ev::MouseEvent| e.stop_propagation()>
                        <h3 class="text-lg font-bold font-mono text-[var(--text-primary)] mb-4">"New Album"</h3>
                        <div class="space-y-4">
                            <div>
                                <label class="block text-sm font-medium text-[var(--text-secondary)] mb-1">"Album Name"</label>
                                <input
                                    type="text"
                                    prop:value=move || new_album_name.get()
                                    on:input=move |ev| set_new_album_name.set(event_target_value(&ev))
                                    class="w-full px-3 py-2 border border-[var(--border-default)] rounded-lg bg-[var(--bg-surface)] text-[var(--text-primary)] focus:ring-2 focus:ring-[var(--border-focus)] focus:border-transparent"
                                    placeholder="Enter album name"
                                />
                            </div>
                            <div>
                                <label class="block text-sm font-medium text-[var(--text-secondary)] mb-1">"Description"</label>
                                <textarea
                                    prop:value=move || new_album_description.get()
                                    on:input=move |ev| set_new_album_description.set(event_target_value(&ev))
                                    class="w-full px-3 py-2 border border-[var(--border-default)] rounded-lg bg-[var(--bg-surface)] text-[var(--text-primary)] focus:ring-2 focus:ring-[var(--border-focus)] focus:border-transparent"
                                    placeholder="Optional description"
                                    rows="3"
                                ></textarea>
                            </div>
                        </div>
                        <div class="flex items-center justify-end gap-3 mt-6">
                            <button
                                on:click=move |_: ev::MouseEvent| set_show_create_album.set(false)
                                class="px-4 py-2 text-sm font-medium text-[var(--text-secondary)] hover:bg-[var(--interactive-hover)] rounded-lg transition-colors"
                            >
                                {t!("common.cancel")}
                            </button>
                            <button
                                on:click=create_album
                                class="px-4 py-2 text-sm font-medium text-[var(--text-on-accent)] bg-[var(--accent)] hover:bg-[var(--accent-hover)] rounded-lg transition-colors"
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
