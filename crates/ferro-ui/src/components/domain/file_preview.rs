use crate::api::endpoints::FileEntry;
use leptos::prelude::*;

fn is_code_file(name: &str) -> bool {
    matches!(
        name.rsplit('.').next().unwrap_or(""),
        "rs" | "py" | "js" | "ts" | "go" | "c" | "cpp" | "h"
    )
}

#[component]
pub fn FilePreview(
    entry: FileEntry,
    server_url: String,
    on_close: Callback<()>,
    #[prop(optional)] entries: Option<Vec<FileEntry>>,
    #[prop(optional)] on_navigate: Option<Callback<FileEntry>>,
) -> impl IntoView {
    let mime = entry.mime_type.clone().unwrap_or_default();
    let url = format!("{}/api/v1/files{}", server_url, entry.path);
    let is_code = is_code_file(&entry.name);

    let prev_entry = entries.as_ref().and_then(|list| {
        let idx = list.iter().position(|e| e.path == entry.path)?;
        if idx > 0 {
            Some(list[idx - 1].clone())
        } else {
            None
        }
    });
    let next_entry = entries.as_ref().and_then(|list| {
        let idx = list.iter().position(|e| e.path == entry.path)?;
        if idx + 1 < list.len() {
            Some(list[idx + 1].clone())
        } else {
            None
        }
    });

    let content = if mime.starts_with("image/") {
        view! {
            <img src=&url alt=&entry.name class="max-h-[80vh] max-w-[90vw] object-contain" />
        }
        .into_any()
    } else if mime.starts_with("video/") {
        view! {
            <video controls src=&url class="max-h-[80vh] max-w-[90vw]" />
        }
        .into_any()
    } else if mime.starts_with("audio/") {
        view! {
            <audio controls src=&url class="w-full" />
        }
        .into_any()
    } else if mime == "application/pdf" {
        view! {
            <iframe src=&url class="w-full h-[80vh] border-0" />
        }
        .into_any()
    } else if mime.starts_with("text/")
        || mime.contains("json")
        || mime.contains("javascript")
    {
        let (text, _set_text) = signal("Loading...".to_string());
        #[cfg(target_arch = "wasm32")]
        {
            let path = entry.path.clone();
            let server_url_clone = server_url.clone();
            let set_t = set_text;
            wasm_bindgen_futures::spawn_local(async move {
                let args = serde_json::json!({
                    "url": server_url_clone,
                    "token": "",
                    "path": path,
                });
                match crate::components::domain::file_browser::tauri_invoke(
                    "get_file_content",
                    &args,
                )
                .await
                {
                    Ok(content) => set_t.set(content),
                    Err(e) => set_t.set(format!("Error loading file: {}", e)),
                }
            });
        }
        if is_code {
            view! {
                <div class="p-4 bg-sunken rounded-lg overflow-auto max-h-[70vh]">
                    <pre class="text-sm font-mono whitespace-pre-wrap">
                        {move || {
                            let t = text.get();
                            let lines: Vec<String> = t.lines().map(String::from).collect();
                            view! {
                                <table class="border-collapse">
                                    <tbody>
                                        {lines.into_iter().enumerate().map(|(i, line)| {
                                            let line_num = i + 1;
                                            view! {
                                                <tr>
                                                    <td class="text-right pr-4 text-white/30 select-none align-top w-12">{line_num}</td>
                                                    <td>{line}</td>
                                                </tr>
                                            }
                                        }).collect_view()}
                                    </tbody>
                                </table>
                            }
                        }}
                    </pre>
                </div>
            }
            .into_any()
        } else {
            view! {
                <pre class="p-4 bg-sunken rounded-lg overflow-auto max-h-[70vh] text-sm font-mono whitespace-pre-wrap">
                    {move || text.get()}
                </pre>
            }
            .into_any()
        }
    } else {
        view! {
            <div class="p-8 text-center">
                <p class="text-secondary mb-4">"Preview not available for this file type"</p>
                <a href=&url download=&entry.name class="btn btn-primary">
                    "Download"
                </a>
            </div>
        }
        .into_any()
    };

    let on_navigate_cb = on_navigate.unwrap_or_else(|| Callback::new(|_| {}));
    let prev_for_nav = prev_entry.clone();
    let next_for_nav = next_entry.clone();
    let on_navigate_for_keys = on_navigate_cb;

    view! {
        <div
            class="fixed inset-0 z-50 flex flex-col"
            role="dialog"
            aria-modal="true"
            aria-label=format!("Preview: {}", entry.name)
            on:keydown=move |ev: web_sys::KeyboardEvent| {
                if ev.key() == "Escape" {
                    on_close.run(());
                } else if ev.key() == "ArrowLeft" {
                    if let Some(prev) = prev_for_nav.clone() {
                        on_navigate_for_keys.run(prev);
                    }
                } else if ev.key() == "ArrowRight"
                    && let Some(next) = next_for_nav.clone() {
                        on_navigate_for_keys.run(next);
                    }
            }
        >
            // Backdrop — clicking closes the preview
            <div
                class="absolute inset-0 bg-black/80"
                on:click=move |_| on_close.run(())
            ></div>
            // Header bar with file metadata and controls
            <div class="relative flex items-center justify-between px-4 py-3 bg-black/50 text-white shrink-0 z-10">
                <div class="flex items-center gap-3 min-w-0">
                    <span class="font-medium truncate">{entry.name.clone()}</span>
                    <span class="text-sm text-white/60 whitespace-nowrap">
                        {crate::components::domain::file_browser::format_size(entry.size)}
                    </span>
                    <span class="text-sm text-white/40 whitespace-nowrap">{mime.clone()}</span>
                </div>
                <div class="flex items-center gap-2 shrink-0 ml-4">
                    <a
                        href={format!("{}/api/v1/files{}", server_url, entry.path)}
                        download={entry.name.clone()}
                        class="btn btn-ghost btn-sm text-white"
                    >
                        "Download"
                    </a>
                    <button
                        class="text-white text-2xl hover:text-white/80"
                        on:click=move |_| on_close.run(())
                    >
                        "\u{00D7}"
                    </button>
                </div>
            </div>
            // Content area — stop propagation so clicks inside don't close the overlay
            <div
                class="relative flex-1 flex items-center justify-center p-4 overflow-auto z-10"
                on:click=|ev: web_sys::MouseEvent| ev.stop_propagation()
            >
                {content}
            </div>
        </div>
    }
}
