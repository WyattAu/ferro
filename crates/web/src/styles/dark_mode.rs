//! Theme management system for Ferro.
//!
//! Supports four themes: light, dark, midnight, and system (auto-detect).
//! Uses CSS custom properties via `data-theme` attribute with localStorage
//! persistence and smooth transitions.

/// Available themes in the application.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Theme {
    #[default]
    Light,
    Dark,
    Midnight,
    System,
}

impl Theme {
    pub fn as_str(&self) -> &'static str {
        match self {
            Theme::Light => "light",
            Theme::Dark => "dark",
            Theme::Midnight => "midnight",
            Theme::System => "system",
        }
    }

    pub fn from_str_value(s: &str) -> Self {
        match s {
            "dark" => Theme::Dark,
            "midnight" => Theme::Midnight,
            "system" => Theme::System,
            _ => Theme::Light,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Theme::Light => "Light",
            Theme::Dark => "Dark",
            Theme::Midnight => "Midnight",
            Theme::System => "System",
        }
    }

    pub fn all() -> &'static [Theme] {
        &[Theme::Light, Theme::Dark, Theme::Midnight, Theme::System]
    }
}

/// Complete CSS custom properties for all themes.
/// This is the single source of truth for the visual design system.
pub const THEME_CSS: &str = r#"
/* ═══════════════════════════════════════════════════════════════════════════
   Ferro Design System - CSS Custom Properties
   ═══════════════════════════════════════════════════════════════════════════ */

/* ── Base resets ────────────────────────────────────────────────────────── */
*, *::before, *::after {
  box-sizing: border-box;
}

html {
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  text-rendering: optimizeLegibility;
}

/* ── Light Theme (Default) ─────────────────────────────────────────────── */
:root,
[data-theme="light"] {
  /* Background hierarchy */
  --bg-base: #f8fafc;
  --bg-surface: #ffffff;
  --bg-surface-raised: #ffffff;
  --bg-surface-sunken: #f1f5f9;
  --bg-surface-overlay: #ffffff;
  --bg-inset: #f1f5f9;

  /* Text hierarchy */
  --text-primary: #0f172a;
  --text-secondary: #475569;
  --text-tertiary: #94a3b8;
  --text-inverse: #ffffff;
  --text-on-accent: #ffffff;

  /* Border hierarchy */
  --border-default: #e2e8f0;
  --border-strong: #cbd5e1;
  --border-subtle: #f1f5f9;
  --border-focus: #3b82f6;

  /* Accent (brand) */
  --accent: #3b82f6;
  --accent-hover: #2563eb;
  --accent-active: #1d4ed8;
  --accent-subtle: rgba(59, 130, 246, 0.08);
  --accent-muted: rgba(59, 130, 246, 0.15);

  /* Semantic colors */
  --danger: #ef4444;
  --danger-hover: #dc2626;
  --danger-subtle: rgba(239, 68, 68, 0.08);
  --success: #22c55e;
  --success-hover: #16a34a;
  --success-subtle: rgba(34, 197, 94, 0.08);
  --warning: #f59e0b;
  --warning-hover: #d97706;
  --warning-subtle: rgba(245, 158, 11, 0.08);
  --info: #3b82f6;
  --info-subtle: rgba(59, 130, 246, 0.08);

  /* Interactive states */
  --interactive-hover: #f8fafc;
  --interactive-active: #f1f5f9;
  --interactive-selected: rgba(59, 130, 246, 0.08);

  /* Shadows */
  --shadow-xs: 0 1px 2px rgba(0, 0, 0, 0.04);
  --shadow-sm: 0 1px 3px rgba(0, 0, 0, 0.06), 0 1px 2px rgba(0, 0, 0, 0.04);
  --shadow-md: 0 4px 6px -1px rgba(0, 0, 0, 0.07), 0 2px 4px -2px rgba(0, 0, 0, 0.05);
  --shadow-lg: 0 10px 15px -3px rgba(0, 0, 0, 0.08), 0 4px 6px -4px rgba(0, 0, 0, 0.04);
  --shadow-xl: 0 20px 25px -5px rgba(0, 0, 0, 0.1), 0 8px 10px -6px rgba(0, 0, 0, 0.06);

  /* Overlay */
  --overlay: rgba(0, 0, 0, 0.4);
  --overlay-heavy: rgba(0, 0, 0, 0.6);

  /* Drag and drop */
  --dropzone-bg: rgba(59, 130, 246, 0.04);
  --dropzone-border: rgba(59, 130, 246, 0.3);
  --dropzone-active-bg: rgba(59, 130, 246, 0.1);
  --dropzone-active-border: rgba(59, 130, 246, 0.6);

  /* Scrollbar */
  --scrollbar-track: transparent;
  --scrollbar-thumb: #cbd5e1;
  --scrollbar-thumb-hover: #94a3b8;

  /* Skeleton loading */
  --skeleton-base: #e2e8f0;
  --skeleton-shine: #f1f5f9;

  color-scheme: light;
}

