use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushTokenRegistration {
    pub device_id: String,
    pub platform: MobilePlatform,
    pub push_token: String,
    pub app_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MobilePlatform {
    Ios,
    Android,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPayload {
    pub event_type: NotificationEvent,
    pub path: Option<String>,
    pub actor: Option<String>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum NotificationEvent {
    FileShared,
    ShareReceived,
    QuotaWarning { percent_used: u8 },
    SyncConflict { path: String },
    CommentAdded { file_path: String },
}
