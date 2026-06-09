//! Reconciliation engine for syncing offline changes with the remote server.
//!
//! Uses the selective-sync `ConflictDetector` and sync-delta `compute_block_diff`
//! to minimize data transfer and resolve conflicts during reconnection.

use crate::change_queue::QueuedOperation;
use common::chunk::{BlockDiffRequest, BlockDiffResult, ChunkInfo, compute_block_diff};
use common::conflict::{ConflictDetector, ConflictType, SyncConflict};
use serde::{Deserialize, Serialize};

/// Result of a reconciliation pass.
#[derive(Debug, Serialize)]
pub struct ReconciliationResult {
    /// Operations successfully synced to the server.
    pub synced_ops: Vec<String>,
    /// Operations that conflicted and need manual resolution.
    pub conflicts: Vec<SyncConflict>,
    /// Total bytes uploaded during reconciliation.
    pub bytes_uploaded: u64,
    /// Total bytes downloaded during reconciliation.
    pub bytes_downloaded: u64,
    /// Duration of the reconciliation in milliseconds.
    pub duration_ms: u64,
}

/// File metadata snapshot from the server (or local).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteFileSnapshot {
    /// File path.
    pub path: String,
    /// Content hash (SHA-256 hex).
    pub content_hash: Option<String>,
    /// File size in bytes.
    pub size: u64,
    /// Last modified timestamp.
    pub modified_at: chrono::DateTime<chrono::Utc>,
    /// Whether the file exists on the server.
    pub exists: bool,
}

impl RemoteFileSnapshot {
    /// Create a snapshot for an existing file.
    pub fn existing(
        path: &str,
        content_hash: &str,
        size: u64,
        modified_at: chrono::DateTime<chrono::Utc>,
    ) -> Self {
        Self {
            path: path.to_string(),
            content_hash: Some(content_hash.to_string()),
            size,
            modified_at,
            exists: true,
        }
    }

    /// Create a snapshot for a deleted file.
    pub fn deleted(path: &str, modified_at: chrono::DateTime<chrono::Utc>) -> Self {
        Self {
            path: path.to_string(),
            content_hash: None,
            size: 0,
            modified_at,
            exists: false,
        }
    }
}

/// Reconciles local offline changes with the remote server state.
#[allow(dead_code)]
pub struct Reconciler {
    conflict_detector: ConflictDetector,
}

impl Reconciler {
    /// Create a new reconciler.
    pub fn new() -> Self {
        Self {
            conflict_detector: ConflictDetector::new(),
        }
    }

    /// Detect conflicts between local queued operations and remote file state.
    ///
    /// Returns a list of `SyncConflict` objects for any operations that
    /// conflict with the remote state. Operations without conflicts
    /// are safe to sync.
    pub fn detect_conflicts(
        &self,
        pending_ops: &[QueuedOperation],
        remote_files: &[RemoteFileSnapshot],
    ) -> Vec<SyncConflict> {
        // Build a map of remote file states
        let remote_map: std::collections::HashMap<&str, &RemoteFileSnapshot> =
            remote_files.iter().map(|f| (f.path.as_str(), f)).collect();

        let mut conflicts = Vec::new();

        for op in pending_ops {
            if op.synced {
                continue;
            }

            match &op.op {
                crate::change_queue::OperationType::Put => {
                    if let Some(remote) = remote_map.get(op.source_path.as_str()) {
                        if !remote.exists {
                            conflicts.push(SyncConflict {
                                local_path: op.source_path.clone(),
                                local_modified: op.queued_at,
                                remote_modified: remote.modified_at,
                                conflict_type: ConflictType::EditDelete,
                                resolution: None,
                            });
                        } else if let Some(remote_hash) = &remote.content_hash
                            && let Some(local_hash) = &op.content_hash
                            && local_hash != remote_hash
                        {
                            conflicts.push(SyncConflict {
                                local_path: op.source_path.clone(),
                                local_modified: op.queued_at,
                                remote_modified: remote.modified_at,
                                conflict_type: ConflictType::EditEdit,
                                resolution: None,
                            });
                        }
                    }
                }
                crate::change_queue::OperationType::Delete => {
                    if let Some(remote) = remote_map.get(op.source_path.as_str())
                        && remote.exists
                    {
                        conflicts.push(SyncConflict {
                            local_path: op.source_path.clone(),
                            local_modified: op.queued_at,
                            remote_modified: remote.modified_at,
                            conflict_type: ConflictType::DeleteEdit,
                            resolution: None,
                        });
                    }
                }
                crate::change_queue::OperationType::Move => {
                    if let Some(dest) = op
                        .dest_path
                        .as_ref()
                        .and_then(|d| remote_map.get(d.as_str()))
                        && dest.exists
                    {
                        conflicts.push(SyncConflict {
                            local_path: op.source_path.clone(),
                            local_modified: op.queued_at,
                            remote_modified: dest.modified_at,
                            conflict_type: ConflictType::EditEdit,
                            resolution: None,
                        });
                    }
                }
                crate::change_queue::OperationType::Copy => {
                    if let Some(dest) = op
                        .dest_path
                        .as_ref()
                        .and_then(|d| remote_map.get(d.as_str()))
                        && dest.exists
                    {
                        conflicts.push(SyncConflict {
                            local_path: op.source_path.clone(),
                            local_modified: op.queued_at,
                            remote_modified: dest.modified_at,
                            conflict_type: ConflictType::EditEdit,
                            resolution: None,
                        });
                    }
                }
                crate::change_queue::OperationType::CreateCollection => {
                    // Collections rarely conflict, skip
                }
            }
        }

        conflicts
    }

