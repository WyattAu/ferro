use crate::error::Result as MigrateResult;
use crate::ferro_target::FerroTarget;
use crate::nextcloud::NextcloudClient;

pub struct WebDavPipeline<'a> {
    source: &'a NextcloudClient,
    target: &'a FerroTarget,
    max_file_size: u64,
    batch_size: usize,
}

impl<'a> WebDavPipeline<'a> {
    pub fn new(
        source: &'a NextcloudClient,
        target: &'a FerroTarget,
        max_file_size: u64,
        batch_size: usize,
    ) -> Self {
        Self {
            source,
            target,
            max_file_size,
            batch_size,
        }
    }

    pub async fn copy_all_files(
        &self,
        user: &str,
        progress: &crate::progress::ProgressTracker,
    ) -> MigrateResult<FileCopyStats> {
        let entries = self.source.list_directory_recursive(user, "/").await?;

        let dirs: Vec<_> = entries.iter().filter(|e| e.is_collection).collect();
        let files: Vec<_> = entries.iter().filter(|e| !e.is_collection).collect();

        progress.set_file_total(files.len() as u64);

        for dir in &dirs {
            let ferro_path = nc_dav_path_to_ferro(&dir.path, user);
            if let Err(e) = self.target.create_directory(&ferro_path).await {
                tracing::warn!("Skipping directory {}: {}", ferro_path, e);
            }
        }

        let mut stats = FileCopyStats::default();
        let mut batch: Vec<&crate::nextcloud::DavEntry> = Vec::new();

        for file in &files {
            if self.max_file_size > 0 && file.size > self.max_file_size {
                tracing::info!("Skipping large file ({} bytes): {}", file.size, file.path);
                stats.skipped += 1;
                progress.inc_file(0);
                continue;
            }

            batch.push(file);

            if batch.len() >= self.batch_size {
                self.process_batch(user, &batch, &mut stats, progress)
                    .await?;
                batch.clear();
            }
        }

        if !batch.is_empty() {
            self.process_batch(user, &batch, &mut stats, progress)
                .await?;
        }

        Ok(stats)
    }

    async fn process_batch(
        &self,
        user: &str,
        batch: &[&crate::nextcloud::DavEntry],
        stats: &mut FileCopyStats,
        progress: &crate::progress::ProgressTracker,
    ) -> MigrateResult<()> {
        for file in batch {
            let ferro_path = nc_dav_path_to_ferro(&file.path, user);

            match self.source.download_file(user, &file.path).await {
                Ok(content) => {
                    let bytes = content.len() as u64;
                    match self.target.put_file(&ferro_path, &content).await {
                        Ok(()) => {
                            stats.migrated += 1;
                            stats.total_bytes += bytes;
                            progress.inc_file(bytes);
                        }
                        Err(e) => {
                            tracing::error!("Failed to upload {}: {}", ferro_path, e);
                            stats.failed += 1;
                            progress.inc_file(0);
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to download {}: {}", file.path, e);
                    stats.failed += 1;
                    progress.inc_file(0);
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct FileCopyStats {
    pub migrated: usize,
    pub skipped: usize,
    pub failed: usize,
    pub total_bytes: u64,
}

fn nc_dav_path_to_fereo(dav_path: &str, user: &str) -> String {
    let prefix = format!("/remote.php/dav/files/{}/", user);
    if let Some(stripped) = dav_path.strip_prefix(&prefix) {
        format!("/{}", stripped)
    } else {
        dav_path.to_string()
    }
}

fn nc_dav_path_to_ferro(dav_path: &str, user: &str) -> String {
    nc_dav_path_to_fereo(dav_path, user)
}
