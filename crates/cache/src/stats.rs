use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub entries: usize,
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub size_bytes: u64,
    pub hit_rate: f64,
}

#[derive(Debug)]
pub(crate) struct StatsTracker {
    hits: AtomicU64,
    misses: AtomicU64,
    evictions: AtomicU64,
    size_bytes: AtomicU64,
}

impl StatsTracker {
    pub fn new() -> Self {
        Self {
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            evictions: AtomicU64::new(0),
            size_bytes: AtomicU64::new(0),
        }
    }

    pub fn record_hit(&self) {
        self.hits.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_miss(&self) {
        self.misses.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_eviction(&self) {
        self.evictions.fetch_add(1, Ordering::Relaxed);
    }

    pub fn add_size(&self, bytes: u64) {
        self.size_bytes.fetch_add(bytes, Ordering::Relaxed);
    }

    pub fn sub_size(&self, bytes: u64) {
        let mut current = self.size_bytes.load(Ordering::Relaxed);
        loop {
            let new = current.saturating_sub(bytes);
            match self
                .size_bytes
                .compare_exchange_weak(current, new, Ordering::Relaxed, Ordering::Relaxed)
            {
                Ok(_) => break,
                Err(actual) => current = actual,
            }
        }
    }

    pub fn snapshot(&self, entries: usize) -> CacheStats {
        let hits = self.hits.load(Ordering::Relaxed);
        let misses = self.misses.load(Ordering::Relaxed);
        let total = hits + misses;
        CacheStats {
            entries,
            hits,
            misses,
            evictions: self.evictions.load(Ordering::Relaxed),
            size_bytes: self.size_bytes.load(Ordering::Relaxed),
            hit_rate: if total > 0 { hits as f64 / total as f64 } else { 0.0 },
        }
    }
}

impl Default for StatsTracker {
    fn default() -> Self {
        Self::new()
    }
}
