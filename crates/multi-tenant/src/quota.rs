use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use dashmap::DashMap;

use crate::error::TenantError;
use crate::tenant::TenantStore;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuotaInfo {
    pub tenant_id: String,
    pub quota_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: i64,
}

#[async_trait]
pub trait UsageTracker: Send + Sync {
    async fn get_storage_usage(&self, tenant_id: &str) -> Result<u64, TenantError>;
    async fn record_upload(&self, tenant_id: &str, bytes: u64) -> Result<(), TenantError>;
    async fn record_delete(&self, tenant_id: &str, bytes: u64) -> Result<(), TenantError>;
}

pub struct InMemoryUsageTracker {
    usage: DashMap<String, AtomicU64>,
}

impl InMemoryUsageTracker {
    pub fn new() -> Self {
        Self {
            usage: DashMap::new(),
        }
    }
}

impl Default for InMemoryUsageTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl UsageTracker for InMemoryUsageTracker {
    async fn get_storage_usage(&self, tenant_id: &str) -> Result<u64, TenantError> {
        Ok(self
            .usage
            .get(tenant_id)
            .map(|entry| entry.value().load(Ordering::Relaxed))
            .unwrap_or(0))
    }

    async fn record_upload(&self, tenant_id: &str, bytes: u64) -> Result<(), TenantError> {
        let counter = self.usage.entry(tenant_id.to_string()).or_default();
        counter.fetch_add(bytes, Ordering::Relaxed);
        Ok(())
    }

    async fn record_delete(&self, tenant_id: &str, bytes: u64) -> Result<(), TenantError> {
        let counter = self.usage.entry(tenant_id.to_string()).or_default();
        let current = counter.load(Ordering::Relaxed);
        if current >= bytes {
            counter.fetch_sub(bytes, Ordering::Relaxed);
        } else {
            counter.store(0, Ordering::Relaxed);
        }
        Ok(())
    }
}

pub struct QuotaManager {
    pub tenant_store: Arc<dyn TenantStore>,
    pub usage_tracker: Arc<dyn UsageTracker>,
}

impl QuotaManager {
    pub fn new(
        tenant_store: Arc<dyn TenantStore>,
        usage_tracker: Arc<dyn UsageTracker>,
    ) -> Self {
        Self {
            tenant_store,
            usage_tracker,
        }
    }

    pub async fn check_quota(
        &self,
        tenant_id: &str,
        additional_bytes: u64,
    ) -> Result<(), TenantError> {
        let tenant = self.tenant_store.get(tenant_id).await?;
        let current = self.usage_tracker.get_storage_usage(tenant_id).await?;

        let quota_bytes = 10u64 * 1024 * 1024 * 1024; // 10 GB default

        if current + additional_bytes > quota_bytes {
            return Err(TenantError::QuotaExceeded {
                tenant_id: tenant_id.to_string(),
                quota: quota_bytes,
                current,
            });
        }

        if tenant.status == crate::tenant::TenantStatus::Suspended {
            return Err(TenantError::Suspended {
                tenant_id: tenant_id.to_string(),
            });
        }

        Ok(())
    }

