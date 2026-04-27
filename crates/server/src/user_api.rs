use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use crate::api_error::ApiError;
use crate::users::{
    CreateUserRequest, ResetPasswordRequest, UpdateSelfRequest, UpdateUserRequest,
    UserInfo, UserRole,
};
use crate::AppState;

#[allow(clippy::result_large_err)]
fn require_admin(info: &UserInfo) -> Result<(), Response> {
    if info.role != UserRole::Admin {
        return Err(ApiError::forbidden("ADMIN_REQUIRED", "Admin role required"));
    }
    Ok(())
}

pub async fn create_user(State(state): State<AppState>, axum::Json(body): axum::Json<CreateUserRequest>) -> Response {
    let info = match state.user_info(&body.username) {
        Some(i) => i,
        None => return ApiError::unauthorized(ApiError::AUTH_REQUIRED, "Not authenticated"),
    };
    if let Err(e) = require_admin(&info) {
        return e;
    }

    if body.username.trim().is_empty() || body.password.trim().is_empty() {
        return ApiError::bad_request("INVALID_INPUT", "Username and password are required");
    }

    let user = crate::users::User {
        id: uuid::Uuid::new_v4().to_string(),
        username: body.username.clone(),
        display_name: if body.display_name.is_empty() { body.username.clone() } else { body.display_name.clone() },
        email: body.email.clone(),
        role: body.role.clone(),
        created_at: chrono::Utc::now(),
        last_login: None,
        status: crate::users::UserStatus::Active,
        storage_quota_bytes: body.storage_quota_bytes,
        storage_used_bytes: 0,
        is_ldap: false,
        password_hash: Some(crate::users::hash_password(&body.password)),
    };

    match state.user_store.create_user(user).await {
        Ok(u) => match serde_json::to_value(&u) {
            Ok(v) => (StatusCode::CREATED, axum::Json(v)).into_response(),
            Err(e) => ApiError::internal("SERIALIZATION_ERROR", format!("Failed to serialize user: {}", e)),
        },
        Err(e) => match e.kind {
            crate::users::UserErrorKind::Conflict => ApiError::conflict("USER_EXISTS", e.message),
            _ => ApiError::internal("USER_CREATE_ERROR", e.message),
        },
    }
}

pub async fn list_users(State(state): State<AppState>) -> Response {
    let admin_user = state.admin_user.as_deref().unwrap_or("");
    let info = match state.user_info(admin_user) {
        Some(i) => i,
        None => return ApiError::unauthorized(ApiError::AUTH_REQUIRED, "Not authenticated"),
    };
    if let Err(e) = require_admin(&info) {
        return e;
    }

    let users = state.user_store.list_users().await;
    let serialized: Vec<serde_json::Value> = users.iter().filter_map(|u| {
        let mut v = serde_json::to_value(u).ok()?;
        if let Some(obj) = v.as_object_mut() {
            obj.remove("password_hash");
        }
        Some(v)
    }).collect();

    (StatusCode::OK, axum::Json(serde_json::json!({ "users": serialized }))).into_response()
}

pub async fn get_user(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    let admin_user = state.admin_user.as_deref().unwrap_or("");
    let info = match state.user_info(admin_user) {
        Some(i) => i,
        None => return ApiError::unauthorized(ApiError::AUTH_REQUIRED, "Not authenticated"),
    };
    if let Err(e) = require_admin(&info) {
        return e;
    }

    match state.user_store.get_user(&id).await {
        Ok(u) => {
            let mut v = match serde_json::to_value(&u) {
                Ok(v) => v,
                Err(e) => return ApiError::internal("SERIALIZATION_ERROR", format!("Failed to serialize user: {}", e)),
            };
            if let Some(obj) = v.as_object_mut() {
                obj.remove("password_hash");
            }
            (StatusCode::OK, axum::Json(v)).into_response()
        }
        Err(e) => match e.kind {
            crate::users::UserErrorKind::NotFound => ApiError::not_found("USER_NOT_FOUND", e.message),
            _ => ApiError::internal("USER_ERROR", e.message),
        },
    }
}

pub async fn update_user(
    State(state): State<AppState>,
    Path(id): Path<String>,
    axum::Json(body): axum::Json<UpdateUserRequest>,
) -> Response {
    let admin_user = state.admin_user.as_deref().unwrap_or("");
    let info = match state.user_info(admin_user) {
        Some(i) => i,
        None => return ApiError::unauthorized(ApiError::AUTH_REQUIRED, "Not authenticated"),
    };
    if let Err(e) = require_admin(&info) {
        return e;
    }

    match state.user_store.update_user(&id, body).await {
        Ok(u) => {
            let mut v = match serde_json::to_value(&u) {
                Ok(v) => v,
                Err(e) => return ApiError::internal("SERIALIZATION_ERROR", format!("Failed to serialize user: {}", e)),
            };
            if let Some(obj) = v.as_object_mut() {
                obj.remove("password_hash");
            }
            (StatusCode::OK, axum::Json(v)).into_response()
        }
        Err(e) => match e.kind {
            crate::users::UserErrorKind::NotFound => ApiError::not_found("USER_NOT_FOUND", e.message),
            crate::users::UserErrorKind::Conflict => ApiError::conflict("USER_CONFLICT", e.message),
            _ => ApiError::internal("USER_ERROR", e.message),
        },
    }
}

