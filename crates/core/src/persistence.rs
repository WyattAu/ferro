//! Unified SQLite persistence for CAS, snapshots, audit log, and locks.
//!
//! All persistent state is stored in a single SQLite database file with
//! separate tables. This avoids managing multiple DB connections and
//! simplifies deployment (one `--data-dir` flag).
//!
//! Usage:
//! ```ignore
//! let store = SqlitePersistence::new("sqlite:///data/ferro.db").await?;
//! // Store implements CasStore, SnapshotStore, AuditLogStore traits
//! ```

use async_trait::async_trait;
use bytes::Bytes;
use ferro_common::error::{FerroError, Result};
use ferro_common::metadata::{ContentHash, FileMetadata};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::str::FromStr;
use tracing::{debug, info};

// ── Trait definitions ──────────────────────────────────────────────────

/// Persistent content-addressable storage.
#[async_trait]
pub trait SnapshotStore: Send + Sync {
    async fn create(&self, description: String, entries: Vec<FileMetadata>) -> Result<String>;
    async fn get(&self, id: &str) -> Result<PersistedSnapshot>;
    async fn list(&self) -> Result<Vec<PersistedSnapshotSummary>>;
    async fn delete(&self, id: &str) -> Result<()>;
    async fn entry_count(&self) -> usize;
}

/// Persistent audit log.
#[async_trait]
pub trait AuditLogStore: Send + Sync {
    async fn log(&self, entry: PersistedAuditEntry) -> Result<()>;
    async fn recent(&self, limit: usize) -> Result<Vec<PersistedAuditEntry>>;
    async fn count(&self) -> usize;
}

