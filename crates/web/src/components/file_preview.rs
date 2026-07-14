use leptos::ev;
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::api::FileEntry;
use crate::components::focus_trap::FocusTrap;
use crate::components::video_player::VideoPlayer;

use crate::t;

const TEXT_EXTENSIONS: &[&str] = &[
    "txt",
    "md",
    "json",
    "xml",
    "toml",
    "yaml",
    "yml",
    "csv",
    "rs",
    "py",
    "js",
    "ts",
    "html",
    "css",
    "sh",
    "log",
    "cfg",
    "ini",
    "env",
    "gitignore",
    "editorconfig",
];

const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "gif", "svg", "webp", "bmp", "ico"];

const VIDEO_EXTENSIONS: &[&str] = &["mp4", "webm", "ogg", "mov", "avi"];

const AUDIO_EXTENSIONS: &[&str] = &["mp3", "wav", "ogg", "flac", "aac"];

fn get_extension(name: &str) -> &str {
    name.rsplit('.').next().unwrap_or("")
}

fn file_category(name: &str) -> &'static str {
    let ext = get_extension(name);
    if IMAGE_EXTENSIONS.contains(&ext) {
        "image"
    } else if TEXT_EXTENSIONS.contains(&ext) {
        "text"
    } else if ext == "pdf" {
        "pdf"
    } else if VIDEO_EXTENSIONS.contains(&ext) {
        "video"
    } else if AUDIO_EXTENSIONS.contains(&ext) {
        "audio"
    } else {
        "other"
    }
}

#[component]
pub fn FilePreview(file: FileEntry, on_close: Callback<()>) -> impl IntoView {
    let (content, set_content) = signal(None::<String>);
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal(None::<String>);

    let category = file_category(&file.name);
    let name = file.name.clone();
    let path = file.path.clone();
    let size = file.size;
    let modified = file.modified_at.clone();
    let is_text = category == "text";

    if is_text {
        set_loading.set(true);
        let p = path.clone();
        spawn_local(async move {
            match crate::api::get_file_content(&p).await {
                Ok(text) => {
                    let truncated = if text.len() > 102_400 {
                        format!(
                            "{}...\n\n[File truncated: showing first 100KB of {} bytes]",
                            &text[..102_400],
                            text.len()
                        )
                    } else {
                        text
                    };
                    set_content.set(Some(truncated));
                    set_loading.set(false);
                }
                Err(e) => {
                    set_error.set(Some(e));
                    set_loading.set(false);
                }
            }
        });
    }

    let size_str = format_file_size(size);

    let handle_keydown = move |ev: ev::KeyboardEvent| {
        if ev.key() == "Escape" {
            on_close.run(());
        }
    };

    let close = move |_: ev::MouseEvent| {
        on_close.run(());
    };

    view! {
        <div
            class="fixed inset-0 bg-black bg-opacity-50 z-50 flex items-center justify-center p-4 backdrop-blur-sm"
            on:keydown=handle_keydown
        >
            <FocusTrap>
            <div
                class="brutal-block rounded shadow-2xl w-full max-w-4xl max-h-[90vh] flex flex-col"
                role="dialog"
                aria-modal="true"
                aria-labelledby="preview-title"
                tabindex="-1"
            >
                // Header
                <div class="flex items-center justify-between px-6 py-4 border-b border-[var(--border-default)]">
                    <div class="min-w-0 flex-1">
                        <h2 id="preview-title" class="text-section font-mono text-[var(--text-primary)] truncate">{name}</h2>
                        <div class="flex items-center gap-4 mt-1 text-sm text-[var(--text-tertiary)] font-mono">
                            <span>{size_str}</span>
                            <span>{modified}</span>
                        </div>
                    </div>
                    <button
                        class="p-2 text-[var(--text-tertiary)] hover:text-gray-600 hover:bg-[var(--interactive-hover)] rounded surface shadow-concrete transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] ml-4 min-w-[44px] min-h-[44px] flex items-center justify-center"
                        aria-label=t!("preview.aria_close")
                        on:click=close
                    >
                        <svg class="w-5 h-5" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                        </svg>
                    </button>
                </div>

                // Content
                <div class="flex-1 overflow-auto p-6">
                    {move || loading.get().then(|| view! {
                        <div class="flex items-center justify-center py-12">
                            <div class="animate-spin w-8 h-8 border-2 border-[var(--accent)] border-t-transparent rounded-full"></div>
                            <span class="ml-3 text-[var(--text-tertiary)]">{t!("common.loading")}</span>
                        </div>
                    })}

                    {move || error.get().map(|e| view! {
                        <div class="bg-red-50 border-l-4 border-l-red-500 rounded p-4 text-red-700">
                            "Failed to load file: " {e}
                        </div>
                    })}

                    {move || content.get().map(|text| view! {
                        <pre class="bg-[var(--bg-base)] border rounded p-4 text-sm text-gray-800 overflow-auto whitespace-pre-wrap font-mono">{text}</pre>
                    })}

                    {move || (!loading.get() && content.get().is_none() && error.get().is_none()).then(|| view! {
                        {
                            let cat = file_category(&file.name);
                            let p = file.path.clone();
                            let n = file.name.clone();
                            match cat {
                                "image" => view! {
                                    <div class="flex items-center justify-center">
                                        <img
                                            src={p}
                                            alt={n}
                                            class="max-w-full max-h-[60vh] object-contain rounded-lg"
                                        />
                                    </div>
                                }.into_any(),
                                "video" => view! {
                                    <div class="flex items-center justify-center">
                                        <VideoPlayer src=p title=n />
                                    </div>
                                }.into_any(),
                                "audio" => view! {
                                    <div class="flex items-center justify-center py-8">
                                        <audio controls aria-label={format!("Audio: {}", n)}>
                                            <source src={p} type="audio/mpeg" />
                                            {t!("preview.no_audio")}
                                        </audio>
                                    </div>
                                }.into_any(),
                                "pdf" => view! {
                                    <iframe
                                        src={p}
                                        class="w-full h-[60vh] rounded-lg border"
                                        title={n}
                                    ></iframe>
                                }.into_any(),
                                _ => view! {
                                    <div class="flex flex-col items-center justify-center py-12 text-center">
                                        <svg class="w-16 h-16 text-gray-300 mb-4" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                                        </svg>
                                        <p class="text-[var(--text-tertiary)] mb-4">{t!("preview.not_available")}</p>
                                        <button
                                            class="px-4 py-2 text-sm bg-[var(--accent)] text-[var(--text-on-accent)] brutal-border rounded-sm font-bold uppercase hover:bg-blue-700 transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] focus:ring-offset-2 dark:focus:ring-offset-gray-800 min-h-[44px]"
                                            on:click=move |_| {
                                                let path = p.clone();
                                                spawn_local(async move {
                                                    drop(crate::api::download_file(&path).await);
                                                });
                                            }
                                        >
                                            {t!("common.download")}
                                        </button>
                                    </div>
                                }.into_any(),
                            }
                        }
                    })}
                </div>
            </div>
            </FocusTrap>
        </div>
    }
}

fn format_file_size(bytes: u64) -> String {
    ferro_common::format::format_size(bytes)
}
