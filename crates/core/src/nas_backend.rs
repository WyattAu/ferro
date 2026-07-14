use async_trait::async_trait;
use bytes::Bytes;
use chrono::Utc;
use ferro_common::error::{FerroError, Result};
use ferro_common::metadata::{ContentHash, FileMetadata};
use ferro_common::storage::{StorageEngine, StorageReader};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::sync::RwLock;
use tracing::debug;

/// Maximum depth for `list_all` to prevent `DoS` on deeply nested trees.
const MAX_LIST_ALL_DEPTH: u32 = 100;

/// The number of retries for NAS operations that may fail due to stale file handles.
const NAS_RETRY_COUNT: u32 = 3;

/// Storage engine backed by a mounted NAS path (NFS, SMB/CIFS).
///
/// The engine operates on a local mount point, handling NAS-specific concerns:
/// - NFS: stale file handles, permission errors, file locking
/// - SMB/CIFS: case insensitivity, special characters, locked files
#[derive(Debug)]
pub struct NasStorageEngine {
    base_path: PathBuf,
    /// Normalized virtual root (e.g., "/" or "/subdir").
    #[allow(dead_code)]
    virtual_root: String,
    /// Detected protocol from the `nas:` prefix.
    #[allow(dead_code)]
    protocol: NasProtocol,
    /// Cached metadata to avoid repeated syscalls.
    metadata_cache: Arc<RwLock<std::collections::HashMap<String, CachedMeta>>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NasProtocol {
    Nfs,
    Smb,
    Auto,
}

impl NasProtocol {
    #[must_use]
    pub fn from_prefix(s: &str) -> Self {
        if s.contains("smb") || s.contains("cifs") || s.contains("samba") {
            Self::Smb
        } else {
            Self::Nfs
        }
    }
}

impl std::fmt::Display for NasProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Nfs => write!(f, "NFS"),
            Self::Smb => write!(f, "SMB/CIFS"),
            Self::Auto => write!(f, "NAS"),
        }
    }
}

#[derive(Debug, Clone)]
struct CachedMeta {
    metadata: FileMetadata,
    cached_at: std::time::Instant,
}

/// Configuration for NAS storage, parsed from `--storage` flag.
///
/// Accepted formats:
/// - `nas:/mnt/nas/ferro` — local mount point path
/// - `nas://server/share/ferro` — UNC-style path (mapped to local mount)
/// - `nas-nfs:/mnt/nas` — explicit NFS protocol
/// - `nas-smb:/mnt/share` — explicit SMB protocol
#[derive(Debug, Clone)]
pub struct NasStorageConfig {
    pub base_path: PathBuf,
    pub protocol: NasProtocol,
}

impl NasStorageConfig {
    /// Parse a `--storage` value into a NAS config.
    ///
    /// Returns `None` if the value doesn't match `nas:` prefix.
    #[must_use]
    pub fn parse(storage_str: &str) -> Option<Self> {
        let (protocol, path_part) = if let Some(rest) = storage_str.strip_prefix("nas-nfs:") {
            (NasProtocol::Nfs, rest)
        } else if let Some(rest) = storage_str.strip_prefix("nas-smb:") {
            (NasProtocol::Smb, rest)
        } else if let Some(rest) = storage_str.strip_prefix("nas:") {
            (NasProtocol::Auto, rest)
        } else {
            return None;
        };

        let base_path = Self::resolve_path(path_part);
        Some(Self { base_path, protocol })
    }

    /// Resolve a path, handling UNC-style `//server/share` paths by
    /// normalizing separators.
    fn resolve_path(raw: &str) -> PathBuf {
        let normalized = raw.replace('\\', "/");
        PathBuf::from(normalized)
    }

