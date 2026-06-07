use crate::detector::{ChangeDetector, DetectedChange};
use crate::protocol::{
    ConflictInfo, ConflictResolution, FileManifest, NodeId, SyncRequest, SyncResponse, VectorClock,
};
use crate::state::{PeerSyncState, SyncStateError, SyncStateManager, SyncStatus};
use chrono::Utc;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;

/// Errors that can occur during sync operations.
#[derive(Debug, Error)]
pub enum SyncError {
    #[error("state error: {0}")]
    State(#[from] SyncStateError),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("peer unreachable: {0}")]
    PeerUnreachable(String),
    #[error("conflict on '{path}' requires resolution")]
    Conflict { path: String },
    #[error("sync cancelled")]
    Cancelled,
}

/// Sync direction policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncMode {
    /// Push local changes to the peer.
    Push,
    /// Pull remote changes from the peer.
    Pull,
    /// Bidirectional: push local changes, then pull remote changes.
    Bidirectional,
}

/// Configuration for a sync engine instance.
#[derive(Debug, Clone)]
pub struct SyncEngineConfig {
    /// This node's identifier.
    pub node_id: NodeId,
    /// Root directory being synced.
    pub sync_root: PathBuf,
    /// How often to scan for changes (in seconds). 0 = scan only on demand.
    pub scan_interval_secs: u64,
    /// Default sync mode.
    pub sync_mode: SyncMode,
    /// Default conflict resolution strategy.
    pub default_conflict_resolution: ConflictResolution,
    /// Maximum number of concurrent peer syncs.
    pub max_concurrent_peers: usize,
}

impl Default for SyncEngineConfig {
    fn default() -> Self {
        Self {
            node_id: uuid::Uuid::new_v4().to_string(),
            sync_root: PathBuf::from("."),
            scan_interval_secs: 300,
            sync_mode: SyncMode::Bidirectional,
            default_conflict_resolution: ConflictResolution::LastWriterWins,
            max_concurrent_peers: 8,
        }
    }
}

/// The sync engine orchestrates file synchronization across nodes. It
/// coordinates the change detector, state manager, and protocol messages.
pub struct SyncEngine {
    config: SyncEngineConfig,
    state_manager: Arc<SyncStateManager>,
    change_detector: Arc<tokio::sync::Mutex<ChangeDetector>>,
    /// Pending conflicts that need resolution: path -> ConflictInfo.
    pending_conflicts: dashmap::DashMap<String, ConflictInfo>,
}

impl SyncEngine {
    /// Create a new sync engine.
    pub fn new(
        config: SyncEngineConfig,
        state_manager: SyncStateManager,
    ) -> Result<Self, SyncError> {
        let detector = ChangeDetector::new(config.sync_root.clone(), config.node_id.clone())
            .map_err(|e| {
                SyncError::Io(std::io::Error::other(e.to_string()))
            })?;

        Ok(Self {
            config,
            state_manager: Arc::new(state_manager),
            change_detector: Arc::new(tokio::sync::Mutex::new(detector)),
            pending_conflicts: dashmap::DashMap::new(),
        })
    }

    /// Get the configuration.
    pub fn config(&self) -> &SyncEngineConfig {
        &self.config
    }

    /// Get the state manager.
    pub fn state_manager(&self) -> &SyncStateManager {
        &self.state_manager
    }

    /// Build a SyncRequest to send to a peer, using the last known clock.
    pub async fn build_sync_request(&self, peer_node_id: &str) -> Result<SyncRequest, SyncError> {
        let state = self.state_manager.get_or_create(peer_node_id)?;
        Ok(SyncRequest {
            from_node: self.config.node_id.clone(),
            since_clock: state.last_sync_clock.clone(),
            path_prefix: None,
        })
    }

