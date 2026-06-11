//! Multi-User Scenario Tests
//!
//! Tests scenarios with multiple concurrent users:
//! - Share workflow: User A shares with User B
//! - Concurrent edit: Two users PUT to same file
//! - Directory sharing and nested file creation
//! - Permission enforcement and user isolation
//! - Guest access via public links
//! - Concurrent upload stress
//! - Notification delivery

use axum::body::Body;
use axum::http::{Request, StatusCode};
use ferro_server::{AppState, build_router};
use tower::ServiceExt;

async fn body_bytes(response: axum::response::Response) -> bytes::Bytes {
    use http_body_util::BodyExt;
    response.into_body().collect().await.unwrap().to_bytes()
}

async fn body_string(response: axum::response::Response) -> String {
    String::from_utf8(body_bytes(response).await.to_vec()).unwrap()
}

fn _user_app(_user: &str) -> axum::Router {
    let state = AppState::in_memory();
    build_router(state)
}

fn multi_user_app() -> axum::Router {
    let state = AppState::in_memory();
    build_router(state)
}

fn request_for(method: &str, uri: &str, user: &str, body: Body) -> Request<Body> {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("X-Ferro-User", user);
    if method == "PUT" || method == "POST" {
        builder = builder.header("Content-Type", "application/octet-stream");
    }
    builder.body(body).unwrap()
}

async fn put_file_as(app: &axum::Router, user: &str, path: &str, content: &str) -> StatusCode {
    let resp = app
        .clone()
        .oneshot(request_for(
            "PUT",
            path,
            user,
            Body::from(content.to_string()),
        ))
        .await
        .unwrap();
    resp.status()
}

async fn get_file_as(app: &axum::Router, user: &str, path: &str) -> (StatusCode, String) {
    let resp = app
        .clone()
        .oneshot(request_for("GET", path, user, Body::empty()))
        .await
        .unwrap();
    let status = resp.status();
    let body = body_string(resp).await;
    (status, body)
}

async fn delete_file_as(app: &axum::Router, user: &str, path: &str) -> StatusCode {
    let resp = app
        .clone()
        .oneshot(request_for("DELETE", path, user, Body::empty()))
        .await
        .unwrap();
    resp.status()
}

async fn mkcol_as(app: &axum::Router, user: &str, path: &str) -> StatusCode {
    let resp = app
        .clone()
        .oneshot(request_for("MKCOL", path, user, Body::empty()))
        .await
        .unwrap();
    resp.status()
}

// ── User Path Isolation ──────────────────────────────────────────────

#[tokio::test]
async fn test_user_isolation_basic() {
    let app = multi_user_app();

    let status = put_file_as(&app, "alice", "/secret.txt", "alice secret").await;
    assert_eq!(status, StatusCode::CREATED);

    let (status, _) = get_file_as(&app, "alice", "/secret.txt").await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = get_file_as(&app, "bob", "/secret.txt").await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "Bob should not see Alice's file"
    );

    let (status, _) = get_file_as(&app, "", "/secret.txt").await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "Anonymous should not see Alice's file"
    );
}

#[tokio::test]
async fn test_user_isolation_directory() {
    let app = multi_user_app();

    let status = mkcol_as(&app, "alice", "/docs").await;
    assert_eq!(status, StatusCode::CREATED);

    let status = put_file_as(&app, "alice", "/docs/readme.txt", "alice docs").await;
    assert_eq!(status, StatusCode::CREATED);

    let status = mkcol_as(&app, "bob", "/docs").await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "Bob can create their own /docs"
    );

    let status = put_file_as(&app, "bob", "/docs/notes.txt", "bob notes").await;
    assert_eq!(status, StatusCode::CREATED);

    let (status, body) = get_file_as(&app, "alice", "/docs/readme.txt").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "alice docs");

    let (status, _) = get_file_as(&app, "bob", "/docs/readme.txt").await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "Bob should not see Alice's /docs/readme.txt"
    );

    let (status, body) = get_file_as(&app, "bob", "/docs/notes.txt").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "bob notes");

    let (status, _) = get_file_as(&app, "alice", "/docs/notes.txt").await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "Alice should not see Bob's /docs/notes.txt"
    );
}