    /// Validate that the base path exists and is a directory.
    pub fn validate(&self) -> Result<()> {
        if !self.base_path.exists() {
            return Err(FerroError::StorageBackend(format!(
                "NAS base path does not exist: {}",
                self.base_path.display()
            )));
        }
        if !self.base_path.is_dir() {
            return Err(FerroError::StorageBackend(format!(
                "NAS base path is not a directory: {}",
                self.base_path.display()
            )));
        }
        let test_file = self.base_path.join(".ferro_nas_test");
        std::fs::write(&test_file, b"test").map_err(|e| {
            FerroError::StorageBackend(format!(
                "NAS base path is not writable: {} ({})",
                self.base_path.display(),
                e
            ))
        })?;
        std::fs::remove_file(&test_file).ok();
        Ok(())
    }
}

impl NasStorageEngine {
    /// Create a new NAS storage engine with the given configuration.
    pub fn new(config: &NasStorageConfig) -> Result<Self> {
        config.validate()?;

        let virtual_root = "/".to_string();

        Ok(Self {
            base_path: config.base_path.clone(),
            virtual_root,
            protocol: config.protocol,
            metadata_cache: Arc::new(RwLock::new(std::collections::HashMap::new())),
        })
    }

    /// Create a new NAS storage engine with a specific subdirectory as root.
    pub fn with_subroot(config: &NasStorageConfig, sub: &str) -> Result<Self> {
        config.validate()?;

        let base = config.base_path.join(sub.trim_start_matches('/'));
        if !base.exists() {
            std::fs::create_dir_all(&base)
                .map_err(|e| FerroError::StorageBackend(format!("Failed to create NAS subroot {base:?}: {e}")))?;
        }
        if !base.is_dir() {
            return Err(FerroError::StorageBackend(format!(
                "NAS subroot is not a directory: {base:?}"
            )));
        }

        let virtual_root = format!("/{}", sub.trim_start_matches('/').trim_end_matches('/'));

        Ok(Self {
            base_path: base,
            virtual_root,
            protocol: config.protocol,
            metadata_cache: Arc::new(RwLock::new(std::collections::HashMap::new())),
        })
    }

    /// Convert a virtual path to a local filesystem path.
    fn to_fs_path(&self, path: &str) -> PathBuf {
        let clean = path.trim_start_matches('/');
        if clean.is_empty() {
            self.base_path.clone()
        } else {
            self.base_path.join(clean)
        }
    }

    /// Validate a virtual path for security (no traversal).
    fn validate_path(path: &str) -> Result<()> {
        if path.contains("..") {
            return Err(FerroError::InvalidArgument("Path traversal not allowed".to_string()));
        }
        if path.is_empty() {
            return Err(FerroError::InvalidArgument("Path must not be empty".to_string()));
        }
        Ok(())
    }

    /// Map OS errors to appropriate `FerroError` variants with NAS-specific context.
    fn map_io_error(e: std::io::Error, path: &str, operation: &str) -> FerroError {
        match e.kind() {
            std::io::ErrorKind::NotFound => FerroError::NotFound(format!("{path}: {e}")),
            std::io::ErrorKind::PermissionDenied => FerroError::PermissionDenied(format!("{path}: {e}")),
            std::io::ErrorKind::BrokenPipe | std::io::ErrorKind::ConnectionReset => FerroError::StorageBackend(
                format!("NAS {operation} failed for {path}: stale file handle or connection lost ({e})"),
            ),
            std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut => {
                FerroError::StorageBackend(format!("NAS {operation} timed out for {path}: {e}"))
            }
            _ => FerroError::StorageBackend(format!("NAS {operation} failed for {path}: {e}")),
        }
    }

    /// Execute a filesystem operation with retries for stale file handles.
    async fn with_retry<F, Fut, T>(&self, path: &str, operation: &str, f: F) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = std::io::Result<T>>,
    {
        let mut last_err = None;
        for attempt in 0..NAS_RETRY_COUNT {
            match f().await {
                Ok(val) => return Ok(val),
                Err(e) => {
                    let retryable = matches!(
                        e.kind(),
                        std::io::ErrorKind::BrokenPipe
                            | std::io::ErrorKind::ConnectionReset
                            | std::io::ErrorKind::ConnectionRefused
                            | std::io::ErrorKind::ConnectionAborted
                    );
                    if retryable && attempt < NAS_RETRY_COUNT - 1 {
                        debug!(
                            "NAS {} retry {}/{} for {}: {}",
                            operation,
                            attempt + 1,
                            NAS_RETRY_COUNT,
                            path,
                            e
                        );
                        tokio::time::sleep(std::time::Duration::from_millis(50 * 2u64.pow(attempt))).await;
                    } else {
                        last_err = Some(e);
                    }
                }
            }
        }
        Err(Self::map_io_error(
            last_err.expect("retry loop must set last_err"),
            path,
            operation,
        ))
    }

