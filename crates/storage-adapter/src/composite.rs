use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;

use crate::backend::{BackendType, ObjectInfo, ObjectMetadata, StorageBackend};
use crate::error::StorageAdapterError;

use ferro_backend_router::{BackendId, BackendRouter};

#[cfg(test)]
use ferro_backend_router::{RoutingPolicy, RoutingRule};

struct BackendEntry {
    backend: Arc<dyn StorageBackend>,
}

pub struct CompositeBackend {
    router: BackendRouter,
    backends: HashMap<String, BackendEntry>,
    default: Option<Arc<dyn StorageBackend>>,
}

impl CompositeBackend {
    pub fn new(router: BackendRouter) -> Self {
        Self {
            router,
            backends: HashMap::new(),
            default: None,
        }
    }

    pub fn register(&mut self, id: &str, backend: Arc<dyn StorageBackend>) {
        let _backend_id = match id {
            "local" => BackendId::Local,
            "s3" => BackendId::S3,
            "gcs" => BackendId::Gcs,
            "azure" | "azureblob" => BackendId::AzureBlob,
            other => BackendId::Custom(other.to_string()),
        };
        self.backends.insert(id.to_string(), BackendEntry { backend });
    }

    pub fn set_default(&mut self, backend: Arc<dyn StorageBackend>) {
        self.default = Some(backend);
    }

    fn resolve_backend(
        &self,
        path: &str,
        metadata: &HashMap<String, String>,
    ) -> Result<Arc<dyn StorageBackend>, StorageAdapterError> {
        let decision = self
            .router
            .route(path, metadata)
            .map_err(|e| StorageAdapterError::BackendUnavailable(e.to_string()))?;

        let key = match &decision.backend_id {
            BackendId::Local => "local".to_string(),
            BackendId::S3 => "s3".to_string(),
            BackendId::Gcs => "gcs".to_string(),
            BackendId::AzureBlob => "azure".to_string(),
            BackendId::Custom(s) => s.clone(),
        };

        if let Some(entry) = self.backends.get(&key) {
            Ok(Arc::clone(&entry.backend))
        } else if let Some(ref default) = self.default {
            Ok(Arc::clone(default))
        } else {
            Err(StorageAdapterError::BackendUnavailable(format!(
                "no backend registered for {key}"
            )))
        }
    }
}

impl std::fmt::Debug for CompositeBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompositeBackend")
            .field("backends", &self.backends.keys().collect::<Vec<_>>())
            .field("has_default", &self.default.is_some())
            .finish()
    }
}

#[async_trait]
impl StorageBackend for CompositeBackend {
    async fn get(&self, path: &str) -> Result<Vec<u8>, StorageAdapterError> {
        let backend = self.resolve_backend(path, &HashMap::new())?;
        backend.get(path).await
    }

    async fn put(
        &self,
        path: &str,
        data: &[u8],
        metadata: &ObjectMetadata,
    ) -> Result<(), StorageAdapterError> {
        let backend = self.resolve_backend(path, &metadata.custom_headers)?;
        backend.put(path, data, metadata).await
    }

    async fn delete(&self, path: &str) -> Result<(), StorageAdapterError> {
        let backend = self.resolve_backend(path, &HashMap::new())?;
        backend.delete(path).await
    }

    async fn exists(&self, path: &str) -> Result<bool, StorageAdapterError> {
        let backend = self.resolve_backend(path, &HashMap::new())?;
        backend.exists(path).await
    }

    async fn list(&self, prefix: &str) -> Result<Vec<ObjectInfo>, StorageAdapterError> {
        let backend = self.resolve_backend(prefix, &HashMap::new())?;
        backend.list(prefix).await
    }

    async fn size(&self, path: &str) -> Result<u64, StorageAdapterError> {
        let backend = self.resolve_backend(path, &HashMap::new())?;
        backend.size(path).await
    }

