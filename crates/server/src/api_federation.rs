use std::sync::Arc;

use axum::extract::{Path as AxumPath, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::AppState;
use crate::api_error::ApiError;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationToken {
    pub token: String,
    pub peer_url: String,
    pub granted_at: String,
    pub expires_at: String,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustedPeer {
    pub url: String,
    pub name: String,
    pub added_at: String,
    pub active: bool,
}

#[derive(Debug, Deserialize)]
pub struct ExchangeTokenRequest {
    pub peer_url: String,
    pub permissions: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct ExchangeTokenResponse {
    pub token: String,
    pub peer_url: String,
    pub expires_at: String,
}

#[derive(Debug, Deserialize)]
pub struct AddPeerRequest {
    pub url: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct FedSearchQuery {
    pub q: String,
    pub peer: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct FedSearchResponse {
    pub results: Vec<FedSearchResult>,
    pub source: String,
}

#[derive(Debug, Serialize)]
pub struct FedSearchResult {
    pub path: String,
    pub peer: String,
    pub score: f64,
}

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct FederationTokenStore {
    pub tokens: Arc<DashMap<String, FederationToken>>,
    pub peers: Arc<RwLock<Vec<TrustedPeer>>>,
    pub federation_secret: String,
}

impl FederationTokenStore {
    pub fn new(federation_secret: String) -> Self {
        Self {
            tokens: Arc::new(DashMap::new()),
            peers: Arc::new(RwLock::new(Vec::new())),
            federation_secret,
        }
    }

    pub async fn create_token(&self, peer_url: &str, permissions: Vec<String>) -> FederationToken {
        let token = format!(
            "fed_{}_{}",
            uuid::Uuid::new_v4(),
            &self.federation_secret[..8.min(self.federation_secret.len())]
        );
        let now = chrono::Utc::now();
        let expires = now + chrono::Duration::hours(24);
        let ft = FederationToken {
            token: token.clone(),
            peer_url: peer_url.to_string(),
            granted_at: now.to_rfc3339(),
            expires_at: expires.to_rfc3339(),
            permissions,
        };
        self.tokens.insert(token.clone(), ft.clone());
        ft
    }

    pub async fn validate_token(&self, token: &str) -> Option<FederationToken> {
        let ft = self.tokens.get(token)?;
        if chrono::Utc::now()
            > chrono::DateTime::parse_from_rfc3339(&ft.expires_at)
                .ok()
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_default()
        {
            self.tokens.remove(token);
            return None;
        }
        Some(ft.clone())
    }

    pub async fn add_peer(&self, url: String, name: String) -> TrustedPeer {
        let peer = TrustedPeer {
            url: url.clone(),
            name,
            added_at: chrono::Utc::now().to_rfc3339(),
            active: true,
        };
        let mut peers = self.peers.write().await;
        if !peers.iter().any(|p| p.url == url) {
            peers.push(peer.clone());
        }
        peer
    }

    pub async fn remove_peer(&self, url: &str) -> bool {
        let mut peers = self.peers.write().await;
        let before = peers.len();
        peers.retain(|p| p.url != url);
        peers.len() < before
    }

    pub async fn list_peers(&self) -> Vec<TrustedPeer> {
        self.peers.read().await.clone()
    }

    pub async fn is_trusted(&self, peer_url: &str) -> bool {
        let peers = self.peers.read().await;
        peers.iter().any(|p| p.url == peer_url && p.active)
    }
}

// ---------------------------------------------------------------------------
// Shared state helper
// ---------------------------------------------------------------------------

fn federation_store(state: &AppState) -> FederationTokenStore {
    FederationTokenStore {
        tokens: Arc::new(DashMap::new()),
        peers: Arc::new(tokio::sync::RwLock::new(Vec::new())),
        federation_secret: state.federation_secret.clone(),
    }
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// POST /api/v1/fed/exchange-token — exchange credentials for a federation token.
pub async fn exchange_token(
    State(state): State<AppState>,
    axum::Json(req): axum::Json<ExchangeTokenRequest>,
) -> Response {
    let store = federation_store(&state);
    let permissions = req.permissions.unwrap_or_default();
    let ft = store.create_token(&req.peer_url, permissions).await;
    axum::Json(ExchangeTokenResponse {
        token: ft.token,
        peer_url: ft.peer_url,
        expires_at: ft.expires_at,
    })
    .into_response()
}

/// GET /api/v1/fed/files/{path} — proxy file content from a trusted peer.
pub async fn get_fed_file(
    State(state): State<AppState>,
    AxumPath(path): AxumPath<String>,
    headers: HeaderMap,
) -> Response {
    let token_header = headers
        .get("x-federation-token")
        .and_then(|v| v.to_str().ok());

    let token = match token_header {
        Some(t) => t,
        None => {
            return ApiError::unauthorized(
                "FED_AUTH_REQUIRED",
                "X-Federation-Token header required",
            );
        }
    };

    let store = federation_store(&state);
    match store.validate_token(token).await {
        Some(ft) if ft.permissions.contains(&"read".to_string()) => {
            // Proxy to the peer's storage
            let client = reqwest::Client::new();
            let url = format!("{}/dav/{}", ft.peer_url, path);
            match client.get(&url).send().await {
                Ok(resp) => {
                    let status = StatusCode::from_u16(resp.status().as_u16())
                        .unwrap_or(StatusCode::BAD_GATEWAY);
                    match resp.bytes().await {
                        Ok(body) => (status, body).into_response(),
                        Err(_) => {
                            ApiError::bad_gateway("FED_PROXY_ERROR", "Failed to read peer response")
                        }
                    }
                }
                Err(_) => ApiError::bad_gateway("FED_PEER_UNREACHABLE", "Could not reach peer"),
            }
        }
        Some(_) => ApiError::forbidden("FED_FORBIDDEN", "Token lacks read permission"),
        None => ApiError::unauthorized("FED_TOKEN_INVALID", "Invalid or expired token"),
    }
}

/// PUT /api/v1/fed/files/{path} — write file content via federation proxy.
pub async fn put_fed_file(
    State(state): State<AppState>,
    AxumPath(path): AxumPath<String>,
    headers: HeaderMap,
    body: axum::body::Body,
) -> Response {
    let token_header = headers
        .get("x-federation-token")
        .and_then(|v| v.to_str().ok());

    let token = match token_header {
        Some(t) => t,
        None => {
            return ApiError::unauthorized(
                "FED_AUTH_REQUIRED",
                "X-Federation-Token header required",
            );
        }
    };

    let store = federation_store(&state);
    match store.validate_token(token).await {
        Some(ft) if ft.permissions.contains(&"write".to_string()) => {
            let body_bytes = match http_body_util::BodyExt::collect(body).await {
                Ok(b) => b.to_bytes(),
                Err(_) => {
                    return ApiError::bad_request("FED_BODY_ERROR", "Failed to read request body");
                }
            };
            let client = reqwest::Client::new();
            let url = format!("{}/dav/{}", ft.peer_url, path);
            match client.put(&url).body(body_bytes).send().await {
                Ok(resp) => {
                    let status = StatusCode::from_u16(resp.status().as_u16())
                        .unwrap_or(StatusCode::BAD_GATEWAY);
                    match resp.bytes().await {
                        Ok(body) => (status, body).into_response(),
                        Err(_) => {
                            ApiError::bad_gateway("FED_PROXY_ERROR", "Failed to read peer response")
                        }
                    }
                }
                Err(_) => ApiError::bad_gateway("FED_PEER_UNREACHABLE", "Could not reach peer"),
            }
        }
        Some(_) => ApiError::forbidden("FED_FORBIDDEN", "Token lacks write permission"),
        None => ApiError::unauthorized("FED_TOKEN_INVALID", "Invalid or expired token"),
    }
}

/// DELETE /api/v1/fed/files/{path} — delete file via federation proxy.
pub async fn delete_fed_file(
    State(state): State<AppState>,
    AxumPath(path): AxumPath<String>,
    headers: HeaderMap,
) -> Response {
    let token_header = headers
        .get("x-federation-token")
        .and_then(|v| v.to_str().ok());

    let token = match token_header {
        Some(t) => t,
        None => {
            return ApiError::unauthorized(
                "FED_AUTH_REQUIRED",
                "X-Federation-Token header required",
            );
        }
    };

    let store = federation_store(&state);
    match store.validate_token(token).await {
        Some(ft) if ft.permissions.contains(&"delete".to_string()) => {
            let client = reqwest::Client::new();
            let url = format!("{}/dav/{}", ft.peer_url, path);
            match client.delete(&url).send().await {
                Ok(resp) => {
                    let status = StatusCode::from_u16(resp.status().as_u16())
                        .unwrap_or(StatusCode::BAD_GATEWAY);
                    (status, "").into_response()
                }
                Err(_) => ApiError::bad_gateway("FED_PEER_UNREACHABLE", "Could not reach peer"),
            }
        }
        Some(_) => ApiError::forbidden("FED_FORBIDDEN", "Token lacks delete permission"),
        None => ApiError::unauthorized("FED_TOKEN_INVALID", "Invalid or expired token"),
    }
}

/// GET /api/v1/fed/search?q=... — federated search across trusted peers.
pub async fn federated_search(
    State(state): State<AppState>,
    axum::extract::Query(query): axum::extract::Query<FedSearchQuery>,
) -> Response {
    let store = federation_store(&state);
    let peers = store.list_peers().await;
    let active_peers: Vec<&TrustedPeer> = peers.iter().filter(|p| p.active).collect();

    if active_peers.is_empty() {
        return axum::Json(FedSearchResponse {
            results: Vec::new(),
            source: "local".to_string(),
        })
        .into_response();
    }

    let limit = query.limit.unwrap_or(20);
    let mut all_results = Vec::new();
    let client = reqwest::Client::new();

    for peer in &active_peers {
        if let Some(ref peer_filter) = query.peer
            && peer_filter != &peer.url
        {
            continue;
        }
        let url = format!("{}/api/v1/search?q={}", peer.url, query.q);
        if let Ok(resp) = client.get(&url).send().await
            && let Ok(body) = resp.json::<serde_json::Value>().await
            && let Some(items) = body.get("results").and_then(|v| v.as_array())
        {
            for item in items.iter().take(limit - all_results.len()) {
                all_results.push(FedSearchResult {
                    path: item
                        .get("path")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    peer: peer.url.clone(),
                    score: item.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0),
                });
            }
        }
    }

    all_results.truncate(limit);
    axum::Json(FedSearchResponse {
        results: all_results,
        source: "federated".to_string(),
    })
    .into_response()
}

/// GET /api/v1/fed/peers — list trusted peers.
pub async fn list_peers(State(state): State<AppState>) -> Response {
    let store = federation_store(&state);
    let peers = store.list_peers().await;
    axum::Json(serde_json::json!({ "peers": peers })).into_response()
}

/// POST /api/v1/fed/peers — add a trusted peer.
pub async fn add_peer(
    State(state): State<AppState>,
    axum::Json(req): axum::Json<AddPeerRequest>,
) -> Response {
    let store = federation_store(&state);
    let peer = store.add_peer(req.url, req.name).await;
    (StatusCode::CREATED, axum::Json(peer)).into_response()
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/fed/exchange-token", axum::routing::post(exchange_token))
        .route(
            "/fed/files/{*path}",
            axum::routing::get(get_fed_file)
                .put(put_fed_file)
                .delete(delete_fed_file),
        )
        .route("/fed/search", axum::routing::get(federated_search))
        .route("/fed/peers", axum::routing::get(list_peers).post(add_peer))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_token_roundtrip() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let store = FederationTokenStore::new("secret123".to_string());
            let ft = store
                .create_token("https://peer1.example.com", vec!["read".into()])
                .await;
            assert!(ft.token.starts_with("fed_"));
            let validated = store.validate_token(&ft.token).await;
            assert!(validated.is_some());
            assert_eq!(validated.unwrap().peer_url, "https://peer1.example.com");
        });
    }

    #[test]
    fn store_peer_management() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let store = FederationTokenStore::new("secret".to_string());
            let _p1 = store
                .add_peer("https://a.example.com".into(), "Peer A".into())
                .await;
            let _p2 = store
                .add_peer("https://b.example.com".into(), "Peer B".into())
                .await;
            assert_eq!(store.list_peers().await.len(), 2);
            assert!(store.is_trusted("https://a.example.com").await);
            store.remove_peer("https://a.example.com").await;
            assert_eq!(store.list_peers().await.len(), 1);
            assert!(!store.is_trusted("https://a.example.com").await);
        });
    }

    #[test]
    fn exchange_token_handler_returns_token() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let state = AppState::in_memory();
            let req = ExchangeTokenRequest {
                peer_url: "https://peer.test".to_string(),
                permissions: Some(vec!["read".to_string()]),
            };
            let resp = exchange_token(State(state), axum::Json(req)).await;
            assert_eq!(resp.status(), StatusCode::OK);
        });
    }

    #[test]
    fn add_peer_handler_returns_created() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let state = AppState::in_memory();
            let req = AddPeerRequest {
                url: "https://peer.test".to_string(),
                name: "Test Peer".to_string(),
            };
            let resp = add_peer(State(state), axum::Json(req)).await;
            assert_eq!(resp.status(), StatusCode::CREATED);
        });
    }
}
