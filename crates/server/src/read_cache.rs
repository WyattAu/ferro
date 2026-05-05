//! In-memory LRU read cache for hot files.
//!
//! Provides a bounded, TTL-based cache for file content reads. Reduces disk I/O
//! and object store round-trips for frequently accessed files (thumbnails, config,
//! small documents). Uses `DashMap` for lock-free concurrent access.
//!
//! Cache entries are keyed by `(path, etag)` — when a file's ETag changes (due to
//! PUT/DELETE), the stale entry is automatically bypassed on the next read.
//!
//! ## Configuration
//! - `max_entries`: Maximum number of cached files (default: 256)
//! - `max_bytes`: Maximum total cache size in bytes (default: 256 MiB)
//! - `ttl`: Time-to-live for cache entries (default: 5 minutes)
//! - `ttl_jitter`: Random jitter ±30% to prevent thundering herd evictions
//!
//! ## Design
//! LRU eviction is approximate — on each `put()`, if limits are exceeded, we
//! scan entries and evict the oldest-accessed ones. This avoids a separate
//! LRU lock that could deadlock with DashMap's internal sharding.

use bytes::Bytes;
use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;

/// A cached file entry.
#[derive(Clone)]
struct CacheEntry {
    /// The file content bytes.
    data: Bytes,
    /// Size of this entry in bytes.
    size: usize,
    /// Monotonic access counter for LRU ordering.
    access_seq: u64,
    /// When this entry was inserted (for TTL).
    inserted_at: chrono::DateTime<Utc>,
    /// TTL for this specific entry (with variance applied).
    ttl_seconds: i64,
}

/// Statistics about the read cache.
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub total_bytes: usize,
    pub entry_count: usize,
}

impl CacheStats {
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

/// Configuration for the read cache.
#[derive(Debug, Clone)]
pub struct ReadCacheConfig {
    pub max_entries: usize,
    pub max_bytes: usize,
    pub ttl_seconds: i64,
    /// Jitter factor (0.0-0.5). Default 0.3 = ±30%.
    pub ttl_jitter: f64,
}

impl Default for ReadCacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 256,
            max_bytes: 256 * 1024 * 1024, // 256 MiB
            ttl_seconds: 300,             // 5 minutes
            ttl_jitter: 0.3,
        }
    }
}

/// Monotonic counter for LRU ordering. Incremented on every cache access.
struct AccessCounter(std::sync::atomic::AtomicU64);

impl AccessCounter {
    fn next(&self) -> u64 {
        self.0.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }
}

/// An in-memory LRU read cache for file content.
///
/// Thread-safe via `DashMap`. LRU is approximate — uses a monotonic access
/// counter per entry. Eviction scans all entries when limits are exceeded.
pub struct ReadCache {
    entries: DashMap<String, CacheEntry>,
    config: ReadCacheConfig,
    stats: Arc<CacheStatsInner>,
    counter: AccessCounter,
}

#[derive(Default)]
struct CacheStatsInner {
    hits: std::sync::atomic::AtomicU64,
    misses: std::sync::atomic::AtomicU64,
    evictions: std::sync::atomic::AtomicU64,
}

impl ReadCache {
    /// Create a new read cache with the given configuration.
    pub fn new(config: ReadCacheConfig) -> Self {
        Self {
            entries: DashMap::with_capacity(config.max_entries),
            stats: Arc::new(CacheStatsInner::default()),
            counter: AccessCounter(std::sync::atomic::AtomicU64::new(1)),
            config,
        }
    }
}

impl Default for ReadCache {
    fn default() -> Self {
        Self::new(ReadCacheConfig::default())
    }
}

impl ReadCache {
    /// Try to get a file from the cache.
    ///
    /// Returns `None` on miss or if the entry has expired.
    pub fn get(&self, path: &str, etag: &str) -> Option<Bytes> {
        let key = Self::cache_key(path, etag);
        let mut entry = match self.entries.get_mut(&key) {
            Some(e) => e,
            None => {
                self.stats
                    .misses
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                return None;
            }
        };

        // Check TTL
        let now = Utc::now();
        let expires = entry
            .inserted_at
            .checked_add_signed(chrono::Duration::seconds(entry.ttl_seconds))
            .unwrap_or(entry.inserted_at);
        if now > expires {
            // Stale entry — remove and return miss
            drop(entry);
            self.entries.remove(&key);
            self.stats
                .misses
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            return None;
        }

        // Update access sequence (promote in LRU)
        entry.access_seq = self.counter.next();
        let data = entry.data.clone();

        self.stats
            .hits
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Some(data)
    }

