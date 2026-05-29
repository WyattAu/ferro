//! Core types for the desktop sync engine.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Direction of a sync operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncDirection {
    /// Upload local changes to server.
    Upload,
    /// Download server changes to local.
    Download,
    /// Both directions (conflict resolution required).
    Both,
}

/// Status of a single file's sync state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileSyncStatus {
    /// File is in sync (local and remote match).
    Synced,
    /// Local file is newer (needs upload).
    LocalModified,
    /// Remote file is newer (needs download).
    RemoteModified,
    /// File exists only locally (new file, needs upload).
    LocalOnly,
    /// File exists only remotely (new file, needs download).
    RemoteOnly,
    /// File was deleted locally (remote copy needs deletion).
    LocalDeleted,
    /// File was deleted remotely (local copy needs deletion).
    RemoteDeleted,
    /// Both local and remote modified since last sync (conflict).
    Conflict,
    /// Sync is in progress for this file.
    Syncing,
    /// Last sync operation failed for this file.
    Error(String),
}

/// A single entry in the sync state database.
/// Tracks the known state of a file on both sides.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncEntry {
    /// Relative path from the sync root (uses forward slashes).
    pub relative_path: String,
    /// Whether this is a directory.
    pub is_dir: bool,
    /// SHA-256 hash of the local file content (empty if directory or deleted).
    pub local_hash: String,
    /// SHA-256 hash of the remote file content (empty if directory or deleted).
    pub remote_hash: String,
    /// Size of the local file in bytes.
    pub local_size: u64,
    /// Size of the remote file in bytes.
    pub remote_size: u64,
    /// Last modified timestamp of the local file (UNIX epoch ms).
    pub local_mtime_ms: i64,
    /// Last modified timestamp of the remote file (UNIX epoch ms).
    pub remote_mtime_ms: i64,
    /// The hash at the time of the last successful sync.
    /// If both local_hash and remote_hash match this, the file is synced.
    pub last_synced_hash: String,
    /// Timestamp of the last successful sync (UNIX epoch ms).
    pub last_synced_ms: i64,
    /// Whether the local file has been deleted since last sync.
    pub local_deleted: bool,
    /// Whether the remote file has been deleted since last sync.
    pub remote_deleted: bool,
}

impl SyncEntry {
    /// Determine the sync status for this entry.
    pub fn status(&self) -> FileSyncStatus {
        if self.local_deleted && self.remote_deleted {
            // Both deleted: clean up the entry
            return FileSyncStatus::Synced;
        }
        if self.local_deleted && !self.remote_deleted {
            return FileSyncStatus::LocalDeleted;
        }
        if self.remote_deleted && !self.local_deleted {
            return FileSyncStatus::RemoteDeleted;
        }
        if self.is_dir {
            // Directories are always considered synced (they are created on demand)
            return FileSyncStatus::Synced;
        }
        if self.local_hash.is_empty() && self.remote_hash.is_empty() {
            return FileSyncStatus::Synced;
        }
        if self.local_hash.is_empty() && !self.remote_hash.is_empty() {
            return FileSyncStatus::RemoteOnly;
        }
        if !self.local_hash.is_empty() && self.remote_hash.is_empty() {
            return FileSyncStatus::LocalOnly;
        }
        if self.local_hash == self.remote_hash {
            return FileSyncStatus::Synced;
        }
        if !self.last_synced_hash.is_empty() {
            let local_changed = self.local_hash != self.last_synced_hash;
            let remote_changed = self.remote_hash != self.last_synced_hash;
            if local_changed && remote_changed {
                return FileSyncStatus::Conflict;
            }
            if local_changed {
                return FileSyncStatus::LocalModified;
            }
            if remote_changed {
                return FileSyncStatus::RemoteModified;
            }
        }
        // No last_synced_hash: treat as conflict (both sides have content we haven't seen)
        if self.last_synced_hash.is_empty()
            && !self.local_hash.is_empty()
            && !self.remote_hash.is_empty()
        {
            return FileSyncStatus::Conflict;
        }
        FileSyncStatus::Synced
    }
}

/// Summary of a sync operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncSummary {
    /// Timestamp of this sync run.
    pub timestamp_ms: i64,
    /// Number of files uploaded.
    pub uploaded: u64,
    /// Number of files downloaded.
    pub downloaded: u64,
    /// Number of files deleted locally.
    pub local_deletions: u64,
    /// Number of files deleted remotely.
    pub remote_deletions: u64,
    /// Number of conflicts detected.
    pub conflicts: u64,
    /// Number of errors encountered.
    pub errors: u64,
    /// Total bytes transferred.
    pub bytes_transferred: u64,
    /// Duration of the sync in milliseconds.
    pub duration_ms: u64,
}

