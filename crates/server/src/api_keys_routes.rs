use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use crate::AppState;
use crate::api_error::ApiError;
use crate::users::UserInfo;
use ferro_auth::api_keys::ApiKeyStoreTrait;
use ferro_auth::api_keys::{ApiKeyPermission, CreateApiKeyRequest};

/// Response for listing API keys (raw key is never included).
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

/// GET /api/v1/api-keys — list all API keys for the authenticated user.
pub async fn list_api_keys(
    State(state): State<AppState>,
    axum::Extension(user_info): axum::Extension<UserInfo>,
) -> Response {
    let keys = state.api_key_store.list_keys(&user_info.user_id).await;
    let items: Vec<ApiKeyListItem> = keys.iter().map(ApiKeyListItem::from).collect();
    (StatusCode::OK, axum::Json(items)).into_response()
}

/// Request body for creating an API key.
#[derive(serde::Deserialize)]
pub struct CreateApiKeyBody {
    /// Human-readable name for the key.
    pub name: String,
    /// Permission level: "Read", "Write", or "Admin". Defaults to "Read".
    #[serde(default)]
    pub permission: ApiKeyPermission,
    /// Optional expiration as ISO 8601 datetime string.
    pub expires_at: Option<String>,
}

/// Response returned after creating an API key (includes the raw key once).
#[derive(serde::Serialize)]
pub struct CreateApiKeyResponse {
    pub id: String,
    pub name: String,
    pub permission: ApiKeyPermission,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    /// The raw API key. This is the only time it is shown — it cannot be recovered.
    pub raw_key: String,
}

/// POST /api/v1/api-keys — create a new API key.
pub async fn create_api_key(
    State(state): State<AppState>,
    axum::Extension(user_info): axum::Extension<UserInfo>,
    axum::Json(body): axum::Json<CreateApiKeyBody>,
) -> Response {
    if body.name.trim().is_empty() {
        return ApiError::bad_request(ApiError::INVALID_INPUT, "API key name is required");
    }

    let request = CreateApiKeyRequest {
        name: body.name,
        permission: body.permission,
        expires_at: body.expires_at,
    };

    match state
        .api_key_store
        .create_key(&user_info.user_id, request)
        .await
    {
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

/// DELETE /api/v1/api-keys/:id — revoke (delete) an API key.
pub async fn delete_api_key(
    State(state): State<AppState>,
    axum::Extension(user_info): axum::Extension<UserInfo>,
    Path(key_id): Path<String>,
) -> Response {
    match state
        .api_key_store
        .revoke_key(&key_id, &user_info.user_id)
        .await
    {
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    fn make_user(user_id: &str) -> UserInfo {
        UserInfo {
            user_id: user_id.to_string(),
            username: user_id.to_string(),
            role: crate::users::UserRole::Admin,
        }
    }

    fn make_read_user(user_id: &str) -> UserInfo {
        UserInfo {
            user_id: user_id.to_string(),
            username: user_id.to_string(),
            role: crate::users::UserRole::ReadOnly,
        }
    }

    fn test_app() -> axum::Router {
        let state = AppState::in_memory().with_wopi_token_secret("test".to_string());
        let user_info = make_user("user1");
        axum::Router::new()
            .route(
                "/api/v1/api-keys",
                axum::routing::get(list_api_keys).post(create_api_key),
            )
            .route(
                "/api/v1/api-keys/:id",
                axum::routing::delete(delete_api_key),
            )
            .layer(axum::Extension(user_info))
            .with_state(state)
    }

    #[tokio::test]
    async fn test_list_keys_empty() {
        let app = test_app();
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/api-keys")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json.as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_create_key() {
        let app = test_app();
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/api-keys")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_string(&serde_json::json!({
                            "name": "test-key",
                            "permission": "Read"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["name"], "test-key");
        assert!(json["raw_key"].as_str().unwrap().starts_with("ferro_"));
        assert!(!json["id"].as_str().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_create_key_empty_name() {
        let app = test_app();
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/api-keys")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_string(&serde_json::json!({
                            "name": "  ",
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_create_and_list_key() {
        let app = test_app();
        // Create a key
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/api-keys")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_string(&serde_json::json!({
                            "name": "my-key",
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);

        // List keys
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/api-keys")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let arr = json.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["name"], "my-key");
        assert!(
            arr[0].get("raw_key").is_none(),
            "raw_key should not be in list response"
        );
    }

    #[tokio::test]
    async fn test_create_and_delete_key() {
        let app = test_app();
        // Create a key
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/api-keys")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_string(&serde_json::json!({
                            "name": "delete-me",
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let key_id = json["id"].as_str().unwrap().to_string();

        // Delete it
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/v1/api-keys/{}", key_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);

        // Verify it's gone
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/api-keys")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json.as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_delete_nonexistent_key() {
        let app = test_app();
        let resp = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/v1/api-keys/nonexistent-id")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_read_only_user_cannot_create_key() {
        let state = AppState::in_memory().with_wopi_token_secret("test".to_string());
        let user_info = make_read_user("user1");
        let app = axum::Router::new()
            .route(
                "/api/v1/api-keys",
                axum::routing::get(list_api_keys).post(create_api_key),
            )
            .layer(axum::Extension(user_info))
            .with_state(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/api-keys")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_string(&serde_json::json!({
                            "name": "test-key",
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_api_key_auth_flow() {
        let state = AppState::in_memory().with_wopi_token_secret("test".to_string());
        let api_key_store = state.api_key_store.clone();

        // Create an API key directly via the store
        let (key, raw_key) = api_key_store
            .create_key(
                "user1",
                CreateApiKeyRequest {
                    name: "auth-test".to_string(),
                    permission: ApiKeyPermission::Read,
                    expires_at: None,
                },
            )
            .await
            .unwrap();

        // Verify the key authenticates
        let auth_result = api_key_store.authenticate(&raw_key).await;
        assert!(auth_result.is_ok());
        let authed_key = auth_result.unwrap();
        assert_eq!(authed_key.id, key.id);
        assert!(authed_key.last_used_at.is_some());
    }
}
