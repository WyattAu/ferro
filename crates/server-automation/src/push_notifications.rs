use axum::extract::Extension;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::AutomationState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushToken {
    pub id: i64,
    pub user_id: String,
    pub token: String,
    pub platform: PushPlatform,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PushPlatform {
    Android,
    Ios,
    Web,
}

impl PushPlatform {
    pub fn as_str(&self) -> &'static str {
        match self {
            PushPlatform::Android => "android",
            PushPlatform::Ios => "ios",
            PushPlatform::Web => "web",
        }
    }

    pub fn parse_platform(s: &str) -> Option<Self> {
        match s {
            "android" => Some(PushPlatform::Android),
            "ios" => Some(PushPlatform::Ios),
            "web" => Some(PushPlatform::Web),
            _ => None,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct RegisterTokenRequest {
    pub token: String,
    pub platform: PushPlatform,
    #[serde(default = "default_user_id")]
    pub user_id: String,
}

fn default_user_id() -> String {
    "default".to_string()
}

#[derive(Debug, Deserialize)]
pub struct UnregisterTokenRequest {
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPayload {
    pub title: String,
    pub body: String,
    pub data: Option<serde_json::Value>,
}

#[derive(Clone, Default, Zeroize, ZeroizeOnDrop)]
pub struct PushNotificationConfig {
    pub fcm_server_key: Option<String>,
    pub apns_key_path: Option<String>,
    pub apns_team_id: Option<String>,
    pub apns_bundle_id: String,
    pub apns_production: bool,
}

impl std::fmt::Debug for PushNotificationConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PushNotificationConfig")
            .field("fcm_server_key", &self.fcm_server_key.as_ref().map(|_| "[REDACTED]"))
            .field("apns_key_path", &self.apns_key_path)
            .field("apns_team_id", &self.apns_team_id)
            .field("apns_bundle_id", &self.apns_bundle_id)
            .field("apns_production", &self.apns_production)
            .finish()
    }
}

pub struct PushNotificationStore {
    db: crate::DbHandle,
}

impl PushNotificationStore {
    pub fn new(db: crate::DbHandle) -> Self {
        Self { db }
    }

    pub fn init_table(&self) -> Result<(), rusqlite::Error> {
        let conn = self.db.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS push_tokens (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id TEXT NOT NULL,
                token TEXT NOT NULL UNIQUE,
                platform TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE INDEX IF NOT EXISTS idx_push_tokens_user ON push_tokens(user_id);
            CREATE INDEX IF NOT EXISTS idx_push_tokens_platform ON push_tokens(platform);",
        )?;
        Ok(())
    }

    pub fn register_token(
        &self,
        user_id: &str,
        token: &str,
        platform: &PushPlatform,
    ) -> Result<PushToken, rusqlite::Error> {
        let conn = self.db.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "INSERT OR REPLACE INTO push_tokens (user_id, token, platform, created_at)
             VALUES (?1, ?2, ?3, datetime('now'))",
            params![user_id, token, platform.as_str()],
        )?;
        let id = conn.last_insert_rowid();
        Ok(PushToken {
            id,
            user_id: user_id.to_string(),
            token: token.to_string(),
            platform: platform.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    pub fn unregister_token(&self, token: &str) -> Result<bool, rusqlite::Error> {
        let conn = self.db.lock().unwrap_or_else(|e| e.into_inner());
        let affected = conn.execute("DELETE FROM push_tokens WHERE token = ?1", params![token])?;
        Ok(affected > 0)
    }

    pub fn list_tokens(&self) -> Result<Vec<PushToken>, rusqlite::Error> {
        let conn = self.db.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt =
            conn.prepare("SELECT id, user_id, token, platform, created_at FROM push_tokens ORDER BY created_at DESC")?;
        let rows = stmt.query_map([], |row| {
            let platform_str: String = row.get(3)?;
            Ok(PushToken {
                id: row.get(0)?,
                user_id: row.get(1)?,
                token: row.get(2)?,
                platform: PushPlatform::parse_platform(&platform_str).unwrap_or(PushPlatform::Web),
                created_at: row.get(4)?,
            })
        })?;
        rows.collect()
    }

    pub fn list_tokens_for_user(&self, user_id: &str) -> Result<Vec<PushToken>, rusqlite::Error> {
        let conn = self.db.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt =
            conn.prepare("SELECT id, user_id, token, platform, created_at FROM push_tokens WHERE user_id = ?1")?;
        let rows = stmt.query_map(params![user_id], |row| {
            let platform_str: String = row.get(3)?;
            Ok(PushToken {
                id: row.get(0)?,
                user_id: row.get(1)?,
                token: row.get(2)?,
                platform: PushPlatform::parse_platform(&platform_str).unwrap_or(PushPlatform::Web),
                created_at: row.get(4)?,
            })
        })?;
        rows.collect()
    }

    pub fn list_tokens_by_platform(&self, platform: &PushPlatform) -> Result<Vec<PushToken>, rusqlite::Error> {
        let conn = self.db.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt =
            conn.prepare("SELECT id, user_id, token, platform, created_at FROM push_tokens WHERE platform = ?1")?;
        let rows = stmt.query_map(params![platform.as_str()], |row| {
            let platform_str: String = row.get(3)?;
            Ok(PushToken {
                id: row.get(0)?,
                user_id: row.get(1)?,
                token: row.get(2)?,
                platform: PushPlatform::parse_platform(&platform_str).unwrap_or(PushPlatform::Web),
                created_at: row.get(4)?,
            })
        })?;
        rows.collect()
    }

    pub fn cleanup_stale_tokens(&self) -> Result<usize, rusqlite::Error> {
        let conn = self.db.lock().unwrap_or_else(|e| e.into_inner());
        let affected = conn.execute(
            "DELETE FROM push_tokens WHERE created_at < datetime('now', '-90 days')",
            [],
        )?;
        Ok(affected)
    }
}

pub struct FcmClient {
    server_key: String,
    http_client: reqwest::Client,
}

impl FcmClient {
    pub fn new(server_key: String) -> Self {
        Self {
            server_key,
            http_client: reqwest::Client::new(),
        }
    }

    pub async fn send(&self, token: &str, payload: &NotificationPayload) -> Result<(), PushError> {
        let body = serde_json::json!({
            "to": token,
            "notification": {
                "title": payload.title,
                "body": payload.body,
            },
            "data": payload.data,
            "priority": "high",
        });

        let response = self
            .http_client
            .post("https://fcm.googleapis.com/fcm/send")
            .header("Authorization", format!("key={}", self.server_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| PushError::Network(format!("FCM request failed: {}", e)))?;

        if response.status().is_success() {
            Ok(())
        } else {
            let status = response.status();
            let text = response.text().await.unwrap_or_else(|_| "unknown error".to_string());
            Err(PushError::Provider(format!("FCM returned {}: {}", status, text)))
        }
    }
}

pub struct ApnsClient {
    key_path: String,
    team_id: String,
    bundle_id: String,
    production: bool,
    http_client: reqwest::Client,
}

impl ApnsClient {
    pub fn new(key_path: String, team_id: String, bundle_id: String, production: bool) -> Self {
        Self {
            key_path,
            team_id,
            bundle_id,
            production,
            http_client: reqwest::Client::new(),
        }
    }

    fn base_url(&self) -> &str {
        if self.production {
            "https://api.push.apple.com/3/device"
        } else {
            "https://api.sandbox.push.apple.com/3/device"
        }
    }

    async fn generate_token(&self) -> Result<String, PushError> {
        let key_path = self.key_path.clone();
        let key_bytes = tokio::task::spawn_blocking(move || std::fs::read(&key_path))
            .await
            .map_err(|e| PushError::Config(format!("Failed to spawn APNS key read: {}", e)))?
            .map_err(|e| PushError::Config(format!("Failed to read APNS key: {}", e)))?;

        let header = serde_json::json!({
            "alg": "ES256",
            "kid": self.team_id
        });
        let payload = serde_json::json!({
            "iss": self.team_id,
            "iat": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        });

        let header_b64 = base64::Engine::encode(
            &base64::engine::general_purpose::URL_SAFE_NO_PAD,
            serde_json::to_string(&header).unwrap_or_default(),
        );
        let payload_b64 = base64::Engine::encode(
            &base64::engine::general_purpose::URL_SAFE_NO_PAD,
            serde_json::to_string(&payload).unwrap_or_default(),
        );
        let signing_input = format!("{}.{}", header_b64, payload_b64);

        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(signing_input.as_bytes());
        let hash = hasher.finalize();

        let key_slice = if key_bytes.len() >= 32 {
            &key_bytes[..32]
        } else {
            &key_bytes
        };

        let mut sig = Vec::with_capacity(64);
        sig.extend_from_slice(&hash[..32]);
        sig.extend_from_slice(key_slice.get(32..64).unwrap_or(&[0u8; 32]));

        let sig_b64 = base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, &sig);

        Ok(format!("{}.{}.{}", header_b64, payload_b64, sig_b64))
    }

    pub async fn send(&self, device_token: &str, payload: &NotificationPayload) -> Result<(), PushError> {
        let jwt = self.generate_token().await?;

        let body = serde_json::json!({
            "aps": {
                "alert": {
                    "title": payload.title,
                    "body": payload.body,
                },
                "badge": 1,
                "sound": "default",
            },
            "data": payload.data,
        });

        let url = format!("{}/{}", self.base_url(), device_token);

        let response = self
            .http_client
            .post(&url)
            .header("authorization", format!("bearer {}", jwt))
            .header("apns-topic", &self.bundle_id)
            .header("apns-push-type", "alert")
            .header("apns-priority", "10")
            .json(&body)
            .send()
            .await
            .map_err(|e| PushError::Network(format!("APNS request failed: {}", e)))?;

        if response.status().is_success() {
            Ok(())
        } else {
            let status = response.status();
            let text = response.text().await.unwrap_or_else(|_| "unknown error".to_string());
            Err(PushError::Provider(format!("APNS returned {}: {}", status, text)))
        }
    }
}

#[derive(Debug)]
pub enum PushError {
    Network(String),
    Provider(String),
    Config(String),
}

impl std::fmt::Display for PushError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PushError::Network(msg) => write!(f, "network error: {}", msg),
            PushError::Provider(msg) => write!(f, "push provider error: {}", msg),
            PushError::Config(msg) => write!(f, "push config error: {}", msg),
        }
    }
}

impl std::error::Error for PushError {}

pub async fn dispatch_push_notifications(
    store: &Arc<RwLock<PushNotificationStore>>,
    config: &PushNotificationConfig,
    user_id: &str,
    event_type: &str,
    path: &str,
) {
    let notification = NotificationPayload {
        title: format!("File {}", event_type),
        body: format!("{} was {}", path, event_type),
        data: Some(serde_json::json!({
            "event_type": event_type,
            "path": path,
        })),
    };

    let tokens = {
        let store = store.read().await;
        match store.list_tokens_for_user(user_id) {
            Ok(tokens) => tokens,
            Err(e) => {
                tracing::warn!("Failed to list push tokens for user {}: {}", user_id, e);
                return;
            }
        }
    };

    for push_token in &tokens {
        let result = match push_token.platform {
            PushPlatform::Android => {
                if let Some(ref fcm_key) = config.fcm_server_key {
                    let client = FcmClient::new(fcm_key.clone());
                    client.send(&push_token.token, &notification).await
                } else {
                    tracing::debug!("FCM server key not configured, skipping Android push");
                    continue;
                }
            }
            PushPlatform::Ios => {
                if let Some(ref key_path) = config.apns_key_path {
                    let client = ApnsClient::new(
                        key_path.clone(),
                        config.apns_team_id.clone().unwrap_or_default(),
                        config.apns_bundle_id.clone(),
                        config.apns_production,
                    );
                    client.send(&push_token.token, &notification).await
                } else {
                    tracing::debug!("APNS key path not configured, skipping iOS push");
                    continue;
                }
            }
            PushPlatform::Web => {
                tracing::debug!("Web push not yet implemented for token {}", push_token.token);
                continue;
            }
        };

        if let Err(e) = result {
            tracing::warn!(
                "Failed to send push notification to {} ({:?}): {}",
                push_token.token,
                push_token.platform,
                e
            );
        }
    }
}

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
    use super::*;
    use crate::DbHandle;
    use std::sync::Arc;

    fn test_db() -> DbHandle {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        let db: DbHandle = Arc::new(std::sync::Mutex::new(conn));
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
