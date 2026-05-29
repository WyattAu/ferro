//! Sync engine: orchestrates bi-directional file synchronization.
//!
//! The engine:
//! 1. Scans the local filesystem for changes
//! 2. Fetches the remote state via WebDAV PROPFIND
//! 3. Compares with the stored sync state to detect changes
//! 4. Plans sync operations (upload, download, delete, conflict)
//! 5. Executes the plan
//! 6. Updates the sync state

use anyhow::Result;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::conflict::{ConflictStrategy, resolve_conflict_keep_both};
use super::remote::scan_remote;
use super::scanner::scan_local;
use super::state::SyncState;
use super::types::*;

/// Configuration for the sync engine.
#[derive(Debug, Clone)]
pub struct SyncConfig {
    /// Local directory to sync.
    pub local_path: PathBuf,
    /// Remote path prefix on the server.
    pub remote_path: String,
    /// Server URL (e.g., "http://localhost:8080").
    pub server_url: String,
    /// Username for authentication.
    pub username: String,
    /// Password for authentication.
    pub password: String,
    /// Conflict resolution strategy.
    pub conflict_strategy: ConflictStrategy,
    /// Maximum file size to sync (bytes). Default: 10 GB.
    pub max_file_size: u64,
    /// Whether to use block-level delta sync.
    pub use_block_sync: bool,
    /// Target block size for block-level sync.
    pub block_size: u64,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            local_path: PathBuf::from("."),
            remote_path: "/".to_string(),
            server_url: "http://localhost:8080".to_string(),
            username: String::new(),
            password: String::new(),
            conflict_strategy: ConflictStrategy::KeepBoth,
            max_file_size: 10_000_000_000, // 10 GB
            use_block_sync: true,
            block_size: 65536, // 64KB
        }
    }
}

/// The sync engine. Runs sync cycles on demand or periodically.
pub struct SyncEngine {
    config: SyncConfig,
    state: Arc<RwLock<SyncState>>,
    client: reqwest::Client,
}

impl SyncEngine {
    /// Create a new sync engine.
    pub fn new(config: SyncConfig) -> Result<Self> {
        let state = SyncState::load(&config.local_path)?;
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()?;

        Ok(Self {
            config,
            state: Arc::new(RwLock::new(state)),
            client,
        })
    }

    /// Get a reference to the sync state (for UI queries).
    pub fn state(&self) -> Arc<RwLock<SyncState>> {
        self.state.clone()
    }

