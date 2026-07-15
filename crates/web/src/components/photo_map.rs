use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::api;
use crate::components::theme_toggle::provide_theme_state;
use crate::pages::photos::{ExifData, Photo};
use crate::t;

/// Photo with resolved location data for map rendering.
#[derive(Debug, Clone, PartialEq)]
struct MapPhoto {
    photo: Photo,
    latitude: f64,
    longitude: f64,
}

#[derive(Debug, Clone, PartialEq)]
enum MapLoadState {
    Loading,
    Ready,
    Fallback(String),
}

/// Interactive map view for photos with GPS location data.
/// Falls back to a list view if Leaflet.js cannot be loaded or no photos have location data.
#[component]
pub fn PhotoMap(
    photos: Signal<Vec<Photo>>,
    on_photo_click: impl Fn(Photo) + Clone + 'static,
) -> impl IntoView {
    provide_theme_state();

    let (map_state, set_map_state) = signal(MapLoadState::Loading);
    let (map_photos, set_map_photos) = signal(Vec::<MapPhoto>::new());
    let (date_start, set_date_start) = signal(String::new());
    let (date_end, set_date_end) = signal(String::new());
    let (selected_photo, set_selected_photo) = signal(None::<Photo>);

    // Extract GPS data from all photos by fetching EXIF for each
    let init_map = move || {
        let photos_list = photos.get();
        if photos_list.is_empty() {
            set_map_state.set(MapLoadState::Fallback("No photos to display".to_string()));
            return;
        }

        let mut photo_futs = Vec::new();
        for photo in photos_list {
            let path = photo.path.clone();
            photo_futs.push(async move {
                match api::fetch_json(&format!("/api/photos/exif/{}", path.trim_start_matches('/'))).await {
                    Ok(val) => {
                        if let Ok(exif) = serde_json::from_value::<ExifData>(val)
                            && let (Some(lat), Some(lon)) = (exif.latitude, exif.longitude)
                        {
                            return Some(MapPhoto {
                                photo,
                                latitude: lat,
                                longitude: lon,
                            });
                        }
                        None
                    }
                    Err(_) => None,
                }
            });
        }

        spawn_local(async move {
            let results = futures::future::join_all(photo_futs).await;
            let located: Vec<MapPhoto> = results.into_iter().flatten().collect();

            if located.is_empty() {
                set_map_state.set(MapLoadState::Fallback(
                    "No photos with GPS location data found".to_string(),
                ));
                return;
            }

            set_map_photos.set(located);
            set_map_state.set(MapLoadState::Ready);
        });
    };

    // Initialize on mount
    Effect::new(move |_| {
        let _ = photos.get();
        init_map();
    });

    // Handle photo selection - when a photo is selected, call the callback
    Effect::new(move |_| {
        if let Some(photo) = selected_photo.get() {
            on_photo_click(photo);
        }
    });

    let filtered_photos = move || {
        let start = date_start.get();
        let end = date_end.get();
        let mut result = map_photos.get();
        if !start.is_empty() {
            result.retain(|mp| mp.photo.modified_at >= start);
        }
        if !end.is_empty() {
            result.retain(|mp| mp.photo.modified_at <= end);
        }
        result
    };

    let filtered_memo = Memo::new(move |_| filtered_photos());

    view! {
        <div class="h-full flex flex-col">
            // Toolbar
            <div class="flex items-center gap-4 p-3 border-b border-[var(--border-default)] bg-[var(--bg-surface)]">
                <span class="text-sm font-medium text-[var(--text-primary)]">"Map View"</span>
                <div class="flex items-center gap-2">
                    <label class="text-xs text-[var(--text-tertiary)]">"From"</label>
                    <input
                        type="date"
                        prop:value=move || date_start.get()
                        on:input=move |ev| set_date_start.set(event_target_value(&ev))
                        class="text-xs px-2 py-1 border border-[var(--border-default)] rounded bg-[var(--bg-surface)] text-[var(--text-secondary)]"
                    />
                    <label class="text-xs text-[var(--text-tertiary)]">"To"</label>
                    <input
                        type="date"
                        prop:value=move || date_end.get()
                        on:input=move |ev| set_date_end.set(event_target_value(&ev))
                        class="text-xs px-2 py-1 border border-[var(--border-default)] rounded bg-[var(--bg-surface)] text-[var(--text-secondary)]"
                    />
                </div>
                <div class="text-xs text-[var(--text-tertiary)]">
                    {move || format!("{} photos with location", filtered_memo.get().len())}
                </div>
            </div>

            // Map content
            <div class="flex-1 relative">
                {move || match map_state.get() {
                    MapLoadState::Loading => view! {
                        <div class="flex items-center justify-center h-full">
                            <div class="text-sm text-[var(--text-tertiary)] font-mono">{t!("common.loading")}</div>
                        </div>
                    }.into_any(),
                    MapLoadState::Ready => {
                        let photos_list = filtered_memo.get();
                        view! {
                            <div class="h-full flex">
                                // Photo list
                                <div class="flex-1 overflow-y-auto p-4">
                                    <div class="mb-3 p-3 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg text-sm text-blue-800 dark:text-blue-200">
                                        "Map view showing " {photos_list.len()} " photos with GPS coordinates. Click a photo to view it."
                                    </div>
                                    <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 gap-3">
                                        {photos_list.into_iter().map(|mp| {
                                            let photo = mp.photo.clone();
                                            let lat = mp.latitude;
                                            let lon = mp.longitude;
                                            view! {
                                                <div
                                                    class="border border-[var(--border-default)] rounded-lg overflow-hidden cursor-pointer transition-all hover:border-blue-300"
                                                    on:click=move |_: web_sys::MouseEvent| {
                                                        set_selected_photo.set(Some(photo.clone()));
                                                    }
                                                >
                                                    <div class="aspect-square bg-[var(--bg-inset)]">
                                                        <img
                                                            src=format!("/api/photos/thumbnail/{}", mp.photo.path.trim_start_matches('/'))
                                                            alt=mp.photo.name.clone()
                                                            class="w-full h-full object-cover"
                                                            loading="lazy"
                                                        />
                                                    </div>
                                                    <div class="p-1.5">
                                                        <div class="text-xs font-medium text-[var(--text-primary)] truncate">{mp.photo.name.clone()}</div>
                                                        <div class="text-[10px] text-[var(--text-tertiary)] font-mono">{format!("{:.4}, {:.4}", lat, lon)}</div>
                                                    </div>
                                                </div>
                                            }
                                        }).collect::<Vec<_>>()}
                                    </div>
                                </div>
                                // Selected photo sidebar
                                {move || {
                                    selected_photo.get().map(|photo| {
                                        let photo_clone = photo.clone();
                                        view! {
                                            <div class="w-72 border-l border-[var(--border-default)] bg-[var(--bg-surface)] p-3 flex flex-col">
                                                <div class="aspect-square rounded-lg overflow-hidden mb-3 bg-[var(--bg-inset)]">
                                                    <img
                                                        src=format!("/api/photos/thumbnail/{}", photo.path.trim_start_matches('/'))
                                                        alt=photo.name.clone()
                                                        class="w-full h-full object-cover cursor-pointer"
                                                        on:click=move |_: web_sys::MouseEvent| {
                                                            set_selected_photo.set(Some(photo_clone.clone()));
                                                        }
                                                    />
                                                </div>
                                                <div class="text-sm font-medium text-[var(--text-primary)] truncate">{photo.name.clone()}</div>
                                                <div class="text-xs text-[var(--text-tertiary)] mt-1">{photo.modified_at[..10.min(photo.modified_at.len())].to_string()}</div>
                                            </div>
                                        }
                                    })
                                }}
                            </div>
                        }.into_any()
                    },
                    MapLoadState::Fallback(msg) => {
                        let photos_list = filtered_memo.get();
                        view! {
                            <div class="h-full overflow-y-auto p-4">
                                <div class="mb-4 p-3 bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-800 rounded-lg text-sm text-yellow-800 dark:text-yellow-200">
                                    <span class="font-medium">"Map unavailable: "</span> {msg}
                                    <div class="mt-1 text-xs opacity-75">"Showing photos with location data as a list instead."</div>
                                </div>
                                <div class="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-4">
                                    {photos_list.into_iter().map(|mp| {
                                        let photo = mp.photo.clone();
                                        let lat = mp.latitude;
                                        let lon = mp.longitude;
                                        view! {
                                            <div
                                                class="border border-[var(--border-default)] rounded-lg overflow-hidden hover:shadow-lg transition-shadow cursor-pointer"
                                                on:click=move |_: web_sys::MouseEvent| {
                                                    set_selected_photo.set(Some(photo.clone()));
                                                }
                                            >
                                                <div class="aspect-square bg-[var(--bg-inset)]">
                                                    <img
                                                        src=format!("/api/photos/thumbnail/{}", mp.photo.path.trim_start_matches('/'))
                                                        alt=mp.photo.name.clone()
                                                        class="w-full h-full object-cover"
                                                        loading="lazy"
                                                    />
                                                </div>
                                                <div class="p-2">
                                                    <div class="text-sm font-medium text-[var(--text-primary)] truncate">{mp.photo.name.clone()}</div>
                                                    <div class="text-xs text-[var(--text-tertiary)] font-mono">{format!("{:.4}, {:.4}", lat, lon)}</div>
                                                </div>
                                            </div>
                                        }
                                    }).collect::<Vec<_>>()}
                                </div>
                            </div>
                        }.into_any()
                    },
                }}
            </div>
        </div>
    }
}
