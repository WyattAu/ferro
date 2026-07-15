use leptos::prelude::*;

use crate::api;
use crate::components::command_palette::{Command, CommandPaletteState};
use crate::components::header::HeaderState;
use crate::components::theme_toggle::ThemeState;
use crate::styles::dark_mode::Theme;

use super::clipboard_ops;
use super::selection_ops;
use super::types::ViewMode;

#[allow(clippy::too_many_arguments)]
pub(crate) fn register_commands(
    palette_state: CommandPaletteState,
    theme_state: ThemeState,
    header_state: Option<HeaderState>,
    set_show_upload: WriteSignal<bool>,
    set_show_new_folder: WriteSignal<bool>,
    set_show_delete_confirm: WriteSignal<bool>,
    set_show_activity: WriteSignal<bool>,
    set_preview_file: WriteSignal<Option<api::FileEntry>>,
    set_view_mode: WriteSignal<ViewMode>,
    view_mode: ReadSignal<ViewMode>,
    all_entries: ReadSignal<Vec<api::FileEntry>>,
    selected_paths: ReadSignal<std::collections::HashSet<String>>,
    set_selected_paths: WriteSignal<std::collections::HashSet<String>>,
    clipboard_state: crate::components::clipboard::ClipboardState,
    load_directory: impl Fn(String) + Clone + Send + Sync + 'static,
    current_path: ReadSignal<String>,
) {
    let do_select_all = {
        move || {
            selection_ops::do_select_all(all_entries, selected_paths, set_selected_paths);
        }
    };

    let clipboard_copy_selected = {
        move || {
            clipboard_ops::clipboard_copy_selected(selected_paths, clipboard_state);
        }
    };

    let clipboard_cut_selected = {
        move || {
            clipboard_ops::clipboard_cut_selected(selected_paths, clipboard_state);
        }
    };

    let clipboard_paste_fn = {
        move |_: ()| {
            clipboard_ops::clipboard_paste(clipboard_state, current_path, || {});
        }
    };

    let ts = theme_state.clone();
    let ld = load_directory.clone();
    Effect::new(move |_| {
        let ts = ts.clone();
        let load_dir = ld.clone();
        let commands = vec![
            Command {
                id: "upload-file".to_string(),
                label: crate::t!("cmd.upload_file").to_string(),
                shortcut: Some("Ctrl+U".to_string()),
                action: Callback::new(move |_| set_show_upload.set(true)),
            },
            Command {
                id: "new-folder".to_string(),
                label: crate::t!("cmd.new_folder").to_string(),
                shortcut: Some("Ctrl+N".to_string()),
                action: Callback::new(move |_| set_show_new_folder.set(true)),
            },
            Command {
                id: "go-home".to_string(),
                label: crate::t!("cmd.go_home").to_string(),
                shortcut: None,
                action: Callback::new(move |_| load_dir("/".to_string())),
            },
            Command {
                id: "go-trash".to_string(),
                label: crate::t!("cmd.go_trash").to_string(),
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
                label: crate::t!("cmd.toggle_dark_mode").to_string(),
                shortcut: None,
                action: Callback::new(move |_| {
                    let current = ts.theme().get();
                    let next = match current {
                        Theme::Light => Theme::Dark,
                        Theme::Dark => Theme::Midnight,
                        Theme::Midnight => Theme::SolarizedLight,
                        Theme::SolarizedLight => Theme::SolarizedDark,
                        Theme::SolarizedDark => Theme::Nord,
                        Theme::Nord => Theme::TokyoNight,
                        Theme::TokyoNight => Theme::Dracula,
                        Theme::Dracula => Theme::HighContrast,
                        Theme::HighContrast => Theme::Sepia,
                        Theme::Sepia => Theme::Forest,
                        Theme::Forest => Theme::Ocean,
                        Theme::Ocean => Theme::System,
                        Theme::System => Theme::Light,
                        Theme::Custom => Theme::Light,
                    };
                    ts.set_theme(next);
                }),
            },
            Command {
                id: "select-all".to_string(),
                label: crate::t!("cmd.select_all").to_string(),
                shortcut: Some("Ctrl+A".to_string()),
                action: Callback::new(move |_| do_select_all()),
            },
            Command {
                id: "delete-selected".to_string(),
                label: crate::t!("cmd.delete_selected").to_string(),
                shortcut: Some("Del".to_string()),
                action: Callback::new(move |_| {
                    if !selected_paths.with(|s| s.is_empty()) {
                        set_show_delete_confirm.set(true);
                    }
                }),
            },
            Command {
                id: "open-preview".to_string(),
                label: crate::t!("cmd.open_file_preview").to_string(),
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
                label: crate::t!("cmd.search_files").to_string(),
                shortcut: Some("Ctrl+F".to_string()),
                action: Callback::new(move |_| {
                    if let Some(hs) = header_state {
                        hs.open_search();
                    }
                }),
            },
            Command {
                id: "toggle-activity".to_string(),
                label: crate::t!("cmd.toggle_activity").to_string(),
                shortcut: None,
                action: Callback::new(move |_| {
                    set_show_activity.update(|v| *v = !*v);
                }),
            },
            Command {
                id: "copy-selected".to_string(),
                label: crate::t!("cmd.copy_selected").to_string(),
                shortcut: Some("Ctrl+C".to_string()),
                action: Callback::new(move |_| clipboard_copy_selected()),
            },
            Command {
                id: "cut-selected".to_string(),
                label: crate::t!("cmd.cut_selected").to_string(),
                shortcut: Some("Ctrl+X".to_string()),
                action: Callback::new(move |_| clipboard_cut_selected()),
            },
            Command {
                id: "paste-files".to_string(),
                label: crate::t!("cmd.paste_files").to_string(),
                shortcut: Some("Ctrl+V".to_string()),
                action: Callback::new(move |_| clipboard_paste_fn(())),
            },
            Command {
                id: "toggle-view".to_string(),
                label: crate::t!("cmd.toggle_view").to_string(),
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
}
