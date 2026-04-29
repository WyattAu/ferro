use axum::response::IntoResponse;
use cedar_policy::{Authorizer, Context, Decision, Entities, EntityUid, PolicySet, Request};
use common::auth::{AuthDecision, AuthRequest, Claims};
use common::error::{FerroError, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, warn};

const FALLBACK_PRINCIPAL: &str = r#"User::"anonymous""#;
const FALLBACK_ACTION: &str = r#"Action::"unknown""#;
const FALLBACK_RESOURCE: &str = r#"File::"unknown""#;

fn fallback_principal() -> EntityUid {
    FALLBACK_PRINCIPAL
        .parse()
        .expect("hardcoded fallback EntityUid must parse")
}

fn fallback_action() -> EntityUid {
    FALLBACK_ACTION
        .parse()
        .expect("hardcoded fallback EntityUid must parse")
}

fn fallback_resource() -> EntityUid {
    FALLBACK_RESOURCE
        .parse()
        .expect("hardcoded fallback EntityUid must parse")
}

/// Cedar policy configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CedarConfig {
    pub default_policy: String,
}

/// Cedar-based authorization engine.
#[derive(Clone)]
pub struct CedarAuthorizer {
    policy_set: Arc<RwLock<PolicySet>>,
}

impl CedarAuthorizer {
    /// Create a new Cedar authorizer with a default permissive policy.
    pub fn new() -> Result<Self> {
        let default_policy = r#"
            @id("all_access")
            permit (
                principal,
                action in [Action::"read", Action::"write", Action::"delete", Action::"list", Action::"admin"],
                resource
            );
        "#;

        let policy_set: PolicySet = default_policy
            .parse()
            .map_err(|e| FerroError::Internal(format!("Policy parse error: {:?}", e)))?;

        Ok(Self {
            policy_set: Arc::new(RwLock::new(policy_set)),
        })
    }

    pub async fn load_policies(&self, policies: &[String]) -> Result<()> {
        let mut policy_set = PolicySet::new();
        for (i, policy_text) in policies.iter().enumerate() {
            let ps: PolicySet = policy_text
                .parse()
                .map_err(|e| FerroError::Internal(format!("Policy {} parse error: {:?}", i, e)))?;
            for policy in ps.policies() {
                policy_set.add(policy.clone()).map_err(|e| {
                    FerroError::Internal(format!("Add policy {} error: {:?}", i, e))
                })?;
            }
        }

        let mut guard = self.policy_set.write().await;
        *guard = policy_set;

        debug!("Loaded {} Cedar policies", policies.len());
        Ok(())
    }

    pub async fn add_policy(&self, policy_text: &str) -> Result<()> {
        let ps: PolicySet = policy_text
            .parse()
            .map_err(|e| FerroError::Internal(format!("Policy parse error: {:?}", e)))?;

        let mut guard = self.policy_set.write().await;
        for policy in ps.policies() {
            guard
                .add(policy.clone())
                .map_err(|e| FerroError::Internal(format!("Add policy error: {:?}", e)))?;
        }

        debug!("Added Cedar policy");
        Ok(())
    }

    pub async fn is_authorized(&self, request: &AuthRequest) -> Result<AuthDecision> {
        let guard = self.policy_set.read().await;

        let principal: EntityUid = match format!("User::\"{}\"", request.principal).parse() {
            Ok(uid) => uid,
            Err(e) => {
                warn!("Failed to parse principal EntityUid for {:?}: {:?}", request.principal, e);
                fallback_principal()
            }
        };

        let action: EntityUid = match format!("Action::\"{}\"", request.action).parse() {
            Ok(uid) => uid,
            Err(e) => {
                warn!("Failed to parse action EntityUid for {:?}: {:?}", request.action, e);
                fallback_action()
            }
        };

        let resource: EntityUid = match format!("File::\"{}\"", request.resource).parse() {
            Ok(uid) => uid,
            Err(e) => {
                warn!("Failed to parse resource EntityUid for {:?}: {:?}", request.resource, e);
                fallback_resource()
            }
        };

        // Build context from request attributes
        // Cedar Context supports building from JSON values for string/bool/long attributes
        let context = Context::empty();

        let q = Request::new(principal, action, resource, context, None)
            .map_err(|e| FerroError::Internal(format!("Request creation error: {:?}", e)))?;

        let entities = Entities::empty();
        let authorizer = Authorizer::new();
        let response = authorizer.is_authorized(&q, &guard, &entities);

        let reasons: Vec<String> = response
            .diagnostics()
            .reason()
            .map(|r| r.to_string())
            .collect();

        let decision = match response.decision() {
            Decision::Allow => AuthDecision::Allow {
                policy_id: reasons.first().cloned(),
            },
            Decision::Deny => AuthDecision::Deny {
                reason: reasons.join(", "),
            },
        };

        debug!(
            "Cedar decision: {:?} for {} {} {}",
            decision, request.principal, request.action, request.resource
        );

        Ok(decision)
    }

