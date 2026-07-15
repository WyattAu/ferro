use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use tracing::instrument;

use crate::AppState;
use ferro_server_state::ServerState as _;

pub async fn liveness() -> impl IntoResponse {
    (StatusCode::OK, "ok")
}

#[instrument(name = "health_check", skip(state), fields(otel.status_code))]
pub async fn health_endpoint(State(state): State<AppState>) -> Response {
    let response = state.health_checker().check_liveness().await;
    let status = match response.status {
        ferro_health::HealthStatus::Healthy => StatusCode::OK,
        ferro_health::HealthStatus::Degraded => StatusCode::OK,
        ferro_health::HealthStatus::Unhealthy => StatusCode::SERVICE_UNAVAILABLE,
        ferro_health::HealthStatus::Unknown => StatusCode::OK,
    };
    let body = serde_json::json!({
        "status": serde_json::to_value(response.status).unwrap_or_default(),
        "version": response.version,
        "uptime_seconds": response.uptime.as_secs(),
        "checks": response.checks,
    });
    tracing::Span::current().record("otel.status_code", status.as_u16());
    (status, axum::Json(body)).into_response()
}

pub async fn startup(State(state): State<AppState>) -> Response {
    ferro_server_health::startup_impl(&state).await
}

pub async fn readiness(State(state): State<AppState>) -> Response {
    ferro_server_health::readiness_impl(&state).await
}

pub async fn health_check(State(state): State<AppState>) -> Response {
    ferro_server_health::health_check_impl(&state).await
}

pub async fn audit_handler(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Response {
    let limit: usize = params.get("limit").and_then(|v| v.parse().ok()).unwrap_or(100);
    let offset: usize = params.get("offset").and_then(|v| v.parse().ok()).unwrap_or(0);
    let total = state.audit_log().len().await;
    let entries = state.audit_log().recent_with_offset(limit, offset).await;
    (
        axum::http::StatusCode::OK,
        axum::Json(serde_json::json!({
            "entries": entries,
            "total": total,
            "limit": limit,
            "offset": offset,
        })),
    )
        .into_response()
}

pub async fn storage_stats(State(state): State<AppState>) -> Response {
    let mut file_count = 0u64;
    let mut total_size = 0u64;
    let mut collection_count = 0u64;

    if let Ok(entries) = state.storage().list_all("/", 1000).await {
        for meta in &entries {
            if meta.is_collection {
                collection_count += 1;
            } else {
                file_count += 1;
                total_size += meta.size;
            }
        }
    }

    let cas_stats: serde_json::Value = if let Some(cas) = state.cas_store() {
        serde_json::json!({
            "enabled": true,
            "content_blocks": cas.content_count().await,
        })
    } else {
        serde_json::json!({"enabled": false})
    };

    (
        axum::http::StatusCode::OK,
        axum::Json(serde_json::json!({
            "files": file_count,
            "collections": collection_count,
            "total_bytes": total_size,
            "cas": cas_stats,
            "metadata_store": state.metadata_store().is_some(),
        })),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferro_server_health::startup_impl;
    use http_body_util::BodyExt;

    fn test_state() -> AppState {
        AppState::in_memory()
    }

    async fn body_json(resp: Response) -> serde_json::Value {
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn test_liveness_returns_ok() {
        use axum::response::IntoResponse;
        let resp = liveness().await.into_response();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_startup_impl_not_started() {
        let state = test_state();
        let resp = startup_impl(&state).await;
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
        let json = body_json(resp).await;
        assert_eq!(json["status"], "starting");
    }

    #[tokio::test]
    async fn test_startup_impl_started() {
        let state = test_state();
        state.startup_complete.store(true, std::sync::atomic::Ordering::Relaxed);
        let resp = startup_impl(&state).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["status"], "ok");
    }

    #[tokio::test]
    async fn test_health_endpoint_ok() {
        let state = test_state();
        let resp = health_endpoint(State(state)).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert!(json["status"].is_string());
        assert!(json["uptime_seconds"].is_number());
    }

    #[tokio::test]
    async fn test_readiness_ok() {
        let state = test_state();
        let resp = readiness(State(state)).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["status"], "ok");
        assert!(json["subsystems"]["storage"] == "ok");
    }

    #[tokio::test]
    async fn test_health_check_ok() {
        let state = test_state();
        let resp = health_check(State(state)).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["status"], "ok");
        assert!(json["subsystems"]["storage"] == "ok");
        assert!(json["subsystems"]["wasm"].is_string());
        assert!(json["subsystems"]["auth"].is_string());
    }

    #[tokio::test]
    async fn test_storage_stats_ok() {
        let state = test_state();
        let resp = storage_stats(State(state)).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert!(json["files"].is_number());
        assert!(json["collections"].is_number());
        assert!(json["total_bytes"].is_number());
        assert!(json["cas"].is_object());
        assert!(json["metadata_store"].is_boolean());
    }

    #[tokio::test]
    async fn test_audit_handler_defaults() {
        let state = test_state();
        let resp = audit_handler(State(state), axum::extract::Query(std::collections::HashMap::new())).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["limit"], 100);
        assert_eq!(json["offset"], 0);
        assert_eq!(json["total"], 0);
        assert!(json["entries"].is_array());
    }

    #[tokio::test]
    async fn test_audit_handler_custom_pagination() {
        let state = test_state();
        let mut params = std::collections::HashMap::new();
        params.insert("limit".to_string(), "10".to_string());
        params.insert("offset".to_string(), "5".to_string());
        let resp = audit_handler(State(state), axum::extract::Query(params)).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["limit"], 10);
        assert_eq!(json["offset"], 5);
    }

    #[tokio::test]
    async fn test_audit_handler_invalid_params_fallback() {
        let state = test_state();
        let mut params = std::collections::HashMap::new();
        params.insert("limit".to_string(), "notanumber".to_string());
        params.insert("offset".to_string(), "xyz".to_string());
        let resp = audit_handler(State(state), axum::extract::Query(params)).await;
        let json = body_json(resp).await;
        assert_eq!(json["limit"], 100);
        assert_eq!(json["offset"], 0);
    }
}
