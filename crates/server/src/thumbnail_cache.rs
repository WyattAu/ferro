use sha2::{Digest, Sha256};
use std::num::NonZeroUsize;
use std::path::{Path as StdPath, PathBuf};
use std::sync::{Arc, Mutex};
use tracing::warn;

const DEFAULT_MAX_BYTES: u64 = 100 * 1024 * 1024;
const DEFAULT_MAX_ENTRIES: usize = 10_000;

#[derive(serde::Serialize, serde::Deserialize)]
struct Meta {
    mime: String,
    timestamp: i64,
}

#[derive(Clone)]
pub struct ThumbnailCache {
    dir: PathBuf,
    lru: Arc<Mutex<lru::LruCache<String, Vec<u8>>>>,
    max_entries: usize,
}

impl std::fmt::Debug for ThumbnailCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ThumbnailCache")
            .field("dir", &self.dir)
            .field("max_entries", &self.max_entries)
            .finish()
    }
}

impl ThumbnailCache {
    pub fn new(data_dir: &str, _max_bytes: u64, max_entries: usize) -> Self {
        let dir = StdPath::new(data_dir).join("thumbnails");
        std::fs::create_dir_all(&dir).ok();

        let caps = if max_entries > 0 {
            max_entries
        } else {
            DEFAULT_MAX_ENTRIES
        };

        Self {
            dir,
            lru: Arc::new(Mutex::new(lru::LruCache::new(
                NonZeroUsize::new(caps).unwrap(),
            ))),
            max_entries: caps,
        }
    }

    pub fn noop() -> Self {
        Self {
            dir: PathBuf::new(),
            lru: Arc::new(Mutex::new(lru::LruCache::new(
                NonZeroUsize::new(1).unwrap(),
            ))),
            max_entries: 0,
        }
    }

    fn hash_key(path: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(path.as_bytes());
        hex::encode(hasher.finalize())
    }

    fn data_path(&self, hash: &str) -> PathBuf {
        self.dir.join(hash)
    }

    fn meta_path(&self, hash: &str) -> PathBuf {
        self.dir.join(format!("{}.meta", hash))
    }

    pub fn get(&self, path: &str) -> Option<(Vec<u8>, String)> {
        if self.max_entries == 0 {
            return None;
        }

        let key = path.to_string();

        if let Ok(mut lru) = self.lru.lock()
            && let Some(data) = lru.get(&key).cloned()
        {
            if let Ok(meta_str) = std::fs::read_to_string(self.meta_path(&Self::hash_key(path)))
                && let Ok(meta) = serde_json::from_str::<Meta>(&meta_str)
            {
                return Some((data, meta.mime));
            }
            return Some((data, "image/jpeg".to_string()));
        }

        let hash = Self::hash_key(path);
        let data_file = self.data_path(&hash);

        if let Ok(data) = std::fs::read(&data_file) {
            let mime = if let Ok(meta_str) = std::fs::read_to_string(self.meta_path(&hash)) {
                serde_json::from_str::<Meta>(&meta_str)
                    .map(|m| m.mime)
                    .unwrap_or_else(|_| "image/jpeg".to_string())
            } else {
                "image/jpeg".to_string()
            };

            if let Ok(mut lru) = self.lru.lock() {
                lru.put(key, data.clone());
            }

            return Some((data, mime));
        }

        None
    }

    pub fn put(&self, path: &str, data: Vec<u8>, mime: &str) {
        if self.max_entries == 0 {
            return;
        }

        let hash = Self::hash_key(path);
        let data_file = self.data_path(&hash);
        let meta_file = self.meta_path(&hash);

        let meta = Meta {
            mime: mime.to_string(),
            timestamp: chrono::Utc::now().timestamp(),
        };

        if let Err(e) = std::fs::write(&data_file, &data) {
            warn!("Failed to write thumbnail cache data: {}", e);
        }
        if let Ok(meta_str) = serde_json::to_string(&meta)
            && let Err(e) = std::fs::write(&meta_file, meta_str)
        {
            warn!("Failed to write thumbnail cache meta: {}", e);
        }

        if let Ok(mut lru) = self.lru.lock() {
            lru.put(path.to_string(), data);
        }

        self.evict_if_needed();
    }

