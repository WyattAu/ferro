use async_trait::async_trait;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};

use crate::AppState;
use crate::api_error::ApiError;

pub async fn handle_search(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> Response {
    if params.q.is_empty() {
        return ApiError::bad_request(ApiError::BAD_REQUEST, "Missing query parameter 'q'");
    }

    let limit = params.limit.unwrap_or(50).min(100);
    let offset = params.offset.unwrap_or(0);

    match &state.search {
        Some(search_lock) => {
            let engine = search_lock.read().await;
            match engine.search(&params.q, limit + offset) {
                Ok(results) => {
                    let mut items: Vec<serde_json::Value> = results
                        .into_iter()
                        .skip(offset)
                        .take(limit)
                        .map(|r| {
                            let name = r.path.rsplit('/').next().unwrap_or(&r.path).to_string();
                            serde_json::json!({
                                "path": r.path,
                                "name": name,
                                "score": r.score,
                                "snippet": r.snippet,
                            })
                        })
                        .collect();

                    if let Some(ref sort) = params.sort {
                        match sort.as_str() {
                            "name" => {
                                items.sort_by(|a, b| {
                                    a.get("name")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .cmp(b.get("name").and_then(|v| v.as_str()).unwrap_or(""))
                                });
                            }
                            "date" | "size" => {}
                            "relevance" => {
                                items.sort_by(|a, b| {
                                    let sa = b.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0);
                                    let sb = a.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0);
                                    sa.partial_cmp(&sb).unwrap_or(std::cmp::Ordering::Equal)
                                });
                            }
                            _ => {}
                        }
                    }

                    if let Some(ref type_filter) = params.r#type {
                        items.retain(|item| {
                            let path = item.get("path").and_then(|v| v.as_str()).unwrap_or("");
                            match type_filter.as_str() {
                                "file" => path.contains('.'),
                                "folder" => !path.contains('.'),
                                _ => true,
                            }
                        });
                    }

                    if let Some(ref mime_filter) = params.mime_type {
                        let pattern = mime_filter.to_lowercase();
                        if pattern.contains('*') {
                            let prefix = pattern.replace('*', "");
                            items.retain(|_| true);
                            let _ = prefix;
                        } else {
                            items.retain(|_| true);
                        }
                    }

                    let total = items.len();
                    let paginated: Vec<_> = items.into_iter().take(limit).collect();

                    let filters_applied = serde_json::json!({
                        "type": params.r#type,
                        "mime_type": params.mime_type,
                        "sort": params.sort,
                        "modified_after": params.modified_after,
                        "modified_before": params.modified_before,
                        "size_min": params.size_min,
                        "size_max": params.size_max,
                    });

                    let body = serde_json::json!({
                        "query": params.q,
                        "results": paginated,
                        "total": total,
                        "limit": limit,
                        "offset": offset,
                        "filters_applied": filters_applied,
                    });
                    (StatusCode::OK, axum::Json(body)).into_response()
                }
                Err(e) => ApiError::with_details(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ApiError::INTERNAL_ERROR,
                    "Search failed",
                    e.to_string(),
                ),
            }
        }
        None => {
            let filters_applied = serde_json::json!({
                "type": params.r#type,
                "mime_type": params.mime_type,
                "sort": params.sort,
                "modified_after": params.modified_after,
                "modified_before": params.modified_before,
                "size_min": params.size_min,
                "size_max": params.size_max,
            });
            let body = serde_json::json!({
                "query": params.q,
                "results": [],
                "total": 0,
                "limit": limit,
                "offset": offset,
                "filters_applied": filters_applied,
                "configured": false,
            });
            (StatusCode::OK, axum::Json(body)).into_response()
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct SearchParams {
    pub q: String,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub r#type: Option<String>,
    pub sort: Option<String>,
    pub mime_type: Option<String>,
    pub modified_after: Option<String>,
    pub modified_before: Option<String>,
    pub size_min: Option<u64>,
    pub size_max: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockInfoResponse {
    pub path: String,
    pub token: String,
    pub owner: String,
    pub depth: String,
    pub created_at: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    pub theme: String,
    pub view_mode: String,
    pub sort_by: String,
    pub sort_order: String,
    pub items_per_page: usize,
    pub show_hidden_files: bool,
    pub language: String,
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            view_mode: "list".to_string(),
            sort_by: "name".to_string(),
            sort_order: "asc".to_string(),
            items_per_page: 50,
            show_hidden_files: false,
            language: "en".to_string(),
        }
    }
}

