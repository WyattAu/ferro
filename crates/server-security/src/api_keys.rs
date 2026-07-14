use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use ferro_auth::api_keys::{ApiKeyPermission, CreateApiKeyRequest};
use ferro_auth::users::{UserInfo, UserRole};

use crate::SecurityAppState;
use crate::error::ApiError;

#[derive(serde::Serialize)]
struct ApiKeyListItem {
    id: String,
    name: String,
    user_id: String,
    permission: ApiKeyPermission,
    created_at: chrono::DateTime<chrono::Utc>,
    expires_at: Option<chrono::DateTime<chrono::Utc>>,
    last_used_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl From<&ferro_auth::api_keys::ApiKey> for ApiKeyListItem {
    fn from(k: &ferro_auth::api_keys::ApiKey) -> Self {
        Self {
            id: k.id.clone(),
            name: k.name.clone(),
            user_id: k.user_id.clone(),
            permission: k.permission.clone(),
            created_at: k.created_at,
            expires_at: k.expires_at,
            last_used_at: k.last_used_at,
        }
    }
}

pub async fn list_api_keys<S: SecurityAppState>(
    State(state): State<S>,
    axum::Extension(user_info): axum::Extension<UserInfo>,
) -> Response {
    let keys = state.api_key_store().list_keys(&user_info.user_id).await;
    let items: Vec<ApiKeyListItem> = keys.iter().map(ApiKeyListItem::from).collect();
    (StatusCode::OK, axum::Json(items)).into_response()
}

#[derive(serde::Deserialize)]
pub struct CreateApiKeyBody {
    pub name: String,
    #[serde(default)]
    pub permission: ApiKeyPermission,
    pub expires_at: Option<String>,
}

#[derive(serde::Serialize)]
pub struct CreateApiKeyResponse {
    pub id: String,
    pub name: String,
    pub permission: ApiKeyPermission,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub raw_key: String,
}

pub async fn create_api_key<S: SecurityAppState>(
    State(state): State<S>,
    axum::Extension(user_info): axum::Extension<UserInfo>,
    axum::Json(body): axum::Json<CreateApiKeyBody>,
) -> Response {
    if user_info.role == UserRole::ReadOnly {
        return ApiError::forbidden(ApiError::ADMIN_REQUIRED, "Read-only users cannot create API keys");
    }

    if body.name.trim().is_empty() {
        return ApiError::bad_request(ApiError::INVALID_INPUT, "API key name is required");
    }

    let request = CreateApiKeyRequest {
        name: body.name,
        permission: body.permission,
        expires_at: body.expires_at,
    };

    match state.api_key_store().create_key(&user_info.user_id, request).await {
        Ok((key, raw_key)) => {
            let resp = CreateApiKeyResponse {
                id: key.id,
                name: key.name,
                permission: key.permission,
                created_at: key.created_at,
                expires_at: key.expires_at,
                raw_key,
            };
            (StatusCode::CREATED, axum::Json(resp)).into_response()
        }
        Err(e) => match e.kind {
            ferro_auth::api_keys::ApiKeyErrorKind::QuotaExceeded => ApiError::respond(
                StatusCode::PAYLOAD_TOO_LARGE,
                ApiError::API_KEY_QUOTA_EXCEEDED,
                e.message,
            ),
            _ => ApiError::internal(ApiError::INTERNAL_ERROR, e.message),
        },
    }
}

pub async fn delete_api_key<S: SecurityAppState>(
    State(state): State<S>,
    axum::Extension(user_info): axum::Extension<UserInfo>,
    Path(key_id): Path<String>,
) -> Response {
    match state.api_key_store().revoke_key(&key_id, &user_info.user_id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => match e.kind {
            ferro_auth::api_keys::ApiKeyErrorKind::NotFound => {
                ApiError::not_found(ApiError::API_KEY_NOT_FOUND, e.message)
            }
            ferro_auth::api_keys::ApiKeyErrorKind::Forbidden => {
                ApiError::forbidden(ApiError::ADMIN_REQUIRED, e.message)
            }
            _ => ApiError::internal(ApiError::INTERNAL_ERROR, e.message),
        },
    }
}
