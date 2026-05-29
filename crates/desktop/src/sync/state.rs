//! Local sync state persistence.
//!
//! Stores the sync state database as a JSON file in the sync directory.
//! Each tracked file has a `SyncEntry` recording local/remote hashes,
//! timestamps, and deletion state.

use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::types::SyncEntry;

const STATE_FILE_NAME: &str = ".ferro-sync-state.json";

/// Persistent sync state database.
#[derive(Debug, Clone)]
pub struct SyncState {
    /// Path to the state file.
    state_path: PathBuf,
    /// Map of relative_path -> SyncEntry.
    entries: HashMap<String, SyncEntry>,
}

impl SyncState {
    /// Create a new SyncState backed by a JSON file in the given directory.
    pub fn new(sync_dir: &Path) -> Self {
        let state_path = sync_dir.join(STATE_FILE_NAME);
        Self {
            state_path,
            entries: HashMap::new(),
        }
    }

    /// Load state from disk. Returns a new state if no file exists.
    pub fn load(sync_dir: &Path) -> Result<Self> {
        let state_path = sync_dir.join(STATE_FILE_NAME);
        if state_path.exists() {
            let data = std::fs::read_to_string(&state_path)?;
            let entries: HashMap<String, SyncEntry> = serde_json::from_str(&data)?;
            Ok(Self {
                state_path,
                entries,
            })
        } else {
            Ok(Self {
                state_path,
                entries: HashMap::new(),
            })
        }
    }

    /// Save state to disk atomically.
    pub fn save(&self) -> Result<()> {
        if let Some(parent) = self.state_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_string_pretty(&self.entries)?;
        // Atomic write: write to temp file, then rename
        let tmp_path = self.state_path.with_extension("tmp");
        std::fs::write(&tmp_path, &data)?;
        std::fs::rename(&tmp_path, &self.state_path)?;
        Ok(())
    }

    /// Get a sync entry by relative path.
    pub fn get(&self, relative_path: &str) -> Option<&SyncEntry> {
        self.entries.get(relative_path)
    }

    /// Insert or update a sync entry.
    pub fn insert(&mut self, entry: SyncEntry) {
        self.entries.insert(entry.relative_path.clone(), entry);
    }

    /// Remove a sync entry.
    pub fn remove(&mut self, relative_path: &str) -> Option<SyncEntry> {
        self.entries.remove(relative_path)
    }

    /// Iterate over all entries.
    pub fn iter(&self) -> impl Iterator<Item = &SyncEntry> {
        self.entries.values()
    }

    /// Number of tracked entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether there are no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get all entries as a mutable map.
    pub fn entries_mut(&mut self) -> &mut HashMap<String, SyncEntry> {
        &mut self.entries
    }

    /// Get all entries as a map reference.
    pub fn entries(&self) -> &HashMap<String, SyncEntry> {
        &self.entries
    }

    /// Mark a path as locally deleted (sets local_deleted=true, clears local_hash).
    pub fn mark_local_deleted(&mut self, relative_path: &str) {
        if let Some(entry) = self.entries.get_mut(relative_path) {
            entry.local_deleted = true;
            entry.local_hash.clear();
            entry.local_size = 0;
            entry.local_mtime_ms = 0;
        }
    }

    /// Mark a path as remotely deleted (sets remote_deleted=true, clears remote_hash).
    pub fn mark_remote_deleted(&mut self, relative_path: &str) {
        if let Some(entry) = self.entries.get_mut(relative_path) {
            entry.remote_deleted = true;
            entry.remote_hash.clear();
            entry.remote_size = 0;
            entry.remote_mtime_ms = 0;
        }
    }

    /// Update the local side of an entry after a file scan.
    pub fn update_local(
        &mut self,
        relative_path: &str,
        hash: String,
        size: u64,
        mtime_ms: i64,
        is_dir: bool,
    ) {
        let entry = self
            .entries
            .entry(relative_path.to_string())
            .or_insert_with(|| SyncEntry {
                relative_path: relative_path.to_string(),
                is_dir,
                local_hash: String::new(),
                remote_hash: String::new(),
                local_size: 0,
                remote_size: 0,
                local_mtime_ms: 0,
                remote_mtime_ms: 0,
                last_synced_hash: String::new(),
                last_synced_ms: 0,
                local_deleted: false,
                remote_deleted: false,
            });
        entry.local_hash = hash;
        entry.local_size = size;
        entry.local_mtime_ms = mtime_ms;
        entry.local_deleted = false;
        entry.is_dir = is_dir;
    }

    /// Update the remote side of an entry after a remote scan.
    pub fn update_remote(
        &mut self,
        relative_path: &str,
        hash: String,
        size: u64,
        mtime_ms: i64,
        is_dir: bool,
    ) {
        let entry = self
            .entries
            .entry(relative_path.to_string())
            .or_insert_with(|| SyncEntry {
                relative_path: relative_path.to_string(),
                is_dir,
                local_hash: String::new(),
                remote_hash: String::new(),
                local_size: 0,
                remote_size: 0,
                local_mtime_ms: 0,
                remote_mtime_ms: 0,
                last_synced_hash: String::new(),
                last_synced_ms: 0,
                local_deleted: false,
                remote_deleted: false,
            });
        entry.remote_hash = hash;
        entry.remote_size = size;
        entry.remote_mtime_ms = mtime_ms;
        entry.remote_deleted = false;
        entry.is_dir = is_dir;
    }

    /// Mark an entry as successfully synced at the current time.
    pub fn mark_synced(&mut self, relative_path: &str) {
        let now_ms = chrono::Utc::now().timestamp_millis();
        if let Some(entry) = self.entries.get_mut(relative_path) {
            entry.last_synced_hash = if entry.local_hash.is_empty() {
                entry.remote_hash.clone()
            } else {
                entry.local_hash.clone()
            };
            entry.last_synced_ms = now_ms;
            entry.local_deleted = false;
            entry.remote_deleted = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_state_save_load() {
        let dir = std::env::temp_dir().join("ferro-sync-test-state");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let mut state = SyncState::new(&dir);
        state.insert(SyncEntry {
            relative_path: "test.txt".to_string(),
            is_dir: false,
            local_hash: "abc123".to_string(),
            remote_hash: "abc123".to_string(),
            local_size: 100,
            remote_size: 100,
            local_mtime_ms: 1000,
            remote_mtime_ms: 1000,
            last_synced_hash: "abc123".to_string(),
            last_synced_ms: 1000,
            local_deleted: false,
            remote_deleted: false,
        });
        state.save().unwrap();

        let loaded = SyncState::load(&dir).unwrap();
        assert_eq!(loaded.len(), 1);
        let entry = loaded.get("test.txt").unwrap();
        assert_eq!(entry.local_hash, "abc123");
        assert_eq!(entry.remote_hash, "abc123");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_mark_synced() {
        let dir = std::env::temp_dir().join("ferro-sync-test-mark");
        let _ = std::fs::remove_dir_all(&dir);

        let mut state = SyncState::new(&dir);
        state.update_local("file.txt", "hash1".to_string(), 100, 1000, false);
        state.update_remote("file.txt", "hash1".to_string(), 100, 1000, false);
        state.mark_synced("file.txt");

        let entry = state.get("file.txt").unwrap();
        assert_eq!(entry.last_synced_hash, "hash1");
        assert!(entry.last_synced_ms > 0);
    }
}
