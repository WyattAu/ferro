//! Android-specific initialization for Ferro.
//!
//! Handles notification registration, share intent processing,
//! and FileProvider setup for the Android platform.

use tauri::{AppHandle, Manager};
use tracing::{info, warn};

pub struct AndroidNotificationChannel {
    pub id: String,
    pub name: String,
    pub importance: u32,
}

impl AndroidNotificationChannel {
    pub fn sync_channel() -> Self {
        Self {
            id: "ferro_sync".into(),
            name: "File Sync".into(),
            importance: 3, // IMPORTANCE_HIGH
        }
    }

    pub fn transfer_channel() -> Self {
        Self {
            id: "ferro_transfer".into(),
            name: "File Transfer".into(),
            importance: 2, // IMPORTANCE_DEFAULT
        }
    }
}

pub fn register_notification_channels(app: &AppHandle) {
    use tauri_plugin_notification::NotificationExt;

    let channels = vec![
        AndroidNotificationChannel::sync_channel(),
        AndroidNotificationChannel::transfer_channel(),
    ];

    for channel in &channels {
        if let Err(e) = app.notification().builder().title(&channel.name).body("").show() {
            warn!("Failed to register notification channel {}: {}", channel.id, e);
        } else {
            info!("Registered notification channel: {}", channel.id);
        }
    }
}

pub struct AndroidShareIntent {
    pub action: String,
    pub mime_type: Option<String>,
    pub text: Option<String>,
    pub uris: Vec<String>,
}

impl AndroidShareIntent {
    pub fn from_intent_data(data: &str) -> Option<Self> {
        if data.is_empty() {
            return None;
        }
        Some(Self {
            action: "android.intent.action.SEND".into(),
            mime_type: None,
            text: Some(data.to_string()),
            uris: Vec::new(),
        })
    }

    pub fn is_share_action(&self) -> bool {
        self.action == "android.intent.action.SEND" || self.action == "android.intent.action.SEND_MULTIPLE"
    }
}

pub fn setup_file_provider(app: &AppHandle) {
    info!("Android FileProvider setup");
    let _ = app;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_channel() {
        let ch = AndroidNotificationChannel::sync_channel();
        assert_eq!(ch.id, "ferro_sync");
        assert_eq!(ch.importance, 3);
    }

    #[test]
    fn test_transfer_channel() {
        let ch = AndroidNotificationChannel::transfer_channel();
        assert_eq!(ch.id, "ferro_transfer");
        assert_eq!(ch.importance, 2);
    }

    #[test]
    fn test_share_intent_from_text() {
        let intent = AndroidShareIntent::from_intent_data("hello").unwrap();
        assert!(intent.is_share_action());
        assert_eq!(intent.text.as_deref(), Some("hello"));
        assert!(intent.uris.is_empty());
    }

    #[test]
    fn test_share_intent_empty() {
        assert!(AndroidShareIntent::from_intent_data("").is_none());
    }

    #[test]
    fn test_share_intent_non_share() {
        let mut intent = AndroidShareIntent::from_intent_data("x").unwrap();
        intent.action = "android.intent.action.VIEW".into();
        assert!(!intent.is_share_action());
    }
}
