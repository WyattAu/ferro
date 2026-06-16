use leptos::ev;
use leptos::prelude::*;

#[component]
pub fn MarkdownEditor(
    #[prop(optional)] initial_content: String,
    #[prop(optional)] on_change: Option<Box<dyn FnMut(String)>>,
    #[prop(optional)] readonly: bool,
) -> impl IntoView {
    let (content, set_content) = signal(initial_content);
    let (fullscreen, set_fullscreen) = signal(false);
    let (show_preview, set_show_preview) = signal(true);

    let handle_input = {
        let mut on_change = on_change;
        move |ev: ev::Event| {
            let val = event_target_value(&ev);
            set_content.set(val.clone());
            if let Some(ref mut cb) = on_change {
                cb(val);
            }
        }
    };

    let insert_markdown = {
        move |prefix: &str, suffix: &str| {
            let content_val = content.get();
            // Simple insertion at cursor position - for basic markdown toolbar
            let new_content = format!("{}{}{}", prefix, content_val, suffix);
            set_content.set(new_content.clone());
        }
    };

    let toggle_fullscreen = move |_: ev::MouseEvent| {
        set_fullscreen.update(|f| *f = !*f);
    };

    let toggle_preview = move |_: ev::MouseEvent| {
        set_show_preview.update(|p| *p = !*p);
    };

    let container_class = move || {
        if fullscreen.get() {
            "fixed inset-0 z-50 flex flex-col bg-white dark:bg-gray-900"
        } else {
            "flex flex-col border border-gray-300 dark:border-gray-600 rounded-lg overflow-hidden"
        }
    };

    view! {
        <div class=container_class>
            // Toolbar
            <div class="flex items-center gap-1 px-2 py-1 bg-gray-50 dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700">
                <button
                    on:click=move |_| insert_markdown("**", "**")
                    class="p-1.5 text-gray-600 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-700 rounded transition-colors text-sm font-bold"
                    title="Bold"
                    disabled=readonly
                >
                    "B"
                </button>
                <button
                    on:click=move |_| insert_markdown("*", "*")
                    class="p-1.5 text-gray-600 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-700 rounded transition-colors text-sm italic"
                    title="Italic"
                    disabled=readonly
                >
                    "I"
                </button>
                <button
                    on:click=move |_| insert_markdown("# ", "")
                    class="p-1.5 text-gray-600 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-700 rounded transition-colors text-sm font-bold"
                    title="Heading"
                    disabled=readonly
                >
                    "H"
                </button>
                <div class="w-px h-4 bg-gray-300 dark:bg-gray-600 mx-1"></div>
                <button
                    on:click=move |_| insert_markdown("[", "](url)")
                    class="p-1.5 text-gray-600 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-700 rounded transition-colors text-sm"
                    title="Link"
                    disabled=readonly
                >
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13.828 10.172a4 4 0 00-5.656 0l-4 4a4 4 0 105.656 5.656l1.102-1.101m-.758-4.899a4 4 0 005.656 0l4-4a4 4 0 00-5.656-5.656l-1.1 1.1" /></svg>
                </button>
                <button
                    on:click=move |_| insert_markdown("![alt](", ")")
                    class="p-1.5 text-gray-600 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-700 rounded transition-colors text-sm"
                    title="Image"
                    disabled=readonly
                >
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z" /></svg>
                </button>
                <button
                    on:click=move |_| insert_markdown("`", "`")
                    class="p-1.5 text-gray-600 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-700 rounded transition-colors text-sm font-mono"
                    title="Code"
                    disabled=readonly
                >
                    "<>"
                </button>
                <button
                    on:click=move |_| insert_markdown("\n- ", "")
                    class="p-1.5 text-gray-600 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-700 rounded transition-colors text-sm"
                    title="List"
                    disabled=readonly
                >
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 12h16M4 18h16" /></svg>
                </button>
                <div class="flex-1"></div>
                <button
                    on:click=toggle_preview
                    class=move || format!("p-1.5 rounded transition-colors text-sm {}",
                        if show_preview.get() {
                            "bg-blue-100 dark:bg-blue-900 text-blue-600 dark:text-blue-400"
                        } else {
                            "text-gray-600 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-700"
                        }
                    )
                    title="Toggle Preview"
                >
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" /><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z" /></svg>
                </button>
                <button
                    on:click=toggle_fullscreen
                    class="p-1.5 text-gray-600 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-700 rounded transition-colors text-sm"
                    title="Toggle Fullscreen"
                >
                    {move || if fullscreen.get() {
                        view! { <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 9V4.5M9 9H4.5M9 9L3.75 3.75M9 15v4.5M9 15H4.5M9 15l-5.25 5.25M15 9h4.5M15 9V4.5M15 9l5.25-5.25M15 15h4.5M15 15v4.5m0-4.5l5.25 5.25" /></svg> }.into_any()
                    } else {
                        view! { <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3.75 3.75v4.5m0-4.5h4.5m-4.5 0L9 9M3.75 20.25v-4.5m0 4.5h4.5m-4.5 0L9 15M20.25 3.75h-4.5m4.5 0v4.5m0-4.5L15 9m5.25 11.25h-4.5m4.5 0v-4.5m0 4.5L15 15" /></svg> }.into_any()
                    }}
                </button>
            </div>

            // Editor area
            <div class="flex flex-1 overflow-hidden">
                // Textarea
                <div class=move || if show_preview.get() { "w-1/2" } else { "w-full" }>
                    <textarea
                        prop:value=move || content.get()
                        on:input=handle_input
                        class="w-full h-full p-3 font-mono text-sm bg-white dark:bg-gray-900 text-gray-900 dark:text-white resize-none focus:outline-none"
                        placeholder="Write your markdown here..."
                        readonly=readonly
                    ></textarea>
                </div>

                // Preview panel
                {move || show_preview.get().then(|| view! {
                    <div class="w-1/2 border-l border-gray-200 dark:border-gray-700 overflow-y-auto p-3 bg-gray-50 dark:bg-gray-800">
                        <div class="prose prose-sm dark:prose-invert max-w-none">
                            {move || render_markdown(&content.get())}
                        </div>
                    </div>
                })}
            </div>
        </div>
    }
}

fn render_markdown(text: &str) -> String {
    let mut html = String::new();
    let mut in_code_block = false;
    let mut in_list = false;

    for line in text.lines() {
        if line.starts_with("```") {
            if in_code_block {
                html.push_str("</code></pre>");
                in_code_block = false;
            } else {
                html.push_str("<pre class=\"bg-gray-100 dark:bg-gray-800 rounded p-2 font-mono text-xs overflow-x-auto\"><code>");
                in_code_block = true;
            }
            continue;
        }

        if in_code_block {
            html.push_str(&html_escape(line));
            html.push('\n');
            continue;
        }

        if line.starts_with("# ") {
            html.push_str(&format!("<h1 class=\"text-2xl font-bold mt-4 mb-2\">{}</h1>", html_escape(&line[2..])));
        } else if line.starts_with("## ") {
            html.push_str(&format!("<h2 class=\"text-xl font-bold mt-3 mb-2\">{}</h2>", html_escape(&line[3..])));
        } else if line.starts_with("### ") {
            html.push_str(&format!("<h3 class=\"text-lg font-bold mt-2 mb-1\">{}</h3>", html_escape(&line[4..])));
        } else if line.starts_with("- ") {
            if !in_list {
                html.push_str("<ul class=\"list-disc list-inside mb-2\">");
                in_list = true;
            }
            html.push_str(&format!("<li>{}</li>", render_inline(&line[2..])));
        } else if line.starts_with("1. ") {
            if !in_list {
                html.push_str("<ol class=\"list-decimal list-inside mb-2\">");
                in_list = true;
            }
            html.push_str(&format!("<li>{}</li>", render_inline(&line[3..])));
        } else if line.starts_with("> ") {
            html.push_str(&format!(
                "<blockquote class=\"border-l-4 border-gray-300 dark:border-gray-600 pl-3 italic text-gray-600 dark:text-gray-400 mb-2\">{}</blockquote>",
                render_inline(&line[2..])
            ));
        } else if line.starts_with("---") {
            html.push_str("<hr class=\"my-4 border-gray-300 dark:border-gray-600\" />");
        } else if line.is_empty() {
            if in_list {
                html.push_str("</ul>");
                in_list = false;
            }
            html.push_str("<br/>");
        } else {
            if in_list {
                html.push_str("</ul>");
                in_list = false;
            }
            html.push_str(&format!("<p class=\"mb-2\">{}</p>", render_inline(line)));
        }
    }

    if in_list {
        html.push_str("</ul>");
    }
    if in_code_block {
        html.push_str("</code></pre>");
    }

    html
}

fn render_inline(text: &str) -> String {
    let mut result = text.to_string();

    // Bold
    while let Some(start) = result.find("**") {
        if let Some(end) = result[start + 2..].find("**") {
            let bold = &result[start + 2..start + 2 + end].to_string();
            result = format!(
                "{}<strong>{}</strong>{}",
                &result[..start],
                html_escape(bold),
                &result[start + 4 + end..]
            );
        } else {
            break;
        }
    }

    // Italic
    while let Some(start) = result.find('*') {
        if let Some(end) = result[start + 1..].find('*') {
            let italic = &result[start + 1..start + 1 + end].to_string();
            result = format!(
                "{}<em>{}</em>{}",
                &result[..start],
                html_escape(italic),
                &result[start + 2 + end..]
            );
        } else {
            break;
        }
    }

    // Inline code
    while let Some(start) = result.find('`') {
        if let Some(end) = result[start + 1..].find('`') {
            let code = &result[start + 1..start + 1 + end].to_string();
            result = format!(
                "{}<code class=\"bg-gray-100 dark:bg-gray-800 px-1 rounded font-mono text-xs\">{}</code>{}",
                &result[..start],
                html_escape(code),
                &result[start + 2 + end..]
            );
        } else {
            break;
        }
    }

    // Links [text](url)
    while let Some(start) = result.find('[') {
        if let Some(mid) = result[start..].find("](") {
            if let Some(end) = result[start + mid + 2..].find(')') {
                let link_text = &result[start + 1..start + mid].to_string();
                let url = &result[start + mid + 2..start + mid + 2 + end].to_string();
                result = format!(
                    "{}<a href=\"{}\" class=\"text-blue-600 dark:text-blue-400 hover:underline\" target=\"_blank\">{}</a>{}",
                    &result[..start],
                    html_escape(url),
                    html_escape(link_text),
                    &result[start + mid + 3 + end..]
                );
            } else {
                break;
            }
        } else {
            break;
        }
    }

    result
}

fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_html_escape() {
        assert_eq!(html_escape("a < b & c > d"), "a &lt; b &amp; c &gt; d");
        assert_eq!(html_escape("\"hello\""), "&quot;hello&quot;");
    }

    #[test]
    fn test_render_inline_bold() {
        assert_eq!(render_inline("hello **world**"), "hello <strong>world</strong>");
    }

    #[test]
    fn test_render_inline_italic() {
        assert_eq!(render_inline("hello *world*"), "hello <em>world</em>");
    }

    #[test]
    fn test_render_inline_code() {
        assert!(render_inline("hello `world`").contains("<code"));
    }

    #[test]
    fn test_render_markdown_heading() {
        let html = render_markdown("# Hello");
        assert!(html.contains("<h1"));
        assert!(html.contains("Hello"));
    }

    #[test]
    fn test_render_markdown_list() {
        let html = render_markdown("- item1\n- item2");
        assert!(html.contains("<ul"));
        assert!(html.contains("<li>"));
    }
}