    /// Compute a block-level diff between local chunks and remote chunks.
    ///
    /// Returns the chunks that need to be uploaded and downloaded.
    pub fn compute_chunk_diff(
        &self,
        local_chunks: &[ChunkInfo],
        remote_chunks: &[ChunkInfo],
    ) -> BlockDiffResult {
        compute_block_diff(&BlockDiffRequest {
            local_chunks: local_chunks.to_vec(),
            new_chunks: remote_chunks.to_vec(),
        })
    }

    /// Build a list of operations that can be safely synced (no conflicts).
    pub fn filter_syncable_ops(
        &self,
        pending_ops: &[QueuedOperation],
        remote_files: &[RemoteFileSnapshot],
    ) -> Vec<QueuedOperation> {
        let conflicts = self.detect_conflicts(pending_ops, remote_files);
        let conflict_ids: std::collections::HashSet<&str> =
            conflicts.iter().map(|c| c.local_path.as_str()).collect();

        pending_ops
            .iter()
            .filter(|op| !op.synced && !conflict_ids.contains(op.source_path.as_str()))
            .cloned()
            .collect()
    }

    /// Build a reconciliation plan: syncable ops, conflicted ops, and conflicts.
    pub fn plan(
        &self,
        pending_ops: &[QueuedOperation],
        remote_files: &[RemoteFileSnapshot],
    ) -> (Vec<QueuedOperation>, Vec<SyncConflict>) {
        let conflicts = self.detect_conflicts(pending_ops, remote_files);
        let conflict_ids: std::collections::HashSet<&str> =
            conflicts.iter().map(|c| c.local_path.as_str()).collect();

        let syncable = pending_ops
            .iter()
            .filter(|op| !op.synced && !conflict_ids.contains(op.source_path.as_str()))
            .cloned()
            .collect();

        (syncable, conflicts)
    }
}

