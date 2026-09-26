//! LAN/WAN peer registry for mesh sync.
//!
//! Desktop clients register themselves after each sync cycle with their
//! reachable addresses and the block hashes they hold. Other peers on the
//! same network query this index to fetch blocks directly instead of
//! round-tripping through the server.

use crate::state::AppState;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::Utc;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

const PEER_TTL_SECS: i64 = 600;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerRegistration {
    /// Caller's OIDC sub (derived from X-Ferro-User, not trusted from body).
    #[serde(default)]
    pub sub: String,
    pub device_name: String,
    /// IP addresses the peer is reachable on (LAN + WireGuard).
    pub addresses: Vec<String>,
    /// Port the peer's block-exchange listener is bound to.
    pub port: u16,
    /// Block hashes this peer holds (post-sync set).
    pub block_hashes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PeerEntry {
    pub sub: String,
    pub device_name: String,
    pub addresses: Vec<String>,
    pub port: u16,
    pub block_count: usize,
    pub last_seen: String,
}

fn caller(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-ferro-user")
        .and_then(|v| v.to_str().ok())
        .filter(|u| !u.is_empty() && *u != "anonymous")
        .map(str::to_owned)
}

pub async fn register_peer(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(mut reg): Json<PeerRegistration>,
) -> Response {
    let Some(sub) = caller(&headers) else {
        return (
            axum::http::StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "authentication required"})),
        )
            .into_response();
    };
    reg.sub = sub;
    reg.block_hashes.sort();
    reg.block_hashes.dedup();

    let key = format!("{}:{}", reg.sub, reg.device_name);
    state
        .mesh_peers
        .insert(key, (reg, Utc::now().timestamp()));

    axum::http::StatusCode::OK.into_response()
}

pub async fn list_peers(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Response {
    let Some(my_sub) = caller(&headers) else {
        return (
            axum::http::StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "authentication required"})),
        )
            .into_response();
    };

    let now = Utc::now().timestamp();
    let mut peers: Vec<PeerEntry> = state
        .mesh_peers
        .iter()
        .filter(|entry| {
            let (reg, ts) = entry.value();
            now - ts < PEER_TTL_SECS && reg.sub != my_sub
        })
        .map(|entry| {
            let (reg, ts) = entry.value();
            PeerEntry {
                sub: reg.sub.clone(),
                device_name: reg.device_name.clone(),
                addresses: reg.addresses.clone(),
                port: reg.port,
                block_count: reg.block_hashes.len(),
                last_seen: format!("{ts}"),
            }
        })
        .collect();

    peers.sort_by(|a, b| b.block_count.cmp(&a.block_count));
    (axum::http::StatusCode::OK, Json(serde_json::json!({ "peers": peers }))).into_response()
}

/// Block-hash index lookup: which registered peers hold a given block?
pub async fn query_block(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Query(params): axum::extract::Query<QueryParams>,
) -> Response {
    let Some(my_sub) = caller(&headers) else {
        return (
            axum::http::StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "authentication required"})),
        )
            .into_response();
    };

    let Some(hash) = params.hash else {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "hash query param required"})),
        )
            .into_response();
    };

    let now = Utc::now().timestamp();
    let holders: Vec<PeerEntry> = state
        .mesh_peers
        .iter()
        .filter(|entry| {
            let (reg, ts) = entry.value();
            now - ts < PEER_TTL_SECS
                && reg.sub != my_sub
                && reg.block_hashes.binary_search(&hash).is_ok()
        })
        .map(|entry| {
            let (reg, ts) = entry.value();
            PeerEntry {
                sub: reg.sub.clone(),
                device_name: reg.device_name.clone(),
                addresses: reg.addresses.clone(),
                port: reg.port,
                block_count: reg.block_hashes.len(),
                last_seen: format!("{ts}"),
            }
        })
        .collect();

    (axum::http::StatusCode::OK, Json(serde_json::json!({ "holders": holders }))).into_response()
}

#[derive(Debug, Deserialize)]
pub struct QueryParams {
    pub hash: Option<String>,
}

/// Register the mesh state on AppState.
pub type MeshPeerStore = Arc<DashMap<String, (PeerRegistration, i64)>>;

pub fn new_mesh_store() -> MeshPeerStore {
    Arc::new(DashMap::new())
}
