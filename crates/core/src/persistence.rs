//! Unified `SQLite` persistence for CAS, snapshots, audit log, and locks.
//!
//! All persistent state is stored in a single `SQLite` database file with
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

/// A persisted filesystem snapshot with its file entries serialized as JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedSnapshot {
    pub id: String,
    pub created_at: String,
    pub description: String,
    pub entries_json: String, // Serialized Vec<FileMetadata>
    pub entry_count: usize,
}

/// Summary of a persisted snapshot without its entries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedSnapshotSummary {
    pub id: String,
    pub created_at: String,
    pub description: String,
    pub entry_count: usize,
}

/// A single audit log entry stored in the database.
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
    /// SHA-256 chain hash for tamper evidence: SHA-256(previous_hash || `this_entry_data`).
    /// Empty string for entries created before chain hashing was enabled.
    pub chain_hash: Option<String>,
}

// ── Unified SQLite Persistence ──────────────────────────────────────────

/// Unified `SQLite` persistence backend implementing CAS, snapshot, and audit log storage.
#[derive(Debug)]
pub struct SqlitePersistence {
    pool: SqlitePool,
}

impl SqlitePersistence {
    /// Open a `SQLite` database and create all tables.
    /// Accepts any `SQLite` URL, e.g. `sqlite:///data/ferro.db`.
    pub async fn new(database_url: &str) -> Result<Self> {
        // Ensure WAL mode and foreign keys for better concurrent access
        let opts = SqliteConnectOptions::from_str(database_url)
            .map_err(|e| FerroError::Internal(format!("Invalid SQLite URL: {e}")))?
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .foreign_keys(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(opts)
            .await
            .map_err(|e| FerroError::Internal(format!("Failed to connect to SQLite: {e}")))?;

        // Run all migrations
        let ddl = [
            r"
            CREATE TABLE IF NOT EXISTS cas_content (
                hash TEXT PRIMARY KEY,
                content BLOB NOT NULL,
                size INTEGER NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            )",
            "CREATE INDEX IF NOT EXISTS idx_cas_created ON cas_content(created_at)",
            r"
            CREATE TABLE IF NOT EXISTS snapshots (
                id TEXT PRIMARY KEY,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                description TEXT NOT NULL DEFAULT '',
                entries_json TEXT NOT NULL,
                entry_count INTEGER NOT NULL DEFAULT 0
            )",
            "CREATE INDEX IF NOT EXISTS idx_snapshots_created ON snapshots(created_at)",
            r"
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
            )",
            "CREATE INDEX IF NOT EXISTS idx_audit_timestamp ON audit_log(timestamp)",
            // AV-009: Add chain_hash column for tamper-evident audit log (idempotent - skip if exists)
            "ALTER TABLE audit_log ADD COLUMN chain_hash TEXT",
            r"CREATE INDEX IF NOT EXISTS idx_audit_chain_hash ON audit_log(chain_hash)",
            r"
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
            )",
        ];

        for stmt in &ddl {
            match sqlx::query(stmt).execute(&pool).await {
                Ok(_) => {}
                Err(e) => {
                    // Ignore "duplicate column" errors for idempotent migrations
                    let msg = e.to_string();
                    if msg.contains("duplicate column") || msg.contains("already exists") {
                        debug!("Migration skip (already applied): {}", msg);
                    } else {
                        return Err(FerroError::Internal(format!("SQLite migration failed: {e}")));
                    }
                }
            }
        }

