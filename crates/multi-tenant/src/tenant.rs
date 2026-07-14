use async_trait::async_trait;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};

use crate::error::TenantError;
use crate::organization::OrganizationId;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TenantId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    pub id: TenantId,
    pub organization_id: OrganizationId,
    pub name: String,
    pub slug: String,
    pub storage_path: String,
    pub owner_id: String,
    pub created_at: DateTime<Utc>,
    pub status: TenantStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TenantStatus {
    Active,
    Suspended,
    OverQuota,
    PendingDeletion,
}

pub struct CreateTenant {
    pub organization_id: OrganizationId,
    pub name: String,
    pub slug: String,
    pub storage_path: String,
    pub owner_id: String,
}

#[async_trait]
pub trait TenantStore: Send + Sync {
    async fn create(&self, tenant: Tenant) -> Result<Tenant, TenantError>;
    async fn get(&self, id: &str) -> Result<Tenant, TenantError>;
    async fn update(&self, tenant: Tenant) -> Result<Tenant, TenantError>;
    async fn delete(&self, id: &str) -> Result<(), TenantError>;
    async fn list_by_organization(&self, org_id: &str) -> Result<Vec<Tenant>, TenantError>;
    async fn get_by_slug(&self, slug: &str) -> Result<Tenant, TenantError>;
}

pub struct InMemoryTenantStore {
    tenants: DashMap<String, Tenant>,
}

impl InMemoryTenantStore {
    pub fn new() -> Self {
        Self {
            tenants: DashMap::new(),
        }
    }
}

impl Default for InMemoryTenantStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TenantStore for InMemoryTenantStore {
    async fn create(&self, tenant: Tenant) -> Result<Tenant, TenantError> {
        for existing in self.tenants.iter() {
            if existing.slug == tenant.slug {
                return Err(TenantError::AlreadyExists {
                    name: tenant.slug.clone(),
                });
            }
        }
        self.tenants.insert(tenant.id.0.clone(), tenant.clone());
        Ok(tenant)
    }

    async fn get(&self, id: &str) -> Result<Tenant, TenantError> {
        self.tenants
            .get(id)
            .map(|entry| entry.value().clone())
            .ok_or_else(|| TenantError::NotFound {
                tenant_id: id.to_string(),
            })
    }

    async fn update(&self, tenant: Tenant) -> Result<Tenant, TenantError> {
        if self.tenants.contains_key(&tenant.id.0) {
            self.tenants.insert(tenant.id.0.clone(), tenant.clone());
            Ok(tenant)
        } else {
            Err(TenantError::NotFound {
                tenant_id: tenant.id.0.clone(),
            })
        }
    }

    async fn delete(&self, id: &str) -> Result<(), TenantError> {
        self.tenants
            .remove(id)
            .map(|_| ())
            .ok_or_else(|| TenantError::NotFound {
                tenant_id: id.to_string(),
            })
    }

    async fn list_by_organization(&self, org_id: &str) -> Result<Vec<Tenant>, TenantError> {
        let tenants: Vec<Tenant> = self
            .tenants
            .iter()
            .filter(|entry| entry.organization_id.0 == org_id)
            .map(|entry| entry.value().clone())
            .collect();
        Ok(tenants)
    }

    async fn get_by_slug(&self, slug: &str) -> Result<Tenant, TenantError> {
        for entry in self.tenants.iter() {
            if entry.slug == slug {
                return Ok(entry.value().clone());
            }
        }
        Err(TenantError::NotFound {
            tenant_id: slug.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn make_tenant(slug: &str, org_id: &str, owner_id: &str) -> Tenant {
        Tenant {
            id: TenantId(uuid::Uuid::new_v4().to_string()),
            organization_id: OrganizationId(org_id.to_string()),
            name: slug.to_string(),
            slug: slug.to_string(),
            storage_path: format!("/data/tenants/{org_id}/{slug}"),
            owner_id: owner_id.to_string(),
            created_at: Utc::now(),
            status: TenantStatus::Active,
        }
    }

    fn make_store() -> Arc<dyn TenantStore> {
        Arc::new(InMemoryTenantStore::new())
    }

    #[tokio::test]
    async fn test_create_tenant() {
        let store = make_store();
        let tenant = make_tenant("acme-prod", "org-1", "user-1");
        let id = tenant.id.0.clone();
        let result = store.create(tenant).await.unwrap();
        assert_eq!(result.id.0, id);
        assert_eq!(result.slug, "acme-prod");
    }

    #[tokio::test]
    async fn test_get_tenant() {
        let store = make_store();
        let tenant = make_tenant("acme-prod", "org-1", "user-1");
        let id = tenant.id.0.clone();
        store.create(tenant).await.unwrap();
        let fetched = store.get(&id).await.unwrap();
        assert_eq!(fetched.slug, "acme-prod");
        assert_eq!(fetched.organization_id.0, "org-1");
    }

    #[tokio::test]
    async fn test_get_tenant_not_found() {
        let store = make_store();
        let err = store.get("nonexistent").await.unwrap_err();
        assert_eq!(
            err,
            TenantError::NotFound {
                tenant_id: "nonexistent".to_string()
            }
        );
    }

    #[tokio::test]
    async fn test_duplicate_slug() {
        let store = make_store();
        store.create(make_tenant("acme-prod", "org-1", "user-1")).await.unwrap();
        let err = store
            .create(make_tenant("acme-prod", "org-2", "user-2"))
            .await
            .unwrap_err();
        assert_eq!(
            err,
            TenantError::AlreadyExists {
                name: "acme-prod".to_string()
            }
        );
    }

    #[tokio::test]
    async fn test_list_by_org() {
        let store = make_store();
        store.create(make_tenant("prod-1", "org-1", "user-1")).await.unwrap();
        store.create(make_tenant("staging", "org-1", "user-1")).await.unwrap();
        store.create(make_tenant("prod-2", "org-2", "user-2")).await.unwrap();

        let tenants = store.list_by_organization("org-1").await.unwrap();
        assert_eq!(tenants.len(), 2);

        let tenants = store.list_by_organization("org-2").await.unwrap();
        assert_eq!(tenants.len(), 1);
    }

    #[tokio::test]
    async fn test_slug_lookup() {
        let store = make_store();
        let tenant = make_tenant("acme-prod", "org-1", "user-1");
        store.create(tenant).await.unwrap();
        let found = store.get_by_slug("acme-prod").await.unwrap();
        assert_eq!(found.organization_id.0, "org-1");

        let err = store.get_by_slug("nonexistent").await.unwrap_err();
        assert_eq!(
            err,
            TenantError::NotFound {
                tenant_id: "nonexistent".to_string()
            }
        );
    }

    #[tokio::test]
    async fn test_delete_tenant() {
        let store = make_store();
        let tenant = make_tenant("acme-prod", "org-1", "user-1");
        let id = tenant.id.0.clone();
        store.create(tenant).await.unwrap();
        store.delete(&id).await.unwrap();
        assert!(store.get(&id).await.is_err());
    }

    #[tokio::test]
    async fn test_update_tenant() {
        let store = make_store();
        let mut tenant = make_tenant("acme-prod", "org-1", "user-1");
        let id = tenant.id.0.clone();
        store.create(tenant.clone()).await.unwrap();

        tenant.status = TenantStatus::Suspended;
        store.update(tenant).await.unwrap();

        let updated = store.get(&id).await.unwrap();
        assert_eq!(updated.status, TenantStatus::Suspended);
    }
}