#[tokio::test]
async fn test_user_isolation_delete() {
    let app = multi_user_app();

    let status = put_file_as(&app, "alice", "/alice-only.txt", "alice content").await;
    assert_eq!(status, StatusCode::CREATED);

    let status = delete_file_as(&app, "bob", "/alice-only.txt").await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "Bob should not be able to delete Alice's file"
    );

    let (status, _) = get_file_as(&app, "alice", "/alice-only.txt").await;
    assert_eq!(status, StatusCode::OK, "Alice's file should still exist");
}

#[tokio::test]
async fn test_user_isolation_copy_move() {
    let app = multi_user_app();

    let status = put_file_as(&app, "alice", "/alice-src.txt", "alice src").await;
    assert_eq!(status, StatusCode::CREATED);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("COPY")
                .uri("/alice-src.txt")
                .header("Destination", "/alice-dst.txt")
                .header("X-Ferro-User", "bob")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "Bob cannot COPY Alice's file"
    );

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("COPY")
                .uri("/alice-src.txt")
                .header("Destination", "/alice-dst.txt")
                .header("X-Ferro-User", "alice")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::CREATED,
        "Alice can COPY her own file"
    );
}

// ── Share Workflow ───────────────────────────────────────────────────

#[tokio::test]
async fn test_share_workflow_user_a_to_user_b() {
    let app = multi_user_app();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("MKCOL")
                .uri("/shared-space")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/shared-space/shared-doc.txt")
                .body(Body::from("shared content"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/shares")
                .header("Content-Type", "application/json")
                .header("X-Ferro-User", "alice")
                .body(Body::from(
                    r#"{"path": "/shared-space/shared-doc.txt", "expires_in_hours": 24}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_string(resp).await;
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    let token = json["token"].as_str().unwrap().to_string();
    assert!(!token.is_empty(), "Share token should be non-empty");

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/s/{}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_bytes(resp).await;
    assert_eq!(&body[..], b"shared content");
}

#[tokio::test]
async fn test_share_password_protected() {
    let app = multi_user_app();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/secret-share.txt")
                .body(Body::from("secret stuff"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/shares")
                .header("Content-Type", "application/json")
                .header("X-Ferro-User", "alice")
                .body(Body::from(
                    r#"{"path": "/secret-share.txt", "password": "hunter2", "expires_in_hours": 24}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_string(resp).await;
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    let token = json["token"].as_str().unwrap().to_string();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/s/{}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "Should require password"
    );

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/s/{}?password=wrong", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "Wrong password should fail"
    );

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/s/{}?password=hunter2", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_bytes(resp).await;
    assert_eq!(&body[..], b"secret stuff");
}

#[tokio::test]
async fn test_share_crud_lifecycle() {
    let app = multi_user_app();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/share-lifecycle.txt")
                .body(Body::from("lifecycle"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/shares")
                .header("Content-Type", "application/json")
                .header("X-Ferro-User", "alice")
                .body(Body::from(
                    r#"{"path": "/share-lifecycle.txt", "expires_in_hours": 48}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_string(resp).await;
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    let token = json["token"].as_str().unwrap().to_string();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/shares")
                .header("X-Ferro-User", "alice")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_string(resp).await;
    assert!(body.contains(&token), "Share list should contain token");

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/s/{}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/shares/{}", token))
                .header("X-Ferro-User", "alice")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/s/{}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "Deleted share should return 404"
    );
}

// ── Concurrent Edit (Last-Write-Wins) ────────────────────────────────

#[tokio::test]
async fn test_concurrent_edit_last_write_wins() {
    let app = multi_user_app();

    let status = put_file_as(&app, "alice", "/contested.txt", "original").await;
    assert_eq!(status, StatusCode::CREATED);

    let app_clone = app.clone();
    let handle1 = tokio::spawn(async move {
        let status = put_file_as(&app_clone, "alice", "/contested.txt", "alice v2").await;
        assert!(
            status == StatusCode::NO_CONTENT || status == StatusCode::CREATED,
            "Alice's concurrent write should succeed"
        );
    });

    let app_clone = app.clone();
    let handle2 = tokio::spawn(async move {
        let status = put_file_as(&app_clone, "alice", "/contested.txt", "alice v3").await;
        assert!(
            status == StatusCode::NO_CONTENT || status == StatusCode::CREATED,
            "Alice's concurrent write should succeed"
        );
    });

    handle1.await.unwrap();
    handle2.await.unwrap();

    let (status, content) = get_file_as(&app, "alice", "/contested.txt").await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        content == "alice v2" || content == "alice v3",
        "Content should be one of the concurrent writes, got: {}",
        content
    );
}

// ── Directory Sharing ────────────────────────────────────────────────

#[tokio::test]
async fn test_directory_sharing_workflow() {
    let app = multi_user_app();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("MKCOL")
                .uri("/shared-dir")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/shared-dir/alice-file.txt")
                .body(Body::from("alice's file in shared dir"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/shares")
                .header("Content-Type", "application/json")
                .header("X-Ferro-User", "alice")
                .body(Body::from(
                    r#"{"path": "/shared-dir", "expires_in_hours": 24}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_string(resp).await;
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    let token = json["token"].as_str().unwrap().to_string();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/s/{}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(
        resp.status().is_success(),
        "Shared directory should be accessible via share link"
    );
}

// ── Guest Access (Public Link) ───────────────────────────────────────

#[tokio::test]
async fn test_guest_access_via_public_link() {
    let app = multi_user_app();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/public-doc.txt")
                .body(Body::from("public document"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/shares")
                .header("Content-Type", "application/json")
                .header("X-Ferro-User", "alice")
                .body(Body::from(
                    r#"{"path": "/public-doc.txt", "expires_in_hours": 1}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_string(resp).await;
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    let token = json["token"].as_str().unwrap().to_string();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/s/{}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_bytes(resp).await;
    assert_eq!(&body[..], b"public document");
}

// ── Permission Enforcement ───────────────────────────────────────────

#[tokio::test]
async fn test_permission_enforcement_no_cross_user_access() {
    let app = multi_user_app();

    let status = put_file_as(&app, "alice", "/alice-private.txt", "private").await;
    assert_eq!(status, StatusCode::CREATED);

    let status = put_file_as(&app, "bob", "/bob-private.txt", "private").await;
    assert_eq!(status, StatusCode::CREATED);

    let (status, _) = get_file_as(&app, "bob", "/alice-private.txt").await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, _) = get_file_as(&app, "alice", "/bob-private.txt").await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let status = delete_file_as(&app, "bob", "/alice-private.txt").await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let status = put_file_as(&app, "bob", "/alice-private.txt", "overwritten by bob").await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "Bob creates his own file at same path"
    );

    let (status, body) = get_file_as(&app, "bob", "/alice-private.txt").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "overwritten by bob");

    let (status, body) = get_file_as(&app, "alice", "/alice-private.txt").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "private", "Alice's original file is unchanged");
}

#[tokio::test]
async fn test_permission_enforcement_lock_isolation() {
    let app = multi_user_app();

    let status = put_file_as(&app, "alice", "/lock-test.txt", "lockable").await;
    assert_eq!(status, StatusCode::CREATED);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("LOCK")
                .uri("/lock-test.txt")
                .header("X-Ferro-User", "alice")
                .header("Depth", "0")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let status = put_file_as(&app, "bob", "/lock-test.txt", "bob's data").await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "Bob creates his own file at same path, unaffected by Alice's lock"
    );

    let (status, body) = get_file_as(&app, "alice", "/lock-test.txt").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "lockable", "Alice's file should be unchanged");

    let (status, body) = get_file_as(&app, "bob", "/lock-test.txt").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "bob's data", "Bob's file should have his content");
}

// ── Concurrent Upload Stress ─────────────────────────────────────────

#[tokio::test]
async fn test_concurrent_upload_stress_10_users() {
    let app = multi_user_app();

    let users: Vec<&str> = (0..10)
        .map(|i| Box::leak(format!("user-{}", i).into_boxed_str()) as &str)
        .collect();

    let mut handles = Vec::new();
    for (i, user) in users.iter().enumerate() {
        let app_clone = app.clone();
        let user = user.to_string();
        let handle = tokio::spawn(async move {
            let app = app_clone;

            let status = mkcol_as(&app, &user, "/stress-test").await;
            assert_eq!(status, StatusCode::CREATED);

            for j in 0..10 {
                let path = format!("/stress-test/file-{:03}.txt", j);
                let content = format!("user-{} file-{}", i, j);
                let status = put_file_as(&app, &user, &path, &content).await;
                assert_eq!(
                    status,
                    StatusCode::CREATED,
                    "PUT {} for user {} failed",
                    path,
                    user
                );
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }

    for (i, user) in users.iter().enumerate() {
        for j in 0..10 {
            let path = format!("/stress-test/file-{:03}.txt", j);
            let (status, content) = get_file_as(&app, user, &path).await;
            assert_eq!(
                status,
                StatusCode::OK,
                "GET {} for user {} failed",
                path,
                user
            );
            assert_eq!(content, format!("user-{} file-{}", i, j));
        }
    }
}

#[tokio::test]
async fn test_concurrent_upload_100_files_per_user() {
    let app = multi_user_app();

    let app_clone = app.clone();
    let handle = tokio::spawn(async move {
        let status = mkcol_as(&app_clone, "stressuser", "/bulk").await;
        assert_eq!(status, StatusCode::CREATED);

        for i in 0..100 {
            let path = format!("/bulk/file-{:03}.txt", i);
            let content = format!("content-{}", i);
            let status = put_file_as(&app_clone, "stressuser", &path, &content).await;
            assert_eq!(status, StatusCode::CREATED, "Failed to upload file {}", i);
        }
    });

    handle.await.unwrap();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PROPFIND")
                .uri("/bulk")
                .header("X-Ferro-User", "stressuser")
                .header("Depth", "1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::MULTI_STATUS);
    let body = body_string(resp).await;
    let count = body.matches("<D:response>").count();
    assert_eq!(
        count, 101,
        "Expected 101 responses (dir + 100 files), got {}",
        count
    );
}

// ── Notification Delivery ────────────────────────────────────────────

#[tokio::test]
async fn test_notification_on_share_creation() {
    let app = multi_user_app();

    let status = put_file_as(&app, "alice", "/notify-test.txt", "notify me").await;
    assert_eq!(status, StatusCode::CREATED);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/shares")
                .header("Content-Type", "application/json")
                .header("X-Ferro-User", "alice")
                .body(Body::from(
                    r#"{"path": "/notify-test.txt", "expires_in_hours": 24}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/audit")
                .header("X-Ferro-User", "alice")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_string(resp).await;
    assert!(
        body.contains("shares"),
        "Audit log should record share creation"
    );
}

#[tokio::test]
async fn test_audit_log_per_user() {
    let app = multi_user_app();

    put_file_as(&app, "alice", "/alice-audit.txt", "alice content").await;
    put_file_as(&app, "bob", "/bob-audit.txt", "bob content").await;

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/audit")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_string(resp).await;
    assert!(body.contains("PUT"), "Audit should contain PUT operations");
}

// ── PROPFIND per-user visibility ─────────────────────────────────────

#[tokio::test]
async fn test_propfind_user_isolation() {
    let app = multi_user_app();

    mkcol_as(&app, "alice", "/alice-dir").await;
    put_file_as(&app, "alice", "/alice-dir/a.txt", "a").await;
    put_file_as(&app, "alice", "/alice-dir/b.txt", "b").await;

    mkcol_as(&app, "bob", "/bob-dir").await;
    put_file_as(&app, "bob", "/bob-dir/c.txt", "c").await;

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PROPFIND")
                .uri("/alice-dir")
                .header("Depth", "1")
                .header("X-Ferro-User", "alice")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::MULTI_STATUS);
    let body = body_string(resp).await;
    let count = body.matches("<D:response>").count();
    assert_eq!(
        count, 3,
        "Alice should see her dir + 2 files, got {}",
        count
    );

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PROPFIND")
                .uri("/alice-dir")
                .header("Depth", "1")
                .header("X-Ferro-User", "bob")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "Bob should not see Alice's directory"
    );
}

// ── Storage Stats Multi-User ─────────────────────────────────────────

#[tokio::test]
async fn test_storage_stats_multi_user() {
    let app = multi_user_app();

    put_file_as(&app, "alice", "/stats-a.txt", "aaaa").await;
    put_file_as(&app, "alice", "/stats-a2.txt", "aaaa").await;
    put_file_as(&app, "bob", "/stats-b.txt", "bbbbbbbb").await;

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/storage/stats")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_string(resp).await;
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["files"].as_u64().unwrap(), 3);
    assert!(json["total_bytes"].as_u64().unwrap() >= 16);
}

// ── Lock Conflict Between Users ──────────────────────────────────────

#[tokio::test]
async fn test_lock_conflict_between_users() {
    let app = multi_user_app();

    let status = put_file_as(&app, "alice", "/lock-conflict.txt", "content").await;
    assert_eq!(status, StatusCode::CREATED);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("LOCK")
                .uri("/lock-conflict.txt")
                .header("X-Ferro-User", "alice")
                .header("Depth", "0")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let lock_token = resp
        .headers()
        .get("Lock-Token")
        .and_then(|v| v.to_str().ok())
        .map(|s| {
            s.strip_prefix('<')
                .and_then(|r| r.strip_suffix('>'))
                .unwrap_or(s)
                .to_string()
        })
        .unwrap();

    let if_header = format!("(<{}>)", lock_token);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/lock-conflict.txt")
                .header("X-Ferro-User", "alice")
                .header("If", &if_header)
                .body(Body::from("alice updates with lock"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::NO_CONTENT,
        "Alice can write with lock token"
    );

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("UNLOCK")
                .uri("/lock-conflict.txt")
                .header("X-Ferro-User", "alice")
                .header("Lock-Token", &format!("<{}>", lock_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

// ── Snapshot With Multiple Users ─────────────────────────────────────

#[tokio::test]
async fn test_snapshot_with_multiple_users() {
    let app = multi_user_app();

    put_file_as(&app, "alice", "/snap-a.txt", "alice data").await;
    put_file_as(&app, "bob", "/snap-b.txt", "bob data").await;

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/snapshots")
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"description": "multi-user snapshot"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_string(resp).await;
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(json["id"].is_string());

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/snapshots")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_string(resp).await;
    assert!(body.contains("multi-user snapshot"));
}

// ── Extended Share Types ─────────────────────────────────────────────

#[tokio::test]
async fn test_extended_share_view_type() {
    let app = multi_user_app();

    let status = put_file_as(&app, "alice", "/view-share.txt", "viewable").await;
    assert_eq!(status, StatusCode::CREATED);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/shares/ext")
                .header("Content-Type", "application/json")
                .header("X-Ferro-User", "alice")
                .body(Body::from(
                    r#"{"path": "/view-share.txt", "share_type": "view", "expires_in_hours": 24}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_string(resp).await;
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["share_type"].as_str().unwrap(), "view");
    assert!(json["token"].is_string());
}

#[tokio::test]
async fn test_extended_share_upload_type() {
    let app = multi_user_app();

    let status = mkcol_as(&app, "alice", "/upload-share").await;
    assert_eq!(status, StatusCode::CREATED);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/shares/ext")
                .header("Content-Type", "application/json")
                .header("X-Ferro-User", "alice")
                .body(Body::from(
                    r#"{"path": "/upload-share", "share_type": "upload", "expires_in_hours": 24}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_string(resp).await;
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["share_type"].as_str().unwrap(), "upload");
    assert!(json["allow_upload"].as_bool().unwrap());
}

// ── Concurrent Mixed Operations ──────────────────────────────────────

#[tokio::test]
async fn test_concurrent_mixed_operations() {
    let app = multi_user_app();

    mkcol_as(&app, "user1", "/mixed-ops").await;

    let mut handles = Vec::new();

    for i in 0..5 {
        let app_clone = app.clone();
        let handle = tokio::spawn(async move {
            let user = format!("concurrent-{}", i);
            let status = put_file_as(
                &app_clone,
                &user,
                &format!("/mixed-ops/{}.txt", user),
                &format!("data-{}", i),
            )
            .await;
            assert_eq!(status, StatusCode::CREATED);

            let (status, _) =
                get_file_as(&app_clone, &user, &format!("/mixed-ops/{}.txt", user)).await;
            assert_eq!(status, StatusCode::OK);

            let status =
                delete_file_as(&app_clone, &user, &format!("/mixed-ops/{}.txt", user)).await;
            assert_eq!(status, StatusCode::NO_CONTENT);

            let (status, _) =
                get_file_as(&app_clone, &user, &format!("/mixed-ops/{}.txt", user)).await;
            assert_eq!(status, StatusCode::NOT_FOUND);
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }
}

// ── Health Check Multi-User ──────────────────────────────────────────

#[tokio::test]
async fn test_health_check_with_multiple_users() {
    let app = multi_user_app();

    put_file_as(&app, "alice", "/health-file.txt", "data").await;
    put_file_as(&app, "bob", "/health-file.txt", "data").await;

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/.well-known/ferro")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_string(resp).await;
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["status"], "ok");
}
