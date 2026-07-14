use crate::error::DistributedError;
use dashmap::DashMap;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct ReplicaLocation {
    pub region: String,
    pub node_id: String,
    pub endpoint: String,
    pub latency_ms: u64,
    pub is_primary: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConsistencyLevel {
    One,
    Quorum,
    All,
}

#[derive(Debug, Clone)]
pub struct ReplicationPolicy {
    pub replication_factor: usize,
    pub write_quorum: usize,
    pub ack_timeout: Duration,
    pub consistency_level: ConsistencyLevel,
}

impl Default for ReplicationPolicy {
    fn default() -> Self {
        let factor = 3;
        Self {
            replication_factor: factor,
            write_quorum: (factor / 2) + 1,
            ack_timeout: Duration::from_secs(5),
            consistency_level: ConsistencyLevel::Quorum,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ReplicationEntry {
    pub key: String,
    pub value: Vec<u8>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub checksum: [u8; 32],
    pub sequence: u64,
}

pub trait ReplicationLog: Send + Sync {
    fn append(&self, entry: ReplicationEntry) -> Result<(), DistributedError>;
    fn get(&self, key: &str) -> Result<Option<ReplicationEntry>, DistributedError>;
    fn list_since(&self, timestamp: chrono::DateTime<chrono::Utc>) -> Result<Vec<ReplicationEntry>, DistributedError>;
}

pub struct InMemoryReplicationLog {
    entries: DashMap<String, ReplicationEntry>,
    sequences: DashMap<u64, String>,
    next_sequence: std::sync::atomic::AtomicU64,
}

impl InMemoryReplicationLog {
    pub fn new() -> Self {
        Self {
            entries: DashMap::new(),
            sequences: DashMap::new(),
            next_sequence: std::sync::atomic::AtomicU64::new(1),
        }
    }
}

impl Default for InMemoryReplicationLog {
    fn default() -> Self {
        Self::new()
    }
}

impl ReplicationLog for InMemoryReplicationLog {
    fn append(&self, entry: ReplicationEntry) -> Result<(), DistributedError> {
        let seq = self.next_sequence.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let mut entry = entry;
        entry.sequence = seq;
        self.sequences.insert(seq, entry.key.clone());
        self.entries.insert(entry.key.clone(), entry);
        Ok(())
    }

    fn get(&self, key: &str) -> Result<Option<ReplicationEntry>, DistributedError> {
        Ok(self.entries.get(key).map(|e| e.value().clone()))
    }

    fn list_since(&self, timestamp: chrono::DateTime<chrono::Utc>) -> Result<Vec<ReplicationEntry>, DistributedError> {
        let mut results: Vec<ReplicationEntry> = self
            .entries
            .iter()
            .filter(|e| e.timestamp > timestamp)
            .map(|e| e.value().clone())
            .collect();
        results.sort_by_key(|e| e.sequence);
        Ok(results)
    }
}

pub struct ReplicationCoordinator {
    replicas: DashMap<String, ReplicaLocation>,
    policy: ReplicationPolicy,
}

impl ReplicationCoordinator {
    pub fn new(policy: ReplicationPolicy) -> Self {
        Self {
            replicas: DashMap::new(),
            policy,
        }
    }

    pub fn add_replica(&self, replica: ReplicaLocation) {
        self.replicas.insert(replica.node_id.clone(), replica);
    }

    pub fn remove_replica(&self, node_id: &str) {
        self.replicas.remove(node_id);
    }

    pub fn get_primary(&self) -> Option<ReplicaLocation> {
        self.replicas
            .iter()
            .find(|r| r.value().is_primary)
            .map(|r| r.value().clone())
    }

    pub fn select_quorum(&self) -> Vec<ReplicaLocation> {
        let mut all: Vec<ReplicaLocation> = self.replicas.iter().map(|r| r.value().clone()).collect();
        all.sort_by_key(|r| r.latency_ms);
        all.into_iter().take(self.policy.write_quorum).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use sha2::{Digest, Sha256};

    fn sha256(data: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hasher.finalize().into()
    }

    #[test]
    fn test_coordinator_add_remove() {
        let coordinator = ReplicationCoordinator::new(ReplicationPolicy::default());

        let replica = ReplicaLocation {
            region: "us-east-1".into(),
            node_id: "node-1".into(),
            endpoint: "http://node-1:8080".into(),
            latency_ms: 10,
            is_primary: true,
        };
        coordinator.add_replica(replica);

        let primary = coordinator.get_primary();
        assert!(primary.is_some());
        assert_eq!(primary.unwrap().node_id, "node-1");

        coordinator.remove_replica("node-1");
        assert!(coordinator.get_primary().is_none());
    }

    #[test]
    fn test_quorum_selection() {
        let policy = ReplicationPolicy {
            replication_factor: 3,
            write_quorum: 2,
            ack_timeout: Duration::from_secs(5),
            consistency_level: ConsistencyLevel::Quorum,
        };
        let coordinator = ReplicationCoordinator::new(policy);

        for (i, (lat, region)) in [(50, "eu"), (10, "us"), (30, "ap")].into_iter().enumerate() {
            coordinator.add_replica(ReplicaLocation {
                region: region.into(),
                node_id: format!("node-{}", i),
                endpoint: format!("http://node-{}:8080", i),
                latency_ms: lat,
                is_primary: i == 1,
            });
        }

        let quorum = coordinator.select_quorum();
        assert_eq!(quorum.len(), 2);
        assert_eq!(quorum[0].latency_ms, 10);
        assert_eq!(quorum[1].latency_ms, 30);
    }

    #[test]
    fn test_log_append_and_read() {
        let log = InMemoryReplicationLog::new();

        let entry = ReplicationEntry {
            key: "key-1".into(),
            value: b"value-1".to_vec(),
            timestamp: Utc::now(),
            checksum: sha256(b"value-1"),
            sequence: 0,
        };
        log.append(entry.clone()).unwrap();

        let retrieved = log.get("key-1").unwrap().unwrap();
        assert_eq!(retrieved.key, "key-1");
        assert_eq!(retrieved.value, b"value-1");
    }

    #[test]
    fn test_list_since_filtering() {
        let log = InMemoryReplicationLog::new();

        let now = Utc::now();
        let earlier = now - chrono::Duration::seconds(10);
        let later = now + chrono::Duration::seconds(10);

        let entry1 = ReplicationEntry {
            key: "key-early".into(),
            value: b"old".to_vec(),
            timestamp: earlier,
            checksum: sha256(b"old"),
            sequence: 0,
        };
        let entry2 = ReplicationEntry {
            key: "key-late".into(),
            value: b"new".to_vec(),
            timestamp: later,
            checksum: sha256(b"new"),
            sequence: 0,
        };

        log.append(entry1).unwrap();
        log.append(entry2).unwrap();

        let results = log.list_since(now).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].key, "key-late");
    }
}