        info!("SQLite persistence initialized: {}", database_url);
        Ok(Self { pool })
    }

    /// Return the inner pool for use by `SqliteMetadataStore`.
    /// This allows sharing a single connection pool across metadata + other stores.
    #[must_use]
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
        let result = sqlx::query("INSERT OR IGNORE INTO cas_content (hash, content, size) VALUES (?, ?, ?)")
            .bind(hash_str)
            .bind(content.as_ref())
            .bind(size)
            .execute(&self.pool)
            .await
            .map_err(|e| FerroError::Internal(format!("CAS put failed: {e}")))?;

        if result.rows_affected() > 0 {
            debug!("CAS PUT: stored content {}", &hash_str[..16]);
        } else {
            debug!("CAS DEDUP: content {} already exists", &hash_str[..16]);
        }

        Ok(hash)
    }

    async fn get_content(&self, hash: &ContentHash) -> Result<Bytes> {
        let row: Option<(Vec<u8>,)> = sqlx::query_as("SELECT content FROM cas_content WHERE hash = ?")
            .bind(hash.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| FerroError::Internal(format!("CAS get failed: {e}")))?;

        row.map(|(bytes,)| Bytes::from(bytes))
            .ok_or_else(|| FerroError::NotFound(format!("content hash {}", hash.as_str())))
    }

    async fn exists(&self, hash: &ContentHash) -> Result<bool> {
        let result: Option<(i64,)> = sqlx::query_as("SELECT COUNT(*) as cnt FROM cas_content WHERE hash = ?")
            .bind(hash.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| FerroError::Internal(format!("CAS exists check failed: {e}")))?;

        Ok(result.is_some_and(|(cnt,)| cnt > 0))
    }

    async fn dedup_check(&self, hash: &ContentHash) -> Result<bool> {
        self.exists(hash).await
    }

    async fn content_count(&self) -> usize {
        let result: Option<(i64,)> = sqlx::query_as("SELECT COUNT(*) as cnt FROM cas_content")
            .fetch_optional(&self.pool)
            .await
            .unwrap_or(None);

        result.map_or(0, |(cnt,)| cnt as usize)
    }
}

// ── Snapshot Store implementation ─────────────────────────────────────────

#[async_trait]
impl SnapshotStore for SqlitePersistence {
    async fn create(&self, description: String, entries: Vec<FileMetadata>) -> Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        let entries_json = serde_json::to_string(&entries)
            .map_err(|e| FerroError::Internal(format!("Failed to serialize snapshot entries: {e}")))?;
        let entry_count = entries.len();

        sqlx::query("INSERT INTO snapshots (id, description, entries_json, entry_count) VALUES (?, ?, ?, ?)")
            .bind(&id)
            .bind(&description)
            .bind(&entries_json)
            .bind(entry_count as i64)
            .execute(&self.pool)
            .await
            .map_err(|e| FerroError::Internal(format!("Snapshot create failed: {e}")))?;

        debug!("Snapshot created: {} ({} entries)", id, entry_count);
        Ok(id)
    }

    async fn get(&self, id: &str) -> Result<PersistedSnapshot> {
        let row: Option<(String, String, String, String, i64)> =
            sqlx::query_as("SELECT id, created_at, description, entries_json, entry_count FROM snapshots WHERE id = ?")
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| FerroError::Internal(format!("Snapshot get failed: {e}")))?;

        row.map(
            |(id, created_at, description, entries_json, entry_count)| PersistedSnapshot {
                id,
                created_at,
                description,
                entries_json,
                entry_count: entry_count as usize,
            },
        )
        .ok_or_else(|| FerroError::NotFound(format!("snapshot {id}")))
    }

    async fn list(&self) -> Result<Vec<PersistedSnapshotSummary>> {
        let rows: Vec<(String, String, String, i64)> =
            sqlx::query_as("SELECT id, created_at, description, entry_count FROM snapshots ORDER BY created_at DESC")
                .fetch_all(&self.pool)
                .await
                .map_err(|e| FerroError::Internal(format!("Snapshot list failed: {e}")))?;

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
            .map_err(|e| FerroError::Internal(format!("Snapshot delete failed: {e}")))?;

        if result.rows_affected() == 0 {
            return Err(FerroError::NotFound(format!("snapshot {id}")));
        }
        Ok(())
    }

    async fn entry_count(&self) -> usize {
        let result: Option<(i64,)> = sqlx::query_as("SELECT COUNT(*) as cnt FROM snapshots")
            .fetch_optional(&self.pool)
            .await
            .unwrap_or(None);

        result.map_or(0, |(cnt,)| cnt as usize)
    }
}

// ── Audit Log Store implementation ──────────────────────────────────────────

#[async_trait]
impl AuditLogStore for SqlitePersistence {
    async fn log(&self, entry: PersistedAuditEntry) -> Result<()> {
        // Compute chain hash for tamper evidence: SHA-256(previous_hash || entry_data)
        let prev_hash: Option<String> =
            sqlx::query_scalar::<_, String>("SELECT chain_hash FROM audit_log ORDER BY id DESC LIMIT 1")
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| FerroError::Internal(format!("Audit chain hash lookup failed: {e}")))?;

