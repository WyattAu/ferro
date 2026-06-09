use chrono::{DateTime, Utc};
use ferro_common::chunk::ChunkInfo;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Node identifier — unique per Ferro instance in a sync group.
pub type NodeId = String;

/// Logical timestamp tracking per-node ordering. Each node monotonically
/// increments its own counter; other counters track the latest value observed
/// from each peer. This allows detecting concurrent modifications across
/// arbitrarily many nodes without a central coordinator.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VectorClock {
    /// Map of node_id -> logical counter for that node.
    pub counters: HashMap<NodeId, u64>,
}

impl VectorClock {
    pub fn new() -> Self {
        Self {
            counters: HashMap::new(),
        }
    }

    /// Increment the counter for the given node and return the new value.
    pub fn increment(&mut self, node_id: &str) -> u64 {
        let entry = self.counters.entry(node_id.to_string()).or_insert(0);
        *entry += 1;
        *entry
    }

    /// Merge another clock into this one, taking the max of each counter.
    pub fn merge(&mut self, other: &VectorClock) {
        for (node, &count) in &other.counters {
            let entry = self.counters.entry(node.clone()).or_insert(0);
            *entry = (*entry).max(count);
        }
    }

    /// Returns true if `self` happened-before `other` (all of self's counters
    /// are <= other's, with at least one strictly less).
    pub fn happened_before(&self, other: &VectorClock) -> bool {
        let mut any_less = false;

        // Check all nodes in other's counters — self's count must be <= other's
        for (node, &count) in &other.counters {
            let my_count = self.counters.get(node).copied().unwrap_or(0);
            if my_count > count {
                return false;
            }
            if my_count < count {
                any_less = true;
            }
        }

        // If self has a counter for a node that other doesn't know about,
        // and it's > 0, then self is ahead in that dimension → not happened-before.
        for (node, &count) in &self.counters {
            if !other.counters.contains_key(node) && count > 0 {
                return false;
            }
        }

        any_less
    }

    /// Two clocks are concurrent if neither happened-before the other.
    pub fn is_concurrent_with(&self, other: &VectorClock) -> bool {
        !self.happened_before(other) && !other.happened_before(self)
    }

    pub fn get_counter(&self, node_id: &str) -> u64 {
        self.counters.get(node_id).copied().unwrap_or(0)
    }
}

impl Default for VectorClock {
    fn default() -> Self {
        Self::new()
    }
}

/// Manifest of a file's state at a point in time. Used to compare versions
/// across nodes without transferring full file content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileManifest {
    /// Relative path within the sync root.
    pub path: String,
    /// Content hash (SHA-256).
    pub content_hash: [u8; 32],
    /// File size in bytes.
    pub size: u64,
    /// Last modification timestamp (source-of-truth node's clock).
    pub modified_at: DateTime<Utc>,
    /// The vector clock of the node that last wrote this file.
    pub vector_clock: VectorClock,
    /// Whether this file has been deleted (tombstone).
    pub deleted: bool,
    /// Block-level chunk info for delta sync (empty = full sync required).
    pub chunks: Vec<ChunkInfo>,
}

/// A sync request is sent from a node that wants to know what changed since
/// a given point in time on a remote peer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRequest {
    /// The requesting node's identifier.
    pub from_node: NodeId,
    /// The requesting node's vector clock — the remote peer uses this to
    /// determine which files the requester is missing or has outdated.
    pub since_clock: VectorClock,
    /// Optional: only sync files under this prefix.
    pub path_prefix: Option<String>,
}

/// A sync response carries the list of file manifests that changed since
/// the requester's clock, plus any conflicts detected.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResponse {
    /// The responding node's identifier.
    pub from_node: NodeId,
    /// The responding node's current vector clock (so requester can merge).
    pub current_clock: VectorClock,
    /// Files that changed since the requester's clock on this node.
    pub changed_files: Vec<FileManifest>,
    /// Conflicts detected between local and remote versions.
    pub conflicts: Vec<ConflictInfo>,
    /// Whether a full sync is required (e.g., first sync, clock reset).
    pub requires_full_sync: bool,
}