pub async fn delete_user(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    let admin_user = state.admin_user.as_deref().unwrap_or("");
    let info = match state.user_info(admin_user) {
        Some(i) => i,
        None => return ApiError::unauthorized(ApiError::AUTH_REQUIRED, "Not authenticated"),
    };
    if let Err(e) = require_admin(&info) {
        return e;
    }

    match state.user_store.delete_user(&id).await {
        Ok(()) => (StatusCode::OK, axum::Json(serde_json::json!({ "ok": true }))).into_response(),
        Err(e) => match e.kind {
            crate::users::UserErrorKind::NotFound => ApiError::not_found("USER_NOT_FOUND", e.message),
            _ => ApiError::internal("USER_ERROR", e.message),
        },
    }
}

pub async fn reset_password(
    State(state): State<AppState>,
    Path(id): Path<String>,
    axum::Json(body): axum::Json<ResetPasswordRequest>,
) -> Response {
    let admin_user = state.admin_user.as_deref().unwrap_or("");
    let info = match state.user_info(admin_user) {
        Some(i) => i,
        None => return ApiError::unauthorized(ApiError::AUTH_REQUIRED, "Not authenticated"),
    };
    if let Err(e) = require_admin(&info) {
        return e;
    }

    if body.new_password.trim().is_empty() {
        return ApiError::bad_request("INVALID_INPUT", "Password cannot be empty");
    }

    let hash = crate::users::hash_password(&body.new_password);
    match state.user_store.set_password(&id, &hash).await {
        Ok(()) => (StatusCode::OK, axum::Json(serde_json::json!({ "ok": true }))).into_response(),
        Err(e) => match e.kind {
            crate::users::UserErrorKind::NotFound => ApiError::not_found("USER_NOT_FOUND", e.message),
            _ => ApiError::internal("USER_ERROR", e.message),
        },
    }
}

pub async fn get_current_user(State(state): State<AppState>) -> Response {
    let username = state
        .admin_user
        .as_deref()
        .unwrap_or("anonymous");

    let user = match state.user_store.get_user_by_username(username).await {
        Ok(u) => u,
        Err(_) => {
            let v = serde_json::json!({
                "username": username,
                "role": "Admin",
                "is_admin": true,
            });
            return (StatusCode::OK, axum::Json(v)).into_response();
        }
    };

    let mut v = match serde_json::to_value(&user) {
        Ok(v) => v,
        Err(e) => return ApiError::internal("SERIALIZATION_ERROR", format!("Failed to serialize user: {}", e)),
    };
    if let Some(obj) = v.as_object_mut() {
        obj.remove("password_hash");
    }
    (StatusCode::OK, axum::Json(v)).into_response()
}

pub async fn update_current_user(
    State(state): State<AppState>,
    axum::Json(body): axum::Json<UpdateSelfRequest>,
) -> Response {
    let username = state
        .admin_user
        .as_deref()
        .unwrap_or("anonymous");

    let user = match state.user_store.get_user_by_username(username).await {
        Ok(u) => u,
        Err(_) => return ApiError::not_found("USER_NOT_FOUND", "Current user not found in user store"),
    };

    let mut updates = UpdateUserRequest::default();
    if let Some(ref display_name) = body.display_name {
        updates.display_name = Some(display_name.clone());
    }

    match state.user_store.update_user(&user.id, updates).await {
        Ok(u) => {
            let mut v = match serde_json::to_value(&u) {
                Ok(v) => v,
                Err(e) => return ApiError::internal("SERIALIZATION_ERROR", format!("Failed to serialize user: {}", e)),
            };
            if let Some(obj) = v.as_object_mut() {
                obj.remove("password_hash");
            }
            let response = if let Some(ref new_pass) = body.password {
                if new_pass.trim().is_empty() {
                    return ApiError::bad_request("INVALID_INPUT", "Password cannot be empty");
                }
                let hash = crate::users::hash_password(new_pass);
                if let Err(e) = state.user_store.set_password(&user.id, &hash).await {
                    return ApiError::internal("PASSWORD_ERROR", e.message);
                }
                v
            } else {
                v
            };
            (StatusCode::OK, axum::Json(response)).into_response()
        }
        Err(e) => ApiError::internal("USER_ERROR", e.message),
    }
}
