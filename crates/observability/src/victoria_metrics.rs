use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use std::sync::Arc;

use crate::registry::MetricsRegistry;

pub async fn vm_write_handler(State(_registry): State<Arc<MetricsRegistry>>, _body: String) -> StatusCode {
    StatusCode::NO_CONTENT
}

pub async fn vm_targets_handler() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "success",
        "data": {
            "activeTargets": [],
            "droppedTargets": []
        }
    }))
}

pub async fn vm_tsdb_status_handler() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "success",
        "data": {
            "status": "OK",
            "timestampMs": chrono::Utc::now().timestamp_millis()
        }
    }))
}
