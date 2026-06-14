use axum::extract::{Path, State};
use axum::Json;
use crate::error::ScimError;
use crate::schema::*;
use crate::ScimState;

pub async fn list_users(State(_state): State<ScimState>) -> Result<Json<ScimListResponse<ScimUser>>, ScimError> {
    Ok(Json(ScimListResponse { schemas: vec!["urn:ietf:params:scim:api:messages:2.0:ListResponse".into()], total_results: 0, start_index: 1, items_per_page: 0, resources: vec![] }))
}

pub async fn create_user(State(_state): State<ScimState>, Json(user): Json<ScimUser>) -> Result<(axum::http::StatusCode, Json<ScimUser>), ScimError> {
    Ok((axum::http::StatusCode::CREATED, Json(user)))
}

pub async fn get_user(Path(_id): Path<String>, State(_state): State<ScimState>) -> Result<Json<ScimUser>, ScimError> {
    Err(ScimError::NotFound)
}

pub async fn replace_user(Path(_id): Path<String>, State(_state): State<ScimState>, Json(user): Json<ScimUser>) -> Result<Json<ScimUser>, ScimError> {
    Ok(Json(user))
}

pub async fn delete_user(Path(_id): Path<String>, State(_state): State<ScimState>) -> Result<axum::http::StatusCode, ScimError> {
    Ok(axum::http::StatusCode::NO_CONTENT)
}

pub async fn list_groups(State(state): State<ScimState>) -> Result<Json<ScimListResponse<ScimGroup>>, ScimError> {
    let groups = state.group_store.list(0, 100);
    Ok(Json(ScimListResponse { schemas: vec!["urn:ietf:params:scim:api:messages:2.0:ListResponse".into()], total_results: state.group_store.count(), start_index: 1, items_per_page: groups.len() as u32, resources: groups }))
}

pub async fn create_group(State(state): State<ScimState>, Json(group): Json<ScimGroup>) -> Result<(axum::http::StatusCode, Json<ScimGroup>), ScimError> {
    let created = state.group_store.create(group)?;
    Ok((axum::http::StatusCode::CREATED, Json(created)))
}

pub async fn get_group(Path(id): Path<String>, State(state): State<ScimState>) -> Result<Json<ScimGroup>, ScimError> {
    let group = state.group_store.get(&id)?;
    Ok(Json(group))
}

pub async fn replace_group(Path(id): Path<String>, State(state): State<ScimState>, Json(group): Json<ScimGroup>) -> Result<Json<ScimGroup>, ScimError> {
    let updated = state.group_store.update(&id, group)?;
    Ok(Json(updated))
}

pub async fn delete_group(Path(id): Path<String>, State(state): State<ScimState>) -> Result<axum::http::StatusCode, ScimError> {
    state.group_store.delete(&id)?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

pub async fn sp_config() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:ServiceProviderConfig"],
        "patch": { "supported": false },
        "bulk": { "supported": false, "maxOperations": 0, "maxPayloadSize": 0 },
        "filter": { "supported": false, "maxResults": 0 },
        "changePassword": { "supported": false },
        "sort": { "supported": false },
        "etag": { "supported": false },
    }))
}

pub async fn schemas() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Schema"],
        "totalResults": 2,
        "Resources": [
            { "id": "urn:ietf:params:scim:schemas:core:2.0:User", "name": "User" },
            { "id": "urn:ietf:params:scim:schemas:core:2.0:Group", "name": "Group" },
        ],
    }))
}

pub async fn resource_types() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:ResourceType"],
        "totalResults": 2,
        "Resources": [
            { "id": "User", "name": "User", "endpoint": "/scim/v2/Users", "schema": "urn:ietf:params:scim:schemas:core:2.0:User" },
            { "id": "Group", "name": "Group", "endpoint": "/scim/v2/Groups", "schema": "urn:ietf:params:scim:schemas:core:2.0:Group" },
        ],
    }))
}
