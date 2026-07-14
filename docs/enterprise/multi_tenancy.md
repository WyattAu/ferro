# Multi-Tenancy

## Overview

Ferro supports multi-tenant deployments for enterprise customers.

## Tenant Isolation

### Database Isolation
- Separate SQLite database per tenant
- Shared PostgreSQL with schema isolation
- Row-level security

### Storage Isolation
- Separate storage directory per tenant
- Shared S3 with prefix isolation
- Access control policies

### Network Isolation
- Tenant-specific subdomains
- Custom domains
- SSL certificates per tenant

## Configuration

```toml
[tenancy]
enabled = true
isolation_level = "schema"  # "database", "schema", "row"
default_tenant = "default"

[tenancy.schema]
prefix = "tenant_"
```

## API Endpoints

### Tenant Management
```http
POST /api/tenants
{
  "name": "Acme Corp",
  "domain": "acme.ferro.example.com",
  "plan": "enterprise"
}

GET /api/tenants/{tenant_id}

PUT /api/tenants/{tenant_id}

DELETE /api/tenants/{tenant_id}
```

### Tenant Context
```http
GET /api/tenant
# Returns current tenant info

POST /api/tenant/switch
{
  "tenant_id": "tenant123"
}
```

## Implementation

### Tenant Types

```rust
pub struct Tenant {
    pub id: String,
    pub name: String,
    pub domain: String,
    pub plan: TenantPlan,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub enum TenantPlan {
    Free,
    Pro,
    Enterprise,
}

pub struct TenantContext {
    pub tenant_id: String,
    pub user_id: String,
    pub permissions: Vec<String>,
}
```

### Tenant Manager

```rust
pub struct TenantManager {
    db: Database,
}

impl TenantManager {
    pub async fn create_tenant(&self, tenant: CreateTenantRequest) -> Result<Tenant, TenantError> {
        // Create tenant
        // Create schema/database
        // Initialize storage
        // Create admin user
        todo!()
    }

    pub async fn get_tenant(&self, tenant_id: &str) -> Result<Tenant, TenantError> {
        todo!()
    }

    pub async fn list_tenants(&self) -> Result<Vec<Tenant>, TenantError> {
        todo!()
    }

    pub async fn delete_tenant(&self, tenant_id: &str) -> Result<(), TenantError> {
        todo!()
    }

    pub async fn switch_tenant(&self, user_id: &str, tenant_id: &str) -> Result<TenantContext, TenantError> {
        todo!()
    }
}
```
