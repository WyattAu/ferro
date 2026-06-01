use ferro_event_bus::EventBus;
use ferro_event_bus::event::FileEvent;

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
}
