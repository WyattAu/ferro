use axum::extract::State;
use axum::response::Response;

use crate::AppState;

impl ferro_server_sync_handlers::events::SyncEventStore for crate::sync::ops::SyncStore {
    fn event_clock(&self) -> u64 {
        self.current_clock()
    }

    fn ops_since(&self, clock: u64) -> Vec<serde_json::Value> {
        self.get_ops_since(clock)
            .into_iter()
            .map(|op| serde_json::to_value(&op).unwrap_or_default())
            .collect()
    }

    fn event_total_ops(&self) -> usize {
        self.ops.len()
    }
}

pub async fn sync_events(State(state): State<AppState>) -> Response {
    ferro_server_sync_handlers::events::sync_events(State(state.sync_store)).await
}

pub async fn sync_delta(
    State(state): State<AppState>,
    query: axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Response {
    ferro_server_sync_handlers::events::sync_delta(State(state.sync_store), query).await
}

pub async fn sync_status(State(state): State<AppState>) -> Response {
    ferro_server_sync_handlers::events::sync_status(State(state.sync_store)).await
}
