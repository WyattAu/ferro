use super::*;
use std::sync::Arc;
use std::time::Duration;

fn make_cache() -> TimedCache<String, String> {
    TimedCache::new(None, None)
}

fn make_lru_cache(max: usize) -> TimedCache<String, String> {
    TimedCache::new(Some(max), None)
}

fn make_size_limited_cache(max_bytes: u64) -> TimedCache<String, String> {
    TimedCache::new(None, Some(max_bytes))
}

#[test]
fn test_set_get() {
    let cache = make_cache();
    cache.set("key".to_string(), "value".to_string(), None);
    assert_eq!(cache.get(&"key".to_string()), Some("value".to_string()));
}

#[test]
fn test_get_nonexistent() {
    let cache = make_cache();
    assert_eq!(cache.get(&"missing".to_string()), None);
}

#[test]
fn test_remove() {
    let cache = make_cache();
    cache.set("key".to_string(), "value".to_string(), None);
    let removed = cache.remove(&"key".to_string());
    assert_eq!(removed, Some("value".to_string()));
    assert_eq!(cache.get(&"key".to_string()), None);
}

#[test]
fn test_remove_nonexistent() {
    let cache = make_cache();
    assert_eq!(cache.remove(&"missing".to_string()), None);
}

#[test]
fn test_clear() {
    let cache = make_cache();
    cache.set("a".to_string(), "1".to_string(), None);
    cache.set("b".to_string(), "2".to_string(), None);
    cache.clear();
    assert!(cache.is_empty());
}

#[test]
fn test_contains_key() {
    let cache = make_cache();
    cache.set("key".to_string(), "value".to_string(), None);
    assert!(cache.contains_key(&"key".to_string()));
    assert!(!cache.contains_key(&"missing".to_string()));
}

#[test]
fn test_len_and_is_empty() {
    let cache = make_cache();
    assert!(cache.is_empty());
    assert_eq!(cache.len(), 0);
    cache.set("a".to_string(), "1".to_string(), None);
    assert_eq!(cache.len(), 1);
    assert!(!cache.is_empty());
}

#[test]
fn test_ttl_expiry() {
    let cache = make_cache();
    cache.set("key".to_string(), "value".to_string(), Some(Duration::from_micros(100)));
    std::thread::sleep(Duration::from_millis(5));
    assert_eq!(cache.get(&"key".to_string()), None);
}

#[test]
fn test_zero_ttl_expires_immediately() {
    let cache = make_cache();
    cache.set("key".to_string(), "value".to_string(), Some(Duration::ZERO));
    std::thread::sleep(Duration::from_millis(1));
    assert_eq!(cache.get(&"key".to_string()), None);
}

#[test]
fn test_no_ttl_never_expires() {
    let cache = make_cache();
    cache.set("key".to_string(), "value".to_string(), None);
    std::thread::sleep(Duration::from_millis(1));
    assert_eq!(cache.get(&"key".to_string()), Some("value".to_string()));
}

#[test]
fn test_lru_eviction_on_insert() {
    let cache = make_lru_cache(3);
    cache.set("a".to_string(), "1".to_string(), None);
    cache.set("b".to_string(), "2".to_string(), None);
    cache.set("c".to_string(), "3".to_string(), None);
    cache.set("d".to_string(), "4".to_string(), None);
    assert_eq!(cache.len(), 3);
    assert_eq!(cache.get(&"a".to_string()), None);
    assert_eq!(cache.get(&"d".to_string()), Some("4".to_string()));
}

#[test]
fn test_lru_eviction_access_updates_order() {
    let cache = make_lru_cache(3);
    cache.set("a".to_string(), "1".to_string(), None);
    cache.set("b".to_string(), "2".to_string(), None);
    cache.set("c".to_string(), "3".to_string(), None);
    let _ = cache.get(&"a".to_string());
    cache.set("d".to_string(), "4".to_string(), None);
    assert_eq!(cache.get(&"b".to_string()), None);
    assert_eq!(cache.get(&"a".to_string()), Some("1".to_string()));
    assert_eq!(cache.get(&"d".to_string()), Some("4".to_string()));
}

#[test]
fn test_zero_max_entries_allows_insert() {
    let cache = make_lru_cache(0);
    cache.set("a".to_string(), "1".to_string(), None);
    assert!(cache.is_empty());
}

#[test]
fn test_stats_hits_and_misses() {
    let cache = make_cache();
    cache.set("key".to_string(), "value".to_string(), None);
    let _ = cache.get(&"key".to_string());
    let _ = cache.get(&"key".to_string());
    let _ = cache.get(&"missing".to_string());
    let stats = cache.stats();
    assert_eq!(stats.hits, 2);
    assert_eq!(stats.misses, 1);
    assert!((stats.hit_rate - 2.0 / 3.0).abs() < 1e-10);
}

