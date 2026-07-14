use axum::extract::{Extension, Path};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::net::ToSocketAddrs;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::warn;
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::AutomationState;
use crate::DbHandle;

const MAX_WEBHOOKS: usize = 100;
const MAX_DELIVERY_ATTEMPTS: u32 = 5;
const BACKOFF_BASE_SECS: u64 = 2;
const BACKOFF_MAX_SECS: u64 = 300;

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

#[derive(Clone, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct WebhookConfig {
    pub id: String,
    pub url: String,
    pub secret: String,
    pub events: Vec<String>,
    pub enabled: bool,
}

impl std::fmt::Debug for WebhookConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebhookConfig")
            .field("id", &self.id)
            .field("url", &self.url)
            .field("secret", &"[REDACTED]")
            .field("events", &self.events)
            .field("enabled", &self.enabled)
            .finish()
    }
}

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

fn calculate_backoff(attempt: u32) -> std::time::Duration {
    let base_delay = BACKOFF_BASE_SECS.saturating_pow(attempt);
    let capped = base_delay.min(BACKOFF_MAX_SECS);
    let jitter = (rand::random::<u64>() * capped) / 2;
    std::time::Duration::from_secs(capped + jitter)
}

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

pub async fn fire_webhooks(webhooks: Arc<RwLock<Vec<WebhookConfig>>>, event: WebhookEvent, db: Option<DbHandle>) {
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
        let db_clone = db.clone();
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

            if let Some(ref db) = db_clone {
                record_delivery_start(
                    db,
                    &delivery_id,
                    &hook_clone.id,
                    &event_clone.event,
                    &hook_clone.url,
                    &payload,
                );
            }

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
                        if let Some(ref db) = db_clone {
                            record_delivery_success(db, &delivery_id, resp.status().as_u16());
                        }
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
                            if let Some(ref db) = db_clone {
                                record_delivery_dead(db, &delivery_id, status, "Max attempts exceeded");
                            }
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
                            if let Some(ref db) = db_clone {
                                record_delivery_dead(db, &delivery_id, 0, &format!("Network error: {e}"));
                            }
                            return;
                        }
                    }
                }

                let delay = calculate_backoff(attempt);
                if let Some(ref db) = db_clone {
                    update_delivery_retry(db, &delivery_id, attempt + 1, &delay);
                }
                tokio::time::sleep(delay).await;
            }
        });
    }
}

pub async fn create_webhook(
    Extension(state): Extension<Arc<AutomationState>>,
    axum::Json(input): axum::Json<CreateWebhookInput>,
) -> Response {
    if input.url.is_empty() {
        return error_bad_request("url is required");
    }
    if input.events.is_empty() {
        return error_bad_request("at least one event is required");
    }

    if let Err(reason) = validate_url(&input.url) {
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
        let hooks = state.webhooks.read().await;
        if hooks.len() >= MAX_WEBHOOKS {
            return error_bad_request(&format!("Maximum number of webhooks ({}) reached", MAX_WEBHOOKS));
        }
    }

    let config = WebhookConfig {
        id: uuid::Uuid::new_v4().to_string(),
        url: input.url,
        secret: input.secret.unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
        events: input.events,
        enabled: input.enabled.unwrap_or(true),
    };

    state.webhooks.write().await.push(config.clone());

    if let Some(ref db) = state.db {
        persist_webhook_create(db, &config);
    }

    (StatusCode::CREATED, axum::Json(config)).into_response()
}

pub async fn list_webhooks(Extension(state): Extension<Arc<AutomationState>>) -> Response {
    let hooks = state.webhooks.read().await;
    (StatusCode::OK, axum::Json(hooks.clone())).into_response()
}

pub async fn delete_webhook(Extension(state): Extension<Arc<AutomationState>>, Path(id): Path<String>) -> Response {
    let mut hooks = state.webhooks.write().await;
    let before = hooks.len();
    hooks.retain(|h| h.id != id);

    if hooks.len() < before {
        if let Some(ref db) = state.db {
            persist_webhook_delete(db, &id);
        }
        (StatusCode::NO_CONTENT, "").into_response()
    } else {
        error_not_found("Webhook not found")
    }
}

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

