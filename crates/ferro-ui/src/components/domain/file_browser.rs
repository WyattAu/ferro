use crate::api::endpoints::FileEntry;
use leptos::prelude::*;

/// File browser with list/grid views, breadcrumb, selection.
#[component]
pub fn FileBrowser(#[prop(into)] server_url: String) -> impl IntoView {
    let (entries, set_entries) = signal(Vec::<FileEntry>::new());
    let (loading, set_loading) = signal(true);
    let (current_path, set_current_path) = signal("/".to_string());
    let (view_mode, set_view_mode) = signal("list".to_string());
    let (selected, set_selected) = signal(std::collections::HashSet::<String>::new());
    let (error, set_error) = signal(None::<String>);

    Effect::new(move |_| {
        let path = current_path.get();
        let url = server_url.clone();
        set_loading.set(true);
        set_error.set(None);

        #[cfg(target_arch = "wasm32")]
        {
            let set_e = set_entries;
            let set_l = set_loading;
            let set_err = set_error;
            let p = path.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let args = serde_json::json!({
                    "url": url,
                    "token": "",
                    "path": p,
                });
                match tauri_invoke("list_files_rest", &args).await {
                    Ok(json_str) => {
                        // Response is { entries: [...] } — parse the entries array
                        match serde_json::from_str::<serde_json::Value>(&json_str) {
                            Ok(val) => {
                                let items: Vec<FileEntry> = val
                                    .get("entries")
                                    .and_then(|e| e.as_array())
                                    .map(|arr| {
                                        arr.iter()
                                            .filter_map(|v| {
                                                Some(FileEntry {
                                                    name: v.get("name")?.as_str()?.to_string(),
                                                    path: v.get("path")?.as_str()?.to_string(),
                                                    size: v.get("size").and_then(|n| n.as_u64()).unwrap_or(0),
                                                    is_collection: v
                                                        .get("isCollection")
                                                        .and_then(|b| b.as_bool())
                                                        .or_else(|| v.get("is_collection").and_then(|b| b.as_bool()))
                                                        .unwrap_or(false),
                                                    modified_at: v
                                                        .get("modifiedAt")
                                                        .and_then(|s| s.as_str())
                                                        .or_else(|| v.get("modified_at").and_then(|s| s.as_str()))
                                                        .unwrap_or("")
                                                        .to_string(),
                                                    mime_type: v
                                                        .get("mimeType")
                                                        .and_then(|s| s.as_str())
                                                        .or_else(|| v.get("mime_type").and_then(|s| s.as_str()))
                                                        .map(String::from),
                                                    etag: v.get("etag").and_then(|s| s.as_str()).map(String::from),
                                                })
                                            })
                                            .collect()
                                    })
                                    .unwrap_or_default();
                                log::info!("[file_browser] loaded {} entries from path={}", items.len(), p);
                                set_e.set(items);
                                set_l.set(false);
                            }
                            Err(e) => {
                                log::error!(
                                    "[file_browser] parse error: {} — raw: {}",
                                    e,
                                    &json_str[..json_str.len().min(200)]
                                );
                                set_err.set(Some(format!("Parse: {}", e)));
                                set_l.set(false);
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("[file_browser] invoke error: {}", e);
                        set_err.set(Some(e));
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

    view! {
        <div class="flex flex-col h-full">
            <div class="flex items-center gap-3 px-4 py-2 border-b border-[var(--color-border)]">
                <button class="btn btn-ghost btn-sm" on:click=move |_| navigate("/".to_string())>"🏠 Home"</button>
                <Breadcrumb path=current_path navigate=Callback::new(navigate.clone()) />
                <div class="ml-auto flex items-center gap-2">
                    <button class=move || format!("btn btn-ghost btn-sm {}", if view_mode.get() == "list" { "btn-primary" } else { "" }) on:click=move |_| set_view_mode.set("list".to_string())>"☰ List"</button>
                    <button class=move || format!("btn btn-ghost btn-sm {}", if view_mode.get() == "grid" { "btn-primary" } else { "" }) on:click=move |_| set_view_mode.set("grid".to_string())>"⊞ Grid"</button>
                    <span class="text-sm text-secondary">{move || format!("{} items", entries.get().len())}</span>
                </div>
            </div>
            {move || {
                let count = selected.get().len();
                if count > 0 {
                    view! { <div class="flex items-center gap-3 px-4 py-2 bg-accent-subtle border-b border-[var(--color-border)]">
                        <span class="text-sm font-medium">{format!("{} selected", count)}</span>
                        <button class="btn btn-ghost btn-sm" on:click=move |_| set_selected.set(std::collections::HashSet::new())>"Clear"</button>
                    </div> }.into_any()
                } else { view! { <></> }.into_any() }
            }}
            <div class="flex-1 overflow-y-auto">
                {move || {
                    if loading.get() {
                        view! { <div class="p-8 text-center"><p class="text-secondary">"Loading..."</p></div> }.into_any()
                    } else if let Some(err) = error.get() {
                        view! { <div class="p-8 text-center text-danger"><p>{format!("Error: {}", err)}</p></div> }.into_any()
                    } else if entries.get().is_empty() {
                        view! { <div class="p-12 text-center"><div class="text-6xl mb-4">"📁"</div><h3 class="text-lg font-semibold mb-2">"This folder is empty"</h3></div> }.into_any()
                    } else if view_mode.get() == "grid" {
                        view! { <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-6 gap-3 p-4">
                            {entries.get().into_iter().map(|entry| { let p = entry.path.clone(); let p2 = p.clone(); let is_dir = entry.is_collection; let icon = if is_dir { "📁".to_string() } else { file_icon(&entry.name).to_string() }; let sz = if is_dir { "--".to_string() } else { format_size(entry.size) }; let nm = entry.name.clone();
                                view! { <div class="card cursor-pointer transition-all hover:shadow-md" on:click=move |_| { if is_dir { navigate(p.clone()) } else { toggle_select(p2.clone()) } }>
                                    <div class="text-4xl text-center mb-2">{icon}</div>
                                    <p class="text-sm font-medium truncate text-center">{nm}</p>
                                    <p class="text-xs text-secondary text-center">{sz}</p>
                                </div> }
                            }).collect_view()}
                        </div> }.into_any()
                    } else {
                        view! { <table class="table w-full"><thead><tr><th class="w-10"></th><th>"Name"</th><th class="w-24">"Size"</th><th class="w-40">"Modified"</th></tr></thead><tbody>
                            {entries.get().into_iter().map(|entry| { let p = entry.path.clone(); let p2 = p.clone(); let is_dir = entry.is_collection; let icon = if is_dir { "📁" } else { file_icon(&entry.name) }; let sz = if is_dir { "--".to_string() } else { format_size(entry.size) }; let nm = entry.name.clone(); let mod_at = entry.modified_at.clone();
                                view! { <tr class="cursor-pointer hover:bg-sunken" on:click=move |_| { if is_dir { navigate(p.clone()) } else { toggle_select(p2.clone()) } }>
                                    <td class="text-center">{icon}</td><td class="font-medium truncate">{nm}</td><td class="text-secondary text-sm">{sz}</td><td class="text-secondary text-sm whitespace-nowrap">{mod_at}</td>
                                </tr> }
                            }).collect_view()}
                        </tbody></table> }.into_any()
                    }
                }}
            </div>
        </div>
    }
}

#[component]
fn Breadcrumb(path: ReadSignal<String>, navigate: Callback<String>) -> impl IntoView {
    view! { <nav class="flex items-center gap-1 text-sm" aria-label="Breadcrumb">
        {move || { let p = path.get(); let parts: Vec<&str> = p.trim_start_matches('/').split('/').filter(|s| !s.is_empty()).collect(); let mut acc = String::new(); let mut items = Vec::new();
            items.push(view! { <button class="nav-link" on:click=move |_| navigate.run("/".to_string())>"Root"</button> }.into_any());
            for part in &parts { acc.push('/'); acc.push_str(part); let fp = acc.clone(); let nm = part.to_string();
                items.push(view! { <span class="text-tertiary">"/"</span> }.into_any());
                items.push(view! { <button class="nav-link" on:click=move |_| navigate.run(fp.clone())>{nm}</button> }.into_any());
            }
            items.into_view()
        }}
    </nav> }
}

async fn tauri_invoke(cmd: &str, args: &serde_json::Value) -> Result<String, String> {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        let window = web_sys::window().ok_or("no window")?;
        let tauri = js_sys::Reflect::get(&window, &wasm_bindgen::JsValue::from_str("__TAURI__"))
            .map_err(|_| "no __TAURI__".to_string())?;
        let core = js_sys::Reflect::get(&tauri, &wasm_bindgen::JsValue::from_str("core"))
            .map_err(|_| "no __TAURI__.core".to_string())?;
        let invoke = js_sys::Reflect::get(&core, &wasm_bindgen::JsValue::from_str("invoke"))
            .map_err(|_| "no __TAURI__.core.invoke".to_string())?;
        let invoke_fn: js_sys::Function = invoke.dyn_into().map_err(|_| "invoke not a function".to_string())?;
        let args_js = js_sys::JSON::parse(&serde_json::to_string(args).unwrap_or_default())
            .map_err(|e| format!("JSON parse: {:?}", e))?;
        let result = invoke_fn
            .call2(&core, &wasm_bindgen::JsValue::from_str(cmd), &args_js)
            .map_err(|e| format!("invoke error: {:?}", e))?;
        let promise: js_sys::Promise = result.dyn_into().map_err(|_| "result not a promise".to_string())?;
        let value = wasm_bindgen_futures::JsFuture::from(promise)
            .await
            .map_err(|e| format!("promise error: {:?}", e))?;
        value.as_string().ok_or("result not a string".to_string())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (cmd, args);
        Err("not wasm32".into())
    }
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

fn file_icon(name: &str) -> &'static str {
    let ext = name.rsplit('.').next().unwrap_or("");
    match ext {
        "pdf" => "📄",
        "doc" | "docx" => "📝",
        "xls" | "xlsx" => "📊",
        "jpg" | "jpeg" | "png" | "gif" | "webp" => "🖼️",
        "mp4" | "avi" | "mov" => "🎬",
        "mp3" | "wav" | "flac" => "🎵",
        "zip" | "tar" | "gz" => "📦",
        "rs" | "py" | "js" | "ts" | "go" => "💻",
        "md" | "txt" => "📄",
        "html" | "css" | "json" | "toml" => "🌐",
        _ => "📄",
    }
}
