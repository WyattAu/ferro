use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::warn;

use crate::ApiCoreState;
use crate::ApiError;
use crate::DbHandle;

const MAX_WEBHOOKS: usize = 100;

/// Maximum delivery attempts before moving to dead letter queue.
const MAX_DELIVERY_ATTEMPTS: u32 = 5;

/// Base delay for exponential backoff (seconds).
const BACKOFF_BASE_SECS: u64 = 2;

/// Maximum backoff delay (seconds).
const BACKOFF_MAX_SECS: u64 = 300;

// ---------------------------------------------------------------------------
// WebhookDeliveryStore
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct WebhookDeliveryStore {
    db: Option<DbHandle>,
}

impl Default for WebhookDeliveryStore {
    fn default() -> Self {
        Self::new()
    }
}

impl WebhookDeliveryStore {
    pub fn new() -> Self {
        Self { db: None }
    }

    pub fn with_db(mut self, db: DbHandle) -> Self {
        self.db = Some(db);
        self
    }

    pub fn record_start(
        &self,
        id: &str,
        webhook_id: &str,
        event: &str,
        url: &str,
        attempt_count: u32,
        max_attempts: u32,
        payload: &str,
    ) -> Result<(), String> {
        let Some(ref db) = self.db else {
            return Ok(());
        };
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "INSERT INTO webhook_deliveries (id, webhook_id, event, url, status, attempt_count, max_attempts, payload) VALUES (?1, ?2, ?3, ?4, 'pending', ?5, ?6, ?7)",
            params![id, webhook_id, event, url, attempt_count, max_attempts, payload],
        )
        .map_err(|e| format!("Failed to record webhook delivery start: {e}"))?;
        Ok(())
    }

    pub fn record_success(&self, id: &str, status_code: u16) -> Result<(), String> {
        let Some(ref db) = self.db else {
            return Ok(());
        };
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "UPDATE webhook_deliveries SET status = 'delivered', status_code = ?1, delivered_at = datetime('now') WHERE id = ?2",
            params![status_code as u32, id],
        )
        .map_err(|e| format!("Failed to record webhook delivery success: {e}"))?;
        Ok(())
    }

    pub fn record_dead(&self, id: &str, status_code: u16, error: &str) -> Result<(), String> {
        let Some(ref db) = self.db else {
            return Ok(());
        };
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "UPDATE webhook_deliveries SET status = 'dead', status_code = ?1, error_message = ?2 WHERE id = ?3",
            params![status_code as u32, error, id],
        )
        .map_err(|e| format!("Failed to record webhook delivery death: {e}"))?;
        Ok(())
    }

    pub fn update_retry(
        &self,
        id: &str,
        attempt_count: u32,
        next_retry_at: &str,
    ) -> Result<(), String> {
        let Some(ref db) = self.db else {
            return Ok(());
        };
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "UPDATE webhook_deliveries SET attempt_count = ?1, next_retry_at = ?2 WHERE id = ?3",
            params![attempt_count, next_retry_at, id],
        )
        .map_err(|e| format!("Failed to update webhook delivery retry: {e}"))?;
        Ok(())
    }

    pub fn list_deliveries(&self, webhook_id: &str) -> Result<Vec<DeliveryRecord>, String> {
        let Some(ref db) = self.db else {
            return Ok(Vec::new());
        };
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn
            .prepare(
                "SELECT id, webhook_id, event, url, status, status_code, attempt_count, max_attempts, next_retry_at, created_at, delivered_at, error_message FROM webhook_deliveries WHERE webhook_id = ?1 ORDER BY created_at DESC LIMIT 100",
            )
            .map_err(|e| format!("Query error: {e}"))?;
        let rows = stmt
            .query_map(params![webhook_id], |row| {
                Ok(DeliveryRecord {
                    id: row.get(0)?,
                    webhook_id: row.get(1)?,
                    event: row.get(2)?,
                    url: row.get(3)?,
                    status: row.get(4)?,
                    status_code: row.get::<_, Option<u32>>(5)?.map(|v| v as u16),
                    attempt_count: row.get::<_, u32>(6)?,
                    max_attempts: row.get::<_, u32>(7)?,
                    next_retry_at: row.get(8)?,
                    created_at: row.get(9)?,
                    delivered_at: row.get(10)?,
                    error_message: row.get(11)?,
                })
            })
            .map_err(|e| format!("Query error: {e}"))?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn list_dead_letters(&self) -> Result<Vec<DeliveryRecord>, String> {
        let Some(ref db) = self.db else {
            return Ok(Vec::new());
        };
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn
            .prepare(
                "SELECT id, webhook_id, event, url, status, status_code, attempt_count, max_attempts, next_retry_at, created_at, delivered_at, error_message FROM webhook_deliveries WHERE status = 'dead' ORDER BY created_at DESC LIMIT 100",
            )
            .map_err(|e| format!("Query error: {e}"))?;
        let rows = stmt
            .query_map([], |row| {
                Ok(DeliveryRecord {
                    id: row.get(0)?,
                    webhook_id: row.get(1)?,
                    event: row.get(2)?,
                    url: row.get(3)?,
                    status: row.get(4)?,
                    status_code: row.get::<_, Option<u32>>(5)?.map(|v| v as u16),
                    attempt_count: row.get::<_, u32>(6)?,
                    max_attempts: row.get::<_, u32>(7)?,
                    next_retry_at: row.get(8)?,
                    created_at: row.get(9)?,
                    delivered_at: row.get(10)?,
                    error_message: row.get(11)?,
                })
            })
            .map_err(|e| format!("Query error: {e}"))?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }
}