    pub async fn is_authorized_simple(
        &self,
        principal: &str,
        action: &str,
        resource: &str,
    ) -> Result<bool> {
        let request = AuthRequest {
            principal: principal.to_string(),
            action: action.to_string(),
            resource: resource.to_string(),
            context: serde_json::Value::Null,
        };

        match self.is_authorized(&request).await? {
            AuthDecision::Allow { .. } => Ok(true),
            AuthDecision::Deny { .. } => Ok(false),
        }
    }
}

/// Map an HTTP method to a Cedar action string.
fn http_method_to_action(method: &axum::http::Method) -> &'static str {
    match *method {
        axum::http::Method::GET | axum::http::Method::HEAD => "read",
        axum::http::Method::DELETE => "delete",
        axum::http::Method::PUT | axum::http::Method::POST | axum::http::Method::PATCH => "write",
        _ => {
            // WebDAV methods are represented as custom Method variants.
            // Match by the method as-str to handle PROPFIND, MKCOL, COPY, MOVE, etc.
            match method.as_str() {
                "PROPFIND" => "list",
                "LOCK" | "UNLOCK" => "admin",
                _ => "write",
            }
        }
    }
}

/// Paths that skip Cedar authorization (in addition to OIDC public paths).
fn is_cedar_exempt_path(path: &str) -> bool {
    // Public API endpoints that should always be accessible
    path == "/.well-known/ferro"
        || path == "/.well-known/openid-configuration"
        || path.starts_with("/api/auth/login")
        // Policy management must remain accessible even under restrictive policies
        || path == "/api/policies"
        // Health and config endpoints
        || path == "/api/config"
        // Shared link access — public by design
        || path.starts_with("/s/")
}

/// Axum middleware that enforces Cedar authorization on every request.
///
/// This middleware MUST run AFTER the OIDC auth middleware, so that `Claims`
/// are always available as a request extension.
///
/// If Cedar is not configured (None), all requests pass through.
/// If Cedar is configured, it checks `is_authorized(principal, action, resource)`.
/// Denial returns HTTP 403 Forbidden.
pub async fn cedar_middleware(
    cedar: Option<Arc<CedarAuthorizer>>,
    request: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let path = request.uri().path();

    // Skip authorization for exempt paths
    if is_cedar_exempt_path(path) {
        return next.run(request).await;
    }

    // If Cedar is not configured, pass through
    let authorizer = match cedar {
        Some(a) => a,
        None => return next.run(request).await,
    };

    // Extract claims (inserted by OIDC middleware or anonymous)
    let claims = request
        .extensions()
        .get::<Claims>()
        .cloned()
        .unwrap_or_else(Claims::anonymous);

    let action = http_method_to_action(request.method());
    let resource = path;

    match authorizer
        .is_authorized_simple(&claims.sub, action, resource)
        .await
    {
        Ok(true) => next.run(request).await,
        Ok(false) => {
            warn!(
                "Cedar denied: {} {} {} by {}",
                request.method(),
                resource,
                action,
                claims.sub
            );
            (axum::http::StatusCode::FORBIDDEN, "Forbidden by policy").into_response()
        }
        Err(e) => {
            warn!("Cedar authorization error: {}", e);
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Authorization error",
            )
                .into_response()
        }
    }
}

