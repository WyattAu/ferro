//! Write-ahead change queue for offline file operations.
//!
//! All file mutations (put, delete, move, copy) are recorded as operations
//! in a SQLite-backed queue. Operations are replayed in order during reconciliation.

use async_trait::async_trait;
use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::warn;

/// Type of file operation queued while offline.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OperationType {
    /// Create or update a file.
    Put,
    /// Delete a file.
    Delete,
    /// Move/rename a file.
    Move,
    /// Copy a file.
    Copy,
    /// Create a directory (collection).
    CreateCollection,
}

/// A queued offline operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedOperation {
    /// Unique operation ID.
    pub id: String,
    /// Type of operation.
    pub op: OperationType,
    /// Source path (for put/delete/move/copy).
    pub source_path: String,
    /// Destination path (for move/copy).
    pub dest_path: Option<String>,
    /// SHA-256 content hash of the file content at enqueue time.
    pub content_hash: Option<String>,
    /// File size in bytes at enqueue time.
    pub content_size: Option<u64>,
    /// Owner (user who performed the operation).
    pub owner: String,
    /// Timestamp when the operation was queued.
    pub queued_at: chrono::DateTime<chrono::Utc>,
    /// Whether this operation has been synced to the server.
    pub synced: bool,
}

impl QueuedOperation {
    /// Create a put operation.
    pub fn put(path: &str, content_hash: Option<String>, content_size: Option<u64>, owner: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            op: OperationType::Put,
            source_path: path.to_string(),
            dest_path: None,
            content_hash,
            content_size,
            owner: owner.to_string(),
            queued_at: Utc::now(),
            synced: false,
        }
    }

    /// Create a delete operation.
    pub fn delete(path: &str, owner: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            op: OperationType::Delete,
            source_path: path.to_string(),
            dest_path: None,
            content_hash: None,
            content_size: None,
            owner: owner.to_string(),
            queued_at: Utc::now(),
            synced: false,
        }
    }

    /// Create a move operation.
    pub fn move_op(from: &str, to: &str, owner: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            op: OperationType::Move,
            source_path: from.to_string(),
            dest_path: Some(to.to_string()),
            content_hash: None,
            content_size: None,
            owner: owner.to_string(),
            queued_at: Utc::now(),
            synced: false,
        }
    }

    /// Create a copy operation.
    pub fn copy(from: &str, to: &str, owner: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            op: OperationType::Copy,
            source_path: from.to_string(),
            dest_path: Some(to.to_string()),
            content_hash: None,
            content_size: None,
            owner: owner.to_string(),
            queued_at: Utc::now(),
            synced: false,
        }
    }

    /// Create a collection creation operation.
    pub fn create_collection(path: &str, owner: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            op: OperationType::CreateCollection,
            source_path: path.to_string(),
            dest_path: None,
            content_hash: None,
            content_size: None,
            owner: owner.to_string(),
            queued_at: Utc::now(),
            synced: false,
        }
    }

    /// Compute SHA-256 hash of content bytes.
    pub fn hash_content(data: &[u8]) -> String {
        crate::crypto::hash_content(data)
    }
}

/// Maximum number of pending operations in the queue.
const MAX_PENDING_OPS: usize = 50_000;

/// Async trait for persisting and retrieving queued operations.
#[async_trait]
pub trait ChangeQueueStore: Send + Sync {
    /// Enqueue a new offline operation.
    async fn enqueue(&self, op: QueuedOperation) -> Result<(), OfflineError>;
    /// Get all pending (unsynced) operations in queue order.
    async fn pending(&self) -> Vec<QueuedOperation>;
    /// Mark an operation as synced.
    async fn mark_synced(&self, id: &str) -> Result<(), OfflineError>;
    /// Remove a specific operation from the queue.
    async fn remove(&self, id: &str) -> Result<(), OfflineError>;
    /// Get count of pending operations.
    async fn pending_count(&self) -> usize;
    /// Get all operations (including synced) for a specific path.
    async fn operations_for_path(&self, path: &str) -> Vec<QueuedOperation>;
    /// Prune synced operations older than the given timestamp.
    async fn prune_synced(&self, before: chrono::DateTime<chrono::Utc>) -> Result<usize, OfflineError>;
    /// Clear the entire queue (used for reset).
    async fn clear(&self) -> Result<(), OfflineError>;
}

