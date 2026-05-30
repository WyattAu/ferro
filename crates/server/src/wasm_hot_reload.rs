use crate::AppState;
use std::sync::Arc;
use tracing::{info, warn};

pub fn spawn_wasm_hot_reload_watcher(
    state: Arc<AppState>,
    workers_dir: std::path::PathBuf,
    cancel: tokio_util::sync::CancellationToken,
) {
    if state.wasm_runtime.is_none() {
        return;
    }

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<notify::Event>();

    let mut watcher = match notify::RecommendedWatcher::new(
        move |res: Result<notify::Event, notify::Error>| {
            if let Ok(event) = res {
                let _ = tx.send(event);
            }
        },
        notify::Config::default(),
    ) {
        Ok(w) => w,
        Err(e) => {
            warn!("Failed to create file watcher for WASM hot-reload: {}", e);
            return;
        }
    };

    if let Err(e) = watcher.watch(&workers_dir, notify::RecursiveMode::NonRecursive) {
        warn!(
            "Failed to watch workers directory {:?} for hot-reload: {}",
            workers_dir, e
        );
        return;
    }

    tokio::spawn(async move {
        info!(
            "WASM hot-reload watcher started, watching {:?}",
            workers_dir
        );

        loop {
            tokio::select! {
                Some(event) = rx.recv() => {
                    handle_fs_event(&state, &event);
                }
                _ = cancel.cancelled() => {
                    info!("WASM hot-reload watcher shutting down");
                    break;
                }
            }
        }
    });
}

fn handle_fs_event(state: &AppState, event: &notify::Event) {
    use notify::EventKind;

    let is_create_or_modify = matches!(
        event.kind,
        EventKind::Create(_) | EventKind::Modify(_)
    );

    if !is_create_or_modify {
        return;
    }

    for path in &event.paths {
        if path.extension().map(|e| e == "wasm").unwrap_or(false) {
            let filename = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            info!(
                "WASM hot-reload: detected change to {}",
                filename
            );

            if let Some(runtime) = &state.wasm_runtime {
                let path_str = path.to_string_lossy().to_string();
                match runtime.reload_module(&path_str) {
                    Ok(()) => info!(
                        "WASM hot-reload: module {} reloaded successfully",
                        filename
                    ),
                    Err(e) => warn!(
                        "WASM hot-reload: module {} failed to reload: {}",
                        filename, e
                    ),
                }
            }
        }
    }
}
