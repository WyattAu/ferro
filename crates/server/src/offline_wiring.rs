//! Offline-first wiring: centralized initialization and helpers for the offline subsystem.
//!
//! This module consolidates the offline queue, content cache, connection monitor,
//! and reconciler setup that is otherwise spread across `main.rs` and `webdav.rs`.
//!
//! CLI flags: `--offline-cache-dir`, `--offline-queue-size`

use crate::AppState;
use ferro_offline::change_queue::{ChangeQueueStore, SqliteChangeQueue};
use ferro_offline::monitor::ConnectionState;
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Offline configuration derived from CLI flags.
#[derive(Debug, Clone)]
pub struct OfflineConfig {
    /// Directory for the offline queue database and content cache.
    pub cache_dir: String,
    /// Maximum number of pending queue operations before rejecting writes.
    pub queue_size: usize,
}

impl OfflineConfig {
    /// Build from CLI args; returns `None` if `--offline-cache-dir` was not set.
    pub fn from_args(cache_dir: Option<String>, queue_size: usize) -> Option<Self> {
        cache_dir.map(|dir| Self {
            cache_dir: dir,
            queue_size,
        })
    }
}

/// Initialize the offline subsystem: create the queue DB, content cache, and
/// return a fully configured `AppState` with offline fields populated.
pub fn init_offline(state: &mut AppState, config: &OfflineConfig) {
    // Ensure the cache directory exists
    if let Err(e) = std::fs::create_dir_all(&config.cache_dir) {
        warn!(
            "Failed to create offline cache dir {}: {}",
            config.cache_dir, e
        );
        return;
    }

    let queue_db_path = Path::new(&config.cache_dir).join("offline_queue.db");
    let queue_db_url = format!("sqlite:{}", queue_db_path.display());

    match rusqlite::Connection::open(&queue_db_path) {
        Ok(conn) => {
            let db_handle = Arc::new(std::sync::Mutex::new(conn));
            let queue = Arc::new(SqliteChangeQueue::new(db_handle));
            if let Err(e) = queue.init() {
                warn!("Failed to init offline queue table: {}", e);
            }
            info!("Offline change queue initialized at {}", queue_db_url);
            *state = state
                .clone()
                .with_offline_queue(queue)
                .with_offline_cache_size(config.queue_size as u64 * 1024);
        }
        Err(e) => {
            warn!(
                "Failed to open offline queue DB: {}, offline queue disabled",
                e
            );
        }
    }
}

/// Check if the connection monitor is currently online.
pub fn is_online(state: &AppState) -> bool {
    state.connection_monitor.is_online()
}

/// Get the current connection state.
pub fn connection_state(state: &AppState) -> ConnectionState {
    state.connection_monitor.state()
}

/// Try to read content from the offline cache for a given path.
/// Returns `Some(bytes)` if the path is cached, `None` otherwise.
pub async fn get_cached_content(state: &AppState, path: &str) -> Option<Vec<u8>> {
    let mut cache = state.offline_cache.write().await;
    cache.get(path)
}

/// Cache content for a given path (called after a successful PUT).
pub async fn cache_content(state: &AppState, path: &str, data: Vec<u8>) {
    let mut cache = state.offline_cache.write().await;
    cache.put(path, data);
}

/// Enqueue a PUT operation for later sync when back online.
pub async fn queue_put(
    state: &AppState,
    path: &str,
    content_hash: Option<String>,
    content_size: Option<u64>,
    owner: &str,
) -> Result<(), ferro_offline::change_queue::OfflineError> {
    let queue = match state.offline_queue {
        Some(ref q) => q,
        None => {
            return Err(ferro_offline::change_queue::OfflineError::Storage(
                "Offline queue not configured".to_string(),
            ));
        }
    };
    let op =
        ferro_offline::change_queue::QueuedOperation::put(path, content_hash, content_size, owner);
    queue.enqueue(op).await
}

/// Enqueue a DELETE operation for later sync.
pub async fn queue_delete(
    state: &AppState,
    path: &str,
    owner: &str,
) -> Result<(), ferro_offline::change_queue::OfflineError> {
    let queue = match state.offline_queue {
        Some(ref q) => q,
        None => {
            return Err(ferro_offline::change_queue::OfflineError::Storage(
                "Offline queue not configured".to_string(),
            ));
        }
    };
    let op = ferro_offline::change_queue::QueuedOperation::delete(path, owner);
    queue.enqueue(op).await
}

/// Enqueue a MOVE operation for later sync.
pub async fn queue_move(
    state: &AppState,
    from: &str,
    to: &str,
    owner: &str,
) -> Result<(), ferro_offline::change_queue::OfflineError> {
    let queue = match state.offline_queue {
        Some(ref q) => q,
        None => {
            return Err(ferro_offline::change_queue::OfflineError::Storage(
                "Offline queue not configured".to_string(),
            ));
        }
    };
    let op = ferro_offline::change_queue::QueuedOperation::move_op(from, to, owner);
    queue.enqueue(op).await
}

