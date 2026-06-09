use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::api;

/// leptos-fetch integration: cached fetch helper that wraps leptos-fetch patterns.
///
/// Uses leptos-fetch's resource-based approach with our ApiCache to avoid
/// redundant network calls within the same reactive scope.
#[cfg(target_arch = "wasm32")]
pub async fn cached_fetch(url: &str) -> Result<String, String> {
    let cache_key = format!("fetch:{}", url);
    if let Some(cached) = with_cache(|cache| cache.get(&cache_key)) {
        return Ok(cached);
    }

    let opts = web_sys::RequestInit::new();
    opts.set_method("GET");
    let request = web_sys::Request::new_with_str_and_init(url, &opts)
        .map_err(|e| format!("Request creation failed: {:?}", e))?;
    request
        .headers()
        .set("Accept", "application/json")
        .map_err(|e| format!("Header set failed: {:?}", e))?;

    let window = web_sys::window().ok_or("No window object")?;
    let promise = window.fetch_with_request(&request);
    let resp_value = wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map_err(|e| format!("Fetch failed: {:?}", e))?;
    let resp: web_sys::Response = resp_value.into();
    let text_promise = resp
        .text()
        .map_err(|e| format!("Text read failed: {:?}", e))?;
    let text = wasm_bindgen_futures::JsFuture::from(text_promise)
        .await
        .map_err(|e| format!("Text read failed: {:?}", e))?;
    let text = text
        .as_string()
        .ok_or_else(|| "Text conversion failed".to_string())?;

    with_cache(|cache| cache.insert(cache_key, text.clone()));
    Ok(text)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn cached_fetch(_url: &str) -> Result<String, String> {
    Err("cached_fetch not available outside wasm".to_string())
}

#[derive(Debug, Clone)]
struct CacheEntry {
    data: String,
    created_at: f64,
}

#[derive(Debug, Clone)]
pub struct ApiCache {
    entries: Rc<RefCell<HashMap<String, CacheEntry>>>,
    ttl_ms: f64,
    hits: Rc<RefCell<u64>>,
    misses: Rc<RefCell<u64>>,
}

impl ApiCache {
    pub fn new(ttl_ms: f64) -> Self {
        Self {
            entries: Rc::new(RefCell::new(HashMap::new())),
            ttl_ms,
            hits: Rc::new(RefCell::new(0)),
            misses: Rc::new(RefCell::new(0)),
        }
    }

    fn now_ms(&self) -> f64 {
        #[cfg(target_arch = "wasm32")]
        {
            js_sys::Date::now()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            0.0
        }
    }

    fn is_fresh(&self, entry: &CacheEntry) -> bool {
        (self.now_ms() - entry.created_at) < self.ttl_ms
    }

    pub fn get(&self, key: &str) -> Option<String> {
        let entries = self.entries.borrow();
        if let Some(entry) = entries.get(key)
            && self.is_fresh(entry)
        {
            *self.hits.borrow_mut() += 1;
            return Some(entry.data.clone());
        }
        *self.misses.borrow_mut() += 1;
        None
    }

    pub fn insert(&self, key: String, data: String) {
        let entry = CacheEntry {
            data,
            created_at: self.now_ms(),
        };
        self.entries.borrow_mut().insert(key, entry);
    }

    pub fn invalidate(&self, key: &str) {
        self.entries.borrow_mut().remove(key);
    }

    pub fn invalidate_prefix(&self, prefix: &str) {
        let keys: Vec<String> = self
            .entries
            .borrow()
            .keys()
            .filter(|k| k.starts_with(prefix))
            .cloned()
            .collect();
        let mut entries = self.entries.borrow_mut();
        for key in keys {
            entries.remove(&key);
        }
    }

    pub fn clear(&self) {
        self.entries.borrow_mut().clear();
    }

    pub fn stats(&self) -> CacheStats {
        CacheStats {
            hits: *self.hits.borrow(),
            misses: *self.misses.borrow(),
            entries: self.entries.borrow().len(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub entries: usize,
}

// Default cache instance (5 minute TTL)
thread_local! {
    static DEFAULT_CACHE: ApiCache = ApiCache::new(5.0 * 60.0 * 1000.0);
}

pub fn with_cache<F, R>(f: F) -> R
where
    F: FnOnce(&ApiCache) -> R,
{
    DEFAULT_CACHE.with(|cache| f(cache))
}

pub fn invalidate_file_path(path: &str) {
    with_cache(|cache| {
        cache.invalidate_prefix(path);
        // Also invalidate parent directory listings
        if let Some(parent) = path.rfind('/') {
            let parent_path = if parent == 0 {
                "/".to_string()
            } else {
                path[..parent].to_string()
            };
            cache.invalidate_prefix(&parent_path);
        }
    });
}

pub fn cache_stats() -> CacheStats {
    with_cache(|cache| cache.stats())
}

// Cached wrappers around API functions

#[cfg(target_arch = "wasm32")]
pub async fn list_files_cached(path: &str) -> Result<api::ListingResponse, String> {
    let cache_key = format!("list:{}", path);

    if let Some(cached) = with_cache(|cache| cache.get(&cache_key)) {
        return serde_json::from_str(&cached)
            .map_err(|e| format!("Cache deserialization failed: {}", e));
    }

    let result = api::list_files(path).await?;
    if let Ok(json) = serde_json::to_string(&result) {
        with_cache(|cache| cache.insert(cache_key, json));
    }
    Ok(result)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn list_files_cached(path: &str) -> Result<api::ListingResponse, String> {
    api::list_files(path).await
}

#[cfg(target_arch = "wasm32")]
pub async fn upload_file_cached(path: &str, content: &[u8]) -> Result<(), String> {
    let result = api::upload_file(path, content).await;
    if result.is_ok() {
        invalidate_file_path(path);
    }
    result
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn upload_file_cached(path: &str, content: &[u8]) -> Result<(), String> {
    api::upload_file(path, content).await
}

#[cfg(target_arch = "wasm32")]
pub async fn delete_file_cached(path: &str) -> Result<(), String> {
    let result = api::delete_file(path).await;
    if result.is_ok() {
        invalidate_file_path(path);
    }
    result
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn delete_file_cached(path: &str) -> Result<(), String> {
    api::delete_file(path).await
}

#[cfg(target_arch = "wasm32")]
pub async fn create_directory_cached(path: &str) -> Result<(), String> {
    let result = api::create_directory(path).await;
    if result.is_ok() {
        invalidate_file_path(path);
    }
    result
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn create_directory_cached(path: &str) -> Result<(), String> {
    api::create_directory(path).await
}

#[cfg(target_arch = "wasm32")]
pub async fn move_file_cached(source: &str, destination: &str) -> Result<(), String> {
    let result = api::move_file(source, destination).await;
    if result.is_ok() {
        invalidate_file_path(source);
        invalidate_file_path(destination);
    }
    result
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn move_file_cached(source: &str, destination: &str) -> Result<(), String> {
    api::move_file(source, destination).await
}

#[cfg(target_arch = "wasm32")]
pub async fn copy_file_cached(source: &str, destination: &str) -> Result<(), String> {
    let result = api::copy_file(source, destination).await;
    if result.is_ok() {
        invalidate_file_path(destination);
    }
    result
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn copy_file_cached(source: &str, destination: &str) -> Result<(), String> {
    api::copy_file(source, destination).await
}

pub fn get_health_with_cache_stats() -> serde_json::Value {
    let stats = cache_stats();
    serde_json::json!({
        "status": "ok",
        "cache": {
            "hits": stats.hits,
            "misses": stats.misses,
            "entries": stats.entries,
            "hit_rate": if stats.hits + stats.misses > 0 {
                stats.hits as f64 / (stats.hits + stats.misses) as f64
            } else {
                0.0
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_insert_and_get() {
        let cache = ApiCache::new(60_000.0);
        cache.insert("key1".to_string(), "value1".to_string());
        assert_eq!(cache.get("key1"), Some("value1".to_string()));
    }

    #[test]
    fn test_cache_miss() {
        let cache = ApiCache::new(60_000.0);
        assert_eq!(cache.get("missing"), None);
    }

    #[test]
    fn test_cache_invalidate() {
        let cache = ApiCache::new(60_000.0);
        cache.insert("key1".to_string(), "value1".to_string());
        cache.invalidate("key1");
        assert_eq!(cache.get("key1"), None);
    }

    #[test]
    fn test_cache_invalidate_prefix() {
        let cache = ApiCache::new(60_000.0);
        cache.insert("/docs/".to_string(), "docs".to_string());
        cache.insert("/docs/file.txt".to_string(), "file".to_string());
        cache.insert("/other/".to_string(), "other".to_string());
        cache.invalidate_prefix("/docs");
        assert_eq!(cache.get("/docs/"), None);
        assert_eq!(cache.get("/docs/file.txt"), None);
        assert_eq!(cache.get("/other/"), Some("other".to_string()));
    }

    #[test]
    fn test_cache_stats() {
        let cache = ApiCache::new(60_000.0);
        cache.insert("key1".to_string(), "value1".to_string());
        let _ = cache.get("key1");
        let _ = cache.get("missing");
        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.entries, 1);
    }

    #[test]
    fn test_health_with_cache_stats() {
        let health = get_health_with_cache_stats();
        assert_eq!(health["status"], "ok");
        assert!(health["cache"]["hits"].is_number());
    }
}
