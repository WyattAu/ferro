use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use dashmap::DashMap;
use tokio::sync::Mutex;

use crate::dead_letter::{DeadLetterEntry, DeadLetterQueue};
use crate::error::EventBusError;
use crate::event::Event;
use crate::handler::{ErasedHandler, HandlerEraser, HandlerResult};

#[async_trait]
pub trait EventInterceptor: Send + Sync {
    async fn before_publish(&self, event_type: &str, event_json: &str)
    -> Result<(), EventBusError>;
    async fn after_publish(&self, event_type: &str, event_json: &str, results: &[HandlerResult]);
}

pub struct EventBusBuilder {
    with_dead_letter: bool,
    dead_letter_max_size: usize,
    with_store: bool,
    interceptor: Option<Box<dyn EventInterceptor>>,
}

impl EventBusBuilder {
    pub fn new() -> Self {
        Self {
            with_dead_letter: true,
            dead_letter_max_size: 1000,
            with_store: false,
            interceptor: None,
        }
    }

    pub fn with_dead_letter(mut self, max_size: usize) -> Self {
        self.with_dead_letter = true;
        self.dead_letter_max_size = max_size;
        self
    }

    pub fn without_dead_letter(mut self) -> Self {
        self.with_dead_letter = false;
        self
    }

    pub fn with_store(mut self) -> Self {
        self.with_store = true;
        self
    }

    pub fn with_interceptor(mut self, interceptor: Box<dyn EventInterceptor>) -> Self {
        self.interceptor = Some(interceptor);
        self
    }

    pub fn build(self) -> EventBus {
        let dead_letter = if self.with_dead_letter {
            Some(DeadLetterQueue::new(self.dead_letter_max_size))
        } else {
            None
        };

        let store = if self.with_store {
            Some(Arc::new(crate::replay::EventStore::new()))
        } else {
            None
        };

        EventBus {
            handlers: DashMap::new(),
            dead_letter,
            store,
            interceptor: Arc::new(Mutex::new(self.interceptor)),
        }
    }
}

impl Default for EventBusBuilder {
    fn default() -> Self {
        Self::new()
    }
}

pub struct EventBus {
    handlers: DashMap<String, Vec<Arc<dyn HandlerEraser>>>,
    dead_letter: Option<DeadLetterQueue>,
    store: Option<Arc<crate::replay::EventStore>>,
    interceptor: Arc<Mutex<Option<Box<dyn EventInterceptor>>>>,
}

impl EventBus {
    pub fn new() -> Self {
        EventBusBuilder::new().build()
    }

    pub fn builder() -> EventBusBuilder {
        EventBusBuilder::new()
    }

    pub fn subscribe<E: Event>(
        &self,
        event_type: &str,
        handler: Box<dyn crate::handler::EventHandler<E>>,
    ) {
        let erased = Arc::new(ErasedHandler::new(handler));
        let mut handlers = self.handlers.entry(event_type.to_string()).or_default();
        handlers.push(erased);
    }

    pub fn unsubscribe(&self, event_type: &str, handler_name: &str) {
        if let Some(mut handlers) = self.handlers.get_mut(event_type) {
            handlers.retain(|h| h.name() != handler_name);
        }
    }