    pub async fn get_quota_info(&self, tenant_id: &str) -> Result<QuotaInfo, TenantError> {
        self.tenant_store.get(tenant_id).await?;
        let quota_bytes = 10u64 * 1024 * 1024 * 1024;
        let used_bytes = self.usage_tracker.get_storage_usage(tenant_id).await?;
        let available = (quota_bytes as i64) - (used_bytes as i64);

        Ok(QuotaInfo {
            tenant_id: tenant_id.to_string(),
            quota_bytes,
            used_bytes,
            available_bytes: available,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tenant::{InMemoryTenantStore, Tenant, TenantId, TenantStatus};
    use crate::organization::OrganizationId;
    use chrono::Utc;

    fn make_test_tenant(id: &str, slug: &str) -> Tenant {
        Tenant {
            id: TenantId(id.to_string()),
            organization_id: OrganizationId("org-1".to_string()),
            name: slug.to_string(),
            slug: slug.to_string(),
            storage_path: format!("/data/tenants/org-1/{slug}"),
            owner_id: "user-1".to_string(),
            created_at: Utc::now(),
            status: TenantStatus::Active,
        }
    }

    fn setup() -> (QuotaManager, Arc<InMemoryUsageTracker>) {
        let tenant_store: Arc<dyn TenantStore> = Arc::new(InMemoryTenantStore::new());
        let usage_tracker: Arc<InMemoryUsageTracker> = Arc::new(InMemoryUsageTracker::new());
        let manager = QuotaManager::new(
            tenant_store.clone(),
            usage_tracker.clone() as Arc<dyn UsageTracker>,
        );
        (manager, usage_tracker)
    }

    #[tokio::test]
    async fn test_under_quota_succeeds() {
        let (manager, tracker) = setup();
        let store = manager.tenant_store.clone();
        store
            .create(make_test_tenant("t1", "tenant-1"))
            .await
            .unwrap();
        tracker.record_upload("t1", 100).await.unwrap();

        manager.check_quota("t1", 50).await.unwrap();
    }

    #[tokio::test]
    async fn test_over_quota_fails() {
        let (manager, tracker) = setup();
        let store = manager.tenant_store.clone();
        store
            .create(make_test_tenant("t1", "tenant-1"))
            .await
            .unwrap();

        let ten_gb = 10u64 * 1024 * 1024 * 1024;
        tracker.record_upload("t1", ten_gb - 1).await.unwrap();
        assert!(manager.check_quota("t1", 2).await.is_err());
    }

    #[tokio::test]
    async fn test_exact_quota_boundary() {
        let (manager, tracker) = setup();
        let store = manager.tenant_store.clone();
        store
            .create(make_test_tenant("t1", "tenant-1"))
            .await
            .unwrap();

        let ten_gb = 10u64 * 1024 * 1024 * 1024;
        tracker.record_upload("t1", ten_gb - 100).await.unwrap();

        manager.check_quota("t1", 100).await.unwrap();
        assert!(manager.check_quota("t1", 101).await.is_err());
    }

    #[tokio::test]
    async fn test_zero_quota() {
        let (manager, tracker) = setup();
        let store = manager.tenant_store.clone();
        store
            .create(make_test_tenant("t1", "tenant-1"))
            .await
            .unwrap();

        tracker.record_upload("t1", 0).await.unwrap();
        assert!(manager.check_quota("t1", 1).await.is_ok());
    }

    #[tokio::test]
    async fn test_get_quota_info() {
        let (manager, tracker) = setup();
        let store = manager.tenant_store.clone();
        store
            .create(make_test_tenant("t1", "tenant-1"))
            .await
            .unwrap();
        tracker.record_upload("t1", 1024).await.unwrap();

        let info = manager.get_quota_info("t1").await.unwrap();
        assert_eq!(info.used_bytes, 1024);
        assert_eq!(info.quota_bytes, 10u64 * 1024 * 1024 * 1024);
        assert_eq!(
            info.available_bytes,
            (10u64 * 1024 * 1024 * 1024 - 1024) as i64
        );
    }

    #[tokio::test]
    async fn test_record_delete() {
        let (manager, tracker) = setup();
        let store = manager.tenant_store.clone();
        store
            .create(make_test_tenant("t1", "tenant-1"))
            .await
            .unwrap();
        tracker.record_upload("t1", 1000).await.unwrap();
        tracker.record_delete("t1", 400).await.unwrap();

        let usage = tracker.get_storage_usage("t1").await.unwrap();
        assert_eq!(usage, 600);
    }

    #[tokio::test]
    async fn test_check_quota_nonexistent_tenant() {
        let (manager, _) = setup();
        assert!(manager.check_quota("nonexistent", 1).await.is_err());
    }
}
