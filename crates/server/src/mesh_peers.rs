//! LAN/WAN peer registry for mesh sync.
//!
//! Desktop clients register themselves after each sync cycle with their
//! reachable addresses. Other peers on the same network discover each other
//! to fetch blocks directly instead of round-tripping through the server.
//!
//! SECURITY (YP-MESH-TRANSPORT-001, THM-MESH-NO-EXISTENCE-ORACLE,
//! DC-MESH-001): the registry stores ONLY identity + reachability. It never
//! stores block hashes. The previous design had clients POST their complete
//! block-hash inventory every 10 minutes — an active, by-design existence
//! oracle (worse than Dropbox's passive LAN-sync leak). Clients may attach a
//! compact Bloom-filter inventory summary for future peer-to-peer
//! reconciliation; it is opaque to the registry and can only answer
//! set-membership probabilistically, never "who has hash H".

use crate::state::AppState;
use axum::Json;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
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
    /// DEPRECATED and ignored: raw block-hash inventories are no longer
    /// accepted or stored (DC-MESH-001). Present so old clients still
    /// register successfully; a warning is logged when a client sends it.
    #[serde(default)]
    pub block_hashes: Vec<String>,
    /// Opaque Bloom-filter inventory summary (base64), for future
    /// peer-to-peer reconciliation. The registry cannot invert it per-hash;
    /// it is never returned to other peers.
    #[serde(default)]
    pub inventory_bloom: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PeerEntry {
    pub sub: String,
    pub device_name: String,
    pub addresses: Vec<String>,
    pub port: u16,
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
    if !reg.block_hashes.is_empty() {
        tracing::warn!(
            device = %reg.device_name,
            count = reg.block_hashes.len(),
            "mesh: old client sent block-hash inventory; ignored (existence-oracle fix)"
        );
    }
    // Never persist hashes. Cap the Bloom summary so it cannot be abused as a
    // free-form storage channel either.
    if let Some(bloom) = &reg.inventory_bloom {
        if bloom.len() > 64 * 1024 {
            reg.inventory_bloom = None;
        }
    } else {
        reg.block_hashes.clear();
    }
    reg.block_hashes.clear();

    let key = format!("{}:{}", reg.sub, reg.device_name);
    state.mesh_peers.insert(key, (reg, Utc::now().timestamp()));

    axum::http::StatusCode::OK.into_response()
}

pub async fn list_peers(State(state): State<AppState>, headers: HeaderMap) -> Response {
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
                last_seen: format!("{ts}"),
            }
        })
        .collect();

    // Stable, content-independent ordering (DC-MESH-004: no inventory-derived
    // side channels — the old block_count sort leaked corpus sizes).
    peers.sort_by(|a, b| a.device_name.cmp(&b.device_name));
    (axum::http::StatusCode::OK, Json(serde_json::json!({ "peers": peers }))).into_response()
}

/// Block-holder lookup — RETIRED (DC-MESH-004).
///
/// Always returns an empty holder list with a constant response shape,
/// independent of whether any peer holds the queried hash: the registry no
/// longer knows who holds what, so it cannot leak it. Peers fall back to
/// server-mediated block fetch (relay-first, per YP-MESH-TRANSPORT-001).
/// Kept as an endpoint so older clients degrade gracefully instead of 404ing.
pub async fn query_block(
    State(_state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Query(_params): axum::extract::Query<QueryParams>,
) -> Response {
    let Some(_my_sub) = caller(&headers) else {
        return (
            axum::http::StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "authentication required"})),
        )
            .into_response();
    };

    (axum::http::StatusCode::OK, Json(serde_json::json!({ "holders": [] }))).into_response()
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
