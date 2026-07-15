//! Admin API endpoints for per-tenant rate limit management.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use ferro_rate_limiter::RateLimiter;
use serde::{Deserialize, Serialize};

use crate::AppState;
use crate::api_error::ApiError;

/// Request body for updating a tenant's rate limit configuration.
#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateTenantRateLimitRequest {
    pub max_requests: Option<u32>,
    pub refill_rate: Option<u32>,
    pub refill_interval_secs: Option<u64>,
}

/// Response body for a tenant's rate limit configuration.
#[derive(Debug, Serialize)]
pub struct TenantRateLimitResponse {
    pub tenant_id: String,
    pub max_requests: u32,
    pub refill_rate: u32,
    pub refill_interval_secs: u64,
}

/// Core logic for getting a tenant's rate limit configuration.
async fn get_tenant_rate_limit_impl<S: ferro_server_state::ServerState>(state: &S, tenant_id: &str) -> Response {
    let Some(store) = state.tenant_rate_limit_store() else {
        return ApiError::not_found("TENANT_RATE_LIMIT_DISABLED", "Tenant rate limiting is not configured")
            .into_response();
    };

    let config = store.get_config(tenant_id);
    Json(TenantRateLimitResponse {
        tenant_id: tenant_id.to_string(),
        max_requests: config.max_requests,
        refill_rate: config.refill_rate,
        refill_interval_secs: config.refill_interval.as_secs(),
    })
    .into_response()
}

/// GET /api/admin/tenants/:id/rate-limit
///
/// Returns the rate limit configuration for a specific tenant.
pub async fn get_tenant_rate_limit(State(state): State<AppState>, Path(tenant_id): Path<String>) -> Response {
    get_tenant_rate_limit_impl(&state, &tenant_id).await
}

/// Core logic for updating a tenant's rate limit configuration.
async fn update_tenant_rate_limit_impl<S: ferro_server_state::ServerState>(
    state: &S,
    tenant_id: &str,
    req: UpdateTenantRateLimitRequest,
) -> Response {
    let Some(store) = state.tenant_rate_limit_store() else {
        return ApiError::not_found("TENANT_RATE_LIMIT_DISABLED", "Tenant rate limiting is not configured")
            .into_response();
    };

    let mut config = store.get_config(tenant_id);

    if let Some(max) = req.max_requests {
        config.max_requests = max;
    }
    if let Some(rate) = req.refill_rate {
        config.refill_rate = rate;
    }
    if let Some(interval) = req.refill_interval_secs {
        config.refill_interval = std::time::Duration::from_secs(interval);
    }

    store.set_config(tenant_id, config.clone());

    // Reset the limiter bucket so the new config takes effect immediately.
    if let Some(limiter) = state.tenant_rate_limiter() {
        limiter.reset_tenant(tenant_id).await;
    }

    Json(TenantRateLimitResponse {
        tenant_id: tenant_id.to_string(),
        max_requests: config.max_requests,
        refill_rate: config.refill_rate,
        refill_interval_secs: config.refill_interval.as_secs(),
    })
    .into_response()
}

/// PUT /api/admin/tenants/:id/rate-limit
///
/// Updates the rate limit configuration for a specific tenant.
/// Only the fields present in the request body are updated.
pub async fn update_tenant_rate_limit(
    State(state): State<AppState>,
    Path(tenant_id): Path<String>,
    Json(req): Json<UpdateTenantRateLimitRequest>,
) -> Response {
    update_tenant_rate_limit_impl(&state, &tenant_id, req).await
}

/// Core logic for getting a tenant's rate limit status.
async fn get_tenant_rate_limit_status_impl<S: ferro_server_state::ServerState>(state: &S, tenant_id: &str) -> Response {
    let Some(limiter) = state.tenant_rate_limiter() else {
        return ApiError::not_found("TENANT_RATE_LIMIT_DISABLED", "Tenant rate limiting is not configured")
            .into_response();
    };

    let Some(store) = state.tenant_rate_limit_store() else {
        return ApiError::not_found("TENANT_RATE_LIMIT_DISABLED", "Tenant rate limiting is not configured")
            .into_response();
    };

    let config = store.get_config(tenant_id);

    let result = limiter.check(tenant_id).await;

    match result {
        Ok(status) => Json(serde_json::json!({
            "tenant_id": tenant_id,
            "allowed": status.allowed,
            "remaining": status.remaining,
            "max_requests": config.max_requests,
            "reset_at": status.reset_at.elapsed().as_secs(),
        }))
        .into_response(),
        Err(e) => {
            ApiError::internal("RATE_LIMIT_CHECK_FAILED", format!("Rate limit check failed: {e}")).into_response()
        }
    }
}

/// GET /api/admin/tenants/:id/rate-limit/status
///
/// Returns the current rate limit usage status for a tenant.
pub async fn get_tenant_rate_limit_status(State(state): State<AppState>, Path(tenant_id): Path<String>) -> Response {
    get_tenant_rate_limit_status_impl(&state, &tenant_id).await
}

