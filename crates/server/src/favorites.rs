use async_trait::async_trait;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use dashmap::DashSet;
use serde::Deserialize;

use crate::AppState;

/// Trait for managing user favorite paths.
#[async_trait]
pub trait FavoriteStore: Send + Sync {
    async fn list(&self) -> Vec<String>;
    async fn add(&self, path: String);
    async fn contains(&self, path: &str) -> bool;
    async fn remove(&self, path: &str);
}

/// In-memory favorite store backed by a [`DashSet`].
pub struct InMemoryFavoriteStore {
    favorites: DashSet<String>,
}

const MAX_FAVORITES: usize = 10_000;

impl InMemoryFavoriteStore {
    /// Create a new empty favorite store.
    pub fn new() -> Self {
        Self {
            favorites: DashSet::new(),
        }
    }
}

#[async_trait]
impl FavoriteStore for InMemoryFavoriteStore {
    async fn list(&self) -> Vec<String> {
        self.favorites.iter().map(|r| r.key().clone()).collect()
    }

    async fn add(&self, path: String) {
        if self.favorites.len() < MAX_FAVORITES {
            self.favorites.insert(path);
        }
    }

    async fn contains(&self, path: &str) -> bool {
        self.favorites.contains(path)
    }

    async fn remove(&self, path: &str) {
        self.favorites.remove(path);
    }
}

impl Default for InMemoryFavoriteStore {
    fn default() -> Self {
        Self::new()
    }
}

/// List the current user's favorite paths.
pub async fn list_favorites(State(state): State<AppState>) -> Response {
    let favorites = state.favorites.list().await;
    (
        StatusCode::OK,
        axum::Json(serde_json::json!({ "paths": favorites })),
    )
        .into_response()
}

/// Request body for adding/removing a favorite path.
#[derive(Debug, Deserialize)]
pub struct FavoritePath {
    pub path: String,
}

/// Add a path to the current user's favorites.
pub async fn add_favorite(
    State(state): State<AppState>,
    axum::Json(body): axum::Json<FavoritePath>,
) -> Response {
    state.favorites.add(body.path).await;
    (
        StatusCode::OK,
        axum::Json(serde_json::json!({ "ok": true })),
    )
        .into_response()
}

/// Remove a path from the current user's favorites.
pub async fn remove_favorite(
    State(state): State<AppState>,
    axum::Json(body): axum::Json<FavoritePath>,
) -> Response {
    state.favorites.remove(&body.path).await;
    (
        StatusCode::OK,
        axum::Json(serde_json::json!({ "ok": true })),
    )
        .into_response()
}

/// List recently created/modified files from the audit log.
pub async fn list_recent(State(state): State<AppState>) -> Response {
    let entries = state.audit_log.recent(50).await;
    let mut seen = std::collections::HashSet::new();
    let mut recent_files: Vec<serde_json::Value> = Vec::new();

    for entry in entries.into_iter().rev() {
        if !seen.insert(entry.path.clone()) {
            continue;
        }
        if entry.path.starts_with("/api/") || entry.path.starts_with("/.well-known") {
            continue;
        }
        if entry.method == "PUT" || entry.method == "MKCOL" {
            recent_files.push(serde_json::json!({
                "path": entry.path,
                "method": entry.method,
                "timestamp": entry.timestamp,
                "user": entry.user,
            }));
        }
        if recent_files.len() >= 50 {
            break;
        }
    }

    (
        StatusCode::OK,
        axum::Json(serde_json::json!({ "files": recent_files })),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_state() -> AppState {
        AppState::in_memory()
    }

    #[tokio::test]
    async fn test_list_favorites_empty() {
        let state = test_state();
        let resp = list_favorites(State(state)).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_add_and_remove_favorite() {
        let state = test_state();

        let resp = add_favorite(
            State(state.clone()),
            axum::Json(FavoritePath {
                path: "/test.txt".to_string(),
            }),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK);

        assert!(state.favorites.contains("/test.txt").await);

        let resp = remove_favorite(
            State(state.clone()),
            axum::Json(FavoritePath {
                path: "/test.txt".to_string(),
            }),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK);

        assert!(!state.favorites.contains("/test.txt").await);
    }

    #[tokio::test]
    async fn test_list_recent_empty() {
        let state = test_state();
        let resp = list_recent(State(state)).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
