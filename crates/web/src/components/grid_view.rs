use leptos::*;

use crate::api::{FileEntry, LockInfo};
use crate::components::file_icon::{FileIcon, FileType, file_type_from_extension};
use ferro_common::format::format_size;

#[component]
pub fn GridCard(
    entry: FileEntry,
    index: usize,
    on_navigate: Callback<String>,
    on_delete: Callback<String>,
    on_download: Callback<String>,
    on_share: Callback<String>,
    on_preview: Callback<String>,
    is_favorited: bool,
    on_toggle_favorite: Callback<String>,
    #[prop(default = false)] show_checkbox: bool,
    #[prop(default = false)] is_selected: bool,
    #[prop(default = Callback::new(move |_: (String, usize, bool, bool)| {}))]
    on_toggle_select: Callback<(String, usize, bool, bool)>,
    #[prop(default = Callback::new(move |_: String| {}))] on_copy: Callback<String>,
    #[prop(default = Callback::new(move |_: String| {}))] on_move: Callback<String>,
    #[prop(default = false)] is_locked: bool,
    #[prop(default = String::new())] lock_owner: String,
    #[prop(default = String::new())] lock_expires: String,
) -> impl IntoView {
    let file_type = if entry.is_collection {
        FileType::Folder
    } else {
        file_type_from_extension(&entry.name)
    };

    let size_str = if entry.is_collection {
        "--".to_string()
    } else {
        format_size(entry.size)
    };

    let modified_display = if entry.modified_at.len() >= 10 {
        entry.modified_at[..10].to_string()
    } else {
        entry.modified_at.clone()
    };

    let path_for_favorite = entry.path.clone();
    let path_for_select = entry.path.clone();
    let path_for_download = entry.path.clone();
    let path_for_share = entry.path.clone();
    let path_for_copy = entry.path.clone();
    let path_for_move = entry.path.clone();
    let path_for_delete = entry.path.clone();
    let path_for_thumbnail = entry.path.clone();
    let name_for_actions = entry.name.clone();
    let name_for_thumb = entry.name.clone();
    let entry_name = entry.name.clone();
    let entry_is_collection = entry.is_collection;
    let entry_index = index;
    let show_thumb = !entry_is_collection && file_type == FileType::Image;

    let path_for_click = entry.path.clone();
    let path_for_keydown = entry.path.clone();
    let path_for_preview_click = entry.path.clone();
    let path_for_preview_keydown = entry.path.clone();

    let handle_click = move |_: ev::MouseEvent| {
        if entry_is_collection {
            on_navigate.call(path_for_click.clone());
        } else {
            on_preview.call(path_for_preview_click.clone());
        }
    };

    let handle_keydown = move |ev: ev::KeyboardEvent| {
        if ev.key() == "Enter" || ev.key() == " " {
            ev.prevent_default();
            if entry_is_collection {
                on_navigate.call(path_for_keydown.clone());
            } else {
                on_preview.call(path_for_preview_keydown.clone());
            }
        }
    };

    let handle_context_menu = move |ev: ev::MouseEvent| {
        ev.prevent_default();
    };

    let handle_favorite_click = move |ev: ev::MouseEvent| {
        ev.stop_propagation();
        on_toggle_favorite.call(path_for_favorite.clone());
    };

    let handle_checkbox_click = move |ev: ev::MouseEvent| {
        ev.stop_propagation();
        let is_shift = ev.shift_key();
        let is_ctrl = ev.ctrl_key() || ev.meta_key();
        let p = path_for_select.clone();
        let idx = entry_index;
        on_toggle_select.call((p, idx, is_shift, is_ctrl));
    };

    let handle_download_click = move |ev: ev::MouseEvent| {
        ev.stop_propagation();
        on_download.call(path_for_download.clone());
    };

    let handle_share_click = move |ev: ev::MouseEvent| {
        ev.stop_propagation();
        on_share.call(path_for_share.clone());
    };

    let handle_copy_click = move |ev: ev::MouseEvent| {
        ev.stop_propagation();
        on_copy.call(path_for_copy.clone());
    };

    let handle_move_click = move |ev: ev::MouseEvent| {
        ev.stop_propagation();
        on_move.call(path_for_move.clone());
    };

    let handle_delete_click = move |ev: ev::MouseEvent| {
        ev.stop_propagation();
        on_delete.call(path_for_delete.clone());
    };

    let lock_tooltip = if is_locked && !lock_owner.is_empty() {
        format!("Locked by {} until {}", lock_owner, lock_expires)
    } else if is_locked {
        "Locked".to_string()
    } else {
        String::new()
    };

    view! {
        <div
            class=move || format!(
                "group relative surface brutal-border rounded-xl p-3 sm:p-4 cursor-pointer transition-all duration-200 hover:shadow-xl hover:border-blue-400 {}",
                if is_selected { "border-blue-500 dark:border-blue-400 ring-2 ring-200" } else { "" }
            )
            role="gridcell"
            tabindex="0"
            on:click=handle_click
            on:keydown=handle_keydown
            on:contextmenu=handle_context_menu
        >
            {show_checkbox.then(|| view! {
                <div class="absolute top-2 left-2 z-10">
                    <input
                        type="checkbox"
                        class="rounded border text-blue-600 focus:ring-blue-500 w-4 h-4"
                        prop:checked=is_selected
                        attr:aria-label=format!("Select {}", name_for_actions)
                        on:click=handle_checkbox_click
                    />
                </div>
            })}

            <button
                class="absolute top-2 right-2 z-10 min-w-[44px] min-h-[44px] flex items-center justify-center rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500"
                class=move || if is_favorited { "text-yellow-500 hover:text-yellow-600 hover:bg-yellow-50" } else { "text-gray-300 hover:text-yellow-500 hover:bg-yellow-50 opacity-0 group-hover:opacity-100" }
                attr:aria-label=format!("{} {}", if is_favorited { "Unfavorite" } else { "Favorite" }, name_for_actions)
                title=if is_favorited { "Remove from favorites" } else { "Add to favorites" }
                on:click=handle_favorite_click
            >
                <svg class="w-4 h-4" aria-hidden="true" fill=move || if is_favorited { "currentColor" } else { "none" } stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11.049 2.927c.3-.921 1.603-.921 1.902 0l1.519 4.674a1 1 0 00.95.69h4.915c.969 0 1.371 1.24.588 1.81l-3.976 2.888a1 1 0 00-.363 1.118l1.518 4.674c.3.922-.755 1.688-1.538 1.118l-3.976-2.888a1 1 0 00-1.176 0l-3.976 2.888c-.783.57-1.838-.197-1.538-1.118l1.518-4.674a1 1 0 00-.363-1.118l-3.976-2.888c-.784-.57-.38-1.81.588-1.81h4.914a1 1 0 00.951-.69l1.519-4.674z" />
                </svg>
            </button>

            <div class="flex flex-col items-center text-center pt-2 sm:pt-4 pb-2">
                <div class="relative mb-2 sm:mb-3">
                    <Show when=move || show_thumb fallback=move || view! { <FileIcon file_type=file_type large=true /> }>
                        <img
                            class="w-10 h-10 rounded object-cover"
                            src=format!("/api/thumbnail{}", path_for_thumbnail)
                            alt=name_for_thumb.clone()
                            loading="lazy"
                        />
                    </Show>
                    {is_locked.then(|| view! {
                        <span class="absolute -bottom-1 -right-1">
                            <svg class="w-4 h-4 text-red-500" fill="none" stroke="currentColor" viewBox="0 0 24 24" title=lock_tooltip.clone()>
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z" />
                            </svg>
                        </span>
                    })}
                </div>

                <div class="w-full min-h-[2.5rem] flex items-center justify-center">
                    <span class="text-xs sm:text-sm font-medium text-gray-900 truncate max-w-full px-1"
                        class=move || if entry_is_collection { "font-semibold" } else { "font-medium" }
                        title=entry_name.clone()
                    >
                        {&entry_name}
                    </span>
                </div>

                <span class="text-[10px] sm:text-xs text-gray-500">{&size_str}</span>
                <span class="text-[10px] text-gray-400 hidden sm:block">{&modified_display}</span>
            </div>

            <div class="flex items-center justify-center gap-1 pt-2 border-t border-gray-100 opacity-0 group-hover:opacity-100 transition-opacity">
                {(!entry_is_collection && !is_locked).then(|| view! {
                    <button
                        class="min-w-[44px] min-h-[44px] flex items-center justify-center text-gray-400 hover:text-blue-600 hover:bg-blue-50 rounded shadow-concrete transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500"
                        attr:aria-label=format!("Download {}", name_for_actions)
                        title="Download"
                        on:click=handle_download_click
                    >
                        <svg class="w-3.5 h-3.5" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" />
                        </svg>
                    </button>
                })}
                {(!entry_is_collection && !is_locked).then(|| view! {
                    <button
                        class="min-w-[44px] min-h-[44px] flex items-center justify-center text-gray-400 hover:text-green-600 hover:bg-green-50 rounded shadow-concrete transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500"
                        attr:aria-label=format!("Share {}", name_for_actions)
                        title="Share"
                        on:click=handle_share_click
                    >
                        <svg class="w-3.5 h-3.5" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8.684 13.342C8.886 12.938 9 12.482 9 12c0-.482-.114-.938-.316-1.342m0 2.684a3 3 0 110-2.684m0 2.684l6.632 3.316m-6.632-6l6.632-3.316m0 0a3 3 0 105.367-2.684 3 3 0 00-5.367 2.684zm0 9.316a3 3 0 105.368 2.684 3 3 0 00-5.368-2.684z" />
                        </svg>
                    </button>
                })}
                <button
                    class="min-w-[44px] min-h-[44px] flex items-center justify-center text-gray-400 hover:text-orange-600 hover:bg-orange-50 rounded shadow-concrete transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500"
                    attr:aria-label=format!("Copy {}", name_for_actions)
                    title="Copy"
                    on:click=handle_copy_click
                >
                    <svg class="w-3.5 h-3.5" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
                    </svg>
                </button>
                <button
                    class="min-w-[44px] min-h-[44px] flex items-center justify-center text-gray-400 hover:text-purple-600 hover:bg-purple-50 rounded shadow-concrete transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500"
                    attr:aria-label=format!("Move {}", name_for_actions)
                    title="Move"
                    on:click=handle_move_click
                >
                    <svg class="w-3.5 h-3.5" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 5l7 7-7 7M5 5l7 7-7 7" />
                    </svg>
                </button>
                {(!is_locked).then(|| view! {
                    <button
                        class="min-w-[44px] min-h-[44px] flex items-center justify-center text-gray-400 hover:text-red-600 hover:bg-red-50 rounded shadow-concrete transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500"
                        attr:aria-label=format!("Delete {}", name_for_actions)
                        title="Delete"
                        on:click=handle_delete_click
                    >
                        <svg class="w-3.5 h-3.5" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                        </svg>
                    </button>
                })}
            </div>
        </div>
    }
}

