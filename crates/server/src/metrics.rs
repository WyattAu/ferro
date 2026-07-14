use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use crate::AppState;
use common::server_context::{HasStorage, HasUptime};

/// GET /metrics — return server metrics as JSON.
///
/// This handler is generic over `HasStorage + HasUptime`, demonstrating
/// the Phase 1 trait-based decomposition pattern. Extracted crates can
/// depend on these traits instead of the concrete `AppState`.
pub async fn metrics_handler_impl<S: HasStorage + HasUptime>(state: &S) -> Response {
    let uptime_secs = state.started_at().elapsed().as_secs();

    let mut file_count = 0u64;
    let mut total_bytes = 0u64;
    if let Ok(entries) = state.storage().list_all("/", 10000).await {
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

/// Concrete axum handler that delegates to the generic implementation.
/// This allows axum to infer the type from `State<AppState>`.
pub async fn metrics_handler(State(state): State<AppState>) -> Response {
    metrics_handler_impl(&state).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::BodyExt;

    async fn body_json(resp: Response) -> serde_json::Value {
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn test_metrics_handler_empty() {
        let state = AppState::in_memory();
        let resp = metrics_handler(State(state)).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert!(json["uptime_seconds"].is_number());
        assert!(json["storage"]["files"].is_number());
        assert!(json["storage"]["total_bytes"].is_number());
        assert!(json["requests"]["total"].is_number());
    }

    #[tokio::test]
    async fn test_metrics_handler_with_files() {
        let state = AppState::in_memory();
        state
            .storage
            .put("/test.txt", bytes::Bytes::from("hello"), "anon")
            .await
            .unwrap();
        let resp = metrics_handler(State(state)).await;
        let json = body_json(resp).await;
        assert_eq!(json["storage"]["files"], 1);
        assert_eq!(json["storage"]["total_bytes"], 5);
    }

    #[tokio::test]
    async fn test_metrics_impl_with_trait() {
        let state = AppState::in_memory();
        let resp = metrics_handler_impl(&state).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
