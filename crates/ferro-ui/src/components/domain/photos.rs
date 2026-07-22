use leptos::prelude::*;

/// Photos page with grid view and lightbox.
#[component]
pub fn PhotosPage() -> impl IntoView {
    let (photos, set_photos) = signal(Vec::<Photo>::new());
    let (selected, set_selected) = signal(None::<usize>);
    let (loading, set_loading) = signal(true);
    let (view_mode, set_view_mode) = signal("grid".to_string());

    #[derive(Clone, Debug)]
    struct Photo {
        path: String,
        name: String,
        thumbnail: String,
        date: String,
        width: u32,
        height: u32,
    }

    Effect::new(move |_| {
        set_loading.set(true);
        #[cfg(target_arch = "wasm32")]
        {
            let set_p = set_photos;
            let set_l = set_loading;
            wasm_bindgen_futures::spawn_local(async move {
                let client = crate::api::ApiClient::new(crate::api::ApiClientConfig::default());
                match client.get::<serde_json::Value>("/api/v1/photos").await {
                    Ok(val) => {
                        if let Some(arr) = val.as_array() {
                            let items: Vec<Photo> = arr.iter().filter_map(|v| {
                                let path = v["path"].as_str()?.to_string();
                                Some(Photo {
                                    thumbnail: format!("/api/v1/photos/thumbnail/{}", path),
                                    name: v["name"].as_str().unwrap_or("photo").to_string(),
                                    date: v["date"].as_str().unwrap_or("").to_string(),
                                    width: v["width"].as_u64().unwrap_or(0) as u32,
                                    height: v["height"].as_u64().unwrap_or(0) as u32,
                                    path,
                                })
                            }).collect();
                            set_p.set(items);
                        }
                        set_l.set(false);
                    }
                    Err(e) => { log::error!("Photos load failed: {}", e); set_l.set(false); }
                }
            });
        }
    });

    view! {
        <div class="flex flex-col h-full">
            <div class="flex items-center gap-3 px-4 py-2 border-b border-[var(--color-border)]">
                <h1 class="text-lg font-semibold">"Photos"</h1>
                <div class="ml-auto flex gap-2">
                    <button class=move || format!("btn btn-ghost btn-sm {}", if view_mode.get() == "grid" { "btn-primary" } else { "" })
                        on:click=move |_| set_view_mode.set("grid".to_string())>"Grid"</button>
                    <button class=move || format!("btn btn-ghost btn-sm {}", if view_mode.get() == "timeline" { "btn-primary" } else { "" })
                        on:click=move |_| set_view_mode.set("timeline".to_string())>"Timeline"</button>
                </div>
            </div>
            <div class="flex-1 overflow-y-auto p-4">
                <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-6 gap-2">
                    {move || photos.get().into_iter().enumerate().map(|(i, photo)| {
                        let idx = i;
                        view! {
                            <div class="aspect-square rounded-lg overflow-hidden cursor-pointer hover:opacity-80 transition-opacity bg-sunken"
                                on:click=move |_| set_selected.set(Some(idx))>
                                <img src=photo.thumbnail alt=photo.name class="w-full h-full object-cover" loading="lazy" />
                            </div>
                        }
                    }).collect_view()}
                </div>
            </div>

            // Lightbox
            {move || {
                if let Some(idx) = selected.get() {
                    let photo = photos.get().get(idx).cloned();
                    if let Some(p) = photo {
                        view! {
                            <div class="fixed inset-0 bg-black/80 z-50 flex items-center justify-center" on:click=move |_| set_selected.set(None)>
                                <img src=format!("/api/v1/files/{}", p.path) alt=p.name class="max-h-[90vh] max-w-[90vw] object-contain rounded-lg" />
                                <button class="absolute top-4 right-4 text-white text-2xl" on:click=move |_| set_selected.set(None)>"×"</button>
                            </div>
                        }.into_any()
                    } else {
                        view! { <></> }.into_any()
                    }
                } else {
                    view! { <></> }.into_any()
                }
            }}
        </div>
    }
}
