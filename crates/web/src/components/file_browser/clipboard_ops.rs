use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::api;
use crate::components::clipboard::{ClipboardAction, ClipboardState};
use crate::components::toast::ToastContext;

pub(crate) fn clipboard_copy_selected(
    selected_paths: ReadSignal<std::collections::HashSet<String>>,
    clipboard_state: ClipboardState,
) {
    let paths: Vec<String> = selected_paths.get().into_iter().collect();
    let count = paths.len();
    clipboard_state.copy_files(paths);
    ToastContext::info(format!("{} file(s) copied to clipboard", count));
}

pub(crate) fn clipboard_cut_selected(
    selected_paths: ReadSignal<std::collections::HashSet<String>>,
    clipboard_state: ClipboardState,
) {
    let paths: Vec<String> = selected_paths.get().into_iter().collect();
    let count = paths.len();
    clipboard_state.cut_files(paths);
    ToastContext::info(format!("{} file(s) cut to clipboard", count));
}

pub(crate) fn clipboard_paste(
    clipboard_state: ClipboardState,
    current_path: ReadSignal<String>,
    reload: impl FnOnce() + Clone + 'static,
) {
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
            ToastContext::success(format!("Bulk action: {} {} succeeded", succeeded, action_str));
        } else {
            ToastContext::warning(format!(
                "Bulk action: {} {} succeeded, {} failed",
                succeeded, action_str, failed
            ));
        }

        clipboard_state.clear();
        reload();
    });
}
