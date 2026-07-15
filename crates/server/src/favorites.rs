use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use crate::AppState;
use ferro_server_state::ServerState;

pub use ferro_server_sharing::favorites::{
    FavoritePath, FavoriteStore, InMemoryFavoriteStore, add_favorite_impl, list_favorites_impl, remove_favorite_impl,
};

/// Concrete axum handler that delegates to the generic implementation.
pub async fn list_favorites(State(state): State<AppState>) -> Response {
    list_favorites_impl(&state).await
}

pub async fn add_favorite(State(state): State<AppState>, axum::Json(body): axum::Json<FavoritePath>) -> Response {
    add_favorite_impl(&state, body.path).await
}

pub async fn remove_favorite(State(state): State<AppState>, axum::Json(body): axum::Json<FavoritePath>) -> Response {
    remove_favorite_impl(&state, &body.path).await
}

/// List recently created/modified files from the audit log.
pub async fn list_recent_impl<S: ServerState>(state: &S) -> Response {
    let entries = state.audit_log().recent(50).await;
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

    (StatusCode::OK, axum::Json(serde_json::json!({ "files": recent_files }))).into_response()
}

/// List recently created/modified files from the audit log.
pub async fn list_recent(State(state): State<AppState>) -> Response {
    list_recent_impl(&state).await
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
