//! Sync event handlers extracted from the Ferro server.
//!
//! Provides SSE streaming of sync operations and delta/status endpoints.

use axum::extract::{Query, State};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use futures::stream::{self, StreamExt};
use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

/// Minimal trait for sync stores that can serve event-stream endpoints.
pub trait SyncEventStore: Send + Sync {
    fn event_clock(&self) -> u64;
    fn ops_since(&self, clock: u64) -> Vec<serde_json::Value>;
    fn event_total_ops(&self) -> usize;
}

/// SSE endpoint that streams sync operations as they arrive.
pub async fn sync_events<S: SyncEventStore + 'static>(State(store): State<Arc<S>>) -> Response {
    let stream = stream::unfold(store.event_clock(), move |clock| {
        let store = store.clone();
        async move {
            loop {
                tokio::time::sleep(Duration::from_millis(500)).await;
                let current = store.event_clock();
                if current > clock {
                    let ops = store.ops_since(clock);
                    return Some((ops, current));
                }
            }
        }
    })
    .flat_map(|ops| {
        stream::iter(ops).map(|op| {
            let data = serde_json::to_string(&op).unwrap_or_default();
            Ok::<_, Infallible>(Event::default().event("file-change").data(data))
        })
    });

    Sse::new(stream).keep_alive(KeepAlive::new()).into_response()
}

/// REST endpoint returning sync operations since a given clock value.
pub async fn sync_delta<S: SyncEventStore>(
    State(store): State<Arc<S>>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let since: u64 = params.get("since").and_then(|v| v.parse().ok()).unwrap_or(0);
    let ops = store.ops_since(since);

    (
        axum::http::StatusCode::OK,
        axum::Json(serde_json::json!({
            "current_clock": store.event_clock(),
            "ops": ops,
            "count": ops.len(),
        })),
    )
        .into_response()
}

/// REST endpoint returning current sync store status.
pub async fn sync_status<S: SyncEventStore>(State(store): State<Arc<S>>) -> Response {
    (
        axum::http::StatusCode::OK,
        axum::Json(serde_json::json!({
            "current_clock": store.event_clock(),
            "total_ops": store.event_total_ops(),
            "max_ops": 100000,
        })),
    )
        .into_response()
}
