use std::collections::HashMap;
use std::hash::Hash;

pub struct LruEvictionPolicy<K: Hash + Eq> {
    access_order: parking_lot::Mutex<Vec<K>>,
    index: parking_lot::Mutex<HashMap<K, usize>>,
    max_entries: usize,
}

impl<K: Hash + Eq + Clone> LruEvictionPolicy<K> {
    pub fn new(max_entries: usize) -> Self {
        Self {
            access_order: parking_lot::Mutex::new(Vec::with_capacity(max_entries)),
            index: parking_lot::Mutex::new(HashMap::new()),
            max_entries,
        }
    }

    pub fn record_access(&self, key: &K) {
        let mut order = self.access_order.lock();
        let mut index = self.index.lock();
        if let Some(&pos) = index.get(key) {
            order.remove(pos);
            for v in index.values_mut() {
                if *v > pos {
                    *v -= 1;
                }
            }
        }
        index.insert(key.clone(), order.len());
        order.push(key.clone());
    }

    pub fn record_insert(&self, key: &K) {
        self.record_access(key);
    }

    pub fn record_remove(&self, key: &K) {
        let mut order = self.access_order.lock();
        let mut index = self.index.lock();
        if let Some(pos) = index.remove(key) {
            order.remove(pos);
            for v in index.values_mut() {
                if *v > pos {
                    *v -= 1;
                }
            }
        }
    }

    pub fn record_clear(&self) {
        self.access_order.lock().clear();
        self.index.lock().clear();
    }

    pub fn should_evict(&self, current_entries: usize) -> bool {
        current_entries >= self.max_entries
    }

    pub fn evict_lru(&self) -> Option<K> {
        let mut order = self.access_order.lock();
        let mut index = self.index.lock();
        if order.is_empty() {
            return None;
        }
        let lru_key = order.remove(0);
        index.remove(&lru_key);
        for v in index.values_mut() {
            *v = v.saturating_sub(1);
        }
        Some(lru_key)
    }
}