/* ── Dark Theme ─────────────────────────────────────────────────────────── */
[data-theme="dark"] {
  --bg-base: #0a0a0f;
  --bg-surface: #111118;
  --bg-surface-raised: #1a1a24;
  --bg-surface-sunken: #07070b;
  --bg-surface-overlay: #1a1a24;
  --bg-inset: #0e0e14;

  --text-primary: #f1f5f9;
  --text-secondary: #94a3b8;
  --text-tertiary: #64748b;
  --text-inverse: #0f172a;
  --text-on-accent: #ffffff;

  --border-default: #262630;
  --border-strong: #3a3a4a;
  --border-subtle: #1e1e28;
  --border-focus: #60a5fa;

  --accent: #60a5fa;
  --accent-hover: #3b82f6;
  --accent-active: #2563eb;
  --accent-subtle: rgba(96, 165, 250, 0.08);
  --accent-muted: rgba(96, 165, 250, 0.15);

  --danger: #f87171;
  --danger-hover: #ef4444;
  --danger-subtle: rgba(248, 113, 113, 0.1);
  --success: #4ade80;
  --success-hover: #22c55e;
  --success-subtle: rgba(74, 222, 128, 0.1);
  --warning: #fbbf24;
  --warning-hover: #f59e0b;
  --warning-subtle: rgba(251, 191, 36, 0.1);
  --info: #60a5fa;
  --info-subtle: rgba(96, 165, 250, 0.1);

  --interactive-hover: #1a1a24;
  --interactive-active: #262630;
  --interactive-selected: rgba(96, 165, 250, 0.1);

  --shadow-xs: 0 1px 2px rgba(0, 0, 0, 0.2);
  --shadow-sm: 0 1px 3px rgba(0, 0, 0, 0.3), 0 1px 2px rgba(0, 0, 0, 0.2);
  --shadow-md: 0 4px 6px -1px rgba(0, 0, 0, 0.35), 0 2px 4px -2px rgba(0, 0, 0, 0.25);
  --shadow-lg: 0 10px 15px -3px rgba(0, 0, 0, 0.4), 0 4px 6px -4px rgba(0, 0, 0, 0.25);
  --shadow-xl: 0 20px 25px -5px rgba(0, 0, 0, 0.5), 0 8px 10px -6px rgba(0, 0, 0, 0.3);

  --overlay: rgba(0, 0, 0, 0.6);
  --overlay-heavy: rgba(0, 0, 0, 0.8);

  --dropzone-bg: rgba(96, 165, 250, 0.06);
  --dropzone-border: rgba(96, 165, 250, 0.3);
  --dropzone-active-bg: rgba(96, 165, 250, 0.12);
  --dropzone-active-border: rgba(96, 165, 250, 0.6);

  --scrollbar-track: transparent;
  --scrollbar-thumb: #3a3a4a;
  --scrollbar-thumb-hover: #4a4a5a;

  --skeleton-base: #1e1e28;
  --skeleton-shine: #262630;

  color-scheme: dark;
}