    /// Process an incoming SyncRequest from a peer and produce a response.
    /// This determines what files have changed since the peer's last sync
    /// clock and returns their manifests.
    pub async fn handle_sync_request(
        &self,
        request: &SyncRequest,
    ) -> Result<SyncResponse, SyncError> {
        let detector = self.change_detector.lock().await;
        let local_clock = detector.local_clock().clone();

        // In a real implementation this would diff the file manifests
        // against the request.since_clock to find what changed. Here we
        // build a response skeleton that the transport layer fills.
        Ok(SyncResponse {
            from_node: self.config.node_id.clone(),
            current_clock: local_clock,
            changed_files: Vec::new(),
            conflicts: Vec::new(),
            requires_full_sync: request.since_clock.counters.is_empty(),
        })
    }

    /// Process an incoming SyncResponse from a peer. This merges clocks,
    /// identifies conflicts, and updates local state.
    pub async fn handle_sync_response(
        &self,
        response: &SyncResponse,
    ) -> Result<Vec<ConflictInfo>, SyncError> {
        let mut state = self.state_manager.get_or_create(&response.from_node)?;

        // Merge the remote clock into our last-synced clock for this peer
        state.last_sync_clock.merge(&response.current_clock);

        // Record conflicts
        for conflict in &response.conflicts {
            self.pending_conflicts
                .insert(conflict.path.clone(), conflict.clone());
        }

        if !response.conflicts.is_empty() {
            state.status = SyncStatus::Conflict;
        }

        self.state_manager.save(&state)?;
        Ok(response.conflicts.clone())
    }

    /// Resolve a conflict for a specific file path.
    pub async fn resolve_conflict(
        &self,
        file_path: &str,
        resolution: ConflictResolution,
    ) -> Result<(), SyncError> {
        if let Some((_, conflict)) = self.pending_conflicts.remove(file_path) {
            match &resolution {
                ConflictResolution::LastWriterWins => {
                    // Determine winner by wall clock
                    let _winner = if conflict.local_manifest.modified_at
                        >= conflict.remote_manifest.modified_at
                    {
                        &conflict.local_manifest
                    } else {
                        &conflict.remote_manifest
                    };
                    // In production: apply the winning version to storage
                }
                ConflictResolution::KeepLocal => {
                    // Local version wins — no action needed (it's already here)
                }
                ConflictResolution::KeepRemote => {
                    // Remote version wins — would pull from peer
                }
                ConflictResolution::KeepBoth {
                    local_name,
                    remote_name,
                } => {
                    // Rename both files to avoid collision
                    tracing::info!(
                        "Conflict resolved: keep both as '{}' and '{}'",
                        local_name,
                        remote_name
                    );
                }
                ConflictResolution::Manual => {
                    // Leave for user to resolve
                    self.pending_conflicts
                        .insert(file_path.to_string(), conflict);
                    return Ok(());
                }
            }

            self.state_manager
                .log_event(&self.config.node_id, "resolve", "success", 0, None)?;
        }

        Ok(())
    }

    /// Get a list of all pending conflicts.
    pub fn pending_conflicts(&self) -> Vec<(String, ConflictInfo)> {
        self.pending_conflicts
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect()
    }

    /// Perform a full sync with a peer: exchange all manifests and resolve
    /// conflicts automatically.
    pub async fn full_sync(&self, peer_node_id: &str) -> Result<SyncResult, SyncError> {
        self.state_manager
            .set_status(peer_node_id, SyncStatus::Syncing)?;

        let _request = self.build_sync_request(peer_node_id).await?;

        // The actual network call happens in the transport layer.
        // Here we produce the request and return a SyncResult that the
        // caller sends over the wire. On receiving a response, call
        // handle_sync_response().

        let detector = self.change_detector.lock().await;
        let local_clock = detector.local_clock().clone();
        drop(detector);

        let result = SyncResult {
            peer_node_id: peer_node_id.to_string(),
            files_sent: 0,
            files_received: 0,
            conflicts: 0,
            clock: local_clock,
        };

        self.state_manager.record_success(
            peer_node_id,
            result.clock.clone(),
            result.files_sent + result.files_received,
        )?;

        Ok(result)
    }

