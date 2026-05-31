use async_trait::async_trait;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};

use crate::error::TenantError;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OrganizationId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organization {
    pub id: OrganizationId,
    pub name: String,
    pub display_name: String,
    pub owner_id: String,
    pub created_at: DateTime<Utc>,
    pub settings: OrganizationSettings,
    pub status: OrganizationStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationSettings {
    #[serde(default = "default_max_tenant_count")]
    pub max_tenant_count: u32,
    #[serde(default = "default_storage_quota_bytes")]
    pub default_storage_quota_bytes: u64,
    #[serde(default)]
    pub allowed_features: Vec<String>,
    pub branding: Option<OrganizationBranding>,
}

fn default_max_tenant_count() -> u32 {
    100
}

fn default_storage_quota_bytes() -> u64 {
    10 * 1024 * 1024 * 1024 // 10 GB
}

impl Default for OrganizationSettings {
    fn default() -> Self {
        Self {
            max_tenant_count: default_max_tenant_count(),
            default_storage_quota_bytes: default_storage_quota_bytes(),
            allowed_features: Vec::new(),
            branding: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationBranding {
    pub logo_url: Option<String>,
    pub primary_color: Option<String>,
    pub custom_domain: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrganizationStatus {
    Active,
    Suspended,
    PendingDeletion,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum OrganizationRole {
    Owner,
    Admin,
    Member,
    ReadOnly,
}

#[derive(Debug, Clone)]
pub struct OrganizationMember {
    pub user_id: String,
    pub role: OrganizationRole,
}

#[derive(Default)]
pub struct CreateOrganization {
    pub name: String,
    pub display_name: String,
    pub owner_id: String,
    pub settings: OrganizationSettings,
}


#[async_trait]
pub trait OrganizationStore: Send + Sync {
    async fn create(&self, org: Organization) -> Result<Organization, TenantError>;
    async fn get(&self, id: &str) -> Result<Organization, TenantError>;
    async fn update(&self, org: Organization) -> Result<Organization, TenantError>;
    async fn delete(&self, id: &str) -> Result<(), TenantError>;
    async fn list_by_owner(&self, owner_id: &str) -> Result<Vec<Organization>, TenantError>;
    async fn add_member(
        &self,
        org_id: &str,
        user_id: &str,
        role: OrganizationRole,
    ) -> Result<(), TenantError>;
    async fn remove_member(&self, org_id: &str, user_id: &str) -> Result<(), TenantError>;
    async fn get_members(&self, org_id: &str) -> Result<Vec<OrganizationMember>, TenantError>;
}

pub struct InMemoryOrganizationStore {
    orgs: DashMap<String, Organization>,
    members: DashMap<String, DashMap<String, OrganizationRole>>,
}

impl InMemoryOrganizationStore {
    pub fn new() -> Self {
        Self {
            orgs: DashMap::new(),
            members: DashMap::new(),
        }
    }
}

impl Default for InMemoryOrganizationStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OrganizationStore for InMemoryOrganizationStore {
    async fn create(&self, org: Organization) -> Result<Organization, TenantError> {
        for existing in self.orgs.iter() {
            if existing.name == org.name {
                return Err(TenantError::AlreadyExists {
                    name: org.name.clone(),
                });
            }
        }
        self.orgs.insert(org.id.0.clone(), org.clone());
        let member_map = self.members.entry(org.id.0.clone()).or_default();
        member_map.insert(org.owner_id.clone(), OrganizationRole::Owner);
        Ok(org)
    }

    async fn get(&self, id: &str) -> Result<Organization, TenantError> {
        self.orgs
            .get(id)
            .map(|entry| entry.value().clone())
            .ok_or_else(|| TenantError::OrganizationNotFound { org_id: id.to_string() })
    }

    async fn update(&self, org: Organization) -> Result<Organization, TenantError> {
        if self.orgs.contains_key(&org.id.0) {
            self.orgs.insert(org.id.0.clone(), org.clone());
            Ok(org)
        } else {
            Err(TenantError::OrganizationNotFound { org_id: org.id.0.clone() })
        }
    }

    async fn delete(&self, id: &str) -> Result<(), TenantError> {
        self.orgs
            .remove(id)
            .map(|_| ())
            .ok_or_else(|| TenantError::OrganizationNotFound { org_id: id.to_string() })
    }

    async fn list_by_owner(&self, owner_id: &str) -> Result<Vec<Organization>, TenantError> {
        let orgs: Vec<Organization> = self
            .orgs
            .iter()
            .filter(|entry| entry.owner_id == owner_id)
            .map(|entry| entry.value().clone())
            .collect();
        Ok(orgs)
    }

    async fn add_member(
        &self,
        org_id: &str,
        user_id: &str,
        role: OrganizationRole,
    ) -> Result<(), TenantError> {
        if !self.orgs.contains_key(org_id) {
            return Err(TenantError::OrganizationNotFound {
                org_id: org_id.to_string(),
            });
        }
        let member_map = self.members.entry(org_id.to_string()).or_default();
        member_map.insert(user_id.to_string(), role);
        Ok(())
    }

    async fn remove_member(&self, org_id: &str, user_id: &str) -> Result<(), TenantError> {
        if !self.orgs.contains_key(org_id) {
            return Err(TenantError::OrganizationNotFound {
                org_id: org_id.to_string(),
            });
        }
        if let Some(member_map) = self.members.get(org_id) {
            member_map.remove(user_id);
        }
        Ok(())
    }

    async fn get_members(&self, org_id: &str) -> Result<Vec<OrganizationMember>, TenantError> {
        let member_map = self
            .members
            .get(org_id)
            .ok_or_else(|| TenantError::OrganizationNotFound {
                org_id: org_id.to_string(),
            })?;
        let members: Vec<OrganizationMember> = member_map
            .iter()
            .map(|entry| OrganizationMember {
                user_id: entry.key().clone(),
                role: entry.value().clone(),
            })
            .collect();
        Ok(members)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn make_org(name: &str, owner_id: &str) -> Organization {
        Organization {
            id: OrganizationId(uuid::Uuid::new_v4().to_string()),
            name: name.to_string(),
            display_name: name.to_string(),
            owner_id: owner_id.to_string(),
            created_at: Utc::now(),
            settings: OrganizationSettings::default(),
            status: OrganizationStatus::Active,
        }
    }

    fn make_store() -> Arc<dyn OrganizationStore> {
        Arc::new(InMemoryOrganizationStore::new())
    }

    #[tokio::test]
    async fn test_create_org() {
        let store = make_store();
        let org = make_org("acme", "user-1");
        let id = org.id.0.clone();
        let result = store.create(org).await.unwrap();
        assert_eq!(result.id.0, id);
        assert_eq!(result.name, "acme");
    }

    #[tokio::test]
    async fn test_get_org() {
        let store = make_store();
        let org = make_org("acme", "user-1");
        let id = org.id.0.clone();
        store.create(org).await.unwrap();
        let fetched = store.get(&id).await.unwrap();
        assert_eq!(fetched.name, "acme");
    }

    #[tokio::test]
    async fn test_get_org_not_found() {
        let store = make_store();
        let err = store.get("nonexistent").await.unwrap_err();
        assert_eq!(
            err,
            TenantError::OrganizationNotFound {
                org_id: "nonexistent".to_string()
            }
        );
    }

    #[tokio::test]
    async fn test_duplicate_name_error() {
        let store = make_store();
        store.create(make_org("acme", "user-1")).await.unwrap();
        let err = store.create(make_org("acme", "user-2")).await.unwrap_err();
        assert_eq!(
            err,
            TenantError::AlreadyExists {
                name: "acme".to_string()
            }
        );
    }

    #[tokio::test]
    async fn test_add_remove_member() {
        let store = make_store();
        let org = make_org("acme", "user-1");
        let id = org.id.0.clone();
        store.create(org).await.unwrap();

        store
            .add_member(&id, "user-2", OrganizationRole::Admin)
            .await
            .unwrap();
        let members = store.get_members(&id).await.unwrap();
        assert_eq!(members.len(), 2);

        store.remove_member(&id, "user-2").await.unwrap();
        let members = store.get_members(&id).await.unwrap();
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].user_id, "user-1");
    }

    #[tokio::test]
    async fn test_list_by_owner() {
        let store = make_store();
        store.create(make_org("acme", "user-1")).await.unwrap();
        store.create(make_org("beta", "user-1")).await.unwrap();
        store.create(make_org("gamma", "user-2")).await.unwrap();

        let orgs = store.list_by_owner("user-1").await.unwrap();
        assert_eq!(orgs.len(), 2);
        let names: Vec<&str> = orgs.iter().map(|o| o.name.as_str()).collect();
        assert!(names.contains(&"acme"));
        assert!(names.contains(&"beta"));

        let orgs = store.list_by_owner("user-2").await.unwrap();
        assert_eq!(orgs.len(), 1);
        assert_eq!(orgs[0].name, "gamma");
    }

    #[tokio::test]
    async fn test_suspension() {
        let store = make_store();
        let org = make_org("acme", "user-1");
        let id = org.id.0.clone();
        store.create(org).await.unwrap();

        let mut fetched = store.get(&id).await.unwrap();
        assert_eq!(fetched.status, OrganizationStatus::Active);
        fetched.status = OrganizationStatus::Suspended;
        store.update(fetched).await.unwrap();

        let suspended = store.get(&id).await.unwrap();
        assert_eq!(suspended.status, OrganizationStatus::Suspended);
    }

    #[tokio::test]
    async fn test_delete_org() {
        let store = make_store();
        let org = make_org("acme", "user-1");
        let id = org.id.0.clone();
        store.create(org).await.unwrap();
        store.delete(&id).await.unwrap();
        assert!(store.get(&id).await.is_err());
    }

    #[tokio::test]
    async fn test_add_member_nonexistent_org() {
        let store = make_store();
        let err = store
            .add_member("nonexistent", "user-1", OrganizationRole::Member)
            .await
            .unwrap_err();
        assert_eq!(
            err,
            TenantError::OrganizationNotFound {
                org_id: "nonexistent".to_string()
            }
        );
    }
}
