//! Local content cache for offline file access.
//!
//! Stores file content locally using SHA-256 keyed storage,
//! enabling read access while offline.

use sha2::{Digest, Sha256};
use std::collections::HashMap;
use tracing::warn;

/// Cache entry for a locally stored file.
#[derive(Debug, Clone)]
pub struct CacheEntry {
    /// SHA-256 content hash.
    pub content_hash: String,
    /// File content bytes.
    pub data: Vec<u8>,
    /// Size in bytes.
    pub size: u64,
    /// Path the content was cached for.
    pub path: String,
    /// When the entry was cached.
    pub cached_at: chrono::DateTime<chrono::Utc>,
    /// Last access time.
    pub last_accessed: chrono::DateTime<chrono::Utc>,
}

/// In-memory content cache with optional SQLite backing.
pub struct ContentCache {
    entries: HashMap<String, CacheEntry>,
    /// Total bytes cached.
    total_size: u64,
    /// Maximum cache size in bytes (0 = unlimited).
    max_size: u64,
}

impl ContentCache {
    /// Create a new content cache with a size limit.
    pub fn new(max_size: u64) -> Self {
        Self {
            entries: HashMap::new(),
            total_size: 0,
            max_size,
        }
    }

    /// Create an unlimited content cache.
    pub fn unlimited() -> Self {
        Self {
            entries: HashMap::new(),
            total_size: 0,
            max_size: 0,
        }
    }

    /// Compute SHA-256 hash of content.
    pub fn hash_content(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hex::encode(hasher.finalize())
    }

    /// Store content in the cache, keyed by path.
    ///
    /// If the cache is full, evicts least-recently-accessed entries
    /// until there is enough room.
    pub fn put(&mut self, path: &str, data: Vec<u8>) {
        let content_hash = Self::hash_content(&data);
        let size = data.len() as u64;

        // Remove existing entry if present
        if let Some(old) = self.entries.remove(path) {
            self.total_size = self.total_size.saturating_sub(old.size);
        }

        // Evict if needed
        while self.max_size > 0
            && self.total_size + size > self.max_size
            && !self.entries.is_empty()
        {
            self.evict_lru();
        }

        if self.max_size > 0 && self.total_size + size > self.max_size {
            warn!(
                "Content cache full, skipping put for {} ({} bytes)",
                path, size
            );
            return;
        }

        let now = chrono::Utc::now();
        self.entries.insert(
            path.to_string(),
            CacheEntry {
                content_hash,
                data,
                size,
                path: path.to_string(),
                cached_at: now,
                last_accessed: now,
            },
        );
        self.total_size += size;
    }

    /// Retrieve content from the cache by path.
    pub fn get(&mut self, path: &str) -> Option<Vec<u8>> {
        let entry = self.entries.get_mut(path)?;
        entry.last_accessed = chrono::Utc::now();
        Some(entry.data.clone())
    }

    /// Check if a path is cached.
    pub fn contains(&self, path: &str) -> bool {
        self.entries.contains_key(path)
    }

    /// Get the content hash for a cached path.
    pub fn content_hash(&self, path: &str) -> Option<String> {
        self.entries.get(path).map(|e| e.content_hash.clone())
    }

    /// Remove a specific path from the cache.
    pub fn remove(&mut self, path: &str) -> Option<CacheEntry> {
        let entry = self.entries.remove(path)?;
        self.total_size = self.total_size.saturating_sub(entry.size);
        Some(entry)
    }

    /// Clear all cached entries.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.total_size = 0;
    }

    /// Get the number of cached entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get the total size of cached content in bytes.
    pub fn total_size(&self) -> u64 {
        self.total_size
    }

    /// List all cached paths.
    pub fn paths(&self) -> Vec<String> {
        self.entries.keys().cloned().collect()
    }

    /// Evict the least-recently-accessed entry.
    fn evict_lru(&mut self) {
        if let Some(lru_key) = self
            .entries
            .iter()
            .min_by_key(|(_, e)| e.last_accessed)
            .map(|(k, _)| k.clone())
            && let Some(removed) = self.entries.remove(&lru_key)
        {
            self.total_size = self.total_size.saturating_sub(removed.size);
        }
    }
}

