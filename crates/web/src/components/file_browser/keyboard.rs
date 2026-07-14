use leptos::prelude::*;

use crate::api;
use crate::components::clipboard::ClipboardState;
use crate::components::command_palette::CommandPaletteState;
use crate::components::header::HeaderState;
use crate::components::theme_toggle::ThemeState;

#[allow(clippy::too_many_arguments)]
pub(crate) fn setup_keyboard_shortcuts(
    _palette_state: CommandPaletteState,
    _clipboard_state: ClipboardState,
    _theme_state: ThemeState,
    _header_state: Option<HeaderState>,
    _set_show_upload: WriteSignal<bool>,
    _set_show_new_folder: WriteSignal<bool>,
    _set_show_delete_confirm: WriteSignal<bool>,
    _set_show_move_dialog: WriteSignal<bool>,
    _set_show_copy_dialog: WriteSignal<bool>,
    _set_show_share_dialog: WriteSignal<bool>,
    _set_preview_file: WriteSignal<Option<api::FileEntry>>,
    _set_selected_paths: WriteSignal<std::collections::HashSet<String>>,
    _selected_paths: ReadSignal<std::collections::HashSet<String>>,
    _preview_file: ReadSignal<Option<api::FileEntry>>,
    _show_new_folder: ReadSignal<bool>,
    _show_upload: ReadSignal<bool>,
    _show_share_dialog: ReadSignal<bool>,
    _show_move_dialog: ReadSignal<bool>,
    _show_copy_dialog: ReadSignal<bool>,
    _show_delete_confirm: ReadSignal<bool>,
    _all_entries: ReadSignal<Vec<api::FileEntry>>,
    _do_rename: impl Fn(String) + Clone + 'static,
    _navigate: impl Fn(String) + Clone + 'static,
    _do_select_all: impl Fn() + Clone + 'static,
    _clipboard_copy_selected: impl Fn() + Clone + 'static,
    _clipboard_cut_selected: impl Fn() + Clone + 'static,
    _clipboard_paste: impl Fn() + Clone + 'static,
) {
    #[cfg(target_arch = "wasm32")]
    {
        let ps = _palette_state;
        let cs = _clipboard_state;
        let su = _set_show_upload;
        let snf = _set_show_new_folder;
        let sa = _do_select_all;
        let spf = _set_preview_file;
        let ssp = _set_selected_paths;
        let sdc = _set_show_delete_confirm;
        let cc = _clipboard_copy_selected;
        let cx = _clipboard_cut_selected;
        let cv = _clipboard_paste;
        let sm = _show_move_dialog;
        let scd = _show_copy_dialog;
        let sshd = _show_share_dialog;
        let snfolder = _show_new_folder;
        let supload = _show_upload;
        let hs = _header_state;
        let sel_paths = _selected_paths;
        let prev_file = _preview_file;
        let show_dc = _show_delete_confirm;
        let set_sm = _set_show_move_dialog;
        let set_scd = _set_show_copy_dialog;
        let set_sshd = _set_show_share_dialog;
        let ts = _theme_state.clone();
        let all_ents = _all_entries;
        let do_rename_fn = _do_rename;
        let navigate_fn = _navigate;

        if let Some(window) = web_sys::window() {
            if let Some(document) = window.document() {
                use wasm_bindgen::JsCast;
                let cb = wasm_bindgen::closure::Closure::wrap(Box::new(move |ev: web_sys::KeyboardEvent| {
                    let ctrl = ev.ctrl_key() || ev.meta_key();
                    let shift = ev.shift_key();
                    let key = ev.key();

                    let tag = ev
                        .target()
                        .and_then(|t| {
                            use wasm_bindgen::JsCast;
                            t.dyn_into::<web_sys::Element>().ok()
                        })
                        .map(|el| el.tag_name().to_lowercase())
                        .unwrap_or_default();

                    let in_input = tag == "input" || tag == "textarea" || tag == "select";

                    // Ctrl+K: Command palette (works everywhere)
                    if ctrl && (key == "k" || key == "K") && !shift {
                        ev.prevent_default();
                        ps.toggle();
                        return;
                    }

                    // Escape: close dialogs/deselect (works everywhere)
                    if key == "Escape" {
                        if ps.is_open() {
                            ps.close();
                            return;
                        }
                        if prev_file.get().is_some() {
                            spf.set(None);
                            return;
                        }
                        if snfolder.get() || supload.get() || sshd.get() || sm.get() || scd.get() || show_dc.get() {
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
                    }

                    // Shortcuts that should NOT fire when in an input field
                    if in_input {
                        return;
                    }

                    // Ctrl+D: Toggle theme
                    if ctrl && (key == "d" || key == "D") && !shift {
                        ev.prevent_default();
                        use crate::styles::dark_mode::Theme;
                        let current = ts.theme().get();
                        let next = match current {
                            Theme::Light => Theme::Dark,
                            Theme::Dark => Theme::Midnight,
                            Theme::Midnight => Theme::System,
                            Theme::System => Theme::Light,
                        };
                        ts.set_theme(next);
                        return;
                    }

                    // Ctrl+Shift+N: New note (navigate to notes page)
                    if ctrl && shift && (key == "n" || key == "N") {
                        ev.prevent_default();
                        #[cfg(target_arch = "wasm32")]
                        {
                            if let Some(window) = web_sys::window() {
                                let _ = window.location().set_href("/ui/notes");
                            }
                        }
                        return;
                    }

                    // Ctrl+N: New folder
                    if ctrl && (key == "n" || key == "N") && !shift {
                        ev.prevent_default();
                        snf.set(true);
                        return;
                    }

                    // Ctrl+U: Upload
                    if ctrl && (key == "u" || key == "U") {
                        ev.prevent_default();
                        su.set(true);
                        return;
                    }

                    // Ctrl+A: Select all
                    if ctrl && (key == "a" || key == "A") {
                        ev.prevent_default();
                        sa();
                        return;
                    }

                    // Ctrl+F: Search
                    if ctrl && (key == "f" || key == "F") {
                        ev.prevent_default();
                        if let Some(h) = hs {
                            h.open_search();
                        }
                        return;
                    }

                    // Ctrl+C: Copy
                    if ctrl && (key == "c" || key == "C") {
                        ev.prevent_default();
                        cc();
                        return;
                    }

                    // Ctrl+X: Cut
                    if ctrl && (key == "x" || key == "X") {
                        ev.prevent_default();
                        cx();
                        return;
                    }

                    // Ctrl+V: Paste
                    if ctrl && (key == "v" || key == "V") {
                        ev.prevent_default();
                        if cs.has_files() {
                            cv();
                        }
                        return;
                    }

                    // Delete / Backspace: Delete selected
                    if key == "Delete" || key == "Backspace" {
                        ev.prevent_default();
                        if !sel_paths.with(|s| s.is_empty()) {
                            sdc.set(true);
                        }
                        return;
                    }

                    // F2: Rename selected file
                    if key == "F2" {
                        ev.prevent_default();
                        let paths: Vec<String> = sel_paths.get().into_iter().collect();
                        if let Some(path) = paths.first() {
                            do_rename_fn(path.clone());
                        }
                        return;
                    }

                    // Enter: Open selected file/folder
                    if key == "Enter" {
                        ev.prevent_default();
                        let paths: Vec<String> = sel_paths.get().into_iter().collect();
                        if let Some(path) = paths.first() {
                            let entries = all_ents.get();
                            if let Some(entry) = entries.iter().find(|e| &e.path == path) {
                                if entry.is_collection {
                                    navigate_fn(path.clone());
                                } else {
                                    spf.set(Some(entry.clone()));
                                }
                            }
                        }
                        return;
                    }

                    // /: Focus search
                    if key == "/" && !ctrl && !shift {
                        ev.prevent_default();
                        if let Some(h) = hs {
                            h.open_search();
                        }
                        return;
                    }
                })
                    as Box<dyn Fn(web_sys::KeyboardEvent)>);
                let _ = document.add_event_listener_with_callback("keydown", cb.as_ref().unchecked_ref());
                std::mem::forget(cb);
            }
        }
    }
}
