use leptos::prelude::*;

/// Predefined icon variants following the lepticons pattern.
///
/// When lepticons is available as a dependency, prefer using its
/// `Icon` component with `lepticons::IconName` variants directly.
/// This enum mirrors the common subset for standalone usage.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum IconName {
    File,
    Folder,
    Upload,
    Download,
    Delete,
    Edit,
    Search,
    Settings,
    User,
    Lock,
    Unlock,
    Star,
    Clock,
    Check,
    X,
    Plus,
    Minus,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    ArrowDown,
}

fn icon_path(name: IconName) -> (&'static str, &'static str) {
    match name {
        IconName::File => (
            "none",
            "M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z",
        ),
        IconName::Folder => (
            "currentColor",
            "M2 6a2 2 0 012-2h5l2 2h5a2 2 0 012 2v6a2 2 0 01-2 2H4a2 2 0 01-2-2V6z",
        ),
        IconName::Upload => ("none", "M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12"),
        IconName::Download => ("none", "M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"),
        IconName::Delete => (
            "none",
            "M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16",
        ),
        IconName::Edit => (
            "none",
            "M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z",
        ),
        IconName::Search => ("none", "M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"),
        IconName::Settings => (
            "none",
            "M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z M15 12a3 3 0 11-6 0 3 3 0 016 0z",
        ),
        IconName::User => (
            "none",
            "M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z",
        ),
        IconName::Lock => (
            "none",
            "M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z",
        ),
        IconName::Unlock => (
            "none",
            "M8 11V7a4 4 0 118 0m-4 8v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2z",
        ),
        IconName::Star => (
            "none",
            "M11.049 2.927c.3-.921 1.603-.921 1.902 0l1.519 4.674a1 1 0 00.95.69h4.915c.969 0 1.371 1.24.588 1.81l-3.976 2.888a1 1 0 00-.363 1.118l1.518 4.674c.3.922-.755 1.688-1.538 1.118l-3.976-2.888a1 1 0 00-1.176 0l-3.976 2.888c-.783.57-1.838-.197-1.538-1.118l1.518-4.674a1 1 0 00-.363-1.118l-3.976-2.888c-.784-.57-.38-1.81.588-1.81h4.914a1 1 0 00.951-.69l1.519-4.674z",
        ),
        IconName::Clock => ("none", "M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"),
        IconName::Check => ("none", "M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"),
        IconName::X => ("none", "M6 18L18 6M6 6l12 12"),
        IconName::Plus => ("none", "M12 4v16m8-8H4"),
        IconName::Minus => ("none", "M20 12H4"),
        IconName::ArrowLeft => ("none", "M10 19l-7-7m0 0l7-7m-7 7h18"),
        IconName::ArrowRight => ("none", "M14 5l7 7m0 0l-7 7m7-7H3"),
        IconName::ArrowUp => ("none", "M5 10l7-7m0 0l7 7m-7-7v18"),
        IconName::ArrowDown => ("none", "M19 14l-7 7m0 0l-7-7m7 7V3"),
    }
}

/// Icon component that renders SVG icons.
///
/// Renders a 24x24 SVG icon from a predefined set.
/// All icons use `stroke` rendering (except Folder which uses fill).
/// Compatible with lepticons icon sizing (`w-5 h-5` default) and
/// supports the lepticons `decorative`/`label` accessibility pattern.
#[component]
pub fn Icon(
    /// Which icon to render.
    name: IconName,
    /// CSS class string for sizing/coloring.
    #[prop(default = "w-5 h-5".to_string())]
    class: String,
    /// Whether the icon is decorative (hidden from screen readers).
    #[prop(default = true)]
    decorative: bool,
    /// Accessible label for the icon (sets aria-label).
    #[prop(default = None)]
    label: Option<String>,
) -> impl IntoView {
    let (fill, d) = icon_path(name);
    let aria_hidden = if decorative { "true" } else { "false" };

    let svg = view! {
        <svg class=class aria-hidden=aria_hidden fill=fill stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d=d />
        </svg>
    };

    if let Some(label_text) = label {
        view! {
            <span class="inline-flex items-center" role="img" aria-label=label_text>
                {svg}
            </span>
        }
        .into_any()
    } else {
        svg.into_any()
    }
}

/// Helper function to create an IconName from a string.
pub fn icon_from_str(s: &str) -> IconName {
    match s {
        "file" => IconName::File,
        "folder" => IconName::Folder,
        "upload" => IconName::Upload,
        "download" => IconName::Download,
        "delete" => IconName::Delete,
        "edit" => IconName::Edit,
        "search" => IconName::Search,
        "settings" => IconName::Settings,
        "user" => IconName::User,
        "lock" => IconName::Lock,
        "unlock" => IconName::Unlock,
        "star" => IconName::Star,
        "clock" => IconName::Clock,
        "check" => IconName::Check,
        "x" => IconName::X,
        "plus" => IconName::Plus,
        "minus" => IconName::Minus,
        "arrow-left" => IconName::ArrowLeft,
        "arrow-right" => IconName::ArrowRight,
        "arrow-up" => IconName::ArrowUp,
        "arrow-down" => IconName::ArrowDown,
        _ => IconName::File,
    }
}
