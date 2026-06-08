// Design tokens as Rust constants.
// These mirror the CSS custom properties and provide a single source of truth
// for spacing, color, typography, and layout values across the web crate.
// ── Colors ──────────────────────────────────────────────────────────────

pub const COLOR_PRIMARY_50: &str = "#eff6ff";
pub const COLOR_PRIMARY_100: &str = "#dbeafe";
pub const COLOR_PRIMARY_200: &str = "#bfdbfe";
pub const COLOR_PRIMARY_300: &str = "#93c5fd";
pub const COLOR_PRIMARY_400: &str = "#60a5fa";
pub const COLOR_PRIMARY_500: &str = "#3b82f6";
pub const COLOR_PRIMARY_600: &str = "#2563eb";
pub const COLOR_PRIMARY_700: &str = "#1d4ed8";
pub const COLOR_PRIMARY_800: &str = "#1e40af";
pub const COLOR_PRIMARY_900: &str = "#1e3a8a";

pub const COLOR_DANGER_50: &str = "#fef2f2";
pub const COLOR_DANGER_500: &str = "#ef4444";
pub const COLOR_DANGER_600: &str = "#dc2626";
pub const COLOR_DANGER_700: &str = "#b91c1c";

pub const COLOR_SUCCESS_50: &str = "#f0fdf4";
pub const COLOR_SUCCESS_500: &str = "#22c55e";
pub const COLOR_SUCCESS_600: &str = "#16a34a";

pub const COLOR_WARNING_50: &str = "#fffbeb";
pub const COLOR_WARNING_500: &str = "#f59e0b";
pub const COLOR_WARNING_600: &str = "#d97706";

pub const COLOR_GRAY_50: &str = "#f9fafb";
pub const COLOR_GRAY_100: &str = "#f3f4f6";
pub const COLOR_GRAY_200: &str = "#e5e7eb";
pub const COLOR_GRAY_300: &str = "#d1d5db";
pub const COLOR_GRAY_400: &str = "#9ca3af";
pub const COLOR_GRAY_500: &str = "#6b7280";
pub const COLOR_GRAY_600: &str = "#4b5563";
pub const COLOR_GRAY_700: &str = "#374151";
pub const COLOR_GRAY_800: &str = "#1f2937";
pub const COLOR_GRAY_900: &str = "#111827";
pub const COLOR_GRAY_950: &str = "#030712";

// ── Spacing (4px grid) ─────────────────────────────────────────────────

pub const SPACE_0: &str = "0";
pub const SPACE_1: &str = "0.25rem"; // 4px
pub const SPACE_2: &str = "0.5rem"; // 8px
pub const SPACE_3: &str = "0.75rem"; // 12px
pub const SPACE_4: &str = "1rem"; // 16px
pub const SPACE_5: &str = "1.25rem"; // 20px
pub const SPACE_6: &str = "1.5rem"; // 24px
pub const SPACE_8: &str = "2rem"; // 32px
pub const SPACE_10: &str = "2.5rem"; // 40px
pub const SPACE_12: &str = "3rem"; // 48px
pub const SPACE_16: &str = "4rem"; // 64px

// ── Border radius ───────────────────────────────────────────────────────

pub const RADIUS_NONE: &str = "0";
pub const RADIUS_SM: &str = "0.25rem"; // 4px
pub const RADIUS_MD: &str = "0.375rem"; // 6px
pub const RADIUS_LG: &str = "0.5rem"; // 8px
pub const RADIUS_XL: &str = "0.75rem"; // 12px
pub const RADIUS_FULL: &str = "9999px";

// ── Typography ──────────────────────────────────────────────────────────

pub const FONT_SIZE_XS: &str = "0.75rem"; // 12px
pub const FONT_SIZE_SM: &str = "0.875rem"; // 14px
pub const FONT_SIZE_BASE: &str = "1rem"; // 16px
pub const FONT_SIZE_LG: &str = "1.125rem"; // 18px
pub const FONT_SIZE_XL: &str = "1.25rem"; // 20px
pub const FONT_SIZE_2XL: &str = "1.5rem"; // 24px
pub const FONT_SIZE_3XL: &str = "1.875rem"; // 30px

pub const FONT_WEIGHT_NORMAL: &str = "400";
pub const FONT_WEIGHT_MEDIUM: &str = "500";
pub const FONT_WEIGHT_SEMIBOLD: &str = "600";
pub const FONT_WEIGHT_BOLD: &str = "700";

pub const LINE_HEIGHT_TIGHT: &str = "1.25";
pub const LINE_HEIGHT_NORMAL: &str = "1.5";
pub const LINE_HEIGHT_RELAXED: &str = "1.75";

// ── Shadows ─────────────────────────────────────────────────────────────

pub const SHADOW_SM: &str = "0 1px 2px 0 rgb(0 0 0 / 0.05)";
pub const SHADOW_MD: &str = "0 4px 6px -1px rgb(0 0 0 / 0.1), 0 2px 4px -2px rgb(0 0 0 / 0.1)";
pub const SHADOW_LG: &str = "0 10px 15px -3px rgb(0 0 0 / 0.1), 0 4px 6px -4px rgb(0 0 0 / 0.1)";
pub const SHADOW_XL: &str = "0 20px 25px -5px rgb(0 0 0 / 0.1), 0 8px 10px -6px rgb(0 0 0 / 0.1)";

// ── Transitions ─────────────────────────────────────────────────────────

pub const TRANSITION_FAST: &str = "150ms cubic-bezier(0.4, 0, 0.2, 1)";
pub const TRANSITION_NORMAL: &str = "200ms cubic-bezier(0.4, 0, 0.2, 1)";
pub const TRANSITION_SLOW: &str = "300ms cubic-bezier(0.4, 0, 0.2, 1)";

// ── Z-index ─────────────────────────────────────────────────────────────

pub const Z_BASE: &str = "0";
pub const Z_DROPDOWN: &str = "1000";
pub const Z_STICKY: &str = "1020";
pub const Z_FIXED: &str = "1030";
pub const Z_BACKDROP: &str = "1040";
pub const Z_MODAL: &str = "1050";
pub const Z_POPOVER: &str = "1060";
pub const Z_TOOLTIP: &str = "1070";

// ── Breakpoints (as strings for media queries) ──────────────────────────

pub const BREAKPOINT_SM: &str = "640px";
pub const BREAKPOINT_MD: &str = "768px";
pub const BREAKPOINT_LG: &str = "1024px";
pub const BREAKPOINT_XL: &str = "1280px";

// ── Minimum touch target size (WCAG 2.5.8) ─────────────────────────────

pub const TOUCH_TARGET_MIN: &str = "44px";
