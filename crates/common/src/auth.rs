use serde::{Deserialize, Serialize};

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
