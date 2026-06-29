use serde::{Deserialize, Serialize};

/// Canonical set of paths that are publicly accessible without authentication.
///
/// Merges the public path lists from `simple_auth`, `cedar`, and `oidc` modules
/// into a single source of truth.
pub fn is_public_auth_path(path: &str) -> bool {
    path == "/healthz"
        || path == "/.well-known/ferro"
        || path == "/.well-known/openid-configuration"
        || path.starts_with("/api/auth/login")
        || path.starts_with("/api/auth/callback")
        || path.starts_with("/api/config")
        || path.starts_with("/api/auth/info")
        || path == "/metrics"
        || path.starts_with("/ui/")
        || path == "/ui"
        || path == "/api/policies"
        || path.starts_with("/s/")
        || path.starts_with("/fed/")
        || path == "/.well-known/webfinger"
        || path.starts_with("/.well-known/webfinger")
}

/// JWT claims extracted from an OIDC/Basic auth token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject — unique identifier for the authenticated user.
    pub sub: String,
    /// Audience — intended recipient of the token.
    pub aud: String,
    /// Issuer — authority that issued the token.
    pub iss: String,
    /// Expiration time as a Unix timestamp.
    pub exp: u64,
    /// Issued-at time as a Unix timestamp.
    pub iat: u64,
    /// OIDC nonce used for replay protection.
    pub nonce: Option<String>,
    /// User's email address.
    pub email: Option<String>,
    /// User's display name.
    pub name: Option<String>,
    /// Groups or roles the user belongs to.
    pub groups: Option<Vec<String>>,
}

impl Claims {
    /// Create anonymous claims for unauthenticated requests.
    pub fn anonymous() -> Self {
        Self {
            sub: "anonymous".to_string(),
            aud: "ferro".to_string(),
            iss: "ferro".to_string(),
            exp: 0,
            iat: 0,
            nonce: None,
            email: None,
            name: None,
            groups: None,
        }
    }
}

/// Result of an authorization policy evaluation.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthDecision {
    /// The request is allowed, optionally with the matching policy ID.
    Allow { policy_id: Option<String> },
    /// The request is denied, with a human-readable reason.
    Deny { reason: String },
}

/// Authorization request submitted to the policy engine.
#[derive(Debug, Clone)]
pub struct AuthRequest {
    /// Principal performing the action (typically a user ID).
    pub principal: String,
    /// Action being performed (e.g. "read", "write").
    pub action: String,
    /// Resource being accessed (e.g. a file path).
    pub resource: String,
    /// Additional context for the authorization decision.
    pub context: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_public_auth_path_healthz() {
        assert!(is_public_auth_path("/healthz"));
        assert!(is_public_auth_path("/.well-known/ferro"));
        assert!(is_public_auth_path("/.well-known/openid-configuration"));
    }

    #[test]
    fn test_is_public_auth_path_auth() {
        assert!(is_public_auth_path("/api/auth/login"));
        assert!(is_public_auth_path("/api/auth/callback"));
        assert!(is_public_auth_path("/api/auth/callback?code=abc"));
        assert!(is_public_auth_path("/api/auth/info"));
    }

    #[test]
    fn test_is_public_auth_path_config() {
        assert!(is_public_auth_path("/api/config"));
        assert!(is_public_auth_path("/api/config?foo=bar"));
    }

    #[test]
    fn test_is_public_auth_path_ui() {
        assert!(is_public_auth_path("/ui"));
        assert!(is_public_auth_path("/ui/"));
        assert!(is_public_auth_path("/ui/index.html"));
    }

    #[test]
    fn test_is_public_auth_path_shares() {
        assert!(is_public_auth_path("/s/abc123"));
    }

    #[test]
    fn test_is_public_auth_path_policies() {
        assert!(is_public_auth_path("/api/policies"));
    }

    #[test]
    fn test_is_public_auth_path_not_public() {
        assert!(!is_public_auth_path("/api/shares"));
        assert!(!is_public_auth_path("/api/upload-url"));
        assert!(!is_public_auth_path("/files/test.txt"));
        assert!(!is_public_auth_path("/api/audit"));
        assert!(!is_public_auth_path("/api/snapshots"));
        assert!(!is_public_auth_path("/"));
        assert!(!is_public_auth_path("/wopi/files/test.txt"));
        assert!(!is_public_auth_path("/api/admin/stats"));
    }

    #[test]
    fn test_claims_anonymous() {
        let c = Claims::anonymous();
        assert_eq!(c.sub, "anonymous");
        assert_eq!(c.aud, "ferro");
        assert_eq!(c.iss, "ferro");
        assert_eq!(c.exp, 0);
        assert_eq!(c.iat, 0);
        assert!(c.nonce.is_none());
        assert!(c.email.is_none());
        assert!(c.name.is_none());
        assert!(c.groups.is_none());
    }

    #[test]
    fn test_claims_serialization() {
        let c = Claims::anonymous();
        let json = serde_json::to_string(&c).unwrap();
        let deserialized: Claims = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.sub, c.sub);
    }

    #[test]
    fn test_auth_decision_variants() {
        let allow = AuthDecision::Allow {
            policy_id: Some("p1".into()),
        };
        let deny = AuthDecision::Deny {
            reason: "no access".into(),
        };
        assert_eq!(
            allow,
            AuthDecision::Allow {
                policy_id: Some("p1".into())
            }
        );
        assert_ne!(allow, deny);
    }

    #[test]
    fn test_auth_request() {
        let req = AuthRequest {
            principal: "alice".into(),
            action: "read".into(),
            resource: "/file.txt".into(),
            context: serde_json::json!({"ip": "127.0.0.1"}),
        };
        assert_eq!(req.principal, "alice");
    }
}
