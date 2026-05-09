use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use std::sync::Arc;

use crate::cedar::CedarAuthorizer;

/// Shared state for the policy API handlers.
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
pub async fn add_policy(
    State(state): State<PolicyState>,
    axum::Json(req): axum::Json<AddPolicyRequest>,
) -> Response {
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

fn policy_error(
    status: StatusCode,
    code: &str,
    message: &str,
    details: Option<String>,
) -> Response {
    let mut body = serde_json::json!({
        "error": message,
        "error_code": code,
    });
    if let Some(d) = details {
        body["details"] = serde_json::json!(d);
    }
    (status, axum::Json(body)).into_response()
}
