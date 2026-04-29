use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use dashmap::DashMap;

use super::clock::VectorClock;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum OpType {
    Create,
    Update,
    Delete,
    Rename,
    Share,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncOp {
    pub id: String,
    pub site_id: String,
    pub clock: VectorClock,
    pub r#type: OpType,
    pub path: String,
    pub new_path: Option<String>,
    pub size: u64,
    pub mime_type: Option<String>,
    pub owner: String,
    pub checksum: String,
    pub timestamp: String,
}

pub struct SyncStore {
    pub ops: Arc<DashMap<String, SyncOp>>,
    max_ops: usize,
    global_clock: Arc<AtomicU64>,
}

impl SyncStore {
    pub fn new() -> Self {
        Self {
            ops: Arc::new(DashMap::new()),
            max_ops: 100_000,
            global_clock: Arc::new(AtomicU64::new(1)),
        }
    }

    pub fn with_max_ops(max_ops: usize) -> Self {
        Self {
            ops: Arc::new(DashMap::new()),
            max_ops,
            global_clock: Arc::new(AtomicU64::new(1)),
        }
    }

    pub fn record_op(&self, op: SyncOp) {
        let id = op.id.clone();
        if self.ops.len() >= self.max_ops {
            let to_remove = self.ops.len() - self.max_ops + 1;
            let keys: Vec<String> = self.ops.iter()
                .take(to_remove)
                .map(|e| e.key().clone())
                .collect();
            for key in keys {
                self.ops.remove(&key);
            }
        }
        self.ops.insert(id, op);
    }

    pub fn get_ops_since(&self, clock: u64) -> Vec<SyncOp> {
        self.ops.iter()
            .filter(|e| e.value().clock.counter > clock)
            .map(|e| e.value().clone())
            .collect()
    }

    pub fn current_clock(&self) -> u64 {
        self.global_clock.load(Ordering::SeqCst)
    }

    pub fn next_op_id(&self) -> (String, u64) {
        let clock = self.global_clock.fetch_add(1, Ordering::SeqCst);
        (format!("op-{}", clock), clock)
    }
}

impl Default for SyncStore {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_store_record_and_delta() {
        let store = SyncStore::new();

        for i in 0..3 {
            let (id, clock) = store.next_op_id();
            store.record_op(SyncOp {
                id,
                site_id: "local".to_string(),
                clock: VectorClock::new("local").with_counter(clock),
                r#type: OpType::Create,
                path: format!("/file{}.txt", i),
                new_path: None,
                size: 100,
                mime_type: Some("text/plain".to_string()),
                owner: "admin".to_string(),
                checksum: "abc123".to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
            });
        }

        assert_eq!(store.ops.len(), 3);

        let delta = store.get_ops_since(1);
        assert_eq!(delta.len(), 2);

        let delta = store.get_ops_since(3);
        assert_eq!(delta.len(), 0);

        let delta = store.get_ops_since(0);
        assert_eq!(delta.len(), 3);
    }

    #[test]
    fn test_sync_store_bounded() {
        let store = SyncStore::with_max_ops(5);
        for i in 0..10 {
            let (id, clock) = store.next_op_id();
            store.record_op(SyncOp {
                id,
                site_id: "local".to_string(),
                clock: VectorClock::new("local").with_counter(clock),
                r#type: OpType::Create,
                path: format!("/f{}", i),
                new_path: None,
                size: 0,
                mime_type: None,
                owner: "admin".to_string(),
                checksum: "".to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
            });
        }
        assert!(store.ops.len() <= 5);
    }
}