    /// Insert a file into the cache.
    ///
    /// If the cache exceeds `max_entries` or `max_bytes`, the least recently
    /// used entries are evicted until the limits are satisfied.
    pub fn put(&self, path: &str, etag: &str, data: Bytes) {
        // Skip caching files larger than 10% of cache capacity
        let max_entry_size = (self.config.max_bytes as f64 * 0.1) as usize;
        if data.len() > max_entry_size {
            return;
        }

        let key = Self::cache_key(path, etag);
        let ttl_variance = (self.config.ttl_seconds as f64 * self.config.ttl_jitter) as i64;
        let jitter = if ttl_variance > 0 {
            use rand::Rng;
            rand::rng().random_range(-ttl_variance..=ttl_variance)
        } else {
            0
        };
        let ttl_seconds = (self.config.ttl_seconds + jitter).max(60);

        let entry = CacheEntry {
            size: data.len(),
            access_seq: self.counter.next(),
            inserted_at: Utc::now(),
            ttl_seconds,
            data,
        };

        self.entries.insert(key, entry);

        // Evict if over limits
        self.evict();
    }

    /// Remove a cached entry (e.g., after a DELETE with known ETag).
    pub fn invalidate(&self, path: &str, etag: &str) {
        let key = Self::cache_key(path, etag);
        self.entries.remove(&key);
    }

    /// Invalidate all entries for a given path (any ETag version).
    pub fn invalidate_path(&self, path: &str) {
        let prefix = format!("{}:", path);
        self.entries.retain(|k, _| !k.starts_with(&prefix));
    }

    /// Clear the entire cache.
    pub fn clear(&self) {
        self.entries.clear();
    }

    /// Get cache statistics.
    pub fn stats(&self) -> CacheStats {
        let mut total_bytes: usize = 0;
        let entry_count = self.entries.len();
        for entry in self.entries.iter() {
            total_bytes += entry.value().size;
        }
        CacheStats {
            hits: self.stats.hits.load(std::sync::atomic::Ordering::Relaxed),
            misses: self.stats.misses.load(std::sync::atomic::Ordering::Relaxed),
            evictions: self
                .stats
                .evictions
                .load(std::sync::atomic::Ordering::Relaxed),
            total_bytes,
            entry_count,
        }
    }