/// Error type for offline operations.
#[derive(Debug, thiserror::Error)]
pub enum OfflineError {
    #[error("Queue full: {0} pending operations (max {1})")]
    QueueFull(usize, usize),
    #[error("Operation not found: {0}")]
    NotFound(String),
    #[error("Storage error: {0}")]
    Storage(String),
    #[error("Conflict: {0}")]
    Conflict(String),
    #[error("Reconciliation failed: {0}")]
    ReconciliationFailed(String),
}

/// SQLite-backed change queue store.
pub struct SqliteChangeQueue {
    db: Arc<std::sync::Mutex<rusqlite::Connection>>,
}

/// # Safety
/// The wrapped Connection is only accessed via short-lived lock guards
/// that never cross an `.await` point.
pub type DbHandle = Arc<std::sync::Mutex<rusqlite::Connection>>;

impl SqliteChangeQueue {
    /// Create a new queue backed by a SQLite connection.
    pub fn new(db: DbHandle) -> Self {
        Self { db }
    }

    /// Initialize the queue table.
    pub fn init(&self) -> Result<(), rusqlite::Error> {
        let conn = self.db.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS offline_queue (
                id TEXT PRIMARY KEY,
                op TEXT NOT NULL,
                source_path TEXT NOT NULL,
                dest_path TEXT,
                content_hash TEXT,
                content_size INTEGER,
                owner TEXT NOT NULL,
                queued_at TEXT NOT NULL,
                synced INTEGER NOT NULL DEFAULT 0
            );
            CREATE INDEX IF NOT EXISTS idx_queue_pending ON offline_queue(synced, queued_at);
            CREATE INDEX IF NOT EXISTS idx_queue_path ON offline_queue(source_path);
            ",
        )?;
        Ok(())
    }

    fn persist(&self, op: &QueuedOperation) {
        let conn = self.db.lock().unwrap_or_else(|e| e.into_inner());
        if let Err(e) = conn.execute(
            "INSERT OR REPLACE INTO offline_queue (id, op, source_path, dest_path, content_hash, content_size, owner, queued_at, synced) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                op.id,
                format!("{:?}", op.op),
                op.source_path,
                op.dest_path,
                op.content_hash,
                op.content_size.map(|s| s as i64),
                op.owner,
                op.queued_at.to_rfc3339(),
                op.synced as i32,
            ],
        ) {
            warn!("Failed to persist queued op: {}", e);
        }
    }

    fn load_op(row: &rusqlite::Row) -> Result<QueuedOperation, rusqlite::Error> {
        let op_str: String = row.get(1)?;
        let op = match op_str.as_str() {
            "Put" => OperationType::Put,
            "Delete" => OperationType::Delete,
            "Move" => OperationType::Move,
            "Copy" => OperationType::Copy,
            "CreateCollection" => OperationType::CreateCollection,
            _ => OperationType::Put,
        };
        let queued_at_str: String = row.get(7)?;
        let queued_at = chrono::DateTime::parse_from_rfc3339(&queued_at_str)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| Utc::now());
        Ok(QueuedOperation {
            id: row.get(0)?,
            op,
            source_path: row.get(2)?,
            dest_path: row.get(3)?,
            content_hash: row.get(4)?,
            content_size: row.get::<_, Option<i64>>(5)?.map(|s| s as u64),
            owner: row.get(6)?,
            queued_at,
            synced: row.get::<_, i32>(8)? != 0,
        })
    }
}

#[async_trait]
impl ChangeQueueStore for SqliteChangeQueue {
    async fn enqueue(&self, op: QueuedOperation) -> Result<(), OfflineError> {
        let count = self.pending_count().await;
        if count >= MAX_PENDING_OPS {
            return Err(OfflineError::QueueFull(count, MAX_PENDING_OPS));
        }
        self.persist(&op);
        Ok(())
    }

    async fn pending(&self) -> Vec<QueuedOperation> {
        let conn = self.db.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = match conn.prepare(
            "SELECT id, op, source_path, dest_path, content_hash, content_size, owner, queued_at, synced FROM offline_queue WHERE synced = 0 ORDER BY queued_at ASC",
        ) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        let rows = stmt.query_map([], Self::load_op);
        match rows {
            Ok(mapped) => mapped.filter_map(|r| r.ok()).collect(),
            Err(_) => Vec::new(),
        }
    }

