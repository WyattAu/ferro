use leptos::ev;
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::api::FileEntry;
use crate::components::focus_trap::FocusTrap;

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
    "css",
    "sh",
    "log",
    "cfg",
    "ini",
    "env",
    "gitignore",
    "editorconfig",
    "go",
    "c",
    "cpp",
    "h",
    "hpp",
    "java",
    "rb",
    "php",
    "swift",
    "kt",
    "scala",
    "lua",
    "r",
    "sql",
    "proto",
    "graphql",
    "dockerfile",
    "makefile",
    "cmake",
    "nix",
    "lock",
];

const HTML_EXTENSIONS: &[&str] = &["html", "htm", "svg"];

fn get_extension(name: &str) -> &str {
    name.rsplit('.').next().unwrap_or("")
}

fn file_category(name: &str) -> &'static str {
    let ext = get_extension(name);
    let lower = name.to_lowercase();
    if ["jpg", "jpeg", "png", "gif", "svg", "webp", "bmp", "ico"].contains(&ext) {
        "image"
    } else if ["mp4", "webm", "ogg", "mov", "avi", "mkv"].contains(&ext) {
        "video"
    } else if ["mp3", "wav", "ogg", "flac", "aac"].contains(&ext) {
        "audio"
    } else if ext == "pdf" {
        "pdf"
    } else if ["epub"].contains(&ext) {
        "epub"
    } else if HTML_EXTENSIONS.contains(&ext) {
        "html"
    } else if ext == "md" || ext == "markdown" {
        "markdown"
    } else if ext == "csv" {
        "csv"
    } else if TEXT_EXTENSIONS.contains(&ext) {
        "text"
    } else if lower == "dockerfile" || lower == "makefile" || lower == "cmakelists.txt" {
        "text"
    } else {
        "other"
    }
}

/// Escape HTML entities to prevent XSS in code preview.
fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