/// Details of a detected conflict between two versions of the same file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictInfo {
    /// Relative path of the conflicting file.
    pub path: String,
    /// The local (responder's) version manifest.
    pub local_manifest: FileManifest,
    /// The remote (requester's) version manifest.
    pub remote_manifest: FileManifest,
    /// Which node's version is "newer" by wall clock (if determinable).
    pub newer_node: Option<NodeId>,
}

/// Conflict resolution strategy.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConflictResolution {
    /// Keep the version with the latest wall-clock timestamp.
    LastWriterWins,
    /// Keep the local version.
    KeepLocal,
    /// Keep the remote version.
    KeepRemote,
    /// Keep both, renaming one with a conflict suffix.
    KeepBoth {
        local_name: String,
        remote_name: String,
    },
    /// Hand off to the user for manual resolution.
    Manual,
}

/// The overall message envelope for the wire protocol. A single enum
/// simplifies serialization/deserialization over TCP or WebSocket.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncMessage {
    /// Client requests a delta sync.
    Request(SyncRequest),
    /// Server responds with manifests + conflicts.
    Response(SyncResponse),
    /// Client requests a full file transfer for a specific path.
    FullSyncRequest { path: String, from_node: NodeId },
    /// Server responds with full file content.
    FullSyncResponse {
        path: String,
        content: Vec<u8>,
        manifest: FileManifest,
    },
    /// Client sends a block-level delta (only changed chunks).
    DeltaSync {
        path: String,
        from_node: NodeId,
        chunks_to_upload: Vec<(ChunkInfo, Vec<u8>)>,
        chunks_to_delete: Vec<ChunkInfo>,
    },
    /// Client sends an explicit conflict resolution.
    ResolveConflict {
        path: String,
        resolution: ConflictResolution,
        from_node: NodeId,
    },
    /// Acknowledgment that a sync operation completed.
    Ack {
        path: String,
        success: bool,
        error: Option<String>,
    },
    /// Heartbeat / keep-alive.
    Ping { from_node: NodeId },
    /// Pong response.
    Pong { from_node: NodeId },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_clock_increment() {
        let mut clock = VectorClock::new();
        assert_eq!(clock.increment("node-a"), 1);
        assert_eq!(clock.increment("node-a"), 2);
        assert_eq!(clock.increment("node-b"), 1);
        assert_eq!(clock.get_counter("node-a"), 2);
        assert_eq!(clock.get_counter("node-b"), 1);
    }

    #[test]
    fn test_vector_clock_merge() {
        let mut a = VectorClock::new();
        a.increment("node-a");
        a.increment("node-a");
        a.increment("node-b");

        let mut b = VectorClock::new();
        b.increment("node-a");
        b.increment("node-c");

        a.merge(&b);
        assert_eq!(a.get_counter("node-a"), 2);
        assert_eq!(a.get_counter("node-b"), 1);
        assert_eq!(a.get_counter("node-c"), 1);
    }

    #[test]
    fn test_happened_before() {
        let mut earlier = VectorClock::new();
        earlier.increment("node-a");

        let mut later = VectorClock::new();
        later.increment("node-a");
        later.increment("node-a");

        assert!(earlier.happened_before(&later));
        assert!(!later.happened_before(&earlier));
    }

    #[test]
    fn test_concurrent_clocks() {
        let mut a = VectorClock::new();
        a.increment("node-a");

        let mut b = VectorClock::new();
        b.increment("node-b");

        assert!(a.is_concurrent_with(&b));
        assert!(b.is_concurrent_with(&a));
    }

    #[test]
    fn test_non_concurrent_clocks() {
        let mut a = VectorClock::new();
        a.increment("node-a");
        a.increment("node-b");

        let mut b = VectorClock::new();
        b.increment("node-a");

        assert!(!a.is_concurrent_with(&b));
        assert!(!b.is_concurrent_with(&a));
    }

    #[test]
    fn test_serialization_roundtrip() {
        let msg = SyncMessage::Ping {
            from_node: "test-node".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: SyncMessage = serde_json::from_str(&json).unwrap();
        match parsed {
            SyncMessage::Ping { from_node } => assert_eq!(from_node, "test-node"),
            _ => panic!("wrong variant"),
        }
    }
}