impl Default for Reconciler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::change_queue::QueuedOperation;
    use chrono::Utc;

    fn make_snapshot(
        path: &str,
        hash: Option<&str>,
        size: u64,
        modified: chrono::DateTime<chrono::Utc>,
        exists: bool,
    ) -> RemoteFileSnapshot {
        if exists {
            RemoteFileSnapshot::existing(path, hash.unwrap_or(""), size, modified)
        } else {
            RemoteFileSnapshot::deleted(path, modified)
        }
    }

    fn now() -> chrono::DateTime<chrono::Utc> {
        Utc::now()
    }

    #[test]
    fn test_no_conflict_new_file() {
        let r = Reconciler::new();
        let ops = vec![QueuedOperation::put(
            "/new.txt",
            Some("h1".into()),
            Some(10),
            "u",
        )];
        let remote = vec![];
        let conflicts = r.detect_conflicts(&ops, &remote);
        assert!(conflicts.is_empty());
    }

    #[test]
    fn test_no_conflict_same_hash() {
        let r = Reconciler::new();
        let ops = vec![QueuedOperation::put(
            "/file.txt",
            Some("hash123".into()),
            Some(10),
            "u",
        )];
        let remote = vec![make_snapshot("/file.txt", Some("hash123"), 10, now(), true)];
        let conflicts = r.detect_conflicts(&ops, &remote);
        assert!(conflicts.is_empty());
    }

    #[test]
    fn test_conflict_edit_edit() {
        let r = Reconciler::new();
        let ops = vec![QueuedOperation::put(
            "/file.txt",
            Some("local_hash".into()),
            Some(10),
            "u",
        )];
        let remote = vec![make_snapshot("/file.txt", Some("remote"), 10, now(), true)];
        let conflicts = r.detect_conflicts(&ops, &remote);
        assert_eq!(conflicts.len(), 1);
        assert!(matches!(conflicts[0].conflict_type, ConflictType::EditEdit));
    }

    #[test]
    fn test_conflict_edit_delete() {
        let r = Reconciler::new();
        let ops = vec![QueuedOperation::put(
            "/file.txt",
            Some("h".into()),
            Some(10),
            "u",
        )];
        let remote = vec![make_snapshot("/file.txt", None, 0, now(), false)];
        let conflicts = r.detect_conflicts(&ops, &remote);
        assert_eq!(conflicts.len(), 1);
        assert!(matches!(
            conflicts[0].conflict_type,
            ConflictType::EditDelete
        ));
    }

    #[test]
    fn test_conflict_delete_edit() {
        let r = Reconciler::new();
        let ops = vec![QueuedOperation::delete("/file.txt", "u")];
        let remote = vec![make_snapshot("/file.txt", Some("h"), 10, now(), true)];
        let conflicts = r.detect_conflicts(&ops, &remote);
        assert_eq!(conflicts.len(), 1);
        assert!(matches!(
            conflicts[0].conflict_type,
            ConflictType::DeleteEdit
        ));
    }

    #[test]
    fn test_no_conflict_delete_delete() {
        let r = Reconciler::new();
        let ops = vec![QueuedOperation::delete("/file.txt", "u")];
        let remote = vec![make_snapshot("/file.txt", None, 0, now(), false)];
        let conflicts = r.detect_conflicts(&ops, &remote);
        assert!(conflicts.is_empty());
    }

    #[test]
    fn test_filter_syncable_ops() {
        let r = Reconciler::new();
        let ops = vec![
            QueuedOperation::put("/safe.txt", Some("h1".into()), Some(10), "u"),
            QueuedOperation::put("/conflict.txt", Some("local".into()), Some(10), "u"),
        ];
        let remote = vec![make_snapshot(
            "/conflict.txt",
            Some("remote"),
            10,
            now(),
            true,
        )];

        let syncable = r.filter_syncable_ops(&ops, &remote);
        assert_eq!(syncable.len(), 1);
        assert_eq!(syncable[0].source_path, "/safe.txt");
    }

    #[test]
    fn test_plan() {
        let r = Reconciler::new();
        let ops = vec![
            QueuedOperation::put("/safe.txt", Some("h1".into()), Some(10), "u"),
            QueuedOperation::put("/conflict.txt", Some("local".into()), Some(10), "u"),
        ];
        let remote = vec![make_snapshot(
            "/conflict.txt",
            Some("remote"),
            10,
            now(),
            true,
        )];

        let (syncable, conflicts) = r.plan(&ops, &remote);
        assert_eq!(syncable.len(), 1);
        assert_eq!(conflicts.len(), 1);
    }

    #[test]
    fn test_already_synced_ops_ignored() {
        let r = Reconciler::new();
        let mut op = QueuedOperation::put("/f.txt", Some("h".into()), Some(10), "u");
        op.synced = true;
        let ops = vec![op];
        let remote = vec![];

        let conflicts = r.detect_conflicts(&ops, &remote);
        assert!(conflicts.is_empty());
    }

    #[test]
    fn test_move_conflict_at_dest() {
        let r = Reconciler::new();
        let ops = vec![QueuedOperation::move_op("/a.txt", "/b.txt", "u")];
        let remote = vec![make_snapshot("/b.txt", Some("h"), 10, now(), true)];

        let conflicts = r.detect_conflicts(&ops, &remote);
        assert_eq!(conflicts.len(), 1);
        assert!(matches!(conflicts[0].conflict_type, ConflictType::EditEdit));
    }

    #[test]
    fn test_copy_no_conflict() {
        let r = Reconciler::new();
        let ops = vec![QueuedOperation::copy("/a.txt", "/b.txt", "u")];
        let remote = vec![];
        let conflicts = r.detect_conflicts(&ops, &remote);
        assert!(conflicts.is_empty());
    }

    #[test]
    fn test_create_collection_no_conflict() {
        let r = Reconciler::new();
        let ops = vec![QueuedOperation::create_collection("/dir/", "u")];
        let remote = vec![];
        let conflicts = r.detect_conflicts(&ops, &remote);
        assert!(conflicts.is_empty());
    }
}