#[component]
pub fn GridView(
    entries: ReadSignal<Vec<FileEntry>>,
    on_navigate: Callback<String>,
    on_delete: Callback<String>,
    on_download: Callback<String>,
    on_share: Callback<String>,
    on_preview: Callback<String>,
    favorites: ReadSignal<Vec<String>>,
    on_toggle_favorite: Callback<String>,
    show_checkbox: bool,
    selected_paths: ReadSignal<std::collections::HashSet<String>>,
    on_toggle_select: Callback<(String, usize, bool, bool)>,
    on_copy: Callback<String>,
    on_move: Callback<String>,
    locks: ReadSignal<std::collections::HashMap<String, LockInfo>>,
) -> impl IntoView {
    let entries_for_each = entries;
    let entries_for_idx = entries;
    let favs = favorites;
    let sels = selected_paths;
    let locks_sig = locks;

    view! {
        <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 xl:grid-cols-5 gap-2 sm:gap-3 p-3 sm:p-4" role="grid" aria-label="File grid">
            <For
                each=move || entries_for_each.get()
                key=|entry| entry.path.clone()
                let:entry
            >
                {
                    let ep = entry.path.clone();
                    let ep2 = entry.path.clone();
                    let entry_idx = {
                        let ents = entries_for_idx.get();
                        let ep3 = entry.path.clone();
                        ents.iter().position(|e| e.path == ep3).unwrap_or(0)
                    };
                    let is_fav = move || favs.with(|f| f.contains(&ep));
                    let ep_lock = ep2.clone();
                    let is_sel = move || sels.with(|s| s.contains(&ep2));
                    let li = move || {
                        let locks_map = locks_sig.get();
                        if let Some(lock) = locks_map.get(&ep_lock) {
                            (true, lock.owner.clone(), lock.expires_at.clone())
                        } else {
                            let mut check = ep_lock.as_str();
                            while check.len() > 1 {
                                check = match check.rfind('/') {
                                    None => break,
                                    Some(0) => break,
                                    Some(i) => &check[..i],
                                };
                                if let Some(lock) = locks_map.get(check)
                                    && lock.depth == "Infinity" {
                                        return (true, lock.owner.clone(), lock.expires_at.clone());
                                    }
                            }
                            (false, String::new(), String::new())
                        }
                    };
                    view! {
                        <GridCard
                            entry=entry
                            index=entry_idx
                            on_navigate=on_navigate
                            on_delete=on_delete
                            on_download=on_download
                            on_share=on_share
                            on_preview=on_preview
                            is_favorited=is_fav()
                            on_toggle_favorite=on_toggle_favorite
                            show_checkbox=show_checkbox
                            is_selected=is_sel()
                            on_toggle_select=on_toggle_select
                            on_copy=on_copy
                            on_move=on_move
                            is_locked=li().0
                            lock_owner=li().1
                            lock_expires=li().2
                        />
                    }
                }
            </For>
        </div>
    }
}
