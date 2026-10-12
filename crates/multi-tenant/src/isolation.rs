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

    /// Canonical storage layout (DC-TENANT-002): `{base}/tenants/{tenant_id}`.
    /// The organization is a policy fact carried in `TenantContext`, never a
    /// path segment — mixing the two made the layout ambiguous between
    /// `TenantPathResolver` (org-less) and this guard (org-ful).
    fn expected_prefix(&self, ctx: &TenantContext) -> String {
        format!(
            "{}/tenants/{}",
            self.tenant_storage_base.trim_end_matches('/'),
            ctx.tenant_id.0
        )
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
        let expected_prefix = normalize_slashes(&self.expected_prefix(ctx));
        let expected_prefix = expected_prefix.trim_end_matches('/').to_string();

        // Canonicalize when the target exists — this resolves symlinks and is
        // the only way to catch a symlink escaping the tenant subtree.
        // When it fails (path not yet created — the normal pre-create check),
        // fall back to LEXICAL normalization instead of trusting the raw
        // string: `..` components are collapsed before the prefix compare, so
        // traversal can never smuggle through the fallback.
        // (DC-TENANT-001: the check fails CLOSED — no raw-path bypass, no
        // blanket denial of not-yet-existing files.)
        let canonical = match PathBuf::from(full_path).canonicalize() {
            Ok(p) => normalize_slashes(&p.to_string_lossy()),
            Err(_) => lexical_normalize(full_path),
        };

        if path_under_prefix(&canonical, &expected_prefix) {
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

fn normalize_slashes(p: &str) -> String {
    p.replace('\\', "/")
}

/// Collapse `.` and `..` components textually (no filesystem access).
fn lexical_normalize(path: &str) -> String {
    let mut components: Vec<&str> = Vec::new();
    for comp in path.split('/') {
        match comp {
            "" | "." => {}
            ".." => {
                components.pop();
            }
            c => components.push(c),
        }
    }
    let mut out = String::new();
    for c in &components {
        out.push('/');
        out.push_str(c);
    }
    if out.is_empty() {
        out.push('/');
    }
    out
}

/// Prefix containment WITH a boundary check (THM-PREFIX-CONTAINMENT).
/// `canonical.starts_with(prefix)` alone is unsound: tenant `t1` would match
/// tenant `t10`'s paths. The next character after the prefix must be `/`
/// (or the paths must be equal).
fn path_under_prefix(canonical: &str, prefix: &str) -> bool {
    if canonical == prefix {
        return true;
    }
    canonical.starts_with(prefix) && canonical.as_bytes().get(prefix.len()) == Some(&b'/')
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

    /// Resolve a tenant-relative path to its canonical storage location
    /// (DC-TENANT-002): `{base}/tenants/{tenant_id}/{relative}` — the SAME
    /// layout `DefaultResourceIsolation::enforce_path_prefix` expects.
    pub fn resolve(&self, tenant_id: &str, relative_path: &str) -> Result<String, TenantError> {
        if relative_path.contains("..") {
            return Err(TenantError::PermissionDenied {
                user_id: "system".to_string(),
                resource: relative_path.to_string(),
                action: "path_traversal".to_string(),
            });
        }

        let base = format!("{}/tenants/{}", self.storage_base.trim_end_matches('/'), tenant_id);
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
        let result = isolation.check_read_access(&ctx, "/data/tenants/t1/file.txt");
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_write_access_readonly_denied() {
        let isolation = DefaultResourceIsolation::new("/data");
        let ctx = make_ctx("t1", "org-1", "user-1", OrganizationRole::ReadOnly);
        let result = isolation.check_write_access(&ctx, "/data/tenants/t1/file.txt");
        assert!(result.is_err());
    }

    #[test]
    fn test_enforce_path_prefix_wrong_tenant() {
        let isolation = DefaultResourceIsolation::new("/data");
        let ctx = make_ctx("t1", "org-1", "user-1", OrganizationRole::Member);
        let result = isolation.enforce_path_prefix(&ctx, "/data/tenants/t2/file.txt");
        assert!(result.is_err());
    }

    /// TV-TENANT-001a: the prefix-boundary regression. Tenant `t1` must NOT
    /// match tenant `t10`'s paths — `starts_with` without a boundary check
    /// allowed exactly that.
    #[test]
    fn test_prefix_boundary_t1_vs_t10() {
        let isolation = DefaultResourceIsolation::new("/data");
        let ctx = make_ctx("t1", "org-1", "user-1", OrganizationRole::Member);
        // t10's path must be denied for t1's context, even though the path
        // does not exist (lexical fallback path).
        let result = isolation.enforce_path_prefix(&ctx, "/data/tenants/t10/file.txt");
        assert!(result.is_err(), "t1 must not access t10 via prefix collision");

        // And the reverse direction: t10's context cannot use t1's files.
        let ctx10 = make_ctx("t10", "org-1", "user-2", OrganizationRole::Member);
        let result = isolation.enforce_path_prefix(&ctx10, "/data/tenants/t1/file.txt");
        assert!(result.is_err());
    }

    /// TV-TENANT-001b (THM-PREFIX-CONTAINMENT): `..` traversal in the
    /// full path is collapsed lexically before the prefix compare — the
    /// fallback for not-yet-existing paths can never smuggle traversal.
    #[test]
    fn test_traversal_via_dotdot_denied() {
        let isolation = DefaultResourceIsolation::new("/data");
        let ctx = make_ctx("t1", "org-1", "user-1", OrganizationRole::Member);
        // Resolves lexically to /data/tenants/t2/file.txt — outside t1.
        let result = isolation.enforce_path_prefix(&ctx, "/data/tenants/t1/../t2/file.txt");
        assert!(result.is_err());
    }

    /// TV-TENANT-001c: a symlink escaping the tenant subtree is caught by
    /// canonicalization when the FULL path exists (the normal case for reads
    /// of existing content). Known limitation (DC-TENANT-003): a symlink whose
    /// final component does not yet exist falls back to lexical checks, which
    /// cannot resolve intermediate symlinks — the kernel-level fix is
    /// openat2 RESOLVE_BENEATH in worker paths.
    #[cfg(unix)]
    #[test]
    fn test_symlink_escape_denied() {
        let base = tempfile::TempDir::new().unwrap();
        let base_path = base.path().to_string_lossy().to_string();
        let tenant_dir = base.path().join("tenants/t1");
        std::fs::create_dir_all(&tenant_dir).unwrap();
        let outside = base.path().join("outside");
        std::fs::create_dir_all(&outside).unwrap();
        // The escape target must exist for canonicalize to resolve through
        // the symlink.
        std::fs::write(outside.join("secret.txt"), b"secret").unwrap();
        std::os::unix::fs::symlink(&outside, tenant_dir.join("escape")).unwrap();

        let isolation = DefaultResourceIsolation::new(&base_path);
        let ctx = make_ctx("t1", "org-1", "user-1", OrganizationRole::Member);
        let escape_path = format!("{}/tenants/t1/escape/secret.txt", base_path);
        let result = isolation.enforce_path_prefix(&ctx, &escape_path);
        assert!(result.is_err(), "symlink escape must be denied");
    }

    /// Not-yet-existing files inside the tenant subtree remain creatable:
    /// the lexical fallback must not blanket-deny pre-create checks.
    #[test]
    fn test_new_file_inside_tenant_allowed() {
        let isolation = DefaultResourceIsolation::new("/data");
        let ctx = make_ctx("t1", "org-1", "user-1", OrganizationRole::Member);
        let result = isolation.check_write_access(&ctx, "/data/tenants/t1/docs/new-file.pdf");
        assert!(result.is_ok());
    }
}