#[test]
fn test_stats_evictions() {
    let cache = make_lru_cache(2);
    cache.set("a".to_string(), "1".to_string(), None);
    cache.set("b".to_string(), "2".to_string(), None);
    cache.set("c".to_string(), "3".to_string(), None);
    let stats = cache.stats();
    assert_eq!(stats.evictions, 1);
}

#[test]
fn test_set_with_size_tracking() {
    let cache = make_cache();
    cache
        .set_with_size("a".to_string(), "data".to_string(), None, 100)
        .unwrap();
    cache
        .set_with_size("b".to_string(), "more".to_string(), None, 200)
        .unwrap();
    let stats = cache.stats();
    assert_eq!(stats.size_bytes, 300);
}

#[test]
fn test_max_size_exceeded_error() {
    let cache = make_size_limited_cache(50);
    let result = cache.set_with_size("a".to_string(), "big".to_string(), None, 100);
    assert!(result.is_err());
}

#[test]
fn test_max_size_rejects_when_full() {
    let cache = make_size_limited_cache(150);
    cache
        .set_with_size("a".to_string(), "big".to_string(), None, 100)
        .unwrap();
    let result = cache.set_with_size("b".to_string(), "big".to_string(), None, 100);
    assert!(result.is_err());
}

#[test]
fn test_cleanup_expired() {
    let cache = make_cache();
    cache.set("a".to_string(), "1".to_string(), Some(Duration::from_micros(100)));
    cache.set("b".to_string(), "2".to_string(), None);
    std::thread::sleep(Duration::from_millis(5));
    let removed = cache.cleanup_expired();
    assert_eq!(removed, 1);
    assert_eq!(cache.len(), 1);
}

#[test]
fn test_overwrite_key() {
    let cache = make_cache();
    cache.set("key".to_string(), "old".to_string(), None);
    cache.set("key".to_string(), "new".to_string(), None);
    assert_eq!(cache.get(&"key".to_string()), Some("new".to_string()));
    assert_eq!(cache.len(), 1);
}

#[test]
fn test_concurrent_access() {
    let cache = Arc::new(TimedCache::new(Some(1000), None));
    let mut handles = Vec::new();
    for i in 0..10 {
        let c = Arc::clone(&cache);
        handles.push(std::thread::spawn(move || {
            let key = format!("key-{}", i);
            let val = format!("val-{}", i);
            c.set(key.clone(), val.clone(), None);
            let _ = c.get(&key);
            c.set(key.clone(), val, None);
        }));
    }
    for h in handles {
        h.join().unwrap();
    }
    assert_eq!(cache.len(), 10);
}

#[test]
fn test_entry_touch_updates_access_count() {
    let cache: TimedCache<String, String> = TimedCache::new(None, None);
    cache.set("key".to_string(), "value".to_string(), None);
    let _ = cache.get(&"key".to_string());
    let _ = cache.get(&"key".to_string());
    assert_eq!(cache.stats().hits, 2);
}

#[test]
fn test_capacity_exceeded_error() {
    let cache = make_lru_cache(2);
    cache.set("a".to_string(), "1".to_string(), None);
    cache.set("b".to_string(), "2".to_string(), None);
    let result = cache.set_with_size("c".to_string(), "3".to_string(), None, 0);
    assert!(result.is_err());
    if let Err(CacheError::CapacityExceeded { entries, max_entries }) = result {
        assert_eq!(entries, 2);
        assert_eq!(max_entries, 2);
    } else {
        panic!("expected CapacityExceeded error");
    }
}

#[tokio::test]
async fn test_async_concurrent_access() {
    let cache = Arc::new(TimedCache::new(Some(100), None));
    let mut handles = Vec::new();
    for i in 0..10u32 {
        let c = Arc::clone(&cache);
        handles.push(tokio::spawn(async move {
            let key = format!("key-{}", i);
            let val = format!("val-{}", i);
            c.set(key.clone(), val.clone(), None);
            let _ = c.get(&key);
        }));
    }
    for h in handles {
        h.await.unwrap();
    }
    assert_eq!(cache.len(), 10);
    let stats = cache.stats();
    assert_eq!(stats.hits, 10);
}

#[test]
fn test_size_replaced_on_overwrite() {
    let cache = make_cache();
    cache
        .set_with_size("k".to_string(), "a".to_string(), None, 100)
        .unwrap();
    cache.set_with_size("k".to_string(), "b".to_string(), None, 50).unwrap();
    let stats = cache.stats();
    assert_eq!(stats.size_bytes, 50);
}