    pub async fn publish(&self, event: impl Event) {
        let event_type = event.event_type().to_string();
        let event_json = match event.to_json() {
            Ok(json) => json,
            Err(err) => {
                eprintln!(
                    "[event-bus] failed to serialize event '{}': {}",
                    event_type, err
                );
                return;
            }
        };
        let timestamp = event.timestamp();

        let interceptor_ref = self.interceptor.clone();
        let has_interceptor = interceptor_ref.lock().await.is_some();

        if has_interceptor {
            let guard = interceptor_ref.lock().await;
            if let Some(ref ic) = *guard
                && let Err(err) = ic.before_publish(&event_type, &event_json).await
            {
                eprintln!(
                    "[event-bus] interceptor rejected event '{}': {}",
                    event_type, err
                );
                return;
            }
            drop(guard);
        }

        let mut results = Vec::new();

        if let Some(handlers) = self.handlers.get(&event_type) {
            for handler in handlers.iter() {
                let name = handler.name().to_string();
                let et = event_type.clone();
                match handler.handle_erased(&event_json, &et).await {
                    Ok(()) => {
                        results.push(HandlerResult::ok(&name));
                    }
                    Err(err) => {
                        eprintln!(
                            "[event-bus] handler '{}' failed for event '{}': {}",
                            name, event_type, err
                        );
                        if let Some(ref dlq) = self.dead_letter {
                            dlq.push(DeadLetterEntry {
                                event_json: event_json.clone(),
                                event_type: et.clone(),
                                handler_name: name.clone(),
                                error: err.to_string(),
                                timestamp: Utc::now(),
                                retry_count: 0,
                            });
                        }
                        results.push(HandlerResult::err(&name, &err.to_string()));
                    }
                }
            }
        }

        if has_interceptor {
            let guard = interceptor_ref.lock().await;
            if let Some(ref ic) = *guard {
                ic.after_publish(&event_type, &event_json, &results).await;
            }
        }

        if let Some(ref store) = self.store {
            store.append(crate::replay::StoredEvent {
                event_json,
                event_type,
                timestamp,
                id: uuid::Uuid::new_v4().to_string(),
            });
        }
    }

    pub async fn publish_and_wait(&self, event: impl Event) -> Vec<HandlerResult> {
        let event_type = event.event_type().to_string();
        let event_json = match event.to_json() {
            Ok(json) => json,
            Err(err) => {
                return vec![HandlerResult::err("serialize", &err.to_string())];
            }
        };
        let timestamp = event.timestamp();

        let interceptor_ref = self.interceptor.clone();
        let has_interceptor = interceptor_ref.lock().await.is_some();

        if has_interceptor {
            let guard = interceptor_ref.lock().await;
            if let Some(ref ic) = *guard
                && let Err(err) = ic.before_publish(&event_type, &event_json).await
            {
                return vec![HandlerResult::err("interceptor", &err.to_string())];
            }
            drop(guard);
        }

        let mut results = Vec::new();

        if let Some(handlers) = self.handlers.get(&event_type) {
            for handler in handlers.iter() {
                let name = handler.name().to_string();
                let et = event_type.clone();
                match handler.handle_erased(&event_json, &et).await {
                    Ok(()) => {
                        results.push(HandlerResult::ok(&name));
                    }
                    Err(err) => {
                        if let Some(ref dlq) = self.dead_letter {
                            dlq.push(DeadLetterEntry {
                                event_json: event_json.clone(),
                                event_type: et.clone(),
                                handler_name: name.clone(),
                                error: err.to_string(),
                                timestamp: Utc::now(),
                                retry_count: 0,
                            });
                        }
                        results.push(HandlerResult::err(&name, &err.to_string()));
                    }
                }
            }
        }

        if has_interceptor {
            let guard = interceptor_ref.lock().await;
            if let Some(ref ic) = *guard {
                ic.after_publish(&event_type, &event_json, &results).await;
            }
        }

        if let Some(ref store) = self.store {
            store.append(crate::replay::StoredEvent {
                event_json,
                event_type,
                timestamp,
                id: uuid::Uuid::new_v4().to_string(),
            });
        }

        results
    }

    pub fn handler_count(&self, event_type: &str) -> usize {
        self.handlers.get(event_type).map(|h| h.len()).unwrap_or(0)
    }

    pub fn event_types(&self) -> Vec<String> {
        self.handlers.iter().map(|kv| kv.key().clone()).collect()
    }

    pub fn dead_letter_queue(&self) -> Option<&DeadLetterQueue> {
        self.dead_letter.as_ref()
    }

    pub fn event_store(&self) -> Option<&Arc<crate::replay::EventStore>> {
        self.store.as_ref()
    }

