use async_trait::async_trait;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};

use crate::ApiCoreState;
use crate::ApiError;

pub async fn handle_search<S: ApiCoreState>(State(state): State<S>, Query(params): Query<SearchParams>) -> Response {
    if params.q.is_empty() {
        return ApiError::bad_request(ApiError::BAD_REQUEST, "Missing query parameter 'q'");
    }

    let limit = params.limit.unwrap_or(50).min(100);
    let offset = params.offset.unwrap_or(0);

    let ranking_config = {
        let cfg = state.search_ranking_config().read().await;
        cfg.clone()
    };

    let mut text_results: Vec<serde_json::Value> = Vec::new();
    let mut search_configured = false;

    if let Some(search_lock) = state.search() {
        search_configured = true;
        let engine = search_lock.read().await;
        match engine.search_with_config(&params.q, limit + offset, &ranking_config) {
            Ok(results) => {
                text_results = results
                    .into_iter()
                    .skip(offset)
                    .take(limit)
                    .map(|r| {
                        let name = r.path.rsplit('/').next().unwrap_or(&r.path).to_string();
                        serde_json::json!({
                            "path": r.path,
                            "name": name,
                            "score": r.normalized_score,
                            "raw_score": r.score,
                            "snippet": r.snippet,
                            "highlights": r.highlights,
                            "match_locations": r.match_locations.to_vec().iter().map(|l| match l {
                                ferro_core::search::MatchLocation::Name => "name",
                                ferro_core::search::MatchLocation::Path => "path",
                                ferro_core::search::MatchLocation::Content => "content",
                                _ => "unknown",
                            }).collect::<Vec<_>>(),
                            "source": "text",
                        })
                    })
                    .collect();
            }
            Err(e) => {
                tracing::warn!("Text search engine error: {}", e);
            }
        }
    }

    let mut semantic_used = false;
    let mut semantic_results: Vec<serde_json::Value> = Vec::new();

    if let Some(ai_bridge) = state.ai_search()
        && ai_bridge.is_available()
    {
        match ai_bridge.semantic_search(&params.q, limit + offset, None) {
            Ok(results) => {
                semantic_used = true;
                semantic_results = results
                    .into_iter()
                    .map(|r| {
                        let name = r.path.rsplit('/').next().unwrap_or(&r.path).to_string();
                        serde_json::json!({
                            "path": r.path,
                            "name": name,
                            "score": r.score as f64,
                            "snippet": null,
                            "source": "semantic",
                        })
                    })
                    .collect();
            }
            Err(e) => {
                tracing::debug!("Semantic search unavailable: {}", e);
            }
        }
    }

    let mut merged: Vec<serde_json::Value> = if semantic_used && !semantic_results.is_empty() {
        let mut score_map: std::collections::HashMap<String, serde_json::Value> = std::collections::HashMap::new();

        for item in text_results {
            let path = item.get("path").and_then(|v| v.as_str()).unwrap_or("");
            score_map.insert(path.to_string(), item);
        }

        for item in &semantic_results {
            let path = item.get("path").and_then(|v| v.as_str()).unwrap_or("");
            if let Some(existing) = score_map.get_mut(path) {
                let text_score = existing.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let sem_score = item.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0);
                existing["score"] = serde_json::json!(text_score * 0.6 + sem_score * 0.4);
                existing["source"] = serde_json::json!("combined");
            } else {
                score_map.insert(path.to_string(), item.clone());
            }
        }

        let mut merged: Vec<serde_json::Value> = score_map.into_values().collect();
        merged.sort_by(|a, b| {
            let sa = b.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let sb = a.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0);
            sa.partial_cmp(&sb).unwrap_or(std::cmp::Ordering::Equal)
        });
        merged
    } else {
        text_results
    };

    if let Some(sort) = params.sort.clone() {
        match sort.as_str() {
            "name" => {
                merged.sort_by(|a, b| {
                    a.get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .cmp(b.get("name").and_then(|v| v.as_str()).unwrap_or(""))
                });
            }
            "relevance" => {
                merged.sort_by(|a, b| {
                    let sa = b.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let sb = a.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    sa.partial_cmp(&sb).unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            "date" | "size" => {}
            _ => {}
        }
    }

    if let Some(type_filter) = params.r#type.clone() {
        merged.retain(|item| {
            let path = item.get("path").and_then(|v| v.as_str()).unwrap_or("");
            match type_filter.as_str() {
                "file" => path.contains('.'),
                "folder" => !path.contains('.'),
                _ => true,
            }
        });
    }

    let total = merged.len();
    let paginated: Vec<_> = merged.into_iter().take(limit).collect();

    let filters_applied = serde_json::json!({
        "type": params.r#type,
        "mime_type": params.mime_type,
        "sort": params.sort,
        "modified_after": params.modified_after,
        "modified_before": params.modified_before,
        "size_min": params.size_min,
        "size_max": params.size_max,
    });

    let ranking_info = serde_json::json!({
        "file_name_boost": ranking_config.file_name_boost,
        "path_boost": ranking_config.path_boost,
        "content_boost": ranking_config.content_boost,
        "recent_file_boost": ranking_config.recent_file_boost,
        "recent_file_threshold_days": ranking_config.recent_file_threshold_days,
        "document_type_boost": ranking_config.document_type_boost,
    });

    let body = serde_json::json!({
        "query": params.q,
        "results": paginated,
        "total": total,
        "limit": limit,
        "offset": offset,
        "filters_applied": filters_applied,
        "semantic_search": semantic_used,
        "ranking": ranking_info,
    });

    if !search_configured && !semantic_used {
        let body = serde_json::json!({
            "query": params.q,
            "results": [],
            "total": 0,
            "limit": limit,
            "offset": offset,
            "filters_applied": filters_applied,
            "configured": false,
            "semantic_search": false,
        });
        (StatusCode::OK, axum::Json(body)).into_response()
    } else {
        (StatusCode::OK, axum::Json(body)).into_response()
    }
}

pub async fn handle_get_search_config<S: ApiCoreState>(State(state): State<S>) -> Response {
    let config = state.search_ranking_config().read().await;
    (
        StatusCode::OK,
        axum::Json(serde_json::to_value(&*config).unwrap_or_default()),
    )
        .into_response()
}

pub async fn handle_update_search_config<S: ApiCoreState>(
    State(state): State<S>,
    axum::Json(body): axum::Json<serde_json::Value>,
) -> Response {
    let mut config = state.search_ranking_config().write().await;
    if let Some(v) = body.get("file_name_boost").and_then(|v| v.as_f64()) {
        config.file_name_boost = v;
    }
    if let Some(v) = body.get("path_boost").and_then(|v| v.as_f64()) {
        config.path_boost = v;
    }
    if let Some(v) = body.get("content_boost").and_then(|v| v.as_f64()) {
        config.content_boost = v;
    }
    if let Some(v) = body.get("recent_file_boost").and_then(|v| v.as_f64()) {
        config.recent_file_boost = v;
    }
    if let Some(v) = body.get("recent_file_threshold_days").and_then(|v| v.as_u64()) {
        config.recent_file_threshold_days = v;
    }
    if let Some(v) = body.get("document_type_boost").and_then(|v| v.as_f64()) {
        config.document_type_boost = v;
    }
    let response = serde_json::to_value(&*config).unwrap_or_default();
    (StatusCode::OK, axum::Json(response)).into_response()
}

pub async fn handle_reindex<S: ApiCoreState>(State(state): State<S>) -> Response {
    if let Some(search_lock) = state.search() {
        let mut engine = search_lock.write().await;
        match engine.commit() {
            Ok(()) => (
                StatusCode::OK,
                axum::Json(serde_json::json!({
                    "reindexed": true,
                    "message": "Search index committed and reader reloaded"
                })),
            )
                .into_response(),
            Err(e) => ApiError::internal(ApiError::INTERNAL_ERROR, format!("Reindex failed: {}", e)),
        }
    } else {
        ApiError::bad_request(ApiError::BAD_REQUEST, "Search engine not configured")
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

pub async fn handle_list_locks<S: ApiCoreState>(State(state): State<S>) -> Response {
    let locks = state.lock_manager().all_locks().await;
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

pub async fn handle_unlock_by_token<S: ApiCoreState>(State(state): State<S>, Path(token): Path<String>) -> Response {
    match state.lock_manager().release_lock(&token).await {
        Ok(()) => (StatusCode::OK, axum::Json(serde_json::json!({ "released": true }))).into_response(),
        Err(e) => ApiError::not_found(ApiError::NOT_FOUND, format!("Lock not found: {}", e)),
    }
}

pub async fn handle_force_unlock<S: ApiCoreState>(
    State(state): State<S>,
    axum::Json(body): axum::Json<serde_json::Value>,
) -> Response {
    let path = body.get("path").and_then(|v| v.as_str()).unwrap_or("");
    if path.is_empty() {
        return ApiError::bad_request(ApiError::BAD_REQUEST, "Missing 'path' in request body");
    }

    if let Some(lock) = state.lock_manager().check_lock(path).await {
        let token = lock.token.as_str().to_string();
        match state.lock_manager().release_lock(&token).await {
            Ok(()) => (
                StatusCode::OK,
                axum::Json(serde_json::json!({
                    "unlocked": true,
                    "path": path,
                    "token": token,
                })),
            )
                .into_response(),
            Err(e) => ApiError::internal(ApiError::INTERNAL_ERROR, format!("Failed to unlock: {}", e)),
        }
    } else {
        ApiError::not_found(ApiError::NOT_FOUND, format!("No active lock on {}", path))
    }
}

pub async fn handle_get_preferences<S: ApiCoreState>(State(state): State<S>) -> Response {
    let prefs = state.preferences().get().await;
    (
        StatusCode::OK,
        axum::Json(serde_json::to_value(prefs).unwrap_or_default()),
    )
        .into_response()
}

pub async fn handle_update_preferences<S: ApiCoreState>(
    State(state): State<S>,
    axum::Json(body): axum::Json<serde_json::Value>,
) -> Response {
    let prefs = state.preferences().update(body).await;
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