        let chain_hash = {
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            if let Some(ref ph) = prev_hash {
                hasher.update(ph.as_bytes());
            }
            hasher.update(entry.timestamp.as_bytes());
            hasher.update(entry.method.as_bytes());
            hasher.update(entry.path.as_bytes());
            hasher.update(entry.user.as_bytes());
            hasher.update(entry.status.to_le_bytes());
            if let Some(ref ip) = entry.client_ip {
                hasher.update(ip.as_bytes());
            }
            if let Some(ref ua) = entry.user_agent {
                hasher.update(ua.as_bytes());
            }
            if let Some(cl) = entry.content_length {
                hasher.update(cl.to_le_bytes());
            }
            hex::encode(hasher.finalize())
        };

        sqlx::query(
            r"INSERT INTO audit_log
                (timestamp, method, path, user, status, client_ip, user_agent, content_length, chain_hash)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&entry.timestamp)
        .bind(&entry.method)
        .bind(&entry.path)
        .bind(&entry.user)
        .bind(i64::from(entry.status))
        .bind(&entry.client_ip)
        .bind(&entry.user_agent)
        .bind(entry.content_length.map(|c| c as i64))
        .bind(&chain_hash)
        .execute(&self.pool)
        .await
        .map_err(|e| FerroError::Internal(format!("Audit log insert failed: {e}")))?;

        Ok(())
    }

    async fn recent(&self, limit: usize) -> Result<Vec<PersistedAuditEntry>> {
        let limit = limit.min(10000) as i64; // Cap at 10k
        let rows: Vec<PersistedAuditEntry> = sqlx::query_as(
            "SELECT id, timestamp, method, path, user, status, client_ip, user_agent, content_length, chain_hash
             FROM audit_log ORDER BY id DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| FerroError::Internal(format!("Audit log recent failed: {e}")))?;

        Ok(rows)
    }

    async fn count(&self) -> usize {
        let result: Option<(i64,)> = sqlx::query_as("SELECT COUNT(*) as cnt FROM audit_log")
            .fetch_optional(&self.pool)
            .await
            .unwrap_or(None);

        result.map_or(0, |(cnt,)| cnt as usize)
    }
}

impl SqlitePersistence {
    /// Verify the audit log chain hash integrity.
    ///
    /// Reads all entries ordered by `id`, recomputes each `chain_hash` as
    /// `SHA-256(prev_hash || entry_data)`, and compares against the stored
    /// value. Returns a report with the total entries checked and any
    /// mismatches found.
    pub async fn verify_audit_chain(&self) -> ChainVerificationReport {
        use sha2::{Digest, Sha256};

        let rows: Vec<PersistedAuditEntry> = match sqlx::query_as(
            "SELECT id, timestamp, method, path, user, status, client_ip, user_agent, content_length, chain_hash
             FROM audit_log ORDER BY id ASC",
        )
        .fetch_all(&self.pool)
        .await
        {
            Ok(r) => r,
            Err(e) => {
                return ChainVerificationReport {
                    total_entries: 0,
                    verified: 0,
                    mismatches: 1,
                    skipped_no_hash: 0,
                    findings: vec![ChainMismatch {
                        entry_id: 0,
                        stored_hash: String::new(),
                        computed_hash: String::new(),
                        description: format!("Failed to read audit log: {e}"),
                    }],
                };
            }
        };

        let total = rows.len();
        let mut verified = 0usize;
        let mut mismatches = 0usize;
        let mut skipped_no_hash = 0usize;
        let mut findings = Vec::new();
        let mut prev_hash: Option<String> = None;

        for row in &rows {
            let stored = match &row.chain_hash {
                Some(h) if !h.is_empty() => h.clone(),
                _ => {
                    // Entries created before chain hashing was enabled have no hash.
                    // Do NOT synthesize a hash — keep prev_hash as-is to match
                    // the behavior of the log() method which sees NULL prev_hash.
                    skipped_no_hash += 1;
                    continue;
                }
            };

            // Recompute the expected hash
            let mut hasher = Sha256::new();
            if let Some(ref ph) = prev_hash {
                hasher.update(ph.as_bytes());
            }
            hasher.update(row.timestamp.as_bytes());
            hasher.update(row.method.as_bytes());
            hasher.update(row.path.as_bytes());
            hasher.update(row.user.as_bytes());
            hasher.update(row.status.to_le_bytes());
            if let Some(ref ip) = row.client_ip {
                hasher.update(ip.as_bytes());
            }
            if let Some(ref ua) = row.user_agent {
                hasher.update(ua.as_bytes());
            }
            if let Some(cl) = row.content_length {
                hasher.update(cl.to_le_bytes());
            }
            let computed = hex::encode(hasher.finalize());

            if computed == stored {
                verified += 1;
            } else {
                mismatches += 1;
                findings.push(ChainMismatch {
                    entry_id: row.id,
                    stored_hash: stored.clone(),
                    computed_hash: computed,
                    description: format!("Chain hash mismatch at entry id={}: stored != computed", row.id),
                });
            }

            prev_hash = Some(stored);
        }

        ChainVerificationReport {
            total_entries: total,
            verified,
            mismatches,
            skipped_no_hash,
            findings,
        }
    }
}