// ── Data types ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedSnapshot {
    pub id: String,
    pub created_at: String,
    pub description: String,
    pub entries_json: String, // Serialized Vec<FileMetadata>
    pub entry_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedSnapshotSummary {
    pub id: String,
    pub created_at: String,
    pub description: String,
    pub entry_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PersistedAuditEntry {
    pub id: i64,
    pub timestamp: String,
    pub method: String,
    pub path: String,
    pub user: String,
    pub status: u16,
    pub client_ip: Option<String>,
    pub user_agent: Option<String>,
    pub content_length: Option<u64>,
}

// ── Unified SQLite Persistence ──────────────────────────────────────────

pub struct SqlitePersistence {
    pool: SqlitePool,
}

impl SqlitePersistence {
    /// Open a SQLite database and create all tables.
    /// Accepts any SQLite URL, e.g. `sqlite:///data/ferro.db`.
    pub async fn new(database_url: &str) -> Result<Self> {
        // Ensure WAL mode and foreign keys for better concurrent access
        let opts = SqliteConnectOptions::from_str(database_url)
            .map_err(|e| FerroError::Internal(format!("Invalid SQLite URL: {}", e)))?
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .foreign_keys(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(opts)
            .await
            .map_err(|e| FerroError::Internal(format!("Failed to connect to SQLite: {}", e)))?;

        // Run all migrations
        let ddl = [
            r#"
            CREATE TABLE IF NOT EXISTS cas_content (
                hash TEXT PRIMARY KEY,
                content BLOB NOT NULL,
                size INTEGER NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            )"#,
            "CREATE INDEX IF NOT EXISTS idx_cas_created ON cas_content(created_at)",
            r#"
            CREATE TABLE IF NOT EXISTS snapshots (
                id TEXT PRIMARY KEY,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                description TEXT NOT NULL DEFAULT '',
                entries_json TEXT NOT NULL,
                entry_count INTEGER NOT NULL DEFAULT 0
            )"#,
            "CREATE INDEX IF NOT EXISTS idx_snapshots_created ON snapshots(created_at)",
            r#"
            CREATE TABLE IF NOT EXISTS audit_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL DEFAULT (datetime('now')),
                method TEXT NOT NULL,
                path TEXT NOT NULL DEFAULT '',
                user TEXT NOT NULL DEFAULT 'anonymous',
                status INTEGER NOT NULL DEFAULT 0,
                client_ip TEXT,
                user_agent TEXT,
                content_length INTEGER
            )"#,
            "CREATE INDEX IF NOT EXISTS idx_audit_timestamp ON audit_log(timestamp)",
            r#"
            CREATE TABLE IF NOT EXISTS file_metadata (
                path TEXT PRIMARY KEY,
                content_hash TEXT NOT NULL,
                size INTEGER NOT NULL DEFAULT 0,
                mime_type TEXT NOT NULL DEFAULT 'application/octet-stream',
                is_collection INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                modified_at TEXT NOT NULL DEFAULT (datetime('now')),
                owner TEXT NOT NULL DEFAULT 'anonymous',
                etag TEXT NOT NULL DEFAULT ''
            )"#,
        ];

        for stmt in &ddl {
            sqlx::query(stmt)
                .execute(&pool)
                .await
                .map_err(|e| FerroError::Internal(format!("SQLite migration failed: {}", e)))?;
        }

        info!("SQLite persistence initialized: {}", database_url);
        Ok(Self { pool })
    }

    /// Return the inner pool for use by SqliteMetadataStore.
    /// This allows sharing a single connection pool across metadata + other stores.
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

// ── CAS Store implementation ────────────────────────────────────────────

#[async_trait]
impl crate::cas::CasStore for SqlitePersistence {
    async fn put_content(&self, content: Bytes) -> Result<ContentHash> {
        let hash = ContentHash::compute(&content);
        let hash_str = hash.as_str();
        let size = content.len() as i64;

        // INSERT OR IGNORE handles dedup — if hash already exists, skip
        let result = sqlx::query(
            "INSERT OR IGNORE INTO cas_content (hash, content, size) VALUES (?, ?, ?)"
        )
        .bind(hash_str)
        .bind(content.as_ref())
        .bind(size)
        .execute(&self.pool)
        .await
        .map_err(|e| FerroError::Internal(format!("CAS put failed: {}", e)))?;

        if result.rows_affected() > 0 {
            debug!("CAS PUT: stored content {}", &hash_str[..16]);
        } else {
            debug!("CAS DEDUP: content {} already exists", &hash_str[..16]);
        }

        Ok(hash)
    }

    async fn get_content(&self, hash: &ContentHash) -> Result<Bytes> {
        let row: Option<(Vec<u8>,)> = sqlx::query_as(
            "SELECT content FROM cas_content WHERE hash = ?"
        )
        .bind(hash.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| FerroError::Internal(format!("CAS get failed: {}", e)))?;

        row.map(|(bytes,)| Bytes::from(bytes))
            .ok_or_else(|| FerroError::NotFound(format!("content hash {}", hash.as_str())))
    }

    async fn exists(&self, hash: &ContentHash) -> Result<bool> {
        let result: Option<(i64,)> = sqlx::query_as(
            "SELECT COUNT(*) as cnt FROM cas_content WHERE hash = ?"
        )
        .bind(hash.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| FerroError::Internal(format!("CAS exists check failed: {}", e)))?;

        Ok(result.map(|(cnt,)| cnt > 0).unwrap_or(false))
    }

    async fn dedup_check(&self, hash: &ContentHash) -> Result<bool> {
        self.exists(hash).await
    }

    async fn content_count(&self) -> usize {
        let result: Option<(i64,)> = sqlx::query_as(
            "SELECT COUNT(*) as cnt FROM cas_content"
        )
        .fetch_optional(&self.pool)
        .await
        .unwrap_or(None);

        result.map(|(cnt,)| cnt as usize).unwrap_or(0)
    }
}

// ── Snapshot Store implementation ─────────────────────────────────────────

#[async_trait]
impl SnapshotStore for SqlitePersistence {
    async fn create(&self, description: String, entries: Vec<FileMetadata>) -> Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        let entries_json = serde_json::to_string(&entries)
            .map_err(|e| FerroError::Internal(format!("Failed to serialize snapshot entries: {}", e)))?;
        let entry_count = entries.len();

        sqlx::query(
            "INSERT INTO snapshots (id, description, entries_json, entry_count) VALUES (?, ?, ?, ?)"
        )
        .bind(&id)
        .bind(&description)
        .bind(&entries_json)
        .bind(entry_count as i64)
        .execute(&self.pool)
        .await
        .map_err(|e| FerroError::Internal(format!("Snapshot create failed: {}", e)))?;

        debug!("Snapshot created: {} ({} entries)", id, entry_count);
        Ok(id)
    }

    async fn get(&self, id: &str) -> Result<PersistedSnapshot> {
        let row: Option<(String, String, String, String, i64)> = sqlx::query_as(
            "SELECT id, created_at, description, entries_json, entry_count FROM snapshots WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| FerroError::Internal(format!("Snapshot get failed: {}", e)))?;

        row.map(|(id, created_at, description, entries_json, entry_count)| PersistedSnapshot {
            id,
            created_at,
            description,
            entries_json,
            entry_count: entry_count as usize,
        })
        .ok_or_else(|| FerroError::NotFound(format!("snapshot {}", id)))
    }

    async fn list(&self) -> Result<Vec<PersistedSnapshotSummary>> {
        let rows: Vec<(String, String, String, i64)> = sqlx::query_as(
            "SELECT id, created_at, description, entry_count FROM snapshots ORDER BY created_at DESC"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| FerroError::Internal(format!("Snapshot list failed: {}", e)))?;

        Ok(rows
            .into_iter()
            .map(|(id, created_at, description, entry_count)| PersistedSnapshotSummary {
                id,
                created_at,
                description,
                entry_count: entry_count as usize,
            })
            .collect())
    }

    async fn delete(&self, id: &str) -> Result<()> {
        let result = sqlx::query("DELETE FROM snapshots WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| FerroError::Internal(format!("Snapshot delete failed: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(FerroError::NotFound(format!("snapshot {}", id)));
        }
        Ok(())
    }

    async fn entry_count(&self) -> usize {
        let result: Option<(i64,)> = sqlx::query_as(
            "SELECT COUNT(*) as cnt FROM snapshots"
        )
        .fetch_optional(&self.pool)
        .await
        .unwrap_or(None);

        result.map(|(cnt,)| cnt as usize).unwrap_or(0)
    }
}

// ── Audit Log Store implementation ──────────────────────────────────────────

#[async_trait]
impl AuditLogStore for SqlitePersistence {
    async fn log(&self, entry: PersistedAuditEntry) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO audit_log
                (timestamp, method, path, user, status, client_ip, user_agent, content_length)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?)"#
        )
        .bind(&entry.timestamp)
        .bind(&entry.method)
        .bind(&entry.path)
        .bind(&entry.user)
        .bind(entry.status as i64)
        .bind(&entry.client_ip)
        .bind(&entry.user_agent)
        .bind(entry.content_length.map(|c| c as i64))
        .execute(&self.pool)
        .await
        .map_err(|e| FerroError::Internal(format!("Audit log insert failed: {}", e)))?;

        Ok(())
    }

    async fn recent(&self, limit: usize) -> Result<Vec<PersistedAuditEntry>> {
        let limit = limit.min(10000) as i64; // Cap at 10k
        let rows: Vec<PersistedAuditEntry> = sqlx::query_as(
            "SELECT id, timestamp, method, path, user, status, client_ip, user_agent, content_length
             FROM audit_log ORDER BY id DESC LIMIT ?"
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| FerroError::Internal(format!("Audit log recent failed: {}", e)))?;

        Ok(rows)
    }

    async fn count(&self) -> usize {
        let result: Option<(i64,)> = sqlx::query_as(
            "SELECT COUNT(*) as cnt FROM audit_log"
        )
        .fetch_optional(&self.pool)
        .await
        .unwrap_or(None);

        result.map(|(cnt,)| cnt as usize).unwrap_or(0)
    }
}