    /// Build `FileMetadata` from filesystem metadata.
    fn fs_metadata_to_file_with_owner(
        &self,
        virtual_path: &str,
        fs_meta: &std::fs::Metadata,
        owner: &str,
    ) -> FileMetadata {
        let modified = fs_meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        let chrono_modified: chrono::DateTime<Utc> = modified.into();

        let is_dir = fs_meta.is_dir();

        if is_dir {
            FileMetadata {
                path: virtual_path.to_string(),
                etag: format!("\"col-{}\"", chrono_modified.timestamp_millis()),
                content_hash: ContentHash::new_unchecked("0".repeat(64)),
                size: 0,
                mime_type: "httpd/unix-directory".to_string(),
                is_collection: true,
                created_at: chrono_modified,
                modified_at: chrono_modified,
                owner: owner.to_string(),
            }
        } else {
            let size = fs_meta.len();
            let etag_str = format!("size-{}-mtime-{}", size, chrono_modified.timestamp_millis());
            let content_hash = ContentHash::compute(etag_str.as_bytes());

            FileMetadata {
                path: virtual_path.to_string(),
                etag: format!("\"{etag_str}\""),
                content_hash,
                size,
                mime_type: "application/octet-stream".to_string(),
                is_collection: false,
                created_at: chrono_modified,
                modified_at: chrono_modified,
                owner: owner.to_string(),
            }
        }
    }

    /// Build `FileMetadata` from filesystem metadata (owner unknown).
    fn fs_metadata_to_file(&self, virtual_path: &str, fs_meta: &std::fs::Metadata) -> FileMetadata {
        self.fs_metadata_to_file_with_owner(virtual_path, fs_meta, "unknown")
    }

    /// Invalidate cache entries that are stale (older than 30 seconds).
    async fn invalidate_stale_cache(&self) {
        let mut cache = self.metadata_cache.write().await;
        let now = std::time::Instant::now();
        cache.retain(|_, entry| now.duration_since(entry.cached_at) < std::time::Duration::from_secs(30));
    }

    /// SMB-specific: sanitize filename for case-insensitive filesystems.
    #[allow(dead_code)]
    fn smb_sanitize_name(name: &str) -> String {
        name.chars()
            .map(|c| match c {
                '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
                _ => c,
            })
            .collect()
    }
}

#[async_trait]
impl StorageEngine for NasStorageEngine {
    async fn put(&self, path: &str, content: Bytes, owner: &str) -> Result<FileMetadata> {
        Self::validate_path(path)?;

        let fs_path = self.to_fs_path(path);
        let content_clone = content.clone();
        let path_owned = path.to_string();
        let owner_owned = owner.to_string();

        self.with_retry(path, "put", || {
            let fs_path = fs_path.clone();
            let content = content_clone.clone();
            async move {
                if let Some(parent) = fs_path.parent() {
                    tokio::fs::create_dir_all(parent).await?;
                }
                tokio::fs::write(&fs_path, &content).await
            }
        })
        .await?;

        let fs_meta = tokio::fs::metadata(&fs_path)
            .await
            .map_err(|e| Self::map_io_error(e, path, "put"))?;

        let meta = self.fs_metadata_to_file_with_owner(&path_owned, &fs_meta, &owner_owned);

        {
            let mut cache = self.metadata_cache.write().await;
            cache.insert(
                path_owned.clone(),
                CachedMeta {
                    metadata: meta.clone(),
                    cached_at: std::time::Instant::now(),
                },
            );
        }

        debug!(
            "NAS PUT {} ({} bytes, owner={})",
            path_owned,
            content.len(),
            owner_owned
        );
        Ok(meta)
    }