/// Core logic for deleting a tenant's rate limit configuration.
async fn delete_tenant_rate_limit_impl<S: ferro_server_state::ServerState>(state: &S, tenant_id: &str) -> Response {
    let Some(store) = state.tenant_rate_limit_store() else {
        return ApiError::not_found("TENANT_RATE_LIMIT_DISABLED", "Tenant rate limiting is not configured")
            .into_response();
    };

    store.remove_config(tenant_id);

    if let Some(limiter) = state.tenant_rate_limiter() {
        limiter.reset_tenant(tenant_id).await;
    }

    StatusCode::NO_CONTENT.into_response()
}

/// DELETE /api/admin/tenants/:id/rate-limit
///
/// Removes a tenant's custom rate limit configuration, reverting to defaults.
pub async fn delete_tenant_rate_limit(State(state): State<AppState>, Path(tenant_id): Path<String>) -> Response {
    delete_tenant_rate_limit_impl(&state, &tenant_id).await
}

/// Core logic for listing all tenant rate limits.
async fn list_tenant_rate_limits_impl<S: ferro_server_state::ServerState>(state: &S) -> Response {
    let Some(store) = state.tenant_rate_limit_store() else {
        return ApiError::not_found("TENANT_RATE_LIMIT_DISABLED", "Tenant rate limiting is not configured")
            .into_response();
    };

    let configs: Vec<TenantRateLimitResponse> = store
        .list_configs()
        .into_iter()
        .map(|(tenant_id, config)| TenantRateLimitResponse {
            tenant_id,
            max_requests: config.max_requests,
            refill_rate: config.refill_rate,
            refill_interval_secs: config.refill_interval.as_secs(),
        })
        .collect();

    Json(serde_json::json!({
        "rate_limits": configs,
        "total": configs.len(),
    }))
    .into_response()
}

/// GET /api/admin/tenants/rate-limits
///
/// Lists all configured tenant rate limits.
pub async fn list_tenant_rate_limits(State(state): State<AppState>) -> Response {
    list_tenant_rate_limits_impl(&state).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AppState;
    use axum::body::Body;
    use http_body_util::BodyExt;
    use std::sync::Arc;

    fn setup_state() -> AppState {
        let mut state = AppState::in_memory();
        let store = Arc::new(ferro_rate_limiter::tenant::TenantRateLimitStore::new());
        state.tenant_rate_limit_store = Some(store.clone());
        state.tenant_rate_limiter = Some(Arc::new(ferro_rate_limiter::tenant::TenantAwareRateLimiter::new(store)));
        state
    }

    fn flat_test_router(state: AppState) -> axum::Router {
        axum::Router::new()
            .route(
                "/admin/tenants/rate-limits",
                axum::routing::get(super::list_tenant_rate_limits),
            )
            .route(
                "/admin/tenants/:id/rate-limit",
                axum::routing::get(super::get_tenant_rate_limit)
                    .put(super::update_tenant_rate_limit)
                    .delete(super::delete_tenant_rate_limit),
            )
            .route(
                "/admin/tenants/:id/rate-limit/status",
                axum::routing::get(super::get_tenant_rate_limit_status),
            )
            .with_state(state)
    }

    #[tokio::test]
    async fn test_get_tenant_rate_limit_default() {
        let state = setup_state();
        let app = flat_test_router(state);

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/admin/tenants/tenant-1/rate-limit")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["tenant_id"], "tenant-1");
        assert_eq!(json["max_requests"], 1000); // default
    }

    #[tokio::test]
    async fn test_update_tenant_rate_limit() {
        let state = setup_state();
        let app = flat_test_router(state);

        // Update
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/admin/tenants/tenant-1/rate-limit")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "max_requests": 500,
                            "refill_rate": 50,
                            "refill_interval_secs": 30,
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["max_requests"], 500);
        assert_eq!(json["refill_rate"], 50);
        assert_eq!(json["refill_interval_secs"], 30);

        // Verify GET returns updated config
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/admin/tenants/tenant-1/rate-limit")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["max_requests"], 500);
    }

    #[tokio::test]
    async fn test_delete_tenant_rate_limit() {
        let state = setup_state();
        let app = flat_test_router(state);

        // Set
        app.clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/admin/tenants/tenant-1/rate-limit")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::json!({"max_requests": 100}).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Delete
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/admin/tenants/tenant-1/rate-limit")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NO_CONTENT);

        // Verify reverted to default
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/admin/tenants/tenant-1/rate-limit")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let json = body_json(resp).await;
        assert_eq!(json["max_requests"], 1000); // default
    }

    #[tokio::test]
    async fn test_list_tenant_rate_limits() {
        let state = setup_state();
        let app = flat_test_router(state);

        // Add two tenants
        for tenant_id in ["t1", "t2"] {
            app.clone()
                .oneshot(
                    Request::builder()
                        .method("PUT")
                        .uri(format!("/admin/tenants/{tenant_id}/rate-limit"))
                        .header("content-type", "application/json")
                        .body(Body::from(serde_json::json!({"max_requests": 200}).to_string()))
                        .unwrap(),
                )
                .await
                .unwrap();
        }

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/admin/tenants/rate-limits")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["total"], 2);
    }

    async fn body_json(resp: Response) -> serde_json::Value {
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    use axum::http::Request;
    use tower::ServiceExt;
}
