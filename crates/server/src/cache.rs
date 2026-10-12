use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::RwLock;

/// Cache entry
struct CacheEntry<T> {
    value: T,
    expires_at: DateTime<Utc>,
    /// Metadata epoch the entry was written under (DC-CACHE-001,
    /// YP-CONSISTENCY-CACHE-001). A hit is valid only while
    /// `entry.epoch == cache.current_epoch()`; bumping the epoch invalidates
    /// every entry at once with zero staleness — TTL is a latency
    /// optimization, never the coherence mechanism (THM-AUTHZ-COHERENCE).
    epoch: u64,
}

/// In-memory cache with TTL and epoch-tagged coherence
pub struct Cache<T> {
    entries: Arc<RwLock<HashMap<String, CacheEntry<T>>>>,
    default_ttl: Duration,
    max_size: usize,
    epoch: Arc<AtomicU64>,
}

impl<T: Clone> Cache<T> {
    pub fn new(default_ttl: Duration, max_size: usize) -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            default_ttl,
            max_size,
            epoch: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Invalidate every cached entry immediately (DC-CACHE-001). Call in the
    /// same logical transaction as any permission/metadata mutation so a
    /// revocation is visible to all readers the moment it commits.
    pub fn bump_epoch(&self) -> u64 {
        self.epoch.fetch_add(1, Ordering::SeqCst) + 1
    }

    pub fn current_epoch(&self) -> u64 {
        self.epoch.load(Ordering::SeqCst)
    }

    /// Get a value from cache. Returns None for expired entries AND for
    /// entries written under a superseded epoch.
    pub async fn get(&self, key: &str) -> Option<T> {
        let entries = self.entries.read().await;
        if let Some(entry) = entries.get(key)
            && Utc::now() < entry.expires_at
            && entry.epoch == self.current_epoch()
        {
            return Some(entry.value.clone());
        }
        None
    }

    /// Set a value in cache
    pub async fn set(&self, key: String, value: T, ttl: Option<Duration>) {
        let mut entries = self.entries.write().await;

        // Evict if at capacity
        if entries.len() >= self.max_size {
            self.evict_expired(&mut entries).await;

            if entries.len() >= self.max_size {
                // Still at capacity, evict oldest
                if let Some(oldest_key) = entries.keys().next().cloned() {
                    entries.remove(&oldest_key);
                }
            }
        }

        let expires_at = Utc::now() + ttl.unwrap_or(self.default_ttl);
        let epoch = self.current_epoch();
        entries.insert(
            key,
            CacheEntry {
                value,
                expires_at,
                epoch,
            },
        );
    }

    /// Remove a value from cache
    pub async fn remove(&self, key: &str) -> bool {
        let mut entries = self.entries.write().await;
        entries.remove(key).is_some()
    }

    /// Clear cache
    pub async fn clear(&self) {
        let mut entries = self.entries.write().await;
        entries.clear();
    }

    /// Get cache stats
    pub async fn stats(&self) -> CacheStats {
        let entries = self.entries.read().await;
        let now = Utc::now();

        let total = entries.len();
        let expired = entries.values().filter(|e| now >= e.expires_at).count();
        let active = total - expired;

        CacheStats {
            total,
            active,
            expired,
            max_size: self.max_size,
        }
    }

    /// Evict expired entries
    async fn evict_expired(&self, entries: &mut HashMap<String, CacheEntry<T>>) {
        let now = Utc::now();
        entries.retain(|_, entry| now < entry.expires_at);
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total: usize,
    pub active: usize,
    pub expired: usize,
    pub max_size: usize,
}

impl CacheStats {
    pub fn hit_rate(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            self.active as f64 / self.total as f64
        }
    }
}

/// Global cache instance
pub struct GlobalCache {
    pub user_cache: Arc<Cache<String>>,
    pub calendar_cache: Arc<Cache<String>>,
    pub event_cache: Arc<Cache<String>>,
}

impl GlobalCache {
    pub fn new() -> Self {
        Self {
            user_cache: Arc::new(Cache::new(Duration::minutes(5), 1000)),
            calendar_cache: Arc::new(Cache::new(Duration::minutes(10), 10000)),
            event_cache: Arc::new(Cache::new(Duration::minutes(5), 100000)),
        }
    }
}

impl Default for GlobalCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_set_get() {
        let cache = Cache::new(Duration::seconds(10), 100);

        cache.set("key1".to_string(), "value1".to_string(), None).await;

        let value = cache.get("key1").await;
        assert_eq!(value, Some("value1".to_string()));
    }

    #[tokio::test]
    async fn test_cache_expiry() {
        let cache = Cache::new(Duration::milliseconds(10), 100);

        cache.set("key1".to_string(), "value1".to_string(), None).await;

        // Wait for expiry
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;

        let value = cache.get("key1").await;
        assert_eq!(value, None);
    }

    #[tokio::test]
    async fn test_cache_eviction() {
        let cache = Cache::new(Duration::seconds(10), 2);

        cache.set("key1".to_string(), "value1".to_string(), None).await;
        cache.set("key2".to_string(), "value2".to_string(), None).await;
        cache.set("key3".to_string(), "value3".to_string(), None).await;

        let stats = cache.stats().await;
        assert_eq!(stats.total, 2);
    }

    #[tokio::test]
    async fn test_cache_stats() {
        let cache = Cache::new(Duration::seconds(10), 100);

        cache.set("key1".to_string(), "value1".to_string(), None).await;

        let stats = cache.stats().await;
        assert_eq!(stats.total, 1);
        assert_eq!(stats.active, 1);
    }

    /// TV-CACHE-001 (THM-AUTHZ-COHERENCE, DC-CACHE-001): bumping the epoch
    /// invalidates cached entries IMMEDIATELY — a revocation is visible to
    /// every reader the moment it commits, regardless of remaining TTL.
    #[tokio::test]
    async fn test_epoch_bump_invalidates_cached_entries_at_any_ttl() {
        // Generous TTL: the old behaviour would serve this entry for 1 hour.
        let cache = Cache::new(Duration::hours(1), 100);

        // "Allow" decision cached.
        cache
            .set("authz:alice:/docs/x".to_string(), "allow".to_string(), None)
            .await;
        assert_eq!(cache.get("authz:alice:/docs/x").await, Some("allow".to_string()));

        // Revocation bumps the epoch (same logical transaction as the
        // permission mutation).
        cache.bump_epoch();

        // The cached decision must be gone — at ANY remaining TTL.
        assert_eq!(
            cache.get("authz:alice:/docs/x").await,
            None,
            "stale authz served after revocation"
        );

        // Fresh decision written under the new epoch is visible again.
        cache
            .set("authz:alice:/docs/x".to_string(), "deny".to_string(), None)
            .await;
        assert_eq!(cache.get("authz:alice:/docs/x").await, Some("deny".to_string()));
    }

    /// Epoch is monotone; multiple bumps keep invalidating.
    #[tokio::test]
    async fn test_epoch_monotone() {
        let cache = Cache::new(Duration::seconds(60), 100);
        assert_eq!(cache.current_epoch(), 0);
        cache.set("k".to_string(), "v".to_string(), None).await;
        assert_eq!(cache.bump_epoch(), 1);
        assert_eq!(cache.get("k").await, None);
        cache.set("k".to_string(), "v2".to_string(), None).await;
        assert_eq!(cache.bump_epoch(), 2);
        assert_eq!(cache.get("k").await, None);
        cache.set("k".to_string(), "v3".to_string(), None).await;
        assert_eq!(cache.get("k").await, Some("v3".to_string()));
    }
}