/// Report from verifying audit log chain hash integrity.
#[derive(Debug, Clone, Serialize)]
pub struct ChainVerificationReport {
    pub total_entries: usize,
    pub verified: usize,
    pub mismatches: usize,
    pub skipped_no_hash: usize,
    pub findings: Vec<ChainMismatch>,
}

/// A single chain hash mismatch found during verification.
#[derive(Debug, Clone, Serialize)]
pub struct ChainMismatch {
    pub entry_id: i64,
    pub stored_hash: String,
    pub computed_hash: String,
    pub description: String,
}

// ── AuditLogPersistence trait impl ────────────────────────────────────────

#[async_trait]
impl ferro_common::audit::AuditLogPersistence for SqlitePersistence {
    async fn log(&self, entry: ferro_common::audit::PersistedAuditEntry) -> std::result::Result<(), String> {
        let persisted = PersistedAuditEntry {
            id: entry.id,
            timestamp: entry.timestamp,
            method: entry.method,
            path: entry.path,
            user: entry.user,
            status: entry.status,
            client_ip: entry.client_ip,
            user_agent: entry.user_agent,
            content_length: entry.content_length,
            chain_hash: entry.chain_hash,
        };
        AuditLogStore::log(self, persisted).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    async fn count(&self) -> usize {
        AuditLogStore::count(self).await
    }

    async fn recent(&self, limit: usize) -> Vec<ferro_common::audit::PersistedAuditEntry> {
        AuditLogStore::recent(self, limit)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|e| ferro_common::audit::PersistedAuditEntry {
                id: e.id,
                timestamp: e.timestamp,
                method: e.method,
                path: e.path,
                user: e.user,
                status: e.status,
                client_ip: e.client_ip,
                user_agent: e.user_agent,
                content_length: e.content_length,
                chain_hash: e.chain_hash,
            })
            .collect()
    }

    async fn verify_audit_chain(&self) -> Option<ferro_common::audit::ChainVerificationReport> {
        let report = self.verify_audit_chain().await;
        Some(ferro_common::audit::ChainVerificationReport {
            total_entries: report.total_entries,
            verified: report.verified,
            mismatches: report.mismatches,
            skipped_no_hash: report.skipped_no_hash,
            findings: report
                .findings
                .into_iter()
                .map(|f| ferro_common::audit::ChainMismatch {
                    entry_id: f.entry_id,
                    stored_hash: f.stored_hash,
                    computed_hash: f.computed_hash,
                    description: f.description,
                })
                .collect(),
        })
    }
}

// ── Metadata Store: reuse existing SqliteMetadataStore ────────────────────
//
// The existing `SqliteMetadataStore` in `sqlx_metadata.rs` already has the
// file_metadata table. Since our unified store creates the same table,
// we provide `from_pool()` to avoid creating a duplicate connection.

// ── Tests ─────────────────────────────────────────────────────────────

// ── CAS garbage collection (aperture-vault pattern, R5) ─────────────────

/// Report of a CAS garbage-collection pass.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct CasGcReport {
    /// Total rows in `cas_content`.
    pub scanned: u64,
    /// Rows protected by a reference (metadata, snapshots, or extra roots).
    pub referenced: u64,
    /// Rows older than the cutoff with no reference found.
    pub orphan_candidates: u64,
    /// Rows actually deleted (0 in dry-run mode).
    pub swept: u64,
    /// Hashes whose content no longer matches its digest (see `verify_cas`).
    pub corrupt: Vec<String>,
    pub dry_run: bool,
}

