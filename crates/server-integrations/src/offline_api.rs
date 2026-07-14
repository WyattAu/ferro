//! Offline-first mode API endpoints.
//!
//! Provides REST API for triggering sync, checking status, listing pending
//! changes, resolving conflicts, and listing cached files.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use ferro_offline::change_queue::ChangeQueueStore;
use serde::{Deserialize, Serialize};

use crate::IntegrationsState;

/// Response body for sync status.
#[derive(Debug, Serialize)]
pub struct OfflineStatusResponse {
    pub online: bool,
    pub pending_changes: usize,
    pub cached_files: usize,
    pub last_sync: Option<String>,
    pub connection_state: String,
}

/// Response body for a pending operation.
#[derive(Debug, Serialize)]
pub struct PendingOperationResponse {
    pub id: String,
    pub operation: String,
    pub source_path: String,
    pub dest_path: Option<String>,
    pub owner: String,
    pub queued_at: String,
    pub content_hash: Option<String>,
    pub content_size: Option<u64>,
}

/// Request body for resolving a sync conflict.
#[derive(Debug, Deserialize)]
pub struct ResolveConflictRequest {
    pub resolution: String,
}

/// Entry representing a cached file.
#[derive(Debug, Serialize)]
pub struct CachedFileEntry {
    pub path: String,
    pub content_hash: String,
    pub size: u64,
    pub cached_at: String,
}

/// POST /api/offline/sync — Trigger offline sync (push local changes, pull remote).
pub async fn trigger_sync<S: IntegrationsState>(State(state): State<S>) -> Response {
    let queue = match state.offline_queue() {
        Some(q) => q,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({
                    "error": "Offline queue not configured",
                })),
            )
                .into_response();
        }
    };

    let pending = queue.pending().await;
    let pending_count = pending.len();

    if pending_count == 0 {
        return (
            StatusCode::OK,
            Json(serde_json::json!({
                "message": "No pending changes to sync",
                "synced": 0,
                "failed": 0,
            })),
        )
            .into_response();
    }

    let mut synced = 0u32;
    let mut failed = 0u32;

    for op in &pending {
        let result: Result<(), common::error::FerroError> = match op.op {
            ferro_offline::change_queue::OperationType::Put => state.storage().head(&op.source_path).await.map(|_| ()),
            ferro_offline::change_queue::OperationType::Delete => state.storage().delete(&op.source_path).await,
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
        };

        match result {
            Ok(()) => {
                let _ = queue.mark_synced(&op.id).await;
                synced += 1;
            }
            Err(e) => {
                tracing::warn!("Failed to sync offline op {}: {}", op.id, e);
                failed += 1;
            }
        }
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "message": format!("Sync complete: {} synced, {} failed", synced, failed),
            "synced": synced,
            "failed": failed,
            "total_pending": pending_count,
        })),
    )
        .into_response()
}

/// GET /api/offline/status — Get sync status.
pub async fn get_status<S: IntegrationsState>(State(state): State<S>) -> Response {
    let pending_count = match state.offline_queue() {
        Some(q) => q.pending_count().await,
        None => 0,
    };

    let cached_count = {
        let cache = state.offline_cache().read().await;
        cache.len()
    };

    let online = state.connection_monitor().is_online();
    let conn_state = state.connection_monitor().state();

    let last_sync = state.connection_monitor().last_online_at().map(|instant| {
        let elapsed = instant.elapsed();
        format!("{} seconds ago", elapsed.as_secs())
    });

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "online": online,
            "connection_state": format!("{:?}", conn_state),
            "pending_changes": pending_count,
            "cached_files": cached_count,
            "last_sync": last_sync,
            "consecutive_failures": state.connection_monitor().consecutive_failures(),
        })),
    )
        .into_response()
}

/// GET /api/offline/pending — List pending offline changes.
pub async fn list_pending<S: IntegrationsState>(State(state): State<S>) -> Response {
    let queue = match state.offline_queue() {
        Some(q) => q,
        None => {
            return (
                StatusCode::OK,
                Json(serde_json::json!({ "operations": [], "total": 0 })),
            )
                .into_response();
        }
    };

    let pending = queue.pending().await;
    let total = pending.len();

    let operations: Vec<serde_json::Value> = pending
        .into_iter()
        .map(|op| {
            serde_json::json!({
                "id": op.id,
                "operation": format!("{:?}", op.op),
                "source_path": op.source_path,
                "dest_path": op.dest_path,
                "owner": op.owner,
                "queued_at": op.queued_at.to_rfc3339(),
                "content_hash": op.content_hash,
                "content_size": op.content_size,
            })
        })
        .collect();

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "operations": operations,
            "total": total,
        })),
    )
        .into_response()
}

/// POST /api/offline/resolve/{id} — Resolve a sync conflict.
pub async fn resolve_conflict<S: IntegrationsState>(
    State(state): State<S>,
    Path(id): Path<String>,
    Json(req): Json<ResolveConflictRequest>,
) -> Response {
    let queue = match state.offline_queue() {
        Some(q) => q,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({ "error": "Offline queue not configured" })),
            )
                .into_response();
        }
    };

    match req.resolution.as_str() {
        "accept_local" | "accept_remote" | "merge" => {}
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Invalid resolution. Must be: accept_local, accept_remote, or merge"
                })),
            )
                .into_response();
        }
    }

    let pending = queue.pending().await;
    let op = pending.iter().find(|op| op.id == id);

    let Some(_op) = op else {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Operation not found" })),
        )
            .into_response();
    };

    if req.resolution == "accept_local" {
        if let Err(e) = queue.mark_synced(&id).await {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response();
        }
    } else if req.resolution == "accept_remote" {
        if let Err(e) = queue.remove(&id).await {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response();
        }
    } else {
        if let Err(e) = queue.mark_synced(&id).await {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response();
        }
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "message": format!("Conflict resolved with resolution: {}", req.resolution),
            "id": id,
            "resolution": req.resolution,
        })),
    )
        .into_response()
}

/// GET /api/offline/cached — List locally cached files.
pub async fn list_cached<S: IntegrationsState>(State(state): State<S>) -> Response {
    let cache = state.offline_cache().read().await;
    let total_size = cache.total_size();
    let paths = cache.paths();

    let entries: Vec<serde_json::Value> = paths
        .into_iter()
        .map(|path| {
            let hash = cache.content_hash(&path).unwrap_or_default();
            serde_json::json!({
                "path": path,
                "content_hash": hash,
            })
        })
        .collect();

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "files": entries,
            "total_files": entries.len(),
            "total_size_bytes": total_size,
        })),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    // TODO: Create a test mock implementing IntegrationsState for these tests.
    // AppState::in_memory() is not available in this crate.
}
