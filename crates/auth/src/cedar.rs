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

#[non_exhaustive]
/// Configuration for the Cedar policy engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CedarConfig {
    /// Default Cedar policy text (Cedar policy language).
    pub default_policy: String,
}

#[non_exhaustive]
/// Cedar-based authorization engine that evaluates policy decisions.
#[derive(Clone)]
pub struct CedarAuthorizer {
    policy_set: Arc<RwLock<PolicySet>>,
}

impl CedarAuthorizer {
    /// Create a new authorizer with a permissive default policy.
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

    /// Replace all policies with the given set.
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

    /// Append a single policy to the current policy set.
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

    /// Evaluate an authorization request and return a full decision.
    pub async fn is_authorized(&self, request: &AuthRequest) -> Result<AuthDecision> {
        let guard = self.policy_set.read().await;

        let principal: EntityUid = match format!("User::\"{}\"", request.principal).parse() {
            Ok(uid) => uid,
            Err(e) => {
                warn!(
                    "Failed to parse principal EntityUid for {:?}: {:?}",
                    request.principal, e
                );
                fallback_principal()
            }
        };

        let action: EntityUid = match format!("Action::\"{}\"", request.action).parse() {
            Ok(uid) => uid,
            Err(e) => {
                warn!(
                    "Failed to parse action EntityUid for {:?}: {:?}",
                    request.action, e
                );
                fallback_action()
            }
        };

        let resource: EntityUid = match format!("File::\"{}\"", request.resource).parse() {
            Ok(uid) => uid,
            Err(e) => {
                warn!(
                    "Failed to parse resource EntityUid for {:?}: {:?}",
                    request.resource, e
                );
                fallback_resource()
            }
        };

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

    /// Evaluate authorization and return a simple boolean (allow/deny).
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
            _ => Err(FerroError::Internal(
                "Unknown authorization decision".to_string(),
            )),
        }
    }
}

#[cfg(feature = "handlers")]
fn http_method_to_action(method: &axum::http::Method) -> &'static str {
    match *method {
        axum::http::Method::GET | axum::http::Method::HEAD => "read",
        axum::http::Method::DELETE => "delete",
        axum::http::Method::PUT | axum::http::Method::POST | axum::http::Method::PATCH => "write",
        _ => match method.as_str() {
            "PROPFIND" => "list",
            "LOCK" | "UNLOCK" => "admin",
            _ => "write",
        },
    }
}

#[cfg(feature = "handlers")]
fn is_cedar_exempt_path(path: &str) -> bool {
    path == "/.well-known/ferro"
        || path == "/.well-known/openid-configuration"
        || path.starts_with("/api/auth/login")
        || path == "/api/policies"
        || path == "/api/config"
        || path.starts_with("/s/")
}

/// Axum middleware that enforces Cedar authorization policies.
#[cfg(feature = "handlers")]
pub async fn cedar_middleware(
    cedar: Option<Arc<CedarAuthorizer>>,
    request: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use axum::response::IntoResponse;

    let path = request.uri().path();

    if is_cedar_exempt_path(path) {
        return next.run(request).await;
    }

    let authorizer = match cedar {
        Some(a) => a,
        None => return next.run(request).await,
    };

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

        assert!(
            authorizer
                .is_authorized_simple("alice", "read", "/file.txt")
                .await
                .unwrap()
        );
        assert!(
            !authorizer
                .is_authorized_simple("bob", "read", "/file.txt")
                .await
                .unwrap()
        );
        assert!(
            !authorizer
                .is_authorized_simple("alice", "write", "/file.txt")
                .await
                .unwrap()
        );
        assert!(
            !authorizer
                .is_authorized_simple("anonymous", "read", "/file.txt")
                .await
                .unwrap()
        );
    }

    #[cfg(feature = "handlers")]
    #[test]
    fn test_http_method_to_action() {
        assert_eq!(http_method_to_action(&axum::http::Method::GET), "read");
        assert_eq!(http_method_to_action(&axum::http::Method::HEAD), "read");
        assert_eq!(http_method_to_action(&axum::http::Method::PUT), "write");
        assert_eq!(http_method_to_action(&axum::http::Method::DELETE), "delete");
        assert_eq!(http_method_to_action(&axum::http::Method::POST), "write");
        assert_eq!(http_method_to_action(&axum::http::Method::PATCH), "write");
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

    #[cfg(feature = "handlers")]
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
                assert!(policy_id.is_some());
            }
            AuthDecision::Deny { reason } => {
                panic!("Should not be denied: {}", reason);
            }
            _ => panic!("Unexpected decision"),
        }
    }
}
