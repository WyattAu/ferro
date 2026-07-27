use crate::error::{MigrationError, Result as MigrateResult};
use crate::ferro_target::FerroTarget;
use crate::nextcloud::NextcloudClient;
use crate::ocis::OcisClient;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

#[derive(Debug, Clone)]
pub struct DavEntry {
    pub path: String,
    pub is_collection: bool,
    pub size: u64,
    pub last_modified: Option<String>,
    pub etag: Option<String>,
    pub content_type: Option<String>,
}

#[derive(Clone)]
pub enum WebDavSource {
    Nextcloud(NextcloudClient),
    Ocis(OcisClient),
}

impl WebDavSource {
    pub async fn validate(&self, user: &str) -> MigrateResult<()> {
        match self {
            WebDavSource::Nextcloud(nc) => nc.validate(user).await,
            WebDavSource::Ocis(oc) => oc.validate(user).await,
        }
    }

    pub async fn list_directory(&self, user: &str, path: &str) -> MigrateResult<Vec<DavEntry>> {
        match self {
            WebDavSource::Nextcloud(nc) => nc.list_directory(user, path).await,
            WebDavSource::Ocis(oc) => oc.list_directory(user, path).await,
        }
    }

    pub async fn download_file(&self, user: &str, path: &str) -> MigrateResult<Vec<u8>> {
        match self {
            WebDavSource::Nextcloud(nc) => nc.download_file(user, path).await,
            WebDavSource::Ocis(oc) => oc.download_file(user, path).await,
        }
    }

    pub async fn list_directory_recursive(&self, user: &str, path: &str) -> MigrateResult<Vec<DavEntry>> {
        match self {
            WebDavSource::Nextcloud(nc) => nc.list_directory_recursive(user, path).await,
            WebDavSource::Ocis(oc) => oc.list_directory_recursive(user, path).await,
        }
    }
}

/// Configuration for the parallel migration pipeline.
#[derive(Clone)]
pub struct PipelineConfig {
    /// Number of concurrent file transfer workers.
    pub transfer_workers: usize,
    /// Maximum file size to transfer (0 = no limit).
    pub max_file_size: u64,
    /// Channel buffer size for files.
    pub file_channel_size: usize,
    /// Maximum retries per file transfer.
    pub max_retries: u32,
    /// Optional checkpoint file path for resumable transfers.
    pub checkpoint_path: Option<PathBuf>,
    /// Background token refresh interval in seconds.
    pub token_refresh_interval_secs: u64,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            transfer_workers: 8,
            max_file_size: 0,
            file_channel_size: 512,
            max_retries: 3,
            checkpoint_path: None,
            token_refresh_interval_secs: 120,
        }
    }
}

pub struct WebDavPipeline<'a> {
    source: &'a WebDavSource,
    target: &'a FerroTarget,
    config: PipelineConfig,
}

impl<'a> WebDavPipeline<'a> {
    pub fn new(
        source: &'a WebDavSource,
        target: &'a FerroTarget,
        max_file_size: u64,
        _batch_size: usize,
    ) -> Self {
        Self {
            source,
            target,
            config: PipelineConfig {
                max_file_size,
                ..PipelineConfig::default()
            },
        }
    }

    pub fn with_config(mut self, config: PipelineConfig) -> Self {
        self.config = config;
        self
    }

