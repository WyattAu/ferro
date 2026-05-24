//! Atomic file write utilities.
//!
//! Provides crash-safe file writes using the temp-file-then-rename pattern.
//! On POSIX systems, `rename()` is atomic, so a crash mid-write never leaves
//! a partial file at the target path.

use std::path::Path;

/// Write data to a file atomically using a temporary file and rename.
///
/// This prevents partial/corrupt files if the process crashes during write.
/// The write goes to a `.tmp` file first, then is renamed to the final path.
/// On error, the temporary file is cleaned up.
///
/// # Errors
/// Returns `std::io::Error` if the temporary file cannot be created,
/// written to, or renamed.
pub fn atomic_write(path: &Path, data: &[u8]) -> std::io::Result<()> {
    let tmp_path = path.with_extension("tmp");

    // Write to temp file
    std::fs::write(&tmp_path, data)?;

    // Rename temp file to target (atomic on POSIX)
    if let Err(e) = std::fs::rename(&tmp_path, path) {
        // Clean up temp file on rename failure
        let _ = std::fs::remove_file(&tmp_path);
        return Err(e);
    }

    Ok(())
}

/// Async version of `atomic_write`. Runs the blocking I/O on a spawn_blocking
/// thread.
pub async fn atomic_write_async(path: std::path::PathBuf, data: Vec<u8>) -> std::io::Result<()> {
    tokio::task::spawn_blocking(move || atomic_write(&path, &data)).await?
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_atomic_write_creates_file() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        atomic_write(&file_path, b"hello world").unwrap();
        assert_eq!(fs::read(&file_path).unwrap(), b"hello world");
        // No temp file should remain
        assert!(!file_path.with_extension("tmp").exists());
    }

    #[test]
    fn test_atomic_write_overwrites_existing() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, b"old content").unwrap();
        atomic_write(&file_path, b"new content").unwrap();
        assert_eq!(fs::read(&file_path).unwrap(), b"new content");
    }

    #[test]
    fn test_atomic_write_no_partial_on_error() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        // Write original content
        fs::write(&file_path, b"original").unwrap();

        // Try to write to a path inside a non-existent deeply nested dir
        // that would fail (can't create temp file)
        let bad_path = dir
            .path()
            .join("nonexistent")
            .join("subdir")
            .join("file.txt");
        let result = atomic_write(&bad_path, b"should fail");
        assert!(result.is_err());

        // Original file should be untouched
        assert_eq!(fs::read(&file_path).unwrap(), b"original");
    }

    #[test]
    fn test_atomic_write_empty_data() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("empty.txt");
        atomic_write(&file_path, b"").unwrap();
        assert_eq!(fs::read(&file_path).unwrap(), b"");
    }

    #[test]
    fn test_atomic_write_large_data() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("large.bin");
        let data = vec![0xAB_u8; 1024 * 1024]; // 1MB
        atomic_write(&file_path, &data).unwrap();
        assert_eq!(fs::read(&file_path).unwrap().len(), 1024 * 1024);
    }
}
