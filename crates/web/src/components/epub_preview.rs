use leptos::ev;
use leptos::prelude::*;
use wasm_bindgen::prelude::*;

#[derive(Debug, Clone)]
pub struct EpubMetadata {
    pub title: Option<String>,
    pub creator: Option<String>,
    pub description: Option<String>,
    pub cover_url: Option<String>,
}

#[component]
pub fn EpubPreview(
    src: String,
    title: String,
    #[prop(default = Callback::new(move |_: String| {}))] _on_error: Callback<String>,
) -> impl IntoView {
    let (metadata, set_metadata) = signal(None::<EpubMetadata>);
    let (loading, set_loading) = signal(true);
    let (error, set_error) = signal(None::<String>);
    let (current_chapter, set_current_chapter) = signal(0usize);
    let (total_chapters, _set_total_chapters) = signal(0usize);
    let (render_failed, set_render_failed) = signal(false);

    let epub_ref: NodeRef<leptos::html::Div> = NodeRef::new();
    let title_stored = StoredValue::new(title.clone());

    let handle_load = move |_: ev::Event| {
        set_loading.set(true);
        set_error.set(None);

        let window = web_sys::window().expect("no global window");

        // Check if ePub.js is available
        let has_epub = js_sys::Reflect::has(
            &window.into(),
            &JsValue::from_str("ePub"),
        )
        .unwrap_or(false);

        if !has_epub {
            set_error.set(Some("EPUB.js library not loaded".to_string()));
            set_loading.set(false);
            set_render_failed.set(true);
            return;
        }

        if let Some(element) = epub_ref.get() {
            let element_id = element.id();

            // Create the book and render it
            let js_code = format!(
                r#"
                (function() {{
                    try {{
                        var book = window.ePub('{}', {{}});
                        var rendition = book.renderTo('{}', {{width: '100%', height: '100%'}});
                        rendition.flow('paginated');
                        
                        book.ready.then(function() {{
                            return book.loaded.metadata;
                        }}).then(function(metadata) {{
                            return metadata;
                        }}).then(function(metadata) {{
                            // Store metadata for later use
                            window._epubMetadata = metadata;
                        }});
                        
                        rendition.display();
                        
                        // Set up navigation handlers
                        rendition.on('relocated', function(location) {{
                            window._epubCurrentLocation = location;
                        }});
                        
                        window._epubBook = book;
                        window._epubRendition = rendition;
                        
                        return true;
                    }} catch(e) {{
                        return e.toString();
                    }}
                }})()
                "#,
                src, element_id
            );

            match js_sys::eval(&js_code) {
                Ok(result) => {
                    if let Some(err_msg) = result.as_string() {
                        set_error.set(Some(err_msg));
                        set_loading.set(false);
                        set_render_failed.set(true);
                    } else {
                        // Try to get metadata
                        let metadata_js = js_sys::eval(
                            r#"
                            (function() {
                                if (window._epubMetadata) {
                                    return JSON.stringify(window._epubMetadata);
                                }
                                return null;
                            })()
                            "#,
                        );

                        if let Ok(meta_str) = metadata_js
                            && let Some(meta_json) = meta_str.as_string()
                            && let Ok(meta_obj) = serde_json::from_str::<serde_json::Value>(&meta_json)
                        {
                            let meta = EpubMetadata {
                                title: meta_obj.get("title").and_then(|v| v.as_str()).map(|s| s.to_string()),
                                creator: meta_obj.get("creator").and_then(|v| v.as_str()).map(|s| s.to_string()),
                                description: meta_obj.get("description").and_then(|v| v.as_str()).map(|s| s.to_string()),
                                cover_url: meta_obj.get("cover").and_then(|v| v.as_str()).map(|s| s.to_string()),
                            };
                            set_metadata.set(Some(meta));
                        }

                        set_loading.set(false);
                    }
                }
                Err(e) => {
                    set_error.set(Some(format!("Failed to initialize EPUB: {:?}", e)));
                    set_loading.set(false);
                    set_render_failed.set(true);
                }
            }
        }
    };

    let prev_chapter = move |_: ev::MouseEvent| {
        let _ = js_sys::eval(
            r#"
            if (window._epubRendition) {
                window._epubRendition.prev();
            }
            "#,
        );
        set_current_chapter.update(|c| {
            if *c > 0 {
                *c -= 1;
            }
        });
    };

    let next_chapter = move |_: ev::MouseEvent| {
        let _ = js_sys::eval(
            r#"
            if (window._epubRendition) {
                window._epubRendition.next();
            }
            "#,
        );
        set_current_chapter.update(|c| *c += 1);
    };

    let handle_keydown = move |ev: ev::KeyboardEvent| {
        match ev.key().as_str() {
            "ArrowLeft" => {
                let _ = js_sys::eval(
                    r#"
                    if (window._epubRendition) {
                        window._epubRendition.prev();
                    }
                    "#,
                );
                set_current_chapter.update(|c| {
                    if *c > 0 {
                        *c -= 1;
                    }
                });
            }
            "ArrowRight" => {
                let _ = js_sys::eval(
                    r#"
                    if (window._epubRendition) {
                        window._epubRendition.next();
                    }
                    "#,
                );
                set_current_chapter.update(|c| *c += 1);
            }
            _ => {}
        }
    };

    view! {
        <div class="flex flex-col h-full" on:keydown=handle_keydown>
            // Header with controls
            <div class="flex items-center justify-between p-2 border-b border-[var(--border-default)] bg-[var(--bg-surface)]">
                <div class="flex items-center gap-2">
                    <button
                        class="px-3 py-1.5 text-sm font-mono bg-[var(--bg-inset)] hover:bg-[var(--interactive-hover)] rounded transition-colors disabled:opacity-50"
                        on:click=prev_chapter
                        disabled=move || current_chapter.get() == 0
                    >
                        "← Prev"
                    </button>
                    <button
                        class="px-3 py-1.5 text-sm font-mono bg-[var(--bg-inset)] hover:bg-[var(--interactive-hover)] rounded transition-colors"
                        on:click=next_chapter
                    >
                        "Next →"
                    </button>
                    <span class="text-xs text-[var(--text-tertiary)] font-mono">
                        {move || format!("Chapter {} / {}", current_chapter.get() + 1, total_chapters.get().max(1))}
                    </span>
                </div>
            </div>

            // Content area
            <div class="flex-1 relative overflow-hidden">
                {move || loading.get().then(|| view! {
                    <div class="absolute inset-0 flex items-center justify-center bg-[var(--bg-surface)]">
                        <div class="flex flex-col items-center gap-3">
                            <div class="animate-spin w-8 h-8 border-2 border-[var(--accent)] border-t-transparent rounded-full"></div>
                            <span class="text-sm text-[var(--text-tertiary)]">"Loading EPUB..."</span>
                        </div>
                    </div>
                })}

                {move || error.get().map(|e| view! {
                    <div class="absolute inset-0 flex items-center justify-center bg-[var(--bg-surface)]">
                        <div class="text-center p-6">
                            <div class="text-[var(--danger)] mb-2">"Error: " {e}</div>
                        </div>
                    </div>
                })}

                {move || render_failed.get().then(|| view! {
                    // Fallback: Show metadata
                    <div class="absolute inset-0 flex items-center justify-center bg-[var(--bg-surface)]">
                        <div class="text-center p-8 max-w-md">
                            <svg class="w-16 h-16 mx-auto mb-4 text-[var(--text-tertiary)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M12 6.253v13m0-13C10.832 5.477 9.246 5 7.5 5S4.168 5.477 3 6.253v13C4.168 18.477 5.754 18 7.5 18s3.332.477 4.5 1.253m0-13C13.168 5.477 14.754 5 16.5 5c1.747 0 3.332.477 4.5 1.253v13C19.832 18.477 18.247 18 16.5 18c-1.746 0-3.332.477-4.5 1.253" />
                            </svg>
                            <h3 class="text-lg font-mono font-bold text-[var(--text-primary)] mb-2">
                                {move || metadata.get().and_then(|m| m.title).unwrap_or_else(|| title_stored.get_value())}
                            </h3>
                            {move || metadata.get().map(|m| view! {
                                <div class="space-y-2 text-sm text-[var(--text-secondary)]">
                                    {m.creator.map(|c| view! {
                                        <div>"Author: " {c}</div>
                                    })}
                                    {m.description.map(|d| view! {
                                        <div class="text-[var(--text-tertiary)] italic">{d}</div>
                                    })}
                                </div>
                            })}
                            <div class="mt-6 p-4 bg-[var(--bg-inset)] rounded-lg">
                                <p class="text-sm text-[var(--text-tertiary)]">
                                    "EPUB preview requires EPUB.js library. Please include it in your HTML."
                                </p>
                                <code class="block mt-2 text-xs text-[var(--text-secondary)] font-mono">
                                    "&lt;script src=\"https://cdnjs.cloudflare.com/ajax/libs/epub.js/0.3.93/epub.min.js\"&gt;&lt;/script&gt;"
                                </code>
                            </div>
                        </div>
                    </div>
                })}

                <div
                    node_ref=epub_ref
                    id="epub-container"
                    class="w-full h-full"
                    on:load=handle_load
                ></div>
            </div>
        </div>
    }
}