/* ── Midnight Theme (Deep blue) ────────────────────────────────────────── */
[data-theme="midnight"] {
  --bg-base: #0b0e1a;
  --bg-surface: #0f1225;
  --bg-surface-raised: #151935;
  --bg-surface-sunken: #080a14;
  --bg-surface-overlay: #151935;
  --bg-inset: #0d1020;

  --text-primary: #e8ecf4;
  --text-secondary: #8892b0;
  --text-tertiary: #5a6380;
  --text-inverse: #0b0e1a;
  --text-on-accent: #ffffff;

  --border-default: #1e2340;
  --border-strong: #2a3055;
  --border-subtle: #161a30;
  --border-focus: #7aa2f7;

  --accent: #7aa2f7;
  --accent-hover: #5a8af7;
  --accent-active: #4070f7;
  --accent-subtle: rgba(122, 162, 247, 0.08);
  --accent-muted: rgba(122, 162, 247, 0.15);

  --danger: #f7768e;
  --danger-hover: #e06070;
  --danger-subtle: rgba(247, 118, 142, 0.1);
  --success: #9ece6a;
  --success-hover: #80c050;
  --success-subtle: rgba(158, 206, 106, 0.1);
  --warning: #e0af68;
  --warning-hover: #d0a050;
  --warning-subtle: rgba(224, 175, 104, 0.1);
  --info: #7aa2f7;
  --info-subtle: rgba(122, 162, 247, 0.1);

  --interactive-hover: #151935;
  --interactive-active: #1e2340;
  --interactive-selected: rgba(122, 162, 247, 0.1);

  --shadow-xs: 0 1px 2px rgba(0, 0, 0, 0.3);
  --shadow-sm: 0 1px 3px rgba(0, 0, 0, 0.4), 0 1px 2px rgba(0, 0, 0, 0.3);
  --shadow-md: 0 4px 6px -1px rgba(0, 0, 0, 0.45), 0 2px 4px -2px rgba(0, 0, 0, 0.35);
  --shadow-lg: 0 10px 15px -3px rgba(0, 0, 0, 0.5), 0 4px 6px -4px rgba(0, 0, 0, 0.3);
  --shadow-xl: 0 20px 25px -5px rgba(0, 0, 0, 0.6), 0 8px 10px -6px rgba(0, 0, 0, 0.4);

  --overlay: rgba(5, 7, 15, 0.7);
  --overlay-heavy: rgba(5, 7, 15, 0.85);

  --dropzone-bg: rgba(122, 162, 247, 0.06);
  --dropzone-border: rgba(122, 162, 247, 0.3);
  --dropzone-active-bg: rgba(122, 162, 247, 0.12);
  --dropzone-active-border: rgba(122, 162, 247, 0.6);

  --scrollbar-track: transparent;
  --scrollbar-thumb: #2a3055;
  --scrollbar-thumb-hover: #3a4065;

  --skeleton-base: #161a30;
  --skeleton-shine: #1e2340;

  color-scheme: dark;
}

/* ── Global utility classes ─────────────────────────────────────────────── */
.bg-base { background-color: var(--bg-base); }
.bg-surface { background-color: var(--bg-surface); }
.bg-surface-raised { background-color: var(--bg-surface-raised); }
.bg-surface-sunken { background-color: var(--bg-surface-sunken); }
.bg-surface-overlay { background-color: var(--bg-surface-overlay); }

.text-primary { color: var(--text-primary); }
.text-secondary { color: var(--text-secondary); }
.text-tertiary { color: var(--text-tertiary); }

.border-default { border-color: var(--border-default); }
.border-strong { border-color: var(--border-strong); }

/* ── Scrollbar styling ─────────────────────────────────────────────────── */
::-webkit-scrollbar {
  width: 8px;
  height: 8px;
}
::-webkit-scrollbar-track {
  background: var(--scrollbar-track);
}
::-webkit-scrollbar-thumb {
  background: var(--scrollbar-thumb);
  border-radius: 4px;
}
::-webkit-scrollbar-thumb:hover {
  background: var(--scrollbar-thumb-hover);
}

