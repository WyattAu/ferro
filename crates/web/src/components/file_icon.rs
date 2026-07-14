use leptos::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FileType {
    Folder,
    Image,
    Video,
    Audio,
    Pdf,
    Text,
    Code,
    Archive,
    Spreadsheet,
    Presentation,
    Generic,
}

pub fn file_type_from_extension(name: &str) -> FileType {
    let name_lower = name.to_lowercase();
    if let Some(ext) = name_lower.rsplit('.').next() {
        match ext {
            "jpg" | "jpeg" | "png" | "gif" | "bmp" | "svg" | "webp" | "ico" | "tiff" | "tif" | "avif" => {
                FileType::Image
            }
            "mp4" | "avi" | "mov" | "mkv" | "webm" | "flv" | "wmv" | "m4v" | "ogv" | "3gp" => FileType::Video,
            "mp3" | "wav" | "ogg" | "flac" | "aac" | "m4a" | "wma" | "opus" => FileType::Audio,
            "pdf" => FileType::Pdf,
            "txt" | "md" | "rtf" | "log" => FileType::Text,
            "rs" | "py" | "js" | "ts" | "tsx" | "jsx" | "go" | "java" | "c" | "cpp" | "h" | "hpp" | "cs" | "rb"
            | "php" | "swift" | "kt" | "scala" | "sh" | "bash" | "zsh" | "toml" | "yaml" | "yml" | "json" | "xml"
            | "html" | "css" | "scss" | "sass" | "sql" | "lua" | "r" | "dart" | "vue" | "svelte" => FileType::Code,
            "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" | "tgz" | "zst" => FileType::Archive,
            "xls" | "xlsx" | "csv" | "ods" => FileType::Spreadsheet,
            "ppt" | "pptx" | "odp" | "key" => FileType::Presentation,
            _ => FileType::Generic,
        }
    } else {
        FileType::Generic
    }
}

pub fn file_type_color(ft: FileType) -> &'static str {
    match ft {
        FileType::Folder => "text-[var(--warning)]",
        FileType::Image => "text-purple-500",
        FileType::Video => "text-[var(--danger)]",
        FileType::Audio => "text-[var(--success)]",
        FileType::Pdf => "text-[var(--warning)]",
        FileType::Text => "text-[var(--text-primary)] dark:text-[var(--text-tertiary)]",
        FileType::Code => "text-[var(--text-primary)] dark:text-[var(--text-tertiary)]",
        FileType::Archive => "text-amber-600",
        FileType::Spreadsheet => "text-emerald-500",
        FileType::Presentation => "text-orange-600",
        FileType::Generic => "text-[var(--text-tertiary)] dark:text-[var(--text-tertiary)]",
    }
}

#[component]
pub fn FileIcon(
    #[prop(default = FileType::Generic)] file_type: FileType,
    #[prop(default = 5)] size: u32,
    #[prop(default = false)] large: bool,
) -> impl IntoView {
    let color = file_type_color(file_type);
    let size_class = if large {
        "w-10 h-10".to_string()
    } else {
        format!("w-{} h-{}", size, size)
    };

    let svg_view = match file_type {
        FileType::Folder => view! {
            <svg class=size_class aria-hidden="true" fill="currentColor" viewBox="0 0 20 20">
                <path d="M2 6a2 2 0 012-2h5l2 2h5a2 2 0 012 2v6a2 2 0 01-2 2H4a2 2 0 01-2-2V6z" />
            </svg>
        }.into_any(),
        FileType::Image => view! {
            <svg class=size_class aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z" />
            </svg>
        }.into_any(),
        FileType::Video => view! {
            <svg class=size_class aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z" />
            </svg>
        }.into_any(),
        FileType::Audio => view! {
            <svg class=size_class aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 19V6l12-3v13M9 19c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zm12-3c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zM9 10l12-3" />
            </svg>
        }.into_any(),
        FileType::Pdf => view! {
            <svg class=size_class aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 21h10a2 2 0 002-2V9.414a1 1 0 00-.293-.707l-5.414-5.414A1 1 0 0012.586 3H7a2 2 0 00-2 2v14a2 2 0 002 2z" />
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M12 11h.01M11 15h2" />
            </svg>
        }.into_any(),
        FileType::Text => view! {
            <svg class=size_class aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
            </svg>
        }.into_any(),
        FileType::Code => view! {
            <svg class=size_class aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 20l4-16m4 4l4 4-4 4M6 16l-4-4 4-4" />
            </svg>
        }.into_any(),
        FileType::Archive => view! {
            <svg class=size_class aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 8h14M5 8a2 2 0 110-4h14a2 2 0 110 4M5 8v10a2 2 0 002 2h10a2 2 0 002-2V8m-9 4h4" />
            </svg>
        }.into_any(),
        FileType::Spreadsheet => view! {
            <svg class=size_class aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 10h18M3 14h18m-9-4v8m-7 0h14a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z" />
            </svg>
        }.into_any(),
        FileType::Presentation => view! {
            <svg class=size_class aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 12l3-3 3 3 4-4M8 21l4-4 4 4M3 4h18M4 4h16v12a1 1 0 01-1 1H5a1 1 0 01-1-1V4z" />
            </svg>
        }.into_any(),
        FileType::Generic => view! {
            <svg class=size_class aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
            </svg>
        }.into_any(),
    };

    view! {
        <span class={color}>{svg_view}</span>
    }
}
