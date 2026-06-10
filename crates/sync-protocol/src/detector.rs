use crate::protocol::{NodeId, VectorClock};
use chrono::{DateTime, Utc};
use notify::{Event as NotifyEvent, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;
use thiserror::Error;

/// Errors from the change detector.
#[derive(Debug, Error)]
pub enum DetectorError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("notify error: {0}")]
    Notify(String),
    #[error("path is not inside sync root: {0}")]
    PathOutsideRoot(String),
}

/// What kind of change was detected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangeKind {
    Created,
    Modified,
    Deleted,
    Renamed { from: PathBuf },
}

/// A detected file system change.
#[derive(Debug, Clone)]
pub struct DetectedChange {
    /// Absolute path of the changed file.
    pub path: PathBuf,
    /// Relative path within the sync root.
    pub relative_path: String,
    /// What kind of change occurred.
    pub kind: ChangeKind,
    /// When the change was detected (wall clock).
    pub detected_at: DateTime<Utc>,
}

/// Configuration for periodic scanning.
#[derive(Debug, Clone)]
pub struct ScanConfig {
    /// How often to do a full scan (in seconds). 0 = disabled.
    pub interval_secs: u64,
    /// Root directory to watch.
    pub sync_root: PathBuf,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            interval_secs: 300,
            sync_root: PathBuf::from("."),
        }
    }
}

/// Tracks file system changes through both real-time notifications and
/// periodic scans. Produces a list of `DetectedChange` that the sync
/// engine can process.
pub struct ChangeDetector {
    /// Real-time watcher (dropped = stops watching).
    _watcher: RecommendedWatcher,
    /// Receiver for file system events from the watcher.
    rx: mpsc::Receiver<Result<notify::Event, notify::Error>>,
    /// The sync root directory (for computing relative paths).
    sync_root: PathBuf,
    /// Last known state of files: relative_path -> (size, mtime).
    file_state: HashMap<String, FileSnapshot>,
    /// The vector clock that gets incremented on each detected change.
    /// In production this is shared with the sync engine.
    local_clock: VectorClock,
    /// Node ID for clock increments.
    node_id: NodeId,
}

#[derive(Debug, Clone)]
struct FileSnapshot {
    size: u64,
    modified: DateTime<Utc>,
}

impl ChangeDetector {
    /// Create a new detector watching the given root directory.
    pub fn new(sync_root: PathBuf, node_id: NodeId) -> Result<Self, DetectorError> {
        let (tx, rx) = mpsc::channel();

        let mut watcher = RecommendedWatcher::new(
            move |res: Result<NotifyEvent, notify::Error>| {
                let _ = tx.send(res);
            },
            notify::Config::default().with_poll_interval(Duration::from_secs(2)),
        )
        .map_err(|e| DetectorError::Notify(e.to_string()))?;

        watcher
            .watch(&sync_root, RecursiveMode::Recursive)
            .map_err(|e| DetectorError::Notify(e.to_string()))?;

        Ok(Self {
            _watcher: watcher,
            rx,
            sync_root,
            file_state: HashMap::new(),
            local_clock: VectorClock::new(),
            node_id,
        })
    }

    /// Create a detector without a file system watcher (for testing or
    /// when only periodic scans are desired).
    pub fn new_passive(sync_root: PathBuf, node_id: NodeId) -> Self {
        let (_, rx) = mpsc::channel();
        Self {
            _watcher: RecommendedWatcher::new(|_| {}, notify::Config::default())
                .map_err(|e| DetectorError::Notify(e.to_string()))
                .expect("failed to create passive file watcher"),
            rx,
            sync_root,
            file_state: HashMap::new(),
            local_clock: VectorClock::new(),
            node_id,
        }
    }

    /// Drain all pending file system events and return detected changes.
    pub fn poll_changes(&mut self) -> Vec<DetectedChange> {
        let mut changes = Vec::new();

        // Drain all pending events
        while let Ok(event_result) = self.rx.try_recv() {
            if let Ok(event) = event_result {
                for path in event.paths {
                    let relative = match path.strip_prefix(&self.sync_root) {
                        Ok(r) => r.to_string_lossy().to_string(),
                        Err(_) => continue,
                    };

                    let kind = match event.kind {
                        notify::EventKind::Create(_) => ChangeKind::Created,
                        notify::EventKind::Modify(_) => ChangeKind::Modified,
                        notify::EventKind::Remove(_) => ChangeKind::Deleted,
                        _ => continue,
                    };

                    self.local_clock.increment(&self.node_id);
                    changes.push(DetectedChange {
                        path,
                        relative_path: relative,
                        kind,
                        detected_at: Utc::now(),
                    });
                }
            }
        }

        changes
    }