    /// Run a full sync cycle.
    pub async fn sync(&self) -> Result<SyncSummary> {
        let start = std::time::Instant::now();
        let mut summary = SyncSummary::default();

        tracing::info!(
            local = %self.config.local_path.display(),
            remote = %self.config.remote_path,
            "starting sync cycle"
        );

        // Step 1: Scan local filesystem
        let local_result = tokio::task::spawn_blocking({
            let local_path = self.config.local_path.clone();
            let max_size = self.config.max_file_size;
            move || scan_local(&local_path, max_size)
        })
        .await??;

        tracing::info!(
            files = local_result.file_count,
            dirs = local_result.dir_count,
            bytes = local_result.total_bytes,
            "local scan complete"
        );

        // Step 2: Scan remote via WebDAV
        let remote_result = scan_remote(
            &self.client,
            &self.config.server_url,
            &self.config.username,
            &self.config.password,
            &self.config.remote_path,
        )
        .await?;

        tracing::info!(
            files = remote_result.file_count,
            dirs = remote_result.dir_count,
            "remote scan complete"
        );

        // Step 3: Update sync state with local and remote scan results
        {
            let mut state = self.state.write().await;
            let local_paths: HashSet<String> = local_result.files.keys().cloned().collect();
            let remote_paths: HashSet<String> = remote_result.files.keys().cloned().collect();

            // Update local entries
            for (path, (hash, size, mtime_ms, is_dir)) in &local_result.files {
                state.update_local(path, hash.clone(), *size, *mtime_ms, *is_dir);
            }

            // Update remote entries
            for (path, (hash, size, mtime_ms, is_dir)) in &remote_result.files {
                state.update_remote(path, hash.clone(), *size, *mtime_ms, *is_dir);
            }

            // Mark locally deleted files
            let local_deletions: Vec<String> = state
                .iter()
                .filter(|entry| {
                    !entry.local_deleted
                        && !entry.is_dir
                        && !entry.local_hash.is_empty()
                        && !local_paths.contains(&entry.relative_path)
                })
                .map(|e| e.relative_path.clone())
                .collect();
            for path in &local_deletions {
                state.mark_local_deleted(path);
            }

            // Mark remotely deleted files
            let remote_deletions: Vec<String> = state
                .iter()
                .filter(|entry| {
                    !entry.remote_deleted
                        && !entry.is_dir
                        && !entry.remote_hash.is_empty()
                        && !remote_paths.contains(&entry.relative_path)
                })
                .map(|e| e.relative_path.clone())
                .collect();
            for path in &remote_deletions {
                state.mark_remote_deleted(path);
            }
        }

        // Step 4: Build sync plan
        let plan = self.build_plan().await;

        tracing::info!(
            upload = plan.to_upload.len(),
            download = plan.to_download.len(),
            remote_del = plan.remote_deletions.len(),
            local_del = plan.local_deletions.len(),
            conflicts = plan.conflicts.len(),
            "sync plan built"
        );

        // Step 5: Execute plan
        summary.conflicts = plan.conflicts.len() as u64;

        // Handle conflicts first
        for path in &plan.conflicts {
            if let Err(e) = self.resolve_conflict(path).await {
                tracing::error!(path = %path, error = %e, "conflict resolution failed");
                summary.errors += 1;
            }
        }

        // Upload files
        for path in &plan.to_upload {
            match self.upload_file(path).await {
                Ok(bytes) => {
                    summary.uploaded += 1;
                    summary.bytes_transferred += bytes;
                    self.state.write().await.mark_synced(path);
                }
                Err(e) => {
                    tracing::error!(path = %path, error = %e, "upload failed");
                    summary.errors += 1;
                }
            }
        }

        // Download files
        for path in &plan.to_download {
            match self.download_file(path).await {
                Ok(bytes) => {
                    summary.downloaded += 1;
                    summary.bytes_transferred += bytes;
                    self.state.write().await.mark_synced(path);
                }
                Err(e) => {
                    tracing::error!(path = %path, error = %e, "download failed");
                    summary.errors += 1;
                }
            }
        }

        // Remote deletions
        for path in &plan.remote_deletions {
            match self.delete_remote(path).await {
                Ok(()) => {
                    summary.remote_deletions += 1;
                    self.state.write().await.remove(path);
                }
                Err(e) => {
                    tracing::error!(path = %path, error = %e, "remote delete failed");
                    summary.errors += 1;
                }
            }
        }

        // Local deletions
        for path in &plan.local_deletions {
            match self.delete_local(path).await {
                Ok(()) => {
                    summary.local_deletions += 1;
                    self.state.write().await.remove(path);
                }
                Err(e) => {
                    tracing::error!(path = %path, error = %e, "local delete failed");
                    summary.errors += 1;
                }
            }
        }

        // Step 6: Persist state
        self.state.read().await.save()?;

        summary.duration_ms = start.elapsed().as_millis() as u64;
        summary.timestamp_ms = chrono::Utc::now().timestamp_millis();

        tracing::info!(
            uploaded = summary.uploaded,
            downloaded = summary.downloaded,
            conflicts = summary.conflicts,
            errors = summary.errors,
            bytes = summary.bytes_transferred,
            duration_ms = summary.duration_ms,
            "sync cycle complete"
        );

        Ok(summary)
    }

    /// Build a sync plan from the current state.
    async fn build_plan(&self) -> SyncPlan {
        let state = self.state.read().await;
        let mut plan = SyncPlan::default();

        for entry in state.iter() {
            match entry.status() {
                FileSyncStatus::Synced => {}
                FileSyncStatus::LocalModified | FileSyncStatus::LocalOnly => {
                    plan.to_upload.push(entry.relative_path.clone());
                }
                FileSyncStatus::RemoteModified | FileSyncStatus::RemoteOnly => {
                    plan.to_download.push(entry.relative_path.clone());
                }
                FileSyncStatus::LocalDeleted => {
                    plan.remote_deletions.push(entry.relative_path.clone());
                }
                FileSyncStatus::RemoteDeleted => {
                    plan.local_deletions.push(entry.relative_path.clone());
                }
                FileSyncStatus::Conflict => {
                    plan.conflicts.push(entry.relative_path.clone());
                }
                FileSyncStatus::Syncing => {}
                FileSyncStatus::Error(_) => {}
            }
        }

        plan
    }

    /// Resolve a conflict according to the configured strategy.
    async fn resolve_conflict(&self, relative_path: &str) -> Result<()> {
        match self.config.conflict_strategy {
            ConflictStrategy::LocalWins => {
                // Upload local version (will overwrite remote)
                self.upload_file(relative_path).await?;
                self.state.write().await.mark_synced(relative_path);
            }
            ConflictStrategy::RemoteWins => {
                // Download remote version (will overwrite local)
                self.download_file(relative_path).await?;
                self.state.write().await.mark_synced(relative_path);
            }
            ConflictStrategy::KeepBoth => {
                // Rename local file, then download remote
                let local = self.config.local_path.join(relative_path);
                let _conflict_path =
                    tokio::task::spawn_blocking(move || resolve_conflict_keep_both(&local))
                        .await??;
                self.download_file(relative_path).await?;
                self.state.write().await.mark_synced(relative_path);
            }
            ConflictStrategy::Skip => {
                tracing::warn!(path = %relative_path, "skipping conflicted file");
            }
        }
        Ok(())
    }