    async fn get(&self, path: &str) -> Result<Bytes> {
        Self::validate_path(path)?;

        let fs_path = self.to_fs_path(path);

        let bytes = self
            .with_retry(path, "get", || {
                let fs_path = fs_path.clone();
                async move { tokio::fs::read(&fs_path).await }
            })
            .await?;

        Ok(Bytes::from(bytes))
    }

    async fn get_stream(&self, path: &str) -> Result<StorageReader> {
        Self::validate_path(path)?;

        let fs_path = self.to_fs_path(path);
        let path_owned = path.to_string();

        if !fs_path.exists() {
            return Err(FerroError::NotFound(path_owned));
        }

        let file = tokio::fs::File::open(&fs_path)
            .await
            .map_err(|e| Self::map_io_error(e, &path_owned, "get_stream"))?;

        let reader = tokio::io::BufReader::new(file);
        Ok(StorageReader::new(Box::pin(reader)))
    }

    async fn delete(&self, path: &str) -> Result<()> {
        Self::validate_path(path)?;

        let fs_path = self.to_fs_path(path);
        let path_owned = path.to_string();

        self.invalidate_stale_cache().await;

        self.with_retry(path, "delete", || {
            let fs_path = fs_path.clone();
            async move {
                if fs_path.is_dir() {
                    tokio::fs::remove_dir(&fs_path).await
                } else {
                    tokio::fs::remove_file(&fs_path).await
                }
            }
        })
        .await?;

        {
            let mut cache = self.metadata_cache.write().await;
            cache.remove(&path_owned);
        }

        debug!("NAS DELETE {}", path_owned);
        Ok(())
    }

    async fn list(&self, prefix: &str) -> Result<Vec<FileMetadata>> {
        Self::validate_path(prefix)?;

        let fs_path = self.to_fs_path(prefix);
        let path_owned = prefix.to_string();

        if !fs_path.exists() {
            return Ok(Vec::new());
        }

        let entries = self
            .with_retry(prefix, "list", || {
                let fs_path = fs_path.clone();
                async move {
                    let mut entries = Vec::new();
                    let mut dir = tokio::fs::read_dir(&fs_path).await?;
                    while let Some(entry) = dir.next_entry().await? {
                        entries.push(entry);
                    }
                    Ok::<_, std::io::Error>(entries)
                }
            })
            .await?;

        let mut items = Vec::with_capacity(entries.len());
        for entry in &entries {
            let entry_name = entry.file_name();
            let name_str = entry_name.to_string_lossy();
            let virtual_path = if path_owned == "/" || path_owned.is_empty() {
                format!("/{name_str}")
            } else {
                format!("{}/{}", path_owned.trim_end_matches('/'), name_str)
            };

            match tokio::fs::metadata(entry.path()).await {
                Ok(meta) => {
                    items.push(self.fs_metadata_to_file(&virtual_path, &meta));
                }
                Err(e) => {
                    debug!("NAS list: skipping entry {} ({})", virtual_path, e);
                }
            }
        }

        items.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(items)
    }

    async fn copy(&self, from: &str, to: &str) -> Result<()> {
        Self::validate_path(from)?;
        Self::validate_path(to)?;

        let from_fs = self.to_fs_path(from);
        let to_fs = self.to_fs_path(to);
        let from_owned = from.to_string();
        let to_owned = to.to_string();

        self.invalidate_stale_cache().await;

        self.with_retry(from, "copy", || {
            let from_fs = from_fs.clone();
            let to_fs = to_fs.clone();
            async move {
                if from_fs.is_dir() {
                    let parent = to_fs.parent().unwrap_or(&to_fs);
                    tokio::fs::create_dir_all(parent).await?;
                    copy_dir_recursive(&from_fs, &to_fs).await
                } else {
                    if let Some(parent) = to_fs.parent() {
                        tokio::fs::create_dir_all(parent).await?;
                    }
                    tokio::fs::copy(&from_fs, &to_fs).await?;
                    Ok(())
                }
            }
        })
        .await?;

        debug!("NAS COPY {} -> {}", from_owned, to_owned);
        Ok(())
    }

