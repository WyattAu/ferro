//! Search indexer that runs asynchronously.
//!
//! Design decision: Search index failures are logged but never propagated to the caller.
//! This ensures that file operations (PUT, DELETE, etc.) never fail due to search issues.
//! The index will eventually heal itself on the next successful operation.

use crate::AppState;
use common::metadata::FileMetadata;
use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use tracing::{debug, info, warn};

/// Index a single file's metadata immediately after PUT/create.
/// This is called by the WebDAV handler on successful write operations.
pub async fn index_file(state: &AppState, metadata: &FileMetadata) {
    let Some(search_lock) = &state.search else {
        return;
    };

    if metadata.is_collection {
        return; // Don't index collections
    }

    let mut engine = search_lock.write().await;
    if let Err(e) = engine.index_metadata(metadata) {
        warn!("Auto-index: failed to index {}: {}", metadata.path, e);
        return;
    }
    if let Err(e) = engine.commit() {
        warn!(
            "Auto-index: failed to commit after indexing {}: {}",
            metadata.path, e
        );
        return;
    }
    debug!("Auto-indexed: {}", metadata.path);
}

/// Index a single file with content immediately after PUT.
/// For text files, we index the content; for binary, just metadata.
pub async fn index_file_with_content(state: &AppState, metadata: &FileMetadata, content: &[u8]) {
    let Some(search_lock) = &state.search else {
        return;
    };

    if metadata.is_collection {
        return;
    }

    let mut engine = search_lock.write().await;

    // Only index content for text-like MIME types
    let is_text = metadata.mime_type.starts_with("text/")
        || metadata.mime_type == "application/json"
        || metadata.mime_type == "application/xml"
        || metadata.mime_type == "application/javascript"
        || metadata.mime_type == "application/x-yaml";

    if is_text {
        if let Ok(content_str) = std::str::from_utf8(content) {
            // Truncate very large files to avoid memory issues
            let truncated = &content_str[..content_str.len().min(1_000_000)];
            if let Err(e) = engine.index_content(metadata, truncated) {
                warn!(
                    "Auto-index: failed to index content for {}: {}",
                    metadata.path, e
                );
                return;
            }
        } else if let Err(e) = engine.index_metadata(metadata) {
            warn!("Auto-index: failed to index {}: {}", metadata.path, e);
            return;
        }
    } else if let Err(e) = engine.index_metadata(metadata) {
        warn!("Auto-index: failed to index {}: {}", metadata.path, e);
        return;
    }

    if let Err(e) = engine.commit() {
        warn!(
            "Auto-index: failed to commit after indexing {}: {}",
            metadata.path, e
        );
        return;
    }
    debug!("Auto-indexed with content: {}", metadata.path);
}

/// Remove a file from the search index immediately after DELETE.
pub async fn remove_file(state: &AppState, path: &str) {
    let Some(search_lock) = &state.search else {
        return;
    };

    let mut engine = search_lock.write().await;
    if let Err(e) = engine.remove(path) {
        warn!("Auto-index: failed to remove {}: {}", path, e);
        return;
    }
    if let Err(e) = engine.commit() {
        warn!(
            "Auto-index: failed to commit after removing {}: {}",
            path, e
        );
        return;
    }
    debug!("Auto-removed from index: {}", path);
}

pub fn spawn_indexer(
    state: Arc<AppState>,
    interval_secs: u64,
    cancel: tokio_util::sync::CancellationToken,
) {
    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(interval_secs));

        tokio::time::sleep(Duration::from_secs(5)).await;
        if !cancel.is_cancelled() {
            index_storage(&state).await;
        }

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if !cancel.is_cancelled() {
                        index_storage(&state).await;
                    }
                }
                _ = cancel.cancelled() => {
                    tracing::info!("Background indexer shutting down");
                    break;
                }
            }
        }
    });

    info!("Background indexer started (interval: {}s)", interval_secs);
}

async fn index_storage(state: &AppState) {
    let Some(search_lock) = &state.search else {
        return;
    };

    let entries = match state.storage.list_all("/", 100).await {
        Ok(entries) => entries,
        Err(e) => {
            warn!("Indexer: failed to list storage: {}", e);
            return;
        }
    };

    let mut engine = search_lock.write().await;

    for meta in &entries {
        if let Err(e) = engine.index_metadata(meta) {
            debug!("Indexer: failed to index {}: {}", meta.path, e);
        }
    }

    if let Err(e) = engine.commit() {
        warn!("Indexer: failed to commit: {}", e);
    }

    debug!("Indexer: processed {} entries", entries.len());
}
