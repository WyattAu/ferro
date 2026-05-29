use leptos::*;
use leptos_router::A;

use crate::api;
use crate::components::clipboard::{ClipboardAction, use_clipboard_state};
use crate::components::command_palette::{Command, use_command_palette_state};
use crate::components::file_preview::FilePreview;
use crate::components::file_row::FileRow;
use crate::components::grid_view::GridView;
use crate::components::header::use_header_state;
use crate::components::skeleton::{SkeletonFavorites, SkeletonGrid, SkeletonList, SkeletonRecent};
use crate::components::theme_toggle::{Theme, use_theme_state};
use crate::components::toast::ToastContext;

#[derive(Clone, Copy, PartialEq, Eq)]
enum BrowserTab {
    Files,
    Favorites,
    Recent,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ViewMode {
    List,
    Grid,
}

impl ViewMode {
    fn as_str(&self) -> &'static str {
        match self {
            ViewMode::List => "list",
            ViewMode::Grid => "grid",
        }
    }

    fn from_str(s: &str) -> Self {
        match s {
            "grid" => ViewMode::Grid,
            _ => ViewMode::List,
        }
    }
}

#[component]
pub fn FileBrowser(initial_path: String) -> impl IntoView {
    let initial = initial_path.clone();
    let (current_path, set_current_path) = create_signal(initial_path);
    let (all_entries, set_all_entries) = create_signal(vec![]);
    let (display_count, set_display_count) = create_signal(50usize);
    let (loading, set_loading) = create_signal(false);
    let (error, set_error) = create_signal(None::<String>);
    let (show_new_folder, set_show_new_folder) = create_signal(false);
    let (new_folder_name, set_new_folder_name) = create_signal(String::new());
    let (show_upload, set_show_upload) = create_signal(false);
    let (upload_drag, set_upload_drag) = create_signal(false);
    let (show_share_dialog, set_show_share_dialog) = create_signal(false);
    let (share_path, set_share_path) = create_signal(String::new());
    let (share_password, set_share_password) = create_signal(String::new());
    let (share_expires, set_share_expires) = create_signal(String::from("168"));
    let (share_url, set_share_url) = create_signal(String::new());
    let (share_creating, set_share_creating) = create_signal(false);
    let (share_error, set_share_error) = create_signal(String::new());
    let (share_copied, set_share_copied) = create_signal(false);
    let (preview_file, set_preview_file) = create_signal(None::<api::FileEntry>);
    let (active_tab, set_active_tab) = create_signal(BrowserTab::Files);
    let (favorites, set_favorites) = create_signal::<Vec<String>>(vec![]);
    let (recent_files, set_recent_files) = create_signal::<Vec<api::FileEntry>>(vec![]);
    let (favorites_loading, set_favorites_loading) = create_signal(false);
    let (recent_loading, set_recent_loading) = create_signal(false);

    let (selected_paths, set_selected_paths) =
        create_signal(std::collections::HashSet::<String>::new());
    let selected_paths_signal = selected_paths;
    let favorites_signal = favorites;
    let (select_mode, set_select_mode) = create_signal(false);
    let (last_clicked_index, set_last_clicked_index) = create_signal(None::<usize>);

    let (show_move_dialog, set_show_move_dialog) = create_signal(false);
    let (move_source, set_move_source) = create_signal(String::new());
    let (move_dest, set_move_dest) = create_signal(String::new());
    let (show_copy_dialog, set_show_copy_dialog) = create_signal(false);
    let (copy_source, set_copy_source) = create_signal(String::new());
    let (copy_dest, set_copy_dest) = create_signal(String::new());
    let (show_activity, set_show_activity) = create_signal(false);
    let (activity_entries, set_activity_entries) = create_signal::<Vec<api::ActivityEntry>>(vec![]);
    let (show_delete_confirm, set_show_delete_confirm) = create_signal(false);

    let (view_mode, set_view_mode) = create_signal(ViewMode::List);

    let clipboard_state = use_clipboard_state();
    let palette_state = use_command_palette_state();
    let theme_state = use_theme_state();
    let header_state = use_header_state();

    create_effect(move |_| {
        spawn_local(async move {
            if let Ok(prefs) = api::get_preferences().await {
                set_view_mode.set(ViewMode::from_str(&prefs.view_mode));
            }
        });
    });

    let toggle_view_mode = move |_: ev::MouseEvent| {
        let current = view_mode.get();
        let next = match current {
            ViewMode::List => ViewMode::Grid,
            ViewMode::Grid => ViewMode::List,
        };
        set_view_mode.set(next);
        let mode_str = next.as_str().to_string();
        spawn_local(async move {
            if let Ok(current_prefs) = api::get_preferences().await {
                let mut prefs = current_prefs;
                prefs.view_mode = mode_str;
                let _ = api::update_preferences(&prefs).await;
            }
        });
    };

    let load_favorites = move || {
        set_favorites_loading.set(true);
        spawn_local(async move {
            match api::list_favorites().await {
                Ok(paths) => set_favorites.set(paths),
                Err(_) => set_favorites.set(vec![]),
            }
            set_favorites_loading.set(false);
        });
    };

    let load_recent = move || {
        set_recent_loading.set(true);
        spawn_local(async move {
            match api::list_recent_files().await {
                Ok(files) => set_recent_files.set(files),
                Err(_) => set_recent_files.set(vec![]),
            }
            set_recent_loading.set(false);
        });
    };

    let do_toggle_favorite = move |path: String| {
        let is_fav = favorites.with(|f| f.contains(&path));
        let fav_path = path.clone();
        spawn_local(async move {
            if is_fav {
                let _ = api::remove_favorite(&fav_path).await;
                ToastContext::info("Removed from favorites");
            } else {
                let _ = api::add_favorite(&fav_path).await;
                ToastContext::info("Added to favorites");
            }
            if let Ok(paths) = api::list_favorites().await {
                set_favorites.set(paths);
            }
        });
    };

    create_effect(move |_| {
        load_favorites();
        load_recent();
    });

    let display_entries = move || {
        let entries = all_entries.get();
        let count = display_count.get();
        if entries.len() > count {
            entries[..count].to_vec()
        } else {
            entries
        }
    };

    // Infinite scroll via IntersectionObserver on the sentinel div.
    // We cannot use on:scroll because Leptos 0.6 uses event delegation
    // and scroll events do not bubble, so the delegated listener never fires.
    // We set root to the scroll container so intersection is computed
    // relative to the scrollable area, not the viewport.
    let scroll_sentinel_ref = create_node_ref::<html::Div>();
    let scroll_container_ref = create_node_ref::<html::Div>();
    {
        create_effect(move |_| {
            let sentinel = scroll_sentinel_ref.get()?;
            use wasm_bindgen::JsCast;
            let callback = {
                let all_entries = all_entries;
                let display_count = display_count;
                let set_display_count = set_display_count;
                wasm_bindgen::closure::Closure::wrap(Box::new(
                    move |entries: js_sys::Array, _: web_sys::IntersectionObserver| {
                        for i in 0..entries.length() {
                            if let Ok(entry) = entries
                                .get(i)
                                .dyn_into::<web_sys::IntersectionObserverEntry>()
                                && entry.is_intersecting()
                            {
                                let total = all_entries.with(Vec::len);
                                let displayed = display_count.get();
                                if displayed < total {
                                    set_display_count.set(displayed + 50);
                                }
                            }
                        }
                    },
                )
                    as Box<dyn Fn(js_sys::Array, web_sys::IntersectionObserver)>)
            };
            let callback_fn: &js_sys::Function = callback.as_ref().unchecked_ref();

            // Build options with root set to the scroll container.
            // Falls back to viewport (null root) if container not found.
            let options = scroll_container_ref
                .get()
                .map(|el| {
                    let opts = web_sys::IntersectionObserverInit::new();
                    opts.set_root(Some(&el));
                    opts
                })
                .unwrap_or_default();

            let observer =
                web_sys::IntersectionObserver::new_with_options(callback_fn, &options).unwrap();
            observer.observe(&sentinel);
            // Keep the closure alive for the lifetime of the effect
            callback.forget();
            Some(())
        });
    }

    let reload = move || {
        let p = current_path.get();
        spawn_local(async move {
            if let Ok(response) = api::list_files(&p).await {
                set_all_entries.set(response.entries);
            }
        });
    };

    let load_directory = move |path: String| {
        set_loading.set(true);
        set_error.set(None);
        set_current_path.set(path.clone());
        set_selected_paths.set(std::collections::HashSet::new());
        let p = path.clone();
        spawn_local(async move {
            match api::list_files(&p).await {
                Ok(response) => {
                    set_all_entries.set(response.entries);
                    set_display_count.set(50);
                    set_loading.set(false);
                }
                Err(e) => {
                    set_error.set(Some(e));
                    set_loading.set(false);
                }
            }
        });
    };

    let _initial = initial;
    create_effect(move |_| {
        load_directory(_initial.clone());
    });

    let navigate = move |path: String| {
        load_directory(path.clone());
        // Update browser URL to reflect current path
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(window) = web_sys::window() {
                let url = if path == "/" {
                    "/ui/".to_string()
                } else {
                    format!("/ui/files{}", path)
                };
                if let Ok(history) = window.history() {
                    let _ =
                        history.push_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(&url));
                }
            }
        }
    };

    let go_up = move |_: ev::MouseEvent| {
        let path = current_path.get();
        if path != "/" {
            let parent = path
                .rfind('/')
                .map(|i| {
                    if i == 0 {
                        "/".to_string()
                    } else {
                        path[..i].to_string()
                    }
                })
                .unwrap_or("/".to_string());
            load_directory(parent);
        }
    };

    let do_create_folder = move |_: ev::MouseEvent| {
        let name = new_folder_name.get();
        if name.is_empty() {
            return;
        }
        let path = current_path.get();
        let folder_path = if path == "/" {
            format!("/{}", name)
        } else {
            format!("{}/{}", path, name)
        };
        let fp = folder_path.clone();
        spawn_local(async move {
            match api::create_directory(&fp).await {
                Ok(()) => {
                    set_show_new_folder.set(false);
                    set_new_folder_name.set(String::new());
                    ToastContext::success("Folder created");
                    reload();
                }
                Err(e) => {
                    set_error.set(Some(format!("Create folder failed: {}", e)));
                    ToastContext::error(format!("Failed to create folder: {}", e));
                }
            }
        });
    };

    let do_delete = move |path: String| {
        spawn_local(async move {
            match api::delete_file(&path).await {
                Ok(()) => {
                    ToastContext::success("File deleted");
                    reload();
                }
                Err(e) => {
                    set_error.set(Some(format!("Delete failed: {}", e)));
                    ToastContext::error(format!("Delete failed: {}", e));
                }
            }
        });
    };

    let do_download = move |path: String| {
        spawn_local(async move {
            let _ = api::download_file(&path).await;
        });
    };

    let do_share = move |path: String| {
        set_share_path.set(path);
        set_share_password.set(String::new());
        set_share_expires.set(String::from("168"));
        set_share_url.set(String::new());
        set_share_error.set(String::new());
        set_share_copied.set(false);
        set_show_share_dialog.set(true);
    };

    let do_create_share = move |_: ev::MouseEvent| {
        let path = share_path.get();
        let password = share_password.get();
        let expires_str = share_expires.get();
        let pw = if password.is_empty() {
            None
        } else {
            Some(password)
        };
        let expires: u32 = expires_str.parse().unwrap_or(168);
        set_share_creating.set(true);
        set_share_error.set(String::new());
        spawn_local(async move {
            let pw_ref = pw.as_deref();
            match api::create_share(&path, pw_ref, Some(expires)).await {
                Ok(resp) => {
                    set_share_url.set(resp.url);
                    set_share_creating.set(false);
                    ToastContext::success("Share link created");
                }
                Err(e) => {
                    let err_msg = e.clone();
                    set_share_error.set(e);
                    set_share_creating.set(false);
                    ToastContext::error(format!("Share creation failed: {}", err_msg));
                }
            }
        });
    };

    let do_copy_share_url = move |_: ev::MouseEvent| {
        let url = share_url.get();
        if !url.is_empty()
            && let Some(window) = web_sys::window()
        {
            let nav = window.navigator();
            let clipboard = nav.clipboard();
            wasm_bindgen_futures::spawn_local(async move {
                let _ = wasm_bindgen_futures::JsFuture::from(clipboard.write_text(&url)).await;
            });
            set_share_copied.set(true);
            ToastContext::info("Link copied to clipboard");
        }
    };

    let do_upload_files = move |file_list: web_sys::FileList| {
        let path = current_path.get();
        let count = file_list.length();
        for i in 0..count {
            let Some(file) = file_list.get(i) else {
                continue;
            };
            let file_name = file.name();
            let file_path = if path == "/" {
                format!("/{}", file_name)
            } else {
                format!("{}/{}", path, file_name)
            };
            spawn_local(async move {
                if let Ok(ab) = wasm_bindgen_futures::JsFuture::from(file.array_buffer()).await {
                    let uint8 = js_sys::Uint8Array::new(&ab);
                    let mut bytes = vec![0u8; uint8.length() as usize];
                    uint8.copy_to(&mut bytes);
                    match api::upload_file(&file_path, &bytes).await {
                        Ok(()) => {
                            ToastContext::success(format!("File uploaded: {}", file_name));
                            api::show_notification(
                                "Upload Complete",
                                &format!("{} uploaded successfully", file_name),
                            );
                            reload();
                        }
                        Err(e) => {
                            ToastContext::error(format!("Upload failed: {}", e));
                        }
                    }
                }
            });
        }
    };

    let handle_drag_over = move |ev: ev::DragEvent| {
        ev.prevent_default();
        set_upload_drag.set(true);
    };

    let handle_drag_leave = move |ev: ev::DragEvent| {
        ev.prevent_default();
        set_upload_drag.set(false);
    };

    let handle_drop = move |ev: ev::DragEvent| {
        ev.prevent_default();
        set_upload_drag.set(false);
        if let Some(dt) = ev.data_transfer()
            && let Some(files) = web_sys::DataTransfer::files(&dt)
        {
            do_upload_files(files);
        }
    };

    let handle_file_input = move |ev: ev::Event| {
        set_show_upload.set(false);
        let target = ev.target();
        let input: Option<web_sys::HtmlInputElement> = target.and_then(|t| {
            use wasm_bindgen::JsCast;
            t.dyn_into::<web_sys::HtmlInputElement>().ok()
        });
        if let Some(input) = input
            && let Some(files) = web_sys::HtmlInputElement::files(&input)
        {
            do_upload_files(files);
        }
    };

    let breadcrumb_segments = move || {
        let path = current_path.get();
        let mut segments: Vec<(String, String)> = vec![("/".to_string(), "Home".to_string())];
        if path != "/" {
            let parts: Vec<&str> = path.trim_matches('/').split('/').collect();
            let mut built = String::new();
            for part in parts {
                built = format!("{}/{}", built, part);
                segments.push((built.clone(), part.to_string()));
            }
        }
        segments
    };

    let open_preview = move |path: String| {
        let entries = all_entries.get();
        if let Some(entry) = entries.iter().find(|e| e.path == path) {
            set_preview_file.set(Some(entry.clone()));
        }
    };

    let close_preview = move |_: ()| {
        set_preview_file.set(None);
    };

    let switch_tab = move |tab: BrowserTab| {
        set_active_tab.set(tab);
        if tab == BrowserTab::Favorites {
            load_favorites();
        } else if tab == BrowserTab::Recent {
            load_recent();
        }
    };

    let is_fav = move |path: String| -> bool { favorites.with(|f| f.contains(&path)) };

    let fav_entries = move || {
        let favs = favorites.get();
        let entries = all_entries.get();
        entries
            .into_iter()
            .filter(|e| favs.contains(&e.path))
            .collect::<Vec<_>>()
    };

    let show_files_view = move || active_tab.get() == BrowserTab::Files;

    let toggle_select_mode = move |_: ev::MouseEvent| {
        let new_mode = !select_mode.get();
        set_select_mode.set(new_mode);
        if !new_mode {
            set_selected_paths.set(std::collections::HashSet::new());
        }
    };

    let toggle_select = move |path: String, index: usize, is_shift: bool, is_ctrl: bool| {
        if is_shift {
            if let Some(last) = last_clicked_index.get() {
                let entries = all_entries.get();
                let start = last.min(index);
                let end = last.max(index);
                set_selected_paths.update(|sel| {
                    for i in start..=end {
                        if let Some(entry) = entries.get(i) {
                            sel.insert(entry.path.clone());
                        }
                    }
                });
            } else {
                set_selected_paths.update(|sel| {
                    if sel.contains(&path) {
                        sel.remove(&path);
                    } else {
                        sel.insert(path);
                    }
                });
            }
        } else if is_ctrl {
            set_selected_paths.update(|sel| {
                if sel.contains(&path) {
                    sel.remove(&path);
                } else {
                    sel.insert(path);
                }
            });
        } else {
            set_selected_paths.update(|sel| {
                sel.clear();
                sel.insert(path);
            });
        }
        set_last_clicked_index.set(Some(index));
    };

    let do_select_all = move || {
        let entries = all_entries.get();
        let all_selected = entries
            .iter()
            .all(|e| selected_paths.with(|s| s.contains(&e.path)));
        if all_selected {
            set_selected_paths.set(std::collections::HashSet::new());
        } else {
            let all: std::collections::HashSet<String> =
                entries.iter().map(|e| e.path.clone()).collect();
            set_selected_paths.set(all);
        }
    };

    let select_all = move |_: ev::MouseEvent| {
        do_select_all();
    };

    let selected_count = move || selected_paths.with(|s| s.len());

    let do_bulk_delete_fn = move || {
        let paths: Vec<String> = selected_paths.get().into_iter().collect();
        if paths.is_empty() {
            return;
        }
        spawn_local(async move {
            match api::bulk_delete(&paths).await {
                Ok(resp) => {
                    let succeeded = resp.succeeded.len();
                    let failed = resp.failed.len();
                    if failed == 0 {
                        ToastContext::success(format!("Deleted {} file(s)", succeeded));
                    } else {
                        ToastContext::warning(format!("Deleted {}, {} failed", succeeded, failed));
                    }
                    set_selected_paths.set(std::collections::HashSet::new());
                    reload();
                }
                Err(e) => {
                    ToastContext::error(format!("Bulk delete failed: {}", e));
                }
            }
        });
    };

    let do_bulk_delete = move |_: ev::MouseEvent| {
        set_show_delete_confirm.set(true);
    };

    let do_confirmed_delete = move |_: ev::MouseEvent| {
        set_show_delete_confirm.set(false);
        do_bulk_delete_fn();
    };

    let do_bulk_download = move |_: ev::MouseEvent| {
        let paths: Vec<String> = selected_paths.get().into_iter().collect();
        for path in &paths {
            let p = path.clone();
            spawn_local(async move {
                let _ = api::download_file(&p).await;
            });
        }
    };

    let clipboard_copy_selected = move || {
        let paths: Vec<String> = selected_paths.get().into_iter().collect();
        let count = paths.len();
        clipboard_state.copy_files(paths);
        ToastContext::info(format!("{} file(s) copied to clipboard", count));
    };

    let clipboard_cut_selected = move || {
        let paths: Vec<String> = selected_paths.get().into_iter().collect();
        let count = paths.len();
        clipboard_state.cut_files(paths);
        ToastContext::info(format!("{} file(s) cut to clipboard", count));
    };

    let clipboard_paste = move || {
        let files = clipboard_state.files();
        let action = clipboard_state.action();
        let dest_path = current_path.get();

        spawn_local(async move {
            let mut succeeded = 0usize;
            let mut failed = 0usize;

            for source_path in &files {
                let file_name = source_path.rsplit('/').next().unwrap_or("");
                let dest = if dest_path == "/" {
                    format!("/{}", file_name)
                } else {
                    format!("{}/{}", dest_path, file_name)
                };

                let result = match action {
                    Some(ClipboardAction::Copy) => api::copy_file(source_path, &dest).await,
                    Some(ClipboardAction::Cut) => api::move_file(source_path, &dest).await,
                    None => Ok(()),
                };

                match result {
                    Ok(()) => succeeded += 1,
                    Err(_) => failed += 1,
                }
            }

            let action_str = match action {
                Some(ClipboardAction::Copy) => "copied",
                Some(ClipboardAction::Cut) => "moved",
                None => "pasted",
            };

            if failed == 0 {
                ToastContext::success(format!("{} file(s) {}", succeeded, action_str));
            } else {
                ToastContext::warning(format!("{} {}, {} failed", succeeded, action_str, failed));
            }

            clipboard_state.clear();
            reload();
        });
    };

    let is_entry_selected =
        move |path: String| -> bool { selected_paths.with(|s| s.contains(&path)) };

    let load_activity = move || {
        spawn_local(async move {
            match api::get_activity(20, 0).await {
                Ok(resp) => set_activity_entries.set(resp.entries),
                Err(_) => set_activity_entries.set(vec![]),
            }
        });
    };

    create_effect(move |_| {
        if show_activity.get() {
            load_activity();
        }
    });

    let toggle_activity = move |_: ev::MouseEvent| {
        set_show_activity.update(|v| *v = !*v);
    };

    create_effect(move |_| {
        let ts = theme_state.clone();
        let commands = vec![
            Command {
                id: "upload-file".to_string(),
                label: "Upload File".to_string(),
                shortcut: Some("Ctrl+U".to_string()),
                action: Callback::new(move |_| set_show_upload.set(true)),
            },
            Command {
                id: "new-folder".to_string(),
                label: "New Folder".to_string(),
                shortcut: Some("Ctrl+N".to_string()),
                action: Callback::new(move |_| set_show_new_folder.set(true)),
            },
            Command {
                id: "go-home".to_string(),
                label: "Go to Home".to_string(),
                shortcut: None,
                action: Callback::new(move |_| load_directory("/".to_string())),
            },
            Command {
                id: "go-trash".to_string(),
                label: "Go to Trash".to_string(),
                shortcut: None,
                action: Callback::new(move |_| {
                    #[cfg(target_arch = "wasm32")]
                    {
                        if let Some(window) = web_sys::window() {
                            let loc = window.location();
                            let _ = loc.set_href("/ui/trash");
                        }
                    }
                }),
            },
            Command {
                id: "toggle-dark-mode".to_string(),
                label: "Toggle Dark Mode".to_string(),
                shortcut: None,
                action: Callback::new(move |_| {
                    let current = ts.theme().get();
                    let next = match current {
                        Theme::Light => Theme::Dark,
                        Theme::Dark => Theme::Light,
                    };
                    ts.set_theme(next);
                }),
            },
            Command {
                id: "select-all".to_string(),
                label: "Select All".to_string(),
                shortcut: Some("Ctrl+A".to_string()),
                action: Callback::new(move |_| do_select_all()),
            },
            Command {
                id: "delete-selected".to_string(),
                label: "Delete Selected".to_string(),
                shortcut: Some("Del".to_string()),
                action: Callback::new(move |_| {
                    if !selected_paths.with(|s| s.is_empty()) {
                        set_show_delete_confirm.set(true);
                    }
                }),
            },
            Command {
                id: "open-preview".to_string(),
                label: "Open File Preview".to_string(),
                shortcut: None,
                action: Callback::new(move |_| {
                    let paths: Vec<String> = selected_paths.get().into_iter().collect();
                    if let Some(path) = paths.first() {
                        let entries = all_entries.get();
                        if let Some(entry) = entries.iter().find(|e| &e.path == path) {
                            set_preview_file.set(Some(entry.clone()));
                        }
                    }
                }),
            },
            Command {
                id: "search-files".to_string(),
                label: "Search Files".to_string(),
                shortcut: Some("Ctrl+F".to_string()),
                action: Callback::new(move |_| {
                    if let Some(hs) = header_state {
                        hs.open_search();
                    }
                }),
            },
            Command {
                id: "toggle-activity".to_string(),
                label: "Toggle Activity Feed".to_string(),
                shortcut: None,
                action: Callback::new(move |_| {
                    set_show_activity.update(|v| *v = !*v);
                }),
            },
            Command {
                id: "copy-selected".to_string(),
                label: "Copy Selected".to_string(),
                shortcut: Some("Ctrl+C".to_string()),
                action: Callback::new(move |_| clipboard_copy_selected()),
            },
            Command {
                id: "cut-selected".to_string(),
                label: "Cut Selected".to_string(),
                shortcut: Some("Ctrl+X".to_string()),
                action: Callback::new(move |_| clipboard_cut_selected()),
            },
            Command {
                id: "paste-files".to_string(),
                label: "Paste Files".to_string(),
                shortcut: Some("Ctrl+V".to_string()),
                action: Callback::new(move |_| clipboard_paste()),
            },
            Command {
                id: "toggle-view".to_string(),
                label: "Toggle Grid/List View".to_string(),
                shortcut: None,
                action: Callback::new(move |_| {
                    let current = view_mode.get();
                    let next = match current {
                        ViewMode::List => ViewMode::Grid,
                        ViewMode::Grid => ViewMode::List,
                    };
                    set_view_mode.set(next);
                }),
            },
        ];
        palette_state.set_commands(commands);
    });

    #[cfg(target_arch = "wasm32")]
    {
        let ps = palette_state;
        let cs = clipboard_state;
        let su = set_show_upload;
        let snf = set_show_new_folder;
        let sa = do_select_all;
        let sbd = do_bulk_delete_fn;
        let spf = set_preview_file;
        let ssp = set_selected_paths;
        let sdc = set_show_delete_confirm;
        let sact = set_show_activity;
        let cc = clipboard_copy_selected;
        let cx = clipboard_cut_selected;
        let cv = clipboard_paste;
        let sm = show_move_dialog;
        let scd = show_copy_dialog;
        let sshd = show_share_dialog;
        let snfolder = show_new_folder;
        let supload = show_upload;
        let hs = header_state;
        let sel_paths = selected_paths;
        let prev_file = preview_file;
        let show_dc = show_delete_confirm;
        let set_sm = set_show_move_dialog;
        let set_scd = set_show_copy_dialog;
        let set_sshd = set_show_share_dialog;

        spawn_local(async move {
            if let Some(window) = web_sys::window() {
                if let Some(document) = window.document() {
                    use wasm_bindgen::JsCast;
                    let cb = wasm_bindgen::closure::Closure::wrap(Box::new(
                        move |ev: web_sys::KeyboardEvent| {
                            let tag = ev
                                .target()
                                .and_then(|t| {
                                    use wasm_bindgen::JsCast;
                                    t.dyn_into::<web_sys::Element>().ok()
                                })
                                .map(|el| el.tag_name().to_lowercase())
                                .unwrap_or_default();

                            if tag == "input" || tag == "textarea" || tag == "select" {
                                return;
                            }

                            let ctrl = ev.ctrl_key() || ev.meta_key();

                            if ctrl && ev.key() == "k" {
                                ev.prevent_default();
                                ps.toggle();
                            } else if ctrl && ev.key() == "n" {
                                ev.prevent_default();
                                snf.set(true);
                            } else if ctrl && ev.key() == "u" {
                                ev.prevent_default();
                                su.set(true);
                            } else if ev.key() == "Delete" || ev.key() == "Backspace" {
                                ev.prevent_default();
                                if !sel_paths.with(|s| s.is_empty()) {
                                    sdc.set(true);
                                }
                            } else if ctrl && ev.key() == "a" {
                                ev.prevent_default();
                                sa();
                            } else if ev.key() == "Escape" {
                                if ps.is_open() {
                                    ps.close();
                                    return;
                                }
                                if prev_file.get().is_some() {
                                    spf.set(None);
                                    return;
                                }
                                if snfolder.get()
                                    || supload.get()
                                    || sshd.get()
                                    || sm.get()
                                    || scd.get()
                                    || show_dc.get()
                                {
                                    snf.set(false);
                                    su.set(false);
                                    set_sshd.set(false);
                                    set_sm.set(false);
                                    set_scd.set(false);
                                    sdc.set(false);
                                    return;
                                }
                                if !sel_paths.with(|s| s.is_empty()) {
                                    ssp.set(std::collections::HashSet::new());
                                    return;
                                }
                            } else if ctrl && ev.key() == "f" {
                                ev.prevent_default();
                                if let Some(h) = hs {
                                    h.open_search();
                                }
                            } else if ctrl && ev.key() == "c" {
                                ev.prevent_default();
                                cc();
                            } else if ctrl && ev.key() == "x" {
                                ev.prevent_default();
                                cx();
                            } else if ctrl && ev.key() == "v" {
                                ev.prevent_default();
                                if cs.has_files() {
                                    cv();
                                }
                            }
                        },
                    )
                        as Box<dyn Fn(web_sys::KeyboardEvent)>);
                    let _ = document
                        .add_event_listener_with_callback("keydown", cb.as_ref().unchecked_ref());
                    cb.forget();
                }
            }
        });
    }

    let do_move = move |path: String| {
        set_move_source.set(path.clone());
        set_move_dest.set(String::new());
        set_show_move_dialog.set(true);
    };

    let do_copy = move |path: String| {
        set_copy_source.set(path.clone());
        set_copy_dest.set(String::new());
        set_show_copy_dialog.set(true);
    };

    let execute_move = move |_: ev::MouseEvent| {
        let source = move_source.get();
        let dest = move_dest.get();
        if dest.is_empty() {
            ToastContext::error("Destination path cannot be empty");
            return;
        }
        let s = source.clone();
        let d = dest.clone();
        spawn_local(async move {
            match api::move_file(&s, &d).await {
                Ok(()) => {
                    set_show_move_dialog.set(false);
                    ToastContext::success(format!("Moved {} to {}", s, d));
                    reload();
                }
                Err(e) => ToastContext::error(format!("Move failed: {}", e)),
            }
        });
    };

    let execute_copy = move |_: ev::MouseEvent| {
        let source = copy_source.get();
        let dest = copy_dest.get();
        if dest.is_empty() {
            ToastContext::error("Destination path cannot be empty");
            return;
        }
        let s = source.clone();
        let d = dest.clone();
        spawn_local(async move {
            match api::copy_file(&s, &d).await {
                Ok(()) => {
                    set_show_copy_dialog.set(false);
                    ToastContext::success(format!("Copied {} to {}", s, d));
                    reload();
                }
                Err(e) => ToastContext::error(format!("Copy failed: {}", e)),
            }
        });
    };

    let grid_cb_navigate = Callback::new(navigate);
    let grid_cb_delete = Callback::new(do_delete);
    let grid_cb_download = Callback::new(do_download);
    let grid_cb_share = Callback::new(do_share);
    let grid_cb_preview = Callback::new(open_preview);
    let grid_cb_fav = Callback::new(do_toggle_favorite);
    let grid_cb_select = Callback::new(
        move |(path, idx, shift, ctrl): (String, usize, bool, bool)| {
            toggle_select(path, idx, shift, ctrl);
        },
    );
    let grid_cb_copy = Callback::new(do_copy);
    let grid_cb_move = Callback::new(do_move);

    view! {
        <div class="flex flex-1 overflow-hidden">
        <div class="flex-1 overflow-auto"
            _ref=scroll_container_ref
            role="region"
            aria-label="File list"
            on:dragover=handle_drag_over
            on:dragleave=handle_drag_leave
            on:drop=handle_drop
        >
            // Toolbar
            <div class="brutal-border border-b px-3 sm:px-6 py-3 surface shadow-concrete">
                <div class="flex items-center justify-between gap-2">
                    <div class="flex items-center gap-2 min-w-0 flex-1">
                        <button
                            class="p-2 text-gray-500 hover:text-gray-700 hover:bg-gray-100 rounded transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 min-w-[44px] min-h-[44px] flex items-center justify-center shrink-0"
                            aria-label="Go to parent directory"
                            on:click=go_up
                            disabled=move || current_path.get() == "/"
                        >
                            <svg class="w-5 h-5" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 17l-5-5m0 0l5-5m-5 5h12" />
                            </svg>
                        </button>

                        <nav aria-label="Breadcrumb" class="flex items-center gap-1 text-sm min-w-0 overflow-hidden">
                            <ol class="flex items-center gap-1 list-none m-0 p-0 overflow-hidden">
                                <For
                                    each=breadcrumb_segments
                                    key=|(path, _)| path.clone()
                                    let:segment
                                >
                                    {
                                        let (path, label) = segment;
                                        let is_root = path == "/";
                                        let p = path.clone();
                                        let is_current = move || path == current_path.get();
                                        view! {
                                            <li class="flex items-center">
                                                {(!is_root).then(|| view! {
                                                    <span class="text-gray-500 mx-1" aria-hidden="true">"/"</span>
                                                })}
                                                <button
                                                    class="text-blue-600 hover:text-blue-800 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 rounded truncate max-w-[120px] sm:max-w-none"
                                                    attr:aria-current=move || if is_current() { Some("page") } else { None }
                                                    on:click=move |_| navigate(p.clone())
                                                >
                                                    {label}
                                                </button>
                                            </li>
                                        }
                                    }
                                </For>
                            </ol>
                        </nav>
                    </div>

                    <div class="flex items-center gap-1 sm:gap-2 flex-wrap justify-end shrink-0">
                        <div class="flex items-center bg-gray-100 dark:bg-gray-700 rounded p-0.5">
                            <button
                                class="px-2 sm:px-3 py-1 text-xs font-bold uppercase tracking-wider font-mono transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500"
                                class=move || if active_tab.get() == BrowserTab::Files { "bg-white dark:bg-gray-600 text-gray-900 dark:text-gray-100 shadow-sm border-b-2 border-b-blue-600" } else { "text-gray-500 hover:text-gray-700" }
                                on:click=move |_| switch_tab(BrowserTab::Files)
                            >
                                "Files"
                            </button>
                            <button
                                class="px-2 sm:px-3 py-1 text-xs font-bold uppercase tracking-wider font-mono transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500"
                                class=move || if active_tab.get() == BrowserTab::Favorites { "bg-white dark:bg-gray-600 text-gray-900 dark:text-gray-100 shadow-sm border-b-2 border-b-blue-600" } else { "text-gray-500 hover:text-gray-700" }
                                on:click=move |_| switch_tab(BrowserTab::Favorites)
                            >
                                <span class="hidden sm:inline">"Favorites"</span>
                                <svg class="w-4 h-4 sm:hidden" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11.049 2.927c.3-.921 1.603-.921 1.902 0l1.519 4.674a1 1 0 00.95.69h4.915c.969 0 1.371 1.24.588 1.81l-3.976 2.888a1 1 0 00-.363 1.118l1.518 4.674c.3.922-.755 1.688-1.538 1.118l-3.976-2.888a1 1 0 00-1.176 0l-3.976 2.888c-.783.57-1.838-.197-1.538-1.118l1.518-4.674a1 1 0 00-.363-1.118l-3.976-2.888c-.784-.57-.38-1.81.588-1.81h4.914a1 1 0 00.951-.69l1.519-4.674z" /></svg>
                            </button>
                            <button
                                class="px-2 sm:px-3 py-1 text-xs font-bold uppercase tracking-wider font-mono transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500"
                                class=move || if active_tab.get() == BrowserTab::Recent { "bg-white dark:bg-gray-600 text-gray-900 dark:text-gray-100 shadow-sm border-b-2 border-b-blue-600" } else { "text-gray-500 hover:text-gray-700" }
                                on:click=move |_| switch_tab(BrowserTab::Recent)
                            >
                                <span class="hidden sm:inline">"Recent"</span>
                                <svg class="w-4 h-4 sm:hidden" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" /></svg>
                            </button>
                        </div>

                        {move || clipboard_state.has_files().then(|| {
                            let count = clipboard_state.file_count();
                            let al = clipboard_state.action().map(|a| match a {
                                ClipboardAction::Copy => "Copy",
                                ClipboardAction::Cut => "Cut",
                            }).unwrap_or_default();
                            view! {
                                <button
                                    class="px-2 sm:px-3 py-1.5 text-xs sm:text-sm bg-green-600 text-white rounded-sm brutal-border font-bold uppercase hover:bg-green-700 transition-colors flex items-center gap-1 focus:outline-none focus:ring-2 focus:ring-green-500 focus:ring-offset-2 dark:focus:ring-offset-gray-800 min-h-[44px]"
                                    on:click=move |_| clipboard_paste()
                                    title=move || format!("{} file(s) on clipboard ({})", count, al)
                                >
                                    <svg class="w-4 h-4 shrink-0" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2" />
                                    </svg>
                                    <span class="hidden sm:inline">{move || format!("{} ({})", count, al)}</span>
                                    <span class="sm:hidden">{count}</span>
                                </button>
                            }.into_any()
                        })}

                        <button
                            class="px-2 sm:px-3 py-1.5 text-xs sm:text-sm bg-blue-600 text-white rounded-sm hover:bg-blue-700 brutal-border shadow-iron transition-colors flex items-center gap-1 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 dark:focus:ring-offset-gray-800 min-h-[44px] uppercase font-bold tracking-wider"
                            aria-label="Upload files"
                            on:click=move |_| set_show_upload.set(true)
                        >
                            <svg class="w-4 h-4 shrink-0" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
                            </svg>
                            <span class="hidden sm:inline">"Upload"</span>
                        </button>
                        <button
                            class="px-2 sm:px-3 py-1.5 text-xs sm:text-sm bg-gray-100 dark:bg-gray-700 text-gray-700 rounded-sm brutal-border font-bold uppercase hover:bg-gray-200 dark:hover:bg-gray-600 transition-colors flex items-center gap-1 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 dark:focus:ring-offset-gray-800 min-h-[44px] tracking-wider"
                            aria-label="New folder"
                            on:click=move |_| set_show_new_folder.set(true)
                        >
                            <svg class="w-4 h-4 shrink-0" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 13h6m-3-3v6m5 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                            </svg>
                            <span class="hidden sm:inline">"New Folder"</span>
                        </button>
                        <A
                            href="/ui/trash"
                            class="px-2 sm:px-3 py-1.5 text-xs sm:text-sm text-gray-600 hover:text-gray-800 rounded hover:bg-gray-100 transition-colors no-underline flex items-center gap-1 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 dark:focus:ring-offset-gray-800 min-h-[44px]"
                            attr:aria-label="Trash"
                        >
                            <svg class="w-4 h-4 shrink-0" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                            </svg>
                            <span class="hidden sm:inline">"Trash"</span>
                        </A>

                        // View mode toggle
                        <button
                            class="p-2 text-gray-500 hover:text-gray-700 hover:bg-gray-100 rounded transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 min-w-[44px] min-h-[44px] flex items-center justify-center"
                            aria-label=move || if view_mode.get() == ViewMode::Grid { "Switch to list view" } else { "Switch to grid view" }
                            title=move || if view_mode.get() == ViewMode::Grid { "List view" } else { "Grid view" }
                            on:click=toggle_view_mode
                        >
                            {move || match view_mode.get() {
                                ViewMode::List => view! {
                                    <svg class="w-5 h-5" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2V6zM14 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2V6zM4 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2v-2zM14 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2v-2z" />
                                    </svg>
                                }.into_any(),
                                ViewMode::Grid => view! {
                                    <svg class="w-5 h-5" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 10h16M4 14h16M4 18h16" />
                                    </svg>
                                }.into_any(),
                            }}
                        </button>

                        <button
                            class=move || format!(
                                "p-2 text-sm rounded transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 min-w-[44px] min-h-[44px] flex items-center justify-center {}",
                                if select_mode.get() { "bg-blue-100 dark:bg-blue-900/40 text-blue-700 dark:text-blue-300" } else { "text-gray-500 hover:text-gray-700 hover:bg-gray-100" }
                            )
                            aria-label="Toggle select mode"
                            aria_pressed=move || select_mode.get()
                            on:click=toggle_select_mode
                        >
                            <svg class="w-5 h-5" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2m-6 9l2 2 4-4" />
                            </svg>
                        </button>
                        <button
                            class=move || format!(
                                "p-2 text-sm rounded transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 min-w-[44px] min-h-[44px] flex items-center justify-center transition-all duration-200 {}",
                                if show_activity.get() { "bg-blue-100 dark:bg-blue-900/40 text-blue-700 dark:text-blue-300" } else { "text-gray-500 hover:text-gray-700 hover:bg-gray-100" }
                            )
                            aria-label="Toggle activity panel"
                            on:click=toggle_activity
                        >
                            <svg class="w-5 h-5" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
                            </svg>
                        </button>
                    </div>
                </div>
            </div>

            // Drag overlay
            {move || upload_drag.get().then(|| view! {
                <div class="fixed inset-0 bg-blue-500 bg-opacity-20 z-50 flex items-center justify-center pointer-events-none backdrop-blur-sm" aria-hidden="true">
                    <div class="surface brutal-border shadow-2xl p-12 text-center blob-shape float-animation">
                        <svg class="w-16 h-16 text-accent mx-auto mb-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12" />
                        </svg>
                        <p class="text-xl font-semibold text-gray-700">"Drop files to upload"</p>
                        <p class="text-sm text-gray-500 mt-1">"Files will be uploaded to the current directory"</p>
                    </div>
                </div>
            })}

            // New folder dialog
            {move || show_new_folder.get().then(|| view! {
                <div class="fixed inset-0 bg-black bg-opacity-50 z-40 flex items-center justify-center transition-opacity duration-200"
                    on:keydown=move |ev: ev::KeyboardEvent| {
                        if ev.key() == "Escape" {
                            set_show_new_folder.set(false);
                        }
                    }
                >
                    <div class="brutal-block rounded shadow-xl p-6 w-[calc(100%-2rem)] sm:w-96 mx-auto transition-all duration-200"
                        role="dialog"
                        aria-modal="true"
                        aria-labelledby="new-folder-title"
                        tabindex="-1"
                    >
                        <h3 id="new-folder-title" class="text-section font-mono text-gray-900 mb-4">"New Folder"</h3>
                        <label class="block mb-4">
                            <span class="block text-xs font-bold uppercase font-mono text-gray-700 mb-1">"Folder name"</span>
                            <input
                                type="text"
                                placeholder="Folder name"
                                class="w-full px-3 py-2 border rounded bg-white dark:bg-gray-800 font-mono text-gray-900 focus:outline-none focus:ring-2 focus:ring-blue-500"
                                prop:value=new_folder_name
                                on:input=move |ev| set_new_folder_name.set(event_target_value(&ev))
                            />
                        </label>
                        <div class="flex justify-end gap-2">
                            <button
                                class="px-4 py-2 text-sm text-gray-600 hover:text-gray-800 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 rounded"
                                on:click=move |_| set_show_new_folder.set(false)
                            >
                                "Cancel"
                            </button>
                            <button
                                class="px-4 py-2 text-sm bg-blue-600 text-white brutal-border rounded-sm font-bold uppercase hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 dark:focus:ring-offset-gray-800"
                                on:click=do_create_folder
                            >
                                "Create"
                            </button>
                        </div>
                    </div>
                </div>
            })}

            // Upload dialog
            {move || show_upload.get().then(|| view! {
                <div class="fixed inset-0 bg-black bg-opacity-50 z-40 flex items-center justify-center transition-opacity duration-200"
                    on:keydown=move |ev: ev::KeyboardEvent| {
                        if ev.key() == "Escape" {
                            set_show_upload.set(false);
                        }
                    }
                >
                    <div class="brutal-block rounded shadow-xl p-6 w-[calc(100%-2rem)] sm:w-96 mx-auto transition-all duration-200"
                        role="dialog"
                        aria-modal="true"
                        aria-labelledby="upload-title"
                        tabindex="-1"
                    >
                        <h3 id="upload-title" class="text-section font-mono text-gray-900 mb-4">"Upload File"</h3>
                        <label class="block w-full border-2 border-dashed border-gray-300 rounded p-8 text-center cursor-pointer hover:border-blue-400 transition-colors">
                            <svg class="w-12 h-12 text-gray-400 mx-auto mb-3" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12" />
                            </svg>
                            <p id="upload-file-hint" class="text-sm text-gray-600">"Click to select files or drag and drop"</p>
                            <input
                                type="file"
                                class="hidden"
                                multiple
                                aria-describedby="upload-file-hint"
                                on:change=handle_file_input
                            />
                        </label>
                        <div class="flex justify-end mt-4">
                            <button
                                class="px-4 py-2 text-sm text-gray-600 hover:text-gray-800 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 rounded"
                                on:click=move |_| set_show_upload.set(false)
                            >
                                "Close"
                            </button>
                        </div>
                    </div>
                </div>
            })}

            // Share dialog
            {move || show_share_dialog.get().then(|| view! {
                <div class="fixed inset-0 bg-black bg-opacity-50 z-40 flex items-center justify-center transition-opacity duration-200"
                    on:keydown=move |ev: ev::KeyboardEvent| {
                        if ev.key() == "Escape" {
                            set_show_share_dialog.set(false);
                        }
                    }
                >
                    <div class="brutal-block rounded shadow-xl p-6 w-[calc(100%-2rem)] sm:w-96 mx-auto transition-all duration-200"
                        role="dialog"
                        aria-modal="true"
                        aria-labelledby="share-title"
                        tabindex="-1"
                    >
                        <div class="flex items-center justify-between mb-4">
                            <h3 id="share-title" class="text-lg font-semibold text-gray-900">"Share File"</h3>
                            <button
                                class="p-1 text-gray-400 hover:text-gray-600 focus:outline-none focus:ring-2 focus:ring-blue-500 rounded"
                                aria-label="Close dialog"
                                on:click=move |_| set_show_share_dialog.set(false)
                            >
                                <svg class="w-5 h-5" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                                </svg>
                            </button>
                        </div>

                        <div class="mb-4">
                            <label class="block text-xs font-bold uppercase font-mono text-gray-700 mb-1">"Path"</label>
                            <div class="px-3 py-2 bg-gray-50 dark:bg-gray-900 border rounded text-sm text-gray-600 truncate">
                                {share_path}
                            </div>
                        </div>

                        <div class="mb-4">
                            <label class="block text-xs font-bold uppercase font-mono text-gray-700 mb-1">"Password (optional)"</label>
                            <input
                                type="password"
                                placeholder="Leave empty for no password"
                                class="w-full px-3 py-2 border rounded bg-white dark:bg-gray-800 font-mono text-gray-900 focus:outline-none focus:ring-2 focus:ring-blue-500 text-sm"
                                prop:value=share_password
                                on:input=move |ev| set_share_password.set(event_target_value(&ev))
                            />
                        </div>

                        <div class="mb-4">
                            <label class="block text-xs font-bold uppercase font-mono text-gray-700 mb-1">"Expires"</label>
                            <select
                                class="w-full px-3 py-2 border rounded bg-white dark:bg-gray-800 font-mono text-gray-900 focus:outline-none focus:ring-2 focus:ring-blue-500 text-sm"
                                on:change=move |ev| set_share_expires.set(event_target_value(&ev))
                            >
                                <option value="1" selected=move || share_expires.get() == "1">"1 hour"</option>
                                <option value="24" selected=move || share_expires.get() == "24">"24 hours"</option>
                                <option value="168" selected=move || share_expires.get() == "168">"7 days"</option>
                                <option value="720" selected=move || share_expires.get() == "720">"30 days"</option>
                            </select>
                        </div>

                        {move || (!share_error.get().is_empty()).then(|| view! {
                            <div class="mb-4 p-2 bg-red-50 border-l-4 border-l-red-500 rounded text-sm text-red-700" role="alert">
                                {share_error}
                            </div>
                        })}

                        {move || (!share_url.get().is_empty()).then(|| view! {
                            <div class="mb-4">
                                <label class="block text-xs font-bold uppercase font-mono text-gray-700 mb-1">"Share URL"</label>
                                <div class="flex items-center gap-2">
                                    <input
                                        type="text"
                                        readonly
                                        class="flex-1 px-3 py-2 bg-gray-50 dark:bg-gray-900 border rounded text-sm text-gray-600 font-mono"
                                        prop:value=share_url
                                    />
                                    <button
                                        class="px-3 py-2 text-sm bg-green-600 text-white brutal-border rounded-sm font-bold uppercase hover:bg-green-700 transition-colors whitespace-nowrap focus:outline-none focus:ring-2 focus:ring-green-500 focus:ring-offset-2 dark:focus:ring-offset-gray-800"
                                        on:click=do_copy_share_url
                                    >
                                        {move || if share_copied.get() { "Copied!" } else { "Copy" }}
                                    </button>
                                </div>
                            </div>
                        })}

                        <div class="flex justify-end gap-2">
                            <button
                                class="px-4 py-2 text-sm text-gray-600 hover:text-gray-800 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 rounded"
                                on:click=move |_| set_show_share_dialog.set(false)
                            >
                                "Close"
                            </button>
                            {move || share_url.get().is_empty().then(|| view! {
                                <button
                                    class="px-4 py-2 text-sm bg-blue-600 text-white brutal-border rounded-sm font-bold uppercase hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 dark:focus:ring-offset-gray-800"
                                    disabled=share_creating
                                    on:click=do_create_share
                                >
                                    {move || if share_creating.get() { "Creating..." } else { "Create Share" }}
                                </button>
                            })}
                        </div>
                    </div>
                </div>
            })}

            // Error display
            {move || error.get().map(|e| view! {
                <div class="bg-red-50 border-b border-l-4 border-l-red-500 px-6 py-3" role="alert" aria-live="assertive">
                    <div class="flex items-center justify-between">
                        <span class="text-red-700 text-sm">"Error: " {e}</span>
                        <button
                            class="text-red-500 hover:text-red-700 focus:outline-none focus:ring-2 focus:ring-red-500 focus:ring-offset-2 rounded p-0.5"
                            aria-label="Dismiss error"
                            on:click=move |_| set_error.set(None)
                        >
                            <svg class="w-4 h-4" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                            </svg>
                        </button>
                    </div>
                </div>
            })}

            // Loading skeleton (grid)
            {move || (loading.get() && view_mode.get() == ViewMode::Grid).then(|| view! {
                <SkeletonGrid />
            })}

            // Loading skeleton (list)
            {move || (loading.get() && view_mode.get() == ViewMode::List).then(|| view! {
                <SkeletonList />
            })}

            // Favorites view
            {move || (active_tab.get() == BrowserTab::Favorites).then(|| view! {
                {move || favorites_loading.get().then(|| view! {
                    <SkeletonFavorites />
                })}
                {move || {
                    if favorites_loading.get() {
                        return view! { <div class="hidden"></div> }.into_any();
                    }
                    let favs = fav_entries();
                    if favs.is_empty() {
                        view! {
                            <div class="px-6 py-16 text-center text-gray-500" role="status">
                                <svg class="w-16 h-16 mx-auto mb-4" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M11.049 2.927c.3-.921 1.603-.921 1.902 0l1.519 4.674a1 1 0 00.95.69h4.915c.969 0 1.371 1.24.588 1.81l-3.976 2.888a1 1 0 00-.363 1.118l1.518 4.674c.3.922-.755 1.688-1.538 1.118l-3.976-2.888a1 1 0 00-1.176 0l-3.976 2.888c-.783.57-1.838-.197-1.538-1.118l1.518-4.674a1 1 0 00-.363-1.118l-3.976 2.888c-.784-.57-.38-1.81.588-1.81h4.914a1 1 0 00.951-.69l1.519 4.674z" />
                                </svg>
                                <div class="text-lg font-medium">"No favorites yet"</div>
                                <div class="text-sm mt-1">"Star files to add them here"</div>
                            </div>
                        }.into_any()
                    } else {
                        view! {
                            <table class="w-full" role="grid">
                                <thead class="bg-gray-50 border-b sticky top-0">
                                    <tr>
                                        <th class="px-4 py-2 text-left text-xs font-bold uppercase font-mono text-gray-500 w-10" scope="col"></th>
                                        <th class="px-4 py-2 text-left text-xs font-bold uppercase font-mono text-gray-500" scope="col">"Name"</th>
                                        <th class="px-4 py-2 text-left text-xs font-bold uppercase font-mono text-gray-500 w-24" scope="col">"Size"</th>
                                        <th class="px-4 py-2 text-left text-xs font-bold uppercase font-mono text-gray-500 w-40" scope="col">"Modified"</th>
                                        <th class="px-4 py-2 text-right text-xs font-bold uppercase font-mono text-gray-500 w-24" scope="col">"Actions"</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    <For
                                        each=move || fav_entries()
                                        key=|entry| entry.path.clone()
                                        let:entry
                                    >
                                        <FileRow
                                            entry=entry
                                            on_navigate=Callback::new(navigate)
                                            on_delete=Callback::new(do_delete)
                                            on_download=Callback::new(do_download)
                                            on_share=Callback::new(do_share)
                                            on_preview=Callback::new(open_preview)
                                            is_favorited=true
                                            on_toggle_favorite=Callback::new(do_toggle_favorite)
                                            show_checkbox=false
                                            is_selected=false
                                            on_toggle_select=Callback::new(move |_: (String, usize, bool, bool)| {})
                                            on_copy=Callback::new(do_copy)
                                            on_move=Callback::new(do_move)
                                        />
                                    </For>
                                </tbody>
                            </table>
                        }.into_any()
                    }
                }}
            })}

            // Recent view
            {move || (active_tab.get() == BrowserTab::Recent).then(|| view! {
                {move || recent_loading.get().then(|| view! {
                    <SkeletonRecent />
                })}
                {move || {
                    if recent_loading.get() {
                        return view! { <div class="hidden"></div> }.into_any();
                    }
                    let recent = recent_files.get();
                    if recent.is_empty() {
                        view! {
                            <div class="px-6 py-16 text-center text-gray-500" role="status">
                                <svg class="w-16 h-16 mx-auto mb-4" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
                                </svg>
                                <div class="text-lg font-medium">"No recent activity"</div>
                                <div class="text-sm mt-1">"Upload files to get started"</div>
                            </div>
                        }.into_any()
                    } else {
                        view! {
                            <table class="w-full" role="grid">
                                <thead class="bg-gray-50 border-b sticky top-0">
                                    <tr>
                                        <th class="px-4 py-2 text-left text-xs font-bold uppercase font-mono text-gray-500 w-10" scope="col"></th>
                                        <th class="px-4 py-2 text-left text-xs font-bold uppercase font-mono text-gray-500" scope="col">"Name"</th>
                                        <th class="px-4 py-2 text-left text-xs font-bold uppercase font-mono text-gray-500 w-24" scope="col">"Size"</th>
                                        <th class="px-4 py-2 text-left text-xs font-bold uppercase font-mono text-gray-500 w-40" scope="col">"Modified"</th>
                                        <th class="px-4 py-2 text-right text-xs font-bold uppercase font-mono text-gray-500 w-24" scope="col">"Actions"</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    <For
                                        each=move || recent_files.get()
                                        key=|entry| entry.path.clone()
                                        let:entry
                                    >
                                        {
                                            let entry_path = entry.path.clone();
                                            view! {
                                                <FileRow
                                                    entry=entry
                                                    on_navigate=Callback::new(navigate)
                                                    on_delete=Callback::new(do_delete)
                                                    on_download=Callback::new(do_download)
                                                    on_share=Callback::new(do_share)
                                                    on_preview=Callback::new(open_preview)
                                                    is_favorited=is_fav(entry_path)
                                                    on_toggle_favorite=Callback::new(do_toggle_favorite)
                                                    show_checkbox=false
                                                    is_selected=false
                                                    on_toggle_select=Callback::new(move |_: (String, usize, bool, bool)| {})
                                                    on_copy=Callback::new(do_copy)
                                                    on_move=Callback::new(do_move)
                                                />
                                            }
                                        }
                                    </For>
                                </tbody>
                            </table>
                        }.into_any()
                    }
                }}
            })}

            // File list/grid (Files tab)
            {move || (show_files_view() && !loading.get()).then(|| view! {
                {match view_mode.get() {
                    ViewMode::Grid => view! {
                        <div class="transition-opacity duration-200">
                            {move || select_mode.get().then(|| view! {
                                <div class="px-4 py-2 flex items-center gap-2">
                                    <input
                                        type="checkbox"
                                        class="rounded border text-blue-600 focus:ring-blue-500"
                                        aria-label="Select all files"
                                        on:click=select_all
                                    />
                                    <span class="text-xs text-gray-500">"Select all"</span>
                                </div>
                            })}
                            <GridView
                                entries=all_entries
                                on_navigate=grid_cb_navigate
                                on_delete=grid_cb_delete
                                on_download=grid_cb_download
                                on_share=grid_cb_share
                                on_preview=grid_cb_preview
                                favorites=favorites_signal
                                on_toggle_favorite=grid_cb_fav
                                show_checkbox=select_mode.get()
                                selected_paths=selected_paths_signal
                                on_toggle_select=grid_cb_select
                                on_copy=grid_cb_copy
                                on_move=grid_cb_move
                            />
                        </div>
                    }.into_any(),
                    ViewMode::List => view! {
                        <div class="transition-opacity duration-200">
                            <table class="w-full" role="grid">
                                <thead class="bg-gray-50 border-b sticky top-0 hidden md:table-header-group">
                                    <tr>
                                        {move || select_mode.get().then(|| view! {
                                            <th class="px-4 py-2 w-10" scope="col">
                                                <input
                                                    type="checkbox"
                                                    class="rounded border text-blue-600 focus:ring-blue-500"
                                                    aria-label="Select all files"
                                                    on:click=select_all
                                                />
                                            </th>
                                        })}
                                        <th class="px-4 py-2 text-left text-xs font-bold uppercase font-mono text-gray-500 w-10" scope="col"></th>
                                        <th class="px-4 py-2 text-left text-xs font-bold uppercase font-mono text-gray-500" scope="col">"Name"</th>
                                        <th class="px-4 py-2 text-left text-xs font-bold uppercase font-mono text-gray-500 w-24" scope="col">"Size"</th>
                                        <th class="px-4 py-2 text-left text-xs font-bold uppercase font-mono text-gray-500 w-40 hidden lg:table-cell" scope="col">"Modified"</th>
                                        <th class="px-4 py-2 text-right text-xs font-bold uppercase font-mono text-gray-500 w-24" scope="col">"Actions"</th>
                                    </tr>
                                </thead>
                                <tbody class="block md:table-row-group">
                                    <For
                                        each=display_entries
                                        key=|entry| entry.path.clone()
                                        let:entry
                                    >
                                        {
                                            let entry_path = entry.path.clone();
                                            let entry_path_for_fav = entry.path.clone();
                                            let sel_path = entry.path.clone();
                                            let _sel_idx = create_memo(move |_| {
                                                all_entries.with(|e| e.iter().position(|x| x.path == entry_path_for_fav).unwrap_or(0))
                                            });
                                            view! {
                                                <FileRow
                                                    entry=entry
                                                    on_navigate=Callback::new(navigate)
                                                    on_delete=Callback::new(do_delete)
                                                    on_download=Callback::new(do_download)
                                                    on_share=Callback::new(do_share)
                                                    on_preview=Callback::new(open_preview)
                                                    is_favorited=is_fav(entry_path)
                                                    on_toggle_favorite=Callback::new(do_toggle_favorite)
                                                    show_checkbox=select_mode.get()
                                                    is_selected=is_entry_selected(sel_path)
                                                    on_toggle_select=Callback::new(move |(path, idx, shift, ctrl): (String, usize, bool, bool)| {
                                                        toggle_select(path, idx, shift, ctrl);
                                                    })
                                                    on_copy=Callback::new(do_copy)
                                                    on_move=Callback::new(do_move)
                                                />
                                            }
                                        }
                                    </For>
                                </tbody>
                            </table>
                        </div>
                    }.into_any(),
                }}
            })}

            // Bulk action bar
            {move || (select_mode.get() && selected_count() > 0).then(|| view! {
                <div class="fixed bottom-0 sm:bottom-4 left-0 sm:left-1/2 sm:-translate-x-1/2 w-full sm:w-auto surface-dark text-white dark:text-gray-900 rounded-none sm:rounded shadow-2xl brutal-border border-t-3 border-t-accent px-4 sm:px-6 py-3 flex items-center gap-2 sm:gap-4 z-50 justify-between sm:justify-center transition-all duration-200">
                    <span class="text-sm font-bold font-mono uppercase tracking-wider">
                        {move || selected_count()} " selected"
                    </span>
                    <div class="flex items-center gap-2">
                        <button
                            class="px-3 py-1.5 text-sm bg-red-600 text-white brutal-border rounded-sm font-bold uppercase hover:bg-red-700 transition-colors focus:outline-none focus:ring-2 focus:ring-red-500 min-h-[44px]"
                            on:click=do_bulk_delete
                        >
                            "Delete"
                        </button>
                        <button
                            class="px-3 py-1.5 text-sm bg-blue-600 text-white brutal-border rounded-sm font-bold uppercase hover:bg-blue-700 transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 min-h-[44px]"
                            on:click=do_bulk_download
                        >
                            <span class="hidden sm:inline">"Download"</span>
                            <svg class="w-4 h-4 sm:hidden" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" /></svg>
                        </button>
                        <button
                            class="px-3 py-1.5 text-sm bg-gray-600 dark:bg-gray-300 text-white dark:text-gray-900 brutal-border rounded-sm font-bold uppercase hover:bg-gray-500 dark:hover:bg-gray-200 transition-colors focus:outline-none focus:ring-2 focus:ring-gray-400 min-h-[44px]"
                            on:click=move |_| {
                                set_selected_paths.set(std::collections::HashSet::new());
                            }
                        >
                            "Clear"
                        </button>
                    </div>
                </div>
            })}

            // Scroll sentinel -- IntersectionObserver watches this element
            {move || {
                let total = all_entries.with(Vec::len);
                let displayed = display_count.get();
                (total > displayed && !loading.get() && show_files_view()).then(|| view! {
                    <div _ref=scroll_sentinel_ref class="py-4 text-center text-sm text-muted font-mono" role="status" aria-live="polite">
                        <div class="animate-spin w-4 h-4 border-2 border-gray-300 dark:border-gray-600 border-t-accent rounded-full mx-auto mb-2"></div>
                        "Loading more..."
                    </div>
                })
            }}

            // Empty state (files tab)
            {move || (!loading.get() && show_files_view() && all_entries.with(Vec::is_empty) && error.get().is_none()).then(|| view! {
                <div class="px-6 py-16 text-center" role="status">
                    <svg class="w-20 h-20 mx-auto mb-4 text-gray-300" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
                    </svg>
                    <div class="text-lg font-medium text-gray-500">"This folder is empty"</div>
                    <div class="text-sm text-gray-400 mt-1 mb-4">"Drop files here or upload your first file"</div>
                    <button
                        class="px-4 py-2 text-sm bg-blue-600 text-white brutal-border rounded-sm font-bold uppercase hover:bg-blue-700 transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 dark:focus:ring-offset-gray-800"
                        on:click=move |_| set_show_upload.set(true)
                    >
                        "Upload your first file"
                    </button>
                </div>
            })}

            // Drag hint
            {move || (!loading.get() && !all_entries.with(Vec::is_empty) && show_files_view()).then(|| view! {
                <div class="border-t border-gray-100 px-6 py-2 text-center">
                    <span class="text-xs text-gray-500">"Drag and drop files here to upload"</span>
                </div>
            })}

            // Move dialog
            {move || show_move_dialog.get().then(|| view! {
                <div class="fixed inset-0 bg-black bg-opacity-50 z-40 flex items-center justify-center transition-opacity duration-200"
                    on:keydown=move |ev: ev::KeyboardEvent| {
                        if ev.key() == "Escape" { set_show_move_dialog.set(false); }
                    }
                >
                    <div class="brutal-block rounded shadow-xl p-6 w-[calc(100%-2rem)] sm:w-96 mx-auto transition-all duration-200"
                        role="dialog"
                        aria-modal="true"
                        tabindex="-1"
                    >
                        <h3 class="text-section font-mono text-gray-900 mb-4">"Move File"</h3>
                        <div class="mb-4">
                            <label class="block text-xs font-bold uppercase font-mono text-gray-700 mb-1">"Source"</label>
                            <div class="px-3 py-2 bg-gray-50 dark:bg-gray-900 border rounded text-sm text-gray-600 truncate">
                                {move_source}
                            </div>
                        </div>
                        <div class="mb-4">
                            <label class="block text-xs font-bold uppercase font-mono text-gray-700 mb-1">"Destination"</label>
                            <input
                                type="text"
                                placeholder="/new/path/file.txt"
                                class="w-full px-3 py-2 border rounded bg-white dark:bg-gray-800 font-mono text-gray-900 focus:outline-none focus:ring-2 focus:ring-blue-500"
                                prop:value=move_dest
                                on:input=move |ev| set_move_dest.set(event_target_value(&ev))
                            />
                        </div>
                        <div class="flex justify-end gap-2">
                            <button
                                class="px-4 py-2 text-sm text-gray-600 hover:text-gray-800 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 rounded"
                                on:click=move |_| set_show_move_dialog.set(false)
                            >"Cancel"</button>
                            <button
                                class="px-4 py-2 text-sm bg-blue-600 text-white brutal-border rounded-sm font-bold uppercase hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 dark:focus:ring-offset-gray-800"
                                on:click=execute_move
                            >"Move"</button>
                        </div>
                    </div>
                </div>
            })}

            // Copy dialog
            {move || show_copy_dialog.get().then(|| view! {
                <div class="fixed inset-0 bg-black bg-opacity-50 z-40 flex items-center justify-center transition-opacity duration-200"
                    on:keydown=move |ev: ev::KeyboardEvent| {
                        if ev.key() == "Escape" { set_show_copy_dialog.set(false); }
                    }
                >
                    <div class="brutal-block rounded shadow-xl p-6 w-[calc(100%-2rem)] sm:w-96 mx-auto transition-all duration-200"
                        role="dialog"
                        aria-modal="true"
                        tabindex="-1"
                    >
                        <h3 class="text-section font-mono text-gray-900 mb-4">"Copy File"</h3>
                        <div class="mb-4">
                            <label class="block text-xs font-bold uppercase font-mono text-gray-700 mb-1">"Source"</label>
                            <div class="px-3 py-2 bg-gray-50 dark:bg-gray-900 border rounded text-sm text-gray-600 truncate">
                                {copy_source}
                            </div>
                        </div>
                        <div class="mb-4">
                            <label class="block text-xs font-bold uppercase font-mono text-gray-700 mb-1">"Destination"</label>
                            <input
                                type="text"
                                placeholder="/new/path/copy.txt"
                                class="w-full px-3 py-2 border rounded bg-white dark:bg-gray-800 font-mono text-gray-900 focus:outline-none focus:ring-2 focus:ring-blue-500"
                                prop:value=copy_dest
                                on:input=move |ev| set_copy_dest.set(event_target_value(&ev))
                            />
                        </div>
                        <div class="flex justify-end gap-2">
                            <button
                                class="px-4 py-2 text-sm text-gray-600 hover:text-gray-800 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 rounded"
                                on:click=move |_| set_show_copy_dialog.set(false)
                            >"Cancel"</button>
                            <button
                                class="px-4 py-2 text-sm bg-blue-600 text-white brutal-border rounded-sm font-bold uppercase hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 dark:focus:ring-offset-gray-800"
                                on:click=execute_copy
                            >"Copy"</button>
                        </div>
                    </div>
                </div>
            })}

            // File preview modal
            {move || preview_file.get().map(|file| view! {
                <FilePreview
                    file=file
                    on_close=Callback::new(close_preview)
                />
            })}

            // Delete confirmation dialog
            {move || show_delete_confirm.get().then(|| view! {
                <div class="fixed inset-0 bg-black bg-opacity-50 z-50 flex items-center justify-center p-4 transition-opacity duration-200"
                    on:keydown=move |ev: ev::KeyboardEvent| {
                        if ev.key() == "Escape" {
                            set_show_delete_confirm.set(false);
                        }
                    }
                >
                    <div class="brutal-block rounded shadow-xl p-6 w-[calc(100%-2rem)] sm:w-96 mx-auto transition-all duration-200"
                        role="dialog"
                        aria-modal="true"
                        aria-labelledby="delete-confirm-title"
                        tabindex="-1"
                    >
                        <h3 id="delete-confirm-title" class="text-lg font-semibold text-gray-900 mb-2">"Confirm Delete"</h3>
                        <p class="text-sm text-gray-600 mb-6">
                            {move || format!("Are you sure you want to delete {} file(s)? This action cannot be undone.", selected_count())}
                        </p>
                        <div class="flex justify-end gap-2">
                            <button
                                class="px-4 py-2 text-sm text-gray-600 hover:text-gray-800 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 rounded min-h-[44px]"
                                on:click=move |_| set_show_delete_confirm.set(false)
                            >
                                "Cancel"
                            </button>
                            <button
                                class="px-4 py-2 text-sm bg-red-600 text-white brutal-border rounded-sm font-bold uppercase hover:bg-red-700 focus:outline-none focus:ring-2 focus:ring-red-500 focus:ring-offset-2 dark:focus:ring-offset-gray-800 min-h-[44px]"
                                on:click=do_confirmed_delete
                            >
                                "Delete"
                            </button>
                        </div>
                    </div>
                </div>
            })}
        </div>

        // Activity sidebar
        {move || show_activity.get().then(|| view! {
            <div class="w-72 brutal-border border-l surface overflow-y-auto transition-all duration-200">
                <div class="px-4 py-3 border-b border-gray-200 flex items-center justify-between">
                    <h3 class="text-label font-mono text-gray-900">"Activity"</h3>
                    <button
                        class="p-1 text-gray-400 hover:text-gray-600 rounded"
                        on:click=move |_| set_show_activity.set(false)
                        aria-label="Close activity panel"
                    >
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" /></svg>
                    </button>
                </div>
                <div class="p-4 space-y-3">
                    <For
                        each=move || activity_entries.get()
                        key=|e| format!("{}{}", e.timestamp.clone(), e.path.clone())
                        let:entry
                    >
                        {
                            let action = entry.action.clone();
                            let entry_path = entry.path.clone();
                            let entry_ts = entry.timestamp.clone();
                            let file_name = entry_path.rsplit('/').next().unwrap_or(&entry_path).to_string();
                            let ts_display = if entry_ts.len() >= 19 { entry_ts[..19].to_string() } else { entry_ts };
                            view! {
                                <div class="flex items-start gap-2">
                                    <span class="text-base mt-0.5 shrink-0 font-mono">
                                        {match action.as_str() {
                                            "upload" => "\u{2191}",
                                            "delete" => "\u{2715}",
                                            "create_folder" => "\u{2192}",
                                            "copy" => "\u{2295}",
                                            "move" => "\u{2192}",
                                            _ => "\u{2022}",
                                        }}
                                    </span>
                                    <div class="min-w-0">
                                        <div class="text-sm font-mono text-gray-900 truncate" title=entry_path.clone()>
                                            {file_name}
                                        </div>
                                        <div class="text-xs text-gray-500 font-mono">
                                            {action} " " {ts_display}
                                        </div>
                                    </div>
                                </div>
                            }
                        }
                    </For>
                    {move || activity_entries.with(Vec::is_empty).then(|| view! {
                        <div class="text-sm text-gray-500 text-center py-4">"No recent activity"</div>
                    })}
                </div>
            </div>
        })}
        </div>
    }
}
