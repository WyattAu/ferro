/// Component-specific CSS class strings.
///
/// Each function returns a pre-built class string for a component variant.
/// All colors use CSS custom properties from the theme system for consistency.
/// Button component styles.
pub mod button {
    pub const BASE: &str = "inline-flex items-center justify-center font-medium transition-colors \
        focus:outline-none focus:ring-2 focus:ring-offset-2 \
        disabled:opacity-50 disabled:cursor-not-allowed \
        min-w-[44px] min-h-[44px]";

    pub const VARIANT_PRIMARY: &str = "bg-[var(--accent)] text-[var(--text-on-accent)] hover:bg-[var(--accent-hover)] focus:ring-[var(--border-focus)]";
    pub const VARIANT_SECONDARY: &str = "bg-[var(--bg-surface-raised)] text-[var(--text-primary)] border border-[var(--border-default)] \
         hover:bg-[var(--interactive-hover)] hover:border-[var(--border-strong)] focus:ring-[var(--border-focus)]";
    pub const VARIANT_DANGER: &str =
        "bg-[var(--danger)] text-white hover:bg-[var(--danger-hover)] focus:ring-[var(--danger)]";
    pub const VARIANT_GHOST: &str = "text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--interactive-hover)] focus:ring-[var(--border-focus)]";
    pub const VARIANT_LINK: &str =
        "text-[var(--accent)] hover:underline focus:ring-[var(--border-focus)] p-0 min-w-0 min-h-0";
    pub const VARIANT_OUTLINE: &str = "border border-[var(--border-default)] text-[var(--text-primary)] bg-transparent \
         hover:bg-[var(--interactive-hover)] hover:border-[var(--border-strong)] focus:ring-[var(--border-focus)]";
    pub const VARIANT_SOFT: &str = "bg-[var(--accent-subtle)] text-[var(--accent)] hover:bg-[var(--accent-muted)] focus:ring-[var(--border-focus)]";

    pub const SIZE_SM: &str = "px-3 py-1.5 text-sm rounded-md";
    pub const SIZE_MD: &str = "px-4 py-2 text-sm rounded-md";
    pub const SIZE_LG: &str = "px-6 py-3 text-base rounded-lg";
}

/// Input component styles.
pub mod input {
    pub const BASE: &str = "block w-full rounded-md border px-3 py-2 text-sm \
        placeholder-[var(--text-tertiary)] \
        focus:outline-none focus:ring-2 focus:ring-offset-1 \
        disabled:bg-[var(--bg-surface-sunken)] disabled:cursor-not-allowed \
        min-h-[44px] transition-colors";

    pub const DEFAULT: &str = "border-[var(--border-default)] bg-[var(--bg-surface)] text-[var(--text-primary)] \
         focus:ring-[var(--border-focus)] focus:border-[var(--border-focus)]";
    pub const ERROR: &str = "border-[var(--danger)] bg-[var(--bg-surface)] text-[var(--text-primary)] \
         focus:ring-[var(--danger)] focus:border-[var(--danger)]";
}

/// Dialog/Modal component styles.
pub mod dialog {
    pub const BACKDROP: &str = "fixed inset-0 z-50 flex items-center justify-center p-4 transition-all duration-200 \
         bg-[var(--overlay-heavy)] backdrop-blur-sm";

    pub const PANEL: &str = "bg-[var(--bg-surface)] rounded-xl shadow-xl p-6 w-[calc(100%-2rem)] sm:w-96 mx-auto \
         transition-all duration-200 border border-[var(--border-default)]";

    pub const TITLE: &str = "text-lg font-semibold text-[var(--text-primary)]";

    pub const CLOSE_BUTTON: &str = "p-1.5 rounded-md text-[var(--text-tertiary)] hover:text-[var(--text-primary)] \
         hover:bg-[var(--interactive-hover)] \
         focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] \
         min-w-[44px] min-h-[44px] flex items-center justify-center transition-colors";
}

/// Table component styles.
pub mod table {
    pub const TABLE: &str = "w-full text-sm text-left";

    pub const HEADER_CELL: &str = "px-4 py-3 font-semibold uppercase text-xs tracking-wider \
         text-[var(--text-tertiary)] border-b border-[var(--border-default)]";

