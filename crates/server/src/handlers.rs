use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use tracing::instrument;

use crate::AppState;

pub async fn liveness() -> impl IntoResponse {
    (StatusCode::OK, "ok")
}

#[instrument(name = "health_check", skip(state), fields(otel.status_code))]
pub async fn health_endpoint(State(state): State<AppState>) -> Response {
    let response = state.health_checker.check_liveness().await;
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

/// GET /startupz — Kubernetes-style startup probe.
/// Returns 200 once the server has completed all startup checks
/// (storage reachability, CAS verification, DB init). Returns 503 until then.
pub async fn startup_impl<S: common::server_context::HasStartupState>(state: &S) -> Response {
    if state.is_started() {
        (StatusCode::OK, axum::Json(serde_json::json!({"status": "ok"}))).into_response()
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            axum::Json(serde_json::json!({"status": "starting"})),
        )
            .into_response()
    }
}

pub async fn startup(State(state): State<AppState>) -> Response {
    startup_impl(&state).await
}

pub async fn readiness(State(state): State<AppState>) -> Response {
    let mut subsystems = serde_json::Map::new();
    let mut healthy = true;

    let storage_ok = state.storage.list("/").await.is_ok();
    subsystems.insert(
        "storage".to_string(),
        serde_json::json!(if storage_ok { "ok" } else { "error" }),
    );
    if !storage_ok {
        healthy = false;
    }

    subsystems.insert(
        "metadata".to_string(),
        serde_json::json!(if state.metadata_store.is_some() {
            "persistent"
        } else {
            "in-memory"
        }),
    );

    // Check SQLite database reachability if configured.
    let db_ok = match &state.db {
        Some(db) => db
            .lock()
            .ok()
            .and_then(|conn| conn.execute_batch("SELECT 1;").ok())
            .is_some(),
        None => true, // No DB configured, not a failure.
    };
    subsystems.insert(
        "database".to_string(),
        serde_json::json!(if db_ok { "ok" } else { "error" }),
    );
    if !db_ok {
        healthy = false;
    }

    // Check search index readiness if configured.
    let search_ok = match &state.search {
        Some(search) => search.try_read().is_ok(),
        None => true, // No search configured, not a failure.
    };
    subsystems.insert(
        "search".to_string(),
        serde_json::json!(if search_ok { "ok" } else { "error" }),
    );
    if !search_ok {
        healthy = false;
    }

    let status = if healthy { "ok" } else { "degraded" };
    let code = if healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    let body = serde_json::json!({
        "status": status,
        "subsystems": subsystems,
    });
    (code, axum::Json(body)).into_response()
}

pub async fn health_check(State(state): State<AppState>) -> Response {
    let mut subsystems = serde_json::Map::new();
    let mut healthy = true;

    let storage_ok = state.storage.list("/").await.is_ok();
    subsystems.insert(
        "storage".to_string(),
        serde_json::json!(if storage_ok { "ok" } else { "error" }),
    );
    if !storage_ok {
        healthy = false;
    }

    subsystems.insert(
        "metadata".to_string(),
        serde_json::json!(if state.metadata_store.is_some() {
            "persistent"
        } else {
            "in-memory"
        }),
    );

    subsystems.insert(
        "wasm".to_string(),
        serde_json::json!(if state.wasm_runtime.is_some() { "ok" } else { "disabled" }),
    );

    subsystems.insert(
        "search".to_string(),
        serde_json::json!(if state.search.is_some() { "ok" } else { "disabled" }),
    );

    subsystems.insert(
        "auth".to_string(),
        serde_json::json!(if state.oidc.is_some() { "configured" } else { "disabled" }),
    );

    subsystems.insert(
        "cas".to_string(),
        serde_json::json!(if state.cas_store.is_some() {
            "enabled"
        } else {
            "disabled"
        }),
    );

    let status = if healthy { "ok" } else { "degraded" };
    let code = if healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    let body = serde_json::json!({
        "status": status,
        "version": env!("CARGO_PKG_VERSION"),
        "uptime_seconds": state.started_at.elapsed().as_secs(),
        "subsystems": subsystems,
    });
    (code, axum::Json(body)).into_response()
}

pub async fn audit_handler(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Response {
    let limit: usize = params.get("limit").and_then(|v| v.parse().ok()).unwrap_or(100);
    let offset: usize = params.get("offset").and_then(|v| v.parse().ok()).unwrap_or(0);
    let total = state.audit_log.len().await;
    let entries = state.audit_log.recent_with_offset(limit, offset).await;
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

    if let Ok(entries) = state.storage.list_all("/", 1000).await {
        for meta in &entries {
            if meta.is_collection {
                collection_count += 1;
            } else {
                file_count += 1;
                total_size += meta.size;
            }
        }
    }

    let cas_stats: serde_json::Value = if let Some(cas) = &state.cas_store {
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
            "metadata_store": state.metadata_store.is_some(),
        })),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
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