static WEBHOOK_CLIENT: std::sync::LazyLock<reqwest::Client> = std::sync::LazyLock::new(|| {
    reqwest::Client::builder()
        .pool_max_idle_per_host(10)
        .pool_idle_timeout(std::time::Duration::from_secs(90))
        .timeout(std::time::Duration::from_secs(30))
        .connect_timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_else(|e| {
            tracing::error!("Failed to build webhook HTTP client: {e}");
            reqwest::Client::new()
        })
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

/// A webhook delivery attempt record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryRecord {
    pub id: String,
    pub webhook_id: String,
    pub event: String,
    pub url: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_code: Option<u16>,
    pub attempt_count: u32,
    pub max_attempts: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_retry_at: Option<String>,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delivered_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

/// Calculate exponential backoff delay with jitter.
fn calculate_backoff(attempt: u32) -> std::time::Duration {
    let base_delay = BACKOFF_BASE_SECS.saturating_pow(attempt);
    let capped = base_delay.min(BACKOFF_MAX_SECS);
    let jitter = (rand::random::<u64>() * capped) / 2;
    std::time::Duration::from_secs(capped + jitter)
}

/// Compute an HMAC-SHA256 signature for webhook payload verification.
pub fn sign_payload(secret: &str, payload: &[u8]) -> String {
    use hmac::{Hmac, KeyInit, Mac};
    use sha2::Sha256;

    type HmacSha256 = Hmac<Sha256>;

    let mut mac = match HmacSha256::new_from_slice(secret.as_bytes()) {
        Ok(m) => m,
        Err(_) => {
            tracing::error!("webhook secret is too long for HMAC key — skipping signature");
            return String::new();
        }
    };
    mac.update(payload);
    let result = mac.finalize();
    hex::encode(result.into_bytes())
}

/// Fire matching webhooks for an event with retry logic and delivery tracking.
pub async fn fire_webhooks(
    webhooks: Arc<RwLock<Vec<WebhookConfig>>>,
    event: WebhookEvent,
    delivery_store: WebhookDeliveryStore,
) {
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
        let store_clone = delivery_store.clone();
        tokio::spawn(async move {
            let payload = match serde_json::to_vec(&event_clone) {
                Ok(p) => p,
                Err(e) => {
                    tracing::warn!("Failed to serialize webhook payload: {}", e);
                    return;
                }
            };

            let signature = sign_payload(&hook_clone.secret, &payload);
            let delivery_id = uuid::Uuid::new_v4().to_string();

            let payload_str = String::from_utf8_lossy(&payload).to_string();
            let _ = store_clone.record_start(
                &delivery_id,
                &hook_clone.id,
                &event_clone.event,
                &hook_clone.url,
                0,
                MAX_DELIVERY_ATTEMPTS,
                &payload_str,
            );

            let client = &WEBHOOK_CLIENT;

            for attempt in 0..MAX_DELIVERY_ATTEMPTS {
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
                            attempt = attempt + 1,
                            "Webhook delivered successfully"
                        );
                        let _ = store_clone.record_success(&delivery_id, resp.status().as_u16());
                        return;
                    }
                    Ok(resp) => {
                        let status = resp.status().as_u16();
                        tracing::warn!(
                            webhook_id = %hook_clone.id,
                            attempt = attempt + 1,
                            status = status,
                            "Webhook delivery failed"
                        );
                        if attempt == MAX_DELIVERY_ATTEMPTS - 1 {
                            let _ = store_clone.record_dead(
                                &delivery_id,
                                status,
                                "Max attempts exceeded",
                            );
                            return;
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            webhook_id = %hook_clone.id,
                            attempt = attempt + 1,
                            error = %e,
                            "Webhook delivery error"
                        );
                        if attempt == MAX_DELIVERY_ATTEMPTS - 1 {
                            let _ = store_clone.record_dead(
                                &delivery_id,
                                0,
                                &format!("Network error: {e}"),
                            );
                            return;
                        }
                    }
                }

                let delay = calculate_backoff(attempt);
                let next_retry = chrono::Utc::now()
                    + chrono::Duration::from_std(delay)
                        .unwrap_or_else(|_| chrono::Duration::seconds(60));
                let _ =
                    store_clone.update_retry(&delivery_id, attempt + 1, &next_retry.to_rfc3339());
                tokio::time::sleep(delay).await;
            }
        });
    }
}

