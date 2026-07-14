use dashmap::DashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

const MAX_CACHE_ENTRIES: usize = 100_000;
const DEFAULT_TTL: Duration = Duration::from_secs(3600);

#[derive(Debug, Clone)]
pub struct IdempotentResponse {
    pub status: u16,
    pub body: bytes::Bytes,
    pub content_type: String,
    pub created_at: Instant,
}

#[derive(Debug)]
pub struct IdempotencyStore {
    cache: Arc<DashMap<String, IdempotentResponse>>,
    ttl: Duration,
}

impl IdempotencyStore {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(DashMap::new()),
            ttl: DEFAULT_TTL,
        }
    }

    pub fn with_ttl(ttl: Duration) -> Self {
        Self {
            cache: Arc::new(DashMap::new()),
            ttl,
        }
    }

    pub fn get(&self, key: &str) -> Option<IdempotentResponse> {
        let entry = self.cache.get(key)?;
        if entry.created_at.elapsed() > self.ttl {
            drop(entry);
            self.cache.remove(key);
            return None;
        }
        Some(entry.value().clone())
    }

    pub fn store(&self, key: &str, response: IdempotentResponse) {
        if self.cache.len() >= MAX_CACHE_ENTRIES {
            self.cache.retain(|_, v| v.created_at.elapsed() <= self.ttl);
            if self.cache.len() >= MAX_CACHE_ENTRIES {
                let mut removed = 0usize;
                let to_remove = self.cache.len() - MAX_CACHE_ENTRIES + 1000;
                self.cache.retain(|_, _| {
                    removed += 1;
                    removed <= to_remove
                });
            }
        }
        self.cache.insert(key.to_string(), response);
    }

    pub fn len(&self) -> usize {
        self.cache.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }
}

impl Default for IdempotencyStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;

    #[test]
    fn test_store_and_get() {
        let store = IdempotencyStore::new();
        let key = "req-123";

        assert!(store.get(key).is_none());

        store.store(
            key,
            IdempotentResponse {
                status: 200,
                body: Bytes::from_static(b"ok"),
                content_type: "application/json".to_string(),
                created_at: Instant::now(),
            },
        );

        let resp = store.get(key).unwrap();
        assert_eq!(resp.status, 200);
        assert_eq!(resp.body, Bytes::from_static(b"ok"));
    }

    #[test]
    fn test_expired_entry_removed() {
        let store = IdempotencyStore::with_ttl(Duration::from_millis(10));
        store.store(
            "key1",
            IdempotentResponse {
                status: 200,
                body: Bytes::from_static(b"ok"),
                content_type: "text/plain".to_string(),
                created_at: Instant::now(),
            },
        );

        std::thread::sleep(Duration::from_millis(50));
        assert!(store.get("key1").is_none());
    }

    #[test]
    fn test_cache_bounded() {
        let store = IdempotencyStore::new();
        for i in 0..200_000 {
            store.store(
                &format!("key-{}", i),
                IdempotentResponse {
                    status: 200,
                    body: Bytes::from_static(b"x"),
                    content_type: "text/plain".to_string(),
                    created_at: Instant::now(),
                },
            );
        }
        assert!(store.len() <= MAX_CACHE_ENTRIES + 1000);
    }

    #[test]
    fn test_different_keys_independent() {
        let store = IdempotencyStore::new();
        store.store(
            "key-a",
            IdempotentResponse {
                status: 200,
                body: Bytes::from_static(b"a"),
                content_type: "text/plain".to_string(),
                created_at: Instant::now(),
            },
        );
        store.store(
            "key-b",
            IdempotentResponse {
                status: 201,
                body: Bytes::from_static(b"b"),
                content_type: "text/plain".to_string(),
                created_at: Instant::now(),
            },
        );

        assert_eq!(store.get("key-a").unwrap().status, 200);
        assert_eq!(store.get("key-b").unwrap().status, 201);
    }

    #[test]
    fn test_is_empty() {
        let store = IdempotencyStore::new();
        assert!(store.is_empty());
        store.store(
            "k",
            IdempotentResponse {
                status: 200,
                body: Bytes::from_static(b"x"),
                content_type: "text/plain".to_string(),
                created_at: Instant::now(),
            },
        );
        assert!(!store.is_empty());
    }
}