    /// Evict entries until both size and count limits are satisfied.
    ///
    /// Collects all entries, sorts by access_seq (oldest first), then removes
    /// the oldest until within limits. This is O(n log n) but only runs when
    /// limits are exceeded.
    fn evict(&self) {
        let current_count = self.entries.len();
        let current_bytes: usize = self.entries.iter().map(|e| e.value().size).sum();

        if current_bytes <= self.config.max_bytes && current_count <= self.config.max_entries {
            return;
        }

        // Collect (key, access_seq, size) for all entries
        let mut candidates: Vec<(String, u64, usize)> = self
            .entries
            .iter()
            .map(|e| (e.key().clone(), e.value().access_seq, e.value().size))
            .collect();

        // Sort by access_seq ascending (oldest first = LRU)
        candidates.sort_by_key(|(_, seq, _)| *seq);

        let mut freed_bytes: usize = 0;
        let mut removed_count: usize = 0;
        let target_bytes = if current_bytes > self.config.max_bytes {
            Some(current_bytes - self.config.max_bytes)
        } else {
            None
        };
        let target_count = if current_count > self.config.max_entries {
            Some(current_count - self.config.max_entries)
        } else {
            None
        };

        for (key, _, size) in candidates {
            let bytes_met = target_bytes.is_none_or(|t| freed_bytes >= t);
            let count_met = target_count.is_none_or(|t| removed_count >= t);
            if bytes_met && count_met {
                break;
            }
            if self.entries.remove(&key).is_some() {
                freed_bytes += size;
                removed_count += 1;
                self.stats
                    .evictions
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
        }
    }

    fn cache_key(path: &str, etag: &str) -> String {
        format!("{}:{}", path, etag)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_hit_and_miss() {
        let cache = ReadCache::default();
        let data = Bytes::from_static(b"hello world");

        assert!(
            cache.get("/test.txt", "\"abc\"").is_none(),
            "Should miss on empty cache"
        );

        cache.put("/test.txt", "\"abc\"", data.clone());
        let hit = cache.get("/test.txt", "\"abc\"");
        assert!(hit.is_some(), "Should hit after put");
        assert_eq!(hit.unwrap(), data);

        // Different ETag = miss
        assert!(
            cache.get("/test.txt", "\"xyz\"").is_none(),
            "Different ETag should miss"
        );
    }

    #[test]
    fn test_invalidate() {
        let cache = ReadCache::default();
        cache.put("/test.txt", "\"abc\"", Bytes::from_static(b"data"));
        assert!(cache.get("/test.txt", "\"abc\"").is_some());
        cache.invalidate("/test.txt", "\"abc\"");
        assert!(cache.get("/test.txt", "\"abc\"").is_none());
    }

    #[test]
    fn test_invalidate_path() {
        let cache = ReadCache::default();
        cache.put("/f.txt", "\"v1\"", Bytes::from_static(b"v1"));
        cache.put("/f.txt", "\"v2\"", Bytes::from_static(b"v2"));
        assert!(cache.get("/f.txt", "\"v1\"").is_some());
        assert!(cache.get("/f.txt", "\"v2\"").is_some());
        cache.invalidate_path("/f.txt");
        assert!(cache.get("/f.txt", "\"v1\"").is_none());
        assert!(cache.get("/f.txt", "\"v2\"").is_none());
    }

    #[test]
    fn test_eviction_on_max_entries() {
        let config = ReadCacheConfig {
            max_entries: 2,
            max_bytes: 1024 * 1024,
            ttl_seconds: 3600,
            ttl_jitter: 0.0,
        };
        let cache = ReadCache::new(config);
        cache.put("/a.txt", "\"1\"", Bytes::from_static(b"aaa"));
        cache.put("/b.txt", "\"2\"", Bytes::from_static(b"bbb"));
        cache.put("/c.txt", "\"3\"", Bytes::from_static(b"ccc"));

        assert!(
            cache.get("/a.txt", "\"1\"").is_none(),
            "LRU: a should be evicted"
        );
        assert!(
            cache.get("/b.txt", "\"2\"").is_some(),
            "LRU: b should survive"
        );
        assert!(
            cache.get("/c.txt", "\"3\"").is_some(),
            "LRU: c should survive"
        );
    }

    #[test]
    fn test_skip_large_files() {
        let config = ReadCacheConfig {
            max_entries: 100,
            max_bytes: 1000, // 1 KB
            ttl_seconds: 3600,
            ttl_jitter: 0.0,
        };
        let cache = ReadCache::new(config);
        let large = Bytes::from(vec![0u8; 200]); // 200 bytes > 100 bytes (10% of 1KB)
        cache.put("/large.bin", "\"etag\"", large);
        assert!(
            cache.get("/large.bin", "\"etag\"").is_none(),
            "Should skip large files"
        );
    }

    #[test]
    fn test_stats() {
        let cache = ReadCache::default();
        assert_eq!(cache.stats().hits, 0);
        assert_eq!(cache.stats().misses, 0);

        cache.put("/f.txt", "\"e\"", Bytes::from_static(b"data"));
        let _ = cache.get("/f.txt", "\"e\""); // hit
        let _ = cache.get("/f.txt", "\"wrong\""); // miss
        let _ = cache.get("/g.txt", "\"e\""); // miss

        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 2);
        assert!(stats.hit_rate() > 0.0);
        assert!(stats.hit_rate() < 1.0);
    }

    #[test]
    fn test_clear() {
        let cache = ReadCache::default();
        cache.put("/f.txt", "\"e\"", Bytes::from_static(b"data"));
        assert!(cache.get("/f.txt", "\"e\"").is_some());
        cache.clear();
        assert!(cache.get("/f.txt", "\"e\"").is_none());
    }
}