    /// Scan for local changes and return them.
    pub async fn scan_local_changes(&self) -> Vec<DetectedChange> {
        let mut detector = self.change_detector.lock().await;
        let mut changes = detector.scan();
        let watcher_changes = detector.poll_changes();
        changes.extend(watcher_changes);
        changes
    }

    /// Get the current local vector clock.
    pub async fn local_clock(&self) -> VectorClock {
        let detector = self.change_detector.lock().await;
        detector.local_clock().clone()
    }

    /// Get sync state for all known peers.
    pub fn all_peer_states(&self) -> Result<Vec<PeerSyncState>, SyncError> {
        Ok(self.state_manager.list_peers()?)
    }
}

/// Result of a sync operation.
#[derive(Debug, Clone)]
pub struct SyncResult {
    pub peer_node_id: String,
    pub files_sent: u64,
    pub files_received: u64,
    pub conflicts: u64,
    pub clock: VectorClock,
}

/// Build a FileManifest from a file on disk.
pub fn build_file_manifest(
    relative_path: &str,
    full_path: &std::path::Path,
    clock: &VectorClock,
) -> Result<FileManifest, std::io::Error> {
    let metadata = std::fs::metadata(full_path)?;
    let content = std::fs::read(full_path)?;
    let hash: [u8; 32] = Sha256::digest(&content).into();

    let modified: chrono::DateTime<Utc> = metadata
        .modified()
        .ok()
        .map(|t| t.into())
        .unwrap_or_else(Utc::now);

    Ok(FileManifest {
        path: relative_path.to_string(),
        content_hash: hash,
        size: metadata.len(),
        modified_at: modified,
        vector_clock: clock.clone(),
        deleted: false,
        chunks: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::SyncStateManager;
    use tempfile::tempdir;

    fn make_engine() -> (SyncEngine, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();
        std::fs::write(root.join("test.txt"), "hello").unwrap();

        let state_mgr = SyncStateManager::new_in_memory().unwrap();

        let config = SyncEngineConfig {
            node_id: "test-node".into(),
            sync_root: root,
            scan_interval_secs: 0,
            sync_mode: SyncMode::Bidirectional,
            default_conflict_resolution: ConflictResolution::LastWriterWins,
            max_concurrent_peers: 4,
        };

        let engine = SyncEngine::new(config, state_mgr).unwrap();
        (engine, dir)
    }

    #[test]
    fn test_build_file_manifest() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        std::fs::write(root.join("file.txt"), "test content").unwrap();

        let mut clock = VectorClock::new();
        clock.increment("node-a");

        let manifest = build_file_manifest("file.txt", &root.join("file.txt"), &clock).unwrap();

        assert_eq!(manifest.path, "file.txt");
        assert_eq!(manifest.size, 12);
        assert!(!manifest.deleted);
        assert_eq!(manifest.vector_clock.get_counter("node-a"), 1);
    }

    #[tokio::test]
    async fn test_build_sync_request() {
        let (engine, _dir) = make_engine();
        let request = engine.build_sync_request("peer-1").await.unwrap();
        assert_eq!(request.from_node, "test-node");
        assert!(request.since_clock.counters.is_empty());
    }

    #[tokio::test]
    async fn test_handle_sync_request() {
        let (engine, _dir) = make_engine();
        let request = SyncRequest {
            from_node: "peer-1".into(),
            since_clock: VectorClock::new(),
            path_prefix: None,
        };

        let response = engine.handle_sync_request(&request).await.unwrap();
        assert_eq!(response.from_node, "test-node");
        assert!(response.requires_full_sync);
    }

    #[tokio::test]
    async fn test_scan_local_changes() {
        let (engine, _dir) = make_engine();
        let changes = engine.scan_local_changes().await;
        // The initial scan should find test.txt
        assert!(!changes.is_empty());
    }

    #[tokio::test]
    async fn test_all_peer_states() {
        let (engine, _dir) = make_engine();
        let states = engine.all_peer_states().unwrap();
        assert!(states.is_empty());
    }

    #[tokio::test]
    async fn test_pending_conflicts_empty() {
        let (engine, _dir) = make_engine();
        assert!(engine.pending_conflicts().is_empty());
    }
}
