/// Component-specific CSS class strings.
///
/// Each function returns a pre-built class string for a component variant.
/// These compose the design tokens into reusable class sets.
/// Button component styles.
pub mod button {

    pub const BASE: &str = "inline-flex items-center justify-center font-medium transition-colors \
        focus:outline-none focus:ring-2 focus:ring-offset-2 dark:focus:ring-offset-gray-800 \
        disabled:opacity-50 disabled:cursor-not-allowed";

    pub const VARIANT_PRIMARY: &str =
        "bg-blue-600 text-white hover:bg-blue-700 focus:ring-blue-500";
    pub const VARIANT_SECONDARY: &str = "bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-200 hover:bg-gray-200 dark:hover:bg-gray-600 focus:ring-gray-500";
    pub const VARIANT_DANGER: &str = "bg-red-600 text-white hover:bg-red-700 focus:ring-red-500";
    pub const VARIANT_GHOST: &str = "text-gray-600 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800 focus:ring-gray-500";

    pub const SIZE_SM: &str = "px-3 py-1.5 text-sm rounded min-h-[44px]";
    pub const SIZE_MD: &str = "px-4 py-2 text-sm rounded-md min-h-[44px]";
    pub const SIZE_LG: &str = "px-6 py-3 text-base rounded-md min-h-[44px]";
}

/// Input component styles.
pub mod input {

    pub const BASE: &str = "block w-full rounded-md border px-3 py-2 text-sm \
        placeholder-gray-400 dark:placeholder-gray-500 \
        focus:outline-none focus:ring-2 focus:ring-offset-1 dark:focus:ring-offset-gray-800 \
        disabled:bg-gray-100 dark:disabled:bg-gray-800 disabled:cursor-not-allowed min-h-[44px]";

    pub const DEFAULT: &str = "border-gray-300 dark:border-gray-600 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-800";
    pub const ERROR: &str =
        "border-red-300 dark:border-red-600 focus:ring-red-500 focus:border-red-500";
}

/// Dialog component styles.
pub mod dialog {

    pub const BACKDROP: &str = "fixed inset-0 bg-black bg-opacity-50 z-50 flex items-center justify-center p-4 transition-opacity duration-200";

    pub const PANEL: &str = "bg-white dark:bg-gray-800 rounded-lg shadow-xl p-6 w-[calc(100%-2rem)] sm:w-96 mx-auto transition-all duration-200";

    pub const TITLE: &str = "text-lg font-semibold text-gray-900 dark:text-gray-100";

    pub const CLOSE_BUTTON: &str = "p-1 rounded-sm text-gray-400 hover:text-gray-600 \
        focus:outline-none focus:ring-2 focus:ring-blue-500 min-w-[44px] min-h-[44px] \
        flex items-center justify-center";
}

/// Table component styles.
pub mod table {

    pub const TABLE: &str = "w-full text-sm text-left";

    pub const HEADER_CELL: &str = "px-4 py-3 font-bold uppercase text-xs tracking-wider \
        text-gray-600 dark:text-gray-400";

    pub const SORT_BUTTON: &str = "flex items-center gap-1 hover:text-gray-900 \
        focus:outline-none focus:ring-2 focus:ring-blue-500 rounded min-h-[44px] px-1";

    pub const ROW: &str = "border-b border-gray-100 dark:border-gray-800 transition-colors";
    pub const ROW_HOVER: &str = "hover:bg-gray-50 dark:hover:bg-gray-800";
    pub const ROW_SELECTED: &str = "bg-blue-50 dark:bg-blue-900/20";

    pub const CELL: &str = "px-4 py-2.5";

    pub const PAGINATION: &str = "flex items-center gap-1";
    pub const PAGE_BUTTON: &str = "p-2 rounded text-gray-500 hover:text-gray-700 hover:bg-gray-100 \
        focus:outline-none focus:ring-2 focus:ring-blue-500 disabled:opacity-50 \
        disabled:cursor-not-allowed min-w-[44px] min-h-[44px] flex items-center justify-center";
}

/// Checkbox component styles.
pub mod checkbox {

    pub const INPUT: &str = "w-4 h-4 rounded border-gray-300 dark:border-gray-600 text-blue-600 \
        focus:ring-blue-500 focus:ring-offset-1 dark:focus:ring-offset-gray-800 \
        disabled:cursor-not-allowed dark:bg-gray-800 min-w-[44px] min-h-[44px] \
        flex items-center justify-center cursor-pointer";

    pub const LABEL: &str =
        "text-sm text-gray-700 dark:text-gray-300 select-none cursor-pointer pt-0.5";
}

/// Select component styles.
pub mod select {

    pub const BASE: &str = "block w-full rounded-md border px-3 py-2 text-sm \
        focus:outline-none focus:ring-2 focus:ring-offset-1 dark:focus:ring-offset-gray-800 \
        disabled:bg-gray-100 dark:disabled:bg-gray-800 disabled:cursor-not-allowed min-h-[44px]";

    pub const DEFAULT: &str = "border-gray-300 dark:border-gray-600 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-800";
    pub const ERROR: &str =
        "border-red-300 dark:border-red-600 focus:ring-red-500 focus:border-red-500";
}

/// Skeleton loading component styles.
pub mod skeleton {

    pub const BASE: &str = "animate-pulse rounded bg-gray-200 dark:bg-gray-700";
    pub const TEXT: &str = "h-4 w-full";
    pub const HEADING: &str = "h-6 w-3/4";
    pub const AVATAR: &str = "h-10 w-10 rounded-full";
    pub const THUMBNAIL: &str = "h-20 w-20 rounded";
}