    async fn move_path(&self, from: &str, to: &str) -> Result<()> {
        Self::validate_path(from)?;
        Self::validate_path(to)?;

        let from_fs = self.to_fs_path(from);
        let to_fs = self.to_fs_path(to);
        let from_owned = from.to_string();
        let to_owned = to.to_string();

        self.invalidate_stale_cache().await;

        self.with_retry(from, "move", || {
            let from_fs = from_fs.clone();
            let to_fs = to_fs.clone();
            async move {
                if let Some(parent) = to_fs.parent() {
                    tokio::fs::create_dir_all(parent).await?;
                }
                tokio::fs::rename(&from_fs, &to_fs).await
            }
        })
        .await?;

        {
            let mut cache = self.metadata_cache.write().await;
            cache.remove(&from_owned);
        }

        debug!("NAS MOVE {} -> {}", from_owned, to_owned);
        Ok(())
    }

    async fn head(&self, path: &str) -> Result<FileMetadata> {
        Self::validate_path(path)?;

        {
            let cache = self.metadata_cache.read().await;
            if let Some(entry) = cache.get(path)
                && entry.cached_at.elapsed() < std::time::Duration::from_secs(10)
            {
                return Ok(entry.metadata.clone());
            }
        }

        let fs_path = self.to_fs_path(path);
        let path_owned = path.to_string();

        let fs_meta = tokio::fs::metadata(&fs_path)
            .await
            .map_err(|e| Self::map_io_error(e, &path_owned, "head"))?;

        let meta = self.fs_metadata_to_file(&path_owned, &fs_meta);

        {
            let mut cache = self.metadata_cache.write().await;
            cache.insert(
                path_owned,
                CachedMeta {
                    metadata: meta.clone(),
                    cached_at: std::time::Instant::now(),
                },
            );
        }

        Ok(meta)
    }

    async fn exists(&self, path: &str) -> Result<bool> {
        Self::validate_path(path)?;

        let fs_path = self.to_fs_path(path);
        Ok(fs_path.exists())
    }

    async fn create_collection(&self, path: &str, owner: &str) -> Result<FileMetadata> {
        Self::validate_path(path)?;

        let fs_path = self.to_fs_path(path);
        let path_owned = path.to_string();
        let owner_owned = owner.to_string();

        self.with_retry(path, "create_collection", || {
            let fs_path = fs_path.clone();
            async move { tokio::fs::create_dir_all(&fs_path).await }
        })
        .await?;

        let meta = FileMetadata::new_collection(path_owned.clone(), owner_owned);
        debug!("NAS MKCOL {}", path_owned);
        Ok(meta)
    }

    async fn list_all(&self, prefix: &str, max_depth: u32) -> Result<Vec<FileMetadata>> {
        Self::validate_path(prefix)?;

        let effective_depth = max_depth.min(MAX_LIST_ALL_DEPTH);
        let base = if prefix == "/" {
            PathBuf::new()
        } else {
            PathBuf::from(prefix.trim_start_matches('/'))
        };
        let base_prefix = prefix.to_string();

        let mut items = Vec::new();
        self.list_all_recursive(&base, &base_prefix, 0, effective_depth, &mut items)
            .await?;
        Ok(items)
    }

    async fn put_multipart(&self, path: &str, content: Bytes, owner: &str) -> Result<FileMetadata> {
        self.put(path, content, owner).await
    }

