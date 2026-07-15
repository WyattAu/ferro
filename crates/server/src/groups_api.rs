use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use crate::AppState;
use crate::api_error::ApiError;
use ferro_server_user_mgmt::groups::{
    CreateGroupRequest, UpdateGroupRequest,
};

// ---------------------------------------------------------------------------
// Group CRUD handlers
// ---------------------------------------------------------------------------

/// Create a new group.
pub async fn create_group(
    State(state): State<AppState>,
    axum::Json(req): axum::Json<CreateGroupRequest>,
) -> Response {
    let created_by = state
        .admin_user
        .clone()
        .unwrap_or_else(|| "anonymous".to_string());

    let group = state.group_store.create(req, created_by).await;

    (
        StatusCode::CREATED,
        axum::Json(serde_json::json!({
            "id": group.id,
            "name": group.name,
            "description": group.description,
            "members": group.members,
            "created_by": group.created_by,
            "created_at": group.created_at,
        })),
    )
        .into_response()
}

/// List all groups.
pub async fn list_groups(State(state): State<AppState>) -> Response {
    let groups = state.group_store.list().await;
    let items: Vec<serde_json::Value> = groups
        .iter()
        .map(|g| {
            serde_json::json!({
                "id": g.id,
                "name": g.name,
                "description": g.description,
                "members": g.members,
                "created_by": g.created_by,
                "created_at": g.created_at,
            })
        })
        .collect();
    (StatusCode::OK, axum::Json(serde_json::json!({ "groups": items }))).into_response()
}

/// Get a group by ID.
pub async fn get_group(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    match state.group_store.get(&id).await {
        Some(group) => (
            StatusCode::OK,
            axum::Json(serde_json::json!({
                "id": group.id,
                "name": group.name,
                "description": group.description,
                "members": group.members,
                "created_by": group.created_by,
                "created_at": group.created_at,
            })),
        )
            .into_response(),
        None => ApiError::not_found(ApiError::SHARE_NOT_FOUND, "Group not found"),
    }
}

/// Update a group.
pub async fn update_group(
    State(state): State<AppState>,
    Path(id): Path<String>,
    axum::Json(req): axum::Json<UpdateGroupRequest>,
) -> Response {
    match state.group_store.update(&id, req).await {
        Some(group) => (
            StatusCode::OK,
            axum::Json(serde_json::json!({
                "id": group.id,
                "name": group.name,
                "description": group.description,
                "members": group.members,
                "created_by": group.created_by,
                "created_at": group.created_at,
            })),
        )
            .into_response(),
        None => ApiError::not_found(ApiError::SHARE_NOT_FOUND, "Group not found"),
    }
}

/// Delete a group.
pub async fn delete_group(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    if state.group_store.delete(&id).await {
        (StatusCode::NO_CONTENT, "").into_response()
    } else {
        ApiError::not_found(ApiError::SHARE_NOT_FOUND, "Group not found")
    }
}

/// Add a member to a group.
pub async fn add_group_member(
    State(state): State<AppState>,
    Path((id, username)): Path<(String, String)>,
) -> Response {
    if state.group_store.add_member(&id, &username).await {
        match state.group_store.get(&id).await {
            Some(group) => (
                StatusCode::OK,
                axum::Json(serde_json::json!({
                    "id": group.id,
                    "name": group.name,
                    "members": group.members,
                })),
            )
                .into_response(),
            None => (StatusCode::OK, axum::Json(serde_json::json!({"ok": true}))).into_response(),
        }
    } else {
        ApiError::not_found(ApiError::SHARE_NOT_FOUND, "Group not found")
    }
}

/// Remove a member from a group.
pub async fn remove_group_member(
    State(state): State<AppState>,
    Path((id, username)): Path<(String, String)>,
) -> Response {
    if state.group_store.remove_member(&id, &username).await {
        (StatusCode::NO_CONTENT, "").into_response()
    } else {
        ApiError::not_found(ApiError::SHARE_NOT_FOUND, "Group or member not found")
    }
}

/// List groups for the current user.
pub async fn list_user_groups(State(state): State<AppState>) -> Response {
    let username = state
        .admin_user
        .clone()
        .unwrap_or_else(|| "anonymous".to_string());

    let groups = state.group_store.list_user_groups(&username).await;
    let items: Vec<serde_json::Value> = groups
        .iter()
        .map(|g| {
            serde_json::json!({
                "id": g.id,
                "name": g.name,
                "description": g.description,
            })
        })
        .collect();
    (StatusCode::OK, axum::Json(serde_json::json!({ "groups": items }))).into_response()
}
