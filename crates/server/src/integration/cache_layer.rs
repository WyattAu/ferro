use std::time::Duration;

use ferro_cache::TimedCache;

pub fn create_metadata_cache(max_entries: usize, _ttl: Duration) -> TimedCache<String, Vec<u8>> {
    TimedCache::new(Some(max_entries), None)
}

pub fn cache_key(method: &str, path: &str, query: &str) -> String {
    format!("{}:{}:{}", method, path, query)
}

#[cfg(test)]
mod tests {
    use ferro_cache::CacheStore;

    use super::*;

    #[test]
    fn test_cache_key_format() {
        assert_eq!(cache_key("GET", "/api/files", ""), "GET:/api/files:");
        assert_eq!(
            cache_key("POST", "/upload", "chunk=1"),
            "POST:/upload:chunk=1"
        );
    }

    #[test]
    fn test_metadata_cache_create() {
        let cache = create_metadata_cache(100, Duration::from_secs(60));
        assert!(cache.is_empty());
        cache.set(
            "key1".to_string(),
            vec![1, 2, 3],
            Some(Duration::from_secs(60)),
        );
        assert_eq!(cache.len(), 1);
        let val = cache.get(&"key1".to_string());
        assert_eq!(val, Some(vec![1, 2, 3]));
    }
}