    async fn put_stream(
        &self,
        path: &str,
        mut reader: std::pin::Pin<Box<dyn tokio::io::AsyncRead + Send>>,
        size: u64,
        owner: &str,
    ) -> Result<FileMetadata> {
        Self::validate_path(path)?;

        let fs_path = self.to_fs_path(path);
        let path_owned = path.to_string();
        let owner_owned = owner.to_string();

        if let Some(parent) = fs_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| Self::map_io_error(e, &path_owned, "put_stream"))?;
        }

        let mut buf = Vec::with_capacity(size as usize);
        reader
            .read_to_end(&mut buf)
            .await
            .map_err(|e| FerroError::StorageBackend(format!("Stream read error: {e}")))?;

        let bytes = Bytes::from(buf);
        self.put(path, bytes, &owner_owned).await
    }

    fn supports_put_stream(&self) -> bool {
        true
    }

    fn supports_multipart(&self) -> bool {
        false
    }
}

impl NasStorageEngine {
    /// Recursive helper for `list_all`.
    async fn list_all_recursive(
        &self,
        relative_base: &Path,
        virtual_prefix: &str,
        current_depth: u32,
        max_depth: u32,
        items: &mut Vec<FileMetadata>,
    ) -> Result<()> {
        if current_depth >= max_depth {
            return Ok(());
        }

        let fs_path = self.base_path.join(relative_base);
        if !fs_path.exists() {
            return Ok(());
        }

        let entries = self
            .with_retry(virtual_prefix, "list_all_recursive", || {
                let fs_path = fs_path.clone();
                async move {
                    let mut entries = Vec::new();
                    if let Ok(mut dir) = tokio::fs::read_dir(&fs_path).await {
                        while let Some(entry) = dir.next_entry().await? {
                            entries.push(entry);
                        }
                    }
                    Ok::<_, std::io::Error>(entries)
                }
            })
            .await?;

        for entry in &entries {
            let entry_name = entry.file_name();
            let name_str = entry_name.to_string_lossy();

            let virtual_path = if virtual_prefix == "/" || virtual_prefix.is_empty() {
                format!("/{name_str}")
            } else {
                format!("{}/{}", virtual_prefix.trim_end_matches('/'), name_str)
            };

            if virtual_path == virtual_prefix {
                continue;
            }

            match tokio::fs::metadata(entry.path()).await {
                Ok(meta) => {
                    if meta.is_dir() {
                        // Recurse into directories but don't add them to results
                        let child_relative = relative_base.join(&*name_str);
                        Box::pin(self.list_all_recursive(
                            &child_relative,
                            &virtual_path,
                            current_depth + 1,
                            max_depth,
                            items,
                        ))
                        .await?;
                    } else {
                        items.push(self.fs_metadata_to_file(&virtual_path, &meta));
                    }
                }
                Err(e) => {
                    debug!("NAS list_all: skipping {} ({})", virtual_path, e);
                }
            }
        }

        Ok(())
    }
}

