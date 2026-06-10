use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictType {
    EditEdit,
    EditDelete,
    DeleteEdit,
    RenameConflict,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictResolution {
    KeepLocal,
    KeepRemote,
    KeepBoth {
        local_name: String,
        remote_name: String,
    },
    KeepNewer,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConflict {
    pub local_path: String,
    pub local_modified: DateTime<Utc>,
    pub remote_modified: DateTime<Utc>,
    pub conflict_type: ConflictType,
    pub resolution: Option<ConflictResolution>,
}

impl SyncConflict {
    pub fn new(
        local_path: String,
        local_modified: DateTime<Utc>,
        remote_modified: DateTime<Utc>,
        conflict_type: ConflictType,
    ) -> Self {
        Self {
            local_path,
            local_modified,
            remote_modified,
            conflict_type,
            resolution: None,
        }
    }

    pub fn resolve(&mut self, resolution: ConflictResolution) {
        self.resolution = Some(resolution);
    }
}

#[derive(Debug)]
pub struct ConflictDetector;

impl ConflictDetector {
    pub fn new() -> Self {
        Self
    }

    pub fn detect_conflicts(
        &self,
        local_files: &[(String, DateTime<Utc>)],
        remote_files: &[(String, DateTime<Utc>)],
    ) -> Vec<SyncConflict> {
        let remote_map: std::collections::HashMap<&str, &DateTime<Utc>> = remote_files
            .iter()
            .map(|(path, ts)| (path.as_str(), ts))
            .collect();

        let mut conflicts = Vec::new();
        for (path, local_ts) in local_files {
            if let Some(&remote_ts) = remote_map.get(path.as_str())
                && local_ts != remote_ts
            {
                let conflict_type = ConflictType::EditEdit;
                conflicts.push(SyncConflict::new(
                    path.clone(),
                    *local_ts,
                    *remote_ts,
                    conflict_type,
                ));
            }
        }
        conflicts
    }

    pub fn detect_conflicts_with_deletions(
        &self,
        local_files: &[(String, Option<DateTime<Utc>>)],
        remote_files: &[(String, Option<DateTime<Utc>>)],
    ) -> Vec<SyncConflict> {
        let remote_map: std::collections::HashMap<&str, &Option<DateTime<Utc>>> = remote_files
            .iter()
            .map(|(path, ts)| (path.as_str(), ts))
            .collect();

        let local_set: std::collections::HashSet<&str> =
            local_files.iter().map(|(p, _)| p.as_str()).collect();

        let mut conflicts = Vec::new();

        for (path, local_ts_opt) in local_files {
            if let Some(remote_ts_opt) = remote_map.get(path.as_str()) {
                match (local_ts_opt, remote_ts_opt) {
                    (Some(local_ts), Some(remote_ts)) if local_ts != remote_ts => {
                        conflicts.push(SyncConflict::new(
                            path.clone(),
                            *local_ts,
                            *remote_ts,
                            ConflictType::EditEdit,
                        ));
                    }
                    (Some(local_ts), None) => {
                        let now = Utc::now();
                        conflicts.push(SyncConflict::new(
                            path.clone(),
                            *local_ts,
                            now,
                            ConflictType::EditDelete,
                        ));
                    }
                    (None, Some(remote_ts)) => {
                        let now = Utc::now();
                        conflicts.push(SyncConflict::new(
                            path.clone(),
                            now,
                            *remote_ts,
                            ConflictType::DeleteEdit,
                        ));
                    }
                    _ => {}
                }
            }
        }

        for (path, remote_ts_opt) in remote_files {
            if !local_set.contains(path.as_str())
                && let Some(remote_ts) = remote_ts_opt
            {
                let now = Utc::now();
                conflicts.push(SyncConflict::new(
                    path.clone(),
                    now,
                    *remote_ts,
                    ConflictType::DeleteEdit,
                ));
            }
        }

        conflicts
    }
}

impl Default for ConflictDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dt(year: i32, month: u32, day: u32) -> DateTime<Utc> {
        chrono::TimeZone::with_ymd_and_hms(&Utc, year, month, day, 0, 0, 0).unwrap()
    }

    #[test]
    fn test_no_conflicts_when_identical() {
        let detector = ConflictDetector::new();
        let local = vec![
            ("file.txt".to_string(), dt(2026, 1, 1)),
            ("readme.md".to_string(), dt(2026, 1, 15)),
        ];
        let remote = vec![
            ("file.txt".to_string(), dt(2026, 1, 1)),
            ("readme.md".to_string(), dt(2026, 1, 15)),
        ];
        let conflicts = detector.detect_conflicts(&local, &remote);
        assert!(conflicts.is_empty());
    }

    #[test]
    fn test_edit_edit_conflict() {
        let detector = ConflictDetector::new();
        let local = vec![("file.txt".to_string(), dt(2026, 1, 10))];
        let remote = vec![("file.txt".to_string(), dt(2026, 1, 12))];
        let conflicts = detector.detect_conflicts(&local, &remote);
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].local_path, "file.txt");
        assert_eq!(conflicts[0].conflict_type, ConflictType::EditEdit);
        assert!(conflicts[0].resolution.is_none());
    }

    #[test]
    fn test_edit_delete_conflict() {
        let detector = ConflictDetector::new();
        let local = vec![("file.txt".to_string(), Some(dt(2026, 1, 10)))];
        let remote = vec![("file.txt".to_string(), None)];
        let conflicts = detector.detect_conflicts_with_deletions(&local, &remote);
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].conflict_type, ConflictType::EditDelete);
    }

    #[test]
    fn test_delete_edit_conflict() {
        let detector = ConflictDetector::new();
        let local = vec![("file.txt".to_string(), None)];
        let remote = vec![("file.txt".to_string(), Some(dt(2026, 1, 10)))];
        let conflicts = detector.detect_conflicts_with_deletions(&local, &remote);
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].conflict_type, ConflictType::DeleteEdit);
    }

    #[test]
    fn test_multiple_conflicts() {
        let detector = ConflictDetector::new();
        let local = vec![
            ("a.txt".to_string(), dt(2026, 1, 5)),
            ("b.txt".to_string(), dt(2026, 1, 10)),
            ("c.txt".to_string(), dt(2026, 1, 3)),
        ];
        let remote = vec![
            ("a.txt".to_string(), dt(2026, 1, 8)),
            ("b.txt".to_string(), dt(2026, 1, 10)),
            ("c.txt".to_string(), dt(2026, 1, 20)),
        ];
        let conflicts = detector.detect_conflicts(&local, &remote);
        assert_eq!(conflicts.len(), 2);
        let paths: Vec<&str> = conflicts.iter().map(|c| c.local_path.as_str()).collect();
        assert!(paths.contains(&"a.txt"));
        assert!(paths.contains(&"c.txt"));
        assert!(!paths.contains(&"b.txt"));
    }

    #[test]
    fn test_conflict_resolution_selection() {
        let mut conflict = SyncConflict::new(
            "file.txt".into(),
            dt(2026, 1, 10),
            dt(2026, 1, 12),
            ConflictType::EditEdit,
        );
        assert!(conflict.resolution.is_none());
        conflict.resolve(ConflictResolution::KeepNewer);
        assert_eq!(conflict.resolution, Some(ConflictResolution::KeepNewer));
    }

    #[test]
    fn test_conflict_resolution_keep_both() {
        let mut conflict = SyncConflict::new(
            "file.txt".into(),
            dt(2026, 1, 10),
            dt(2026, 1, 12),
            ConflictType::EditEdit,
        );
        conflict.resolve(ConflictResolution::KeepBoth {
            local_name: "file (local).txt".to_string(),
            remote_name: "file (remote).txt".to_string(),
        });
        assert!(matches!(
            conflict.resolution,
            Some(ConflictResolution::KeepBoth { .. })
        ));
    }
}