    pub const SORT_BUTTON: &str = "flex items-center gap-1 hover:text-[var(--text-primary)] \
         focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] rounded min-h-[44px] px-1";

    pub const ROW: &str = "border-b border-[var(--border-subtle)] transition-colors";
    pub const ROW_HOVER: &str = "hover:bg-[var(--interactive-hover)]";
    pub const ROW_SELECTED: &str = "bg-[var(--interactive-selected)]";

    pub const CELL: &str = "px-4 py-2.5";

    pub const PAGINATION: &str = "flex items-center gap-1";
    pub const PAGE_BUTTON: &str = "p-2 rounded-md text-[var(--text-secondary)] hover:text-[var(--text-primary)] \
         hover:bg-[var(--interactive-hover)] \
         focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] \
         disabled:opacity-50 disabled:cursor-not-allowed \
         min-w-[44px] min-h-[44px] flex items-center justify-center transition-colors";
}

/// Checkbox component styles.
pub mod checkbox {
    pub const INPUT: &str = "w-4 h-4 rounded border-[var(--border-default)] text-[var(--accent)] \
         focus:ring-[var(--border-focus)] focus:ring-offset-1 \
         disabled:cursor-not-allowed bg-[var(--bg-surface)] min-w-[44px] min-h-[44px] \
         flex items-center justify-center cursor-pointer";

    pub const LABEL: &str = "text-sm text-[var(--text-primary)] select-none cursor-pointer pt-0.5";
}

/// Select component styles.
pub mod select {
    pub const BASE: &str = "block w-full rounded-md border px-3 py-2 text-sm \
        focus:outline-none focus:ring-2 focus:ring-offset-1 \
        disabled:bg-[var(--bg-surface-sunken)] disabled:cursor-not-allowed \
        min-h-[44px] transition-colors";

    pub const DEFAULT: &str = "border-[var(--border-default)] bg-[var(--bg-surface)] text-[var(--text-primary)] \
         focus:ring-[var(--border-focus)] focus:border-[var(--border-focus)]";
    pub const ERROR: &str = "border-[var(--danger)] bg-[var(--bg-surface)] text-[var(--text-primary)] \
         focus:ring-[var(--danger)] focus:border-[var(--danger)]";
}

/// Skeleton loading component styles.
pub mod skeleton {
    pub const BASE: &str = "skeleton rounded-md";
    pub const TEXT: &str = "h-4 w-full";
    pub const HEADING: &str = "h-6 w-3/4";
    pub const AVATAR: &str = "h-10 w-10 rounded-full";
    pub const THUMBNAIL: &str = "h-20 w-20 rounded-lg";
}

/// Badge/Tag component styles.
pub mod badge {
    pub const BASE: &str = "inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium";

    pub const DEFAULT: &str =
        "bg-[var(--bg-surface-raised)] text-[var(--text-secondary)] border border-[var(--border-default)]";
    pub const ACCENT: &str = "bg-[var(--accent-subtle)] text-[var(--accent)]";
    pub const DANGER: &str = "bg-[var(--danger-subtle)] text-[var(--danger)]";
    pub const SUCCESS: &str = "bg-[var(--success-subtle)] text-[var(--success)]";
    pub const WARNING: &str = "bg-[var(--warning-subtle)] text-[var(--warning)]";
}

/// Card component styles.
pub mod card {
    pub const BASE: &str = "bg-[var(--bg-surface)] border border-[var(--border-default)] rounded-xl shadow-sm";

    pub const HEADER: &str = "px-6 py-4 border-b border-[var(--border-default)]";

    pub const BODY: &str = "px-6 py-4";

    pub const FOOTER: &str = "px-6 py-4 border-t border-[var(--border-default)]";
}

/// Tooltip component styles.
pub mod tooltip {
    pub const BASE: &str = "absolute z-[1070] px-2.5 py-1.5 text-xs font-medium rounded-md \
         bg-[var(--bg-surface-overlay)] text-[var(--text-primary)] \
         border border-[var(--border-default)] shadow-md \
         pointer-events-none transition-opacity duration-150";
}
