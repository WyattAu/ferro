use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::warn;

use crate::AppState;
use crate::api_error::ApiError;
use crate::db::DbHandle;

const MAX_WEBHOOKS: usize = 100;

static WEBHOOK_CLIENT: std::sync::LazyLock<reqwest::Client> = std::sync::LazyLock::new(|| {
    reqwest::Client::builder()
        .pool_max_idle_per_host(10)
        .pool_idle_timeout(std::time::Duration::from_secs(90))
        .timeout(std::time::Duration::from_secs(30))
        .connect_timeout(std::time::Duration::from_secs(10))
        .build()
        .expect("Failed to build webhook HTTP client")
});

/// Configuration for a webhook subscription.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    pub id: String,
    pub url: String,
    pub secret: String,
    pub events: Vec<String>,
    pub enabled: bool,
}

/// A webhook event payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEvent {
    pub event: String,
    pub timestamp: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub etag: Option<String>,
}

/// Compute an HMAC-SHA256 signature for webhook payload verification.
pub fn sign_payload(secret: &str, payload: &[u8]) -> String {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    type HmacSha256 = Hmac<Sha256>;

    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .expect("HMAC key initialization failed — this is a ring library invariant");
    mac.update(payload);
    let result = mac.finalize();
    hex::encode(result.into_bytes())
}

/// Fire matching webhooks for an event with retry logic.
pub async fn fire_webhooks(webhooks: Arc<RwLock<Vec<WebhookConfig>>>, event: WebhookEvent) {
    let hooks = {
        let guard = webhooks.read().await;
        guard
            .iter()
            .filter(|h| h.enabled && h.events.contains(&event.event))
            .cloned()
            .collect::<Vec<_>>()
    };

    for hook in hooks {
        let hook_clone = hook.clone();
        let event_clone = event.clone();
        tokio::spawn(async move {
            let payload = match serde_json::to_vec(&event_clone) {
                Ok(p) => p,
                Err(e) => {
                    tracing::warn!("Failed to serialize webhook payload: {}", e);
                    return;
                }
            };

            let signature = sign_payload(&hook_clone.secret, &payload);

            let client = &WEBHOOK_CLIENT;
            let mut delay = std::time::Duration::from_secs(1);

            for attempt in 0..3 {
                let result = client
                    .post(&hook_clone.url)
                    .header("Content-Type", "application/json")
                    .header("X-Ferro-Signature", format!("sha256={}", signature))
                    .header("X-Ferro-Event", &event_clone.event)
                    .body(payload.clone())
                    .send()
                    .await;

                match result {
                    Ok(resp) if resp.status().is_success() => {
                        tracing::debug!(
                            webhook_id = %hook_clone.id,
                            event = %event_clone.event,
                            "Webhook delivered successfully"
                        );
                        return;
                    }
                    Ok(resp) => {
                        tracing::warn!(
                            webhook_id = %hook_clone.id,
                            attempt = attempt + 1,
                            status = resp.status().as_u16(),
                            "Webhook delivery failed"
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            webhook_id = %hook_clone.id,
                            attempt = attempt + 1,
                            error = %e,
                            "Webhook delivery error"
                        );
                    }
                }

                if attempt < 2 {
                    tokio::time::sleep(delay).await;
                    delay *= 2;
                }
            }
        });
    }
}

/// POST /api/admin/webhooks — create a webhook subscription.
pub async fn create_webhook(
    State(state): State<AppState>,
    axum::Json(input): axum::Json<CreateWebhookInput>,
) -> Response {
    if input.url.is_empty() {
        return ApiError::bad_request(ApiError::BAD_REQUEST, "url is required");
    }
    if input.events.is_empty() {
        return ApiError::bad_request(ApiError::BAD_REQUEST, "at least one event is required");
    }

    {
        let hooks = state.webhooks.read().await;
        if hooks.len() >= MAX_WEBHOOKS {
            return ApiError::bad_request(
                "WEBHOOK_LIMIT_REACHED",
                format!("Maximum number of webhooks ({}) reached", MAX_WEBHOOKS),
            );
        }
    }

    let config = WebhookConfig {
        id: uuid::Uuid::new_v4().to_string(),
        url: input.url,
        secret: input
            .secret
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
        events: input.events,
        enabled: input.enabled.unwrap_or(true),
    };

    state.webhooks.write().await.push(config.clone());

    if let Some(ref db) = state.db {
        persist_webhook_create(db, &config);
    }

    (StatusCode::CREATED, axum::Json(config)).into_response()
}

/// GET /api/admin/webhooks — list all webhook subscriptions.
pub async fn list_webhooks(State(state): State<AppState>) -> Response {
    let hooks = state.webhooks.read().await;
    (StatusCode::OK, axum::Json(hooks.clone())).into_response()
}

/// DELETE /api/admin/webhooks/:id — delete a webhook subscription.
pub async fn delete_webhook(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    let mut hooks = state.webhooks.write().await;
    let before = hooks.len();
    hooks.retain(|h| h.id != id);

    if hooks.len() < before {
        if let Some(ref db) = state.db {
            persist_webhook_delete(db, &id);
        }
        (StatusCode::NO_CONTENT, "").into_response()
    } else {
        ApiError::not_found(ApiError::NOT_FOUND, "Webhook not found")
    }
}

/// Request body for creating a webhook.
#[derive(Debug, Deserialize)]
pub struct CreateWebhookInput {
    pub url: String,
    pub secret: Option<String>,
    pub events: Vec<String>,
    pub enabled: Option<bool>,
}

