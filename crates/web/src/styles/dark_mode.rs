/// Dark mode CSS variables and theme management.
///
/// Provides CSS custom properties for dark/light themes using `data-theme` attribute,
/// localStorage persistence, system theme detection via `prefers-color-scheme`,
/// and smooth transitions between themes.

/// CSS for dark mode variables and transitions.
/// Injected as a `<style>` tag on app mount.
pub const DARK_MODE_CSS: &str = r#"
:root,
[data-theme="light"] {
  --color-bg: #ffffff;
  --color-surface: #f9fafb;
  --color-surface-raised: #ffffff;
  --color-text: #111827;
  --color-text-secondary: #6b7280;
  --color-text-muted: #9ca3af;
  --color-border: #e5e7eb;
  --color-border-strong: #d1d5db;
  --color-accent: #3b82f6;
  --color-accent-hover: #2563eb;
  --color-danger: #ef4444;
  --color-success: #22c55e;
  --color-warning: #f59e0b;
  --color-overlay: rgba(0, 0, 0, 0.5);
  --color-drop-zone: rgba(59, 130, 246, 0.08);
  --color-drop-zone-active: rgba(59, 130, 246, 0.18);
  --shadow-sm: 0 1px 2px 0 rgb(0 0 0 / 0.05);
  --shadow-md: 0 4px 6px -1px rgb(0 0 0 / 0.1), 0 2px 4px -2px rgb(0 0 0 / 0.1);
  --shadow-lg: 0 10px 15px -3px rgb(0 0 0 / 0.1), 0 4px 6px -4px rgb(0 0 0 / 0.1);
  color-scheme: light;
}

[data-theme="dark"] {
  --color-bg: #030712;
  --color-surface: #111827;
  --color-surface-raised: #1f2937;
  --color-text: #f9fafb;
  --color-text-secondary: #9ca3af;
  --color-text-muted: #6b7280;
  --color-border: #374151;
  --color-border-strong: #4b5563;
  --color-accent: #60a5fa;
  --color-accent-hover: #3b82f6;
  --color-danger: #f87171;
  --color-success: #4ade80;
  --color-warning: #fbbf24;
  --color-overlay: rgba(0, 0, 0, 0.7);
  --color-drop-zone: rgba(96, 165, 250, 0.08);
  --color-drop-zone-active: rgba(96, 165, 250, 0.2);
  --shadow-sm: 0 1px 2px 0 rgb(0 0 0 / 0.3);
  --shadow-md: 0 4px 6px -1px rgb(0 0 0 / 0.4), 0 2px 4px -2px rgb(0 0 0 / 0.3);
  --shadow-lg: 0 10px 15px -3px rgb(0 0 0 / 0.4), 0 4px 6px -4px rgb(0 0 0 / 0.3);
  color-scheme: dark;
}

/* Smooth theme transitions */
html.theme-transition,
html.theme-transition *,
html.theme-transition *::before,
html.theme-transition *::after {
  transition: background-color 0.3s ease, color 0.2s ease, border-color 0.3s ease, box-shadow 0.3s ease 0.05s !important;
}
"#;

/// Inject dark mode CSS into the document head.
#[cfg(target_arch = "wasm32")]
pub fn inject_dark_mode_css() {
    use wasm_bindgen::JsCast;

    if let Some(window) = web_sys::window() {
        if let Some(doc) = window.document() {
            if doc.query_selector("#ferro-dark-mode-css").ok().flatten().is_some() {
                return;
            }

            if let Some(style) = doc
                .create_element("style")
                .ok()
                .and_then(|e| e.dyn_into::<web_sys::HtmlStyleElement>().ok())
            {
                style.set_id("ferro-dark-mode-css");
                let _ = style.set_text_content(Some(DARK_MODE_CSS));
                if let Some(head) = doc.head() {
                    let _ = head.append_child(&style);
                }
            }
        }
    }
}

/// Set the `data-theme` attribute on the `<html>` element with a brief
/// transition class so colors animate smoothly.
#[cfg(target_arch = "wasm32")]
pub fn apply_theme(theme: &str) {
    use wasm_bindgen::JsCast;

    if let Some(window) = web_sys::window() {
        if let Some(doc) = window.document() {
            if let Some(html) = doc.document_element() {
                // Add transition class for smooth animation
                let _ = html.class_list().add_1("theme-transition");

                // Set data-theme attribute
                let _ = html.set_attribute("data-theme", theme);

                // Also toggle dark class for Tailwind compatibility
                if theme == "dark" {
                    let _ = html.class_list().add_1("dark");
                } else {
                    let _ = html.class_list().remove_1("dark");
                }

                // Remove transition class after animation completes
                if let Some(window) = web_sys::window() {
                    let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
                        wasm_bindgen::closure::Closure::once_into_js(|| {
                            if let Some(window) = web_sys::window() {
                                if let Some(doc) = window.document() {
                                    if let Some(html) = doc.document_element() {
                                        let _ = html.class_list().remove_1("theme-transition");
                                    }
                                }
                            }
                        })
                        .unchecked_ref(),
                        350,
                    );
                }
            }
        }
    }
}

/// Persist theme preference to localStorage.
#[cfg(target_arch = "wasm32")]
pub fn persist_theme(theme: &str) {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            let _ = storage.set_item("ferro_theme", theme);
        }
    }
}

/// Read persisted theme from localStorage.
#[cfg(target_arch = "wasm32")]
pub fn read_persisted_theme() -> Option<String> {
    web_sys::window()
        .and_then(|w| w.local_storage().ok())
        .flatten()
        .and_then(|s| s.get_item("ferro_theme").ok())
        .flatten()
}

/// Detect system preference via `prefers-color-scheme` media query.
#[cfg(target_arch = "wasm32")]
pub fn detect_system_theme() -> &'static str {
    web_sys::window()
        .and_then(|w| w.match_media("(prefers-color-scheme: dark)").ok())
        .flatten()
        .map(|mql| if mql.matches() { "dark" } else { "light" })
        .unwrap_or("light")
}

/// Resolve the initial theme: localStorage > system preference > light default.
#[cfg(target_arch = "wasm32")]
pub fn resolve_initial_theme() -> &'static str {
    if let Some(stored) = read_persisted_theme() {
        if stored == "dark" || stored == "light" {
            return match stored.as_str() {
                "dark" => "dark",
                _ => "light",
            };
        }
    }
    detect_system_theme()
}
