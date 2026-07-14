use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use std::sync::Arc;

use crate::cedar::CedarAuthorizer;

/// Shared state for the policy API handlers.
#[derive(Debug)]
pub struct PolicyState {
    pub cedar: Option<Arc<CedarAuthorizer>>,
}

/// List currently configured policies.
pub async fn list_policies(State(state): State<PolicyState>) -> Response {
    match &state.cedar {
        None => {
            let body = serde_json::json!({
                "policies": [],
                "configured": false,
            });
            (StatusCode::OK, axum::Json(body)).into_response()
        }
        Some(_authorizer) => {
            let body = serde_json::json!({
                "policies": [],
                "configured": true,
            });
            (StatusCode::OK, axum::Json(body)).into_response()
        }
    }
}

/// Request body for adding a new Cedar policy.
#[derive(Debug, Deserialize)]
pub struct AddPolicyRequest {
    pub policy: String,
}

/// Add a new Cedar policy to the authorizer.
pub async fn add_policy(State(state): State<PolicyState>, axum::Json(req): axum::Json<AddPolicyRequest>) -> Response {
    match &state.cedar {
        None => policy_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "NOT_CONFIGURED",
            "Cedar authorization is not configured",
            None,
        ),
        Some(authorizer) => match authorizer.add_policy(&req.policy).await {
            Ok(()) => {
                let body = serde_json::json!({
                    "status": "added",
                });
                (StatusCode::CREATED, axum::Json(body)).into_response()
            }
            Err(e) => policy_error(
                StatusCode::BAD_REQUEST,
                "POLICY_INVALID",
                "Invalid policy",
                Some(e.to_string()),
            ),
        },
    }
}

/// Request body for deleting a policy by ID.
#[derive(Debug, Deserialize)]
pub struct DeletePolicyRequest {
    pub policy_id: String,
}

/// Delete a policy by ID (not yet implemented).
pub async fn delete_policy(
    State(state): State<PolicyState>,
    axum::Json(_req): axum::Json<DeletePolicyRequest>,
) -> Response {
    match &state.cedar {
        None => policy_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "NOT_CONFIGURED",
            "Cedar authorization is not configured",
            None,
        ),
        Some(_) => policy_error(
            StatusCode::NOT_IMPLEMENTED,
            "NOT_FOUND",
            "Policy removal requires reloading the full policy set. Use PUT /api/policies to replace all policies.",
            None,
        ),
    }
}

fn policy_error(status: StatusCode, code: &str, message: &str, details: Option<String>) -> Response {
    let mut body = serde_json::json!({
        "error": message,
        "error_code": code,
    });
    if let Some(d) = details {
        body["details"] = serde_json::json!(d);
    }
    (status, axum::Json(body)).into_response()
}

#[cfg(test)]
#[cfg(feature = "handlers")]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_list_policies_not_configured() {
        let state = PolicyState { cedar: None };
        let resp = list_policies(State(state)).await;
        let (parts, body) = resp.into_parts();
        assert_eq!(parts.status, StatusCode::OK);
        let bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["configured"], false);
    }

    #[tokio::test]
    async fn test_list_policies_configured() {
        let cedar = std::sync::Arc::new(crate::cedar::CedarAuthorizer::new().unwrap());
        let state = PolicyState { cedar: Some(cedar) };
        let resp = list_policies(State(state)).await;
        let (parts, body) = resp.into_parts();
        assert_eq!(parts.status, StatusCode::OK);
        let bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["configured"], true);
    }

    #[tokio::test]
    async fn test_add_policy_not_configured() {
        let state = PolicyState { cedar: None };
        let req = AddPolicyRequest {
            policy: "permit(principal, action, resource);".to_string(),
        };
        let resp = add_policy(State(state), axum::Json(req)).await;
        let (parts, _) = resp.into_parts();
        assert_eq!(parts.status, StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn test_add_policy_invalid() {
        let cedar = std::sync::Arc::new(crate::cedar::CedarAuthorizer::new().unwrap());
        let state = PolicyState { cedar: Some(cedar) };
        let req = AddPolicyRequest {
            policy: "NOT VALID CEDAR!!!".to_string(),
        };
        let resp = add_policy(State(state), axum::Json(req)).await;
        let (parts, _) = resp.into_parts();
        assert_eq!(parts.status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_delete_policy_not_configured() {
        let state = PolicyState { cedar: None };
        let req = DeletePolicyRequest {
            policy_id: "test".to_string(),
        };
        let resp = delete_policy(State(state), axum::Json(req)).await;
        let (parts, _) = resp.into_parts();
        assert_eq!(parts.status, StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn test_delete_policy_not_implemented() {
        let cedar = std::sync::Arc::new(crate::cedar::CedarAuthorizer::new().unwrap());
        let state = PolicyState { cedar: Some(cedar) };
        let req = DeletePolicyRequest {
            policy_id: "test".to_string(),
        };
        let resp = delete_policy(State(state), axum::Json(req)).await;
        let (parts, _) = resp.into_parts();
        assert_eq!(parts.status, StatusCode::NOT_IMPLEMENTED);
    }
}