    /// Perform a full scan of the sync root and return any changes since
    /// the last scan.
    pub fn scan(&mut self) -> Vec<DetectedChange> {
        let mut changes = Vec::new();
        let mut current_state = HashMap::new();

        if self.sync_root.exists() {
            let root = self.sync_root.clone();
            self.walk_directory(&root, &mut current_state, &mut changes);
        }

        // Detect deletions: files in old state but not in current scan
        let to_remove: Vec<String> = self
            .file_state
            .keys()
            .filter(|path| !current_state.contains_key(*path))
            .cloned()
            .collect();

        for relative_path in to_remove {
            self.local_clock.increment(&self.node_id);
            changes.push(DetectedChange {
                path: self.sync_root.join(&relative_path),
                relative_path: relative_path.clone(),
                kind: ChangeKind::Deleted,
                detected_at: Utc::now(),
            });
            self.file_state.remove(&relative_path);
        }

        self.file_state = current_state;
        changes
    }

    fn walk_directory(
        &mut self,
        dir: &Path,
        current_state: &mut HashMap<String, FileSnapshot>,
        changes: &mut Vec<DetectedChange>,
    ) {
        let entries = match std::fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                self.walk_directory(&path, current_state, changes);
                continue;
            }

            let metadata = match std::fs::metadata(&path) {
                Ok(m) => m,
                Err(_) => continue,
            };

            let relative = match path.strip_prefix(&self.sync_root) {
                Ok(r) => r.to_string_lossy().to_string(),
                Err(_) => continue,
            };

            let modified = metadata
                .modified()
                .ok()
                .map(|t| {
                    let datetime: DateTime<Utc> = t.into();
                    datetime
                })
                .unwrap_or_else(Utc::now);

            let snapshot = FileSnapshot {
                size: metadata.len(),
                modified,
            };

            if let Some(old) = self.file_state.get(&relative) {
                if old.modified != snapshot.modified || old.size != snapshot.size {
                    self.local_clock.increment(&self.node_id);
                    changes.push(DetectedChange {
                        path: path.clone(),
                        relative_path: relative.clone(),
                        kind: ChangeKind::Modified,
                        detected_at: Utc::now(),
                    });
                }
            } else {
                // New file not in previous state
                self.local_clock.increment(&self.node_id);
                changes.push(DetectedChange {
                    path: path.clone(),
                    relative_path: relative.clone(),
                    kind: ChangeKind::Created,
                    detected_at: Utc::now(),
                });
            }

            current_state.insert(relative, snapshot);
        }
    }

    /// Get the current local vector clock.
    pub fn local_clock(&self) -> &VectorClock {
        &self.local_clock
    }

    /// Get the sync root path.
    pub fn sync_root(&self) -> &Path {
        &self.sync_root
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_scan_new_files() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();

        let mut detector = ChangeDetector::new_passive(root.clone(), "test-node".into());

        // Create some files
        std::fs::write(root.join("file1.txt"), "hello").unwrap();
        std::fs::write(root.join("file2.txt"), "world").unwrap();

        let changes = detector.scan();
        assert_eq!(changes.len(), 2);
        let kinds: Vec<&ChangeKind> = changes.iter().map(|c| &c.kind).collect();
        assert!(kinds.contains(&&ChangeKind::Created));
    }

    #[test]
    fn test_scan_modified_files() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();

        let mut detector = ChangeDetector::new_passive(root.clone(), "test-node".into());

        std::fs::write(root.join("file1.txt"), "hello").unwrap();
        detector.scan(); // initial scan

        // Modify the file
        std::fs::write(root.join("file1.txt"), "hello world").unwrap();
        let changes = detector.scan();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].kind, ChangeKind::Modified);
    }

    #[test]
    fn test_scan_deleted_files() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();

        let mut detector = ChangeDetector::new_passive(root.clone(), "test-node".into());

        std::fs::write(root.join("file1.txt"), "hello").unwrap();
        detector.scan(); // initial scan

        std::fs::remove_file(root.join("file1.txt")).unwrap();
        let changes = detector.scan();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].kind, ChangeKind::Deleted);
    }

    #[test]
    fn test_scan_no_changes() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();

        let mut detector = ChangeDetector::new_passive(root.clone(), "test-node".into());

        std::fs::write(root.join("file1.txt"), "hello").unwrap();
        detector.scan(); // initial scan

        let changes = detector.scan();
        assert!(changes.is_empty());
    }

    #[test]
    fn test_clock_increments_on_change() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();

        let mut detector = ChangeDetector::new_passive(root.clone(), "test-node".into());

        assert_eq!(detector.local_clock().get_counter("test-node"), 0);

        std::fs::write(root.join("a.txt"), "a").unwrap();
        detector.scan();
        assert_eq!(detector.local_clock().get_counter("test-node"), 1);

        std::fs::write(root.join("b.txt"), "b").unwrap();
        detector.scan();
        assert_eq!(detector.local_clock().get_counter("test-node"), 2);
    }
}
