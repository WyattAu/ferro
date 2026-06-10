//! Role-Based Access Control (RBAC) presets for the Cedar policy engine.
//!
//! Provides pre-built Cedar policies for common role assignments:
//! Admin, User, ReadOnly. These presets generate Cedar policy text that
//! can be loaded via [`CedarAuthorizer::load_policies`].
//!
//! Roles map directly to [`crate::users::UserRole`] but provide a
//! Cedar-compatible policy surface for organizations that prefer
//! role-based access control over fine-grained Cedar policies.

use crate::users::UserRole;
use serde::{Deserialize, Serialize};

/// A role preset with a human-readable name and Cedar policy.
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RolePreset {
    /// Role identifier (matches `UserRole` name).
    pub id: String,
    /// Human-readable display name.
    pub name: String,
    /// Description of what this role permits.
    pub description: String,
    /// Whether this is a system-defined role (cannot be deleted).
    pub is_system: bool,
}

/// Pre-defined system roles. Cannot be modified or deleted by users.
pub fn system_roles() -> Vec<RolePreset> {
    vec![
        RolePreset {
            id: "Admin".into(),
            name: "Administrator".into(),
            description: "Full access to all resources and administrative operations.".into(),
            is_system: true,
        },
        RolePreset {
            id: "User".into(),
            name: "Standard User".into(),
            description: "Read and write access to own resources. No admin operations.".into(),
            is_system: true,
        },
        RolePreset {
            id: "ReadOnly".into(),
            name: "Read-Only".into(),
            description: "Read and list access only. No write or delete operations.".into(),
            is_system: true,
        },
    ]
}

/// Generate a Cedar policy set that enforces the given role hierarchy.
///
/// This creates a restrictive baseline: actions are only permitted if
/// explicitly allowed by the user's role. If no roles are specified,
/// returns a permissive default (all actions allowed for all principals).
///
/// The generated policies use entity types:
/// - `User` (principal)
/// - `Action` (action: "read", "write", "delete", "list", "admin")
/// - `File` (resource)
///
/// # Arguments
/// * `roles` - Map of user ID -> role assignment
pub fn generate_role_policies(roles: &[(String, UserRole)]) -> String {
    if roles.is_empty() {
        // No role assignments = permissive default
        return r#"
@id("role_default_permit")
permit (
    principal,
    action in [Action::"read", Action::"write", Action::"delete", Action::"list", Action::"admin"],
    resource
);
"#
        .to_string();
    }

    let mut policies = Vec::new();

    for (uid, role) in roles {
        let policy = generate_single_role_policy(uid, role);
        policies.push(policy);
    }

    policies.join("\n")
}

/// Generate a single Cedar policy for a specific user-role assignment.
///
/// Useful for incremental policy updates without rebuilding the entire set.
pub fn generate_single_role_policy(user_id: &str, role: &UserRole) -> String {
    let safe_id = user_id.replace('"', "_");
    match role {
        UserRole::Admin => format!(
            r#"
@id("role_{}_admin")
permit (
    principal == User::"{uid}",
    action in [Action::"read", Action::"write", Action::"delete", Action::"list", Action::"admin"],
    resource
);
"#,
            safe_id,
            uid = user_id
        ),
        UserRole::User => format!(
            r#"
@id("role_{}_user")
permit (
    principal == User::"{uid}",
    action in [Action::"read", Action::"write", Action::"delete", Action::"list"],
    resource
);
"#,
            safe_id,
            uid = user_id
        ),
        UserRole::ReadOnly => format!(
            r#"
@id("role_{}_readonly")
permit (
    principal == User::"{uid}",
    action in [Action::"read", Action::"list"],
    resource
);
"#,
            safe_id,
            uid = user_id
        ),
    }
}

/// Get the system role preset for a given UserRole.
pub fn role_preset_for(role: &UserRole) -> RolePreset {
    let roles = system_roles();
    let role_id = match role {
        UserRole::Admin => "Admin",
        UserRole::User => "User",
        UserRole::ReadOnly => "ReadOnly",
    };
    roles
        .into_iter()
        .find(|r| r.id == role_id)
        .ok_or_else(|| format!("system role '{role_id}' not found"))
        .expect("system roles must include Admin, User, and ReadOnly presets")
}

