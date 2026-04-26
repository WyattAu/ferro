use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use crate::AppState;

pub async fn metrics_handler(State(state): State<AppState>) -> Response {
    let uptime_secs = state.started_at.elapsed().as_secs();

    let mut file_count = 0u64;
    let mut total_bytes = 0u64;
    if let Ok(entries) = state.storage.list_all("/", 10000).await {
        for meta in &entries {
            if !meta.is_collection {
                file_count += 1;
                total_bytes += meta.size;
            }
        }
    }

    let body = serde_json::json!({
        "uptime_seconds": uptime_secs,
        "storage": {
            "files": file_count,
            "total_bytes": total_bytes,
        },
        "requests": {
            "total": 0,
        }
    });

    (StatusCode::OK, axum::Json(body)).into_response()
}