/* Firefox scrollbar */
* {
  scrollbar-width: thin;
  scrollbar-color: var(--scrollbar-thumb) var(--scrollbar-track);
}

/* ── Skeleton loading animation ────────────────────────────────────────── */
@keyframes skeleton-shimmer {
  0% { background-position: -200% 0; }
  100% { background-position: 200% 0; }
}

.skeleton {
  background: linear-gradient(
    90deg,
    var(--skeleton-base) 25%,
    var(--skeleton-shine) 50%,
    var(--skeleton-base) 75%
  );
  background-size: 200% 100%;
  animation: skeleton-shimmer 1.5s ease-in-out infinite;
  border-radius: 6px;
}

/* ── Focus ring ────────────────────────────────────────────────────────── */
.focus-ring:focus-visible {
  outline: 2px solid var(--border-focus);
  outline-offset: 2px;
}

/* ── Smooth theme transitions ──────────────────────────────────────────── */
html.theme-transition,
html.theme-transition *,
html.theme-transition *::before,
html.theme-transition *::after {
  transition:
    background-color 0.25s cubic-bezier(0.4, 0, 0.2, 1),
    color 0.15s cubic-bezier(0.4, 0, 0.2, 1),
    border-color 0.25s cubic-bezier(0.4, 0, 0.2, 1),
    box-shadow 0.25s cubic-bezier(0.4, 0, 0.2, 1) !important;
}

/* ── Skip navigation (accessibility) ───────────────────────────────────── */
.skip-link {
  position: absolute;
  top: -100%;
  left: 0;
  z-index: 9999;
  padding: 8px 16px;
  background: var(--accent);
  color: var(--text-on-accent);
  font-weight: 600;
  text-decoration: none;
  border-radius: 0 0 6px 0;
  transition: top 0.15s ease;
}
.skip-link:focus {
  top: 0;
}

/* ── Reduced motion ────────────────────────────────────────────────────── */
@media (prefers-reduced-motion: reduce) {
  *, *::before, *::after {
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.01ms !important;
  }
}
"#;

/// Inject theme CSS into the document head.
#[cfg(target_arch = "wasm32")]
pub fn inject_theme_css() {
    use wasm_bindgen::JsCast;

    if let Some(window) = web_sys::window() {
        if let Some(doc) = window.document() {
            if doc.query_selector("#ferro-theme-css").ok().flatten().is_some() {
                return;
            }

            if let Some(style) = doc
                .create_element("style")
                .ok()
                .and_then(|e| e.dyn_into::<web_sys::HtmlStyleElement>().ok())
            {
                style.set_id("ferro-theme-css");
                let _ = style.set_text_content(Some(THEME_CSS));
                if let Some(head) = doc.head() {
                    let _ = head.append_child(&style);
                }
            }
        }
    }
}

/// Resolve the actual theme to apply (resolves "system" to light/dark).
#[cfg(target_arch = "wasm32")]
pub fn resolve_effective_theme(theme: Theme) -> &'static str {
    match theme {
        Theme::System => detect_system_theme(),
        other => other.as_str(),
    }
}

/// Apply theme to the DOM with smooth transition.
#[cfg(target_arch = "wasm32")]
pub fn apply_theme(theme: &str) {
    use wasm_bindgen::JsCast;

    if let Some(window) = web_sys::window() {
        if let Some(doc) = window.document() {
            if let Some(html) = doc.document_element() {
                let _ = html.class_list().add_1("theme-transition");
                let _ = html.set_attribute("data-theme", theme);

                // Toggle dark class for any Tailwind-like utilities
                if theme == "dark" || theme == "midnight" {
                    let _ = html.class_list().add_1("dark");
                } else {
                    let _ = html.class_list().remove_1("dark");
                }

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
                        300,
                    );
                }
            }
        }
    }
}

/// Persist theme to localStorage.
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

/// Resolve initial theme: localStorage > system preference > light default.
#[cfg(target_arch = "wasm32")]
pub fn resolve_initial_theme() -> Theme {
    if let Some(stored) = read_persisted_theme() {
        return Theme::from_str_value(&stored);
    }
    Theme::System
}
