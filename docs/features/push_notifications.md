# Push Notifications

## Overview

Push notifications (RFC 8594) notify clients of calendar changes in real-time.

## Features

### Web Push
- Browser push notifications
- Mobile push notifications
- Desktop push notifications

### Notification Types
- Event created
- Event updated
- Event deleted
- Event reminder

## API Endpoints

### Register Push Subscription
```http
POST /dav/push/subscribe
Content-Type: application/json

{
  "endpoint": "https://fcm.googleapis.com/fcm/send/...",
  "keys": {
    "p256dh": "...",
    "auth": "..."
  }
}
```

### Unregister Push Subscription
```http
DELETE /dav/push/subscribe/{subscription_id}
```

## Implementation

### Database Schema
```sql
CREATE TABLE push_subscriptions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    endpoint TEXT NOT NULL,
    p256dh_key TEXT NOT NULL,
    auth_key TEXT NOT NULL,
    created_at DATETIME NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id)
);
```

### Rust Types
```rust
pub struct PushSubscription {
    pub id: String,
    pub user_id: String,
    pub endpoint: String,
    pub p256dh_key: String,
    pub auth_key: String,
    pub created_at: DateTime<Utc>,
}

pub enum NotificationType {
    EventCreated,
    EventUpdated,
    EventDeleted,
    EventReminder,
}

pub struct PushNotification {
    pub subscription_id: String,
    pub notification_type: NotificationType,
    pub payload: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}
```

### Web Push Implementation
```rust
use web_push::{WebPushClient, WebPushMessage, WebPushError};

pub async fn send_push_notification(
    subscription: &PushSubscription,
    notification: &PushNotification,
) -> Result<(), WebPushError> {
    let client = WebPushClient::new()?;
    
    let message = WebPushMessage::new()
        .set_endpoint(&subscription.endpoint)
        .set_p256dh(&subscription.p256dh_key)
        .set_auth(&subscription.auth_key)
        .set_payload(serde_json::to_vec(notification)?)?;
    
    client.send(message).await?;
    
    Ok(())
}
```