impl Default for SyncSummary {
    fn default() -> Self {
        Self {
            timestamp_ms: chrono::Utc::now().timestamp_millis(),
            uploaded: 0,
            downloaded: 0,
            local_deletions: 0,
            remote_deletions: 0,
            conflicts: 0,
            errors: 0,
            bytes_transferred: 0,
            duration_ms: 0,
        }
    }
}

/// Configuration for a sync folder.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncFolderConfig {
    /// Local directory path.
    pub local_path: PathBuf,
    /// Remote path prefix on the server (e.g., "/documents").
    pub remote_path: String,
    /// Whether sync is enabled for this folder.
    pub enabled: bool,
    /// Interval between automatic syncs in seconds (0 = manual only).
    pub interval_secs: u64,
}

/// A plan of actions to execute during a sync cycle.
#[derive(Debug, Clone, Default)]
pub struct SyncPlan {
    /// Files to upload.
    pub to_upload: Vec<String>,
    /// Files to download.
    pub to_download: Vec<String>,
    /// Remote files to delete.
    pub remote_deletions: Vec<String>,
    /// Local files to delete.
    pub local_deletions: Vec<String>,
    /// Conflicts to resolve.
    pub conflicts: Vec<String>,
}

impl SyncPlan {
    /// Create an empty plan.
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_status_synced() {
        let entry = SyncEntry {
            relative_path: "test.txt".to_string(),
            is_dir: false,
            local_hash: "abc".to_string(),
            remote_hash: "abc".to_string(),
            local_size: 10,
            remote_size: 10,
            local_mtime_ms: 1000,
            remote_mtime_ms: 1000,
            last_synced_hash: "abc".to_string(),
            last_synced_ms: 1000,
            local_deleted: false,
            remote_deleted: false,
        };
        assert_eq!(entry.status(), FileSyncStatus::Synced);
    }

    #[test]
    fn test_sync_status_local_modified() {
        let entry = SyncEntry {
            relative_path: "test.txt".to_string(),
            is_dir: false,
            local_hash: "def".to_string(),
            remote_hash: "abc".to_string(),
            local_size: 20,
            remote_size: 10,
            local_mtime_ms: 2000,
            remote_mtime_ms: 1000,
            last_synced_hash: "abc".to_string(),
            last_synced_ms: 1000,
            local_deleted: false,
            remote_deleted: false,
        };
        assert_eq!(entry.status(), FileSyncStatus::LocalModified);
    }

    #[test]
    fn test_sync_status_conflict() {
        let entry = SyncEntry {
            relative_path: "test.txt".to_string(),
            is_dir: false,
            local_hash: "local_hash".to_string(),
            remote_hash: "remote_hash".to_string(),
            local_size: 20,
            remote_size: 30,
            local_mtime_ms: 2000,
            remote_mtime_ms: 3000,
            last_synced_hash: "original_hash".to_string(),
            last_synced_ms: 1000,
            local_deleted: false,
            remote_deleted: false,
        };
        assert_eq!(entry.status(), FileSyncStatus::Conflict);
    }

    #[test]
    fn test_sync_status_local_only() {
        let entry = SyncEntry {
            relative_path: "new.txt".to_string(),
            is_dir: false,
            local_hash: "abc".to_string(),
            remote_hash: String::new(),
            local_size: 10,
            remote_size: 0,
            local_mtime_ms: 1000,
            remote_mtime_ms: 0,
            last_synced_hash: String::new(),
            last_synced_ms: 0,
            local_deleted: false,
            remote_deleted: false,
        };
        assert_eq!(entry.status(), FileSyncStatus::LocalOnly);
    }

    #[test]
    fn test_sync_status_remote_deleted() {
        let entry = SyncEntry {
            relative_path: "gone.txt".to_string(),
            is_dir: false,
            local_hash: "abc".to_string(),
            remote_hash: String::new(),
            local_size: 10,
            remote_size: 0,
            local_mtime_ms: 1000,
            remote_mtime_ms: 0,
            last_synced_hash: "abc".to_string(),
            last_synced_ms: 1000,
            local_deleted: false,
            remote_deleted: true,
        };
        assert_eq!(entry.status(), FileSyncStatus::RemoteDeleted);
    }
}
