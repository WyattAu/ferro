use dashmap::DashMap;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tracing::warn;

use super::clock::VectorClock;
use crate::db::DbHandle;

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
    pub(crate) ops: Arc<DashMap<String, SyncOp>>,
    max_ops: usize,
    global_clock: Arc<AtomicU64>,
    db: Option<DbHandle>,
}

impl SyncStore {
    pub fn new() -> Self {
        Self {
            ops: Arc::new(DashMap::new()),
            max_ops: 100_000,
            global_clock: Arc::new(AtomicU64::new(1)),
            db: None,
        }
    }

    pub fn with_max_ops(max_ops: usize) -> Self {
        Self {
            ops: Arc::new(DashMap::new()),
            max_ops,
            global_clock: Arc::new(AtomicU64::new(1)),
            db: None,
        }
    }

    pub fn with_db(mut self, db: DbHandle) -> Self {
        self.db = Some(db);
        self
    }

    pub fn record_op(&self, op: SyncOp) {
        let id = op.id.clone();
        if self.ops.len() >= self.max_ops {
            let to_remove = self.ops.len() - self.max_ops + 1;
            let keys: Vec<String> = self
                .ops
                .iter()
                .take(to_remove)
                .map(|e| e.key().clone())
                .collect();
            for key in keys {
                self.ops.remove(&key);
            }
        }
        self.ops.insert(id.clone(), op.clone());
        if let Some(ref db) = self.db {
            let conn = db.lock().unwrap();
            if let Err(e) = conn.execute(
                "INSERT OR REPLACE INTO sync_ops (op_id, site_id, clock_counter, op_type, path, new_path, size, mime_type, owner, checksum, timestamp) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    op.id,
                    op.site_id,
                    op.clock.counter as i64,
                    format!("{:?}", op.r#type),
                    op.path,
                    op.new_path,
                    op.size as i64,
                    op.mime_type,
                    op.owner,
                    op.checksum,
                    op.timestamp,
                ],
            ) {
                warn!("Failed to persist sync op to SQLite: {}", e);
            }
        }
    }

    pub fn get_ops_since(&self, clock: u64) -> Vec<SyncOp> {
        self.ops
            .iter()
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

    pub fn load_all_from_db(&self, conn: &rusqlite::Connection) -> std::result::Result<(), rusqlite::Error> {
        let mut stmt = conn.prepare(
            "SELECT op_id, site_id, clock_counter, op_type, path, new_path, size, mime_type, owner, checksum, timestamp FROM sync_ops ORDER BY clock_counter ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            let site_id: String = row.get(1)?;
            let clock_counter: i64 = row.get(2)?;
            let op_type_str: String = row.get(3)?;
            let op_type = match op_type_str.as_str() {
                "Create" => OpType::Create,
                "Update" => OpType::Update,
                "Delete" => OpType::Delete,
                "Rename" => OpType::Rename,
                "Share" => OpType::Share,
                _ => OpType::Create,
            };
            Ok(SyncOp {
                id: row.get(0)?,
                site_id: row.get(1)?,
                clock: VectorClock::new(&site_id).with_counter(clock_counter as u64),
                r#type: op_type,
                path: row.get(4)?,
                new_path: row.get(5)?,
                size: row.get::<_, i64>(6)? as u64,
                mime_type: row.get(7)?,
                owner: row.get(8)?,
                checksum: row.get(9)?,
                timestamp: row.get(10)?,
            })
        })?;
        let mut max_clock = 0u64;
        for row in rows {
            let op: SyncOp = row?;
            if op.clock.counter > max_clock {
                max_clock = op.clock.counter;
            }
            self.ops.insert(op.id.clone(), op);
        }
        if max_clock > 0 {
            self.global_clock.store(max_clock + 1, Ordering::SeqCst);
        }
        Ok(())
    }
}

impl Default for SyncStore {
    fn default() -> Self {
        Self::new()
    }
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
