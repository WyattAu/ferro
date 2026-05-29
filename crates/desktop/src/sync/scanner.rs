//! Local filesystem scanner.
//!
//! Walks the local sync directory and computes SHA-256 hashes for all files.
//! Returns a map of relative_path -> (hash, size, mtime_ms, is_dir).

use anyhow::Result;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;
use walkdir::WalkDir;

/// Result of scanning the local filesystem.
pub struct LocalScanResult {
    /// Map of relative_path -> (sha256_hex, size_bytes, mtime_epoch_ms, is_dir).
    pub files: HashMap<String, (String, u64, i64, bool)>,
    /// Number of files scanned.
    pub file_count: usize,
    /// Number of directories scanned.
    pub dir_count: usize,
    /// Total bytes scanned.
    pub total_bytes: u64,
    /// Duration of the scan in milliseconds.
    pub scan_duration_ms: u64,
}

/// Scan a local directory recursively, computing SHA-256 hashes for all files.
///
/// Excludes:
/// - Hidden files/dirs starting with `.`
/// - The sync state file `.ferro-sync-state.json`
/// - Files larger than `max_file_size` bytes (default: 10 GB)
pub fn scan_local(local_root: &Path, max_file_size: u64) -> Result<LocalScanResult> {
    let start = std::time::Instant::now();
    let mut files = HashMap::new();
    let mut file_count = 0usize;
    let mut dir_count = 0usize;
    let mut total_bytes = 0u64;

    if !local_root.exists() {
        std::fs::create_dir_all(local_root)?;
        return Ok(LocalScanResult {
            files,
            file_count: 0,
            dir_count: 0,
            total_bytes: 0,
            scan_duration_ms: start.elapsed().as_millis() as u64,
        });
    }

    for entry in WalkDir::new(local_root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            // Skip hidden files and dirs
            let name = e.file_name().to_string_lossy();
            if name.starts_with('.') {
                return false;
            }
            true
        })
    {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!(error = %e, "skipping inaccessible entry during local scan");
                continue;
            }
        };

        let relative = match entry.path().strip_prefix(local_root) {
            Ok(r) => r,
            Err(_) => continue,
        };

        // Skip the root itself
        if relative.as_os_str().is_empty() {
            continue;
        }

        // Use forward slashes for consistency with remote paths
        let relative_path = relative.to_string_lossy().replace('\\', "/");

        if entry.file_type().is_dir() {
            dir_count += 1;
            files.insert(relative_path, (String::new(), 0, 0, true));
        } else if entry.file_type().is_file() {
            let metadata = match entry.metadata() {
                Ok(m) => m,
                Err(e) => {
                    tracing::warn!(path = %relative_path, error = %e, "skipping file: metadata error");
                    continue;
                }
            };

            let size = metadata.len();
            if size > max_file_size {
                tracing::warn!(
                    path = %relative_path,
                    size,
                    max = max_file_size,
                    "skipping file: exceeds size limit"
                );
                continue;
            }

            let mtime_ms = metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0);

            // Compute SHA-256 hash
            let hash = match compute_file_hash(entry.path()) {
                Ok(h) => h,
                Err(e) => {
                    tracing::warn!(path = %relative_path, error = %e, "skipping file: hash error");
                    continue;
                }
            };

            total_bytes += size;
            file_count += 1;
            files.insert(relative_path, (hash, size, mtime_ms, false));
        }
    }

    Ok(LocalScanResult {
        files,
        file_count,
        dir_count,
        total_bytes,
        scan_duration_ms: start.elapsed().as_millis() as u64,
    })
}

/// Compute the SHA-256 hash of a file, returning the hex-encoded digest.
fn compute_file_hash(path: &Path) -> Result<String> {
    let mut hasher = Sha256::new();
    let mut file = std::fs::File::open(path)?;
    std::io::copy(&mut file, &mut hasher)?;
    let result = hasher.finalize();
    Ok(hex::encode(result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_scan_empty_directory() {
        let dir = std::env::temp_dir().join("ferro-scan-test-empty");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let result = scan_local(&dir, 10_000_000_000).unwrap();
        assert_eq!(result.file_count, 0);
        assert_eq!(result.dir_count, 0);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_scan_with_files() {
        let dir = std::env::temp_dir().join("ferro-scan-test-files");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        // Create test files
        std::fs::write(dir.join("hello.txt"), b"hello world").unwrap();
        std::fs::create_dir(dir.join("subdir")).unwrap();
        std::fs::write(dir.join("subdir/nested.txt"), b"nested content").unwrap();

        // Create hidden file (should be skipped)
        std::fs::write(dir.join(".hidden"), b"hidden").unwrap();

        let result = scan_local(&dir, 10_000_000_000).unwrap();
        assert_eq!(result.file_count, 2); // hello.txt + nested.txt
        assert!(result.files.contains_key("hello.txt"));
        assert!(result.files.contains_key("subdir/nested.txt"));
        assert!(!result.files.contains_key(".hidden"));

        // Verify hash is correct
        let (hash, size, _, is_dir) = result.files.get("hello.txt").unwrap();
        assert_eq!(*size, 11);
        assert!(!is_dir);
        // SHA-256 of "hello world"
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_scan_max_file_size() {
        let dir = std::env::temp_dir().join("ferro-scan-test-max");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        // Create a file that exceeds the limit
        std::fs::write(dir.join("big.txt"), b"x".repeat(100)).unwrap();

        let result = scan_local(&dir, 50).unwrap(); // max 50 bytes
        assert_eq!(result.file_count, 0); // skipped due to size

        let _ = std::fs::remove_dir_all(&dir);
    }
}
