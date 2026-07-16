use crate::t;
use leptos::ev;
use leptos::html;
use leptos::prelude::*;
use leptos::task::spawn_local;

mod breadcrumb;
mod clipboard_ops;
mod commands;
mod keyboard;
mod selection_ops;
mod toolbar;
mod types;

pub(crate) use types::{BrowserTab, ViewMode};

use self::breadcrumb::Breadcrumb;
use self::toolbar::Toolbar;

use crate::api;
use crate::components::activity_sidebar::ActivitySidebar;
use crate::components::bulk_action_bar::BulkActionBar;
use crate::components::clipboard::use_clipboard_state;
use crate::components::command_palette::use_command_palette_state;
use crate::components::delete_confirm::DeleteConfirmDialog;
use crate::components::drag_hint::DragHint;
use crate::components::dual_pane::DualPane;
use crate::components::empty_state::EmptyState;
use crate::components::file_preview::FilePreview;
use crate::components::file_row::FileRow;
use crate::components::graph_view::GraphView;
use crate::components::grid_view::GridView;
use crate::components::header::use_header_state;
use crate::components::keyboard_shortcuts_help::KeyboardShortcutsHelp;
use crate::components::new_folder_dialog::NewFolderDialog;
use crate::components::path_dialog::PathDialog;
use crate::components::scroll_sentinel::ScrollSentinel;
use crate::components::share_dialog::ShareDialog;
use crate::components::skeleton::{SkeletonFavorites, SkeletonGrid, SkeletonList, SkeletonRecent};
use crate::components::smart_collections_sidebar::SmartCollectionsSidebar;
use crate::components::theme_toggle::use_theme_state;
use crate::components::toast::ToastContext;
use crate::components::upload_dialog::UploadDialog;
use crate::components::version_history::VersionHistory;
use crate::utils::device::use_is_mobile;

