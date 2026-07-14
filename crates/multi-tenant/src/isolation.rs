use std::path::PathBuf;

use crate::error::TenantError;
use crate::organization::{OrganizationId, OrganizationRole};
use crate::tenant::TenantId;

#[derive(Debug, Clone)]
pub struct TenantContext {
    pub tenant_id: TenantId,
    pub organization_id: OrganizationId,
    pub user_id: String,
    pub role: OrganizationRole,
}

impl TenantContext {
    pub fn new(tenant_id: TenantId, organization_id: OrganizationId, user_id: String, role: OrganizationRole) -> Self {
        Self {
            tenant_id,
            organization_id,
            user_id,
            role,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TenantGuard {
    context: TenantContext,
}

impl TenantGuard {
    pub fn new(context: TenantContext) -> Self {
        Self { context }
    }

    pub fn can_access_tenant(&self, target_tenant_id: &str) -> bool {
        if self.context.tenant_id.0 == target_tenant_id {
            return true;
        }
        matches!(self.context.role, OrganizationRole::Owner | OrganizationRole::Admin)
    }

    pub fn context(&self) -> &TenantContext {
        &self.context
    }
}

#[allow(async_fn_in_trait)]
pub trait ResourceIsolation: Send + Sync {
    fn check_read_access(&self, ctx: &TenantContext, resource_path: &str) -> Result<(), TenantError>;
    fn check_write_access(&self, ctx: &TenantContext, resource_path: &str) -> Result<(), TenantError>;
    fn enforce_path_prefix(&self, ctx: &TenantContext, full_path: &str) -> Result<(), TenantError>;
}

pub struct DefaultResourceIsolation {
    tenant_storage_base: String,
}

impl DefaultResourceIsolation {
    pub fn new(tenant_storage_base: &str) -> Self {
        Self {
            tenant_storage_base: tenant_storage_base.to_string(),
        }
    }
}

impl ResourceIsolation for DefaultResourceIsolation {
    fn check_read_access(&self, ctx: &TenantContext, resource_path: &str) -> Result<(), TenantError> {
        if matches!(
            ctx.role,
            OrganizationRole::ReadOnly | OrganizationRole::Member | OrganizationRole::Admin | OrganizationRole::Owner
        ) {
            self.enforce_path_prefix(ctx, resource_path)?;
            Ok(())
        } else {
            Err(TenantError::PermissionDenied {
                user_id: ctx.user_id.clone(),
                resource: resource_path.to_string(),
                action: "read".to_string(),
            })
        }
    }

    fn check_write_access(&self, ctx: &TenantContext, resource_path: &str) -> Result<(), TenantError> {
        if matches!(
            ctx.role,
            OrganizationRole::Owner | OrganizationRole::Admin | OrganizationRole::Member
        ) {
            self.enforce_path_prefix(ctx, resource_path)?;
            Ok(())
        } else {
            Err(TenantError::PermissionDenied {
                user_id: ctx.user_id.clone(),
                resource: resource_path.to_string(),
                action: "write".to_string(),
            })
        }
    }

    fn enforce_path_prefix(&self, ctx: &TenantContext, full_path: &str) -> Result<(), TenantError> {
        let expected_prefix = format!(
            "{}/tenants/{}/{}",
            self.tenant_storage_base, ctx.organization_id.0, ctx.tenant_id.0
        );
        let canonical = match PathBuf::from(full_path).canonicalize() {
            Ok(p) => p.to_string_lossy().to_string(),
            Err(_) => full_path.to_string(),
        };

        let canonical = canonical.replace('\\', "/");
        let expected_prefix = expected_prefix.replace('\\', "/");

        if canonical.starts_with(&expected_prefix) {
            Ok(())
        } else {
            Err(TenantError::PermissionDenied {
                user_id: ctx.user_id.clone(),
                resource: full_path.to_string(),
                action: "access".to_string(),
            })
        }
    }
}

pub struct TenantPathResolver {
    storage_base: String,
}

impl TenantPathResolver {
    pub fn new(storage_base: &str) -> Self {
        Self {
            storage_base: storage_base.to_string(),
        }
    }

    pub fn resolve(&self, tenant_id: &str, relative_path: &str) -> Result<String, TenantError> {
        if relative_path.contains("..") {
            return Err(TenantError::PermissionDenied {
                user_id: "system".to_string(),
                resource: relative_path.to_string(),
                action: "path_traversal".to_string(),
            });
        }

        let base = format!("{}/tenants/{}", self.storage_base, tenant_id);
        let normalized = relative_path.trim_start_matches('/');
        let resolved = format!("{base}/{normalized}");

        Ok(resolved)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ctx(tenant_id: &str, org_id: &str, user_id: &str, role: OrganizationRole) -> TenantContext {
        TenantContext::new(
            TenantId(tenant_id.to_string()),
            OrganizationId(org_id.to_string()),
            user_id.to_string(),
            role,
        )
    }

    #[test]
    fn test_same_tenant_access() {
        let ctx = make_ctx("t1", "org-1", "user-1", OrganizationRole::Member);
        let guard = TenantGuard::new(ctx);
        assert!(guard.can_access_tenant("t1"));
    }

    #[test]
    fn test_cross_tenant_denied() {
        let ctx = make_ctx("t1", "org-1", "user-1", OrganizationRole::Member);
        let guard = TenantGuard::new(ctx);
        assert!(!guard.can_access_tenant("t2"));
    }

    #[test]
    fn test_org_admin_can_access() {
        let ctx = make_ctx("t1", "org-1", "user-1", OrganizationRole::Admin);
        let guard = TenantGuard::new(ctx);
        assert!(guard.can_access_tenant("t2"));
    }

    #[test]
    fn test_org_owner_can_access() {
        let ctx = make_ctx("t1", "org-1", "user-1", OrganizationRole::Owner);
        let guard = TenantGuard::new(ctx);
        assert!(guard.can_access_tenant("t99"));
    }

    #[test]
    fn test_readonly_member_denied() {
        let ctx = make_ctx("t1", "org-1", "user-1", OrganizationRole::ReadOnly);
        let guard = TenantGuard::new(ctx);
        assert!(!guard.can_access_tenant("t2"));
    }

    #[test]
    fn test_path_resolution() {
        let resolver = TenantPathResolver::new("/data");
        let path = resolver.resolve("t1", "docs/report.pdf").unwrap();
        assert_eq!(path, "/data/tenants/t1/docs/report.pdf");
    }

    #[test]
    fn test_path_resolution_leading_slash() {
        let resolver = TenantPathResolver::new("/data");
        let path = resolver.resolve("t1", "/docs/report.pdf").unwrap();
        assert_eq!(path, "/data/tenants/t1/docs/report.pdf");
    }

    #[test]
    fn test_path_traversal_blocked() {
        let resolver = TenantPathResolver::new("/data");
        let result = resolver.resolve("t1", "../other/file.txt");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(
            err,
            TenantError::PermissionDenied {
                user_id: "system".to_string(),
                resource: "../other/file.txt".to_string(),
                action: "path_traversal".to_string(),
            }
        );
    }

    #[test]
    fn test_path_traversal_mid_blocked() {
        let resolver = TenantPathResolver::new("/data");
        let result = resolver.resolve("t1", "docs/../../../etc/passwd");
        assert!(result.is_err());
    }

    #[test]
    fn test_check_read_access_readonly_allowed() {
        let isolation = DefaultResourceIsolation::new("/data");
        let ctx = make_ctx("t1", "org-1", "user-1", OrganizationRole::ReadOnly);
        let result = isolation.check_read_access(&ctx, "/data/tenants/org-1/t1/file.txt");
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_write_access_readonly_denied() {
        let isolation = DefaultResourceIsolation::new("/data");
        let ctx = make_ctx("t1", "org-1", "user-1", OrganizationRole::ReadOnly);
        let result = isolation.check_write_access(&ctx, "/data/tenants/org-1/t1/file.txt");
        assert!(result.is_err());
    }

    #[test]
    fn test_enforce_path_prefix_wrong_tenant() {
        let isolation = DefaultResourceIsolation::new("/data");
        let ctx = make_ctx("t1", "org-1", "user-1", OrganizationRole::Member);
        let result = isolation.enforce_path_prefix(&ctx, "/data/tenants/org-1/t2/file.txt");
        assert!(result.is_err());
    }
}
