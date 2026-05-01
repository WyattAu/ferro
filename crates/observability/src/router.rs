use axum::Json;
use axum::Router;
use axum::extract::{FromRef, State};
use axum::routing::{get, post};
use serde::Deserialize;
use std::sync::Arc;

use crate::log_buffer::LogBuffer;
use crate::registry::MetricsRegistry;
use crate::{exporter, loki, victoria_logs, victoria_metrics};

#[derive(Clone)]
pub struct ObservabilityState {
    pub registry: Arc<MetricsRegistry>,
    pub log_buffer: Arc<LogBuffer>,
}

impl FromRef<ObservabilityState> for Arc<MetricsRegistry> {
    fn from_ref(state: &ObservabilityState) -> Self {
        state.registry.clone()
    }
}

impl FromRef<ObservabilityState> for Arc<LogBuffer> {
    fn from_ref(state: &ObservabilityState) -> Self {
        state.log_buffer.clone()
    }
}

pub fn build_observability_router(
    registry: Arc<MetricsRegistry>,
    log_buffer: Arc<LogBuffer>,
) -> Router {
    let state = ObservabilityState {
        registry,
        log_buffer,
    };

    Router::new()
        .route("/metrics", get(prometheus_metrics_handler))
        .route("/health", get(observability_health))
        .route("/loki/api/v1/push", post(loki::loki_push_handler))
        .route("/loki/api/v1/query", get(loki::loki_query_handler))
        .route("/loki/api/v1/labels", get(loki::loki_labels_handler))
        .route("/api/v1/write", post(victoria_metrics::vm_write_handler))
        .route("/api/v1/targets", get(victoria_metrics::vm_targets_handler))
        .route(
            "/api/v1/status/tsdb",
            get(victoria_metrics::vm_tsdb_status_handler),
        )
        .route("/insert/jsonline", post(victoria_logs::vl_insert_handler))
        .route("/insert/promtail", post(victoria_logs::vl_promtail_handler))
        .route("/api/v1/logs", get(query_logs_handler))
        .with_state(state)
}

async fn prometheus_metrics_handler(State(obs): State<ObservabilityState>) -> String {
    exporter::export_prometheus(&obs.registry)
}

async fn observability_health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

#[derive(Deserialize)]
struct LogQueryParams {
    level: Option<String>,
    limit: Option<usize>,
}

async fn query_logs_handler(
    State(obs): State<ObservabilityState>,
    axum::extract::Query(params): axum::extract::Query<LogQueryParams>,
) -> Json<serde_json::Value> {
    let entries = obs
        .log_buffer
        .query(params.level.as_deref(), params.limit.unwrap_or(100));
    Json(serde_json::json!({
        "total": entries.len(),
        "entries": entries
    }))
}
