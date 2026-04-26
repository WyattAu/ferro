use crate::AppState;
use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use tracing::{debug, info, warn};

pub fn spawn_worker_runner(state: Arc<AppState>, interval_secs: u64) {
    if state.wasm_runtime.is_none() {
        return;
    }

    let seen_files: Arc<tokio::sync::RwLock<std::collections::HashMap<String, String>>> =
        Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));

    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(interval_secs));

        loop {
            interval.tick().await;

            let Some(runtime) = &state.wasm_runtime else {
                continue;
            };

            let entries = match state.storage.list_all("/", 100).await {
                Ok(e) => e,
                Err(e) => {
                    warn!("Worker runner: failed to list files: {}", e);
                    continue;
                }
            };

            let mut seen = seen_files.write().await;

            for entry in &entries {
                if entry.is_collection {
                    continue;
                }

                let current_hash = entry.content_hash.as_str().to_string();
                let prev_hash = seen.get(&entry.path).cloned();

                if prev_hash.as_ref() == Some(&current_hash) {
                    continue;
                }

                // Skip files recently processed by the inline PUT trigger
                if state.recently_processed.contains(&entry.path) {
                    // Still record the hash so we don't re-check
                    seen.insert(entry.path.clone(), current_hash);
                    state.recently_processed.remove(&entry.path);
                    continue;
                }

                let workers = runtime.find_matching_workers(&entry.path).await;

                for worker in &workers {
                    debug!(
                        "Worker triggered: {} matches {}",
                        worker.pattern,
                        entry.path
                    );

                    let content = match state.storage.get(&entry.path).await {
                        Ok(c) => c,
                        Err(e) => {
                            warn!("Worker: failed to read {}: {}", entry.path, e);
                            continue;
                        }
                    };

                    match runtime.execute(
                        &worker.module_path,
                        &worker.function_name,
                        &content,
                        Some(worker.config.clone()),
                    ).await {
                        Ok(result) => {
                            if result.success {
                                info!(
                                    "Worker {}::{} completed for {} (fuel: {}, time: {}ms)",
                                    worker.module_path,
                                    worker.function_name,
                                    entry.path,
                                    result.fuel_consumed,
                                    result.execution_time_ms,
                                );
                            } else if let Some(err) = &result.error {
                                warn!(
                                    "Worker {}::{} failed for {}: {}",
                                    worker.module_path,
                                    worker.function_name,
                                    entry.path,
                                    err,
                                );
                            }
                        }
                        Err(e) => {
                            warn!("Worker execution error: {}", e);
                        }
                    }
                }

                seen.insert(entry.path.clone(), current_hash);
            }
        }
    });

    info!("WASM worker runner started (interval: {}s)", interval_secs);
}
