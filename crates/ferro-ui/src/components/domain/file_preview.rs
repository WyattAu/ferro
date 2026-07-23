use crate::api::endpoints::FileEntry;
use leptos::prelude::*;

#[component]
pub fn FilePreview(
    entry: FileEntry,
    server_url: String,
    on_close: Callback<()>,
) -> impl IntoView {
    let mime = entry.mime_type.clone().unwrap_or_default();
    let url = format!("{}/api/v1/files{}", server_url, entry.path);

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
            let set_t = _set_text;
            wasm_bindgen_futures::spawn_local(async move {
                let args = serde_json::json!({
                    "url": server_url,
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
        view! {
            <pre class="p-4 bg-sunken rounded-lg overflow-auto max-h-[70vh] text-sm font-mono whitespace-pre-wrap">
                {move || text.get()}
            </pre>
        }
        .into_any()
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

    view! {
        <div
            class="fixed inset-0 z-50 flex flex-col"
            role="dialog"
            aria-modal="true"
            aria-label=format!("Preview: {}", entry.name)
            on:keydown=move |ev: web_sys::KeyboardEvent| {
                if ev.key() == "Escape" {
                    on_close.run(());
                }
            }
        >
            // Backdrop — clicking closes the preview
            <div
                class="absolute inset-0 bg-black/80"
                on:click=move |_| on_close.run(())
            ></div>
            // Header bar with file metadata and close button
            <div class="relative flex items-center justify-between px-4 py-3 bg-black/50 text-white shrink-0 z-10">
                <div class="flex items-center gap-3 min-w-0">
                    <span class="font-medium truncate">{entry.name.clone()}</span>
                    <span class="text-sm text-white/60 whitespace-nowrap">
                        {crate::components::domain::file_browser::format_size(entry.size)}
                    </span>
                    <span class="text-sm text-white/40 whitespace-nowrap">{mime.clone()}</span>
                </div>
                <button
                    class="text-white text-2xl hover:text-white/80 shrink-0 ml-4"
                    on:click=move |_| on_close.run(())
                >
                    "\u{00D7}"
                </button>
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
