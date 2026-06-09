use leptos::prelude::*;
use leptos::ev;

use crate::api::FileEntry;
use crate::t;
use ferro_common::format::format_size;

#[component]
pub fn FileRow(
    entry: FileEntry,
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
    #[prop(default = Callback::new(move |_: String| {}))] on_rename: Callback<String>,
    #[prop(default = Callback::new(move |_: (String, bool)| {}))] on_drop_on_folder: Callback<(String, bool)>,
    #[prop(default = false)] is_locked: bool,
    #[prop(default = String::new())] lock_owner: String,
    #[prop(default = String::new())] lock_expires: String,
) -> impl IntoView {
    let folder_icon = view! {
        <svg class="w-5 h-5 text-yellow-500" aria-hidden="true" fill="currentColor" viewBox="0 0 20 20">
            <path d="M2 6a2 2 0 012-2h5l2 2h5a2 2 0 012 2v6a2 2 0 01-2 2H4a2 2 0 01-2-2V6z" />
        </svg>
    };
    let file_icon = view! {
        <svg class="w-5 h-5 text-gray-400" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
        </svg>
    };

    let size = if entry.is_collection {
        "--".to_string()
    } else {
        format_size(entry.size)
    };

    let path_for_delete = entry.path.clone();
    let path_for_download = entry.path.clone();
    let path_for_share = entry.path.clone();
    let path_for_preview = entry.path.clone();
    let path_for_favorite = entry.path.clone();
    let path_for_select = entry.path.clone();
    let path_for_copy = entry.path.clone();
    let path_for_move = entry.path.clone();
    let path_for_rename = entry.path.clone();
    let drag_path_row = entry.path.clone();
    let folder_drop_path_row = entry.path.clone();
    let path_for_click = entry.path.clone();
    let name_for_download = entry.name.clone();
    let name_for_share = entry.name.clone();
    let name_for_delete = entry.name.clone();
    let name_for_favorite = entry.name.clone();
    let name_for_copy = entry.name.clone();
    let name_for_move = entry.name.clone();

    let handle_click = move |_: ev::MouseEvent| {
        if entry.is_collection {
            on_navigate.run(path_for_click.clone());
        } else {
            on_preview.run(path_for_preview.clone());
        }
    };

    let path_for_select_cb = path_for_select.clone();
    let on_toggle_select_cb = on_toggle_select;
    let handle_checkbox_click_desktop = move |ev: ev::MouseEvent| {
        ev.stop_propagation();
        let is_shift = ev.shift_key();
        let is_ctrl = ev.ctrl_key() || ev.meta_key();
        let p = path_for_select.clone();
        on_toggle_select.run((p, 0, is_shift, is_ctrl));
    };
    let handle_checkbox_click_mobile = move |ev: ev::MouseEvent| {
        ev.stop_propagation();
        let is_shift = ev.shift_key();
        let is_ctrl = ev.ctrl_key() || ev.meta_key();
        let p = path_for_select_cb.clone();
        on_toggle_select_cb.run((p, 0, is_shift, is_ctrl));
    };

    let entry_name = entry.name.clone();
    let entry_modified = entry.modified_at.clone();
    let entry_size = size.clone();
    let entry_is_collection = entry.is_collection;

    // Drag-and-drop handlers
    let (folder_hovering, set_folder_hovering) = create_signal(false);

    let handle_drag_start = move |ev: ev::DragEvent| {
        ev.stop_propagation();
        if let Some(data_transfer) = ev.data_transfer() {
            let _ = data_transfer.set_data("text/plain", &drag_path_row);
            let _ = data_transfer.set_data(
                "application/x-ferro-file",
                &serde_json::json!({
                    "path": drag_path_row,
                    "is_collection": entry_is_collection,
                }).to_string(),
            );
            data_transfer.set_drop_effect("move");
        }
    };

    let handle_folder_drag_over = move |ev: ev::DragEvent| {
        if !entry_is_collection || is_locked {
            return;
        }
        ev.prevent_default();
        ev.stop_propagation();
        if let Some(dt) = ev.data_transfer() {
            dt.set_drop_effect(if ev.ctrl_key() { "copy" } else { "move" });
        }
        set_folder_hovering.set(true);
    };

    let handle_folder_drag_leave = move |ev: ev::DragEvent| {
        ev.stop_propagation();
        set_folder_hovering.set(false);
    };

    let handle_folder_drop = move |ev: ev::DragEvent| {
        ev.prevent_default();
        ev.stop_propagation();
        set_folder_hovering.set(false);
        if !entry_is_collection || is_locked {
            return;
        }
        let is_copy = ev.ctrl_key();
        if let Some(dt) = ev.data_transfer()
            && let Ok(source) = dt.get_data("text/plain")
            && !source.is_empty() && source != folder_drop_path_row
        {
            on_drop_on_folder.run((source, is_copy));
        }
    };

    let handle_drag_end = move |_: ev::DragEvent| {
        set_folder_hovering.set(false);
    };

    let lock_tooltip = if is_locked && !lock_owner.is_empty() {
        format!("Locked by {} until {}", lock_owner, lock_expires)
    } else if is_locked {
        t!("common.locked").to_string()
    } else {
        String::new()
    };

    view! {
        <tr
            class=move || format!(
                "border-b border-gray-100 md:group cursor-pointer transition-colors {} {} block md:table-row mb-2 md:mb-0 px-3 py-2 md:px-0 md:py-0 rounded md:rounded-none mx-2 md:mx-0 md:border-0 md:first:border-t-0 {}",
                if is_selected { "bg-blue-50 dark:bg-blue-900/20" } else { "hover:bg-gray-50 md:hover:bg-gray-50" },
                if show_checkbox { "select-none" } else { "" },
                if entry_is_collection && folder_hovering.get() { "ring-2 ring-blue-400 border-blue-400 bg-blue-50 dark:bg-blue-900/30" } else { "" }
            )
            role="row"
            draggable=move || !entry_is_collection && !is_locked
            on:dragstart=move |ev| {
                if !entry_is_collection && !is_locked {
                    handle_drag_start(ev);
                }
            }
            on:dragover=move |ev| {
                if entry_is_collection {
                    handle_folder_drag_over(ev);
                }
            }
            on:dragleave=move |ev| {
                if entry_is_collection {
                    handle_folder_drag_leave(ev);
                }
            }
            on:drop=move |ev| {
                if entry_is_collection {
                    handle_folder_drop(ev);
                }
            }
            on:dragend=handle_drag_end
            on:click=handle_click
        >
            <td class="hidden md:table-cell px-4 py-2.5" role="gridcell" hidden=move || !show_checkbox>
                <input
                    type="checkbox"
                    class="rounded border text-blue-600 focus:ring-blue-500"
                    prop:checked=is_selected
                    attr:aria-label=format!("Select {}", name_for_delete)
                    on:click=handle_checkbox_click_desktop
                />
            </td>
            <td class="hidden md:table-cell px-4 py-2.5" role="gridcell">
                <div class="flex items-center gap-1">
                    {if entry.is_collection { folder_icon.into_any() } else { file_icon.into_any() }}
                    {is_locked.then(|| view! {
                        <span class="text-xs" title=lock_tooltip.clone()>
                            <svg class="w-4 h-4 text-red-500" fill="none" stroke="currentColor" viewBox="0 0 24 24" title=lock_tooltip.clone()>
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z" />
                            </svg>
                        </span>
                    })}
                </div>
            </td>
            <td class="px-0 py-0 md:px-4 md:py-2.5" role="rowheader">
                <div class="flex md:table-cell items-center gap-3 px-1 py-2 md:px-0 md:py-0 min-h-[44px]">
                    <span class="md:hidden shrink-0 flex items-center gap-1">
                        {if entry.is_collection {
                            view! {
                                <svg class="w-5 h-5 text-yellow-500" aria-hidden="true" fill="currentColor" viewBox="0 0 20 20">
                                    <path d="M2 6a2 2 0 012-2h5l2 2h5a2 2 0 012 2v6a2 2 0 01-2 2H4a2 2 0 01-2-2V6z" />
                                </svg>
                            }.into_any()
                        } else {
                            view! {
                                <svg class="w-5 h-5 text-gray-400" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                                </svg>
                            }.into_any()
                        }}
                        {is_locked.then(|| view! {
                            <svg class="w-4 h-4 text-red-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z" />
                            </svg>
                        })}
                    </span>
                    <span class={if entry_is_collection { "font-semibold font-mono text-gray-900 truncate" } else { "text-gray-700 truncate" }}>
                        {entry_name.clone()}
                    </span>
                    {is_locked.then(|| view! {
                        <span class="text-xs text-red-500 font-medium">{t!("common.locked")}</span>
                    })}
                </div>
            </td>
            <td class="px-1 py-0 md:px-4 md:py-2.5 text-gray-500 text-sm font-mono tabular-nums md:table-cell block" role="gridcell">
                <span class="md:hidden text-xs">{entry_size.clone()}</span>
                <span class="hidden md:inline">{entry_size.clone()}</span>
            </td>
            <td class="px-1 py-0 md:px-4 md:py-2.5 text-gray-500 text-sm font-mono tabular-nums hidden lg:table-cell" role="gridcell">{entry_modified.clone()}</td>
            <td class="px-1 py-1 md:px-4 md:py-2.5 text-right md:table-cell block" role="gridcell">
                <div class="flex items-center justify-end gap-1 opacity-100 md:opacity-0 md:group-hover:opacity-100 transition-opacity">
                    {show_checkbox.then(|| view! {
                        <div class="md:hidden mr-1">
                            <input
                                type="checkbox"
                                class="rounded border text-blue-600 focus:ring-blue-500 min-w-[44px] min-h-[44px]"
                                prop:checked=is_selected
                                attr:aria-label=format!("Select {}", name_for_delete)
                                on:click=handle_checkbox_click_mobile
                            />
                        </div>
                    })}
                    <button
                        class=move || {
                            let base = "p-2 md:p-1.5 rounded shadow-concrete transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 min-w-[44px] min-h-[44px] md:min-w-0 md:min-h-0 flex items-center justify-center";
                            let color = if is_favorited { "text-yellow-500 hover:text-yellow-600 hover:bg-yellow-50" } else { "text-gray-300 hover:text-yellow-500 hover:bg-yellow-50" };
                            format!("{} {}", base, color)
                        }
                        attr:aria-label=format!("{} {}", if is_favorited { t!("fav.unfavorite") } else { t!("fav.favorite") }, name_for_favorite)
                        title=if is_favorited { t!("fav.remove") } else { t!("fav.add") }
                        on:click=move |ev| {
                            ev.stop_propagation();
                            on_toggle_favorite.run(path_for_favorite.clone());
                        }
                    >
                        <svg class="w-4 h-4" aria-hidden="true" fill=move || if is_favorited { "currentColor" } else { "none" } stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11.049 2.927c.3-.921 1.603-.921 1.902 0l1.519 4.674a1 1 0 00.95.69h4.915c.969 0 1.371 1.24.588 1.81l-3.976 2.888a1 1 0 00-.363 1.118l1.518 4.674c.3.922-.755 1.688-1.538 1.118l-3.976-2.888a1 1 0 00-1.176 0l-3.976 2.888c-.783.57-1.838-.197-1.538-1.118l1.518-4.674a1 1 0 00-.363-1.118l-3.976-2.888c-.784-.57-.38-1.81.588-1.81h4.914a1 1 0 00.951-.69l1.519-4.674z" />
                        </svg>
                    </button>
                    {(!entry.is_collection && !is_locked).then(|| view! {
                        <button
                            class="p-2 md:p-1.5 text-gray-400 hover:text-blue-600 hover:bg-blue-50 rounded shadow-concrete transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 min-w-[44px] min-h-[44px] md:min-w-0 md:min-h-0 flex items-center justify-center"
                            attr:aria-label=format!("Download {}", name_for_download)
                            title=t!("common.download")
                            on:click=move |ev| {
                                ev.stop_propagation();
                                on_download.run(path_for_download.clone());
                            }
                        >
                            <svg class="w-4 h-4" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" />
                            </svg>
                        </button>
                    })}
                    {(!entry.is_collection && !is_locked).then(|| view! {
                        <button
                            class="p-2 md:p-1.5 text-gray-400 hover:text-green-600 hover:bg-green-50 rounded shadow-concrete transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 min-w-[44px] min-h-[44px] md:min-w-0 md:min-h-0 flex items-center justify-center"
                            attr:aria-label=format!("Share {}", name_for_share)
                            title=t!("common.share")
                            on:click=move |ev| {
                                ev.stop_propagation();
                                on_share.run(path_for_share.clone());
                            }
                        >
                            <svg class="w-4 h-4" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8.684 13.342C8.886 12.938 9 12.482 9 12c0-.482-.114-.938-.316-1.342m0 2.684a3 3 0 110-2.684m0 2.684l6.632 3.316m-6.632-6l6.632-3.316m0 0a3 3 0 105.367-2.684 3 3 0 00-5.367 2.684zm0 9.316a3 3 0 105.368 2.684 3 3 0 00-5.368-2.684z" />
                            </svg>
                        </button>
                    })}
                    <button
                        class="p-2 md:p-1.5 text-gray-400 hover:text-orange-600 hover:bg-orange-50 rounded shadow-concrete transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 min-w-[44px] min-h-[44px] md:min-w-0 md:min-h-0 flex items-center justify-center"
                        attr:aria-label=format!("Copy {}", name_for_copy)
                        title=t!("common.copy")
                        on:click=move |ev| {
                            ev.stop_propagation();
                            on_copy.run(path_for_copy.clone());
                        }
                    >
                        <svg class="w-4 h-4" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
                        </svg>
                    </button>
                    <button
                        class="p-2 md:p-1.5 text-gray-400 hover:text-purple-600 hover:bg-purple-50 rounded shadow-concrete transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 min-w-[44px] min-h-[44px] md:min-w-0 md:min-h-0 flex items-center justify-center"
                        attr:aria-label=format!("Move {}", name_for_move)
                        title=t!("common.move")
                        on:click=move |ev| {
                            ev.stop_propagation();
                            on_move.run(path_for_move.clone());
                        }
                    >
                        <svg class="w-4 h-4" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 5l7 7-7 7M5 5l7 7-7 7" />
                        </svg>
                    </button>
                    {(!entry.is_collection && !is_locked).then(|| view! {
                        <button
                            class="p-2 md:p-1.5 text-gray-400 hover:text-cyan-600 hover:bg-cyan-50 rounded shadow-concrete transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 min-w-[44px] min-h-[44px] md:min-w-0 md:min-h-0 flex items-center justify-center"
                            attr:aria-label=format!("Rename {}", entry.name)
                            title=t!("common.rename")
                            on:click=move |ev| {
                                ev.stop_propagation();
                                on_rename.run(path_for_rename.clone());
                            }
                        >
                            <svg class="w-4 h-4" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
                            </svg>
                        </button>
                    })}
                    {is_locked.then(|| view! {
                        <span class="text-xs text-red-500 font-medium px-2">{t!("common.locked")}</span>
                    })}
                    {(!is_locked).then(|| view! {
                        <button
                            class="p-2 md:p-1.5 text-gray-400 hover:text-red-600 hover:bg-red-50 rounded shadow-concrete transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 min-w-[44px] min-h-[44px] md:min-w-0 md:min-h-0 flex items-center justify-center"
                            attr:aria-label=format!("Delete {}", name_for_delete)
                            title=t!("common.delete")
                            on:click=move |ev| {
                                ev.stop_propagation();
                                on_delete.run(path_for_delete.clone());
                            }
                        >
                            <svg class="w-4 h-4" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                            </svg>
                        </button>
                    })}
                </div>
            </td>
        </tr>
    }
}