#[component]
pub fn FileBrowser(initial_path: String) -> impl IntoView {
    let initial = initial_path.clone();
    let (current_path, set_current_path) = signal(initial_path);
    let (all_entries, set_all_entries) = signal(vec![]);
    let (display_count, set_display_count) = signal(50usize);
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal(None::<String>);
    let (show_new_folder, set_show_new_folder) = signal(false);
    let (show_upload, set_show_upload) = signal(false);
    let (upload_drag, set_upload_drag) = signal(false);
    let (show_share_dialog, set_show_share_dialog) = signal(false);
    let (preview_file, set_preview_file) = signal(None::<api::FileEntry>);
    let (active_tab, set_active_tab) = signal(BrowserTab::Files);
    let (favorites, set_favorites) = signal::<Vec<String>>(vec![]);
    let (recent_files, set_recent_files) = signal::<Vec<api::FileEntry>>(vec![]);
    let (favorites_loading, set_favorites_loading) = signal(false);
    let (recent_loading, set_recent_loading) = signal(false);

    let (selected_paths, set_selected_paths) = signal(std::collections::HashSet::<String>::new());
    let selected_paths_signal = selected_paths;
    let favorites_signal = favorites;
    let (select_mode, set_select_mode) = signal(false);
    let (last_clicked_index, set_last_clicked_index) = signal(None::<usize>);

    let (show_activity, set_show_activity) = signal(false);
    let (show_smart_collections, set_show_smart_collections) = signal(false);
    let (show_version_history, set_show_version_history) = signal(false);
    let (version_history_path, set_version_history_path) = signal(String::new());
    let (show_delete_confirm, set_show_delete_confirm) = signal(false);
    // Move dialog signals (owned here, passed to PathDialog)
    let (show_move_dialog, set_show_move_dialog) = signal(false);
    let (move_source, set_move_source) = signal(String::new());
    let (move_dest, set_move_dest) = signal(String::new());
    // Copy dialog signals (owned here, passed to PathDialog)
    let (show_copy_dialog, set_show_copy_dialog) = signal(false);
    let (copy_source, set_copy_source) = signal(String::new());
    let (copy_dest, set_copy_dest) = signal(String::new());

    let (view_mode, set_view_mode) = signal(ViewMode::List);

    let (locks_state, set_locks_state) = signal(std::collections::HashMap::<String, api::LockInfo>::new());

    let clipboard_state = use_clipboard_state();
    let palette_state = use_command_palette_state();
    let theme_state = use_theme_state();
    let header_state = use_header_state();

    let (show_rename_dialog, set_show_rename_dialog) = signal(false);
    let (rename_source, set_rename_source) = signal(String::new());
    let (rename_new_name, set_rename_new_name) = signal(String::new());

    Effect::new(move |_| {
        spawn_local(async move {
            if let Ok(prefs) = api::get_preferences().await {
                set_view_mode.set(ViewMode::from_str(&prefs.view_mode));
            }
        });
    });

    let is_mobile = use_is_mobile();

    let _toggle_view_mode = move |_: ev::MouseEvent| {
        let current = view_mode.get();
        let next = match current {
            ViewMode::List => ViewMode::Grid,
            ViewMode::Grid => ViewMode::Graph,
            ViewMode::Graph => {
                // On mobile, skip DualPane - go back to List
                if is_mobile.get() {
                    ViewMode::List
                } else {
                    ViewMode::DualPane
                }
            }
            ViewMode::DualPane => ViewMode::List,
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
                ToastContext::info(t!("toast.removed_from_favorites"));
            } else {
                let _ = api::add_favorite(&fav_path).await;
                ToastContext::info(t!("toast.added_to_favorites"));
            }
            if let Ok(paths) = api::list_favorites().await {
                set_favorites.set(paths);
            }
        });
    };

    Effect::new(move |_| {
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
    let scroll_sentinel_ref = NodeRef::<html::Div>::new();
    let scroll_container_ref = NodeRef::<html::Div>::new();
    {
        Effect::new(move |_| {
            let sentinel = scroll_sentinel_ref.get()?;
            use wasm_bindgen::JsCast;

            let callback: wasm_bindgen::closure::Closure<dyn Fn(js_sys::Array, web_sys::IntersectionObserver)> =
                wasm_bindgen::closure::Closure::new(move |entries: js_sys::Array, _: web_sys::IntersectionObserver| {
                    for i in 0..entries.length() {
                        if let Ok(entry) = entries.get(i).dyn_into::<web_sys::IntersectionObserverEntry>()
                            && entry.is_intersecting()
                        {
                            let total = all_entries.with(Vec::len);
                            let displayed = display_count.get();
                            if displayed < total {
                                set_display_count.set(displayed + 50);
                            }
                        }
                    }
                });
            let callback_fn: &js_sys::Function = callback.as_ref().unchecked_ref();

            let options = scroll_container_ref
                .get()
                .map(|el| {
                    let opts = web_sys::IntersectionObserverInit::new();
                    opts.set_root(Some(&el));
                    opts
                })
                .unwrap_or_default();

            let observer = web_sys::IntersectionObserver::new_with_options(callback_fn, &options).unwrap();
            observer.observe(&sentinel);
            // Leak callback to avoid Send/Sync issues (safe in WASM single-threaded env)
            std::mem::forget(callback);
            on_cleanup(move || {
                observer.disconnect();
            });
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

    // Load initial directory immediately on mount
    {
        let p = initial.clone();
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
    }

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
                    let _ = history.push_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(&url));
                }
            }
        }
    };

    let go_up = move |_: ev::MouseEvent| {
        let path = current_path.get();
        if path != "/" {
            let parent = path
                .rfind('/')
                .map(|i| if i == 0 { "/".to_string() } else { path[..i].to_string() })
                .unwrap_or("/".to_string());
            load_directory(parent);
        }
    };

    // Folder creation is now handled by NewFolderDialog component with on_created callback

    let do_delete = move |path: String| {
        spawn_local(async move {
            match api::delete_file(&path).await {
                Ok(()) => {
                    ToastContext::success(t!("toast.file_deleted"));
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

    // Share dialog opens via context handle (ShareDialogHandle.open_for)
    let do_share = move |_path: String| {
        set_show_share_dialog.set(true);
        // ShareDialogHandle will be available after component mounts via provide_context
    };

    let open_version_history = move |path: String| {
        set_version_history_path.set(path);
        set_show_version_history.set(true);
    };

    // do_upload_files remains here because it is also used by drag-and-drop on the container
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
                                t!("toast.upload_complete"),
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

    let load_locks = {
        let set_locks = set_locks_state;
        move || {
            let set_locks = set_locks;
            spawn_local(async move {
                match api::list_locks().await {
                    Ok(locks) => {
                        let map: std::collections::HashMap<String, api::LockInfo> =
                            locks.into_iter().map(|l| (l.path.clone(), l)).collect();
                        set_locks.set(map);
                    }
                    Err(_) => set_locks.set(std::collections::HashMap::new()),
                }
            });
        }
    };

    // Alive flag for polling loop cleanup on component unmount
    let (alive, set_alive) = signal(true);

    Effect::new(move |_| {
        load_locks();
    });

    Effect::new(move |_| {
        spawn_local(async move {
            loop {
                // Check if component is still mounted
                if !alive.get() {
                    break;
                }
                let promise = js_sys::Promise::new(&mut |resolve, _reject| {
                    let _ = web_sys::window().and_then(|w| {
                        w.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 10_000)
                            .ok()
                    });
                });
                let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
                load_locks();
            }
        });
        on_cleanup(move || {
            set_alive.set(false);
        });
    });

    let get_lock_info = move |path: &str| -> (bool, String, String) {
        let locks = locks_state.get();
        if let Some(lock) = locks.get(path) {
            (true, lock.owner.clone(), lock.expires_at.clone())
        } else {
            let mut check = path;
            while check.len() > 1 {
                check = match check.rfind('/') {
                    None => break,
                    Some(0) => break,
                    Some(i) => &check[..i],
                };
                if let Some(lock) = locks.get(check)
                    && lock.depth == "Infinity"
                {
                    return (true, lock.owner.clone(), lock.expires_at.clone());
                }
            }
            (false, String::new(), String::new())
        }
    };

    let fav_entries = move || {
        let favs = favorites.get();
        let entries = all_entries.get();
        entries
            .into_iter()
            .filter(|e| favs.contains(&e.path))
            .collect::<Vec<_>>()
    };

    let show_files_view = move || active_tab.get() == BrowserTab::Files;

    let toggle_select_mode = selection_ops::toggle_select_mode(select_mode, set_select_mode, set_selected_paths);

    let toggle_select = selection_ops::ToggleSelect::new(
        all_entries,
        set_selected_paths,
        last_clicked_index,
        set_last_clicked_index,
    );

    let do_select_all = move || {
        selection_ops::do_select_all(all_entries, selected_paths, set_selected_paths);
    };

    let select_all = move |_: ev::MouseEvent| {
        do_select_all();
    };

    let _do_bulk_delete = move |_: ev::MouseEvent| {
        set_show_delete_confirm.set(true);
    };
    let do_bulk_delete_nop = move || {
        set_show_delete_confirm.set(true);
    };

    let on_delete_confirm = Callback::new(move |_: ev::MouseEvent| {
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
    });

    let _do_bulk_download = move |_: ev::MouseEvent| {
        let paths: Vec<String> = selected_paths.get().into_iter().collect();
        for path in &paths {
            let p = path.clone();
            spawn_local(async move {
                let _ = api::download_file(&p).await;
            });
        }
    };

    let clipboard_copy_selected = move || {
        clipboard_ops::clipboard_copy_selected(selected_paths, clipboard_state);
    };

    let clipboard_cut_selected = move || {
        clipboard_ops::clipboard_cut_selected(selected_paths, clipboard_state);
    };

    let clipboard_paste = move || {
        clipboard_ops::clipboard_paste(clipboard_state, current_path, || {});
    };

    let is_entry_selected = move |path: String| -> bool { selected_paths.with(|s| s.contains(&path)) };

    // Activity loading is now handled by ActivitySidebar component

    let toggle_activity = move |_: ev::MouseEvent| {
        set_show_activity.update(|v| *v = !*v);
    };

    let toggle_smart_collections = move |_: ev::MouseEvent| {
        set_show_smart_collections.update(|v| *v = !*v);
    };

    commands::register_commands(
        palette_state,
        theme_state.clone(),
        header_state,
        set_show_upload,
        set_show_new_folder,
        set_show_delete_confirm,
        set_show_activity,
        set_preview_file,
        set_view_mode,
        view_mode,
        all_entries,
        selected_paths,
        set_selected_paths,
        clipboard_state,
        load_directory,
        current_path,
    );

    let do_rename = move |path: String| {
        let file_name = path.rsplit('/').next().unwrap_or("").to_string();
        set_rename_source.set(path.clone());
        set_rename_new_name.set(file_name);
        set_show_rename_dialog.set(true);
    };

    keyboard::setup_keyboard_shortcuts(
        palette_state,
        clipboard_state,
        theme_state,
        header_state,
        set_show_upload,
        set_show_new_folder,
        set_show_delete_confirm,
        set_show_move_dialog,
        set_show_copy_dialog,
        set_show_share_dialog,
        set_preview_file,
        set_selected_paths,
        selected_paths,
        preview_file,
        show_new_folder,
        show_upload,
        show_share_dialog,
        show_move_dialog,
        show_copy_dialog,
        show_delete_confirm,
        all_entries,
        do_rename,
        navigate,
        {
            move || {
                selection_ops::do_select_all(all_entries, selected_paths, set_selected_paths);
            }
        },
        clipboard_copy_selected,
        clipboard_cut_selected,
        clipboard_paste,
    );

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

    let on_rename_confirm = Callback::new(move |(source, new_name): (String, String)| {
        let parent = source.rsplit_once('/').map(|(p, _)| p).unwrap_or("/");
        let dest = if parent == "/" {
            format!("/{}", new_name)
        } else {
            format!("{}/{}", parent, new_name)
        };
        let s = source.clone();
        let d = dest.clone();
        spawn_local(async move {
            match api::move_file(&s, &d).await {
                Ok(()) => {
                    set_show_rename_dialog.set(false);
                    ToastContext::success(format!("Renamed to {}", new_name));
                    reload();
                }
                Err(e) => ToastContext::error(format!("Rename failed: {}", e)),
            }
        });
    });

    // PathDialog on_confirm callbacks handle the actual API calls
    let on_move_confirm = Callback::new(move |(source, dest): (String, String)| {
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
    });

    let on_copy_confirm = Callback::new(move |(source, dest): (String, String)| {
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
    });

    let on_created = Callback::new(move |_: ()| reload());
    let on_uploaded = Callback::new(move |_: ()| reload());

    // Handle drag-and-drop file move/copy between folders
    let do_drop_on_folder = move |(source_path, is_copy): (String, bool)| {
        let file_name = source_path.rsplit('/').next().unwrap_or("").to_string();
        let dest_path = current_path.get();
        let dest = if dest_path == "/" {
            format!("/{}", file_name)
        } else {
            format!("{}/{}", dest_path, file_name)
        };
        if is_copy {
            let s = source_path.clone();
            spawn_local(async move {
                match api::copy_file(&s, &dest).await {
                    Ok(()) => {
                        ToastContext::success(format!("Copied {} to {}", file_name, dest));
                        reload();
                    }
                    Err(e) => ToastContext::error(format!("Copy failed: {}", e)),
                }
            });
        } else {
            let s = source_path;
            spawn_local(async move {
                match api::move_file(&s, &dest).await {
                    Ok(()) => {
                        ToastContext::success(format!("Moved {} to {}", file_name, dest));
                        reload();
                    }
                    Err(e) => ToastContext::error(format!("Move failed: {}", e)),
                }
            });
        }
    };

    let grid_cb_navigate = Callback::new(navigate);
    let grid_cb_delete = Callback::new(do_delete);
    let grid_cb_download = Callback::new(do_download);
    let grid_cb_share = Callback::new(do_share);
    let grid_cb_preview = Callback::new(open_preview);
    let grid_cb_fav = Callback::new(do_toggle_favorite);
    let grid_cb_select = Callback::new(move |(path, idx, shift, ctrl): (String, usize, bool, bool)| {
        toggle_select.call(path, idx, shift, ctrl);
    });
    let grid_cb_copy = Callback::new(do_copy);
    let grid_cb_move = Callback::new(do_move);
    let grid_cb_rename = Callback::new(do_rename);
    let grid_cb_drop = Callback::new(do_drop_on_folder);

    view! {
       <div
           node_ref=scroll_container_ref
           role="region"
            aria-label=t!("file_list.aria")
           on:dragover=handle_drag_over
           on:dragleave=handle_drag_leave
           on:drop=handle_drop
       >
              // Toolbar - compact on mobile
              <Toolbar
                  current_path
                  go_up
                  active_tab
                  switch_tab=Callback::new(switch_tab)
                  clipboard_state
                  clipboard_paste=Callback::new(move |_: ()| clipboard_paste())
                  set_show_upload
                  set_show_new_folder
                  view_mode
                  set_view_mode
                  select_mode
                  toggle_select_mode
                  show_activity
                  toggle_activity
                  show_smart_collections
                  toggle_smart_collections
              >
                 <Breadcrumb current_path=Signal::from(current_path) navigate=Callback::new(navigate) />
             </Toolbar>

           // Drag overlay
           {move || upload_drag.get().then(|| view! {
                <div class="fixed inset-0 bg-[var(--accent)] bg-opacity-20 z-50 flex items-center justify-center pointer-events-none backdrop-blur-sm" aria-hidden="true">
                   <div class="surface brutal-border shadow-2xl p-12 text-center blob-shape float-animation">
                       <svg class="w-16 h-16 text-accent mx-auto mb-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                           <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12" />
                       </svg>
                         <p class="text-xl font-semibold text-[var(--text-primary)]">{t!("drop.overlay")}</p>
                         <p class="text-sm text-[var(--text-tertiary)] mt-1">{t!("drop.overlay_hint")}</p>
                   </div>
               </div>
           })}

            // New folder dialog (extracted component)
            <NewFolderDialog
                open=show_new_folder
                set_open=set_show_new_folder
                current_path=current_path
                on_created=on_created
            />

            // Upload dialog (extracted component)
            <UploadDialog
                open=show_upload
                set_open=set_show_upload
                current_path=current_path
                on_uploaded=on_uploaded
            />

            // Share dialog (extracted component)
            <ShareDialog
                open=show_share_dialog
                set_open=set_show_share_dialog
            />

            // Error display
            {move || error.get().map(|e| view! {
               <div class="bg-[var(--danger-subtle)] border-b border-l-4 border-l-[var(--danger)] px-6 py-3" role="alert" aria-live="assertive">
                    <div class="flex items-center justify-between">
                         <span class="text-[var(--danger)] text-sm">{t!("error.prefix")} {e}</span>
                        <button
                            class="text-[var(--danger)] hover:text-[var(--danger)] focus:outline-none focus:ring-2 focus:ring-[var(--danger)] focus:ring-offset-2 rounded p-0.5"
                            aria-label=t!("error.dismiss")
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
                            <div class="px-6 py-16 text-center text-[var(--text-tertiary)]" role="status">
                                <svg class="w-16 h-16 mx-auto mb-4" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M11.049 2.927c.3-.921 1.603-.921 1.902 0l1.519 4.674a1 1 0 00.95.69h4.915c.969 0 1.371 1.24.588 1.81l-3.976 2.888a1 1 0 00-.363 1.118l1.518 4.674c.3.922-.755 1.688-1.538 1.118l-3.976-2.888a1 1 0 00-1.176 0l-3.976 2.888c-.783.57-1.838-.197-1.538-1.118l1.518-4.674a1 1 0 00-.363-1.118l-3.976 2.888c-.784-.57-.38-1.81.588-1.81h4.914a1 1 0 00.951-.69l1.519 4.674z" />
                                </svg>
                                <div class="text-lg font-medium">{t!("empty.favorites")}</div>
                                <div class="text-sm mt-1">{t!("empty.favorites_hint")}</div>
                           </div>
                       }.into_any()
                   } else {
                       view! {
                           <table class="w-full table-fixed" role="grid">
                                <thead class="bg-[var(--bg-surface-sunken)] border-b sticky top-0">
                                    <tr>
                                        <th class="px-4 py-2 text-left text-xs font-bold uppercase font-mono text-[var(--text-tertiary)] w-10" scope="col"></th>
                                         <th class="px-4 py-2 text-left text-xs font-bold uppercase font-mono text-[var(--text-tertiary)]" scope="col">{t!("common.name")}</th>
                                         <th class="px-4 py-2 text-left text-xs font-bold uppercase font-mono text-[var(--text-tertiary)] w-24" scope="col">{t!("common.size")}</th>
                                         <th class="px-4 py-2 text-left text-xs font-bold uppercase font-mono text-[var(--text-tertiary)] w-40" scope="col">{t!("common.modified")}</th>
                                         <th class="px-4 py-2 text-right text-xs font-bold uppercase font-mono text-[var(--text-tertiary)] w-24" scope="col">{t!("common.actions")}</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    <For
                                        each=move || fav_entries()
                                       key=|entry| entry.path.clone()
                                       let:entry
                                   >
                                       {
                                           let ep = entry.path.clone();
                                           let (lk, lo, le) = get_lock_info(&ep);
                                            view! {
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
                                                    on_rename=Callback::new(do_rename)
                                                    on_drop_on_folder=Callback::new(do_drop_on_folder)
                                                    is_locked=lk
                                                    lock_owner=lo
                                                    lock_expires=le
                                                    on_version_history=Callback::new(open_version_history)
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
                            <div class="px-6 py-16 text-center text-[var(--text-tertiary)]" role="status">
                                <svg class="w-16 h-16 mx-auto mb-4" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
                                </svg>
                                <div class="text-lg font-medium">{t!("empty.recent")}</div>
                                <div class="text-sm mt-1">{t!("empty.recent_hint")}</div>
                           </div>
                       }.into_any()
                   } else {
                       view! {
                           <table class="w-full table-fixed" role="grid">
                                <thead class="bg-[var(--bg-surface-sunken)] border-b sticky top-0">
                                    <tr>
                                        <th class="px-4 py-2 text-left text-xs font-bold uppercase font-mono text-[var(--text-tertiary)] w-10" scope="col"></th>
                                         <th class="px-4 py-2 text-left text-xs font-bold uppercase font-mono text-[var(--text-tertiary)]" scope="col">{t!("common.name")}</th>
                                         <th class="px-4 py-2 text-left text-xs font-bold uppercase font-mono text-[var(--text-tertiary)] w-24" scope="col">{t!("common.size")}</th>
                                         <th class="px-4 py-2 text-left text-xs font-bold uppercase font-mono text-[var(--text-tertiary)] w-40" scope="col">{t!("common.modified")}</th>
                                         <th class="px-4 py-2 text-right text-xs font-bold uppercase font-mono text-[var(--text-tertiary)] w-24" scope="col">{t!("common.actions")}</th>
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
                                           let (lk, lo, le) = get_lock_info(&entry_path);
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
                                                    on_rename=Callback::new(do_rename)
                                                    on_drop_on_folder=Callback::new(do_drop_on_folder)
                                                    is_locked=lk
                                                    lock_owner=lo
                                                    lock_expires=le
                                                    on_version_history=Callback::new(open_version_history)
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
                                        class="rounded border text-[var(--accent)] focus:ring-[var(--border-focus)]"
                                         aria-label=t!("file_list.aria_select_all")
                                         on:click=select_all
                                     />
                                     <span class="text-xs text-[var(--text-tertiary)]">{t!("toolbar.select_all")}</span>
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
                                on_rename=grid_cb_rename
                                on_drop_on_folder=grid_cb_drop
                                locks=locks_state
                            />
                       </div>
                   }.into_any(),
                   ViewMode::Graph => view! {
                       <div class="transition-opacity duration-200">
                           <GraphView
                               entries=all_entries.get()
                               on_open_file=Callback::new(navigate)
                           />
                       </div>
                   }.into_any(),
                   ViewMode::DualPane => view! {
                       <div class="transition-opacity duration-200">
                           <DualPane
                               initial_left=Some(current_path.get())
                               initial_right=Some(current_path.get())
                           />
                       </div>
                   }.into_any(),
                   ViewMode::List => view! {
                       <div class="transition-opacity duration-200">
                           <table class="w-full table-fixed" role="grid">
                                <thead class="bg-[var(--bg-surface-sunken)] border-b sticky top-0 hidden md:table-header-group">
                                    <tr>
                                        {move || select_mode.get().then(|| view! {
                                            <th class="px-4 py-2 w-10" scope="col">
                                                <input
                                                    type="checkbox"
                                                    class="rounded border text-[var(--accent)] focus:ring-[var(--border-focus)]"
                                                     aria-label=t!("file_list.aria_select_all")
                                                    on:click=select_all
                                                />
                                            </th>
                                        })}
                                        <th class="px-4 py-2 text-left text-xs font-bold uppercase font-mono text-[var(--text-tertiary)] w-10" scope="col"></th>
                                         <th class="px-4 py-2 text-left text-xs font-bold uppercase font-mono text-[var(--text-tertiary)]" scope="col">{t!("common.name")}</th>
                                         <th class="px-4 py-2 text-left text-xs font-bold uppercase font-mono text-[var(--text-tertiary)] w-24" scope="col">{t!("common.size")}</th>
                                         <th class="px-4 py-2 text-left text-xs font-bold uppercase font-mono text-[var(--text-tertiary)] w-40 hidden lg:table-cell" scope="col">{t!("common.modified")}</th>
                                         <th class="px-4 py-2 text-right text-xs font-bold uppercase font-mono text-[var(--text-tertiary)] w-24" scope="col">{t!("common.actions")}</th>
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
                                            let sel_path = entry.path.clone();
                                            let (lk, lo, le) = get_lock_info(&entry.path);
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
                                                        toggle_select.call(path, idx, shift, ctrl);
                                                    })
                                                    on_copy=Callback::new(do_copy)
                                                    on_move=Callback::new(do_move)
                                                    on_rename=Callback::new(do_rename)
                                                    on_drop_on_folder=Callback::new(do_drop_on_folder)
                                                    is_locked=lk
                                                    lock_owner=lo
                                                    lock_expires=le
                                                    on_version_history=Callback::new(open_version_history)
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

            // Bulk action bar (extracted component)
            <BulkActionBar
                select_mode=select_mode
                selected_count=Signal::derive(move || selected_paths.with(|s| s.len()))
                on_delete=Callback::new(move |_| do_bulk_delete_nop())
                on_download=Callback::new(move |_: ()| {
                    let paths: Vec<String> = selected_paths.get().into_iter().collect();
                    for path in &paths {
                        let p = path.clone();
                        spawn_local(async move {
                            let _ = api::download_file(&p).await;
                        });
                    }
                })
                on_clear=Callback::new(move |_| set_selected_paths.set(std::collections::HashSet::new()))
            />

            // Scroll sentinel (extracted component)
            <ScrollSentinel
                total=Signal::derive(move || all_entries.with(Vec::len))
                displayed=display_count
                loading=loading
                files_tab_active=Signal::derive(show_files_view)
                sentinel_ref=scroll_sentinel_ref
            />

            // Empty state (extracted component)
            <EmptyState
                loading=loading
                files_tab_active=Signal::derive(show_files_view)
                has_error=Signal::derive(move || error.get().is_some())
                is_empty=Signal::derive(move || all_entries.with(Vec::is_empty))
                on_upload=Callback::new(move |_| set_show_upload.set(true))
            />

            // Drag hint (extracted component)
            <DragHint
                loading=loading
                has_entries=Signal::derive(move || !all_entries.with(Vec::is_empty))
                files_tab_active=Signal::derive(show_files_view)
                is_dragging=upload_drag.into()
            />

            // Move dialog (extracted PathDialog component)
            <PathDialog
                 title=t!("dialog.path.source_label")
                 action_label=t!("common.move")
                open=show_move_dialog
                set_open=set_show_move_dialog
                source=move_source
                dest=move_dest
                set_dest=set_move_dest
                on_confirm=on_move_confirm
            />

            // Copy dialog (extracted PathDialog component)
            <PathDialog
                 title=t!("common.copy")
                 action_label=t!("common.copy")
                open=show_copy_dialog
                set_open=set_show_copy_dialog
                source=copy_source
                dest=copy_dest
                set_dest=set_copy_dest
                on_confirm=on_copy_confirm
            />

            // Rename dialog (extracted PathDialog component)
            <PathDialog
                 title=t!("common.rename")
                 action_label=t!("common.rename")
                open=show_rename_dialog
                set_open=set_show_rename_dialog
                source=rename_source
                dest=rename_new_name
                set_dest=set_rename_new_name
                on_confirm=on_rename_confirm
            />

            // File preview modal
            {move || preview_file.get().map(|file| view! {
                <FilePreview
                    file=file
                    on_close=Callback::new(close_preview)
                />
            })}

            // Delete confirmation dialog (extracted component)
            <DeleteConfirmDialog
                open=show_delete_confirm
                set_open=set_show_delete_confirm
                count=Signal::derive(move || selected_paths.with(|s| s.len()))
                on_confirm=on_delete_confirm
            />
        </div>

        // Activity sidebar (extracted component)
        <ActivitySidebar
            open=show_activity
            set_open=set_show_activity
        />

        // Smart Collections sidebar (extracted component)
        <SmartCollectionsSidebar
            open=show_smart_collections
            set_open=set_show_smart_collections
        />

        // Version history panel (extracted component)
        <VersionHistory
            open=show_version_history
            set_open=set_show_version_history
            file_path=version_history_path
        />

        // Keyboard shortcuts help overlay
        <KeyboardShortcutsHelp />
    }
}
