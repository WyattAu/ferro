use rusqlite::{Connection, params};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::fs;

pub struct OfflineCache {
    db: Arc<Mutex<Connection>>,
    blobs_dir: PathBuf,
}

impl OfflineCache {
    pub fn new(cache_dir: PathBuf) -> Result<Self, String> {
        std::fs::create_dir_all(&cache_dir).map_err(|e| e.to_string())?;
        let blobs_dir = cache_dir.join("blobs");
        std::fs::create_dir_all(&blobs_dir).map_err(|e| e.to_string())?;

        let db_path = cache_dir.join("cache.db");
        let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS cache_entries (
                remote_path TEXT PRIMARY KEY,
                cache_key TEXT NOT NULL,
                size INTEGER NOT NULL,
                mtime INTEGER NOT NULL,
                etag TEXT,
                last_accessed INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_cache_entries_mtime ON cache_entries(mtime);
            CREATE TABLE IF NOT EXISTS pending_writes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                remote_path TEXT NOT NULL,
                local_blob_key TEXT NOT NULL,
                queued_at INTEGER NOT NULL
            );",
        )
        .map_err(|e| e.to_string())?;

        Ok(Self {
            db: Arc::new(Mutex::new(conn)),
            blobs_dir,
        })
    }

    pub async fn put(
        &self,
        remote_path: &str,
        data: &[u8],
        etag: Option<&str>,
    ) -> Result<u64, String> {
        let cache_key = Self::hash_content(data);
        let blob_path = self.blobs_dir.join(&cache_key);

        fs::write(&blob_path, data)
            .await
            .map_err(|e| e.to_string())?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let db = self.db.lock().map_err(|e| e.to_string())?;
        db.execute(
            "INSERT OR REPLACE INTO cache_entries (remote_path, cache_key, size, mtime, etag, last_accessed) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![remote_path, cache_key, data.len() as i64, now, etag, now],
        ).map_err(|e| e.to_string())?;

        Ok(data.len() as u64)
    }

    pub async fn get(&self, remote_path: &str) -> Result<Option<Vec<u8>>, String> {
        let cache_key = {
            let db = self.db.lock().map_err(|e| e.to_string())?;
            let mut stmt = db
                .prepare("SELECT cache_key FROM cache_entries WHERE remote_path = ?1")
                .map_err(|e| e.to_string())?;
            let key: Option<String> = stmt.query_row(params![remote_path], |row| row.get(0)).ok();
            match key {
                Some(k) => k,
                None => return Ok(None),
            }
        };

        let blob_path = self.blobs_dir.join(&cache_key);
        if !blob_path.exists() {
            return Ok(None);
        }

        let data = fs::read(&blob_path).await.map_err(|e| e.to_string())?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if let Ok(db) = self.db.lock() {
            let _ = db.execute(
                "UPDATE cache_entries SET last_accessed = ?1 WHERE remote_path = ?2",
                params![now, remote_path],
            );
        }

        Ok(Some(data))
    }

    pub async fn queue_write(&self, remote_path: &str, data: &[u8]) -> Result<(), String> {
        let cache_key = Self::hash_content(data);
        let blob_path = self.blobs_dir.join(&cache_key);
        fs::write(&blob_path, data)
            .await
            .map_err(|e| e.to_string())?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let db = self.db.lock().map_err(|e| e.to_string())?;
        db.execute(
            "INSERT INTO pending_writes (remote_path, local_blob_key, queued_at) VALUES (?1, ?2, ?3)",
            params![remote_path, cache_key, now],
        ).map_err(|e| e.to_string())?;

        Ok(())
    }

    #[allow(dead_code)]
    pub fn get_pending_writes(&self) -> Result<Vec<(String, String)>, String> {
        let db = self.db.lock().map_err(|e| e.to_string())?;
        let mut stmt = db
            .prepare(
                "SELECT remote_path, local_blob_key FROM pending_writes ORDER BY queued_at ASC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    #[allow(dead_code)]
    pub fn clear_pending_write(&self, remote_path: &str) -> Result<(), String> {
        let db = self.db.lock().map_err(|e| e.to_string())?;
        db.execute(
            "DELETE FROM pending_writes WHERE remote_path = ?1",
            params![remote_path],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn invalidate(&self, remote_path: &str) -> Result<(), String> {
        let db = self.db.lock().map_err(|e| e.to_string())?;
        db.execute(
            "DELETE FROM cache_entries WHERE remote_path = ?1",
            params![remote_path],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn stats(&self) -> Result<CacheStats, String> {
        let db = self.db.lock().map_err(|e| e.to_string())?;
        let cached_files: i64 = db
            .query_row("SELECT COUNT(*) FROM cache_entries", [], |row| row.get(0))
            .unwrap_or(0);
        let pending_writes: i64 = db
            .query_row("SELECT COUNT(*) FROM pending_writes", [], |row| row.get(0))
            .unwrap_or(0);
        let total_size: i64 = db
            .query_row(
                "SELECT COALESCE(SUM(size), 0) FROM cache_entries",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);
        Ok(CacheStats {
            cached_files,
            pending_writes,
            total_bytes: total_size,
        })
    }

    fn hash_content(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hex::encode(hasher.finalize())
    }
}

#[allow(dead_code)]
pub struct CacheStats {
    pub cached_files: i64,
    pub pending_writes: i64,
    pub total_bytes: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_cache_put_get() {
        let dir = tempdir().unwrap();
        let cache = OfflineCache::new(dir.path().to_path_buf()).unwrap();

        cache
            .put("/test/file.txt", b"hello world", Some("etag-1"))
            .await
            .unwrap();

        let result = cache.get("/test/file.txt").await.unwrap();
        assert_eq!(result, Some(b"hello world".to_vec()));
    }

    #[tokio::test]
    async fn test_cache_miss() {
        let dir = tempdir().unwrap();
        let cache = OfflineCache::new(dir.path().to_path_buf()).unwrap();

        let result = cache.get("/nonexistent").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_cache_overwrite() {
        let dir = tempdir().unwrap();
        let cache = OfflineCache::new(dir.path().to_path_buf()).unwrap();

        cache.put("/file.txt", b"v1", None).await.unwrap();
        cache.put("/file.txt", b"v2", None).await.unwrap();

        let result = cache.get("/file.txt").await.unwrap();
        assert_eq!(result, Some(b"v2".to_vec()));
    }

    #[tokio::test]
    async fn test_pending_writes() {
        let dir = tempdir().unwrap();
        let cache = OfflineCache::new(dir.path().to_path_buf()).unwrap();

        cache
            .queue_write("/queued/file.txt", b"pending data")
            .await
            .unwrap();
        cache
            .queue_write("/queued/other.txt", b"more pending")
            .await
            .unwrap();

        let pending = cache.get_pending_writes().unwrap();
        assert_eq!(pending.len(), 2);
        assert_eq!(pending[0].0, "/queued/file.txt");

        cache.clear_pending_write("/queued/file.txt").unwrap();
        let pending = cache.get_pending_writes().unwrap();
        assert_eq!(pending.len(), 1);
    }

    #[tokio::test]
    async fn test_cache_invalidate() {
        let dir = tempdir().unwrap();
        let cache = OfflineCache::new(dir.path().to_path_buf()).unwrap();

        cache.put("/file.txt", b"data", None).await.unwrap();
        cache.invalidate("/file.txt").unwrap();

        let result = cache.get("/file.txt").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_cache_stats() {
        let dir = tempdir().unwrap();
        let cache = OfflineCache::new(dir.path().to_path_buf()).unwrap();

        cache.put("/a.txt", b"aaa", None).await.unwrap();
        cache.put("/b.txt", b"bbbb", None).await.unwrap();

        let stats = cache.stats().unwrap();
        assert_eq!(stats.cached_files, 2);
        assert_eq!(stats.total_bytes, 7);
        assert_eq!(stats.pending_writes, 0);
    }

    #[tokio::test]
    async fn test_deduplication_same_content() {
        let dir = tempdir().unwrap();
        let cache = OfflineCache::new(dir.path().to_path_buf()).unwrap();

        cache
            .put("/path1.txt", b"same content", None)
            .await
            .unwrap();
        cache
            .put("/path2.txt", b"same content", None)
            .await
            .unwrap();

        assert!(cache.get("/path1.txt").await.unwrap().is_some());
        assert!(cache.get("/path2.txt").await.unwrap().is_some());

        let stats = cache.stats().unwrap();
        assert_eq!(stats.cached_files, 2);
        assert_eq!(stats.total_bytes, 24);
    }
}