impl SqlitePersistence {
    /// Re-verify up to `limit` CAS rows against their digests and return the
    /// hashes whose content no longer matches (bit-rot detection — the
    /// aperture-vault `verify` contract). `limit <= 0` scans everything.
    pub async fn verify_cas(&self, limit: i64) -> Result<Vec<String>> {
        let q = if limit > 0 {
            format!("SELECT hash, content FROM cas_content LIMIT {limit}")
        } else {
            "SELECT hash, content FROM cas_content".to_string()
        };
        let rows: Vec<(String, Vec<u8>)> = sqlx::query_as(&q)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| FerroError::Internal(format!("CAS verify failed: {e}")))?;
        let mut corrupt = Vec::new();
        for (hash, content) in rows {
            if ContentHash::compute(&Bytes::from(content)).as_str() != hash {
                corrupt.push(hash);
            }
        }
        Ok(corrupt)
    }

    /// Orphan sweep for the CAS (ALG-CAS-GC-001, R5-A): delete rows that are
    /// (a) older than `older_than_days`, (b) not referenced by
    /// `file_metadata.content_hash`, (c) not pinned by any snapshot JSON
    /// containing the hash, and (d) not in `extra_roots` (callers pass sync
    /// manifest block hashes, which the server does not track table-side).
    ///
    /// `dry_run = true` reports candidates without deleting.
    pub async fn cas_gc(&self, older_than_days: u32, extra_roots: &[String], dry_run: bool) -> Result<CasGcReport> {
        let scanned: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM cas_content")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| FerroError::Internal(format!("CAS gc scan failed: {e}")))?;

        // Protected set 1: metadata references.
        let meta_referenced: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM cas_content WHERE hash IN (SELECT content_hash FROM file_metadata)")
                .fetch_one(&self.pool)
                .await
                .map_err(|e| FerroError::Internal(format!("CAS gc failed: {e}")))?;

        // Protected set 2: snapshot-pinned (hash appears in entries_json).
        let snap_protected: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM cas_content c WHERE EXISTS (
                 SELECT 1 FROM snapshots s WHERE instr(s.entries_json, c.hash) > 0)",
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| FerroError::Internal(format!("CAS gc failed: {e}")))?;

        // Orphan candidates: old + not metadata-referenced + not
        // snapshot-pinned + not an extra root.
        let mut sql = String::from(
            "SELECT hash FROM cas_content
             WHERE created_at < datetime('now', '-' || ? || ' days')
               AND hash NOT IN (SELECT content_hash FROM file_metadata)
               AND NOT EXISTS (SELECT 1 FROM snapshots s WHERE instr(s.entries_json, cas_content.hash) > 0)",
        );
        for _ in extra_roots {
            sql.push_str(" AND hash != ?");
        }
        let mut q = sqlx::query_as::<_, (String,)>(&sql).bind(older_than_days.to_string());
        for root in extra_roots {
            q = q.bind(root);
        }
        let orphans: Vec<(String,)> = q
            .fetch_all(&self.pool)
            .await
            .map_err(|e| FerroError::Internal(format!("CAS gc failed: {e}")))?;

        let mut report = CasGcReport {
            scanned: scanned.0.max(0) as u64,
            referenced: meta_referenced.0.max(0) as u64,
            orphan_candidates: orphans.len() as u64,
            swept: 0,
            corrupt: Vec::new(),
            dry_run,
        };

        if dry_run {
            return Ok(report);
        }

        for (hash,) in &orphans {
            let res = sqlx::query("DELETE FROM cas_content WHERE hash = ?")
                .bind(hash)
                .execute(&self.pool)
                .await
                .map_err(|e| FerroError::Internal(format!("CAS gc delete failed: {e}")))?;
            report.swept += res.rows_affected() as u64;
        }

        Ok(report)
    }
}

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
            ContentHash::new("a".repeat(64)).expect("valid hardcoded hash"),
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
            chain_hash: None,
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
            store
                .log(PersistedAuditEntry {
                    id: 0,
                    timestamp: "2026-04-21T12:00:00Z".to_string(),
                    method: "GET".to_string(),
                    path: format!("/file{}.txt", i),
                    user: "bob".to_string(),
                    status: 200,
                    client_ip: None,
                    user_agent: None,
                    content_length: None,
                    chain_hash: None,
                })
                .await
                .unwrap();
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
        store
            .log(PersistedAuditEntry {
                id: 0,
                timestamp: "now".to_string(),
                method: "GET".to_string(),
                path: "/".to_string(),
                user: "test".to_string(),
                status: 200,
                client_ip: None,
                user_agent: None,
                content_length: None,
                chain_hash: None,
            })
            .await
            .unwrap();

        assert_eq!(store.content_count().await, 1);
        assert_eq!(store.entry_count().await, 1);
        assert_eq!(store.count().await, 1);
    }

    #[tokio::test]
    async fn test_audit_chain_verification_valid() {
        let (_tmp, store) = make_store().await;

        // Insert several entries — chain hashes are computed automatically
        for i in 0..5 {
            store
                .log(PersistedAuditEntry {
                    id: 0,
                    timestamp: format!("2026-01-01T00:{:02}:00Z", i),
                    method: "PUT".to_string(),
                    path: format!("/file{}.txt", i),
                    user: "alice".to_string(),
                    status: 201,
                    client_ip: Some("10.0.0.1".to_string()),
                    user_agent: None,
                    content_length: Some(100 + i as u64),
                    chain_hash: None,
                })
                .await
                .unwrap();
        }

        let report = store.verify_audit_chain().await;
        assert_eq!(report.total_entries, 5);
        assert_eq!(report.verified, 5);
        assert_eq!(report.mismatches, 0);
        assert!(report.findings.is_empty());
    }

    #[tokio::test]
    async fn test_audit_chain_detection_tamper() {
        let (_tmp, store) = make_store().await;

        // Insert 3 entries
        for i in 0..3 {
            store
                .log(PersistedAuditEntry {
                    id: 0,
                    timestamp: format!("2026-01-01T00:{:02}:00Z", i),
                    method: "GET".to_string(),
                    path: format!("/doc{}.txt", i),
                    user: "bob".to_string(),
                    status: 200,
                    client_ip: None,
                    user_agent: None,
                    content_length: None,
                    chain_hash: None,
                })
                .await
                .unwrap();
        }

        // Tamper with entry 2's chain_hash directly in SQLite
        sqlx::query("UPDATE audit_log SET chain_hash = 'deadbeef' WHERE id = 2")
            .execute(&store.pool)
            .await
            .unwrap();

        let report = store.verify_audit_chain().await;
        // Entry 2 should be a mismatch, entry 3 will also mismatch because
        // the chain is broken (prev_hash for entry 3 came from the tampered entry 2)
        assert!(report.mismatches >= 1, "Expected at least 1 mismatch");
        assert!(
            report.findings.iter().any(|f| f.entry_id == 2),
            "Expected mismatch at entry id=2"
        );
    }

    #[tokio::test]
    async fn test_audit_chain_verification_skips_no_hash() {
        let (_tmp, store) = make_store().await;

        // Manually insert an entry without a chain_hash (simulating pre-migration data)
        sqlx::query(
            "INSERT INTO audit_log (timestamp, method, path, user, status, chain_hash) VALUES (?, ?, ?, ?, ?, NULL)",
        )
        .bind("2025-01-01T00:00:00Z")
        .bind("GET")
        .bind("/old.txt")
        .bind("legacy")
        .bind(200i64)
        .execute(&store.pool)
        .await
        .unwrap();

        // Insert a modern entry that chains from the legacy one
        store
            .log(PersistedAuditEntry {
                id: 0,
                timestamp: "2026-01-01T00:00:00Z".to_string(),
                method: "PUT".to_string(),
                path: "/new.txt".to_string(),
                user: "alice".to_string(),
                status: 201,
                client_ip: None,
                user_agent: None,
                content_length: None,
                chain_hash: None,
            })
            .await
            .unwrap();

        let report = store.verify_audit_chain().await;
        assert_eq!(report.total_entries, 2);
        assert_eq!(report.skipped_no_hash, 1);
        assert_eq!(report.verified, 1);
        assert_eq!(report.mismatches, 0);
    }

    #[tokio::test]
    async fn test_cas_gc_dry_run_and_sweep() {
        let (tmp, url) = make_db_path();
        let store = SqlitePersistence::new(&url).await.unwrap();

        let live = store.put_content(Bytes::from_static(b"live content")).await.unwrap();
        let orphan = store.put_content(Bytes::from_static(b"orphan content")).await.unwrap();

        // Reference the live hash from file_metadata.
        sqlx::query("INSERT INTO file_metadata (path, content_hash) VALUES (?, ?)")
            .bind("/live.txt")
            .bind(live.as_str())
            .execute(&store.pool)
            .await
            .unwrap();

        // Age both rows past the cutoff so the sweep can consider them.
        sqlx::query("UPDATE cas_content SET created_at = datetime('now', '-30 days')")
            .execute(&store.pool)
            .await
            .unwrap();

        // Dry run: reports the orphan, deletes nothing.
        let report = store.cas_gc(7, &[], true).await.unwrap();
        assert_eq!(report.scanned, 2);
        assert_eq!(report.orphan_candidates, 1);
        assert_eq!(report.swept, 0);
        assert!(report.dry_run);

        // Real sweep.
        let report = store.cas_gc(7, &[], false).await.unwrap();
        assert_eq!(report.swept, 1);
        assert!(store.get_content(&orphan).await.is_err());
        assert!(store.get_content(&live).await.is_ok());
    }

    #[tokio::test]
    async fn test_cas_gc_respects_extra_roots_and_age() {
        let (tmp, url) = make_db_path();
        let store = SqlitePersistence::new(&url).await.unwrap();

        let pinned = store.put_content(Bytes::from_static(b"pinned by root")).await.unwrap();
        let fresh_orphan = store
            .put_content(Bytes::from_static(b"too young to sweep"))
            .await
            .unwrap();

        // Age ONLY the pinned row; the fresh orphan stays inside the cutoff.
        sqlx::query("UPDATE cas_content SET created_at = datetime('now', '-30 days') WHERE hash = ?")
            .bind(pinned.as_str())
            .execute(&store.pool)
            .await
            .unwrap();

        // Pinned by an extra root: not an orphan even though aged. The fresh
        // row is not an orphan either (younger than the cutoff).
        let report = store.cas_gc(7, &[pinned.as_str().to_string()], true).await.unwrap();
        assert_eq!(report.orphan_candidates, 0);

        // Age the unreferenced row: now it is the only candidate (the pinned
        // row is still excluded via the extra root).
        sqlx::query("UPDATE cas_content SET created_at = datetime('now', '-30 days') WHERE hash = ?")
            .bind(fresh_orphan.as_str())
            .execute(&store.pool)
            .await
            .unwrap();
        let report = store.cas_gc(7, &[pinned.as_str().to_string()], true).await.unwrap();
        assert_eq!(report.orphan_candidates, 1);

        // Sweep with the root: only the fresh orphan is removed.
        let report = store.cas_gc(7, &[pinned.as_str().to_string()], false).await.unwrap();
        assert_eq!(report.swept, 1);
        assert!(store.get_content(&fresh_orphan).await.is_err());
        assert!(store.get_content(&pinned).await.is_ok());
    }

    #[tokio::test]
    async fn test_cas_gc_snapshot_pin_and_verify() {
        let (tmp, url) = make_db_path();
        let store = SqlitePersistence::new(&url).await.unwrap();

        let pinned = store
            .put_content(Bytes::from_static(b"pinned by snapshot"))
            .await
            .unwrap();
        sqlx::query("UPDATE cas_content SET created_at = datetime('now', '-30 days')")
            .execute(&store.pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO snapshots (id, entries_json) VALUES (?, ?)")
            .bind("snap-1")
            .bind(format!("{{\"entries\":[{{\"hash\":\"{}\"}}]}}", pinned.as_str()))
            .execute(&store.pool)
            .await
            .unwrap();

        let report = store.cas_gc(7, &[], true).await.unwrap();
        assert_eq!(report.orphan_candidates, 0);

        // verify_cas: all rows healthy; a corrupt row is detected.
        assert!(store.verify_cas(-1).await.unwrap().is_empty());
        sqlx::query("UPDATE cas_content SET content = X'00' WHERE hash = ?")
            .bind(pinned.as_str())
            .execute(&store.pool)
            .await
            .unwrap();
        let corrupt = store.verify_cas(-1).await.unwrap();
        assert_eq!(corrupt, vec![pinned.as_str().to_string()]);
    }
}