    /// Upload a file to the server via WebDAV PUT.
    async fn upload_file(&self, relative_path: &str) -> Result<u64> {
        let local_path = self.config.local_path.join(relative_path);
        let data = tokio::task::spawn_blocking(move || std::fs::read(&local_path)).await??;

        let remote_url = self.remote_url(relative_path);
        let size = data.len() as u64;

        let response = self
            .client
            .put(&remote_url)
            .basic_auth(&self.config.username, Some(&self.config.password))
            .header("Content-Type", "application/octet-stream")
            .body(data)
            .send()
            .await?;

        if !response.status().is_success() && response.status().as_u16() != 204 {
            anyhow::bail!("upload failed: {} for {}", response.status(), relative_path);
        }

        Ok(size)
    }

    /// Download a file from the server via WebDAV GET.
    async fn download_file(&self, relative_path: &str) -> Result<u64> {
        let remote_url = self.remote_url(relative_path);
        let local_path = self.config.local_path.join(relative_path);

        let response = self
            .client
            .get(&remote_url)
            .basic_auth(&self.config.username, Some(&self.config.password))
            .send()
            .await?;

        if !response.status().is_success() {
            anyhow::bail!(
                "download failed: {} for {}",
                response.status(),
                relative_path
            );
        }

        let data = response.bytes().await?;
        let size = data.len() as u64;

        // Ensure parent directory exists
        let parent = local_path.parent().map(|p| p.to_path_buf());
        if let Some(parent_dir) = parent {
            tokio::task::spawn_blocking(move || std::fs::create_dir_all(&parent_dir)).await??;
        }

        // Write file atomically
        let tmp_path = local_path.with_extension("download.tmp");
        tokio::task::spawn_blocking(move || {
            std::fs::write(&tmp_path, &data)?;
            std::fs::rename(&tmp_path, &local_path)?;
            Ok::<(), std::io::Error>(())
        })
        .await??;

        Ok(size)
    }

    /// Delete a file on the remote server.
    async fn delete_remote(&self, relative_path: &str) -> Result<()> {
        let remote_url = self.remote_url(relative_path);
        let response = self
            .client
            .delete(&remote_url)
            .basic_auth(&self.config.username, Some(&self.config.password))
            .send()
            .await?;

        if !response.status().is_success() && response.status().as_u16() != 204 {
            anyhow::bail!(
                "remote delete failed: {} for {}",
                response.status(),
                relative_path
            );
        }
        Ok(())
    }

    /// Delete a local file.
    async fn delete_local(&self, relative_path: &str) -> Result<()> {
        let local_path = self.config.local_path.join(relative_path);
        tokio::task::spawn_blocking(move || {
            if local_path.exists() {
                std::fs::remove_file(&local_path)?;
            }
            Ok::<(), std::io::Error>(())
        })
        .await??;
        Ok(())
    }

    /// Construct the full remote URL for a relative path.
    fn remote_url(&self, relative_path: &str) -> String {
        format!(
            "{}/{}{}",
            self.config.server_url.trim_end_matches('/'),
            self.config
                .remote_path
                .trim_start_matches('/')
                .trim_end_matches('/'),
            if relative_path.starts_with('/') {
                relative_path.to_string()
            } else {
                format!("/{}", relative_path)
            }
        )
    }

    /// Run a background sync loop at the configured interval.
    pub async fn run_periodically(
        self: &Arc<Self>,
        interval_secs: u64,
        mut shutdown: tokio::sync::watch::Receiver<bool>,
    ) {
        if interval_secs == 0 {
            tracing::info!("automatic sync disabled (interval=0)");
            return;
        }

        let mut interval = tokio::time::interval(std::time::Duration::from_secs(interval_secs));

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Err(e) = self.sync().await {
                        tracing::error!(error = %e, "periodic sync failed");
                    }
                }
                _ = shutdown.changed() => {
                    tracing::info!("sync engine shutting down");
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_config_default() {
        let config = SyncConfig::default();
        assert_eq!(config.max_file_size, 10_000_000_000);
        assert_eq!(config.conflict_strategy, ConflictStrategy::KeepBoth);
        assert!(config.use_block_sync);
    }

    #[test]
    fn test_remote_url_construction() {
        let engine = SyncEngine::new(SyncConfig {
            server_url: "http://localhost:8080".to_string(),
            remote_path: "/docs".to_string(),
            ..Default::default()
        })
        .unwrap();

        assert_eq!(
            engine.remote_url("file.txt"),
            "http://localhost:8080/docs/file.txt"
        );
        assert_eq!(
            engine.remote_url("sub/file.txt"),
            "http://localhost:8080/docs/sub/file.txt"
        );
    }

    #[test]
    fn test_build_plan_empty_state() {
        let dir = std::env::temp_dir().join("ferro-sync-plan-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let engine = SyncEngine::new(SyncConfig {
            local_path: dir.clone(),
            remote_path: "/".to_string(),
            server_url: "http://localhost:8080".to_string(),
            username: "admin".to_string(),
            password: "test".to_string(),
            ..Default::default()
        })
        .unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();
        let plan = rt.block_on(engine.build_plan());
        assert!(plan.to_upload.is_empty());
        assert!(plan.to_download.is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
