use serde::{Deserialize, Serialize};

/// JWT claims extracted from an OIDC/Basic auth token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub aud: String,
    pub iss: String,
    pub exp: u64,
    pub iat: u64,
    pub nonce: Option<String>,
    pub email: Option<String>,
    pub name: Option<String>,
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthDecision {
    Allow { policy_id: Option<String> },
    Deny { reason: String },
}

/// Authorization request submitted to the policy engine.
#[derive(Debug, Clone)]
pub struct AuthRequest {
    pub principal: String,
    pub action: String,
    pub resource: String,
    pub context: serde_json::Value,
}
