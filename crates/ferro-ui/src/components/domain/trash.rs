use leptos::prelude::*;

#[derive(Clone, Debug)]
struct TrashItem {
    name: String,
    path: String,
    original_path: String,
    deleted_at: String,
    size: u64,
}

/// Trash page with restore/purge functionality.
#[allow(unused_variables)]
#[component]
pub fn TrashPage() -> impl IntoView {
    let (items, set_items) = signal(Vec::<TrashItem>::new());
    let (loading, set_loading) = signal(true);
    let (error, set_error) = signal(None::<String>);

    Effect::new(move |_| {
        set_loading.set(true);
        #[cfg(target_arch = "wasm32")]
        {
            let set_i = set_items;
            let set_l = set_loading;
            wasm_bindgen_futures::spawn_local(async move {
                let client = crate::api::ApiClient::new(crate::api::ApiClientConfig::default());
                match client.get::<serde_json::Value>("/api/v1/trash").await {
                    Ok(val) => {
                        if let Some(arr) = val.as_array() {
                            let items: Vec<TrashItem> = arr
                                .iter()
                                .filter_map(|v| {
                                    Some(TrashItem {
                                        name: v["name"].as_str()?.to_string(),
                                        path: v["path"].as_str().unwrap_or("").to_string(),
                                        original_path: v["original_path"].as_str().unwrap_or("").to_string(),
                                        deleted_at: v["deleted_at"].as_str().unwrap_or("").to_string(),
                                        size: v["size"].as_u64().unwrap_or(0),
                                    })
                                })
                                .collect();
                            set_i.set(items);
                        }
                        set_l.set(false);
                    }
                    Err(e) => {
                        log::error!("Trash load failed: {}", e);
                        set_error.set(Some(e.to_string()));
                        set_l.set(false);
                    }
                }
            });
        }
    });

    view! {
        <div class="flex flex-col h-full">
            <div class="flex items-center gap-3 px-4 py-2 border-b border-[var(--color-border)]">
                <h1 class="text-lg font-semibold">"Trash"</h1>
                <span class="text-secondary text-sm">{move || format!("{} items", items.get().len())}</span>
                <div class="ml-auto">
                    <button class="btn btn-danger btn-sm">"Empty Trash"</button>
                </div>
            </div>
            <div class="flex-1 overflow-y-auto">
                {move || {
                    if loading.get() {
                        return view! { <div class="p-8 text-center text-secondary"><div class="text-2xl mb-2">"..."</div><p>"Loading trash..."</p></div> }.into_any();
                    }
                    if let Some(err) = error.get() {
                        return view! { <div class="p-8 text-center text-danger"><div class="text-2xl mb-2">"!"</div><p>{format!("Error: {}", err)}</p></div> }.into_any();
                    }
                    if items.get().is_empty() {
                        return view! { <div class="p-8 text-center text-secondary"><div class="text-2xl mb-2">"--"</div><p>"Trash is empty"</p></div> }.into_any();
                    }
                    view! {
                        <table class="table w-full">
                            <thead>
                                <tr>
                                    <th>"Name"</th>
                                    <th>"Original Path"</th>
                                    <th>"Deleted"</th>
                                    <th>"Size"</th>
                                    <th>"Actions"</th>
                                </tr>
                            </thead>
                            <tbody>
                                {move || items.get().into_iter().map(|item| {
                                    let path = item.path.clone();
                                    let original = item.original_path.clone();
                                    let path_r = path.clone();
                                    let path_d = path.clone();
                                    view! {
                                        <tr>
                                            <td class="font-medium">{item.name}</td>
                                            <td class="text-secondary text-sm">{original}</td>
                                            <td class="text-secondary text-sm">{item.deleted_at}</td>
                                            <td class="text-secondary text-sm">{format_size(item.size)}</td>
                                            <td>
                                                <button class="btn btn-ghost btn-sm text-success"
                                                    on:click=move |_| {
                                                        log::info!("[trash] restore: {}", path_r);
                                                    }
                                                >"Restore"</button>
                                                <button class="btn btn-ghost btn-sm text-danger"
                                                    on:click=move |_| {
                                                        log::info!("[trash] delete: {}", path_d);
                                                    }
                                                >"Delete"</button>
                                            </td>
                                        </tr>
                                    }
                                }).collect_view()}
                            </tbody>
                        </table>
                    }.into_any()
                }}
            </div>
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