// ── Metadata Store: reuse existing SqliteMetadataStore ────────────────────
//
// The existing `SqliteMetadataStore` in `sqlx_metadata.rs` already has the
// file_metadata table. Since our unified store creates the same table,
// we provide `from_pool()` to avoid creating a duplicate connection.

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cas::CasStore;

    fn make_db_path() -> (tempfile::TempDir, String) {
        let tmp = tempfile::TempDir::new().unwrap();
        let db_path = tmp.path().join("test.db");
        let url = format!("sqlite:{}", db_path.display());
        (tmp, url)
    }

    async fn make_store() -> (tempfile::TempDir, SqlitePersistence) {
        let (tmp, url) = make_db_path();
        let store = SqlitePersistence::new(&url).await.unwrap();
        (tmp, store)
    }

    #[tokio::test]
    async fn test_cas_persistence() {
        let (_tmp, store) = make_store().await;
        let content = Bytes::from("persistent content");
        let hash = store.put_content(content.clone()).await.unwrap();

        // Content survives (would be lost with InMemoryCasStore)
        assert_eq!(store.content_count().await, 1);
        let retrieved = store.get_content(&hash).await.unwrap();
        assert_eq!(content, retrieved);
    }

    #[tokio::test]
    async fn test_cas_dedup_persistent() {
        let (_tmp, store) = make_store().await;
        let content = Bytes::from("dedup me");
        let hash1 = store.put_content(content.clone()).await.unwrap();
        let hash2 = store.put_content(content.clone()).await.unwrap();
        assert_eq!(hash1, hash2);
        assert_eq!(store.content_count().await, 1); // Still just 1 row
    }

    #[tokio::test]
    async fn test_snapshot_persistence() {
        let (_tmp, store) = make_store().await;
        use ferro_common::metadata::ContentHash;

        let entries = vec![FileMetadata::new(
            "/test.txt".to_string(),
            ContentHash::new("a".repeat(64)),
            42,
            "user1".to_string(),
        )];
        let id = store.create("test snapshot".to_string(), entries).await.unwrap();

        let summary = store.list().await.unwrap();
        assert_eq!(summary.len(), 1);
        assert_eq!(summary[0].entry_count, 1);

        let snap = store.get(&id).await.unwrap();
        assert_eq!(snap.entry_count, 1);
        let restored: Vec<FileMetadata> = serde_json::from_str(&snap.entries_json).unwrap();
        assert_eq!(restored[0].path, "/test.txt");
    }

    #[tokio::test]
    async fn test_snapshot_delete() {
        let (_tmp, store) = make_store().await;
        let id = store.create("to delete".to_string(), vec![]).await.unwrap();
        assert_eq!(store.list().await.unwrap().len(), 1);

        store.delete(&id).await.unwrap();
        assert_eq!(store.list().await.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_audit_persistence() {
        let (_tmp, store) = make_store().await;
        let entry = PersistedAuditEntry {
            id: 0, // auto-increment
            timestamp: "2026-04-21T12:00:00Z".to_string(),
            method: "PUT".to_string(),
            path: "/test.txt".to_string(),
            user: "alice".to_string(),
            status: 201,
            client_ip: Some("10.0.0.1".to_string()),
            user_agent: None,
            content_length: Some(42),
        };
        store.log(entry).await.unwrap();

        let recent = store.recent(100).await.unwrap();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].method, "PUT");
        assert_eq!(recent[0].path, "/test.txt");
        assert_eq!(recent[0].user, "alice");
        assert_eq!(recent[0].content_length, Some(42));
    }

    #[tokio::test]
    async fn test_audit_multiple_entries() {
        let (_tmp, store) = make_store().await;
        for i in 0..5 {
            store.log(PersistedAuditEntry {
                id: 0,
                timestamp: "2026-04-21T12:00:00Z".to_string(),
                method: "GET".to_string(),
                path: format!("/file{}.txt", i),
                user: "bob".to_string(),
                status: 200,
                client_ip: None,
                user_agent: None,
                content_length: None,
            }).await.unwrap();
        }
        assert_eq!(store.count().await, 5);
        assert_eq!(store.recent(2).await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_shared_database_across_traits() {
        // All traits share the same SQLite database
        let (_tmp, store) = make_store().await;

        // CAS
        let hash = store.put_content(Bytes::from("data")).await.unwrap();
        assert!(store.exists(&hash).await.unwrap());

        // Snapshots
        let _snap_id = store.create("desc".to_string(), vec![]).await.unwrap();
        assert_eq!(store.list().await.unwrap().len(), 1);

        // Audit
        store.log(PersistedAuditEntry {
            id: 0, timestamp: "now".to_string(), method: "GET".to_string(),
            path: "/".to_string(), user: "test".to_string(), status: 200,
            client_ip: None, user_agent: None, content_length: None,
        }).await.unwrap();

        assert_eq!(store.content_count().await, 1);
        assert_eq!(store.entry_count().await, 1);
        assert_eq!(store.count().await, 1);
    }
}
