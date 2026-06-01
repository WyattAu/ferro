//! Multi-tenancy integration.
//!
//! Provides helpers for tenant management and isolation.

use ferro_multi_tenant::tenant::{Tenant, TenantId, TenantStatus, InMemoryTenantStore};
use ferro_multi_tenant::organization::OrganizationId;

pub fn create_tenant_record(
    org_id: &str,
    name: &str,
    slug: &str,
    storage_path: &str,
    owner_id: &str,
) -> Tenant {
    Tenant {
        id: TenantId(uuid::Uuid::new_v4().to_string()),
        organization_id: OrganizationId(org_id.to_string()),
        name: name.to_string(),
        slug: slug.to_string(),
        storage_path: storage_path.to_string(),
        owner_id: owner_id.to_string(),
        created_at: chrono::Utc::now(),
        status: TenantStatus::Active,
    }
}

pub fn create_in_memory_tenant_store() -> InMemoryTenantStore {
    InMemoryTenantStore::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferro_multi_tenant::tenant::TenantStore;

    #[tokio::test]
    async fn test_create_tenant_record() {
        let tenant = create_tenant_record("org-1", "Acme", "acme", "/data/acme", "user-1");
        assert_eq!(tenant.name, "Acme");
        assert_eq!(tenant.slug, "acme");
        assert_eq!(tenant.organization_id.0, "org-1");
        assert_eq!(tenant.status, TenantStatus::Active);
    }

    #[tokio::test]
    async fn test_tenant_store_create_and_get() {
        let store = create_in_memory_tenant_store();
        let tenant = create_tenant_record("org-1", "Test", "test", "/data/test", "user-1");
        let id = tenant.id.0.clone();
        let _created = store.create(tenant).await.unwrap();
        let fetched = store.get(&id).await.unwrap();
        assert_eq!(fetched.id.0, id);
        assert_eq!(fetched.slug, "test");
    }
}