pub fn load_webhooks_from_db(conn: &rusqlite::Connection) -> Result<Vec<WebhookConfig>, rusqlite::Error> {
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

fn record_delivery_start(db: &DbHandle, delivery_id: &str, webhook_id: &str, event: &str, url: &str, payload: &[u8]) {
    let conn = match db.lock() {
        Ok(c) => c,
        Err(e) => {
            warn!("DB lock poisoned: {e}");
            return;
        }
    };
    if let Err(e) = conn.execute(
        "INSERT INTO webhook_deliveries (id, webhook_id, event, url, status, attempt_count, max_attempts, payload) VALUES (?1, ?2, ?3, ?4, 'pending', 0, ?5, ?6)",
        params![delivery_id, webhook_id, event, url, MAX_DELIVERY_ATTEMPTS, String::from_utf8_lossy(payload)],
    ) {
        warn!("Failed to record webhook delivery start: {e}");
    }
}

fn record_delivery_success(db: &DbHandle, delivery_id: &str, status_code: u16) {
    let conn = match db.lock() {
        Ok(c) => c,
        Err(e) => {
            warn!("DB lock poisoned: {e}");
            return;
        }
    };
    if let Err(e) = conn.execute(
        "UPDATE webhook_deliveries SET status = 'delivered', status_code = ?1, delivered_at = datetime('now') WHERE id = ?2",
        params![status_code as u32, delivery_id],
    ) {
        warn!("Failed to record webhook delivery success: {e}");
    }
}

fn record_delivery_dead(db: &DbHandle, delivery_id: &str, status_code: u16, error: &str) {
    let conn = match db.lock() {
        Ok(c) => c,
        Err(e) => {
            warn!("DB lock poisoned: {e}");
            return;
        }
    };
    if let Err(e) = conn.execute(
        "UPDATE webhook_deliveries SET status = 'dead', status_code = ?1, error_message = ?2 WHERE id = ?3",
        params![status_code as u32, error, delivery_id],
    ) {
        warn!("Failed to record webhook delivery death: {e}");
    }
}

fn update_delivery_retry(db: &DbHandle, delivery_id: &str, attempt: u32, delay: &std::time::Duration) {
    let conn = match db.lock() {
        Ok(c) => c,
        Err(e) => {
            warn!("DB lock poisoned: {e}");
            return;
        }
    };
    let next_retry =
        chrono::Utc::now() + chrono::Duration::from_std(*delay).unwrap_or_else(|_| chrono::Duration::seconds(60));
    if let Err(e) = conn.execute(
        "UPDATE webhook_deliveries SET attempt_count = ?1, next_retry_at = ?2 WHERE id = ?3",
        params![attempt, next_retry.to_rfc3339(), delivery_id],
    ) {
        warn!("Failed to update webhook delivery retry: {e}");
    }
}

pub async fn list_webhook_deliveries(
    Extension(state): Extension<Arc<AutomationState>>,
    Path(webhook_id): Path<String>,
) -> Response {
    let db = match state.db {
        Some(ref db) => db.clone(),
        None => return error_internal("Database not available"),
    };

    let conn = match db.lock() {
        Ok(c) => c,
        Err(e) => return error_internal(&format!("Database lock error: {e}")),
    };

    let mut stmt = match conn.prepare(
        "SELECT id, webhook_id, event, url, status, status_code, attempt_count, max_attempts, next_retry_at, created_at, delivered_at, error_message FROM webhook_deliveries WHERE webhook_id = ?1 ORDER BY created_at DESC LIMIT 100"
    ) {
        Ok(s) => s,
        Err(e) => return error_internal(&format!("Query error: {e}")),
    };

    let rows = match stmt.query_map(params![webhook_id], |row| {
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
    }) {
        Ok(r) => r,
        Err(e) => return error_internal(&format!("Query error: {e}")),
    };

    let deliveries: Vec<DeliveryRecord> = rows.filter_map(|r| r.ok()).collect();
    (StatusCode::OK, axum::Json(deliveries)).into_response()
}

pub async fn list_dead_letters(Extension(state): Extension<Arc<AutomationState>>) -> Response {
    let db = match state.db {
        Some(ref db) => db.clone(),
        None => return error_internal("Database not available"),
    };

    let conn = match db.lock() {
        Ok(c) => c,
        Err(e) => return error_internal(&format!("Database lock error: {e}")),
    };

    let mut stmt = match conn.prepare(
        "SELECT id, webhook_id, event, url, status, status_code, attempt_count, max_attempts, next_retry_at, created_at, delivered_at, error_message FROM webhook_deliveries WHERE status = 'dead' ORDER BY created_at DESC LIMIT 100"
    ) {
        Ok(s) => s,
        Err(e) => return error_internal(&format!("Query error: {e}")),
    };

    let rows = match stmt.query_map([], |row| {
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
    }) {
        Ok(r) => r,
        Err(e) => return error_internal(&format!("Query error: {e}")),
    };

    let deliveries: Vec<DeliveryRecord> = rows.filter_map(|r| r.ok()).collect();
    (StatusCode::OK, axum::Json(deliveries)).into_response()
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
        CREATE INDEX IF NOT EXISTS idx_webhook_deliveries_status ON webhook_deliveries(status);",
    ) {
        warn!("Failed to create webhook_deliveries table: {e}");
    }
}

