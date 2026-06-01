use crate::error::CacheError;
use crate::lru::LruEvictionPolicy;
use crate::stats::{CacheStats, StatsTracker};
use dashmap::DashMap;
use std::hash::Hash;
use std::time::{Duration, Instant};

pub struct CacheEntry<V: Clone> {
    pub value: V,
    pub created_at: Instant,
    pub expires_at: Option<Instant>,
    pub last_accessed: Instant,
    pub access_count: u64,
    pub size_bytes: u64,
}

impl<V: Clone> CacheEntry<V> {
    fn new(value: V, ttl: Option<Duration>, size_bytes: u64) -> Self {
        let now = Instant::now();
        Self {
            value,
            created_at: now,
            expires_at: ttl.map(|d| now + d),
            last_accessed: now,
            access_count: 0,
            size_bytes,
        }
    }

    fn is_expired(&self) -> bool {
        self.expires_at.is_some_and(|exp| Instant::now() >= exp)
    }

    fn touch(&mut self) {
        self.last_accessed = Instant::now();
        self.access_count += 1;
    }
}

pub trait CacheStore<K, V>: Send + Sync
where
    K: Hash + Eq + Clone + Send + Sync,
    V: Clone + Send + Sync,
{
    fn get(&self, key: &K) -> Option<V>;
    fn set(&self, key: K, value: V, ttl: Option<Duration>);
    fn remove(&self, key: &K) -> Option<V>;
    fn clear(&self);
    fn contains_key(&self, key: &K) -> bool;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool;
    fn stats(&self) -> CacheStats;
}

pub struct TimedCache<K, V>
where
    K: Hash + Eq + Clone + Send + Sync,
    V: Clone + Send + Sync,
{
    pub(crate) entries: DashMap<K, CacheEntry<V>>,
    max_entries: Option<usize>,
    max_size_bytes: Option<u64>,
    lru: Option<LruEvictionPolicy<K>>,
    stats: StatsTracker,
}

impl<K, V> TimedCache<K, V>
where
    K: Hash + Eq + Clone + Send + Sync,
    V: Clone + Send + Sync,
{
    pub fn new(max_entries: Option<usize>, max_size_bytes: Option<u64>) -> Self {
        let lru = max_entries.map(LruEvictionPolicy::new);
        Self {
            entries: DashMap::new(),
            max_entries,
            max_size_bytes,
            lru,
            stats: StatsTracker::new(),
        }
    }

    fn evict_lru_entries(&self) {
        let Some(ref lru) = self.lru else {
            return;
        };
        while lru.should_evict(self.entries.len()) {
            if let Some(key) = lru.evict_lru() {
                if let Some((_, entry)) = self.entries.remove(&key) {
                    self.stats.sub_size(entry.size_bytes);
                    self.stats.record_eviction();
                }
            } else {
                break;
            }
        }
    }

    fn insert_entry(&self, key: K, value: V, ttl: Option<Duration>, size_bytes: u64) {
        if let Some(ref lru) = self.lru {
            lru.record_insert(&key);
        }
        if let Some(existing) = self.entries.get(&key) {
            self.stats.sub_size(existing.size_bytes);
        }
        self.stats.add_size(size_bytes);
        self.entries
            .insert(key, CacheEntry::new(value, ttl, size_bytes));
    }

    pub fn set_with_size(
        &self,
        key: K,
        value: V,
        ttl: Option<Duration>,
        size_bytes: u64,
    ) -> Result<(), CacheError> {
        if let Some(max_entries) = self.max_entries
            && self.entries.len() >= max_entries
            && !self.entries.contains_key(&key)
        {
            return Err(CacheError::CapacityExceeded {
                entries: self.entries.len(),
                max_entries,
            });
        }

        if let Some(max_size) = self.max_size_bytes {
            let current_size = self.stats.snapshot(self.entries.len()).size_bytes;
            let existing_size = self.entries.get(&key).map(|e| e.size_bytes).unwrap_or(0);
            if current_size.saturating_sub(existing_size) + size_bytes > max_size {
                return Err(CacheError::SizeExceeded {
                    size_bytes: current_size.saturating_sub(existing_size) + size_bytes,
                    max_size_bytes: max_size,
                });
            }
        }

        self.insert_entry(key, value, ttl, size_bytes);
        Ok(())
    }

    pub fn cleanup_expired(&self) -> usize {
        let mut expired_keys = Vec::new();
        for entry in self.entries.iter() {
            if entry.is_expired() {
                expired_keys.push(entry.key().clone());
            }
        }
        let count = expired_keys.len();
        for key in expired_keys {
            if let Some((_, entry)) = self.entries.remove(&key) {
                self.stats.sub_size(entry.size_bytes);
                if let Some(ref lru) = self.lru {
                    lru.record_remove(&key);
                }
            }
        }
        count
    }
}

impl<K, V> CacheStore<K, V> for TimedCache<K, V>
where
    K: Hash + Eq + Clone + Send + Sync,
    V: Clone + Send + Sync,
{
    fn get(&self, key: &K) -> Option<V> {
        let mut entry = match self.entries.get_mut(key) {
            Some(e) => e,
            None => {
                self.stats.record_miss();
                return None;
            }
        };
        if entry.is_expired() {
            drop(entry);
            if let Some((_, removed)) = self.entries.remove(key) {
                self.stats.sub_size(removed.size_bytes);
                if let Some(ref lru) = self.lru {
                    lru.record_remove(key);
                }
            }
            self.stats.record_miss();
            return None;
        }
        entry.touch();
        if let Some(ref lru) = self.lru {
            lru.record_access(key);
        }
        self.stats.record_hit();
        Some(entry.value.clone())
    }

    fn set(&self, key: K, value: V, ttl: Option<Duration>) {
        let is_new = !self.entries.contains_key(&key);
        if is_new {
            self.evict_lru_entries();
        }
        if is_new
            && let Some(max_entries) = self.max_entries
            && self.entries.len() >= max_entries
        {
            return;
        }
        self.insert_entry(key, value, ttl, 0);
    }

    fn remove(&self, key: &K) -> Option<V> {
        let entry = self.entries.remove(key)?;
        self.stats.sub_size(entry.1.size_bytes);
        if let Some(ref lru) = self.lru {
            lru.record_remove(key);
        }
        Some(entry.1.value)
    }

    fn clear(&self) {
        self.entries.clear();
        if let Some(ref lru) = self.lru {
            lru.record_clear();
        }
        self.stats.sub_size(self.stats.snapshot(0).size_bytes);
    }

    fn contains_key(&self, key: &K) -> bool {
        self.entries
            .get(key)
            .is_some_and(|entry| !entry.is_expired())
    }

    fn len(&self) -> usize {
        self.entries.len()
    }

    fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn stats(&self) -> CacheStats {
        self.stats.snapshot(self.entries.len())
    }
}
