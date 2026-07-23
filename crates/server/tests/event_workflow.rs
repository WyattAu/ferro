async fn body_string(response: axum::response::Response) -> String {
    use http_body_util::BodyExt;
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8(bytes.to_vec()).unwrap_or_default()
}

use axum::body::Body;
use axum::http::{Request, StatusCode};
use ferro_event_bus::{Event, EventBus, FileEvent};
use ferro_server::make_app;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tower::ServiceExt;

#[tokio::test]
async fn test_file_created_event_published() {
    let bus = Arc::new(EventBus::new());
    let received = Arc::new(AtomicUsize::new(0));

    struct CountingHandler {
        count: Arc<AtomicUsize>,
    }

    impl CountingHandler {
        fn new(count: Arc<AtomicUsize>) -> Self {
            Self { count }
        }
    }

    #[async_trait::async_trait]
    impl ferro_event_bus::EventHandler<FileEvent> for CountingHandler {
        async fn handle(&self, event: &FileEvent) -> Result<(), ferro_event_bus::EventBusError> {
            assert_eq!(event.event_type, "file.created");
            assert!(!event.path.is_empty());
            self.count.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
        fn name(&self) -> &str {
            "counter"
        }
    }

    bus.subscribe("file.created", Box::new(CountingHandler::new(received.clone())));

    let event = FileEvent::new("file.created", "/docs/report.pdf", "alice");
    bus.publish(event).await;

    assert_eq!(received.load(Ordering::Relaxed), 1);
    assert_eq!(bus.handler_count("file.created"), 1);
}

#[tokio::test]
async fn test_multiple_handlers_for_same_event() {
    let bus = Arc::new(EventBus::new());
    let count_a = Arc::new(AtomicUsize::new(0));
    let count_b = Arc::new(AtomicUsize::new(0));

    struct Handler {
        count: Arc<AtomicUsize>,
    }

    impl Handler {
        fn new(count: Arc<AtomicUsize>) -> Self {
            Self { count }
        }
    }

    #[async_trait::async_trait]
    impl ferro_event_bus::EventHandler<FileEvent> for Handler {
        async fn handle(&self, _event: &FileEvent) -> Result<(), ferro_event_bus::EventBusError> {
            self.count.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
        fn name(&self) -> &str {
            "multi"
        }
    }

    bus.subscribe("file.upload", Box::new(Handler::new(count_a.clone())));
    bus.subscribe("file.upload", Box::new(Handler::new(count_b.clone())));

    let event = FileEvent::new("file.upload", "/photo.jpg", "bob");
    bus.publish(event).await;

    assert_eq!(count_a.load(Ordering::Relaxed), 1);
    assert_eq!(count_b.load(Ordering::Relaxed), 1);
    assert_eq!(bus.handler_count("file.upload"), 2);
}

#[tokio::test]
async fn test_event_serialization_roundtrip() {
    let event = FileEvent::new("file.deleted", "/old-file.txt", "admin");
    let json = event.to_json().unwrap();
    let deserialized: FileEvent = FileEvent::from_json(&json).unwrap();

    assert_eq!(deserialized.event_type, "file.deleted");
    assert_eq!(deserialized.path, "/old-file.txt");
    assert_eq!(deserialized.user_id, "admin");
}

#[tokio::test]
async fn test_event_store_persistence() {
    let bus = EventBus::builder().with_store().build();
    let received = Arc::new(AtomicUsize::new(0));

    struct StoreHandler {
        count: Arc<AtomicUsize>,
    }

    impl StoreHandler {
        fn new(count: Arc<AtomicUsize>) -> Self {
            Self { count }
        }
    }

    #[async_trait::async_trait]
    impl ferro_event_bus::EventHandler<FileEvent> for StoreHandler {
        async fn handle(&self, _event: &FileEvent) -> Result<(), ferro_event_bus::EventBusError> {
            self.count.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
        fn name(&self) -> &str {
            "store_test"
        }
    }

    bus.subscribe("file.created", Box::new(StoreHandler::new(received.clone())));

    for i in 0..5 {
        let event = FileEvent::new("file.created", format!("/file-{}.txt", i), "user");
        bus.publish(event).await;
    }

    assert_eq!(received.load(Ordering::Relaxed), 5);

    let store = bus.event_store().unwrap();
    assert_eq!(store.len(), 5);
}

#[tokio::test]
async fn test_failed_handler_goes_to_dlq() {
    let bus = EventBus::new();

    struct FailHandler;

    #[async_trait::async_trait]
    impl ferro_event_bus::EventHandler<FileEvent> for FailHandler {
        async fn handle(&self, _event: &FileEvent) -> Result<(), ferro_event_bus::EventBusError> {
            Err(ferro_event_bus::EventBusError::handler_failed(
                "fail",
                "file.error",
                "boom",
            ))
        }
        fn name(&self) -> &str {
            "fail"
        }
    }

    bus.subscribe("file.error", Box::new(FailHandler));

    let event = FileEvent::new("file.error", "/broken.txt", "system");
    bus.publish(event).await;

    let dlq = bus.dead_letter_queue().unwrap();
    assert_eq!(dlq.len(), 1);
    assert_eq!(dlq.all()[0].event_type, "file.error");
}

#[tokio::test]
async fn test_concurrent_event_publish() {
    let bus = Arc::new(EventBus::new());
    let received = Arc::new(AtomicUsize::new(0));

    struct ConcurrentHandler {
        count: Arc<AtomicUsize>,
    }

    impl ConcurrentHandler {
        fn new(count: Arc<AtomicUsize>) -> Self {
            Self { count }
        }
    }

    #[async_trait::async_trait]
    impl ferro_event_bus::EventHandler<FileEvent> for ConcurrentHandler {
        async fn handle(&self, _event: &FileEvent) -> Result<(), ferro_event_bus::EventBusError> {
            self.count.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
        fn name(&self) -> &str {
            "concurrent"
        }
    }

    bus.subscribe("file.created", Box::new(ConcurrentHandler::new(received.clone())));

    let mut handles = Vec::new();
    for i in 0..50 {
        let b = bus.clone();
        handles.push(tokio::spawn(async move {
            let event = FileEvent::new("file.created", format!("/concurrent-{}.txt", i), "user");
            b.publish(event).await;
        }));
    }

    for h in handles {
        h.await.unwrap();
    }

    assert_eq!(received.load(Ordering::Relaxed), 50);
}

#[tokio::test]
#[ignore = "Requires full middleware stack with database connectivity; fails when compiled with pg feature without a running PostgreSQL instance"]
async fn test_audit_chain_after_file_operations() {
    let app = make_app();

    let operations = vec![
        ("PUT", "/audit-chain/ops/report.pdf", "admin"),
        ("GET", "/audit-chain/ops/report.pdf", "admin"),
        ("DELETE", "/audit-chain/ops/report.pdf", "admin"),
        ("PUT", "/audit-chain/ops/vacation.jpg", "alice"),
        ("PUT", "/audit-chain/ops/settings.json", "admin"),
    ];

    for (method, path, _user) in &operations {
        let builder = if *method == "PUT" {
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/files{}", path))
                .body(Body::from("audit content"))
        } else if *method == "GET" {
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/files{}", path))
                .body(Body::empty())
        } else {
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/files{}", path))
                .body(Body::empty())
        };
        app.clone().oneshot(builder.unwrap()).await.unwrap();
    }

    let audit_resp = app
        .clone()
        .oneshot(Request::builder().uri("/api/audit").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(audit_resp.status(), StatusCode::OK);

    let audit_body = body_string(audit_resp).await;
    assert!(audit_body.contains("/audit-chain/ops/report.pdf"));
    assert!(audit_body.contains("/audit-chain/ops/vacation.jpg"));
    assert!(audit_body.contains("/audit-chain/ops/settings.json"));
}

#[tokio::test]
async fn test_audit_log_serialization() {
    let audit_log = ferro_server::audit::AuditLog::new();

    audit_log
        .log(ferro_server::audit::build_audit_entry(
            "PUT",
            "/test.txt",
            "admin",
            201,
            Some("10.0.0.1".to_string()),
            Some("test-agent".to_string()),
        ))
        .await;

    let entries = audit_log.entries().await;
    assert_eq!(entries.len(), 1);

    let json = serde_json::to_string(&entries).unwrap();
    let parsed: Vec<serde_json::Value> = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0]["method"], "PUT");
    assert_eq!(parsed[0]["path"], "/test.txt");
    assert_eq!(parsed[0]["user"], "admin");
    assert_eq!(parsed[0]["status"], 201);
    assert!(parsed[0].get("timestamp").is_some());
}
