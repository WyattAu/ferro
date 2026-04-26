use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;

use crate::api_error::ApiError;
use crate::AppState;

pub async fn list_policies(State(state): State<AppState>) -> Response {
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

#[derive(Debug, Deserialize)]
pub struct AddPolicyRequest {
    pub policy: String,
}

pub async fn add_policy(
    State(state): State<AppState>,
    axum::Json(req): axum::Json<AddPolicyRequest>,
) -> Response {
    match &state.cedar {
        None => {
            ApiError::service_unavailable("NOT_CONFIGURED", "Cedar authorization is not configured")
        }
        Some(authorizer) => {
            match authorizer.add_policy(&req.policy).await {
                Ok(()) => {
                    let body = serde_json::json!({
                        "status": "added",
                    });
                    (StatusCode::CREATED, axum::Json(body)).into_response()
                }
                Err(e) => ApiError::with_details(
                    StatusCode::BAD_REQUEST,
                    ApiError::POLICY_INVALID,
                    "Invalid policy",
                    e.to_string(),
                ),
            }
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct DeletePolicyRequest {
    pub policy_id: String,
}

pub async fn delete_policy(
    State(state): State<AppState>,
    axum::Json(_req): axum::Json<DeletePolicyRequest>,
) -> Response {
    match &state.cedar {
        None => {
            ApiError::service_unavailable("NOT_CONFIGURED", "Cedar authorization is not configured")
        }
        Some(_) => {
            ApiError::not_implemented(
                ApiError::NOT_FOUND,
                "Policy removal requires reloading the full policy set. Use PUT /api/policies to replace all policies.",
            )
        }
    }
}
