use axum::body::Body;
use axum::http::{Request, StatusCode};
use ferro_server::make_app;
use tower::ServiceExt;

async fn body_string(response: axum::response::Response) -> String {
    use http_body_util::BodyExt;
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8(bytes.to_vec()).unwrap_or_default()
}

async fn body_json(response: axum::response::Response) -> serde_json::Value {
    let bytes = body_string(response).await;
    serde_json::from_str(&bytes).unwrap_or(serde_json::Value::Null)
}

#[tokio::test]
async fn test_put_get_delete_with_audit() {
    let app = make_app();

    let put_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/files/audit-crud-test.txt")
                .header("X-Forwarded-For", "192.168.1.100")
                .header("User-Agent", "integration-test")
                .body(Body::from("audit lifecycle content"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(put_resp.status(), StatusCode::CREATED);
    let put_json = body_json(put_resp).await;
    assert!(put_json.get("etag").is_some(), "PUT should return etag");
    let etag = put_json["etag"].as_str().unwrap().to_string();

    let cond_get_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/files/audit-crud-test.txt")
                .header("If-None-Match", &etag)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(cond_get_resp.status(), StatusCode::NOT_MODIFIED);

    let get_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/files/audit-crud-test.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(get_resp.status(), StatusCode::OK);
    let body = body_string(get_resp).await;
    assert_eq!(body, "audit lifecycle content");

    let del_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/v1/files/audit-crud-test.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(del_resp.status(), StatusCode::NO_CONTENT);

    let gone_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/files/audit-crud-test.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(gone_resp.status(), StatusCode::NOT_FOUND);

    let audit_resp = app
        .clone()
        .oneshot(Request::builder().uri("/api/audit").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(audit_resp.status(), StatusCode::OK);
    let audit_body = body_string(audit_resp).await;
    assert!(audit_body.contains("PUT"), "Audit should contain PUT");
    assert!(audit_body.contains("GET"), "Audit should contain GET");
    assert!(audit_body.contains("DELETE"), "Audit should contain DELETE");
}

#[tokio::test]
async fn test_full_file_lifecycle_with_events() {
    let app = make_app();

    let mkdir_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/files/mkdir")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"path": "/lifecycle-events"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(mkdir_resp.status(), StatusCode::CREATED);

    let put_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/files/lifecycle-events/readme.md")
                .body(Body::from("# Lifecycle Events\n\nIntegration test content."))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(put_resp.status(), StatusCode::CREATED);
    let put_json = body_json(put_resp).await;
    assert!(put_json.get("size").is_some());
    let put_size = put_json["size"].as_u64().unwrap();
    assert!(put_size > 0, "File size should be positive");

    let get_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/files/lifecycle-events/readme.md")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(get_resp.status(), StatusCode::OK);
    let get_body = body_string(get_resp).await;
    assert!(get_body.contains("# Lifecycle Events"));

    let copy_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/files/copy")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"source": "/lifecycle-events/readme.md", "destination": "/lifecycle-events/readme-copy.md"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(copy_resp.status() == StatusCode::CREATED || copy_resp.status() == StatusCode::OK,);

    let list_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/files?path=/lifecycle-events")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(list_resp.status(), StatusCode::OK);
    let list_json = body_json(list_resp).await;
    let entries = list_json["entries"].as_array().unwrap();
    assert!(!entries.is_empty());

    let del_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/v1/files/lifecycle-events/readme.md")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(del_resp.status(), StatusCode::NO_CONTENT);

    let gone_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/files/lifecycle-events/readme.md")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(gone_resp.status(), StatusCode::NOT_FOUND);

    let audit_resp = app
        .clone()
        .oneshot(Request::builder().uri("/api/audit").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(audit_resp.status(), StatusCode::OK);
    let audit_body = body_string(audit_resp).await;
    assert!(audit_body.contains("PUT"));
    assert!(audit_body.contains("DELETE"));
}

#[tokio::test]
async fn test_storage_operations_consistency() {
    let app = make_app();

    for i in 0..10 {
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/files/mkdir")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(r#"{{"path": "/consistency-dir-{}"}}"#, i)))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED, "MKCOL dir-{} should succeed", i);
    }

    for i in 0..10 {
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/api/v1/files/consistency-dir-{}/file.txt", i))
                    .body(Body::from(format!("content {}", i)))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED, "PUT dir-{} file should succeed", i);
    }

    for i in 0..10 {
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/files/consistency-dir-{}/file.txt", i))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_string(resp).await;
        assert_eq!(body, format!("content {}", i));
    }

    let list_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/files?path=/consistency-dir-0")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(list_resp.status(), StatusCode::OK);
    let list_json = body_json(list_resp).await;
    let entries = list_json["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 1);

    for i in 0..10 {
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/v1/files/consistency-dir-{}/file.txt", i))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::NO_CONTENT,
            "DELETE file-{} should succeed",
            i
        );
    }

    for i in 0..10 {
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/v1/files/consistency-dir-{}", i))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(
            resp.status() == StatusCode::NO_CONTENT || resp.status() == StatusCode::NOT_FOUND,
            "DELETE dir-{} should succeed",
            i
        );
    }
}

#[tokio::test]
async fn test_websocket_event_broadcast() {
    let manager = ferro_server::ws::WsManager::new();

    let mut rx = manager.subscribe();
    assert_eq!(manager.connection_count(), 1);

    manager.broadcast(&ferro_server::ws::WsEvent::FileCreated {
        path: "/test.txt".to_string(),
        size: 1024,
        owner: "admin".to_string(),
    });

    let msg = rx.recv().await.unwrap();
    assert!(msg.contains("file_created"));
    assert!(msg.contains("/test.txt"));

    manager.broadcast(&ferro_server::ws::WsEvent::FileDeleted {
        path: "/old.txt".to_string(),
        owner: "admin".to_string(),
    });

    let msg = rx.recv().await.unwrap();
    assert!(msg.contains("file_deleted"));
    assert!(msg.contains("/old.txt"));

    manager.broadcast(&ferro_server::ws::WsEvent::FileMoved {
        from: "/a.txt".to_string(),
        to: "/b.txt".to_string(),
        owner: "admin".to_string(),
    });

    let msg = rx.recv().await.unwrap();
    assert!(msg.contains("file_moved"));
}

#[tokio::test]
async fn test_snapshot_and_audit_integration() {
    let app = make_app();

    app.clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/snapshot-audit.txt")
                .body(Body::from("snapshot me"))
                .unwrap(),
        )
        .await
        .unwrap();

    let snap_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/snapshots")
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"description": "pre-delete snapshot"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(snap_resp.status(), StatusCode::CREATED);
    let snap_json = body_json(snap_resp).await;
    let snap_id = snap_json["id"].as_str().unwrap().to_string();

    let snap_list = app
        .clone()
        .oneshot(Request::builder().uri("/api/snapshots").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(snap_list.status(), StatusCode::OK);

    app.clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/snapshots/invalid-id")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let del_snap = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/snapshots/{}", snap_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(del_snap.status(), StatusCode::NO_CONTENT);

    let audit_resp = app
        .clone()
        .oneshot(Request::builder().uri("/api/audit").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(audit_resp.status(), StatusCode::OK);
}