    pub fn invalidate(&self, path: &str) {
        let hash = Self::hash_key(path);

        if let Ok(mut lru) = self.lru.lock() {
            lru.pop(&path.to_string());
        }

        let _ = std::fs::remove_file(self.data_path(&hash));
        let _ = std::fs::remove_file(self.meta_path(&hash));
    }

    pub fn clear(&self) {
        if let Ok(mut lru) = self.lru.lock() {
            lru.clear();
        }

        if self.dir.exists()
            && let Ok(entries) = std::fs::read_dir(&self.dir)
        {
            for entry in entries.flatten() {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }

    fn evict_if_needed(&self) {
        if !self.dir.exists() {
            return;
        }

        let total: u64 = std::fs::read_dir(&self.dir)
            .ok()
            .map(|entries| {
                entries
                    .flatten()
                    .map(|e| e.metadata().map(|m| m.len()).unwrap_or(0))
                    .sum()
            })
            .unwrap_or(0);

        if total > DEFAULT_MAX_BYTES {
            let mut to_remove = Vec::new();
            if let Ok(mut lru) = self.lru.lock() {
                while lru.len() > self.max_entries / 2 {
                    if let Some((key, _)) = lru.pop_lru() {
                        to_remove.push(Self::hash_key(&key));
                    } else {
                        break;
                    }
                }
            }
            for hash in &to_remove {
                let _ = std::fs::remove_file(self.data_path(hash));
                let _ = std::fs::remove_file(self.meta_path(hash));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_key_deterministic() {
        let h1 = ThumbnailCache::hash_key("/photos/cat.jpg");
        let h2 = ThumbnailCache::hash_key("/photos/cat.jpg");
        let h3 = ThumbnailCache::hash_key("/photos/dog.jpg");
        assert_eq!(h1, h2);
        assert_ne!(h1, h3);
        assert_eq!(h1.len(), 64);
    }

    #[tokio::test]
    async fn test_put_get_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let cache = ThumbnailCache::new(dir.path().to_str().unwrap(), 1024 * 1024, 100);

        let data = vec![0xDE, 0xAD, 0xBE, 0xEF];
        cache.put("/test/image.jpg", data.clone(), "image/jpeg");

        let result = cache.get("/test/image.jpg");
        assert!(result.is_some());
        let (cached_data, mime) = result.unwrap();
        assert_eq!(cached_data, data);
        assert_eq!(mime, "image/jpeg");
    }

    #[tokio::test]
    async fn test_get_miss() {
        let dir = tempfile::tempdir().unwrap();
        let cache = ThumbnailCache::new(dir.path().to_str().unwrap(), 1024 * 1024, 100);

        assert!(cache.get("/nonexistent.jpg").is_none());
    }

    #[tokio::test]
    async fn test_invalidate() {
        let dir = tempfile::tempdir().unwrap();
        let cache = ThumbnailCache::new(dir.path().to_str().unwrap(), 1024 * 1024, 100);

        cache.put("/to-delete.jpg", vec![1, 2, 3], "image/png");
        assert!(cache.get("/to-delete.jpg").is_some());

        cache.invalidate("/to-delete.jpg");
        assert!(cache.get("/to-delete.jpg").is_none());
    }

    #[tokio::test]
    async fn test_clear() {
        let dir = tempfile::tempdir().unwrap();
        let cache = ThumbnailCache::new(dir.path().to_str().unwrap(), 1024 * 1024, 100);

        cache.put("/a.jpg", vec![1], "image/jpeg");
        cache.put("/b.jpg", vec![2], "image/jpeg");

        cache.clear();

        assert!(cache.get("/a.jpg").is_none());
        assert!(cache.get("/b.jpg").is_none());
    }

    #[tokio::test]
    async fn test_disk_persistence() {
        let dir = tempfile::tempdir().unwrap();
        let path_str = dir.path().to_str().unwrap().to_string();

        {
            let cache = ThumbnailCache::new(&path_str, 1024 * 1024, 100);
            cache.put("/persist.jpg", vec![42; 100], "image/webp");
        }

        {
            let cache = ThumbnailCache::new(&path_str, 1024 * 1024, 100);
            let result = cache.get("/persist.jpg");
            assert!(result.is_some());
            let (data, mime) = result.unwrap();
            assert_eq!(data, vec![42; 100]);
            assert_eq!(mime, "image/webp");
        }
    }
}