impl Default for ContentCache {
    fn default() -> Self {
        Self::unlimited()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_put_and_get() {
        let mut cache = ContentCache::unlimited();
        cache.put("/file.txt", b"hello".to_vec());
        assert_eq!(cache.get("/file.txt"), Some(b"hello".to_vec()));
    }

    #[test]
    fn test_get_missing() {
        let mut cache = ContentCache::unlimited();
        assert!(cache.get("/missing").is_none());
    }

    #[test]
    fn test_contains() {
        let mut cache = ContentCache::unlimited();
        cache.put("/file.txt", b"data".to_vec());
        assert!(cache.contains("/file.txt"));
        assert!(!cache.contains("/other"));
    }

    #[test]
    fn test_content_hash() {
        let mut cache = ContentCache::unlimited();
        cache.put("/file.txt", b"hello".to_vec());
        assert_eq!(
            cache.content_hash("/file.txt"),
            Some(ContentCache::hash_content(b"hello"))
        );
    }

    #[test]
    fn test_remove() {
        let mut cache = ContentCache::unlimited();
        cache.put("/file.txt", b"data".to_vec());
        cache.remove("/file.txt");
        assert!(!cache.contains("/file.txt"));
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_clear() {
        let mut cache = ContentCache::unlimited();
        cache.put("/a", vec![1]);
        cache.put("/b", vec![2]);
        cache.put("/c", vec![3]);
        cache.clear();
        assert!(cache.is_empty());
        assert_eq!(cache.total_size(), 0);
    }

    #[test]
    fn test_max_size_eviction() {
        let mut cache = ContentCache::new(150);
        cache.put("/a", vec![0u8; 100]); // total 100
        cache.put("/b", vec![0u8; 100]); // needs 200 total, evict /a first → total 100+100=200 OK? No, max is 150
        // After put /b: total 100 + 100 = 200 > 150. Evict LRU (/a) → total 0 + 100 = 100. OK.
        assert!(cache.contains("/b"));
        assert!(!cache.contains("/a"));
    }

    #[test]
    fn test_eviction_order() {
        let mut cache = ContentCache::new(1000);
        cache.put("/a", vec![0u8; 400]);
        cache.put("/b", vec![0u8; 400]);
        // Access /a to make it recently used
        let _ = cache.get("/a");
        cache.put("/c", vec![0u8; 400]); // needs 1200 > 1000, evict /b (LRU)

        assert!(cache.contains("/a"));
        assert!(cache.contains("/c"));
        assert!(!cache.contains("/b"));
    }

    #[test]
    fn test_update_existing() {
        let mut cache = ContentCache::new(1000);
        cache.put("/f", vec![1, 2, 3]);
        assert_eq!(cache.total_size(), 3);
        cache.put("/f", vec![1, 2, 3, 4, 5]);
        assert_eq!(cache.total_size(), 5);
    }

    #[test]
    fn test_paths() {
        let mut cache = ContentCache::unlimited();
        cache.put("/a", vec![]);
        cache.put("/b", vec![]);
        let mut paths = cache.paths();
        paths.sort();
        assert_eq!(paths, vec!["/a", "/b"]);
    }

    #[test]
    fn test_hash_content_deterministic() {
        let h1 = ContentCache::hash_content(b"test");
        let h2 = ContentCache::hash_content(b"test");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }

    #[test]
    fn test_cache_full_skips_put() {
        let mut cache = ContentCache::new(10);
        cache.put("/big", vec![0u8; 100]); // too big for cache
        assert!(!cache.contains("/big"));
    }
}