/// Recursively copy a directory tree.
async fn copy_dir_recursive(from: &Path, to: &Path) -> std::io::Result<()> {
    tokio::fs::create_dir_all(to).await?;
    let mut entries = tokio::fs::read_dir(from).await?;
    while let Some(entry) = entries.next_entry().await? {
        let file_type = entry.file_type().await?;
        let dest = to.join(entry.file_name());
        if file_type.is_dir() {
            Box::pin(copy_dir_recursive(&entry.path(), &dest)).await?;
        } else {
            tokio::fs::copy(entry.path(), &dest).await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_test_engine() -> (NasStorageEngine, TempDir) {
        let tmp = TempDir::new().unwrap();
        let config = NasStorageConfig {
            base_path: tmp.path().to_path_buf(),
            protocol: NasProtocol::Auto,
        };
        let engine = NasStorageEngine::new(&config).unwrap();
        (engine, tmp)
    }

    #[tokio::test]
    async fn test_put_get_roundtrip() {
        let (engine, _tmp) = make_test_engine();
        let content = Bytes::from("hello world");

        let meta = engine.put("/test.txt", content.clone(), "user1").await.unwrap();
        assert_eq!(meta.path, "/test.txt");
        assert_eq!(meta.size, 11);
        assert_eq!(meta.owner, "user1");

        let retrieved = engine.get("/test.txt").await.unwrap();
        assert_eq!(content, retrieved);
    }

    #[tokio::test]
    async fn test_put_delete() {
        let (engine, _tmp) = make_test_engine();
        engine.put("/test.txt", Bytes::from("hello"), "user1").await.unwrap();
        assert!(engine.exists("/test.txt").await.unwrap());

        engine.delete("/test.txt").await.unwrap();
        assert!(!engine.exists("/test.txt").await.unwrap());
    }

    #[tokio::test]
    async fn test_create_collection_and_list() {
        let (engine, _tmp) = make_test_engine();
        engine.put("/docs/a.txt", Bytes::from("aaa"), "user1").await.unwrap();
        engine.put("/docs/b.txt", Bytes::from("bbb"), "user1").await.unwrap();

        let items = engine.list("/docs").await.unwrap();
        assert_eq!(items.len(), 2);
    }

    #[tokio::test]
    async fn test_copy() {
        let (engine, _tmp) = make_test_engine();
        engine.put("/src.txt", Bytes::from("hello"), "user1").await.unwrap();

        engine.copy("/src.txt", "/dst.txt").await.unwrap();
        assert!(engine.exists("/src.txt").await.unwrap());
        assert!(engine.exists("/dst.txt").await.unwrap());

        let src = engine.get("/src.txt").await.unwrap();
        let dst = engine.get("/dst.txt").await.unwrap();
        assert_eq!(src, dst);
    }

    #[tokio::test]
    async fn test_move_path() {
        let (engine, _tmp) = make_test_engine();
        engine.put("/old.txt", Bytes::from("hello"), "user1").await.unwrap();

        engine.move_path("/old.txt", "/new.txt").await.unwrap();
        assert!(!engine.exists("/old.txt").await.unwrap());
        assert!(engine.exists("/new.txt").await.unwrap());

        let content = engine.get("/new.txt").await.unwrap();
        assert_eq!(content, Bytes::from("hello"));
    }

    #[tokio::test]
    async fn test_exists() {
        let (engine, _tmp) = make_test_engine();
        assert!(!engine.exists("/nope.txt").await.unwrap());

        engine.put("/yes.txt", Bytes::from("data"), "user1").await.unwrap();
        assert!(engine.exists("/yes.txt").await.unwrap());
    }

    #[tokio::test]
    async fn test_nested_collections() {
        let (engine, _tmp) = make_test_engine();
        engine
            .put("/a/b/c/file.txt", Bytes::from("nested"), "user1")
            .await
            .unwrap();

        let items = engine.list("/a/b").await.unwrap();
        assert!(!items.is_empty());
    }

    #[tokio::test]
    async fn test_list_all_descendants() {
        let (engine, _tmp) = make_test_engine();
        engine.put("/root/f1.txt", Bytes::from("a"), "user1").await.unwrap();
        engine.put("/root/sub/f2.txt", Bytes::from("b"), "user1").await.unwrap();
        engine
            .put("/root/sub/deep/f3.txt", Bytes::from("c"), "user1")
            .await
            .unwrap();

        let all = engine.list_all("/root", 100).await.unwrap();
        assert_eq!(all.len(), 3);
    }

    #[tokio::test]
    async fn test_list_all_depth_limit() {
        let (engine, _tmp) = make_test_engine();
        engine.put("/root/f1.txt", Bytes::from("a"), "user1").await.unwrap();
        engine.put("/root/sub/f2.txt", Bytes::from("b"), "user1").await.unwrap();
        engine
            .put("/root/sub/deep/f3.txt", Bytes::from("c"), "user1")
            .await
            .unwrap();

        let items = engine.list_all("/root", 1).await.unwrap();
        assert_eq!(items.len(), 1);

        let items = engine.list_all("/root", 100).await.unwrap();
        assert_eq!(items.len(), 3);
    }

    #[tokio::test]
    async fn test_head_metadata() {
        let (engine, _tmp) = make_test_engine();
        engine.put("/meta.txt", Bytes::from("data"), "user1").await.unwrap();

        let meta = engine.head("/meta.txt").await.unwrap();
        assert_eq!(meta.path, "/meta.txt");
        assert_eq!(meta.size, 4);
        assert!(!meta.is_collection);
    }

    #[tokio::test]
    async fn test_path_traversal_rejected() {
        let (engine, _tmp) = make_test_engine();
        let result = engine.get("/../etc/passwd").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_path_traversal_double_dot() {
        let (engine, _tmp) = make_test_engine();
        let result = engine.put("/../../etc/shadow", Bytes::from("bad"), "user1").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_stream() {
        let (engine, _tmp) = make_test_engine();
        let content = Bytes::from("streaming test data");
        engine.put("/stream.txt", content.clone(), "user1").await.unwrap();

        let mut reader = engine.get_stream("/stream.txt").await.unwrap();
        let mut buf = vec![0u8; 64];
        let n = reader.read(&mut buf).await.unwrap();
        assert_eq!(n, content.len());
        assert_eq!(&buf[..n], &content[..]);
    }

    #[tokio::test]
    async fn test_get_stream_not_found() {
        let (engine, _tmp) = make_test_engine();
        let result = engine.get_stream("/nonexistent.txt").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_put_overwrite() {
        let (engine, _tmp) = make_test_engine();
        engine.put("/test.txt", Bytes::from("v1"), "user1").await.unwrap();
        engine.put("/test.txt", Bytes::from("v2"), "user1").await.unwrap();

        let content = engine.get("/test.txt").await.unwrap();
        assert_eq!(content, Bytes::from("v2"));
    }

    #[tokio::test]
    async fn test_nas_config_parse() {
        let config = NasStorageConfig::parse("nas:/mnt/nas/ferro").unwrap();
        assert_eq!(config.base_path, PathBuf::from("/mnt/nas/ferro"));
        assert_eq!(config.protocol, NasProtocol::Auto);

        let config = NasStorageConfig::parse("nas-nfs:/mnt/nas").unwrap();
        assert_eq!(config.protocol, NasProtocol::Nfs);

        let config = NasStorageConfig::parse("nas-smb:/mnt/share").unwrap();
        assert_eq!(config.protocol, NasProtocol::Smb);

        assert!(NasStorageConfig::parse("memory").is_none());
        assert!(NasStorageConfig::parse("local:/path").is_none());
    }

    #[tokio::test]
    async fn test_create_collection() {
        let (engine, _tmp) = make_test_engine();
        let meta = engine.create_collection("/docs", "user1").await.unwrap();
        assert!(meta.is_collection);
        assert!(engine.exists("/docs").await.unwrap());
    }

    #[tokio::test]
    async fn test_head_collection() {
        let (engine, _tmp) = make_test_engine();
        engine.create_collection("/docs", "user1").await.unwrap();

        let meta = engine.head("/docs").await.unwrap();
        assert!(meta.is_collection);
        assert_eq!(meta.size, 0);
    }

    #[tokio::test]
    async fn test_copy_directory() {
        let (engine, _tmp) = make_test_engine();
        engine
            .put("/src/file.txt", Bytes::from("hello"), "user1")
            .await
            .unwrap();

        engine.copy("/src", "/dst").await.unwrap();
        assert!(engine.exists("/dst/file.txt").await.unwrap());

        let content = engine.get("/dst/file.txt").await.unwrap();
        assert_eq!(content, Bytes::from("hello"));
    }

    #[tokio::test]
    async fn test_delete_not_found() {
        let (engine, _tmp) = make_test_engine();
        let result = engine.delete("/nonexistent.txt").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_smb_sanitize() {
        assert_eq!(NasStorageEngine::smb_sanitize_name("file.txt"), "file.txt");
        assert_eq!(NasStorageEngine::smb_sanitize_name("file:name.txt"), "file_name.txt");
        assert_eq!(NasStorageEngine::smb_sanitize_name("a/b"), "a_b");
    }
}