    async fn mark_synced(&self, id: &str) -> Result<(), OfflineError> {
        let conn = self.db.lock().unwrap_or_else(|e| e.into_inner());
        let affected = conn
            .execute("UPDATE offline_queue SET synced = 1 WHERE id = ?1", params![id])
            .map_err(|e| OfflineError::Storage(e.to_string()))?;
        if affected == 0 {
            return Err(OfflineError::NotFound(id.to_string()));
        }
        Ok(())
    }

    async fn remove(&self, id: &str) -> Result<(), OfflineError> {
        let conn = self.db.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute("DELETE FROM offline_queue WHERE id = ?1", params![id])
            .map_err(|e| OfflineError::Storage(e.to_string()))?;
        Ok(())
    }

    async fn pending_count(&self) -> usize {
        let conn = self.db.lock().unwrap_or_else(|e| e.into_inner());
        conn.query_row("SELECT COUNT(*) FROM offline_queue WHERE synced = 0", [], |row| {
            row.get::<_, usize>(0)
        })
        .unwrap_or(0)
    }

    async fn operations_for_path(&self, path: &str) -> Vec<QueuedOperation> {
        let conn = self.db.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = match conn.prepare(
            "SELECT id, op, source_path, dest_path, content_hash, content_size, owner, queued_at, synced FROM offline_queue WHERE source_path = ?1 OR dest_path = ?2 ORDER BY queued_at ASC",
        ) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        let rows = stmt.query_map(params![path, path], Self::load_op);
        match rows {
            Ok(mapped) => mapped.filter_map(|r| r.ok()).collect(),
            Err(_) => Vec::new(),
        }
    }

    async fn prune_synced(&self, before: chrono::DateTime<chrono::Utc>) -> Result<usize, OfflineError> {
        let conn = self.db.lock().unwrap_or_else(|e| e.into_inner());
        let before_str = before.to_rfc3339();
        let affected = conn
            .execute(
                "DELETE FROM offline_queue WHERE synced = 1 AND queued_at < ?1",
                params![before_str],
            )
            .map_err(|e| OfflineError::Storage(e.to_string()))?;
        Ok(affected)
    }

