//! Theme management system for Ferro.
//!
//! Supports fourteen themes: light, dark, midnight, system (auto-detect),
//! solarized-light, solarized-dark, nord, tokyo-night, dracula, high-contrast,
//! sepia, forest, ocean, and custom (user-defined via localStorage).
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
    SolarizedLight,
    SolarizedDark,
    Nord,
    TokyoNight,
    Dracula,
    HighContrast,
    Sepia,
    Forest,
    Ocean,
    Custom,
}

impl Theme {
    pub fn as_str(&self) -> &'static str {
        match self {
            Theme::Light => "light",
            Theme::Dark => "dark",
            Theme::Midnight => "midnight",
            Theme::System => "system",
            Theme::SolarizedLight => "solarized-light",
            Theme::SolarizedDark => "solarized-dark",
            Theme::Nord => "nord",
            Theme::TokyoNight => "tokyo-night",
            Theme::Dracula => "dracula",
            Theme::HighContrast => "high-contrast",
            Theme::Sepia => "sepia",
            Theme::Forest => "forest",
            Theme::Ocean => "ocean",
            Theme::Custom => "custom",
        }
    }

    pub fn from_str_value(s: &str) -> Self {
        match s {
            "dark" => Theme::Dark,
            "midnight" => Theme::Midnight,
            "system" => Theme::System,
            "solarized-light" => Theme::SolarizedLight,
            "solarized-dark" => Theme::SolarizedDark,
            "nord" => Theme::Nord,
            "tokyo-night" => Theme::TokyoNight,
            "dracula" => Theme::Dracula,
            "high-contrast" => Theme::HighContrast,
            "sepia" => Theme::Sepia,
            "forest" => Theme::Forest,
            "ocean" => Theme::Ocean,
            "custom" => Theme::Custom,
            _ => Theme::Light,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Theme::Light => "Light",
            Theme::Dark => "Dark",
            Theme::Midnight => "Midnight",
            Theme::System => "System",
            Theme::SolarizedLight => "Solarized Light",
            Theme::SolarizedDark => "Solarized Dark",
            Theme::Nord => "Nord",
            Theme::TokyoNight => "Tokyo Night",
            Theme::Dracula => "Dracula",
            Theme::HighContrast => "High Contrast",
            Theme::Sepia => "Sepia",
            Theme::Forest => "Forest",
            Theme::Ocean => "Ocean",
            Theme::Custom => "Custom",
        }
    }

    pub fn all() -> &'static [Theme] {
        &[
            Theme::Light,
            Theme::Dark,
            Theme::Midnight,
            Theme::System,
            Theme::SolarizedLight,
            Theme::SolarizedDark,
            Theme::Nord,
            Theme::TokyoNight,
            Theme::Dracula,
            Theme::HighContrast,
            Theme::Sepia,
            Theme::Forest,
            Theme::Ocean,
            Theme::Custom,
        ]
    }

    pub fn is_dark(&self) -> bool {
        matches!(
            self,
            Theme::Dark
                | Theme::Midnight
                | Theme::SolarizedDark
                | Theme::Nord
                | Theme::TokyoNight
                | Theme::Dracula
                | Theme::HighContrast
                | Theme::Forest
                | Theme::Ocean
        )
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

/* ── Solarized Light (Ethan Schoonover) ────────────────────────────────── */
[data-theme="solarized-light"] {
  --bg-base: #fdf6e3;
  --bg-surface: #eee8d5;
  --bg-surface-raised: #eee8d5;
  --bg-surface-sunken: #f5efdc;
  --bg-surface-overlay: #eee8d5;
  --bg-inset: #f5efdc;

  --text-primary: #073642;
  --text-secondary: #586e75;
  --text-tertiary: #657b83;
  --text-inverse: #fdf6e3;
  --text-on-accent: #fdf6e3;

  --border-default: #d3cbb9;
  --border-strong: #b8ad98;
  --border-subtle: #eee8d5;
  --border-focus: #268bd2;

  --accent: #268bd2;
  --accent-hover: #1a7ab8;
  --accent-active: #14608c;
  --accent-subtle: rgba(38, 139, 210, 0.08);
  --accent-muted: rgba(38, 139, 210, 0.15);

  --danger: #dc322f;
  --danger-hover: #c52825;
  --danger-subtle: rgba(220, 50, 47, 0.08);
  --success: #859900;
  --success-hover: #6d7d00;
  --success-subtle: rgba(133, 153, 0, 0.08);
  --warning: #b58900;
  --warning-hover: #9a7400;
  --warning-subtle: rgba(181, 137, 0, 0.08);
  --info: #268bd2;
  --info-subtle: rgba(38, 139, 210, 0.08);

  --interactive-hover: #eee8d5;
  --interactive-active: #ddd6c4;
  --interactive-selected: rgba(38, 139, 210, 0.08);

  --shadow-xs: 0 1px 2px rgba(0, 0, 0, 0.04);
  --shadow-sm: 0 1px 3px rgba(0, 0, 0, 0.06), 0 1px 2px rgba(0, 0, 0, 0.04);
  --shadow-md: 0 4px 6px -1px rgba(0, 0, 0, 0.07), 0 2px 4px -2px rgba(0, 0, 0, 0.05);
  --shadow-lg: 0 10px 15px -3px rgba(0, 0, 0, 0.08), 0 4px 6px -4px rgba(0, 0, 0, 0.04);
  --shadow-xl: 0 20px 25px -5px rgba(0, 0, 0, 0.1), 0 8px 10px -6px rgba(0, 0, 0, 0.06);

  --overlay: rgba(0, 0, 0, 0.4);
  --overlay-heavy: rgba(0, 0, 0, 0.6);

  --dropzone-bg: rgba(38, 139, 210, 0.04);
  --dropzone-border: rgba(38, 139, 210, 0.3);
  --dropzone-active-bg: rgba(38, 139, 210, 0.1);
  --dropzone-active-border: rgba(38, 139, 210, 0.6);

  --scrollbar-track: transparent;
  --scrollbar-thumb: #b8ad98;
  --scrollbar-thumb-hover: #9a8e7a;

  --skeleton-base: #d3cbb9;
  --skeleton-shine: #eee8d5;

  color-scheme: light;
}

/* ── Solarized Dark (Ethan Schoonover) ─────────────────────────────────── */
[data-theme="solarized-dark"] {
  --bg-base: #002b36;
  --bg-surface: #073642;
  --bg-surface-raised: #0a4050;
  --bg-surface-sunken: #001f28;
  --bg-surface-overlay: #0a4050;
  --bg-inset: #00222e;

  --text-primary: #839496;
  --text-secondary: #93a1a1;
  --text-tertiary: #657b83;
  --text-inverse: #002b36;
  --text-on-accent: #fdf6e3;

  --border-default: #0a4050;
  --border-strong: #0e5060;
  --border-subtle: #05303c;
  --border-focus: #268bd2;

  --accent: #268bd2;
  --accent-hover: #4aa0dc;
  --accent-active: #6cb8e4;
  --accent-subtle: rgba(38, 139, 210, 0.1);
  --accent-muted: rgba(38, 139, 210, 0.18);

  --danger: #dc322f;
  --danger-hover: #e84845;
  --danger-subtle: rgba(220, 50, 47, 0.12);
  --success: #859900;
  --success-hover: #a0b500;
  --success-subtle: rgba(133, 153, 0, 0.12);
  --warning: #b58900;
  --warning-hover: #d0a000;
  --warning-subtle: rgba(181, 137, 0, 0.12);
  --info: #268bd2;
  --info-subtle: rgba(38, 139, 210, 0.12);

  --interactive-hover: #073642;
  --interactive-active: #0a4050;
  --interactive-selected: rgba(38, 139, 210, 0.12);

  --shadow-xs: 0 1px 2px rgba(0, 0, 0, 0.2);
  --shadow-sm: 0 1px 3px rgba(0, 0, 0, 0.3), 0 1px 2px rgba(0, 0, 0, 0.2);
  --shadow-md: 0 4px 6px -1px rgba(0, 0, 0, 0.35), 0 2px 4px -2px rgba(0, 0, 0, 0.25);
  --shadow-lg: 0 10px 15px -3px rgba(0, 0, 0, 0.4), 0 4px 6px -4px rgba(0, 0, 0, 0.25);
  --shadow-xl: 0 20px 25px -5px rgba(0, 0, 0, 0.5), 0 8px 10px -6px rgba(0, 0, 0, 0.3);

  --overlay: rgba(0, 0, 0, 0.6);
  --overlay-heavy: rgba(0, 0, 0, 0.8);

  --dropzone-bg: rgba(38, 139, 210, 0.06);
  --dropzone-border: rgba(38, 139, 210, 0.3);
  --dropzone-active-bg: rgba(38, 139, 210, 0.12);
  --dropzone-active-border: rgba(38, 139, 210, 0.6);

  --scrollbar-track: transparent;
  --scrollbar-thumb: #0e5060;
  --scrollbar-thumb-hover: #1a6070;

  --skeleton-base: #05303c;
  --skeleton-shine: #073642;

  color-scheme: dark;
}

/* ── Nord (Arctic color scheme) ─────────────────────────────────────────── */
[data-theme="nord"] {
  --bg-base: #2e3440;
  --bg-surface: #3b4252;
  --bg-surface-raised: #434c5e;
  --bg-surface-sunken: #272c36;
  --bg-surface-overlay: #434c5e;
  --bg-inset: #2a303c;

  --text-primary: #eceff4;
  --text-secondary: #d8dee9;
  --text-tertiary: #a0aabe;
  --text-inverse: #2e3440;
  --text-on-accent: #2e3440;

  --border-default: #3b4252;
  --border-strong: #4c566a;
  --border-subtle: #2e3440;
  --border-focus: #88c0d0;

  --accent: #88c0d0;
  --accent-hover: #8fbcbb;
  --accent-active: #81a1c1;
  --accent-subtle: rgba(136, 192, 208, 0.08);
  --accent-muted: rgba(136, 192, 208, 0.15);

  --danger: #bf616a;
  --danger-hover: #d08770;
  --danger-subtle: rgba(191, 97, 106, 0.12);
  --success: #a3be8c;
  --success-hover: #b48ead;
  --success-subtle: rgba(163, 190, 140, 0.12);
  --warning: #ebcb8b;
  --warning-hover: #d08770;
  --warning-subtle: rgba(235, 203, 139, 0.12);
  --info: #88c0d0;
  --info-subtle: rgba(136, 192, 208, 0.12);

  --interactive-hover: #3b4252;
  --interactive-active: #434c5e;
  --interactive-selected: rgba(136, 192, 208, 0.12);

  --shadow-xs: 0 1px 2px rgba(0, 0, 0, 0.2);
  --shadow-sm: 0 1px 3px rgba(0, 0, 0, 0.3), 0 1px 2px rgba(0, 0, 0, 0.2);
  --shadow-md: 0 4px 6px -1px rgba(0, 0, 0, 0.35), 0 2px 4px -2px rgba(0, 0, 0, 0.25);
  --shadow-lg: 0 10px 15px -3px rgba(0, 0, 0, 0.4), 0 4px 6px -4px rgba(0, 0, 0, 0.25);
  --shadow-xl: 0 20px 25px -5px rgba(0, 0, 0, 0.5), 0 8px 10px -6px rgba(0, 0, 0, 0.3);

  --overlay: rgba(0, 0, 0, 0.6);
  --overlay-heavy: rgba(0, 0, 0, 0.8);

  --dropzone-bg: rgba(136, 192, 208, 0.06);
  --dropzone-border: rgba(136, 192, 208, 0.3);
  --dropzone-active-bg: rgba(136, 192, 208, 0.12);
  --dropzone-active-border: rgba(136, 192, 208, 0.6);

  --scrollbar-track: transparent;
  --scrollbar-thumb: #4c566a;
  --scrollbar-thumb-hover: #5e81ac;

  --skeleton-base: #2a303c;
  --skeleton-shine: #3b4252;

  color-scheme: dark;
}

/* ── Tokyo Night ────────────────────────────────────────────────────────── */
[data-theme="tokyo-night"] {
  --bg-base: #1a1b26;
  --bg-surface: #24283b;
  --bg-surface-raised: #292e42;
  --bg-surface-sunken: #16161e;
  --bg-surface-overlay: #292e42;
  --bg-inset: #1a1f2e;

  --text-primary: #c0caf5;
  --text-secondary: #a9b1d6;
  --text-tertiary: #565f89;
  --text-inverse: #1a1b26;
  --text-on-accent: #1a1b26;

  --border-default: #292e42;
  --border-strong: #414868;
  --border-subtle: #1f2335;
  --border-focus: #7aa2f7;

  --accent: #7aa2f7;
  --accent-hover: #89b4fa;
  --accent-active: #7dcfff;
  --accent-subtle: rgba(122, 162, 247, 0.08);
  --accent-muted: rgba(122, 162, 247, 0.15);

  --danger: #f7768e;
  --danger-hover: #ff7a93;
  --danger-subtle: rgba(247, 118, 142, 0.12);
  --success: #9ece6a;
  --success-hover: #a9da6a;
  --success-subtle: rgba(158, 206, 106, 0.12);
  --warning: #e0af68;
  --warning-hover: #f0c078;
  --warning-subtle: rgba(224, 175, 104, 0.12);
  --info: #7aa2f7;
  --info-subtle: rgba(122, 162, 247, 0.12);

  --interactive-hover: #292e42;
  --interactive-active: #343a52;
  --interactive-selected: rgba(122, 162, 247, 0.12);

  --shadow-xs: 0 1px 2px rgba(0, 0, 0, 0.2);
  --shadow-sm: 0 1px 3px rgba(0, 0, 0, 0.3), 0 1px 2px rgba(0, 0, 0, 0.2);
  --shadow-md: 0 4px 6px -1px rgba(0, 0, 0, 0.35), 0 2px 4px -2px rgba(0, 0, 0, 0.25);
  --shadow-lg: 0 10px 15px -3px rgba(0, 0, 0, 0.4), 0 4px 6px -4px rgba(0, 0, 0, 0.25);
  --shadow-xl: 0 20px 25px -5px rgba(0, 0, 0, 0.5), 0 8px 10px -6px rgba(0, 0, 0, 0.3);

  --overlay: rgba(0, 0, 0, 0.6);
  --overlay-heavy: rgba(0, 0, 0, 0.8);

  --dropzone-bg: rgba(122, 162, 247, 0.06);
  --dropzone-border: rgba(122, 162, 247, 0.3);
  --dropzone-active-bg: rgba(122, 162, 247, 0.12);
  --dropzone-active-border: rgba(122, 162, 247, 0.6);

  --scrollbar-track: transparent;
  --scrollbar-thumb: #414868;
  --scrollbar-thumb-hover: #565f89;

  --skeleton-base: #1f2335;
  --skeleton-shine: #292e42;

  color-scheme: dark;
}

/* ── Dracula ────────────────────────────────────────────────────────────── */
[data-theme="dracula"] {
  --bg-base: #282a36;
  --bg-surface: #44475a;
  --bg-surface-raised: #525770;
  --bg-surface-sunken: #1e1f29;
  --bg-surface-overlay: #44475a;
  --bg-inset: #21222c;

  --text-primary: #f8f8f2;
  --text-secondary: #cccce0;
  --text-tertiary: #6272a4;
  --text-inverse: #282a36;
  --text-on-accent: #282a36;

  --border-default: #44475a;
  --border-strong: #6272a4;
  --border-subtle: #343746;
  --border-focus: #bd93f9;

  --accent: #bd93f9;
  --accent-hover: #caaafa;
  --accent-active: #ff79c6;
  --accent-subtle: rgba(189, 147, 249, 0.08);
  --accent-muted: rgba(189, 147, 249, 0.15);

  --danger: #ff5555;
  --danger-hover: #ff6e6e;
  --danger-subtle: rgba(255, 85, 85, 0.12);
  --success: #50fa7b;
  --success-hover: #6ffb93;
  --success-subtle: rgba(80, 250, 123, 0.12);
  --warning: #f1fa8c;
  --warning-hover: #f4fbad;
  --warning-subtle: rgba(241, 250, 140, 0.12);
  --info: #8be9fd;
  --info-subtle: rgba(139, 233, 253, 0.12);

  --interactive-hover: #44475a;
  --interactive-active: #525770;
  --interactive-selected: rgba(189, 147, 249, 0.12);

  --shadow-xs: 0 1px 2px rgba(0, 0, 0, 0.2);
  --shadow-sm: 0 1px 3px rgba(0, 0, 0, 0.3), 0 1px 2px rgba(0, 0, 0, 0.2);
  --shadow-md: 0 4px 6px -1px rgba(0, 0, 0, 0.35), 0 2px 4px -2px rgba(0, 0, 0, 0.25);
  --shadow-lg: 0 10px 15px -3px rgba(0, 0, 0, 0.4), 0 4px 6px -4px rgba(0, 0, 0, 0.25);
  --shadow-xl: 0 20px 25px -5px rgba(0, 0, 0, 0.5), 0 8px 10px -6px rgba(0, 0, 0, 0.3);

  --overlay: rgba(0, 0, 0, 0.6);
  --overlay-heavy: rgba(0, 0, 0, 0.8);

  --dropzone-bg: rgba(189, 147, 249, 0.06);
  --dropzone-border: rgba(189, 147, 249, 0.3);
  --dropzone-active-bg: rgba(189, 147, 249, 0.12);
  --dropzone-active-border: rgba(189, 147, 249, 0.6);

  --scrollbar-track: transparent;
  --scrollbar-thumb: #6272a4;
  --scrollbar-thumb-hover: #7a80b0;

  --skeleton-base: #343746;
  --skeleton-shine: #44475a;

  color-scheme: dark;
}

/* ── High Contrast (WCAG AAA) ──────────────────────────────────────────── */
[data-theme="high-contrast"] {
  --bg-base: #000000;
  --bg-surface: #0a0a0a;
  --bg-surface-raised: #141414;
  --bg-surface-sunken: #000000;
  --bg-surface-overlay: #141414;
  --bg-inset: #050505;

  --text-primary: #ffffff;
  --text-secondary: #e0e0e0;
  --text-tertiary: #b0b0b0;
  --text-inverse: #000000;
  --text-on-accent: #000000;

  --border-default: #ffffff;
  --border-strong: #ffffff;
  --border-subtle: #808080;
  --border-focus: #00d4ff;

  --accent: #00d4ff;
  --accent-hover: #33ddff;
  --accent-active: #00b8e0;
  --accent-subtle: rgba(0, 212, 255, 0.12);
  --accent-muted: rgba(0, 212, 255, 0.2);

  --danger: #ff4444;
  --danger-hover: #ff6666;
  --danger-subtle: rgba(255, 68, 68, 0.15);
  --success: #00ff88;
  --success-hover: #33ff9f;
  --success-subtle: rgba(0, 255, 136, 0.15);
  --warning: #ffcc00;
  --warning-hover: #ffd633;
  --warning-subtle: rgba(255, 204, 0, 0.15);
  --info: #00d4ff;
  --info-subtle: rgba(0, 212, 255, 0.15);

  --interactive-hover: #1a1a1a;
  --interactive-active: #333333;
  --interactive-selected: rgba(0, 212, 255, 0.15);

  --shadow-xs: 0 1px 2px rgba(0, 0, 0, 0.5);
  --shadow-sm: 0 1px 3px rgba(0, 0, 0, 0.6), 0 1px 2px rgba(0, 0, 0, 0.5);
  --shadow-md: 0 4px 6px -1px rgba(0, 0, 0, 0.65), 0 2px 4px -2px rgba(0, 0, 0, 0.55);
  --shadow-lg: 0 10px 15px -3px rgba(0, 0, 0, 0.7), 0 4px 6px -4px rgba(0, 0, 0, 0.55);
  --shadow-xl: 0 20px 25px -5px rgba(0, 0, 0, 0.8), 0 8px 10px -6px rgba(0, 0, 0, 0.6);

  --overlay: rgba(0, 0, 0, 0.7);
  --overlay-heavy: rgba(0, 0, 0, 0.9);

  --dropzone-bg: rgba(0, 212, 255, 0.08);
  --dropzone-border: rgba(0, 212, 255, 0.5);
  --dropzone-active-bg: rgba(0, 212, 255, 0.15);
  --dropzone-active-border: rgba(0, 212, 255, 0.8);

  --scrollbar-track: transparent;
  --scrollbar-thumb: #808080;
  --scrollbar-thumb-hover: #ffffff;

  --skeleton-base: #333333;
  --skeleton-shine: #555555;

  color-scheme: dark;
}

/* ── Sepia (warm, low blue light) ───────────────────────────────────────── */
[data-theme="sepia"] {
  --bg-base: #f4ecd8;
  --bg-surface: #ede4cf;
  --bg-surface-raised: #e8dfc9;
  --bg-surface-sunken: #f8f0de;
  --bg-surface-overlay: #ede4cf;
  --bg-inset: #f0e8d5;

  --text-primary: #433422;
  --text-secondary: #5b4636;
  --text-tertiary: #7a6652;
  --text-inverse: #f4ecd8;
  --text-on-accent: #f4ecd8;

  --border-default: #d4c9b4;
  --border-strong: #b8a98e;
  --border-subtle: #e5dcc9;
  --border-focus: #8b6914;

  --accent: #8b6914;
  --accent-hover: #a07a18;
  --accent-active: #6e5310;
  --accent-subtle: rgba(139, 105, 20, 0.08);
  --accent-muted: rgba(139, 105, 20, 0.15);

  --danger: #c23b22;
  --danger-hover: #a8301c;
  --danger-subtle: rgba(194, 59, 34, 0.08);
  --success: #5a7a32;
  --success-hover: #4a6628;
  --success-subtle: rgba(90, 122, 50, 0.08);
  --warning: #b8860b;
  --warning-hover: #9a720a;
  --warning-subtle: rgba(184, 134, 11, 0.08);
  --info: #5a7a8c;
  --info-subtle: rgba(90, 122, 140, 0.08);

  --interactive-hover: #ede4cf;
  --interactive-active: #e5dcc9;
  --interactive-selected: rgba(139, 105, 20, 0.08);

  --shadow-xs: 0 1px 2px rgba(67, 52, 34, 0.04);
  --shadow-sm: 0 1px 3px rgba(67, 52, 34, 0.06), 0 1px 2px rgba(67, 52, 34, 0.04);
  --shadow-md: 0 4px 6px -1px rgba(67, 52, 34, 0.07), 0 2px 4px -2px rgba(67, 52, 34, 0.05);
  --shadow-lg: 0 10px 15px -3px rgba(67, 52, 34, 0.08), 0 4px 6px -4px rgba(67, 52, 34, 0.04);
  --shadow-xl: 0 20px 25px -5px rgba(67, 52, 34, 0.1), 0 8px 10px -6px rgba(67, 52, 34, 0.06);

  --overlay: rgba(67, 52, 34, 0.4);
  --overlay-heavy: rgba(67, 52, 34, 0.6);

  --dropzone-bg: rgba(139, 105, 20, 0.04);
  --dropzone-border: rgba(139, 105, 20, 0.3);
  --dropzone-active-bg: rgba(139, 105, 20, 0.1);
  --dropzone-active-border: rgba(139, 105, 20, 0.6);

  --scrollbar-track: transparent;
  --scrollbar-thumb: #b8a98e;
  --scrollbar-thumb-hover: #9a8b70;

  --skeleton-base: #d4c9b4;
  --skeleton-shine: #e5dcc9;

  color-scheme: light;
}

/* ── Forest (green-toned dark) ─────────────────────────────────────────── */
[data-theme="forest"] {
  --bg-base: #0d1a0d;
  --bg-surface: #1a2e1a;
  --bg-surface-raised: #243824;
  --bg-surface-sunken: #091209;
  --bg-surface-overlay: #243824;
  --bg-inset: #111f11;

  --text-primary: #c8e6c8;
  --text-secondary: #a8d5a2;
  --text-tertiary: #6b8f65;
  --text-inverse: #0d1a0d;
  --text-on-accent: #0d1a0d;

  --border-default: #2a4a2a;
  --border-strong: #3a6a3a;
  --border-subtle: #1a301a;
  --border-focus: #4ade80;

  --accent: #4ade80;
  --accent-hover: #6ee89a;
  --accent-active: #2dc868;
  --accent-subtle: rgba(74, 222, 128, 0.08);
  --accent-muted: rgba(74, 222, 128, 0.15);

  --danger: #ef5350;
  --danger-hover: #f44336;
  --danger-subtle: rgba(239, 83, 80, 0.12);
  --success: #4ade80;
  --success-hover: #22c55e;
  --success-subtle: rgba(74, 222, 128, 0.12);
  --warning: #fbbf24;
  --warning-hover: #f59e0b;
  --warning-subtle: rgba(251, 191, 36, 0.12);
  --info: #4ade80;
  --info-subtle: rgba(74, 222, 128, 0.12);

  --interactive-hover: #1a2e1a;
  --interactive-active: #243824;
  --interactive-selected: rgba(74, 222, 128, 0.12);

  --shadow-xs: 0 1px 2px rgba(0, 0, 0, 0.2);
  --shadow-sm: 0 1px 3px rgba(0, 0, 0, 0.3), 0 1px 2px rgba(0, 0, 0, 0.2);
  --shadow-md: 0 4px 6px -1px rgba(0, 0, 0, 0.35), 0 2px 4px -2px rgba(0, 0, 0, 0.25);
  --shadow-lg: 0 10px 15px -3px rgba(0, 0, 0, 0.4), 0 4px 6px -4px rgba(0, 0, 0, 0.25);
  --shadow-xl: 0 20px 25px -5px rgba(0, 0, 0, 0.5), 0 8px 10px -6px rgba(0, 0, 0, 0.3);

  --overlay: rgba(0, 0, 0, 0.6);
  --overlay-heavy: rgba(0, 0, 0, 0.8);

  --dropzone-bg: rgba(74, 222, 128, 0.06);
  --dropzone-border: rgba(74, 222, 128, 0.3);
  --dropzone-active-bg: rgba(74, 222, 128, 0.12);
  --dropzone-active-border: rgba(74, 222, 128, 0.6);

  --scrollbar-track: transparent;
  --scrollbar-thumb: #3a6a3a;
  --scrollbar-thumb-hover: #4a8a4a;

  --skeleton-base: #1a301a;
  --skeleton-shine: #243824;

  color-scheme: dark;
}

/* ── Ocean (blue-teal dark) ─────────────────────────────────────────────── */
[data-theme="ocean"] {
  --bg-base: #0a1628;
  --bg-surface: #0f2035;
  --bg-surface-raised: #152a42;
  --bg-surface-sunken: #060f1e;
  --bg-surface-overlay: #152a42;
  --bg-inset: #0c1a30;

  --text-primary: #d4e8f8;
  --text-secondary: #a0c4e8;
  --text-tertiary: #5a8ab5;
  --text-inverse: #0a1628;
  --text-on-accent: #0a1628;

  --border-default: #1a3555;
  --border-strong: #2a5080;
  --border-subtle: #102540;
  --border-focus: #22d3ee;

  --accent: #22d3ee;
  --accent-hover: #40ddf2;
  --accent-active: #0ec0d8;
  --accent-subtle: rgba(34, 211, 238, 0.08);
  --accent-muted: rgba(34, 211, 238, 0.15);

  --danger: #f87171;
  --danger-hover: #ef4444;
  --danger-subtle: rgba(248, 113, 113, 0.12);
  --success: #34d399;
  --success-hover: #10b981;
  --success-subtle: rgba(52, 211, 153, 0.12);
  --warning: #fbbf24;
  --warning-hover: #f59e0b;
  --warning-subtle: rgba(251, 191, 36, 0.12);
  --info: #22d3ee;
  --info-subtle: rgba(34, 211, 238, 0.12);

  --interactive-hover: #0f2035;
  --interactive-active: #152a42;
  --interactive-selected: rgba(34, 211, 238, 0.12);

  --shadow-xs: 0 1px 2px rgba(0, 0, 0, 0.2);
  --shadow-sm: 0 1px 3px rgba(0, 0, 0, 0.3), 0 1px 2px rgba(0, 0, 0, 0.2);
  --shadow-md: 0 4px 6px -1px rgba(0, 0, 0, 0.35), 0 2px 4px -2px rgba(0, 0, 0, 0.25);
  --shadow-lg: 0 10px 15px -3px rgba(0, 0, 0, 0.4), 0 4px 6px -4px rgba(0, 0, 0, 0.25);
  --shadow-xl: 0 20px 25px -5px rgba(0, 0, 0, 0.5), 0 8px 10px -6px rgba(0, 0, 0, 0.3);

  --overlay: rgba(0, 0, 0, 0.6);
  --overlay-heavy: rgba(0, 0, 0, 0.8);

  --dropzone-bg: rgba(34, 211, 238, 0.06);
  --dropzone-border: rgba(34, 211, 238, 0.3);
  --dropzone-active-bg: rgba(34, 211, 238, 0.12);
  --dropzone-active-border: rgba(34, 211, 238, 0.6);

  --scrollbar-track: transparent;
  --scrollbar-thumb: #2a5080;
  --scrollbar-thumb-hover: #3a6a9a;

  --skeleton-base: #102540;
  --skeleton-shine: #1a3555;

  color-scheme: dark;
}

/* ── Custom Theme (user-defined via localStorage) ───────────────────────── */
[data-theme="custom"] {
  --bg-base: #f8fafc;
  --bg-surface: #ffffff;
  --bg-surface-raised: #ffffff;
  --bg-surface-sunken: #f1f5f9;
  --bg-surface-overlay: #ffffff;
  --bg-inset: #f1f5f9;

  --text-primary: #0f172a;
  --text-secondary: #475569;
  --text-tertiary: #94a3b8;
  --text-inverse: #ffffff;
  --text-on-accent: #ffffff;

  --border-default: #e2e8f0;
  --border-strong: #cbd5e1;
  --border-subtle: #f1f5f9;
  --border-focus: #3b82f6;

  --accent: #3b82f6;
  --accent-hover: #2563eb;
  --accent-active: #1d4ed8;
  --accent-subtle: rgba(59, 130, 246, 0.08);
  --accent-muted: rgba(59, 130, 246, 0.15);

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

  --interactive-hover: #f8fafc;
  --interactive-active: #f1f5f9;
  --interactive-selected: rgba(59, 130, 246, 0.08);

  --shadow-xs: 0 1px 2px rgba(0, 0, 0, 0.04);
  --shadow-sm: 0 1px 3px rgba(0, 0, 0, 0.06), 0 1px 2px rgba(0, 0, 0, 0.04);
  --shadow-md: 0 4px 6px -1px rgba(0, 0, 0, 0.07), 0 2px 4px -2px rgba(0, 0, 0, 0.05);
  --shadow-lg: 0 10px 15px -3px rgba(0, 0, 0, 0.08), 0 4px 6px -4px rgba(0, 0, 0, 0.04);
  --shadow-xl: 0 20px 25px -5px rgba(0, 0, 0, 0.1), 0 8px 10px -6px rgba(0, 0, 0, 0.06);

  --overlay: rgba(0, 0, 0, 0.4);
  --overlay-heavy: rgba(0, 0, 0, 0.6);

  --dropzone-bg: rgba(59, 130, 246, 0.04);
  --dropzone-border: rgba(59, 130, 246, 0.3);
  --dropzone-active-bg: rgba(59, 130, 246, 0.1);
  --dropzone-active-border: rgba(59, 130, 246, 0.6);

  --scrollbar-track: transparent;
  --scrollbar-thumb: #cbd5e1;
  --scrollbar-thumb-hover: #94a3b8;

  --skeleton-base: #e2e8f0;
  --skeleton-shine: #f1f5f9;

  color-scheme: light;
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

.font-display { font-family: var(--font-display, ui-sans-serif, system-ui, sans-serif); }

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

                let parsed = Theme::from_str_value(theme);
                if parsed.is_dark() {
                    let _ = html.class_list().add_1("dark");
                } else {
                    let _ = html.class_list().remove_1("dark");
                }

                if theme == "custom" {
                    apply_custom_theme_overrides();
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

/// Apply custom theme CSS variable overrides from localStorage.
///
/// Expects `ferro_custom_theme` to contain a JSON object mapping CSS variable
/// names (without `--` prefix) to their values, e.g.:
/// `{"bg-base": "#1a1a2e", "accent": "#ff6b6b", "text-primary": "#e0e0e0"}`
#[cfg(target_arch = "wasm32")]
pub fn apply_custom_theme_overrides() {
    use wasm_bindgen::JsCast;

    let overrides_json = web_sys::window()
        .and_then(|w| w.local_storage().ok())
        .flatten()
        .and_then(|s| s.get_item("ferro_custom_theme").ok())
        .flatten();

    if let Some(json) = overrides_json {
        if let Ok(obj) = serde_json::from_str::<serde_json::Value>(&json) {
            if let Some(map) = obj.as_object() {
                if let Some(window) = web_sys::window() {
                    if let Some(doc) = window.document() {
                        if let Some(style) = doc
                            .create_element("style")
                            .ok()
                            .and_then(|e| e.dyn_into::<web_sys::HtmlStyleElement>().ok())
                        {
                            style.set_id("ferro-custom-theme-overrides");
                            let mut css = String::from("[data-theme=\"custom\"] {\n");
                            for (key, value) in map {
                                if let Some(val_str) = value.as_str() {
                                    css.push_str(&format!("  --{}: {};\n", key, val_str));
                                }
                            }
                            css.push_str("}\n");
                            let _ = style.set_text_content(Some(&css));
                            if let Some(head) = doc.head() {
                                if let Some(existing) =
                                    doc.query_selector("#ferro-custom-theme-overrides")
                                        .ok()
                                        .flatten()
                                {
                                    let _ = head.remove_child(&existing);
                                }
                                let _ = head.append_child(&style);
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Resolve initial theme: localStorage > system preference > light default.
#[cfg(target_arch = "wasm32")]
pub fn resolve_initial_theme() -> Theme {
    if let Some(stored) = read_persisted_theme() {
        return Theme::from_str_value(&stored);
    }
    Theme::System
}
