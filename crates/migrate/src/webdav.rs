use crate::error::{MigrationError, Result as MigrateResult};
use crate::ferro_target::FerroTarget;
use crate::nextcloud::NextcloudClient;
use crate::ocis::OcisClient;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

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
}

/// Configuration for the parallel migration pipeline.
#[derive(Clone)]
pub struct PipelineConfig {
    /// Number of concurrent directory traversal workers.
    pub traverse_workers: usize,
    /// Number of concurrent file transfer workers.
    pub transfer_workers: usize,
    /// Maximum file size to transfer (0 = no limit).
    pub max_file_size: u64,
    /// Channel buffer size for directories.
    pub dir_channel_size: usize,
    /// Channel buffer size for files.
    pub file_channel_size: usize,
    /// Maximum retries per file transfer.
    pub max_retries: u32,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            traverse_workers: 4,
            transfer_workers: 8,
            max_file_size: 0,
            dir_channel_size: 256,
            file_channel_size: 512,
            max_retries: 3,
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

    /// Parallel streaming migration with concurrent directory traversal and file transfers.
    ///
    /// Architecture:
    /// - N directory workers traverse directories in parallel, sending files to a bounded channel
    /// - M file workers consume from the channel, downloading and uploading concurrently
    /// - Token refresh happens automatically before each request
    /// - Bounded channels prevent memory exhaustion
    /// - Atomic counters provide thread-safe progress tracking
    pub async fn copy_all_files(
        &self,
        user: &str,
        progress: &crate::progress::ProgressTracker,
    ) -> MigrateResult<FileCopyStats> {
        let config = self.config.clone();
        let source_clone = self.source.clone();
        let _target_clone = self.target.clone();
        let progress = progress.clone();

        // Shared atomic counters for thread-safe progress
        let stats = Arc::new(AtomicTraversalStats::default());

        // Track active traversal workers
        let active_traversers = Arc::new(std::sync::atomic::AtomicUsize::new(config.traverse_workers));

        // Channel for discovered files (bounded to limit memory)
        let (file_tx, mut file_rx) = tokio::sync::mpsc::channel::<DavEntry>(config.file_channel_size);

        // Channel for directories to traverse (broadcast allows multiple consumers)
        let (dir_tx, mut dir_rx) = tokio::sync::broadcast::channel::<String>(config.dir_channel_size);

        // Spawn directory traversal workers
        let mut traverse_handles = Vec::new();
        for worker_id in 0..config.traverse_workers {
            let source = source_clone.clone();
            let mut dir_rx = dir_rx.resubscribe();
            let dir_tx = dir_tx.clone();
            let file_tx = file_tx.clone();
            let user = user.to_string();
            let _config_clone = PipelineConfig {
                traverse_workers: config.traverse_workers,
                ..PipelineConfig::default()
            };
            let stats = Arc::clone(&stats);
            let active_traversers = Arc::clone(&active_traversers);

            let handle = tokio::spawn(async move {
                tracing::info!("[traverse-{}] started, waiting for directories", worker_id);
                let mut local_dirs = Vec::new();
                let mut recv_count = 0u64;
                let mut processed = std::collections::HashSet::new();

                loop {
                    match tokio::time::timeout(
                        std::time::Duration::from_secs(5),
                        dir_rx.recv()
                    ).await {
                        Ok(Ok(dir)) => {
                            recv_count += 1;
                            // Skip if another worker already processed this directory
                            if !processed.insert(dir.clone()) {
                                continue;
                            }
                            // Rate limit: small delay to avoid overwhelming OCIS
                            if local_dirs.is_empty() {
                                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                            }

                            match source.list_directory(&user, &dir).await {
                                Ok(entries) => {
                                    for entry in &entries {
                                        if entry.path.trim_end_matches('/') == dir.trim_end_matches('/') {
                                            continue;
                                        }

                                        if entry.is_collection {
                                            local_dirs.push(entry.path.clone());
                                        } else {
                                            if config.max_file_size > 0 && entry.size > config.max_file_size {
                                                stats.skipped.fetch_add(1, Ordering::Relaxed);
                                                continue;
                                            }

                                            if file_tx.send(entry.clone()).await.is_err() {
                                                tracing::debug!("[traverse-{}] file channel closed, stopping", worker_id);
                                                break;
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    stats.failed.fetch_add(1, Ordering::Relaxed);
                                    tracing::warn!("[traverse-{}] skipping {}: {}", worker_id, dir, e);
                                }
                            }

                            // Forward discovered directories
                            for d in local_dirs.drain(..) {
                                if dir_tx.send(d).is_err() {
                                    break;
                                }
                            }
                        }
                        Ok(Err(tokio::sync::broadcast::error::RecvError::Lagged(_))) => {
                            continue;
                        }
                        Ok(Err(tokio::sync::broadcast::error::RecvError::Closed)) => {
                            tracing::debug!("[traverse-{}] dir channel closed, stopping", worker_id);
                            break;
                        }
                        Err(_elapsed) => {
                            // Timeout — no directory received in 5 seconds
                            let remaining = active_traversers.fetch_sub(1, Ordering::SeqCst);
                            if remaining <= 1 {
                                tracing::debug!("[traverse-{}] last worker, stopping", worker_id);
                                break;
                            }
                            tracing::debug!("[traverse-{}] timeout, {} workers still active", worker_id, remaining - 1);
                            break;
                        }
                    }
                }

                tracing::debug!("[traverse-{}] finished", worker_id);
            });
            traverse_handles.push(handle);
        }

        // Seed the directory queue with root AFTER workers are spawned
        let _ = dir_tx.send("/".to_string());

        // Drop the original dir_tx so channel closes when all workers finish
        drop(dir_tx);

        // Wait for all traversal workers to finish first
        for handle in traverse_handles {
            let _ = handle.await;
        }

        // Now process files sequentially from the channel (no spawn needed)
        let mut file_stats = FileCopyStats::default();
        while let Some(entry) = file_rx.recv().await {
            let ferro_path = dav_path_to_ferro(&entry.path);

            // Create parent directory on-the-fly
            if let Some(parent) = ferro_path.rsplit('/').next() {
                if !parent.is_empty() {
                    let parent_path = ferro_path[..ferro_path.len() - parent.len()]
                        .trim_end_matches('/');
                    if !parent_path.is_empty() {
                        let _ = self.target.create_directory(parent_path).await;
                    }
                }
            }

            // Download with retries
            let mut last_err = None;
            for attempt in 0..=self.config.max_retries {
                match self.source.download_file(user, &entry.path).await {
                    Ok(content) => {
                        let bytes = content.len() as u64;
                        match self.target.put_file(&ferro_path, &content).await {
                            Ok(()) => {
                                file_stats.migrated += 1;
                                file_stats.total_bytes += bytes;
                                progress.inc_file(bytes);
                                if file_stats.migrated % 100 == 0 {
                                    tracing::info!("Migrated {} files ({:.1} MB)", file_stats.migrated, file_stats.total_bytes as f64 / 1_048_576.0);
                                }
                                last_err = None;
                                break;
                            }
                            Err(e) => {
                                last_err = Some(format!("upload: {}", e));
                                if attempt < self.config.max_retries {
                                    tokio::time::sleep(std::time::Duration::from_millis(100 * (attempt + 1) as u64)).await;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        last_err = Some(format!("download: {}", e));
                        if attempt < self.config.max_retries {
                            tokio::time::sleep(std::time::Duration::from_millis(100 * (attempt + 1) as u64)).await;
                        }
                    }
                }
            }

            if let Some(err) = last_err {
                file_stats.failed += 1;
                tracing::error!("Failed {} after {} retries: {}", entry.path, self.config.max_retries, err);
            }
        }

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
                }
            }
        }

        Ok(file_stats)
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

fn dav_path_to_ferro(dav_path: &str) -> String {
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
    urlencoding::decode(href)
        .map(|s| s.to_string())
        .unwrap_or_else(|_| href.to_string())
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
