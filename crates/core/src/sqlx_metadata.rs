use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ferro_common::error::{FerroError, Result};
use ferro_common::metadata::{ContentHash, FileMetadata};
use sqlx::{FromRow, SqlitePool};
#[cfg(feature = "postgres")]
use sqlx::PgPool;
use tracing::debug;

use crate::metadata::MetadataStore;

/// PostgreSQL-backed metadata store.
#[cfg(feature = "postgres")]
pub struct PgMetadataStore {
    pool: PgPool,
}

#[cfg(feature = "postgres")]
impl PgMetadataStore {
    /// Connect to PostgreSQL and create the metadata table if it does not exist.
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = PgPool::connect(database_url)
            .await
            .map_err(|e| FerroError::Internal(format!("Failed to connect to PostgreSQL: {}", e)))?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS file_metadata (
                path VARCHAR(4096) PRIMARY KEY,
                content_hash VARCHAR(64) NOT NULL,
                size BIGINT NOT NULL DEFAULT 0,
                mime_type VARCHAR(256) NOT NULL DEFAULT 'application/octet-stream',
                is_collection BOOLEAN NOT NULL DEFAULT FALSE,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                modified_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                owner VARCHAR(256) NOT NULL DEFAULT 'anonymous',
                etag VARCHAR(128) NOT NULL DEFAULT ''
            )
        "#,
        )
        .execute(&pool)
        .await
        .map_err(|e| FerroError::Internal(format!("Migration failed: {}", e)))?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_file_metadata_path_prefix
            ON file_metadata (path varchar_pattern_ops)
        "#,
        )
        .execute(&pool)
        .await
        .map_err(|e| FerroError::Internal(format!("Index creation failed: {}", e)))?;

        debug!("PgMetadataStore initialized");
        Ok(Self { pool })
    }

    /// Return a reference to the underlying connection pool.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl MetadataStore for PgMetadataStore {
    async fn get(&self, path: &str) -> Result<FileMetadata> {
        let row = sqlx::query_as::<_, MetadataRow>(
            "SELECT path, content_hash, size, mime_type, is_collection, created_at, modified_at, owner, etag FROM file_metadata WHERE path = $1"
        )
        .bind(path)
        .fetch_one(&self.pool)
        .await
        .map_err(|_| FerroError::NotFound(path.to_string()))?;

        Ok(row.into())
    }

    async fn put(&self, metadata: FileMetadata) -> Result<()> {
        sqlx::query(r#"
            INSERT INTO file_metadata (path, content_hash, size, mime_type, is_collection, created_at, modified_at, owner, etag)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT (path) DO UPDATE SET
                content_hash = $2, size = $3, mime_type = $4, is_collection = $5,
                modified_at = $7, owner = $8, etag = $9
        "#)
        .bind(&metadata.path)
        .bind(metadata.content_hash.as_str())
        .bind(metadata.size as i64)
        .bind(&metadata.mime_type)
        .bind(metadata.is_collection)
        .bind(metadata.created_at)
        .bind(metadata.modified_at)
        .bind(&metadata.owner)
        .bind(&metadata.etag)
        .execute(&self.pool)
        .await
        .map_err(|e| FerroError::Internal(format!("Put failed: {}", e)))?;

        debug!("META PUT: {}", metadata.path);
        Ok(())
    }

    async fn delete(&self, path: &str) -> Result<()> {
        let result = sqlx::query("DELETE FROM file_metadata WHERE path = $1")
            .bind(path)
            .execute(&self.pool)
            .await
            .map_err(|e| FerroError::Internal(format!("Delete failed: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(FerroError::NotFound(path.to_string()));
        }
        debug!("META DELETE: {}", path);
        Ok(())
    }

    async fn list(&self, prefix: &str) -> Result<Vec<FileMetadata>> {
        let parent_prefix = if prefix == "/" { "" } else { prefix };
        let pattern = if parent_prefix.is_empty() {
            "/[^/]+".to_string()
        } else {
            format!("{}/[^/]+", parent_prefix.trim_end_matches('/'))
        };

        let rows = sqlx::query_as::<_, MetadataRow>(
            "SELECT path, content_hash, size, mime_type, is_collection, created_at, modified_at, owner, etag FROM file_metadata WHERE path ~ $1 ORDER BY path"
        )
        .bind(&pattern)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| FerroError::Internal(format!("List failed: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn exists(&self, path: &str) -> Result<bool> {
        let result: (bool,) =
            sqlx::query_as("SELECT EXISTS(SELECT 1 FROM file_metadata WHERE path = $1)")
                .bind(path)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| FerroError::Internal(format!("Exists check failed: {}", e)))?;

        Ok(result.0)
    }
}

/// SQLite-backed metadata store.
pub struct SqliteMetadataStore {
    pool: SqlitePool,
}

impl SqliteMetadataStore {
    /// Connect to SQLite and create the metadata table if it does not exist.
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = SqlitePool::connect(database_url)
            .await
            .map_err(|e| FerroError::Internal(format!("Failed to connect to SQLite: {}", e)))?;

        sqlx::query(
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
            )
        "#,
        )
        .execute(&pool)
        .await
        .map_err(|e| FerroError::Internal(format!("Migration failed: {}", e)))?;

        debug!("SqliteMetadataStore initialized");
        Ok(Self { pool })
    }

    /// Create from an existing pool (e.g. shared with SqlitePersistence).
    /// Assumes the `file_metadata` table already exists.
    pub fn from_pool(pool: SqlitePool) -> Self {
        debug!("SqliteMetadataStore initialized from shared pool");
        Self { pool }
    }
}

#[async_trait]
impl MetadataStore for SqliteMetadataStore {
    async fn get(&self, path: &str) -> Result<FileMetadata> {
        let row = sqlx::query_as::<_, SqliteMetadataRow>(
            "SELECT path, content_hash, size, mime_type, is_collection, created_at, modified_at, owner, etag FROM file_metadata WHERE path = ?"
        )
        .bind(path)
        .fetch_one(&self.pool)
        .await
        .map_err(|_| FerroError::NotFound(path.to_string()))?;

        Ok(row.into())
    }

    async fn put(&self, metadata: FileMetadata) -> Result<()> {
        sqlx::query(r#"
            INSERT INTO file_metadata (path, content_hash, size, mime_type, is_collection, created_at, modified_at, owner, etag)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT (path) DO UPDATE SET
                content_hash = excluded.content_hash, size = excluded.size, mime_type = excluded.mime_type,
                is_collection = excluded.is_collection, modified_at = excluded.modified_at,
                owner = excluded.owner, etag = excluded.etag
        "#)
        .bind(&metadata.path)
        .bind(metadata.content_hash.as_str())
        .bind(metadata.size as i64)
        .bind(&metadata.mime_type)
        .bind(metadata.is_collection)
        .bind(metadata.created_at.to_rfc3339())
        .bind(metadata.modified_at.to_rfc3339())
        .bind(&metadata.owner)
        .bind(&metadata.etag)
        .execute(&self.pool)
        .await
        .map_err(|e| FerroError::Internal(format!("Put failed: {}", e)))?;

        Ok(())
    }

    async fn delete(&self, path: &str) -> Result<()> {
        let result = sqlx::query("DELETE FROM file_metadata WHERE path = ?")
            .bind(path)
            .execute(&self.pool)
            .await
            .map_err(|e| FerroError::Internal(format!("Delete failed: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(FerroError::NotFound(path.to_string()));
        }
        Ok(())
    }

    async fn list(&self, prefix: &str) -> Result<Vec<FileMetadata>> {
        let parent_prefix = if prefix == "/" {
            ""
        } else {
            prefix.trim_end_matches('/')
        };
        let pattern = format!("{}%", parent_prefix);
        let exclude_pattern = if parent_prefix.is_empty() {
            "/%/%/%".to_string()
        } else {
            format!("{}/%/%", parent_prefix)
        };

        let rows = sqlx::query_as::<_, SqliteMetadataRow>(
            "SELECT path, content_hash, size, mime_type, is_collection, created_at, modified_at, owner, etag FROM file_metadata WHERE path LIKE ? AND path NOT LIKE ? ORDER BY path"
        )
        .bind(&pattern)
        .bind(&exclude_pattern)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| FerroError::Internal(format!("List failed: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn exists(&self, path: &str) -> Result<bool> {
        let result: (bool,) =
            sqlx::query_as("SELECT EXISTS(SELECT 1 FROM file_metadata WHERE path = ?)")
                .bind(path)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| FerroError::Internal(format!("Exists check failed: {}", e)))?;

        Ok(result.0)
    }
}

#[derive(Debug, FromRow)]
struct MetadataRow {
    path: String,
    content_hash: String,
    size: i64,
    mime_type: String,
    is_collection: bool,
    created_at: DateTime<Utc>,
    modified_at: DateTime<Utc>,
    owner: String,
    etag: String,
}

impl From<MetadataRow> for FileMetadata {
    fn from(row: MetadataRow) -> Self {
        Self {
            path: row.path,
            content_hash: ContentHash::new(row.content_hash),
            size: row.size as u64,
            mime_type: row.mime_type,
            is_collection: row.is_collection,
            created_at: row.created_at,
            modified_at: row.modified_at,
            owner: row.owner,
            etag: row.etag,
        }
    }
}

#[derive(Debug, FromRow)]
struct SqliteMetadataRow {
    path: String,
    content_hash: String,
    size: i64,
    mime_type: String,
    is_collection: bool,
    created_at: String,
    modified_at: String,
    owner: String,
    etag: String,
}

impl From<SqliteMetadataRow> for FileMetadata {
    fn from(row: SqliteMetadataRow) -> Self {
        let created = DateTime::parse_from_rfc3339(&row.created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());
        let modified = DateTime::parse_from_rfc3339(&row.modified_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        Self {
            path: row.path,
            content_hash: ContentHash::new(row.content_hash),
            size: row.size as u64,
            mime_type: row.mime_type,
            is_collection: row.is_collection,
            created_at: created,
            modified_at: modified,
            owner: row.owner,
            etag: row.etag,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_metadata() -> FileMetadata {
        FileMetadata::new(
            "/test/file.txt".to_string(),
            ContentHash::new("a".repeat(64)),
            42,
            "anonymous".to_string(),
        )
    }

    async fn setup_store() -> SqliteMetadataStore {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let path_str = db_path.to_string_lossy().to_string();
        std::mem::forget(dir);
        std::fs::File::create(&path_str).unwrap();
        SqliteMetadataStore::new(&format!("sqlite:{}", path_str))
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn test_sqlite_put_and_get() {
        let store = setup_store().await;
        let meta = test_metadata();

        store.put(meta.clone()).await.unwrap();

        let retrieved = store.get("/test/file.txt").await.unwrap();
        assert_eq!(retrieved.path, "/test/file.txt");
        assert_eq!(retrieved.size, 42);
        assert_eq!(retrieved.mime_type, "application/octet-stream");
        assert!(!retrieved.is_collection);
        assert_eq!(retrieved.content_hash.as_str(), "a".repeat(64));
    }

    #[tokio::test]
    async fn test_sqlite_put_upsert() {
        let store = setup_store().await;

        let meta1 = FileMetadata::new(
            "/test/file.txt".to_string(),
            ContentHash::new("a".repeat(64)),
            10,
            "user1".to_string(),
        );
        store.put(meta1).await.unwrap();

        let meta2 = FileMetadata::new(
            "/test/file.txt".to_string(),
            ContentHash::new("b".repeat(64)),
            20,
            "user2".to_string(),
        );
        store.put(meta2).await.unwrap();

        let retrieved = store.get("/test/file.txt").await.unwrap();
        assert_eq!(retrieved.size, 20);
        assert_eq!(retrieved.owner, "user2");
        assert_eq!(retrieved.content_hash.as_str(), "b".repeat(64));
    }

    #[tokio::test]
    async fn test_sqlite_delete() {
        let store = setup_store().await;
        let meta = test_metadata();

        store.put(meta).await.unwrap();
        assert!(store.exists("/test/file.txt").await.unwrap());

        store.delete("/test/file.txt").await.unwrap();
        assert!(!store.exists("/test/file.txt").await.unwrap());

        let result = store.delete("/test/nonexistent.txt").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_sqlite_exists() {
        let store = setup_store().await;
        assert!(!store.exists("/test/nope.txt").await.unwrap());

        let meta = test_metadata();
        store.put(meta).await.unwrap();
        assert!(store.exists("/test/file.txt").await.unwrap());
    }

    #[tokio::test]
    async fn test_sqlite_list() {
        let store = setup_store().await;

        store
            .put(FileMetadata::new(
                "/docs/readme.md".to_string(),
                ContentHash::new("a".repeat(64)),
                100,
                "user".to_string(),
            ))
            .await
            .unwrap();

        store
            .put(FileMetadata::new(
                "/docs/nested/file.txt".to_string(),
                ContentHash::new("b".repeat(64)),
                200,
                "user".to_string(),
            ))
            .await
            .unwrap();

        store
            .put(FileMetadata::new(
                "/other/file.txt".to_string(),
                ContentHash::new("c".repeat(64)),
                300,
                "user".to_string(),
            ))
            .await
            .unwrap();

        let items = store.list("/docs").await.unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].path, "/docs/readme.md");

        let items = store.list("/").await.unwrap();
        assert_eq!(items.len(), 2);
        let paths: Vec<&str> = items.iter().map(|i| i.path.as_str()).collect();
        assert!(paths.contains(&"/docs/readme.md"));
        assert!(paths.contains(&"/other/file.txt"));
    }

    #[tokio::test]
    async fn test_sqlite_collection() {
        let store = setup_store().await;

        let mut meta = FileMetadata::new(
            "/test/folder".to_string(),
            ContentHash::new("d".repeat(64)),
            0,
            "user".to_string(),
        );
        meta.is_collection = true;
        meta.mime_type = "inode/directory".to_string();

        store.put(meta).await.unwrap();

        let retrieved = store.get("/test/folder").await.unwrap();
        assert!(retrieved.is_collection);
        assert_eq!(retrieved.mime_type, "inode/directory");
    }

    #[tokio::test]
    async fn test_sqlite_get_nonexistent() {
        let store = setup_store().await;
        let result = store.get("/nonexistent").await;
        assert!(result.is_err());
    }
}