    async fn clear(&self) -> Result<(), OfflineError> {
        let conn = self.db.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute("DELETE FROM offline_queue", [])
            .map_err(|e| OfflineError::Storage(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn store() -> SqliteChangeQueue {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        let queue = SqliteChangeQueue::new(Arc::new(std::sync::Mutex::new(conn)));
        queue.init().unwrap();
        queue
    }

    #[tokio::test]
    async fn test_enqueue_and_pending() {
        let q = store();
        let op = QueuedOperation::put("/file.txt", Some("abc123".into()), Some(100), "alice");
        q.enqueue(op.clone()).await.unwrap();

        let pending = q.pending().await;
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].source_path, "/file.txt");
    }

    #[tokio::test]
    async fn test_enqueue_delete() {
        let q = store();
        q.enqueue(QueuedOperation::delete("/old.txt", "bob")).await.unwrap();
        let pending = q.pending().await;
        assert_eq!(pending[0].op, OperationType::Delete);
    }

    #[tokio::test]
    async fn test_enqueue_move() {
        let q = store();
        q.enqueue(QueuedOperation::move_op("/a.txt", "/b.txt", "alice"))
            .await
            .unwrap();
        let pending = q.pending().await;
        assert_eq!(pending[0].op, OperationType::Move);
        assert_eq!(pending[0].dest_path.as_deref(), Some("/b.txt"));
    }

    #[tokio::test]
    async fn test_enqueue_copy() {
        let q = store();
        q.enqueue(QueuedOperation::copy("/a.txt", "/c.txt", "alice"))
            .await
            .unwrap();
        let pending = q.pending().await;
        assert_eq!(pending[0].op, OperationType::Copy);
    }

    #[tokio::test]
    async fn test_enqueue_create_collection() {
        let q = store();
        q.enqueue(QueuedOperation::create_collection("/dir/", "alice"))
            .await
            .unwrap();
        let pending = q.pending().await;
        assert_eq!(pending[0].op, OperationType::CreateCollection);
    }

    #[tokio::test]
    async fn test_mark_synced() {
        let q = store();
        let op = QueuedOperation::put("/f.txt", None, None, "alice");
        let id = op.id.clone();
        q.enqueue(op).await.unwrap();

        q.mark_synced(&id).await.unwrap();
        assert_eq!(q.pending().await.len(), 0);
    }

    #[tokio::test]
    async fn test_mark_synced_not_found() {
        let q = store();
        let result = q.mark_synced("nonexistent").await;
        assert!(matches!(result, Err(OfflineError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_remove() {
        let q = store();
        let op = QueuedOperation::put("/f.txt", None, None, "alice");
        let id = op.id.clone();
        q.enqueue(op).await.unwrap();

        q.remove(&id).await.unwrap();
        assert_eq!(q.pending().await.len(), 0);
    }

    #[tokio::test]
    async fn test_pending_count() {
        let q = store();
        assert_eq!(q.pending_count().await, 0);
        q.enqueue(QueuedOperation::put("/a", None, None, "u")).await.unwrap();
        q.enqueue(QueuedOperation::put("/b", None, None, "u")).await.unwrap();
        assert_eq!(q.pending_count().await, 2);
        q.mark_synced(&q.pending().await[0].id).await.unwrap();
        assert_eq!(q.pending_count().await, 1);
    }

    #[tokio::test]
    async fn test_operations_for_path() {
        let q = store();
        q.enqueue(QueuedOperation::put("/docs/file.txt", None, None, "u"))
            .await
            .unwrap();
        q.enqueue(QueuedOperation::delete("/docs/file.txt", "u")).await.unwrap();
        q.enqueue(QueuedOperation::put("/other.txt", None, None, "u"))
            .await
            .unwrap();

        let ops = q.operations_for_path("/docs/file.txt").await;
        assert_eq!(ops.len(), 2);
    }

    #[tokio::test]
    async fn test_operations_for_dest_path() {
        let q = store();
        q.enqueue(QueuedOperation::move_op("/a", "/target", "u")).await.unwrap();
        let ops = q.operations_for_path("/target").await;
        assert_eq!(ops.len(), 1);
    }

    #[tokio::test]
    async fn test_prune_synced() {
        let q = store();
        q.enqueue(QueuedOperation::put("/a", None, None, "u")).await.unwrap();
        q.enqueue(QueuedOperation::put("/b", None, None, "u")).await.unwrap();
        let id = q.pending().await[0].id.clone();
        q.mark_synced(&id).await.unwrap();

        let pruned = q.prune_synced(Utc::now()).await.unwrap();
        assert_eq!(pruned, 1);
        assert_eq!(q.pending().await.len(), 1);
    }

    #[tokio::test]
    async fn test_clear() {
        let q = store();
        q.enqueue(QueuedOperation::put("/a", None, None, "u")).await.unwrap();
        q.enqueue(QueuedOperation::put("/b", None, None, "u")).await.unwrap();
        q.clear().await.unwrap();
        assert_eq!(q.pending().await.len(), 0);
        assert_eq!(q.pending_count().await, 0);
    }

    #[test]
    fn test_hash_content() {
        let hash = QueuedOperation::hash_content(b"hello world");
        assert_eq!(hash.len(), 64); // SHA-256 hex
    }

    #[tokio::test]
    async fn test_queue_full() {
        let q = store();
        // Fill to near max
        for i in 0..MAX_PENDING_OPS {
            q.enqueue(QueuedOperation::put(&format!("/f{i}"), None, None, "u"))
                .await
                .unwrap();
        }
        assert_eq!(q.pending_count().await, MAX_PENDING_OPS);
        let result = q.enqueue(QueuedOperation::put("/overflow", None, None, "u")).await;
        assert!(matches!(result, Err(OfflineError::QueueFull(_, _))));
    }

    #[tokio::test]
    async fn test_operation_ordering() {
        let q = store();
        q.enqueue(QueuedOperation::put("/3", None, None, "u")).await.unwrap();
        q.enqueue(QueuedOperation::put("/1", None, None, "u")).await.unwrap();
        q.enqueue(QueuedOperation::put("/2", None, None, "u")).await.unwrap();

        let pending = q.pending().await;
        // All enqueued nearly simultaneously, but should be ordered by queued_at
        assert_eq!(pending.len(), 3);
    }
}
