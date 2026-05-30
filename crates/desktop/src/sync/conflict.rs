//! Conflict resolution strategies.
//!
//! When both the local and remote file have changed since the last sync,
//! a conflict arises. This module provides resolution strategies.

use anyhow::Result;
use std::path::Path;

/// Strategy for resolving sync conflicts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictStrategy {
    /// Keep the local version, upload it (overwrites remote).
    LocalWins,
    /// Keep the remote version, download it (overwrites local).
    RemoteWins,
    /// Keep both versions. The local file is renamed to `filename (conflict).ext`.
    KeepBoth,
    /// Skip the file entirely (manual resolution required).
    Skip,
}

impl ConflictStrategy {
    /// Parse a conflict strategy from a string.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "local" | "local-wins" => Some(Self::LocalWins),
            "remote" | "remote-wins" => Some(Self::RemoteWins),
            "both" | "keep-both" => Some(Self::KeepBoth),
            "skip" | "manual" => Some(Self::Skip),
            _ => None,
        }
    }
}

impl std::fmt::Display for ConflictStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LocalWins => write!(f, "local-wins"),
            Self::RemoteWins => write!(f, "remote-wins"),
            Self::KeepBoth => write!(f, "keep-both"),
            Self::Skip => write!(f, "skip"),
        }
    }
}

/// Generate a conflict file path.
///
/// Given `/path/to/file.txt`, produces `/path/to/file (conflict 2026-05-29).txt`.
pub fn conflict_path(local_path: &Path) -> std::path::PathBuf {
    let parent = local_path.parent().unwrap_or(Path::new("."));
    let stem = local_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("file");
    let ext = local_path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("");

    let date = chrono::Local::now().format("%Y-%m-%d");
    let conflict_name = if ext.is_empty() {
        format!("{} (conflict {}).{}", stem, date, "txt")
    } else {
        format!("{} (conflict {}).{}", stem, date, ext)
    };

    parent.join(conflict_name)
}

/// Resolve a conflict by renaming the local file (KeepBoth strategy).
///
/// Moves the local file to a conflict filename, then the normal download
/// can proceed without overwriting the user's local changes.
pub fn resolve_conflict_keep_both(local_path: &Path) -> Result<std::path::PathBuf> {
    let conflict = conflict_path(local_path);
    if local_path.exists() {
        std::fs::rename(local_path, &conflict)?;
    }
    Ok(conflict)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conflict_strategy_parsing() {
        assert_eq!(
            ConflictStrategy::parse("local"),
            Some(ConflictStrategy::LocalWins)
        );
        assert_eq!(
            ConflictStrategy::parse("remote-wins"),
            Some(ConflictStrategy::RemoteWins)
        );
        assert_eq!(
            ConflictStrategy::parse("both"),
            Some(ConflictStrategy::KeepBoth)
        );
        assert_eq!(
            ConflictStrategy::parse("skip"),
            Some(ConflictStrategy::Skip)
        );
        assert_eq!(ConflictStrategy::parse("unknown"), None);
    }

    #[test]
    fn test_conflict_path_generation() {
        let path = std::path::Path::new("/home/user/docs/report.pdf");
        let conflict = conflict_path(path);
        assert!(conflict.to_string_lossy().contains("conflict"));
        assert!(conflict.to_string_lossy().ends_with(".pdf"));
        assert!(conflict.to_string_lossy().contains("report"));
    }

    #[test]
    fn test_conflict_path_no_extension() {
        let path = std::path::Path::new("/home/user/docs/README");
        let conflict = conflict_path(path);
        assert!(conflict.to_string_lossy().contains("conflict"));
    }
}
