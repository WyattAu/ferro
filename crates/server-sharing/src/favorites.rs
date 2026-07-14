use async_trait::async_trait;
use axum::extract::Extension;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use dashmap::DashSet;
use rusqlite::params;
use serde::Deserialize;
use tracing::warn;

use crate::SharingState;
use crate::db::DbHandle;

#[async_trait]
pub trait FavoriteStore: Send + Sync {
    async fn list(&self) -> Vec<String>;
    async fn add(&self, path: String);
    async fn contains(&self, path: &str) -> bool;
    async fn remove(&self, path: &str);
}

pub struct InMemoryFavoriteStore {
    pub favorites: DashSet<String>,
    db: Option<DbHandle>,
}

const MAX_FAVORITES: usize = 10_000;

impl InMemoryFavoriteStore {
    pub fn new() -> Self {
        Self {
            favorites: DashSet::new(),
            db: None,
        }
    }

    pub fn with_db(mut self, db: DbHandle) -> Self {
        self.db = Some(db);
        self
    }

    fn persist_add(&self, path: &str) {
        if let Some(ref db) = self.db
            && let Err(e) = db
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .execute("INSERT OR IGNORE INTO favorites (path) VALUES (?1)", params![path])
        {
            warn!("Failed to persist favorite to SQLite: {}", e);
        }
    }

    fn persist_remove(&self, path: &str) {
        if let Some(ref db) = self.db
            && let Err(e) = db
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .execute("DELETE FROM favorites WHERE path = ?1", params![path])
        {
            warn!("Failed to remove favorite from SQLite: {}", e);
        }
    }

    pub fn load_all_from_db(conn: &rusqlite::Connection) -> Result<Vec<String>, rusqlite::Error> {
        let mut stmt = conn.prepare("SELECT path FROM favorites")?;
        let rows = stmt.query_map([], |row| row.get(0))?;
        let mut paths = Vec::new();
        for row in rows {
            paths.push(row?);
        }
        Ok(paths)
    }
}

#[async_trait]
impl FavoriteStore for InMemoryFavoriteStore {
    async fn list(&self) -> Vec<String> {
        self.favorites.iter().map(|r| r.key().clone()).collect()
    }

    async fn add(&self, path: String) {
        if self.favorites.len() < MAX_FAVORITES {
            self.favorites.insert(path.clone());
            self.persist_add(&path);
        }
    }

    async fn contains(&self, path: &str) -> bool {
        self.favorites.contains(path)
    }

    async fn remove(&self, path: &str) {
        self.favorites.remove(path);
        self.persist_remove(path);
    }
}

impl Default for InMemoryFavoriteStore {
    fn default() -> Self {
        Self::new()
    }
}

pub async fn list_favorites(Extension(state): Extension<SharingState>) -> Response {
    let favorites = state.favorites.list().await;
    (StatusCode::OK, axum::Json(serde_json::json!({ "paths": favorites }))).into_response()
}

#[derive(Debug, Deserialize)]
pub struct FavoritePath {
    pub path: String,
}

pub async fn add_favorite(
    Extension(state): Extension<SharingState>,
    axum::Json(body): axum::Json<FavoritePath>,
) -> Response {
    state.favorites.add(body.path).await;
    (StatusCode::OK, axum::Json(serde_json::json!({ "ok": true }))).into_response()
}

pub async fn remove_favorite(
    Extension(state): Extension<SharingState>,
    axum::Json(body): axum::Json<FavoritePath>,
) -> Response {
    state.favorites.remove(&body.path).await;
    (StatusCode::OK, axum::Json(serde_json::json!({ "ok": true }))).into_response()
}

/// List the current user's favorite paths.
///
/// Generic over `HasFavorites` for crate decomposition.
pub async fn list_favorites_impl<S: common::server_context::HasFavorites>(state: &S) -> Response {
    let favorites = state.list_favorites().await;
    (StatusCode::OK, axum::Json(serde_json::json!({ "paths": favorites }))).into_response()
}

/// Add a path to the current user's favorites.
pub async fn add_favorite_impl<S: common::server_context::HasFavorites>(state: &S, path: String) -> Response {
    state.add_favorite(path).await;
    (StatusCode::OK, axum::Json(serde_json::json!({ "ok": true }))).into_response()
}

/// Remove a path from the current user's favorites.
pub async fn remove_favorite_impl<S: common::server_context::HasFavorites>(state: &S, path: &str) -> Response {
    state.remove_favorite(path).await;
    (StatusCode::OK, axum::Json(serde_json::json!({ "ok": true }))).into_response()
}

pub async fn list_recent(Extension(state): Extension<SharingState>) -> Response {
    let entries = state.audit_log.recent_audit(50).await;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_memory_store_add_remove() {
        let store = InMemoryFavoriteStore::new();
        assert!(!futures::executor::block_on(store.contains("/test.txt")));
        futures::executor::block_on(store.add("/test.txt".to_string()));
        assert!(futures::executor::block_on(store.contains("/test.txt")));
        futures::executor::block_on(store.remove("/test.txt"));
        assert!(!futures::executor::block_on(store.contains("/test.txt")));
    }

    #[test]
    fn test_list_empty() {
        let store = InMemoryFavoriteStore::new();
        let list = futures::executor::block_on(store.list());
        assert!(list.is_empty());
    }
}
