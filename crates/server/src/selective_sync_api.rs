use axum::extract::{Path as AxumPath, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use ferro_selective_sync::filter::PathFilter;
use ferro_selective_sync::persistence::{ProfileStore, StoreError};
use ferro_selective_sync::profile::{
    FilterPreviewRequest, FilterPreviewResponse, RuleDirection, SyncProfile, SyncRule,
};
use serde::Deserialize;

use crate::AppState;
use crate::api_error::ApiError;

#[derive(Debug, Deserialize)]
pub struct CreateProfileRequest {
    pub name: String,
    pub rules: Vec<SyncRuleInput>,
    #[serde(default)]
    pub path_prefix: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct SyncRuleInput {
    pub pattern: String,
    pub direction: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProfileRequest {
    pub name: Option<String>,
    pub rules: Option<Vec<SyncRuleInput>>,
    pub path_prefix: Option<String>,
    pub enabled: Option<bool>,
}

fn default_true() -> bool {
    true
}

fn parse_rule_input(input: SyncRuleInput) -> Result<SyncRule, Box<Response>> {
    let direction = match input.direction.as_str() {
        "include" => RuleDirection::Include,
        "exclude" => RuleDirection::Exclude,
        other => {
            return Err(Box::new(ApiError::bad_request(
                ApiError::INVALID_INPUT,
                format!(
                    "Invalid rule direction '{}'. Use 'include' or 'exclude'.",
                    other
                ),
            )));
        }
    };
    Ok(SyncRule {
        pattern: input.pattern,
        direction,
    })
}

fn store_error_response(e: StoreError) -> Response {
    match &e {
        StoreError::NotFound(id) => {
            ApiError::not_found("PROFILE_NOT_FOUND", format!("Profile not found: {}", id))
        }
        _ => {
            tracing::error!(error = %e, "profile store error");
            ApiError::internal(ApiError::INTERNAL_ERROR, "Profile store error")
        }
    }
}

fn get_or_create_store(state: &AppState) -> Result<std::sync::Arc<ProfileStore>, Box<Response>> {
    state.selective_sync_store.clone().ok_or_else(|| {
        Box::new(ApiError::service_unavailable(
            "NOT_CONFIGURED",
            "Selective sync not configured",
        ))
    })
}

pub async fn list_profiles(State(state): State<AppState>) -> Response {
    let store = match get_or_create_store(&state) {
        Ok(s) => s,
        Err(r) => return *r,
    };

    let owner = "anonymous".to_string();
    match store.list_profiles(&owner) {
        Ok(profiles) => (
            StatusCode::OK,
            axum::Json(serde_json::json!({ "profiles": profiles })),
        )
            .into_response(),
        Err(e) => store_error_response(e),
    }
}

pub async fn create_profile(
    State(state): State<AppState>,
    axum::Json(body): axum::Json<CreateProfileRequest>,
) -> Response {
    let store = match get_or_create_store(&state) {
        Ok(s) => s,
        Err(r) => return *r,
    };

    if body.name.is_empty() {
        return ApiError::bad_request(ApiError::MISSING_FIELD, "Profile name is required");
    }

    let rules: Vec<SyncRule> = match body.rules.into_iter().map(parse_rule_input).collect() {
        Ok(r) => r,
        Err(e) => return *e,
    };

    let owner = "anonymous".to_string();
    let mut profile = SyncProfile::new(body.name, owner, rules);
    profile.path_prefix = body.path_prefix;
    profile.enabled = body.enabled;

    match store.create_profile(&profile) {
        Ok(()) => (
            StatusCode::CREATED,
            axum::Json(serde_json::json!({ "profile": profile })),
        )
            .into_response(),
        Err(e) => store_error_response(e),
    }
}

pub async fn update_profile(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<String>,
    axum::Json(body): axum::Json<UpdateProfileRequest>,
) -> Response {
    let store = match get_or_create_store(&state) {
        Ok(s) => s,
        Err(r) => return *r,
    };

    let mut profile = match store.get_profile(&id) {
        Ok(p) => p,
        Err(e) => return store_error_response(e),
    };

    if let Some(name) = body.name {
        profile.name = name;
    }
    if let Some(rules_input) = body.rules {
        let rules: Vec<SyncRule> = match rules_input.into_iter().map(parse_rule_input).collect() {
            Ok(r) => r,
            Err(e) => return *e,
        };
        profile.rules = rules;
    }
    if let Some(path_prefix) = body.path_prefix {
        profile.path_prefix = Some(path_prefix);
    }
    if let Some(enabled) = body.enabled {
        profile.enabled = enabled;
    }
    profile.updated_at = chrono::Utc::now().to_rfc3339();

    match store.update_profile(&profile) {
        Ok(()) => (
            StatusCode::OK,
            axum::Json(serde_json::json!({ "profile": profile })),
        )
            .into_response(),
        Err(e) => store_error_response(e),
    }
}

pub async fn delete_profile(
    State(state): State<AppState>,
    AxumPath(id): AxumPath<String>,
) -> Response {
    let store = match get_or_create_store(&state) {
        Ok(s) => s,
        Err(r) => return *r,
    };

    match store.delete_profile(&id) {
        Ok(()) => (
            StatusCode::OK,
            axum::Json(serde_json::json!({ "status": "deleted" })),
        )
            .into_response(),
        Err(e) => store_error_response(e),
    }
}

pub async fn filter_preview(
    State(_state): State<AppState>,
    axum::Json(body): axum::Json<FilterPreviewRequest>,
) -> Response {
    let filter = match PathFilter::from_rules(&body.rules) {
        Ok(f) => f,
        Err(e) => {
            return ApiError::bad_request("INVALID_GLOB", format!("Invalid glob pattern: {}", e));
        }
    };

    let (matched_refs, missed_refs) = filter.filter_paths(&body.paths);
    let matched: Vec<String> = matched_refs.into_iter().cloned().collect();
    let missed: Vec<String> = missed_refs.into_iter().cloned().collect();

    let response = FilterPreviewResponse { matched, missed };
    (StatusCode::OK, axum::Json(response)).into_response()
}
