pub mod bus;
pub mod dead_letter;
pub mod error;
pub mod event;
pub mod handler;
pub mod queue;
pub mod replay;

pub use bus::{EventBus, EventBusBuilder, EventInterceptor};
pub use dead_letter::{DeadLetterEntry, DeadLetterQueue};
pub use error::EventBusError;
pub use event::{Event, FileEvent, SystemEvent};
pub use handler::{EventHandler, HandlerResult};
pub use queue::{EventQueue, QueuedEvent};
pub use replay::{EventFilter, EventStore, StoredEvent};

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::handler::EventHandler;

    struct LogHandler {
        name: String,
        received: std::sync::Mutex<Vec<String>>,
    }

    impl LogHandler {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
                received: std::sync::Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait::async_trait]
    impl EventHandler<FileEvent> for LogHandler {
        async fn handle(&self, event: &FileEvent) -> Result<(), EventBusError> {
            self.received.lock().unwrap().push(event.path.clone());
            Ok(())
        }
        fn name(&self) -> &str {
            &self.name
        }
    }

    struct SysLogHandler {
        name: String,
        received: std::sync::Mutex<Vec<String>>,
    }

    impl SysLogHandler {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
                received: std::sync::Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait::async_trait]
    impl EventHandler<SystemEvent> for SysLogHandler {
        async fn handle(&self, event: &SystemEvent) -> Result<(), EventBusError> {
            self.received.lock().unwrap().push(event.source.clone());
            Ok(())
        }
        fn name(&self) -> &str {
            &self.name
        }
    }

    struct FailSysHandler;

    struct MoveInterceptor<T: EventInterceptor>(std::sync::Arc<T>);

    #[async_trait::async_trait]
    impl<T: EventInterceptor> EventInterceptor for MoveInterceptor<T> {
        async fn before_publish(&self, event_type: &str, event_json: &str) -> Result<(), EventBusError> {
            self.0.before_publish(event_type, event_json).await
        }
        async fn after_publish(&self, event_type: &str, event_json: &str, results: &[HandlerResult]) {
            self.0.after_publish(event_type, event_json, results).await
        }
    }

    #[async_trait::async_trait]
    impl EventHandler<SystemEvent> for FailSysHandler {
        async fn handle(&self, _event: &SystemEvent) -> Result<(), EventBusError> {
            Err(EventBusError::handler_failed("fail_sys", "user.login", "boom"))
        }
        fn name(&self) -> &str {
            "fail_sys"
        }
    }

    #[tokio::test]
    async fn file_event_roundtrip() {
        let bus = EventBus::new();
        let handler = LogHandler::new("file_logger");
        bus.subscribe("file.created", Box::new(handler));
        let event = FileEvent::new("file.created", "/docs/readme.md", "alice");
        bus.publish(event).await;
        assert_eq!(bus.handler_count("file.created"), 1);
    }

    #[tokio::test]
    async fn system_event_roundtrip() {
        let bus = EventBus::new();
        let handler = SysLogHandler::new("sys_logger");
        bus.subscribe("user.login", Box::new(handler));
        let event = SystemEvent::new("user.login", "auth-service");
        bus.publish(event).await;
        assert_eq!(bus.handler_count("user.login"), 1);
    }

    #[tokio::test]
    async fn system_event_handler_error_to_dlq() {
        let bus = EventBus::new();
        bus.subscribe("user.login", Box::new(FailSysHandler));
        let event = SystemEvent::new("user.login", "auth-service");
        bus.publish(event).await;
        let dlq = bus.dead_letter_queue().unwrap();
        assert_eq!(dlq.len(), 1);
        assert_eq!(dlq.all()[0].event_type, "user.login");
    }

    #[tokio::test]
    async fn concurrent_publish() {
        let bus = std::sync::Arc::new(EventBus::new());
        let handler = LogHandler::new("concurrent_handler");
        bus.subscribe("file.created", Box::new(handler));
        let mut handles = Vec::new();
        for _ in 0..10 {
            let b = bus.clone();
            handles.push(tokio::spawn(async move {
                let event = FileEvent::new("file.created", "/test.txt", "user1");
                b.publish(event).await;
            }));
        }
        for h in handles {
            h.await.unwrap();
        }
        assert_eq!(bus.handler_count("file.created"), 1);
    }

    #[tokio::test]
    async fn file_event_serialization() {
        let event = FileEvent::new("file.uploaded", "/photo.jpg", "bob");
        let json = event.to_json().unwrap();
        let deserialized: FileEvent = FileEvent::from_json(&json).unwrap();
        assert_eq!(deserialized.path, "/photo.jpg");
        assert_eq!(deserialized.user_id, "bob");
    }

    #[tokio::test]
    async fn system_event_serialization() {
        let event = SystemEvent::new("config.changed", "admin-panel");
        let json = event.to_json().unwrap();
        let deserialized: SystemEvent = SystemEvent::from_json(&json).unwrap();
        assert_eq!(deserialized.source, "admin-panel");
    }

    #[tokio::test]
    async fn event_store_replay_integration() {
        let bus = EventBus::builder().with_store().build();
        let handler = LogHandler::new("replay_handler");
        bus.subscribe("file.created", Box::new(handler));
        let e1 = FileEvent::new("file.created", "/a.txt", "u1");
        let e2 = FileEvent::new("file.created", "/b.txt", "u2");
        bus.publish(e1).await;
        bus.publish(e2).await;
        let store = bus.event_store().unwrap();
        assert_eq!(store.len(), 2);
        bus.replay(&EventFilter::new()).await;
    }

    #[tokio::test]
    async fn unsubscribe_removes_correct_handler() {
        let bus = EventBus::new();
        let h1 = LogHandler::new("keep");
        let h2 = LogHandler::new("remove");
        bus.subscribe("file.created", Box::new(h1));
        bus.subscribe("file.created", Box::new(h2));
        assert_eq!(bus.handler_count("file.created"), 2);
        bus.unsubscribe("file.created", "remove");
        assert_eq!(bus.handler_count("file.created"), 1);
    }

    #[tokio::test]
    async fn publish_no_handlers_empty_results() {
        let bus = EventBus::new();
        let event = FileEvent::new("file.created", "/ghost.txt", "nobody");
        let results = bus.publish_and_wait(event).await;
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn dead_letter_max_size() {
        let bus = EventBus::builder().with_dead_letter(3).build();
        struct AlwaysFail;
        #[async_trait::async_trait]
        impl EventHandler<FileEvent> for AlwaysFail {
            async fn handle(&self, _e: &FileEvent) -> Result<(), EventBusError> {
                Err(EventBusError::handler_failed("always", "file.created", "fail"))
            }
            fn name(&self) -> &str {
                "always"
            }
        }
        bus.subscribe("file.created", Box::new(AlwaysFail));
        for _ in 0..5 {
            bus.publish(FileEvent::new("file.created", "/f.txt", "u")).await;
        }
        let dlq = bus.dead_letter_queue().unwrap();
        assert_eq!(dlq.len(), 3);
    }

    #[tokio::test]
    async fn interceptor_called_on_publish() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        struct CountingInterceptor {
            before_count: AtomicUsize,
            after_count: AtomicUsize,
        }
        #[async_trait::async_trait]
        impl EventInterceptor for CountingInterceptor {
            async fn before_publish(&self, _event_type: &str, _event_json: &str) -> Result<(), EventBusError> {
                self.before_count.fetch_add(1, Ordering::Relaxed);
                Ok(())
            }
            async fn after_publish(&self, _event_type: &str, _event_json: &str, _results: &[HandlerResult]) {
                self.after_count.fetch_add(1, Ordering::Relaxed);
            }
        }
        let interceptor = Arc::new(CountingInterceptor {
            before_count: AtomicUsize::new(0),
            after_count: AtomicUsize::new(0),
        });
        let ic = interceptor.clone();
        let bus = EventBus::builder()
            .with_interceptor(Box::new(MoveInterceptor(ic)))
            .build();
        bus.subscribe("file.created", Box::new(LogHandler::new("h")));
        bus.publish(FileEvent::new("file.created", "/a.txt", "u1")).await;
        bus.publish(FileEvent::new("file.created", "/b.txt", "u2")).await;
        assert_eq!(interceptor.before_count.load(Ordering::Relaxed), 2);
        assert_eq!(interceptor.after_count.load(Ordering::Relaxed), 2);
    }
}