    pub async fn retry_dead_letters(&self) {
        let Some(dlq) = &self.dead_letter else {
            return;
        };
        let entries = dlq.drain(dlq.len());
        for entry in entries {
            let event_type = entry.event_type.clone();
            if let Some(handlers) = self.handlers.get(&event_type) {
                for handler in handlers.iter() {
                    if handler.name() == entry.handler_name {
                        match handler.handle_erased(&entry.event_json, &event_type).await {
                            Ok(()) => {}
                            Err(err) => {
                                dlq.push(DeadLetterEntry {
                                    retry_count: entry.retry_count + 1,
                                    ..entry.clone()
                                });
                                eprintln!(
                                    "[event-bus] retry failed for handler '{}' on event '{}': {}",
                                    handler.name(),
                                    event_type,
                                    err
                                );
                            }
                        }
                        break;
                    }
                }
            }
        }
    }

    pub async fn replay(&self, filter: &crate::replay::EventFilter) {
        let Some(store) = &self.store else {
            return;
        };
        let events = store.query(filter);
        for stored in events {
            let event_type = stored.event_type.clone();
            if let Some(handlers) = self.handlers.get(&event_type) {
                for handler in handlers.iter() {
                    let _ = handler.handle_erased(&stored.event_json, &event_type).await.map_err(|e| {
                        tracing::warn!(event_type = %event_type, error = %e, "event replay handler failed");
                        e
                    });
                }
            }
        }
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::FileEvent;
    use crate::handler::EventHandler;

    struct CounterHandler {
        name: String,
        count: std::sync::atomic::AtomicUsize,
    }

    impl CounterHandler {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
                count: std::sync::atomic::AtomicUsize::new(0),
            }
        }