/// Render a CSV string as an HTML table.
fn csv_to_html_table(csv: &str) -> String {
    let mut out =
        String::from(r#"<div class="overflow-auto"><table class="w-full text-sm font-mono border-collapse">#"#);
    for (i, line) in csv.lines().enumerate() {
        let tag = if i == 0 { "th" } else { "td" };
        let row_class = if i == 0 {
            "bg-[var(--accent)] text-[var(--text-on-accent)] font-bold"
        } else if i % 2 == 0 {
            "bg-[var(--bg-subtle)]"
        } else {
            ""
        };
        out.push_str(&format!("<tr class=\"{}\">", row_class));
        for cell in line.split(',') {
            out.push_str(&format!(
                "<{} class=\"px-3 py-1.5 border border-[var(--border-default)] whitespace-nowrap\">{}</{}>",
                tag,
                escape_html(cell.trim()),
                tag
            ));
        }
        out.push_str("</tr>");
    }
    out.push_str("</table></div>");
    out
}

/// Render markdown to simple HTML (bold, italic, headings, code blocks, lists).
fn render_markdown(md: &str) -> String {
    let mut out = String::new();
    let mut in_code_block = false;
    for line in md.lines() {
        if line.starts_with("```") {
            if in_code_block {
                out.push_str("</code></pre>");
                in_code_block = false;
            } else {
                out.push_str(r#"<pre class="bg-[var(--bg-base)] border rounded p-3 my-2 text-sm font-mono"><code>"#);
                in_code_block = true;
            }
            continue;
        }
        if in_code_block {
            out.push_str(&escape_html(line));
            out.push('\n');
            continue;
        }
        let mut line = escape_html(line);
        // Headings
        if line.starts_with("### ") {
            line = format!(
                r#"<h3 class="text-lg font-bold mt-4 mb-2 font-mono">{}</h3>"#,
                &line[4..]
            );
        } else if line.starts_with("## ") {
            line = format!(
                r#"<h2 class="text-xl font-bold mt-4 mb-2 font-mono">{}</h2>"#,
                &line[3..]
            );
        } else if line.starts_with("# ") {
            line = format!(
                r#"<h1 class="text-2xl font-bold mt-4 mb-2 font-mono">{}</h1>"#,
                &line[2..]
            );
        } else if line.starts_with("- ") || line.starts_with("* ") {
            line = format!(r#"<li class="ml-4">{}</li>"#, &line[2..]);
        } else if line.trim().is_empty() {
            line = "<br/>".to_string();
        } else {
            // Inline code
            let mut result = String::new();
            let mut rest = line.as_str();
            while let Some(start) = rest.find('`') {
                if let Some(end) = rest[start + 1..].find('`') {
                    result.push_str(&rest[..start]);
                    result.push_str(&format!(
                        r#"<code class="bg-[var(--bg-base)] px-1 rounded text-sm">{}</code>"#,
                        &rest[start + 1..start + 1 + end]
                    ));
                    rest = &rest[start + 1 + end + 1..];
                } else {
                    result.push_str(rest);
                    rest = "";
                }
            }
            result.push_str(rest);
            line = result;
            // Bold and italic
            while let Some(start) = line.find("**") {
                if let Some(end) = line[start + 2..].find("**") {
                    let bold = &line[start + 2..start + 2 + end];
                    let replacement = format!(r#"<strong>{}</strong>"#, bold);
                    line = format!("{}{}{}", &line[..start], replacement, &line[start + 2 + end + 2..]);
                } else {
                    break;
                }
            }
            while let Some(start) = line.find('*') {
                if let Some(end) = line[start + 1..].find('*') {
                    if !line[start..start + 2].contains('*') {
                        break;
                    }
                    let italic = &line[start + 1..start + 1 + end];
                    let replacement = format!(r#"<em>{}</em>"#, italic);
                    line = format!("{}{}{}", &line[..start], replacement, &line[start + 1 + end + 1..]);
                } else {
                    break;
                }
            }
        }
        out.push_str(&line);
        out.push('\n');
    }
    out
}

#[component]
pub fn FilePreview(file: FileEntry, on_close: Callback<()>) -> impl IntoView {
    let (content, set_content) = signal(None::<String>);
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal(None::<String>);
    #[allow(unused)] // Used inside view! macro closures
    let (edit_mode, set_edit_mode) = signal(false);

    let category = file_category(&file.name);
    let name = file.name.clone();
    let path = file.path.clone();
    let size = file.size;
    let modified = file.modified_at.clone();
    let is_text = category == "text" || category == "markdown" || category == "csv";

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

    let cat_for_view = category.to_string();
    let file_path_for_view = file.path.clone();
    let file_name_for_view = file.name.clone();

    view! {
        <div
            class="fixed inset-0 bg-black/80 z-50 flex items-center justify-center p-4"
            on:keydown=handle_keydown
        >
            <FocusTrap>
            <div
                class="bg-[var(--bg-base)] border border-[var(--border-default)] rounded-lg shadow-2xl w-full max-w-5xl max-h-[90vh] flex flex-col"
                role="dialog"
                aria-modal="true"
                aria-labelledby="preview-title"
                tabindex="-1"
            >
                <div class="flex items-center justify-between px-6 py-4 border-b border-[var(--border-default)] bg-[var(--bg-subtle)]">
                    <div class="min-w-0 flex-1">
                        <h2 id="preview-title" class="text-lg font-mono font-bold text-[var(--text-primary)] truncate">{name.clone()}</h2>
                        <div class="flex items-center gap-4 mt-1 text-sm text-[var(--text-tertiary)] font-mono">
                            <span>{size_str}</span>
                            <span>{modified}</span>
                        </div>
                    </div>
                    <button
                        class="p-2 text-[var(--text-tertiary)] hover:text-[var(--text-secondary)] hover:bg-[var(--interactive-hover)] rounded transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] ml-4 min-w-[44px] min-h-[44px] flex items-center justify-center"
                        aria-label=t!("preview.aria_close")
                        on:click=close
                    >
                        <svg class="w-5 h-5" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                        </svg>
                    </button>
                </div>

                <div class="flex-1 overflow-auto p-6">
                    {move || loading.get().then(|| view! {
                        <div class="flex items-center justify-center py-12">
                            <div class="animate-spin w-8 h-8 border-2 border-[var(--accent)] border-t-transparent rounded-full"></div>
                            <span class="ml-3 text-[var(--text-tertiary)]">{t!("common.loading")}</span>
                        </div>
                    })}

                    {move || error.get().map(|e| view! {
                        <div class="bg-[var(--danger-subtle)] border-l-4 border-l-[var(--danger)] rounded p-4 text-[var(--danger)]">
                            "Failed to load file: " {e}
                        </div>
                    })}

                    // Text content (plain text, markdown, csv)
                    {move || content.get().map(|text| {
                        let cat = file_category(&file.name);
                        match cat {
                            "markdown" => {
                                let html = render_markdown(&text);
                                view! {
                                    <div class="prose prose-invert max-w-none text-[var(--text-primary)] leading-relaxed" inner_html=html></div>
                                }.into_any()
                            }
                            "csv" => {
                                let html = csv_to_html_table(&text);
                                view! {
                                    <div class="text-[var(--text-primary)]" inner_html=html></div>
                                }.into_any()
                            }
                            _ => view! {
                                <pre class="bg-[var(--bg-base)] border border-[var(--border-default)] rounded p-4 text-sm text-[var(--text-primary)] overflow-auto whitespace-pre-wrap font-mono leading-relaxed">{text}</pre>
                            }.into_any(),
                        }
                    })}

                    // Media / other file previews (only shown when content is None = non-text files)
                    {move || {
                        if !loading.get() && content.get().is_none() && error.get().is_none() {
                            let cat = cat_for_view.clone();
                            let p = file_path_for_view.clone();
                            let n = file_name_for_view.clone();
                            let img_path = file_path_for_view.clone();
                            Some((cat, p, n, img_path))
                        } else {
                            None
                        }
                    }}
                    .map(|(cat, p, n, img_path)| {
                        match cat.as_str() {
                            "image" => view! {
                                <div>
                                    {move || edit_mode.get().then(|| {
                                        let close_cb = Callback::new(move |_| set_edit_mode.set(false));
                                        let src_path = img_path.clone();
                                        let fp_path = img_path.clone();
                                        view! {
                                            <PhotoEditor src=src_path file_path=fp_path on_close=close_cb />
                                        }
                                    })}
                                    <div class="flex items-center justify-center relative group">
                                        <img
                                            src={p}
                                            alt={n}
                                            class="max-w-full max-h-[70vh] object-contain rounded-lg"
                                        />
                                        {move || (!edit_mode.get()).then(|| view! {
                                            <button
                                                class="absolute top-2 right-2 px-3 py-1.5 text-xs bg-[var(--accent)] text-[var(--text-on-accent)] border border-[var(--border-default)] rounded font-bold uppercase opacity-0 group-hover:opacity-100 transition-opacity hover:bg-[var(--accent-hover)]"
                                                on:click=move |_| set_edit_mode.set(true)
                                            >
                                                "Edit"
                                            </button>
                                        })}
                                    </div>
                                </div>
                            }.into_any(),
                            "html" => view! {
                                <div class="w-full border border-[var(--border-default)] rounded-lg overflow-hidden">
                                    <div class="bg-[var(--bg-subtle)] px-4 py-2 text-xs font-mono text-[var(--text-tertiary)] border-b border-[var(--border-default)]">
                                        "HTML Preview"
                                    </div>
                                    <iframe
                                        src={p}
                                        class="w-full h-[70vh] bg-white"
                                        title={n}
                                        sandbox="allow-same-origin"
                                    ></iframe>
                                </div>
                            }.into_any(),
                            "video" => view! {
                                <div class="flex items-center justify-center">
                                    <video
                                        controls
                                        class="max-w-full max-h-[70vh] rounded-lg"
                                        preload="metadata"
                                        aria-label={format!("Video: {}", n)}
                                    >
                                        <source src={p} />
                                        {t!("preview.no_video")}
                                    </video>
                                </div>
                            }.into_any(),
                            "audio" => view! {
                                <div class="flex flex-col items-center justify-center py-12 gap-6">
                                    <svg class="w-24 h-24 text-[var(--accent)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 19V6l12-3v13M9 19c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zm12-3c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zM9 10l12-3" />
                                    </svg>
                                    <audio controls class="w-full max-w-md" aria-label={format!("Audio: {}", n)}>
                                        <source src={p} type="audio/mpeg" />
                                        {t!("preview.no_audio")}
                                    </audio>
                                    <p class="text-sm text-[var(--text-tertiary)] font-mono">{n}</p>
                                </div>
                            }.into_any(),
                            "epub" => view! {
                                <div class="h-[70vh]">
                                    <EpubPreview src=p title=n />
                                </div>
                            }.into_any(),
                            "pdf" => view! {
                                <div class="w-full border border-[var(--border-default)] rounded-lg overflow-hidden">
                                    <iframe
                                        src={p}
                                        class="w-full h-[70vh]"
                                        title={n}
                                    ></iframe>
                                </div>
                            }.into_any(),
                            _ => view! {
                                <div class="flex flex-col items-center justify-center py-12 text-center">
                                    <svg class="w-16 h-16 text-[var(--text-tertiary)] mb-4" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                                    </svg>
                                    <p class="text-[var(--text-tertiary)] mb-4">{t!("preview.not_available")}</p>
                                    <button
                                        class="px-4 py-2 text-sm bg-[var(--accent)] text-[var(--text-on-accent)] border border-[var(--border-default)] rounded font-bold uppercase hover:bg-[var(--accent-hover)] transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] min-h-[44px]"
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
                    })
                </div>
            </div>
            </FocusTrap>
        </div>
    }
}

fn format_file_size(bytes: u64) -> String {
    ferro_common::format::format_size(bytes)
}