/// Enqueue a COPY operation for later sync.
pub async fn queue_copy(
    state: &AppState,
    from: &str,
    to: &str,
    owner: &str,
) -> Result<(), ferro_offline::change_queue::OfflineError> {
    let queue = match state.offline_queue {
        Some(ref q) => q,
        None => {
            return Err(ferro_offline::change_queue::OfflineError::Storage(
                "Offline queue not configured".to_string(),
            ));
        }
    };
    let op = ferro_offline::change_queue::QueuedOperation::copy(from, to, owner);
    queue.enqueue(op).await
}

/// Enqueue a CREATE COLLECTION operation for later sync.
pub async fn queue_create_collection(
    state: &AppState,
    path: &str,
    owner: &str,
) -> Result<(), ferro_offline::change_queue::OfflineError> {
    let queue = match state.offline_queue {
        Some(ref q) => q,
        None => {
            return Err(ferro_offline::change_queue::OfflineError::Storage(
                "Offline queue not configured".to_string(),
            ));
        }
    };
    let op = ferro_offline::change_queue::QueuedOperation::create_collection(path, owner);
    queue.enqueue(op).await
}

/// Attempt to sync all pending offline operations.
/// Returns (synced_count, failed_count).
pub async fn sync_pending_ops(state: &AppState) -> (u32, u32) {
    let queue = match state.offline_queue {
        Some(ref q) => q,
        None => return (0, 0),
    };

    let pending = queue.pending().await;
    if pending.is_empty() {
        return (0, 0);
    }

    info!("Syncing {} pending offline operations", pending.len());
    let mut synced = 0u32;
    let mut failed = 0u32;

    for op in &pending {
        let result: Result<(), common::error::FerroError> = match op.op {
            ferro_offline::change_queue::OperationType::Put => {
                state.storage.head(&op.source_path).await.map(|_| ())
            }
            ferro_offline::change_queue::OperationType::Delete => {
                state.storage.delete(&op.source_path).await
            }
            ferro_offline::change_queue::OperationType::Move => {
                if let Some(ref dest) = op.dest_path {
                    state.storage.move_path(&op.source_path, dest).await
                } else {
                    Ok(())
                }
            }
            ferro_offline::change_queue::OperationType::Copy => {
                if let Some(ref dest) = op.dest_path {
                    state.storage.copy(&op.source_path, dest).await
                } else {
                    Ok(())
                }
            }
            ferro_offline::change_queue::OperationType::CreateCollection => state
                .storage
                .create_collection(&op.source_path, &op.owner)
                .await
                .map(|_| ()),
            _ => {
                warn!("Unhandled offline operation type: {:?}", op.op);
                Ok(())
            }
        };

        match result {
            Ok(()) => {
                let _ = queue.mark_synced(&op.id).await;
                synced += 1;
            }
            Err(e) => {
                warn!("Failed to sync offline op {}: {}", op.id, e);
                failed += 1;
            }
        }
    }

    info!(
        "Offline sync complete: {} synced, {} failed",
        synced, failed
    );
    (synced, failed)
}

/// Spawn the reconnection listener task.
///
/// When the `ConnectionMonitor` transitions to Online, syncs all pending
/// offline operations. Runs until `cancel` is signalled.
pub fn spawn_reconnection_listener(state: AppState, cancel: tokio_util::sync::CancellationToken) {
    let monitor = state.connection_monitor.clone();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                new_state = monitor.wait_for_change() => {
                    if new_state == ConnectionState::Online {
                        info!("Connection restored — syncing offline queue");
                        let (synced, failed) = sync_pending_ops(&state).await;
                        debug!("Reconnection sync: {} synced, {} failed", synced, failed);
                    }
                }
                _ = cancel.cancelled() => {
                    tracing::info!("Offline reconnection listener shutting down");
                    break;
                }
            }
        }
    });
}

/// Get the number of pending operations in the offline queue.
pub async fn pending_count(state: &AppState) -> usize {
    match state.offline_queue {
        Some(ref queue) => queue.pending_count().await,
        None => 0,
    }
}

/// Clear all pending operations in the offline queue.
pub async fn clear_queue(
    state: &AppState,
) -> Result<(), ferro_offline::change_queue::OfflineError> {
    match state.offline_queue {
        Some(ref queue) => queue.clear().await,
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AppState;

    #[test]
    fn test_offline_config_from_args() {
        assert!(OfflineConfig::from_args(None, 50000).is_none());
        let config =
            OfflineConfig::from_args(Some("/tmp/ferro-offline".to_string()), 10000).unwrap();
        assert_eq!(config.cache_dir, "/tmp/ferro-offline");
        assert_eq!(config.queue_size, 10000);
    }

    #[tokio::test]
    async fn test_pending_count_no_queue() {
        let state = AppState::in_memory();
        assert_eq!(pending_count(&state).await, 0);
    }

    #[tokio::test]
    async fn test_clear_queue_no_queue() {
        let state = AppState::in_memory();
        assert!(clear_queue(&state).await.is_ok());
    }

    #[tokio::test]
    async fn test_queue_put_no_queue() {
        let state = AppState::in_memory();
        let result = queue_put(&state, "/f.txt", None, None, "u").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_cached_content_empty() {
        let state = AppState::in_memory();
        assert!(get_cached_content(&state, "/missing.txt").await.is_none());
    }

    #[test]
    fn test_is_online_default() {
        let state = AppState::in_memory();
        assert!(is_online(&state));
    }
}
