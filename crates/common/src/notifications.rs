use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Push subscription
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushSubscription {
    pub id: String,
    pub user_id: String,
    pub endpoint: String,
    pub p256dh_key: String,
    pub auth_key: String,
    pub created_at: DateTime<Utc>,
}

/// Notification type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NotificationType {
    EventCreated,
    EventUpdated,
    EventDeleted,
    EventReminder,
}

/// Push notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushNotification {
    pub subscription_id: String,
    pub notification_type: NotificationType,
    pub payload: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}

/// Notification manager
pub struct NotificationManager {
    subscriptions: Vec<PushSubscription>,
}

impl Default for NotificationManager {
    fn default() -> Self {
        Self::new()
    }
}

impl NotificationManager {
    pub fn new() -> Self {
        Self {
            subscriptions: Vec::new(),
        }
    }

    /// Register a push subscription
    pub fn register(&mut self, subscription: PushSubscription) {
        self.subscriptions.push(subscription);
    }

    /// Unregister a push subscription
    pub fn unregister(&mut self, subscription_id: &str) -> bool {
        let len_before = self.subscriptions.len();
        self.subscriptions.retain(|s| s.id != subscription_id);
        self.subscriptions.len() < len_before
    }

    /// Get subscriptions for a user
    pub fn get_user_subscriptions(&self, user_id: &str) -> Vec<&PushSubscription> {
        self.subscriptions.iter().filter(|s| s.user_id == user_id).collect()
    }

    /// Send notification to all subscribers
    pub fn notify(
        &self,
        user_id: &str,
        notification_type: NotificationType,
        payload: serde_json::Value,
    ) -> Vec<PushNotification> {
        self.get_user_subscriptions(user_id)
            .into_iter()
            .map(|subscription| PushNotification {
                subscription_id: subscription.id.clone(),
                notification_type: notification_type.clone(),
                payload: payload.clone(),
                timestamp: Utc::now(),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_subscription() {
        let mut manager = NotificationManager::new();

        let subscription = PushSubscription {
            id: "sub1".to_string(),
            user_id: "user1".to_string(),
            endpoint: "https://fcm.googleapis.com/fcm/send/123".to_string(),
            p256dh_key: "key1".to_string(),
            auth_key: "auth1".to_string(),
            created_at: Utc::now(),
        };

        manager.register(subscription);

        let subscriptions = manager.get_user_subscriptions("user1");
        assert_eq!(subscriptions.len(), 1);
    }

    #[test]
    fn test_unregister_subscription() {
        let mut manager = NotificationManager::new();

        let subscription = PushSubscription {
            id: "sub1".to_string(),
            user_id: "user1".to_string(),
            endpoint: "https://fcm.googleapis.com/fcm/send/123".to_string(),
            p256dh_key: "key1".to_string(),
            auth_key: "auth1".to_string(),
            created_at: Utc::now(),
        };

        manager.register(subscription);
        assert!(manager.unregister("sub1"));

        let subscriptions = manager.get_user_subscriptions("user1");
        assert_eq!(subscriptions.len(), 0);
    }

    #[test]
    fn test_notify() {
        let mut manager = NotificationManager::new();

        let subscription = PushSubscription {
            id: "sub1".to_string(),
            user_id: "user1".to_string(),
            endpoint: "https://fcm.googleapis.com/fcm/send/123".to_string(),
            p256dh_key: "key1".to_string(),
            auth_key: "auth1".to_string(),
            created_at: Utc::now(),
        };

        manager.register(subscription);

        let notifications = manager.notify(
            "user1",
            NotificationType::EventCreated,
            serde_json::json!({"event_id": "evt1"}),
        );

        assert_eq!(notifications.len(), 1);
        assert_eq!(notifications[0].notification_type, NotificationType::EventCreated);
    }
}
