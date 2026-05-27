use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use chrono::Utc;
use ferro_core::persistence::SnapshotStore as PersistenceSnapshotStore;
use serde::Deserialize;
use serde::{Deserialize as SerdeDeserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use crate::AppState;
use crate::api_error::ApiError;

/// A filesystem snapshot containing metadata for all files at a point in time.
#[derive(Debug, Clone, Serialize, SerdeDeserialize)]
pub struct Snapshot {
    pub id: String,
    pub created_at: String,
    pub description: String,
    pub entries: Vec<common::metadata::FileMetadata>,
    pub entry_count: usize,
}

/// In-memory snapshot store with optional SQLite persistence.
pub struct SnapshotStore {
    snapshots: Arc<RwLock<Vec<Snapshot>>>,
    max_snapshots: usize,
    persistence: Option<Arc<ferro_core::persistence::SqlitePersistence>>,
}

impl SnapshotStore {
    /// Create a new snapshot store with a maximum retention count.
    pub fn new(max_snapshots: usize) -> Self {
        Self {
            snapshots: Arc::new(RwLock::new(Vec::new())),
            max_snapshots,
            persistence: None,
        }
    }

    /// Add optional SQLite persistence.
    pub fn with_persistence(
        mut self,
        persistence: Arc<ferro_core::persistence::SqlitePersistence>,
    ) -> Self {
        self.persistence = Some(persistence);
        self
    }

    /// Create a new snapshot.
    pub async fn create(
        &self,
        description: String,
        entries: Vec<common::metadata::FileMetadata>,
    ) -> Snapshot {
        let snapshot = Snapshot {
            id: uuid::Uuid::new_v4().to_string(),
            created_at: Utc::now().to_rfc3339(),
            description: description.clone(),
            entry_count: entries.len(),
            entries: entries.clone(),
        };
        info!(
            "Snapshot created: {} ({} entries, {})",
            snapshot.id, snapshot.entry_count, snapshot.description
        );

        if let Some(ref p) = self.persistence {
            let _ = p
                .create(description, entries)
                .await
                .map_err(|e| tracing::error!(error = %e, "failed to persist snapshot to database"));
        }

        let mut snapshots = self.snapshots.write().await;
        snapshots.push(snapshot.clone());

        if snapshots.len() > self.max_snapshots {
            snapshots.remove(0);
        }

        snapshot
    }

    /// Get a snapshot by ID.
    pub async fn get(&self, id: &str) -> Option<Snapshot> {
        if let Some(ref p) = self.persistence
            && let Ok(persisted) = p.get(id).await
        {
            let entries: Vec<common::metadata::FileMetadata> =
                serde_json::from_str(&persisted.entries_json).unwrap_or_else(|e| {
                    tracing::warn!(
                        "snapshot {}: corrupt entries_json, discarding: {e}",
                        persisted.id
                    );
                    Vec::new()
                });
            return Some(Snapshot {
                id: persisted.id,
                created_at: persisted.created_at,
                description: persisted.description,
                entry_count: persisted.entry_count,
                entries,
            });
        }
        self.snapshots
            .read()
            .await
            .iter()
            .find(|s| s.id == id)
            .cloned()
    }

    /// List all snapshots.
    pub async fn list(&self) -> Vec<Snapshot> {
        if let Some(ref p) = self.persistence
            && let Ok(summaries) = p.list().await
        {
            let in_memory = self.snapshots.read().await;
            return summaries
                .into_iter()
                .map(|s| {
                    let entries = in_memory
                        .iter()
                        .find(|sn| sn.id == s.id)
                        .map(|sn| sn.entries.clone())
                        .unwrap_or_default();
                    Snapshot {
                        id: s.id,
                        created_at: s.created_at,
                        description: s.description,
                        entry_count: s.entry_count,
                        entries,
                    }
                })
                .collect();
        }
        self.snapshots.read().await.clone()
    }

    /// Delete a snapshot by ID.
    pub async fn delete(&self, id: &str) -> bool {
        if let Some(ref p) = self.persistence {
            if p.delete(id).await.is_ok() {
                let mut snapshots = self.snapshots.write().await;
                if let Some(pos) = snapshots.iter().position(|s| s.id == id) {
                    snapshots.remove(pos);
                }
                return true;
            }
            return false;
        }
        let mut snapshots = self.snapshots.write().await;
        if let Some(pos) = snapshots.iter().position(|s| s.id == id) {
            snapshots.remove(pos);
            true
        } else {
            false
        }
    }
}

/// Request body for creating a snapshot.
#[derive(Debug, Deserialize)]
pub struct CreateSnapshotRequest {
    pub description: Option<String>,
}

/// POST /api/snapshots — create a new filesystem snapshot.
pub async fn create_snapshot(
    State(state): State<AppState>,
    axum::Json(req): axum::Json<CreateSnapshotRequest>,
) -> Response {
    let entries = match state.storage.list_all("/", 1000).await {
        Ok(e) => e,
        Err(e) => {
            return ApiError::internal(
                "SNAPSHOT_LIST_FAILED",
                format!("Failed to list files: {}", e),
            );
        }
    };

    let snapshot = state
        .snapshot_store
        .create(
            req.description
                .unwrap_or_else(|| "Manual snapshot".to_string()),
            entries,
        )
        .await;

    (
        StatusCode::CREATED,
        axum::Json(serde_json::json!({
            "id": snapshot.id,
            "description": snapshot.description,
            "created_at": snapshot.created_at,
            "entry_count": snapshot.entry_count,
        })),
    )
        .into_response()
}

/// GET /api/snapshots — list all snapshots.
pub async fn list_snapshots(State(state): State<AppState>) -> Response {
    let snapshots: Vec<Snapshot> = state.snapshot_store.list().await;
    let items: Vec<serde_json::Value> = snapshots
        .iter()
        .map(|s| {
            serde_json::json!({
                "id": s.id,
                "description": s.description,
                "created_at": s.created_at,
                "entry_count": s.entry_count,
            })
        })
        .collect();
    (
        StatusCode::OK,
        axum::Json(serde_json::json!({ "snapshots": items })),
    )
        .into_response()
}

/// DELETE /api/snapshots/:id — delete a snapshot.
pub async fn delete_snapshot_by_id(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Response {
    if state.snapshot_store.delete(&id).await {
        (StatusCode::NO_CONTENT, "").into_response()
    } else {
        ApiError::not_found(ApiError::SNAPSHOT_NOT_FOUND, "Snapshot not found")
    }
}

/// POST /api/snapshots/:id/restore — restore a snapshot.
pub async fn restore_snapshot(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Response {
    let snapshot = match state.snapshot_store.get(&id).await {
        Some(s) => s,
        None => {
            return ApiError::not_found(ApiError::SNAPSHOT_NOT_FOUND, "Snapshot not found");
        }
    };

    let mut restored = 0u64;
    let mut collections_created = 0u64;
    let mut missing_content = 0u64;
    for entry in &snapshot.entries {
        if entry.is_collection {
            if !state.storage.exists(&entry.path).await.unwrap_or(false) {
                if let Err(e) = state
                    .storage
                    .create_collection(&entry.path, &entry.owner)
                    .await
                {
                    tracing::warn!(error = %e, path = %entry.path, "failed to recreate collection during snapshot restore");
                }
                collections_created += 1;
            }
        } else if state.storage.exists(&entry.path).await.unwrap_or(false) {
            restored += 1;
        } else {
            missing_content += 1;
            tracing::warn!(
                "Cannot restore {}: file deleted, content not preserved in snapshot",
                entry.path
            );
        }
    }

    info!(
        "Restored snapshot {} ({} entries, {} files intact, {} collections recreated, {} missing content)",
        id, snapshot.entry_count, restored, collections_created, missing_content
    );
    (
        StatusCode::OK,
        axum::Json(serde_json::json!({
            "id": id,
            "entries": snapshot.entry_count,
            "files_intact": restored,
            "collections_created": collections_created,
            "missing_content": missing_content,
        })),
    )
        .into_response()
}
