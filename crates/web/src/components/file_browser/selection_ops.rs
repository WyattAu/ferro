use leptos::ev;
use leptos::prelude::*;

use crate::api;

pub(crate) fn toggle_select_mode(
    select_mode: ReadSignal<bool>,
    set_select_mode: WriteSignal<bool>,
    set_selected_paths: WriteSignal<std::collections::HashSet<String>>,
) -> impl FnMut(ev::MouseEvent) {
    move |_: ev::MouseEvent| {
        let new_mode = !select_mode.get();
        set_select_mode.set(new_mode);
        if !new_mode {
            set_selected_paths.set(std::collections::HashSet::new());
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct ToggleSelect {
    all_entries: ReadSignal<Vec<api::FileEntry>>,
    set_selected_paths: WriteSignal<std::collections::HashSet<String>>,
    last_clicked_index: ReadSignal<Option<usize>>,
    set_last_clicked_index: WriteSignal<Option<usize>>,
}

impl ToggleSelect {
    pub fn new(
        all_entries: ReadSignal<Vec<api::FileEntry>>,
        set_selected_paths: WriteSignal<std::collections::HashSet<String>>,
        last_clicked_index: ReadSignal<Option<usize>>,
        set_last_clicked_index: WriteSignal<Option<usize>>,
    ) -> Self {
        Self {
            all_entries,
            set_selected_paths,
            last_clicked_index,
            set_last_clicked_index,
        }
    }

    pub fn call(&self, path: String, index: usize, is_shift: bool, is_ctrl: bool) {
        if is_shift {
            if let Some(last) = self.last_clicked_index.get() {
                let entries = self.all_entries.get();
                let start = last.min(index);
                let end = last.max(index);
                self.set_selected_paths.update(|sel| {
                    for i in start..=end {
                        if let Some(entry) = entries.get(i) {
                            sel.insert(entry.path.clone());
                        }
                    }
                });
            } else {
                self.set_selected_paths.update(|sel| {
                    if sel.contains(&path) {
                        sel.remove(&path);
                    } else {
                        sel.insert(path);
                    }
                });
            }
        } else if is_ctrl {
            self.set_selected_paths.update(|sel| {
                if sel.contains(&path) {
                    sel.remove(&path);
                } else {
                    sel.insert(path);
                }
            });
        } else {
            self.set_selected_paths.update(|sel| {
                sel.clear();
                sel.insert(path);
            });
        }
        self.set_last_clicked_index.set(Some(index));
    }
}

pub(crate) fn do_select_all(
    all_entries: ReadSignal<Vec<api::FileEntry>>,
    selected_paths: ReadSignal<std::collections::HashSet<String>>,
    set_selected_paths: WriteSignal<std::collections::HashSet<String>>,
) {
    let entries = all_entries.get();
    let all_selected = entries.iter().all(|e| selected_paths.with(|s| s.contains(&e.path)));
    if all_selected {
        set_selected_paths.set(std::collections::HashSet::new());
    } else {
        let all: std::collections::HashSet<String> = entries.iter().map(|e| e.path.clone()).collect();
        set_selected_paths.set(all);
    }
}