impl Default for CedarAuthorizer {
    fn default() -> Self {
        Self::new().expect("default CedarAuthorizer creation must succeed")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_default_policy_allows_everything() {
        let authorizer = CedarAuthorizer::new().unwrap();
        assert!(
            authorizer
                .is_authorized_simple("alice", "read", "/file.txt")
                .await
                .unwrap()
        );
        assert!(
            authorizer
                .is_authorized_simple("alice", "write", "/file.txt")
                .await
                .unwrap()
        );
        assert!(
            authorizer
                .is_authorized_simple("alice", "delete", "/file.txt")
                .await
                .unwrap()
        );
        assert!(
            authorizer
                .is_authorized_simple("alice", "list", "/")
                .await
                .unwrap()
        );
        assert!(
            authorizer
                .is_authorized_simple("alice", "admin", "/file.txt")
                .await
                .unwrap()
        );
        assert!(
            authorizer
                .is_authorized_simple("anonymous", "read", "/file.txt")
                .await
                .unwrap()
        );
    }

    #[tokio::test]
    async fn test_restrictive_policy() {
        let authorizer = CedarAuthorizer::new().unwrap();
        // Replace all policies with a restrictive one: only alice can read
        let restrictive = r#"
            @id("alice_only")
            permit (
                principal == User::"alice",
                action in Action::"read",
                resource
            );
        "#;
        authorizer
            .load_policies(&[restrictive.to_string()])
            .await
            .unwrap();

        // alice can read (explicit permit)
        assert!(
            authorizer
                .is_authorized_simple("alice", "read", "/file.txt")
                .await
                .unwrap()
        );
        // bob is denied (no matching permit)
        assert!(
            !authorizer
                .is_authorized_simple("bob", "read", "/file.txt")
                .await
                .unwrap()
        );
        // alice can't write (no permit for write)
        assert!(
            !authorizer
                .is_authorized_simple("alice", "write", "/file.txt")
                .await
                .unwrap()
        );
        // anonymous is denied
        assert!(
            !authorizer
                .is_authorized_simple("anonymous", "read", "/file.txt")
                .await
                .unwrap()
        );
    }

    #[test]
    fn test_http_method_to_action() {
        assert_eq!(http_method_to_action(&axum::http::Method::GET), "read");
        assert_eq!(http_method_to_action(&axum::http::Method::HEAD), "read");
        assert_eq!(http_method_to_action(&axum::http::Method::PUT), "write");
        assert_eq!(http_method_to_action(&axum::http::Method::DELETE), "delete");
        assert_eq!(http_method_to_action(&axum::http::Method::POST), "write");
        assert_eq!(http_method_to_action(&axum::http::Method::PATCH), "write");
        // WebDAV methods (custom, matched via as_str())
        let propfind: axum::http::Method = "PROPFIND".parse().unwrap();
        assert_eq!(http_method_to_action(&propfind), "list");
        let lock: axum::http::Method = "LOCK".parse().unwrap();
        assert_eq!(http_method_to_action(&lock), "admin");
        let unlock: axum::http::Method = "UNLOCK".parse().unwrap();
        assert_eq!(http_method_to_action(&unlock), "admin");
        let mkcol: axum::http::Method = "MKCOL".parse().unwrap();
        assert_eq!(http_method_to_action(&mkcol), "write");
        let copy: axum::http::Method = "COPY".parse().unwrap();
        assert_eq!(http_method_to_action(&copy), "write");
        let move_: axum::http::Method = "MOVE".parse().unwrap();
        assert_eq!(http_method_to_action(&move_), "write");
        let proppatch: axum::http::Method = "PROPPATCH".parse().unwrap();
        assert_eq!(http_method_to_action(&proppatch), "write");
    }

    #[test]
    fn test_is_cedar_exempt_path() {
        assert!(is_cedar_exempt_path("/.well-known/ferro"));
        assert!(is_cedar_exempt_path("/.well-known/openid-configuration"));
        assert!(is_cedar_exempt_path("/api/auth/login"));
        assert!(is_cedar_exempt_path("/api/policies"));
        assert!(is_cedar_exempt_path("/api/config"));
        assert!(is_cedar_exempt_path("/s/abc123"));
        assert!(!is_cedar_exempt_path("/"));
        assert!(!is_cedar_exempt_path("/file.txt"));
        assert!(!is_cedar_exempt_path("/api/audit"));
        assert!(!is_cedar_exempt_path("/api/shares"));
    }

    #[tokio::test]
    async fn test_is_authorized_returns_decision() {
        let authorizer = CedarAuthorizer::new().unwrap();
        let request = AuthRequest {
            principal: "alice".to_string(),
            action: "read".to_string(),
            resource: "/file.txt".to_string(),
            context: serde_json::Value::Null,
        };
        let decision = authorizer.is_authorized(&request).await.unwrap();
        match decision {
            AuthDecision::Allow { policy_id } => {
                // Should be allowed by the default policy
                assert!(policy_id.is_some());
            }
            AuthDecision::Deny { reason } => {
                panic!("Should not be denied: {}", reason);
            }
        }
    }
}