    async fn copy(&self, from: &str, to: &str) -> Result<(), StorageAdapterError> {
        let backend = self.resolve_backend(from, &HashMap::new())?;
        backend.copy(from, to).await
    }

    async fn move_obj(&self, from: &str, to: &str) -> Result<(), StorageAdapterError> {
        let backend = self.resolve_backend(from, &HashMap::new())?;
        backend.move_obj(from, to).await
    }

    async fn metadata(&self, path: &str) -> Result<ObjectInfo, StorageAdapterError> {
        let backend = self.resolve_backend(path, &HashMap::new())?;
        backend.metadata(path).await
    }

    fn backend_type(&self) -> BackendType {
        BackendType::Composite
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_router() -> BackendRouter {
        let mut router = BackendRouter::new();
        router
            .add_policy(
                RoutingPolicy::new("test", BackendId::Local).add_rule(RoutingRule {
                    pattern: "local/**".to_string(),
                    backend_id: BackendId::Local,
                    priority: 10,
                    metadata_filter: HashMap::new(),
                    read_fallback: false,
                }),
            )
            .unwrap();
        router
            .add_policy(
                RoutingPolicy::new("s3-policy", BackendId::S3).add_rule(RoutingRule {
                    pattern: "s3/**".to_string(),
                    backend_id: BackendId::S3,
                    priority: 10,
                    metadata_filter: HashMap::new(),
                    read_fallback: false,
                }),
            )
            .unwrap();
        router
    }

    #[tokio::test]
    async fn test_routes_to_correct_backend() {
        let mem_local = Arc::new(crate::memory::InMemoryBackend::new());
        let mem_s3 = Arc::new(crate::memory::InMemoryBackend::new());

        let mut comp = CompositeBackend::new(make_router());
        comp.register("local", mem_local.clone());
        comp.register("s3", mem_s3.clone());

        let meta = ObjectMetadata::new();
        comp.put("local/a.txt", b"local-data", &meta).await.unwrap();
        comp.put("s3/b.txt", b"s3-data", &meta).await.unwrap();

        assert!(mem_local.exists("local/a.txt").await.unwrap());
        assert!(mem_s3.exists("s3/b.txt").await.unwrap());
        assert!(!mem_local.exists("s3/b.txt").await.unwrap());
    }

    #[tokio::test]
    async fn test_get_from_composite() {
        let mem = Arc::new(crate::memory::InMemoryBackend::new());
        let mut comp = CompositeBackend::new(make_router());
        comp.register("local", mem.clone());
        comp.put("local/x", b"data", &ObjectMetadata::new()).await.unwrap();
        assert_eq!(comp.get("local/x").await.unwrap(), b"data");
    }

    #[tokio::test]
    async fn test_default_fallback() {
        let mem = Arc::new(crate::memory::InMemoryBackend::new());
        let mut comp = CompositeBackend::new(make_router());
        comp.set_default(mem.clone());

        let meta = ObjectMetadata::new();
        comp.put("unknown/path", b"fallback", &meta).await.unwrap();
        assert_eq!(comp.get("unknown/path").await.unwrap(), b"fallback");
    }

    #[tokio::test]
    async fn test_no_backend_error() {
        let comp = CompositeBackend::new(make_router());
        let result = comp.get("local/nope").await;
        assert!(matches!(result, Err(StorageAdapterError::BackendUnavailable(_))));
    }

    #[tokio::test]
    async fn test_delete_through_composite() {
        let mem = Arc::new(crate::memory::InMemoryBackend::new());
        let mut comp = CompositeBackend::new(make_router());
        comp.register("s3", mem.clone());

        comp.put("s3/d", b"x", &ObjectMetadata::new()).await.unwrap();
        comp.delete("s3/d").await.unwrap();
        assert!(!comp.exists("s3/d").await.unwrap());
    }

    #[tokio::test]
    async fn test_composite_backend_type() {
        let comp = CompositeBackend::new(make_router());
        assert_eq!(comp.backend_type(), BackendType::Composite);
    }
}