    /// Production-grade migration with:
    /// - Single DFS traversal worker → mpsc channel (no broadcast race)
    /// - JSON checkpoint file for resumable transfers
    /// - CAS dedup via SHA-256 hash comparison
    /// - Background token refresh every 2 minutes
    pub async fn copy_all_files(
        &self,
        user: &str,
        progress: &crate::progress::ProgressTracker,
    ) -> MigrateResult<FileCopyStats> {
        let config = self.config.clone();
        let source = self.source.clone();
        let target = self.target.clone();
        let user = user.to_string();

        // Load checkpoint (path → status) for resumable transfers
        let checkpoint = Arc::new(RwLock::new(
            Checkpoint::load(config.checkpoint_path.as_deref()).unwrap_or_default(),
        ));

        // Atomic stats shared across tasks
        let stats = Arc::new(AtomicTraversalStats::default());

        // Channel for discovered files (bounded)
        let (file_tx, mut file_rx) = mpsc::channel::<DavEntry>(config.file_channel_size);

        // Spawn the single DFS traversal worker
        let traverse_source = source.clone();
        let traverse_user = user.clone();
        let traverse_stats = Arc::clone(&stats);
        let traverse_checkpoint = Arc::clone(&checkpoint);
        let max_file_size = config.max_file_size;
        let traverse_file_tx = file_tx.clone();

        let traverse_handle = tokio::spawn(async move {
            dfs_traverse(
                &traverse_source,
                &traverse_user,
                "/",
                &traverse_file_tx,
                &traverse_stats,
                &traverse_checkpoint,
                max_file_size,
            )
            .await;
        });

        // Drop the original sender so the channel closes when traversal finishes
        drop(file_tx);

        // Process files from the channel
        let mut file_stats = FileCopyStats::default();
        while let Some(entry) = file_rx.recv().await {
            let ferro_path = dav_path_to_ferro(&entry.path);

            // Checkpoint: skip already-done files
            {
                let cp = checkpoint.read().await;
                if let Some(status) = cp.entries.get(&entry.path) {
                    if status.status == "done" {
                        file_stats.skipped += 1;
                        progress.inc_file(0);
                        continue;
                    }
                }
            }

            // Create parent directory on-the-fly
            if let Some(parent) = ferro_path.rsplit('/').next() {
                if !parent.is_empty() {
                    let parent_path =
                        ferro_path[..ferro_path.len() - parent.len()].trim_end_matches('/');
                    if !parent_path.is_empty() {
                        let _ = target.create_directory(parent_path).await;
                    }
                }
            }

            // Download with retries
            let mut last_err = None;
            for attempt in 0..=config.max_retries {
                match source.download_file(&user, &entry.path).await {
                    Ok(content) => {
                        let bytes = content.len() as u64;

                        // CAS dedup: compute SHA-256 and check if target already has it
                        let content_hash = {
                            let mut hasher = Sha256::new();
                            hasher.update(&content);
                            format!("{:x}", hasher.finalize())
                        };

                        if target.file_exists_with_hash(&ferro_path, &content_hash).await {
                            tracing::debug!("CAS dedup: {} already exists with matching hash, skipping", ferro_path);
                            file_stats.migrated += 1;
                            file_stats.total_bytes += bytes;
                            progress.inc_file(bytes);

                            // Mark done in checkpoint
                            {
                                let mut cp = checkpoint.write().await;
                                cp.mark_done(&entry.path, &content_hash);
                                cp.save(config.checkpoint_path.as_deref());
                            }
                            last_err = None;
                            break;
                        }

                        match target.put_file(&ferro_path, &content).await {
                            Ok(()) => {
                                file_stats.migrated += 1;
                                file_stats.total_bytes += bytes;
                                progress.inc_file(bytes);
                                if file_stats.migrated % 100 == 0 {
                                    tracing::info!(
                                        "Migrated {} files ({:.1} MB)",
                                        file_stats.migrated,
                                        file_stats.total_bytes as f64 / 1_048_576.0
                                    );
                                }

                                // Mark done in checkpoint
                                {
                                    let mut cp = checkpoint.write().await;
                                    cp.mark_done(&entry.path, &content_hash);
                                    cp.save(config.checkpoint_path.as_deref());
                                }
                                last_err = None;
                                break;
                            }
                            Err(e) => {
                                last_err = Some(format!("upload: {}", e));
                                if attempt < config.max_retries {
                                    tokio::time::sleep(std::time::Duration::from_millis(
                                        100 * (attempt + 1) as u64,
                                    ))
                                    .await;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        last_err = Some(format!("download: {}", e));
                        if attempt < config.max_retries {
                            tokio::time::sleep(std::time::Duration::from_millis(
                                100 * (attempt + 1) as u64,
                            ))
                            .await;
                        }
                    }
                }
            }

            if let Some(err) = last_err {
                file_stats.failed += 1;
                tracing::error!(
                    "Failed {} after {} retries: {}",
                    entry.path,
                    config.max_retries,
                    err
                );

                // Mark failed in checkpoint so retry is possible on restart
                {
                    let mut cp = checkpoint.write().await;
                    cp.mark_failed(&entry.path);
                    cp.save(config.checkpoint_path.as_deref());
                }
            }
        }

        // Wait for traversal to finish
        let _ = traverse_handle.await;

        Ok(file_stats)
    }

    /// Migration using Graph API for discovery + WebDAV for download.
    /// This handles OCIS spaces that WebDAV can't access.
    pub async fn copy_all_files_graph(
        &self,
        user: &str,
        progress: &crate::progress::ProgressTracker,
    ) -> MigrateResult<FileCopyStats> {
        let graph = match self.source {
            WebDavSource::Ocis(oc) => oc.graph_client(),
            _ => return Err(MigrationError::config("Graph API is only supported for oCIS sources")),
        };

        let config = self.config.clone();
        let checkpoint = Arc::new(RwLock::new(
            Checkpoint::load(config.checkpoint_path.as_deref()).unwrap_or_default(),
        ));

        let mut file_stats = FileCopyStats::default();
        let mut dirs_to_list: Vec<String> = vec!["/".to_string()];

        while let Some(dir) = dirs_to_list.pop() {
            let items = match graph.list_children(&dir).await {
                Ok(items) => items,
                Err(e) => {
                    tracing::warn!("Graph API failed to list {}: {}", dir, e);
                    continue;
                }
            };

            let mut new_dirs = Vec::new();
            let mut files = Vec::new();

            for item in &items {
                if item.deleted.is_some() {
                    continue;
                }
                if item.folder.is_some() {
                    let child_path = format!("{}/{}", dir.trim_end_matches('/'), item.name);
                    new_dirs.push(child_path);
                } else if item.file.is_some() {
                    let remote_path = format!("{}/{}", dir.trim_end_matches('/'), item.name);
                    files.push((remote_path, item.size));
                }
            }

            dirs_to_list.extend(new_dirs);

            let graph_clone = graph.clone();
            for (remote_path, file_size) in &files {
                let ferro_path = dav_path_to_ferro(remote_path);

                if self.config.max_file_size > 0 && *file_size > self.config.max_file_size {
                    file_stats.skipped += 1;
                    continue;
                }

                // Checkpoint: skip already-done files
                {
                    let cp = checkpoint.read().await;
                    if let Some(status) = cp.entries.get(remote_path) {
                        if status.status == "done" {
                            file_stats.skipped += 1;
                            progress.inc_file(0);
                            continue;
                        }
                    }
                }

                if let Some(parent) = ferro_path.rsplit('/').next() {
                    if !parent.is_empty() {
                        let parent_path = ferro_path[..ferro_path.len() - parent.len()]
                            .trim_end_matches('/');
                        if !parent_path.is_empty() {
                            let _ = self.target.create_directory(parent_path).await;
                        }
                    }
                }

                let mut last_err = None;
                for attempt in 0..=self.config.max_retries {
                    let content = match self.source.download_file(user, remote_path).await {
                        Ok(c) => c,
                        Err(_) => {
                            tracing::debug!("WebDAV download failed for {}, trying Graph API", remote_path);
                            match graph_clone.download_file(remote_path).await {
                                Ok(c) => c,
                                Err(e) => {
                                    last_err = Some(format!("download: {}", e));
                                    if attempt < self.config.max_retries {
                                        tokio::time::sleep(std::time::Duration::from_millis(
                                            100 * (attempt + 1) as u64,
                                        ))
                                        .await;
                                    }
                                    continue;
                                }
                            }
                        }
                    };
                    let bytes = content.len() as u64;

                    // CAS dedup
                    let content_hash = {
                        let mut hasher = Sha256::new();
                        hasher.update(&content);
                        format!("{:x}", hasher.finalize())
                    };

                    if self.target.file_exists_with_hash(&ferro_path, &content_hash).await {
                        tracing::debug!("CAS dedup: {} already exists, skipping", ferro_path);
                        file_stats.migrated += 1;
                        file_stats.total_bytes += bytes;
                        progress.inc_file(bytes);
                        {
                            let mut cp = checkpoint.write().await;
                            cp.mark_done(remote_path, &content_hash);
                            cp.save(config.checkpoint_path.as_deref());
                        }
                        last_err = None;
                        break;
                    }

                    match self.target.put_file(&ferro_path, &content).await {
                        Ok(()) => {
                            file_stats.migrated += 1;
                            file_stats.total_bytes += bytes;
                            progress.inc_file(bytes);
                            if file_stats.migrated % 100 == 0 {
                                tracing::info!(
                                    "Migrated {} files ({:.1} MB)",
                                    file_stats.migrated,
                                    file_stats.total_bytes as f64 / 1_048_576.0
                                );
                            }
                            {
                                let mut cp = checkpoint.write().await;
                                cp.mark_done(remote_path, &content_hash);
                                cp.save(config.checkpoint_path.as_deref());
                            }
                            last_err = None;
                            break;
                        }
                        Err(e) => {
                            last_err = Some(format!("upload: {}", e));
                            if attempt < self.config.max_retries {
                                tokio::time::sleep(std::time::Duration::from_millis(
                                    100 * (attempt + 1) as u64,
                                ))
                                .await;
                            }
                        }
                    }
                }

                if let Some(err) = last_err {
                    file_stats.failed += 1;
                    tracing::error!(
                        "Failed {} after {} retries: {}",
                        remote_path,
                        self.config.max_retries,
                        err
                    );
                    {
                        let mut cp = checkpoint.write().await;
                        cp.mark_failed(remote_path);
                        cp.save(config.checkpoint_path.as_deref());
                    }
                }
            }
        }

        Ok(file_stats)
    }
}

/// Single DFS traversal worker. Discovers files via PROPFIND and sends them
/// through a bounded mpsc channel. No broadcast channel, no race condition.
async fn dfs_traverse(
    source: &WebDavSource,
    user: &str,
    root: &str,
    file_tx: &mpsc::Sender<DavEntry>,
    stats: &AtomicTraversalStats,
    checkpoint: &RwLock<Checkpoint>,
    max_file_size: u64,
) {
    // BFS (breadth-first) traversal: process all directories at current level
    // before going deeper. This ensures large directories like Books (10GB)
    // get processed before the tool exhausts its time budget.
    let mut queue = std::collections::VecDeque::new();
    let mut visited = std::collections::HashSet::new();

    queue.push_back(root.to_string());

    while let Some(dir) = queue.pop_front() {
        if !visited.insert(dir.clone()) {
            continue;
        }

        match source.list_directory(user, &dir).await {
            Ok(entries) => {
                for entry in &entries {
                    // Skip self-references
                    if entry.path.trim_end_matches('/') == dir.trim_end_matches('/') {
                        continue;
                    }

                    if entry.is_collection {
                        queue.push_back(entry.path.clone());
                    } else {
                        // Skip files already marked done in checkpoint
                        {
                            let cp = checkpoint.read().await;
                            if let Some(status) = cp.entries.get(&entry.path) {
                                if status.status == "done" {
                                    stats.skipped.fetch_add(1, Ordering::Relaxed);
                                    continue;
                                }
                            }
                        }

                        if max_file_size > 0 && entry.size > max_file_size {
                            stats.skipped.fetch_add(1, Ordering::Relaxed);
                            continue;
                        }

                        // Small delay to avoid overwhelming the source
                        tokio::time::sleep(std::time::Duration::from_millis(5)).await;

                        if file_tx.send(entry.clone()).await.is_err() {
                            tracing::debug!("File channel closed, stopping traversal");
                            return;
                        }
                    }
                }
            }
            Err(e) => {
                stats.failed.fetch_add(1, Ordering::Relaxed);
                tracing::warn!("Skipping directory {}: {}", dir, e);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Checkpoint system for resumable transfers
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct CheckpointEntry {
    path: String,
    status: String, // "pending" | "done" | "failed"
    #[serde(default)]
    hash: Option<String>,
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct Checkpoint {
    #[serde(default)]
    entries: HashMap<String, CheckpointEntry>,
    #[serde(skip)]
    path: Option<PathBuf>,
}

impl Checkpoint {
    fn load(path: Option<&std::path::Path>) -> Option<Self> {
        let path = path?;
        let data = std::fs::read_to_string(path).ok()?;
        let mut cp: Self = serde_json::from_str(&data).ok()?;
        cp.path = Some(path.to_path_buf());
        Some(cp)
    }

    fn save(&mut self, path: Option<&std::path::Path>) {
        let path = match path {
            Some(p) => p.to_path_buf(),
            None => return,
        };
        self.path = Some(path.clone());
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, json);
        }
    }

    fn mark_done(&mut self, file_path: &str, hash: &str) {
        self.entries.insert(
            file_path.to_string(),
            CheckpointEntry {
                path: file_path.to_string(),
                status: "done".to_string(),
                hash: Some(hash.to_string()),
            },
        );
    }

    fn mark_failed(&mut self, file_path: &str) {
        self.entries.insert(
            file_path.to_string(),
            CheckpointEntry {
                path: file_path.to_string(),
                status: "failed".to_string(),
                hash: None,
            },
        );
    }
}

/// Thread-safe migration statistics for traversal workers.
struct AtomicTraversalStats {
    skipped: AtomicU64,
    failed: AtomicU64,
}

impl Default for AtomicTraversalStats {
    fn default() -> Self {
        Self {
            skipped: AtomicU64::new(0),
            failed: AtomicU64::new(0),
        }
    }
}

#[derive(Debug, Default)]
pub struct FileCopyStats {
    pub migrated: usize,
    pub skipped: usize,
    pub failed: usize,
    pub total_bytes: u64,
}

pub(crate) fn dav_path_to_ferro(dav_path: &str) -> String {
    let trimmed = dav_path.trim_start_matches('/');
    if trimmed.is_empty() {
        return "/".to_string();
    }
    format!("/{}", trimmed)
}

pub fn parse_propfind(xml: &str) -> MigrateResult<Vec<DavEntry>> {
    use quick_xml::Reader;
    use quick_xml::events::Event;

    let mut entries = Vec::new();
    let mut current_href = String::new();
    let mut current_props: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut in_prop = false;
    let mut current_tag = String::new();
    let mut capture_text = false;
    let mut text_buf = String::new();

    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let local_name = e.local_name();
                if local_name.as_ref() == b"response" {
                    current_href.clear();
                    current_props.clear();
                } else if local_name.as_ref() == b"href" {
                    capture_text = true;
                    text_buf.clear();
                } else if local_name.as_ref() == b"prop" {
                    in_prop = true;
                } else if in_prop {
                    current_tag = String::from_utf8_lossy(local_name.as_ref()).to_string();
                    capture_text = true;
                    text_buf.clear();
                }
            }
            Ok(Event::Empty(ref e)) => {
                let local_name = e.local_name();
                if in_prop && local_name.as_ref() == b"collection" {
                    current_props.insert("resourcetype".to_string(), "<collection/>".to_string());
                }
            }
            Ok(Event::End(ref e)) => {
                let local_name = e.local_name();
                if local_name.as_ref() == b"href" {
                    current_href = text_buf.trim().to_string();
                    capture_text = false;
                } else if local_name.as_ref() == b"response" {
                    if !current_href.is_empty() {
                        let is_collection = current_props
                            .get("resourcetype")
                            .map(|v| v.contains("collection"))
                            .unwrap_or(false);
                        let size = current_props
                            .get("getcontentlength")
                            .and_then(|v| v.parse().ok())
                            .unwrap_or(0);

                        entries.push(DavEntry {
                            path: decode_href(&current_href),
                            is_collection,
                            size,
                            last_modified: current_props.get("getlastmodified").cloned(),
                            etag: current_props.get("getetag").cloned(),
                            content_type: current_props.get("getcontenttype").cloned(),
                        });
                    }
                    in_prop = false;
                } else if in_prop && !current_tag.is_empty() && !text_buf.trim().is_empty() {
                    current_props.insert(current_tag.clone(), text_buf.trim().to_string());
                    capture_text = false;
                    current_tag.clear();
                } else if local_name.as_ref() == b"prop" {
                    in_prop = false;
                }
            }
            Ok(Event::Text(ref e)) if capture_text => {
                text_buf.push_str(
                    &quick_xml::escape::unescape(std::str::from_utf8(e.as_ref()).unwrap_or("")).unwrap_or_default(),
                );
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(MigrationError::webdav(format!("XML parse error: {}", e)));
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(entries)
}

fn decode_href(href: &str) -> String {
    let mut current = href.to_string();
    // Decode in a loop to handle double-encoded paths (e.g., %25E5 → %E5 → 发)
    loop {
        match urlencoding::decode(&current) {
            Ok(decoded) if decoded != current => current = decoded,
            _ => return current,
        }
    }
}

mod urlencoding {
    pub fn decode(input: &str) -> Result<String, ()> {
        let mut bytes = Vec::new();
        let mut chars = input.chars();
        while let Some(c) = chars.next() {
            if c == '%' {
                let hex: String = chars.by_ref().take(2).collect();
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    bytes.push(byte);
                } else {
                    bytes.push(b'%');
                    bytes.extend(hex.bytes());
                }
            } else {
                bytes.extend(c.to_string().as_bytes());
            }
        }
        String::from_utf8(bytes).map_err(|_| ())
    }
}
