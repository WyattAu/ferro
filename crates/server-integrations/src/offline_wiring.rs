//! Offline-first wiring helpers for the offline subsystem.
//!
//! This module provides trait-based functions for offline queue operations,
//! caching, and connection monitoring.

use ferro_offline::change_queue::{ChangeQueueStore, OfflineError, QueuedOperation};
use ferro_offline::monitor::ConnectionState;
use tracing::{debug, info, warn};

use crate::IntegrationsState;

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

/// Check if the connection monitor is currently online.
pub fn is_online<S: IntegrationsState>(state: &S) -> bool {
    state.connection_monitor().is_online()
}

/// Get the current connection state.
pub fn connection_state<S: IntegrationsState>(state: &S) -> ConnectionState {
    state.connection_monitor().state()
}

/// Try to read content from the offline cache for a given path.
/// Returns `Some(bytes)` if the path is cached, `None` otherwise.
pub async fn get_cached_content<S: IntegrationsState>(state: &S, path: &str) -> Option<Vec<u8>> {
    let mut cache = state.offline_cache().write().await;
    cache.get(path)
}

/// Cache content for a given path (called after a successful PUT).
pub async fn cache_content<S: IntegrationsState>(state: &S, path: &str, data: Vec<u8>) {
    let mut cache = state.offline_cache().write().await;
    cache.put(path, data);
}

/// Enqueue a PUT operation for later sync when back online.
pub async fn queue_put<S: IntegrationsState>(
    state: &S,
    path: &str,
    content_hash: Option<String>,
    content_size: Option<u64>,
    owner: &str,
) -> Result<(), OfflineError> {
    let queue = match state.offline_queue() {
        Some(q) => q,
        None => {
            return Err(OfflineError::Storage(
                "Offline queue not configured".to_string(),
            ));
        }
    };
    let op = QueuedOperation::put(path, content_hash, content_size, owner);
    queue.enqueue(op).await
}

/// Enqueue a DELETE operation for later sync.
pub async fn queue_delete<S: IntegrationsState>(
    state: &S,
    path: &str,
    owner: &str,
) -> Result<(), OfflineError> {
    let queue = match state.offline_queue() {
        Some(q) => q,
        None => {
            return Err(OfflineError::Storage(
                "Offline queue not configured".to_string(),
            ));
        }
    };
    let op = QueuedOperation::delete(path, owner);
    queue.enqueue(op).await
}

/// Enqueue a MOVE operation for later sync.
pub async fn queue_move<S: IntegrationsState>(
    state: &S,
    from: &str,
    to: &str,
    owner: &str,
) -> Result<(), OfflineError> {
    let queue = match state.offline_queue() {
        Some(q) => q,
        None => {
            return Err(OfflineError::Storage(
                "Offline queue not configured".to_string(),
            ));
        }
    };
    let op = QueuedOperation::move_op(from, to, owner);
    queue.enqueue(op).await
}

/// Enqueue a COPY operation for later sync.
pub async fn queue_copy<S: IntegrationsState>(
    state: &S,
    from: &str,
    to: &str,
    owner: &str,
) -> Result<(), OfflineError> {
    let queue = match state.offline_queue() {
        Some(q) => q,
        None => {
            return Err(OfflineError::Storage(
                "Offline queue not configured".to_string(),
            ));
        }
    };
    let op = QueuedOperation::copy(from, to, owner);
    queue.enqueue(op).await
}

/// Enqueue a CREATE COLLECTION operation for later sync.
pub async fn queue_create_collection<S: IntegrationsState>(
    state: &S,
    path: &str,
    owner: &str,
) -> Result<(), OfflineError> {
    let queue = match state.offline_queue() {
        Some(q) => q,
        None => {
            return Err(OfflineError::Storage(
                "Offline queue not configured".to_string(),
            ));
        }
    };
    let op = QueuedOperation::create_collection(path, owner);
    queue.enqueue(op).await
}

/// Attempt to sync all pending offline operations.
/// Returns (synced_count, failed_count).
pub async fn sync_pending_ops<S: IntegrationsState>(state: &S) -> (u32, u32) {
    let queue = match state.offline_queue() {
        Some(q) => q,
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
                state.storage().head(&op.source_path).await.map(|_| ())
            }
            ferro_offline::change_queue::OperationType::Delete => {
                state.storage().delete(&op.source_path).await
            }
            ferro_offline::change_queue::OperationType::Move => {
                if let Some(ref dest) = op.dest_path {
                    state.storage().move_path(&op.source_path, dest).await
                } else {
                    Ok(())
                }
            }
            ferro_offline::change_queue::OperationType::Copy => {
                if let Some(ref dest) = op.dest_path {
                    state.storage().copy(&op.source_path, dest).await
                } else {
                    Ok(())
                }
            }
            ferro_offline::change_queue::OperationType::CreateCollection => state
                .storage()
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
pub fn spawn_reconnection_listener<S: IntegrationsState + 'static>(
    state: S,
    cancel: tokio_util::sync::CancellationToken,
) {
    let monitor = state.connection_monitor().clone();
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
pub async fn pending_count<S: IntegrationsState>(state: &S) -> usize {
    match state.offline_queue() {
        Some(queue) => queue.pending_count().await,
        None => 0,
    }
}

/// Clear all pending operations in the offline queue.
pub async fn clear_queue<S: IntegrationsState>(state: &S) -> Result<(), OfflineError> {
    match state.offline_queue() {
        Some(queue) => queue.clear().await,
        None => Ok(()),
    }
}