/// POST /api/admin/webhooks — create a webhook subscription.
pub async fn create_webhook<S: ApiCoreState>(
    State(state): State<S>,
    axum::Json(input): axum::Json<CreateWebhookInput>,
) -> Response {
    if input.url.is_empty() {
        return ApiError::bad_request(ApiError::BAD_REQUEST, "url is required");
    }
    if input.events.is_empty() {
        return ApiError::bad_request(ApiError::BAD_REQUEST, "at least one event is required");
    }

    // Validate URL scheme, length, and SSRF protection.
    if let Err(reason) = crate::validate_url(&input.url) {
        return (
            StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({
                "error": "INVALID_URL",
                "message": reason,
            })),
        )
            .into_response();
    }

    {
        let hooks = state.webhooks().read().await;
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

    state.webhooks().write().await.push(config.clone());

    if let Some(db) = state.db() {
        persist_webhook_create(db, &config);
    }

    (StatusCode::CREATED, axum::Json(config)).into_response()
}

/// GET /api/admin/webhooks — list all webhook subscriptions.
pub async fn list_webhooks<S: ApiCoreState>(State(state): State<S>) -> Response {
    let hooks = state.webhooks().read().await;
    (StatusCode::OK, axum::Json(hooks.clone())).into_response()
}

/// DELETE /api/admin/webhooks/:id — delete a webhook subscription.
pub async fn delete_webhook<S: ApiCoreState>(
    State(state): State<S>,
    Path(id): Path<String>,
) -> Response {
    let mut hooks = state.webhooks().write().await;
    let before = hooks.len();
    hooks.retain(|h| h.id != id);

    if hooks.len() < before {
        if let Some(db) = state.db() {
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

pub fn create_webhook_delivery_tables(conn: &rusqlite::Connection) {
    if let Err(e) = conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS webhook_deliveries (
            id TEXT PRIMARY KEY,
            webhook_id TEXT NOT NULL,
            event TEXT NOT NULL,
            url TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending',
            status_code INTEGER,
            attempt_count INTEGER NOT NULL DEFAULT 0,
            max_attempts INTEGER NOT NULL DEFAULT 5,
            next_retry_at TEXT,
            payload TEXT NOT NULL,
            response_body TEXT,
            error_message TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            delivered_at TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_webhook_deliveries_webhook_id ON webhook_deliveries(webhook_id);
        CREATE INDEX IF NOT EXISTS idx_webhook_deliveries_status ON webhook_deliveries(status);"
    ) {
        warn!("Failed to create webhook_deliveries table: {e}");
    }
}

/// GET /api/admin/webhooks/:id/deliveries — list delivery history for a webhook.
pub async fn list_webhook_deliveries<S: ApiCoreState>(
    State(state): State<S>,
    Path(webhook_id): Path<String>,
) -> Response {
    match state.webhook_delivery_store().list_deliveries(&webhook_id) {
        Ok(deliveries) => (StatusCode::OK, axum::Json(deliveries)).into_response(),
        Err(e) => ApiError::internal("INTERNAL_ERROR", format!("Query error: {e}")),
    }
}

/// GET /api/admin/webhooks/deliveries/dead — list dead letter queue entries.
pub async fn list_dead_letters<S: ApiCoreState>(State(state): State<S>) -> Response {
    match state.webhook_delivery_store().list_dead_letters() {
        Ok(deliveries) => (StatusCode::OK, axum::Json(deliveries)).into_response(),
        Err(e) => ApiError::internal("INTERNAL_ERROR", format!("Query error: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
