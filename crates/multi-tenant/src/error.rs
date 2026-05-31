use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TenantError {
    NotFound { tenant_id: String },
    OrganizationNotFound { org_id: String },
    QuotaExceeded {
        tenant_id: String,
        quota: u64,
        current: u64,
    },
    AlreadyExists { name: String },
    PermissionDenied {
        user_id: String,
        resource: String,
        action: String,
    },
    Suspended { tenant_id: String },
}

impl fmt::Display for TenantError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound { tenant_id } => write!(f, "tenant not found: {tenant_id}"),
            Self::OrganizationNotFound { org_id } => {
                write!(f, "organization not found: {org_id}")
            }
            Self::QuotaExceeded {
                tenant_id,
                quota,
                current,
            } => {
                write!(
                    f,
                    "quota exceeded for tenant {tenant_id}: {current}/{quota} bytes"
                )
            }
            Self::AlreadyExists { name } => write!(f, "resource already exists: {name}"),
            Self::PermissionDenied {
                user_id,
                resource,
                action,
            } => {
                write!(
                    f,
                    "permission denied: user {user_id} cannot {action} on {resource}"
                )
            }
            Self::Suspended { tenant_id } => {
                write!(f, "tenant is suspended: {tenant_id}")
            }
        }
    }
}

impl std::error::Error for TenantError {}