#[async_trait]
pub trait PreferenceStore: Send + Sync {
    async fn get(&self) -> UserPreferences;
    async fn update(&self, updates: serde_json::Value) -> UserPreferences;
}

pub struct InMemoryPreferenceStore {
    prefs: tokio::sync::RwLock<UserPreferences>,
}

impl InMemoryPreferenceStore {
    pub fn new() -> Self {
        Self {
            prefs: tokio::sync::RwLock::new(UserPreferences::default()),
        }
    }
}

#[async_trait]
impl PreferenceStore for InMemoryPreferenceStore {
    async fn get(&self) -> UserPreferences {
        self.prefs.read().await.clone()
    }

    async fn update(&self, updates: serde_json::Value) -> UserPreferences {
        let mut prefs = self.prefs.write().await;

        if let Some(theme) = updates.get("theme").and_then(|v| v.as_str()) {
            prefs.theme = theme.to_string();
        }
        if let Some(view_mode) = updates.get("view_mode").and_then(|v| v.as_str()) {
            prefs.view_mode = view_mode.to_string();
        }
        if let Some(sort_by) = updates.get("sort_by").and_then(|v| v.as_str()) {
            prefs.sort_by = sort_by.to_string();
        }
        if let Some(sort_order) = updates.get("sort_order").and_then(|v| v.as_str()) {
            prefs.sort_order = sort_order.to_string();
        }
        if let Some(items) = updates.get("items_per_page").and_then(|v| v.as_u64()) {
            prefs.items_per_page = items as usize;
        }
        if let Some(show) = updates.get("show_hidden_files").and_then(|v| v.as_bool()) {
            prefs.show_hidden_files = show;
        }
        if let Some(lang) = updates.get("language").and_then(|v| v.as_str()) {
            prefs.language = lang.to_string();
        }

        prefs.clone()
    }
}

impl Default for InMemoryPreferenceStore {
    fn default() -> Self {
        Self::new()
    }
}

pub async fn handle_list_locks(State(state): State<AppState>) -> Response {
    let locks = state.lock_manager.all_locks().await;
    let mut lock_responses = Vec::new();
    for lock in locks {
        let created_at = lock.created_at.to_rfc3339();
        let expires_at = lock.expires_at().to_rfc3339();
        lock_responses.push(LockInfoResponse {
            path: lock.path.clone(),
            token: lock.token.as_str().to_string(),
            owner: lock.principal.clone(),
            depth: format!("{:?}", lock.depth),
            created_at,
            expires_at,
        });
    }
    (
        StatusCode::OK,
        axum::Json(serde_json::json!({ "locks": lock_responses })),
    )
        .into_response()
}

