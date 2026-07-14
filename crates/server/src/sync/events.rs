use axum::extract::{Query, State};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use futures::stream::{self, StreamExt};
use std::collections::HashMap;
use std::convert::Infallible;
use std::time::Duration;

use crate::AppState;
// NOTE: sync_store is not in ServerState trait due to type mismatch between
// ferro_server_state::SyncStoreTrait and crate::sync::ops::SyncStore.
// These handlers access state.sync_store directly.

pub async fn sync_events(State(state): State<AppState>) -> Response {
    let store = state.sync_store.clone();

    let stream = stream::unfold(store.current_clock(), move |clock| {
        let store = store.clone();
        async move {
            loop {
                tokio::time::sleep(Duration::from_millis(500)).await;
                let current = store.current_clock();
                if current > clock {
                    let ops = store.get_ops_since(clock);
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

pub async fn sync_delta(State(state): State<AppState>, Query(params): Query<HashMap<String, String>>) -> Response {
    let since: u64 = params.get("since").and_then(|v| v.parse().ok()).unwrap_or(0);
    let ops = state.sync_store.get_ops_since(since);

    (
        axum::http::StatusCode::OK,
        axum::Json(serde_json::json!({
            "current_clock": state.sync_store.current_clock(),
            "ops": ops,
            "count": ops.len(),
        })),
    )
        .into_response()
}

pub async fn sync_status(State(state): State<AppState>) -> Response {
    (
        axum::http::StatusCode::OK,
        axum::Json(serde_json::json!({
            "current_clock": state.sync_store.current_clock(),
            "total_ops": state.sync_store.ops.len(),
            "max_ops": 100000,
        })),
    )
        .into_response()
}
