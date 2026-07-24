use crate::error::{MigrationError, Result as MigrateResult};
use crate::ferro_target::FerroTarget;
use crate::nextcloud::NextcloudClient;
use crate::ocis::OcisClient;

#[derive(Debug, Clone)]
pub struct DavEntry {
    pub path: String,
    pub is_collection: bool,
    pub size: u64,
    pub last_modified: Option<String>,
    pub etag: Option<String>,
    pub content_type: Option<String>,
}

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

    pub async fn list_directory_recursive(&self, user: &str, path: &str) -> MigrateResult<Vec<DavEntry>> {
        match self {
            WebDavSource::Nextcloud(nc) => nc.list_directory_recursive(user, path).await,
            WebDavSource::Ocis(oc) => oc.list_directory_recursive(user, path).await,
        }
    }

    pub async fn download_file(&self, user: &str, path: &str) -> MigrateResult<Vec<u8>> {
        match self {
            WebDavSource::Nextcloud(nc) => nc.download_file(user, path).await,
            WebDavSource::Ocis(oc) => oc.download_file(user, path).await,
        }
    }
}

pub struct WebDavPipeline<'a> {
    source: &'a WebDavSource,
    target: &'a FerroTarget,
    max_file_size: u64,
}

impl<'a> WebDavPipeline<'a> {
    pub fn new(source: &'a WebDavSource, target: &'a FerroTarget, max_file_size: u64, _batch_size: usize) -> Self {
        Self {
            source,
            target,
            max_file_size,
        }
    }

    /// Streaming migration: traverses directories lazily and uploads files
    /// as they are discovered. Bounded memory — never holds the full file list.
    pub async fn copy_all_files(
        &self,
        user: &str,
        progress: &crate::progress::ProgressTracker,
    ) -> MigrateResult<FileCopyStats> {
        let mut stats = FileCopyStats::default();
        let mut stack = vec!["/".to_string()];
        let mut visited = std::collections::HashSet::new();

        while let Some(dir) = stack.pop() {
            if !visited.insert(dir.clone()) {
                continue;
            }

            match self.source.list_directory(user, &dir).await {
                Ok(entries) => {
                    for entry in &entries {
                        // Skip the directory itself
                        if entry.path.trim_end_matches('/') == dir.trim_end_matches('/') {
                            continue;
                        }

                        if entry.is_collection {
                            stack.push(entry.path.clone());
                        } else {
                            if self.max_file_size > 0 && entry.size > self.max_file_size {
                                tracing::info!("Skipping large file ({} bytes): {}", entry.size, entry.path);
                                stats.skipped += 1;
                                progress.inc_file(0);
                                continue;
                            }

                            // Upload immediately — no batching, no full tree in memory
                            let ferro_path = dav_path_to_ferro(&entry.path);

                            // Create parent directory on-the-fly
                            if let Some(parent) = ferro_path.rsplit('/').next() {
                                if !parent.is_empty() {
                                    let parent_path = ferro_path[..ferro_path.len() - parent.len()].trim_end_matches('/');
                                    if !parent_path.is_empty() {
                                        let _ = self.target.create_directory(parent_path).await;
                                    }
                                }
                            }

                            match self.source.download_file(user, &entry.path).await {
                                Ok(content) => {
                                    let bytes = content.len() as u64;
                                    tracing::debug!("Downloaded {} ({} bytes), uploading to {}", entry.path, bytes, ferro_path);
                                    match self.target.put_file(&ferro_path, &content).await {
                                        Ok(()) => {
                                            stats.migrated += 1;
                                            stats.total_bytes += bytes;
                                            progress.inc_file(bytes);
                                            if stats.migrated % 100 == 0 {
                                                tracing::info!("Migrated {} files ({:.1} MB)", stats.migrated, stats.total_bytes as f64 / 1_048_576.0);
                                            }
                                        }
                                        Err(e) => {
                                            tracing::error!("Failed to upload {}: {}", ferro_path, e);
                                            stats.failed += 1;
                                            progress.inc_file(0);
                                        }
                                    }
                                }
                                Err(e) => {
                                    tracing::error!("Failed to download {}: {}", entry.path, e);
                                    stats.failed += 1;
                                    progress.inc_file(0);
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Skipping directory {}: {}", dir, e);
                }
            }
        }

        Ok(stats)
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
        let mut result = String::new();
        let mut chars = input.chars();
        while let Some(c) = chars.next() {
            if c == '%' {
                let hex: String = chars.by_ref().take(2).collect();
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    result.push(byte as char);
                } else {
                    result.push('%');
                    result.push_str(&hex);
                }
            } else {
                result.push(c);
            }
        }
        Ok(result)
    }
}