pub fn persist_webhook_create(db: &DbHandle, config: &WebhookConfig) {
    let conn = db.lock().unwrap_or_else(|e| e.into_inner());
    if let Err(e) = conn.execute(
        "INSERT OR REPLACE INTO webhooks (id, url, events, secret, enabled) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            config.id,
            config.url,
            serde_json::to_string(&config.events).unwrap_or_default(),
            config.secret,
            config.enabled as i32,
        ],
    ) {
        warn!("Failed to persist webhook to SQLite: {}", e);
    }
}

pub fn persist_webhook_delete(db: &DbHandle, id: &str) {
    let conn = db.lock().unwrap_or_else(|e| e.into_inner());
    if let Err(e) = conn.execute("DELETE FROM webhooks WHERE id = ?1", params![id]) {
        warn!("Failed to delete webhook from SQLite: {}", e);
    }
}

pub fn load_webhooks_from_db(
    conn: &rusqlite::Connection,
) -> Result<Vec<WebhookConfig>, rusqlite::Error> {
    let mut stmt = conn.prepare("SELECT id, url, events, secret, enabled FROM webhooks")?;
    let rows = stmt.query_map([], |row| {
        let events_json: String = row.get(2)?;
        let events: Vec<String> = serde_json::from_str(&events_json).unwrap_or_default();
        Ok(WebhookConfig {
            id: row.get(0)?,
            url: row.get(1)?,
            secret: row.get(3)?,
            events,
            enabled: row.get::<_, i32>(4)? != 0,
        })
    })?;
    let mut hooks = Vec::new();
    for row in rows {
        hooks.push(row?);
    }
    Ok(hooks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AppState;
    use crate::build_router;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    async fn body_json(response: axum::response::Response) -> serde_json::Value {
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn test_create_webhook() {
        let app = build_router(AppState::in_memory());
        let resp = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/admin/webhooks")
                    .header("Content-Type", "application/json")
                    .body(axum::body::Body::from(
                        serde_json::json!({
                            "url": "https://example.com/hook",
                            "events": ["file.upload"]
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::CREATED);
        let json = body_json(resp).await;
        assert_eq!(json["url"], "https://example.com/hook");
        assert!(json["id"].is_string());
        assert!(json["secret"].is_string());
        assert_eq!(json["events"][0], "file.upload");
        assert_eq!(json["enabled"], true);
    }

    #[tokio::test]
    async fn test_create_webhook_missing_url() {
        let app = build_router(AppState::in_memory());
        let resp = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/admin/webhooks")
                    .header("Content-Type", "application/json")
                    .body(axum::body::Body::from(
                        serde_json::json!({
                            "url": "",
                            "events": ["file.upload"]
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_create_and_list_webhooks() {
        let app = build_router(AppState::in_memory());

        app.clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/admin/webhooks")
                    .header("Content-Type", "application/json")
                    .body(axum::body::Body::from(
                        serde_json::json!({
                            "url": "https://example.com/hook1",
                            "events": ["file.upload", "file.delete"]
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        app.clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/admin/webhooks")
                    .header("Content-Type", "application/json")
                    .body(axum::body::Body::from(
                        serde_json::json!({
                            "url": "https://example.com/hook2",
                            "events": ["file.upload"]
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        let resp = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/admin/webhooks")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert!(json.is_array());
        assert_eq!(json.as_array().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_delete_webhook() {
        let app = build_router(AppState::in_memory());

        let create_resp = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/admin/webhooks")
                    .header("Content-Type", "application/json")
                    .body(axum::body::Body::from(
                        serde_json::json!({
                            "url": "https://example.com/to-delete",
                            "events": ["file.upload"]
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        let id = body_json(create_resp).await["id"]
            .as_str()
            .unwrap()
            .to_string();

        let del_resp = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/admin/webhooks/{}", id))
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(del_resp.status(), StatusCode::NO_CONTENT);

        let list_resp = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/admin/webhooks")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let json = body_json(list_resp).await;
        assert_eq!(json.as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_delete_webhook_route_matching() {
        let app = build_router(AppState::in_memory());

        let resp = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("DELETE")
                    .uri("/api/admin/webhooks/test-id")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let status = resp.status();
        let ct = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("none")
            .to_string();
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let body = String::from_utf8_lossy(&bytes);
        assert!(
            body.contains("Webhook not found") || status == StatusCode::NO_CONTENT,
            "Expected handler response, got status={} ct={} body={}",
            status,
            ct,
            &body[..body.len().min(200)],
        );
    }

    #[tokio::test]
    async fn test_minimal_routing_no_catchall() {
        use axum::routing::delete;
        let app = crate::Router::new()
            .route(
                "/api/test/:id",
                delete(|Path(id): Path<String>| async move {
                    (
                        axum::http::StatusCode::NO_CONTENT,
                        format!("deleted {}", id),
                    )
                        .into_response()
                }),
            )
            .with_state(AppState::in_memory());

        let resp = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("DELETE")
                    .uri("/api/test/some-id")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), axum::http::StatusCode::NO_CONTENT);
    }

    #[test]
    fn test_sign_payload_deterministic() {
        let sig1 = sign_payload("secret", b"payload");
        let sig2 = sign_payload("secret", b"payload");
        assert_eq!(sig1, sig2);
        assert!(!sig1.is_empty());
    }

    #[test]
    fn test_sign_payload_different_secrets() {
        let sig1 = sign_payload("secret-a", b"payload");
        let sig2 = sign_payload("secret-b", b"payload");
        assert_ne!(sig1, sig2);
    }
}