        #[allow(dead_code)]
        fn count(&self) -> usize {
            self.count.load(std::sync::atomic::Ordering::Relaxed)
        }
    }

    #[async_trait]
    impl EventHandler<FileEvent> for CounterHandler {
        async fn handle(&self, _event: &FileEvent) -> Result<(), EventBusError> {
            self.count
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Ok(())
        }

        fn name(&self) -> &str {
            &self.name
        }
    }

    struct FailHandler {
        name: String,
    }

    #[async_trait]
    impl EventHandler<FileEvent> for FailHandler {
        async fn handle(&self, _event: &FileEvent) -> Result<(), EventBusError> {
            Err(EventBusError::handler_failed(
                &self.name,
                "file.created",
                "intentional failure",
            ))
        }

        fn name(&self) -> &str {
            &self.name
        }
    }

    #[tokio::test]
    async fn publish_subscribe_file_event() {
        let bus = EventBus::new();
        let handler = CounterHandler::new("counter");
        bus.subscribe("file.created", Box::new(handler));
        let event = FileEvent::new("file.created", "/test.txt", "user1");
        bus.publish(event).await;
        assert_eq!(bus.handler_count("file.created"), 1);
    }

    #[tokio::test]
    async fn multiple_handlers_same_event() {
        let bus = EventBus::new();
        let h1 = CounterHandler::new("h1");
        let h2 = CounterHandler::new("h2");
        bus.subscribe("file.created", Box::new(h1));
        bus.subscribe("file.created", Box::new(h2));
        assert_eq!(bus.handler_count("file.created"), 2);
    }

    #[tokio::test]
    async fn handler_error_goes_to_dead_letter() {
        let bus = EventBus::new();
        bus.subscribe(
            "file.created",
            Box::new(FailHandler {
                name: "fail".into(),
            }),
        );
        let event = FileEvent::new("file.created", "/test.txt", "user1");
        bus.publish(event).await;
        let dlq = bus.dead_letter_queue().unwrap();
        assert_eq!(dlq.len(), 1);
        let entry = dlq.drain(1);
        assert_eq!(entry[0].handler_name, "fail");
    }

    #[tokio::test]
    async fn no_handlers_no_panic() {
        let bus = EventBus::new();
        let event = FileEvent::new("file.created", "/test.txt", "user1");
        bus.publish(event).await;
    }

    #[tokio::test]
    async fn unsubscribe_handler() {
        let bus = EventBus::new();
        bus.subscribe("file.created", Box::new(CounterHandler::new("h1")));
        bus.subscribe("file.created", Box::new(CounterHandler::new("h2")));
        assert_eq!(bus.handler_count("file.created"), 2);
        bus.unsubscribe("file.created", "h1");
        assert_eq!(bus.handler_count("file.created"), 1);
    }

    #[tokio::test]
    async fn publish_and_wait_collects_results() {
        let bus = EventBus::new();
        bus.subscribe("file.created", Box::new(CounterHandler::new("ok_handler")));
        bus.subscribe(
            "file.created",
            Box::new(FailHandler {
                name: "fail_handler".into(),
            }),
        );
        let event = FileEvent::new("file.created", "/test.txt", "user1");
        let results = bus.publish_and_wait(event).await;
        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|r| r.success));
        assert!(results.iter().any(|r| !r.success));
    }

    #[tokio::test]
    async fn event_types_list() {
        let bus = EventBus::new();
        bus.subscribe("file.created", Box::new(CounterHandler::new("fh")));
        let types = bus.event_types();
        assert!(!types.is_empty());
        assert!(types.contains(&"file.created".to_string()));
    }

    #[tokio::test]
    async fn dead_letter_drain_and_retry() {
        let bus = EventBus::new();
        bus.subscribe(
            "file.created",
            Box::new(FailHandler {
                name: "fail".into(),
            }),
        );
        let event = FileEvent::new("file.created", "/test.txt", "user1");
        bus.publish(event).await;
        let dlq = bus.dead_letter_queue().unwrap();
        assert_eq!(dlq.len(), 1);
        let drained = dlq.drain(10);
        assert_eq!(drained.len(), 1);
        assert!(dlq.is_empty());
    }

    #[tokio::test]
    async fn builder_with_store() {
        let bus = EventBus::builder().with_store().build();
        bus.subscribe("file.created", Box::new(CounterHandler::new("h")));
        let event = FileEvent::new("file.created", "/test.txt", "user1");
        bus.publish(event).await;
        let store = bus.event_store().unwrap();
        assert_eq!(store.len(), 1);
    }

    #[tokio::test]
    async fn builder_without_dead_letter() {
        let bus = EventBus::builder().without_dead_letter().build();
        assert!(bus.dead_letter_queue().is_none());
        bus.subscribe(
            "file.created",
            Box::new(FailHandler {
                name: "fail".into(),
            }),
        );
        let event = FileEvent::new("file.created", "/test.txt", "user1");
        bus.publish(event).await;
    }

    #[tokio::test]
    async fn replay_from_store() {
        let bus = EventBus::builder().with_store().build();
        bus.subscribe("file.created", Box::new(CounterHandler::new("h")));
        let event = FileEvent::new("file.created", "/test.txt", "user1");
        bus.publish(event).await;
        assert_eq!(bus.event_store().unwrap().len(), 1);
        bus.replay(&crate::replay::EventFilter::new()).await;
    }

    #[tokio::test]
    async fn interceptor_mock() {
        use std::sync::atomic::AtomicBool;
        struct MockInterceptor {
            before_called: AtomicBool,
            after_called: AtomicBool,
        }

        #[async_trait]
        impl EventInterceptor for MockInterceptor {
            async fn before_publish(
                &self,
                _event_type: &str,
                _event_json: &str,
            ) -> Result<(), EventBusError> {
                self.before_called
                    .store(true, std::sync::atomic::Ordering::Relaxed);
                Ok(())
            }

            async fn after_publish(
                &self,
                _event_type: &str,
                _event_json: &str,
                _results: &[HandlerResult],
            ) {
                self.after_called
                    .store(true, std::sync::atomic::Ordering::Relaxed);
            }
        }

        let interceptor = MockInterceptor {
            before_called: AtomicBool::new(false),
            after_called: AtomicBool::new(false),
        };
        let bus = EventBus::builder()
            .with_interceptor(Box::new(interceptor))
            .build();
        bus.subscribe("file.created", Box::new(CounterHandler::new("h")));
        let event = FileEvent::new("file.created", "/test.txt", "user1");
        bus.publish(event).await;
    }
}
