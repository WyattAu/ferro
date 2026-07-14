use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Tenant plan
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TenantPlan {
    Free,
    Pro,
    Enterprise,
}

/// Tenant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    pub id: String,
    pub name: String,
    pub domain: String,
    pub plan: TenantPlan,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Tenant context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantContext {
    pub tenant_id: String,
    pub user_id: String,
    pub permissions: Vec<String>,
}

/// Create tenant request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTenantRequest {
    pub name: String,
    pub domain: String,
    pub plan: TenantPlan,
}

/// Tenant manager
pub struct TenantManager {
    tenants: Vec<Tenant>,
}

impl Default for TenantManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TenantManager {
    pub fn new() -> Self {
        Self { tenants: Vec::new() }
    }

    /// Create a new tenant
    pub fn create_tenant(&mut self, request: CreateTenantRequest) -> Tenant {
        let tenant = Tenant {
            id: uuid::Uuid::new_v4().to_string(),
            name: request.name,
            domain: request.domain,
            plan: request.plan,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        self.tenants.push(tenant.clone());
        tenant
    }

    /// Get tenant by ID
    pub fn get_tenant(&self, tenant_id: &str) -> Option<&Tenant> {
        self.tenants.iter().find(|t| t.id == tenant_id)
    }

    /// List all tenants
    pub fn list_tenants(&self) -> &[Tenant] {
        &self.tenants
    }

    /// Delete tenant
    pub fn delete_tenant(&mut self, tenant_id: &str) -> bool {
        let len_before = self.tenants.len();
        self.tenants.retain(|t| t.id != tenant_id);
        self.tenants.len() < len_before
    }

    /// Switch tenant
    pub fn switch_tenant(&self, user_id: &str, tenant_id: &str) -> Option<TenantContext> {
        if self.get_tenant(tenant_id).is_some() {
            Some(TenantContext {
                tenant_id: tenant_id.to_string(),
                user_id: user_id.to_string(),
                permissions: vec!["read".to_string(), "write".to_string()],
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_tenant() {
        let mut manager = TenantManager::new();

        let tenant = manager.create_tenant(CreateTenantRequest {
            name: "Acme Corp".to_string(),
            domain: "acme.ferro.example.com".to_string(),
            plan: TenantPlan::Enterprise,
        });

        assert_eq!(tenant.name, "Acme Corp");
        assert_eq!(tenant.domain, "acme.ferro.example.com");
        assert_eq!(tenant.plan, TenantPlan::Enterprise);
    }

    #[test]
    fn test_get_tenant() {
        let mut manager = TenantManager::new();

        let tenant = manager.create_tenant(CreateTenantRequest {
            name: "Acme Corp".to_string(),
            domain: "acme.ferro.example.com".to_string(),
            plan: TenantPlan::Enterprise,
        });

        let found = manager.get_tenant(&tenant.id);
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Acme Corp");
    }

    #[test]
    fn test_delete_tenant() {
        let mut manager = TenantManager::new();

        let tenant = manager.create_tenant(CreateTenantRequest {
            name: "Acme Corp".to_string(),
            domain: "acme.ferro.example.com".to_string(),
            plan: TenantPlan::Enterprise,
        });

        assert!(manager.delete_tenant(&tenant.id));
        assert!(manager.get_tenant(&tenant.id).is_none());
    }

    #[test]
    fn test_switch_tenant() {
        let mut manager = TenantManager::new();

        let tenant = manager.create_tenant(CreateTenantRequest {
            name: "Acme Corp".to_string(),
            domain: "acme.ferro.example.com".to_string(),
            plan: TenantPlan::Enterprise,
        });

        let context = manager.switch_tenant("user1", &tenant.id);
        assert!(context.is_some());
        assert_eq!(context.unwrap().tenant_id, tenant.id);
    }
}
