use axum::extract::Extension;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use std::sync::Arc;

use crate::AutomationState;
use ferro_server_integrations::push_notifications::{RegisterTokenRequest, UnregisterTokenRequest};

fn push_not_configured() -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        axum::Json(serde_json::json!({"error": "Push notifications not configured"})),
    )
        .into_response()
}

pub async fn register_push_token(
    Extension(state): Extension<Arc<AutomationState>>,
    axum::Json(req): axum::Json<RegisterTokenRequest>,
) -> Response {
    let store = match &state.push_notification_store {
        Some(store) => store,
        None => return push_not_configured(),
    };

    let store = store.read().await;
    match store.register_token(&req.user_id, &req.token, &req.platform) {
        Ok(token) => (StatusCode::CREATED, axum::Json(serde_json::json!(token))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(serde_json::json!({"error": format!("Failed to register token: {}", e)})),
        )
            .into_response(),
    }
}

pub async fn unregister_push_token(
    Extension(state): Extension<Arc<AutomationState>>,
    axum::Json(req): axum::Json<UnregisterTokenRequest>,
) -> Response {
    let store = match &state.push_notification_store {
        Some(store) => store,
        None => return push_not_configured(),
    };

    let store = store.read().await;
    match store.unregister_token(&req.token) {
        Ok(deleted) => {
            if deleted {
                (StatusCode::OK, axum::Json(serde_json::json!({"status": "removed"}))).into_response()
            } else {
                (
                    StatusCode::NOT_FOUND,
                    axum::Json(serde_json::json!({"error": "Token not found"})),
                )
                    .into_response()
            }
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(serde_json::json!({"error": format!("Failed to unregister token: {}", e)})),
        )
            .into_response(),
    }
}

pub async fn list_push_tokens(Extension(state): Extension<Arc<AutomationState>>) -> Response {
    let store = match &state.push_notification_store {
        Some(store) => store,
        None => return push_not_configured(),
    };

    let store = store.read().await;
    match store.list_tokens() {
        Ok(tokens) => (StatusCode::OK, axum::Json(serde_json::json!({"tokens": tokens}))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(serde_json::json!({"error": format!("Failed to list tokens: {}", e)})),
        )
            .into_response(),
    }
}

#[cfg(test)]
mod tests {
    use ferro_server_integrations::push_notifications::{PushNotificationStore, PushPlatform};

    fn test_db() -> common::DbHandle {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        let db: common::DbHandle = std::sync::Arc::new(std::sync::Mutex::new(conn));
        let store = PushNotificationStore::new(db.clone());
        store.init_table().unwrap();
        db
    }

    #[test]
    fn test_register_and_list_tokens() {
        let db = test_db();
        let store = PushNotificationStore::new(db);
        let token = store.register_token("user1", "abc123", &PushPlatform::Android).unwrap();
        assert_eq!(token.user_id, "user1");
        assert_eq!(token.platform, PushPlatform::Android);

        let tokens = store.list_tokens().unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].token, "abc123");
    }

    #[test]
    fn test_unregister_token() {
        let db = test_db();
        let store = PushNotificationStore::new(db);
        store.register_token("user1", "abc123", &PushPlatform::Ios).unwrap();
        let removed = store.unregister_token("abc123").unwrap();
        assert!(removed);
        let tokens = store.list_tokens().unwrap();
        assert!(tokens.is_empty());
    }

    #[test]
    fn test_list_tokens_for_user() {
        let db = test_db();
        let store = PushNotificationStore::new(db);
        store
            .register_token("user1", "token_a", &PushPlatform::Android)
            .unwrap();
        store.register_token("user2", "token_b", &PushPlatform::Ios).unwrap();
        store.register_token("user1", "token_c", &PushPlatform::Ios).unwrap();

        let user1_tokens = store.list_tokens_for_user("user1").unwrap();
        assert_eq!(user1_tokens.len(), 2);
    }

    #[test]
    fn test_platform_as_str_roundtrip() {
        assert_eq!(PushPlatform::Android.as_str(), "android");
        assert_eq!(PushPlatform::Ios.as_str(), "ios");
        assert_eq!(PushPlatform::Web.as_str(), "web");
        assert_eq!(PushPlatform::parse_platform("android"), Some(PushPlatform::Android));
        assert_eq!(PushPlatform::parse_platform("ios"), Some(PushPlatform::Ios));
        assert_eq!(PushPlatform::parse_platform("web"), Some(PushPlatform::Web));
        assert_eq!(PushPlatform::parse_platform("unknown"), None);
    }
}
