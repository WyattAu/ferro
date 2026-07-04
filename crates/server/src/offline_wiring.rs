//! Offline-first wiring: centralized initialization and helpers for the offline subsystem.
//!
//! This module consolidates the offline queue, content cache, connection monitor,
//! and reconciler setup that is otherwise spread across `main.rs` and `webdav.rs`.
//!
//! CLI flags: `--offline-cache-dir`, `--offline-queue-size`

use crate::AppState;
use std::path::Path;
use std::sync::Arc;
use tracing::info;
use tracing::warn;

pub use ferro_server_integrations::offline_wiring::{
    OfflineConfig, cache_content, clear_queue, connection_state, get_cached_content, is_online,
    pending_count, queue_copy, queue_create_collection, queue_delete, queue_move, queue_put,
    spawn_reconnection_listener, sync_pending_ops,
};

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
            let queue = Arc::new(ferro_offline::change_queue::SqliteChangeQueue::new(
                db_handle,
            ));
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
