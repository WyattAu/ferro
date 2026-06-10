use std::sync::Arc;

use ferro_event_bus::EventBus;
use ferro_event_bus::error::EventBusError;
use ferro_event_bus::event::FileEvent;
use ferro_event_bus::handler::EventHandler;

use crate::AppState;

pub fn create_event_bus() -> EventBus {
    EventBus::new()
}

pub async fn publish_file_created(
    bus: &EventBus,
    path: &str,
    user_id: &str,
    size: u64,
    content_type: &str,
) {
    let mut event = FileEvent::new("file.created", path, user_id);
    event.size = Some(size);
    event.content_type = Some(content_type.to_string());
    bus.publish(event).await;
}

pub async fn publish_file_deleted(bus: &EventBus, path: &str, user_id: &str) {
    let event = FileEvent::new("file.deleted", path, user_id);
    bus.publish(event).await;
}

pub async fn publish_file_modified(
    bus: &EventBus,
    path: &str,
    user_id: &str,
    size: Option<u64>,
    content_type: Option<&str>,
) {
    let mut event = FileEvent::new("file.modified", path, user_id);
    event.size = size;
    event.content_type = content_type.map(|s| s.to_string());
    bus.publish(event).await;
}

struct WebhookBusHandler {
    webhooks: Arc<tokio::sync::RwLock<Vec<crate::webhooks::WebhookConfig>>>,
    db: Option<crate::db::DbHandle>,
}

impl WebhookBusHandler {
    fn new(
        webhooks: Arc<tokio::sync::RwLock<Vec<crate::webhooks::WebhookConfig>>>,
        db: Option<crate::db::DbHandle>,
    ) -> Self {
        Self { webhooks, db }
    }
}

fn bus_event_to_webhook_event(event_type: &str) -> String {
    match event_type {
        "file.created" => "file.upload".to_string(),
        "file.deleted" => "file.delete".to_string(),
        "file.modified" => "file.modify".to_string(),
        other => other.to_string(),
    }
}

#[async_trait::async_trait]
impl EventHandler<FileEvent> for WebhookBusHandler {
    async fn handle(&self, event: &FileEvent) -> Result<(), EventBusError> {
        let webhook_event = crate::webhooks::WebhookEvent {
            event: bus_event_to_webhook_event(&event.event_type),
            timestamp: chrono::Utc::now().to_rfc3339(),
            path: event.path.clone(),
            size: event.size,
            user: Some(event.user_id.clone()),
            etag: None,
        };
        crate::webhooks::fire_webhooks(self.webhooks.clone(), webhook_event, self.db.clone()).await;
        Ok(())
    }

    fn name(&self) -> &str {
        "webhook_bridge"
    }
}

struct NotificationBusHandler {
    push_store: Option<Arc<tokio::sync::RwLock<crate::push_notifications::PushNotificationStore>>>,
    push_config: crate::push_notifications::PushNotificationConfig,
}

impl NotificationBusHandler {
    fn new(
        push_store: Option<
            Arc<tokio::sync::RwLock<crate::push_notifications::PushNotificationStore>>,
        >,
        push_config: crate::push_notifications::PushNotificationConfig,
    ) -> Self {
        Self {
            push_store,
            push_config,
        }
    }
}

#[async_trait::async_trait]
impl EventHandler<FileEvent> for NotificationBusHandler {
    async fn handle(&self, event: &FileEvent) -> Result<(), EventBusError> {
        if let Some(ref store) = self.push_store {
            crate::push_notifications::dispatch_push_notifications(
                store,
                &self.push_config,
                &event.user_id,
                &event.event_type,
                &event.path,
            )
            .await;
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "notification_bridge"
    }
}

pub fn setup_event_handlers(state: &AppState) {
    let bus = &state.event_bus;

    let wh = WebhookBusHandler::new(state.webhooks.clone(), state.db.clone());
    bus.subscribe("file.created", Box::new(wh));
    let wh = WebhookBusHandler::new(state.webhooks.clone(), state.db.clone());
    bus.subscribe("file.deleted", Box::new(wh));
    let wh = WebhookBusHandler::new(state.webhooks.clone(), state.db.clone());
    bus.subscribe("file.modified", Box::new(wh));

    let nh = NotificationBusHandler::new(
        state.push_notification_store.clone(),
        state.push_notification_config.clone(),
    );
    bus.subscribe("file.created", Box::new(nh));
    let nh = NotificationBusHandler::new(
        state.push_notification_store.clone(),
        state.push_notification_config.clone(),
    );
    bus.subscribe("file.deleted", Box::new(nh));
    let nh = NotificationBusHandler::new(
        state.push_notification_store.clone(),
        state.push_notification_config.clone(),
    );
    bus.subscribe("file.modified", Box::new(nh));

    tracing::info!(
        created = bus.handler_count("file.created"),
        deleted = bus.handler_count("file.deleted"),
        modified = bus.handler_count("file.modified"),
        "Event bus handlers registered"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_event_bus() {
        let bus = create_event_bus();
        assert_eq!(bus.handler_count("file.created"), 0);
    }

    #[tokio::test]
    async fn test_publish_file_created() {
        let bus = create_event_bus();
        publish_file_created(&bus, "/docs/test.txt", "alice", 1024, "text/plain").await;
    }

    #[tokio::test]
    async fn test_publish_file_deleted() {
        let bus = create_event_bus();
        publish_file_deleted(&bus, "/docs/old.txt", "bob").await;
    }

    #[tokio::test]
    async fn test_event_bus_with_store() {
        let bus = EventBus::builder().with_store().build();
        publish_file_created(&bus, "/a.txt", "u1", 100, "text/plain").await;
        let store = bus.event_store().unwrap();
        assert_eq!(store.len(), 1);
    }

    #[tokio::test]
    async fn test_setup_event_handlers() {
        let state = AppState::in_memory();
        setup_event_handlers(&state);
        assert_eq!(state.event_bus.handler_count("file.created"), 2);
        assert_eq!(state.event_bus.handler_count("file.deleted"), 2);
        assert_eq!(state.event_bus.handler_count("file.modified"), 2);
    }

    #[tokio::test]
    async fn test_publish_file_modified() {
        let bus = create_event_bus();
        publish_file_modified(
            &bus,
            "/docs/note.txt",
            "carol",
            Some(512),
            Some("text/plain"),
        )
        .await;
    }
}