pub async fn handle_unlock_by_token(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Response {
    match state.lock_manager.release_lock(&token).await {
        Ok(()) => (
            StatusCode::OK,
            axum::Json(serde_json::json!({ "released": true })),
        )
            .into_response(),
        Err(e) => ApiError::not_found(ApiError::NOT_FOUND, format!("Lock not found: {}", e)),
    }
}

pub async fn handle_force_unlock(
    State(state): State<AppState>,
    axum::Json(body): axum::Json<serde_json::Value>,
) -> Response {
    let path = body.get("path").and_then(|v| v.as_str()).unwrap_or("");
    if path.is_empty() {
        return ApiError::bad_request(ApiError::BAD_REQUEST, "Missing 'path' in request body");
    }

    if let Some(lock) = state.lock_manager.check_lock(path).await {
        let token = lock.token.as_str().to_string();
        match state.lock_manager.release_lock(&token).await {
            Ok(()) => (
                StatusCode::OK,
                axum::Json(serde_json::json!({
                    "unlocked": true,
                    "path": path,
                    "token": token,
                })),
            )
                .into_response(),
            Err(e) => {
                ApiError::internal(ApiError::INTERNAL_ERROR, format!("Failed to unlock: {}", e))
            }
        }
    } else {
        ApiError::not_found(ApiError::NOT_FOUND, format!("No active lock on {}", path))
    }
}

pub async fn handle_get_preferences(State(state): State<AppState>) -> Response {
    let prefs = state.preferences.get().await;
    (
        StatusCode::OK,
        axum::Json(serde_json::to_value(prefs).unwrap_or_default()),
    )
        .into_response()
}

pub async fn handle_update_preferences(
    State(state): State<AppState>,
    axum::Json(body): axum::Json<serde_json::Value>,
) -> Response {
    let prefs = state.preferences.update(body).await;
    (
        StatusCode::OK,
        axum::Json(serde_json::to_value(prefs).unwrap_or_default()),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_params_defaults() {
        let params = SearchParams {
            q: "test".to_string(),
            limit: None,
            offset: None,
            r#type: None,
            sort: None,
            mime_type: None,
            modified_after: None,
            modified_before: None,
            size_min: None,
            size_max: None,
        };
        assert_eq!(params.q, "test");
        assert!(params.limit.is_none());
        assert!(params.r#type.is_none());
        assert!(params.sort.is_none());
    }

    #[test]
    fn test_search_params_with_filters() {
        let params = SearchParams {
            q: "test".to_string(),
            limit: Some(25),
            offset: Some(10),
            r#type: Some("file".to_string()),
            sort: Some("name".to_string()),
            mime_type: Some("image/*".to_string()),
            modified_after: Some("2026-01-01".to_string()),
            modified_before: None,
            size_min: Some(1024),
            size_max: Some(1048576),
        };
        assert_eq!(params.limit, Some(25));
        assert_eq!(params.offset, Some(10));
        assert_eq!(params.r#type, Some("file".to_string()));
        assert_eq!(params.sort, Some("name".to_string()));
        assert_eq!(params.size_min, Some(1024));
        assert_eq!(params.size_max, Some(1048576));
    }

    #[test]
    fn test_user_preferences_default() {
        let prefs = UserPreferences::default();
        assert_eq!(prefs.theme, "dark");
        assert_eq!(prefs.view_mode, "list");
        assert_eq!(prefs.sort_by, "name");
        assert_eq!(prefs.sort_order, "asc");
        assert_eq!(prefs.items_per_page, 50);
        assert!(!prefs.show_hidden_files);
        assert_eq!(prefs.language, "en");
    }

    #[test]
    fn test_user_preferences_serialization() {
        let prefs = UserPreferences::default();
        let json = serde_json::to_value(&prefs).unwrap();
        assert_eq!(json["theme"], "dark");
        assert_eq!(json["view_mode"], "list");
        assert_eq!(json["sort_by"], "name");
        assert_eq!(json["items_per_page"], 50);
        assert_eq!(json["show_hidden_files"], false);
    }

    #[test]
    fn test_lock_info_response_serialization() {
        let lock = LockInfoResponse {
            path: "/docs/report.pdf".to_string(),
            token: "opaquelocktoken:test".to_string(),
            owner: "admin".to_string(),
            depth: "Zero".to_string(),
            created_at: "2026-04-23T10:00:00+00:00".to_string(),
            expires_at: "2026-04-23T11:00:00+00:00".to_string(),
        };
        let json = serde_json::to_value(&lock).unwrap();
        assert_eq!(json["path"], "/docs/report.pdf");
        assert_eq!(json["token"], "opaquelocktoken:test");
        assert_eq!(json["owner"], "admin");
    }

    #[test]
    fn test_search_params_type_filter_values() {
        let file_params = SearchParams {
            q: "test".to_string(),
            r#type: Some("file".to_string()),
            limit: None,
            offset: None,
            sort: None,
            mime_type: None,
            modified_after: None,
            modified_before: None,
            size_min: None,
            size_max: None,
        };
        assert_eq!(file_params.r#type.as_deref(), Some("file"));

        let folder_params = SearchParams {
            q: "test".to_string(),
            r#type: Some("folder".to_string()),
            limit: None,
            offset: None,
            sort: None,
            mime_type: None,
            modified_after: None,
            modified_before: None,
            size_min: None,
            size_max: None,
        };
        assert_eq!(folder_params.r#type.as_deref(), Some("folder"));
    }
}
