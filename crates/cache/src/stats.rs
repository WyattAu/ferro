use parking_lot::Mutex;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub entries: usize,
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub size_bytes: u64,
    pub hit_rate: f64,
}

#[derive(Debug, Default)]
pub(crate) struct StatsInner {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub size_bytes: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct StatsTracker {
    inner: Arc<Mutex<StatsInner>>,
}

impl StatsTracker {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(StatsInner::default())),
        }
    }

    pub fn record_hit(&self) {
        self.inner.lock().hits += 1;
    }

    pub fn record_miss(&self) {
        self.inner.lock().misses += 1;
    }

    pub fn record_eviction(&self) {
        self.inner.lock().evictions += 1;
    }

    pub fn add_size(&self, bytes: u64) {
        self.inner.lock().size_bytes += bytes;
    }

    pub fn sub_size(&self, bytes: u64) {
        let new_size = self.inner.lock().size_bytes.saturating_sub(bytes);
        self.inner.lock().size_bytes = new_size;
    }

    pub fn snapshot(&self, entries: usize) -> CacheStats {
        let inner = self.inner.lock();
        let total = inner.hits + inner.misses;
        CacheStats {
            entries,
            hits: inner.hits,
            misses: inner.misses,
            evictions: inner.evictions,
            size_bytes: inner.size_bytes,
            hit_rate: if total > 0 {
                inner.hits as f64 / total as f64
            } else {
                0.0
            },
        }
    }
}

impl Default for StatsTracker {
    fn default() -> Self {
        Self::new()
    }
}
