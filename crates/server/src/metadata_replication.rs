use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::AppState;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MetadataOp {
    Create,
    Update,
    Delete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataChange {
    pub id: String,
    pub path: String,
    pub operation: MetadataOp,
    pub timestamp: DateTime<Utc>,
    pub checksum: Option<String>,
    pub size: Option<u64>,
    pub owner: String,
    pub site_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMetadata {
    pub node_id: String,
    pub endpoint: String,
    pub last_sync: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct MetadataReplicationConfig {
    pub node_id: String,
    pub peer_endpoints: Vec<String>,
    pub sync_interval_secs: u64,
    pub http_timeout_secs: u64,
}

impl Default for MetadataReplicationConfig {
    fn default() -> Self {
        Self {
            node_id: uuid::Uuid::new_v4().to_string(),
            peer_endpoints: Vec::new(),
            sync_interval_secs: 30,
            http_timeout_secs: 10,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataSnapshot {
    pub node_id: String,
    pub entries: Vec<MetadataChange>,
    pub generated_at: DateTime<Utc>,
}

pub trait MetadataChangeLog: Send + Sync {
    fn record(&self, change: MetadataChange);
    fn changes_since(&self, since: DateTime<Utc>, site_id: Option<&str>) -> Vec<MetadataChange>;
    fn all_changes(&self) -> Vec<MetadataChange>;
    fn latest_for_path(&self, path: &str) -> Option<MetadataChange>;
}

pub struct InMemoryChangeLog {
    changes: DashMap<String, MetadataChange>,
    max_entries: usize,
}

impl InMemoryChangeLog {
    pub fn new() -> Self {
        Self {
            changes: DashMap::new(),
            max_entries: 100_000,
        }
    }

    pub fn with_max_entries(max_entries: usize) -> Self {
        Self {
            changes: DashMap::new(),
            max_entries,
        }
    }
}

impl Default for InMemoryChangeLog {
    fn default() -> Self {
        Self::new()
    }
}

impl MetadataChangeLog for InMemoryChangeLog {
    fn record(&self, change: MetadataChange) {
        let id = change.id.clone();
        self.changes.insert(id, change);
        if self.changes.len() > self.max_entries {
            let to_remove = self.changes.len() - self.max_entries;
            let keys: Vec<String> = self
                .changes
                .iter()
                .take(to_remove)
                .map(|e| e.key().clone())
                .collect();
            for key in keys {
                self.changes.remove(&key);
            }
        }
    }

    fn changes_since(&self, since: DateTime<Utc>, site_id: Option<&str>) -> Vec<MetadataChange> {
        self.changes
            .iter()
            .filter(|entry| {
                entry.value().timestamp > since
                    && site_id
                        .map(|sid| entry.value().site_id != sid)
                        .unwrap_or(true)
            })
            .map(|entry| entry.value().clone())
            .collect()
    }

    fn all_changes(&self) -> Vec<MetadataChange> {
        self.changes
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    fn latest_for_path(&self, path: &str) -> Option<MetadataChange> {
        self.changes
            .iter()
            .filter(|entry| entry.value().path == path)
            .max_by_key(|entry| entry.value().timestamp)
            .map(|entry| entry.value().clone())
    }
}

#[async_trait]
pub trait MetadataTransport: Send + Sync {
    async fn send_changes(&self, peer: &str, changes: &[MetadataChange]) -> Result<(), String>;
    async fn request_changes(
        &self,
        peer: &str,
        since: DateTime<Utc>,
    ) -> Result<Vec<MetadataChange>, String>;
}

pub struct HttpMetadataTransport {
    client: reqwest::Client,
}

impl HttpMetadataTransport {
    pub fn new(timeout_secs: u64) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(timeout_secs))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self { client }
    }
}

#[async_trait]
impl MetadataTransport for HttpMetadataTransport {
    async fn send_changes(&self, peer: &str, changes: &[MetadataChange]) -> Result<(), String> {
        let url = format!("{}/api/v1/sync/metadata/changes", peer);
        self.client
            .post(&url)
            .json(changes)
            .send()
            .await
            .map_err(|e| format!("Failed to send changes to {}: {}", peer, e))?
            .error_for_status()
            .map_err(|e| format!("Peer {} rejected changes: {}", peer, e))?;
        Ok(())
    }

    async fn request_changes(
        &self,
        peer: &str,
        since: DateTime<Utc>,
    ) -> Result<Vec<MetadataChange>, String> {
        let url = format!(
            "{}/api/v1/sync/metadata/changes?since={}",
            peer,
            since.to_rfc3339()
        );
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Failed to request changes from {}: {}", peer, e))?;

        response
            .json::<Vec<MetadataChange>>()
            .await
            .map_err(|e| format!("Failed to parse changes from {}: {}", peer, e))
    }
}

pub struct MetadataReplicator {
    config: MetadataReplicationConfig,
    change_log: Arc<dyn MetadataChangeLog>,
    transport: Arc<dyn MetadataTransport>,
    last_sync: Arc<RwLock<DateTime<Utc>>>,
    peer_states: DashMap<String, NodeMetadata>,
}

impl MetadataReplicator {
    pub fn new(
        config: MetadataReplicationConfig,
        change_log: Arc<dyn MetadataChangeLog>,
        transport: Arc<dyn MetadataTransport>,
    ) -> Self {
        let now = Utc::now();
        Self {
            config,
            change_log,
            transport,
            last_sync: Arc::new(RwLock::new(now)),
            peer_states: DashMap::new(),
        }
    }

    pub async fn record_local_change(&self, change: MetadataChange) {
        self.change_log.record(change.clone());

        let peers = self.config.peer_endpoints.clone();
        for peer in &peers {
            match self
                .transport
                .send_changes(peer, std::slice::from_ref(&change))
                .await
            {
                Ok(()) => {
                    info!("Replicated change {} to {}", change.id, peer);
                }
                Err(e) => {
                    warn!(
                        "Failed to replicate change {} to {}: {}",
                        change.id, peer, e
                    );
                }
            }
        }
    }

    pub async fn sync_with_peers(&self) {
        let peers = self.config.peer_endpoints.clone();
        let since = {
            let guard = self.last_sync.read().await;
            *guard
        };

        for peer in &peers {
            match self.transport.request_changes(peer, since).await {
                Ok(remote_changes) => {
                    let applied = self.apply_remote_changes(&remote_changes).await;
                    info!(
                        "Synced with {}: {} changes applied out of {} received",
                        peer,
                        applied,
                        remote_changes.len()
                    );
                }
                Err(e) => {
                    warn!("Failed to sync with {}: {}", peer, e);
                }
            }
        }

        let mut last_sync = self.last_sync.write().await;
        *last_sync = Utc::now();
    }

    async fn apply_remote_changes(&self, changes: &[MetadataChange]) -> usize {
        let mut applied = 0;
        for change in changes {
            if let Some(local) = self.change_log.latest_for_path(&change.path)
                && local.timestamp >= change.timestamp
            {
                continue;
            }

            self.change_log.record(change.clone());
            applied += 1;

            info!(
                "Applied remote change: {} {:?} for {}",
                change.id, change.operation, change.path
            );
        }
        applied
    }

    pub async fn consistency_check(&self, _state: &AppState) -> Vec<ConsistencyIssue> {
        let mut issues = Vec::new();
        let peers = self.config.peer_endpoints.clone();

        for peer in &peers {
            match self
                .transport
                .request_changes(peer, Utc::now() - chrono::Duration::hours(1))
                .await
            {
                Ok(remote_changes) => {
                    for change in &remote_changes {
                        let local = self.change_log.latest_for_path(&change.path);
                        match &local {
                            Some(local_change) => {
                                if local_change.timestamp != change.timestamp {
                                    issues.push(ConsistencyIssue {
                                        path: change.path.clone(),
                                        local_timestamp: local_change.timestamp,
                                        remote_timestamp: change.timestamp,
                                        remote_node: peer.clone(),
                                        resolution: ConflictResolution::LatestTimestampWins,
                                    });
                                }
                            }
                            None => {
                                if change.operation != MetadataOp::Delete {
                                    issues.push(ConsistencyIssue {
                                        path: change.path.clone(),
                                        local_timestamp: Utc::now(),
                                        remote_timestamp: change.timestamp,
                                        remote_node: peer.clone(),
                                        resolution: ConflictResolution::MissingLocally,
                                    });
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("Consistency check failed for {}: {}", peer, e);
                }
            }
        }

        issues
    }

    pub fn get_peer_states(&self) -> Vec<NodeMetadata> {
        self.peer_states
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct ConsistencyIssue {
    pub path: String,
    pub local_timestamp: DateTime<Utc>,
    pub remote_timestamp: DateTime<Utc>,
    pub remote_node: String,
    pub resolution: ConflictResolution,
}

#[derive(Debug, Clone)]
pub enum ConflictResolution {
    LatestTimestampWins,
    MissingLocally,
    MissingRemotely,
}

pub async fn start_metadata_replication(state: Arc<AppState>, config: MetadataReplicationConfig) {
    let change_log = Arc::new(InMemoryChangeLog::new());
    let transport = Arc::new(HttpMetadataTransport::new(config.http_timeout_secs));
    let replicator = Arc::new(MetadataReplicator::new(
        config.clone(),
        change_log,
        transport,
    ));

    let interval_secs = config.sync_interval_secs;
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(interval_secs));
        loop {
            interval.tick().await;
            replicator.sync_with_peers().await;

            let issues = replicator.consistency_check(&state).await;
            if !issues.is_empty() {
                info!("Consistency check found {} issues", issues.len());
                for issue in &issues {
                    info!(
                        "  {} - local: {}, remote: {} ({})",
                        issue.path,
                        issue.local_timestamp,
                        issue.remote_timestamp,
                        issue.remote_node
                    );
                }
            }
        }
    });

    info!(
        "Metadata replication started (node: {}, peers: {})",
        config.node_id,
        config.peer_endpoints.len()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_change_log_record_and_retrieve() {
        let log = InMemoryChangeLog::new();
        let change = MetadataChange {
            id: "1".to_string(),
            path: "/test.txt".to_string(),
            operation: MetadataOp::Create,
            timestamp: Utc::now(),
            checksum: None,
            size: Some(100),
            owner: "alice".to_string(),
            site_id: "local".to_string(),
        };

        log.record(change.clone());
        let all = log.all_changes();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].path, "/test.txt");
    }

    #[test]
    fn test_change_log_latest_for_path() {
        let log = InMemoryChangeLog::new();

        let change1 = MetadataChange {
            id: "1".to_string(),
            path: "/test.txt".to_string(),
            operation: MetadataOp::Create,
            timestamp: Utc::now() - chrono::Duration::hours(1),
            checksum: None,
            size: Some(100),
            owner: "alice".to_string(),
            site_id: "local".to_string(),
        };

        let change2 = MetadataChange {
            id: "2".to_string(),
            path: "/test.txt".to_string(),
            operation: MetadataOp::Update,
            timestamp: Utc::now(),
            checksum: None,
            size: Some(200),
            owner: "alice".to_string(),
            site_id: "local".to_string(),
        };

        log.record(change1);
        log.record(change2);

        let latest = log.latest_for_path("/test.txt").unwrap();
        assert_eq!(latest.id, "2");
        assert_eq!(latest.size, Some(200));
    }

    #[test]
    fn test_changes_since_filters_correctly() {
        let log = InMemoryChangeLog::new();
        let cutoff = Utc::now() - chrono::Duration::hours(1);

        let old = MetadataChange {
            id: "old".to_string(),
            path: "/old.txt".to_string(),
            operation: MetadataOp::Create,
            timestamp: cutoff - chrono::Duration::minutes(30),
            checksum: None,
            size: Some(100),
            owner: "alice".to_string(),
            site_id: "local".to_string(),
        };

        let recent = MetadataChange {
            id: "recent".to_string(),
            path: "/recent.txt".to_string(),
            operation: MetadataOp::Create,
            timestamp: cutoff + chrono::Duration::minutes(30),
            checksum: None,
            size: Some(200),
            owner: "alice".to_string(),
            site_id: "local".to_string(),
        };

        log.record(old);
        log.record(recent);

        let since_cutoff = log.changes_since(cutoff, None);
        assert_eq!(since_cutoff.len(), 1);
        assert_eq!(since_cutoff[0].id, "recent");
    }
}