const MAX_URL_LENGTH: usize = 2048;
const ALLOWED_URL_SCHEMES: &[&str] = &["http", "https"];

fn is_private_ip(ip: std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(v4) => {
            let octets = v4.octets();
            octets[0] == 0
                || octets[0] == 10
                || octets[0] == 127
                || (octets[0] == 169 && octets[1] == 254)
                || (octets[0] == 172 && (octets[1] & 0xF0) == 16)
                || (octets[0] == 192 && octets[1] == 0 && octets[2] == 0)
                || (octets[0] == 192 && octets[1] == 168)
                || (octets[0] == 198 && (octets[1] & 0xFE) == 18)
                || octets[0] >= 224
        }
        std::net::IpAddr::V6(v6) => {
            v6.is_loopback()
                || (v6.segments()[0] & 0xFFC0) == 0xFE80
                || (v6.segments()[0] & 0xFE00) == 0xFC00
                || matches!(v6.to_ipv4_mapped(), Some(v4) if is_private_ip(v4.into()))
        }
    }
}

fn validate_url(url: &str) -> Result<(), String> {
    if url.len() > MAX_URL_LENGTH {
        return Err(format!("URL exceeds maximum length of {} characters", MAX_URL_LENGTH));
    }
    if url.is_empty() {
        return Err("URL must not be empty".to_string());
    }
    let parsed = url::Url::parse(url).map_err(|e| format!("Invalid URL: {}", e))?;
    let scheme = parsed.scheme();
    if !ALLOWED_URL_SCHEMES.contains(&scheme) {
        return Err(format!(
            "URL scheme '{}' is not allowed. Only http and https are permitted.",
            scheme
        ));
    }
    if !parsed.username().is_empty() {
        return Err("URL must not contain credentials (user:pass@host)".to_string());
    }
    let host = parsed.host_str().ok_or_else(|| "URL must have a host".to_string())?;
    let host_lower = host.to_lowercase();
    if host_lower == "localhost"
        || host_lower == "metadata.google.internal"
        || host_lower.ends_with(".local")
        || host_lower.ends_with(".internal")
    {
        return Err(format!("URL host '{}' is not allowed", host));
    }
    let port = parsed.port().unwrap_or(80);
    if let Ok(addrs) = format!("{}:{}", host, port).to_socket_addrs() {
        for addr in addrs {
            if is_private_ip(addr.ip()) {
                return Err(format!(
                    "URL host '{}' resolves to a private/reserved IP address, which is not allowed",
                    host
                ));
            }
        }
    }
    Ok(())
}

fn error_bad_request(msg: &str) -> Response {
    (
        StatusCode::BAD_REQUEST,
        axum::Json(serde_json::json!({
            "error": msg,
            "error_code": "BAD_REQUEST",
        })),
    )
        .into_response()
}

fn error_not_found(msg: &str) -> Response {
    (
        StatusCode::NOT_FOUND,
        axum::Json(serde_json::json!({
            "error": msg,
            "error_code": "NOT_FOUND",
        })),
    )
        .into_response()
}

fn error_internal(msg: &str) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        axum::Json(serde_json::json!({
            "error": msg,
            "error_code": "INTERNAL_ERROR",
        })),
    )
        .into_response()
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

    #[test]
    fn test_validate_url_rejects_empty() {
        assert!(validate_url("").is_err());
    }

    #[test]
    fn test_validate_url_rejects_localhost() {
        assert!(validate_url("http://localhost/webhook").is_err());
    }

    #[test]
    fn test_validate_url_accepts_https() {
        assert!(validate_url("https://example.com/webhook").is_ok());
    }
}