/// Parse a role name string into a UserRole.
///
/// Case-insensitive. Returns `None` for unrecognized role names.
pub fn parse_role_name(name: &str) -> Option<UserRole> {
    match name.to_lowercase().as_str() {
        "admin" | "administrator" => Some(UserRole::Admin),
        "user" | "standard" | "member" => Some(UserRole::User),
        "readonly" | "read-only" | "guest" => Some(UserRole::ReadOnly),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_roles_defined() {
        let roles = system_roles();
        assert_eq!(roles.len(), 3);
        assert_eq!(roles[0].id, "Admin");
        assert_eq!(roles[1].id, "User");
        assert_eq!(roles[2].id, "ReadOnly");
        assert!(roles.iter().all(|r| r.is_system));
    }

    #[test]
    fn test_generate_empty_roles_permissive() {
        let policy = generate_role_policies(&[]);
        assert!(policy.contains("role_default_permit"));
        assert!(policy.contains("Action::\"admin\""));
    }

    #[test]
    fn test_generate_admin_role() {
        let policy = generate_role_policies(&[("alice".into(), UserRole::Admin)]);
        assert!(policy.contains("User::\"alice\""));
        assert!(policy.contains("Action::\"admin\""));
    }

    #[test]
    fn test_generate_user_role_no_admin() {
        let policy = generate_role_policies(&[("bob".into(), UserRole::User)]);
        assert!(policy.contains("User::\"bob\""));
        assert!(policy.contains("Action::\"read\""));
        assert!(policy.contains("Action::\"write\""));
        assert!(!policy.contains("Action::\"admin\""));
    }

    #[test]
    fn test_generate_readonly_role_limited() {
        let policy = generate_role_policies(&[("charlie".into(), UserRole::ReadOnly)]);
        assert!(policy.contains("Action::\"read\""));
        assert!(policy.contains("Action::\"list\""));
        assert!(!policy.contains("Action::\"write\""));
        assert!(!policy.contains("Action::\"delete\""));
    }

    #[test]
    fn test_generate_mixed_roles() {
        let policy = generate_role_policies(&[
            ("alice".into(), UserRole::Admin),
            ("bob".into(), UserRole::User),
            ("charlie".into(), UserRole::ReadOnly),
        ]);
        assert!(policy.contains("User::\"alice\""));
        assert!(policy.contains("User::\"bob\""));
        assert!(policy.contains("User::\"charlie\""));
    }

    #[test]
    fn test_single_role_policy_admin() {
        let policy = generate_single_role_policy("alice", &UserRole::Admin);
        assert!(policy.contains("User::\"alice\""));
        assert!(policy.contains("Action::\"admin\""));
    }

    #[test]
    fn test_single_role_policy_user() {
        let policy = generate_single_role_policy("bob", &UserRole::User);
        assert!(policy.contains("User::\"bob\""));
        assert!(!policy.contains("Action::\"admin\""));
    }

    #[test]
    fn test_single_role_policy_readonly() {
        let policy = generate_single_role_policy("guest", &UserRole::ReadOnly);
        assert!(!policy.contains("Action::\"write\""));
    }

    #[test]
    fn test_role_preset_for() {
        let admin = role_preset_for(&UserRole::Admin);
        assert_eq!(admin.id, "Admin");
        let user = role_preset_for(&UserRole::User);
        assert_eq!(user.id, "User");
        let ro = role_preset_for(&UserRole::ReadOnly);
        assert_eq!(ro.id, "ReadOnly");
    }

    #[test]
    fn test_role_preset_serialization() {
        let role = &system_roles()[0];
        let json = serde_json::to_string(role).unwrap();
        let deser: RolePreset = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.id, role.id);
        assert!(deser.is_system);
    }

    #[test]
    fn test_parse_role_name() {
        assert!(matches!(parse_role_name("admin"), Some(UserRole::Admin)));
        assert!(matches!(
            parse_role_name("Administrator"),
            Some(UserRole::Admin)
        ));
        assert!(matches!(parse_role_name("user"), Some(UserRole::User)));
        assert!(matches!(parse_role_name("Standard"), Some(UserRole::User)));
        assert!(matches!(
            parse_role_name("readonly"),
            Some(UserRole::ReadOnly)
        ));
        assert!(matches!(
            parse_role_name("Read-Only"),
            Some(UserRole::ReadOnly)
        ));
        assert!(matches!(parse_role_name("Guest"), Some(UserRole::ReadOnly)));
        assert!(parse_role_name("nonexistent").is_none());
    }

    #[test]
    fn test_generated_policy_parses_as_cedar() {
        let policy = generate_role_policies(&[
            ("alice".into(), UserRole::Admin),
            ("bob".into(), UserRole::User),
        ]);
        let ps: cedar_policy::PolicySet = policy
            .parse()
            .expect("Generated policy must parse as valid Cedar");
        assert!(ps.policies().count() >= 2);
    }
}
