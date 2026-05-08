use axum::body::Body;
use axum::http::{Request, StatusCode};
use base64::Engine;
use bytes::Bytes;
use ferro_server::make_app;
use std::collections::HashSet;
use tower::ServiceExt;

async fn body_bytes(response: axum::response::Response) -> Bytes {
    use http_body_util::BodyExt;
    response.into_body().collect().await.unwrap().to_bytes()
}

async fn body_string(response: axum::response::Response) -> String {
    String::from_utf8(body_bytes(response).await.to_vec()).unwrap()
}

fn count_propfind_responses(xml: &str) -> usize {
    xml.matches("<D:response>").count()
}

fn extract_etags(xml: &str) -> HashSet<String> {
    let mut etags = HashSet::new();
    for part in xml.split("<D:getetag>") {
        if let Some(end) = part.find("</D:getetag>") {
            etags.insert(part[..end].to_string());
        }
    }
    etags
}

fn make_wopi_token(path: &str) -> String {
    let secret = "test-wopi-secret-for-integration";
    let exp = chrono::Utc::now().timestamp() + 3600;
    let payload = serde_json::json!({ "path": path, "user": "test", "exp": exp });
    let payload_str = serde_json::to_string(&payload).unwrap();
    use hmac::{KeyInit, Mac};
    use sha2::Sha256;
    let mut mac = hmac::Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(payload_str.as_bytes());
    let sig = hex::encode(mac.finalize().into_bytes());
    base64::engine::general_purpose::STANDARD.encode(format!("{}:{}", payload_str, sig))
}

#[tokio::test]
async fn test_full_webdav_lifecycle() {
    let app = make_app();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("MKCOL")
                .uri("/dir")
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
                .uri("/dir/file1.txt")
                .body(Body::from("hello"))
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
                .uri("/dir/file2.txt")
                .body(Body::from("world"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PROPFIND")
                .uri("/dir")
                .header("Depth", "1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::MULTI_STATUS);
    let body = body_string(resp).await;
    assert_eq!(count_propfind_responses(&body), 3);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/dir/file1.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_bytes(resp).await;
    assert_eq!(&body[..], b"hello");

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("HEAD")
                .uri("/dir/file1.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert!(resp.headers().contains_key("Content-Length"));
    assert!(resp.headers().contains_key("ETag"));
    assert!(resp.headers().contains_key("Content-Type"));

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("COPY")
                .uri("/dir/file1.txt")
                .header("Destination", "/dir/file1_copy.txt")
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
                .method("GET")
                .uri("/dir/file1_copy.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_bytes(resp).await;
    assert_eq!(&body[..], b"hello");

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("MOVE")
                .uri("/dir/file1_copy.txt")
                .header("Destination", "/dir/file1_moved.txt")
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
                .method("GET")
                .uri("/dir/file1_copy.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// ── Full-stack integration tests (middleware stack + request lifecycle) ──

#[tokio::test]
async fn test_cors_preflight_on_api_config() {
    let app = make_app();
    let response = app
        .oneshot(
            Request::builder()
                .method("OPTIONS")
                .uri("/api/config")
                .header("Origin", "https://example.com")
                .header("Access-Control-Request-Method", "GET")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert!(
        response
            .headers()
            .get("access-control-allow-origin")
            .is_some()
    );
}

#[tokio::test]
async fn test_rate_limit_returns_429_after_exhaustion() {
    let app = make_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/config")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_workers_list_endpoint() {
    let app = make_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/workers")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = body_string(response).await;
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(json.get("workers").is_some());
}

#[tokio::test]
async fn test_config_includes_version_and_features() {
    let app = make_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/config")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = body_string(response).await;
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(json.get("version").is_some());
    assert!(json.get("wasm_enabled").is_some());
    assert!(json.get("wopi_configured").is_some());
}

#[tokio::test]
async fn test_propfind_depth_variants() {
    let app = make_app();

    for path in ["/a", "/a/b", "/a/b/c"] {
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("MKCOL")
                    .uri(path)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::CREATED,
            "MKCOL {} should succeed",
            path
        );
    }

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/a/b/c/file.txt")
                .body(Body::from("nested content"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PROPFIND")
                .uri("/a")
                .header("Depth", "0")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::MULTI_STATUS);
    let body = body_string(resp).await;
    assert_eq!(count_propfind_responses(&body), 1);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PROPFIND")
                .uri("/a")
                .header("Depth", "1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::MULTI_STATUS);
    let body = body_string(resp).await;
    assert_eq!(count_propfind_responses(&body), 2);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PROPFIND")
                .uri("/a")
                .header("Depth", "infinity")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::MULTI_STATUS);
    let body = body_string(resp).await;
    let count = count_propfind_responses(&body);
    assert!(count >= 4, "Expected at least 4 responses, got {}", count);
}

#[tokio::test]
async fn test_lock_protects_resource() {
    let app = make_app();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/locked.txt")
                .body(Body::from("initial"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let lock_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("LOCK")
                .uri("/locked.txt")
                .header("Depth", "0")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(lock_resp.status(), StatusCode::OK);

    let lock_token_header = lock_resp
        .headers()
        .get("Lock-Token")
        .expect("Lock-Token header missing")
        .to_str()
        .unwrap();
    let _lock_token = lock_token_header.to_string();

    // The Lock-Token header is <urn:uuid:UUID>; extract the full URN for If/UNLOCK.
    let urn_token = lock_token_header
        .strip_prefix("<")
        .and_then(|r| r.strip_suffix(">"))
        .unwrap();
    let if_header = format!("(<{}>)", urn_token);
    let unlock_token = format!("<{}>", urn_token);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/locked.txt")
                .body(Body::from("should fail"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::LOCKED);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/locked.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::LOCKED);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/locked.txt")
                .header("If", &if_header)
                .body(Body::from("updated"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("UNLOCK")
                .uri("/locked.txt")
                .header("Lock-Token", &unlock_token)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn test_mkcol_already_exists() {
    let app = make_app();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("MKCOL")
                .uri("/exists")
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
                .method("MKCOL")
                .uri("/exists")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn test_copy_and_move_nonexistent() {
    let app = make_app();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("COPY")
                .uri("/nonexistent")
                .header("Destination", "/dest")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("MOVE")
                .uri("/nonexistent")
                .header("Destination", "/dest")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_propfind_nonexistent() {
    let app = make_app();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PROPFIND")
                .uri("/nonexistent")
                .header("Depth", "0")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_put_overwrite() {
    let app = make_app();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/file.txt")
                .body(Body::from("v1"))
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
                .uri("/file.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_bytes(resp).await;
    assert_eq!(&body[..], b"v1");

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/file.txt")
                .body(Body::from("v2"))
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
                .uri("/file.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_bytes(resp).await;
    assert_eq!(&body[..], b"v2");
}

#[tokio::test]
async fn test_options_dav_header() {
    let app = make_app();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("OPTIONS")
                .uri("/")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let dav = resp.headers().get("DAV").unwrap().to_str().unwrap();
    assert_eq!(dav, "1, 2, 3");

    let allow = resp.headers().get("Allow").unwrap().to_str().unwrap();
    let methods: Vec<&str> = allow.split(", ").collect();
    assert!(methods.contains(&"OPTIONS"));
    assert!(methods.contains(&"GET"));
    assert!(methods.contains(&"HEAD"));
    assert!(methods.contains(&"PUT"));
    assert!(methods.contains(&"DELETE"));
    assert!(methods.contains(&"MKCOL"));
    assert!(methods.contains(&"COPY"));
    assert!(methods.contains(&"MOVE"));
    assert!(methods.contains(&"PROPFIND"));
    assert!(methods.contains(&"LOCK"));
    assert!(methods.contains(&"UNLOCK"));
}

#[tokio::test]
async fn test_nested_collection_operations() {
    let app = make_app();

    for path in ["/a", "/a/b", "/a/b/c"] {
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("MKCOL")
                    .uri(path)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/a/b/c/file.txt")
                .body(Body::from("deep file"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PROPFIND")
                .uri("/a/b/c")
                .header("Depth", "0")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::MULTI_STATUS);
    let body = body_string(resp).await;
    assert!(
        body.contains("<D:collection/>"),
        "Expected collection type in PROPFIND"
    );

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/a/b/c/file.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    for path in ["/a/b/c", "/a/b", "/a"] {
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(path)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::NO_CONTENT,
            "DELETE {} should succeed",
            path
        );
    }

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PROPFIND")
                .uri("/a")
                .header("Depth", "0")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_large_number_of_files() {
    let app = make_app();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("MKCOL")
                .uri("/bulk")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    for i in 1..=100 {
        let name = format!("/bulk/file_{:03}.txt", i);
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(&name)
                    .body(Body::from(format!("content {}", i)))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::CREATED,
            "PUT {} should succeed",
            name
        );
    }

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PROPFIND")
                .uri("/bulk")
                .header("Depth", "1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::MULTI_STATUS);
    let body = body_string(resp).await;
    let count = count_propfind_responses(&body);
    assert_eq!(count, 101);

    let etags = extract_etags(&body);
    assert_eq!(
        etags.len(),
        101,
        "All 101 resources should have unique ETags"
    );
}

#[tokio::test]
async fn test_audit_log_captures_requests() {
    let app = make_app();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/audit_test.txt")
                .header("X-Forwarded-For", "192.168.1.1")
                .header("User-Agent", "test-agent")
                .body(Body::from("audit me"))
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
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_string(resp).await;
    assert!(body.contains("PUT"), "Audit log should contain PUT method");
    assert!(
        body.contains("audit_test.txt"),
        "Audit log should contain path"
    );
    assert!(
        body.contains("192.168.1.1"),
        "Audit log should contain client IP"
    );
    assert!(
        body.contains("test-agent"),
        "Audit log should contain user agent"
    );
}

#[tokio::test]
async fn test_share_link_crud() {
    let app = make_app();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/shareme.txt")
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
                .body(Body::from(
                    r#"{"path": "/shareme.txt", "expires_in_hours": 24}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_string(resp).await;
    assert!(
        body.contains("token"),
        "Share response should contain token"
    );
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    let token = json["token"].as_str().unwrap().to_string();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/shares")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_string(resp).await;
    assert!(body.contains(&token), "Share list should contain our token");

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

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/shares/{}", token))
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
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_share_link_password_required() {
    let app = make_app();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/protected.txt")
                .body(Body::from("secret content"))
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
                .body(Body::from(r#"{"path": "/protected.txt", "password": "secret123", "expires_in_hours": 24}"#))
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
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

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
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/s/{}?password=secret123", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_bytes(resp).await;
    assert_eq!(&body[..], b"secret content");
}

#[tokio::test]
async fn test_snapshot_create_and_list() {
    let app = make_app();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/snap_test.txt")
                .body(Body::from("snapshot me"))
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
                .uri("/api/snapshots")
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"description": "test snapshot"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_string(resp).await;
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    let snapshot_id = json["id"].as_str().unwrap().to_string();

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
    assert!(
        body.contains("test snapshot"),
        "Snapshot list should contain our snapshot"
    );
    assert!(
        body.contains(&snapshot_id),
        "Snapshot list should contain our ID"
    );

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/snapshots/{}", snapshot_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn test_cors_preflight() {
    let app = make_app();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("OPTIONS")
                .uri("/")
                .header("Origin", "http://example.com")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    assert_eq!(
        resp.headers().get("Access-Control-Allow-Origin").unwrap(),
        "*"
    );
    assert!(resp.headers().get("Access-Control-Allow-Methods").is_some());
    assert!(resp.headers().get("Access-Control-Allow-Headers").is_some());
}

#[tokio::test]
async fn test_same_origin_options_unaffected() {
    let app = make_app();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("OPTIONS")
                .uri("/")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        resp.headers().get("DAV").unwrap().to_str().unwrap(),
        "1, 2, 3"
    );
    assert!(resp.headers().get("Access-Control-Allow-Origin").is_none());
}

#[tokio::test]
async fn test_user_path_isolation() {
    let app = make_app();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/alice_file.txt")
                .header("X-Ferro-User", "alice")
                .body(Body::from("alice content"))
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
                .uri("/bob_file.txt")
                .header("X-Ferro-User", "bob")
                .body(Body::from("bob content"))
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
                .uri("/alice_file.txt")
                .header("X-Ferro-User", "alice")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_bytes(resp).await;
    assert_eq!(&body[..], b"alice content");

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/alice_file.txt")
                .header("X-Ferro-User", "bob")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/alice_file.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_content_type_sniffing() {
    let app = make_app();

    let png_bytes = b"\x89PNG\r\n\x1a\nfake-png-data";
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/image.png")
                .body(Body::from(png_bytes.as_ref()))
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
                .uri("/image.png")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let content_type = resp
        .headers()
        .get("Content-Type")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(
        content_type.contains("image/png") || content_type.contains("application/octet-stream"),
        "Expected image/png or application/octet-stream, got {}",
        content_type
    );

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/data.bin")
                .header("Content-Type", "application/custom")
                .body(Body::from("some data"))
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
                .uri("/data.bin")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let content_type = resp
        .headers()
        .get("Content-Type")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(
        content_type.contains("application/octet-stream")
            || content_type.contains("application/custom"),
        "Expected application/octet-stream or application/custom, got {}",
        content_type
    );
}

#[tokio::test]
async fn test_proppatch_set_property() {
    let app = make_app();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/propfile.txt")
                .body(Body::from("content"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let prop_patch_xml = r#"<?xml version="1.0" encoding="utf-8"?>
<D:propertyupdate xmlns:D="DAV:">
    <D:set>
        <D:prop>
            <D:displayname>My File</D:displayname>
        </D:prop>
    </D:set>
</D:propertyupdate>"#;

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PROPPATCH")
                .uri("/propfile.txt")
                .header("Content-Type", "application/xml")
                .body(Body::from(prop_patch_xml))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::MULTI_STATUS);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PROPFIND")
                .uri("/propfile.txt")
                .header("Depth", "0")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::MULTI_STATUS);
    let body = body_string(resp).await;
    assert!(body.contains("propfile.txt"));
}

#[tokio::test]
async fn test_storage_stats_endpoint() {
    let app = make_app();

    for i in 1..=3 {
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/stat{}.txt", i))
                    .body(Body::from(format!("data {}", i)))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("MKCOL")
                .uri("/statdir")
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
    assert_eq!(json["collections"].as_u64().unwrap(), 1);
    assert!(json["total_bytes"].as_u64().unwrap() > 0);
    assert!(!json["metadata_store"].as_bool().unwrap());
}

#[tokio::test]
async fn test_health_check() {
    let app = make_app();

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
    assert_eq!(json["subsystems"]["storage"], "ok");
    assert_eq!(json["subsystems"]["auth"], "disabled");
}

#[tokio::test]
async fn test_conditional_get_not_modified() {
    let app = make_app();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/cached.txt")
                .body(Body::from("cached content"))
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
                .uri("/cached.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let etag = resp
        .headers()
        .get("ETag")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/cached.txt")
                .header("If-None-Match", &etag)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_MODIFIED);
}

#[tokio::test]
async fn test_delete_nonexistent() {
    let app = make_app();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/no_such_file.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_auth_info_anonymous() {
    let app = make_app();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/auth/info")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_string(resp).await;
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["sub"].as_str().unwrap(), "anonymous");
    assert_eq!(json["iss"].as_str().unwrap(), "ferro");
}

// ── Cedar authorization integration tests ─────────────────────────────

use ferro_server::AppState;
use ferro_server::auth::cedar::CedarAuthorizer;

fn make_app_with_cedar(policy: &str) -> axum::Router {
    let state = AppState::in_memory();
    let authorizer = CedarAuthorizer::new().unwrap();
    // Use a dedicated thread to avoid nested runtime issues
    let auth_clone = authorizer.clone();
    let policy_owned = policy.to_string();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            auth_clone.load_policies(&[policy_owned]).await.unwrap();
        });
    })
    .join()
    .unwrap();
    let state = state.with_cedar(authorizer);
    ferro_server::build_router(state)
}

#[tokio::test]
async fn test_cedar_permissive_allows_all() {
    let app = make_app_with_cedar(
        r#"
        @id("open")
        permit (principal, action in [Action::"read", Action::"write", Action::"delete", Action::"list", Action::"admin"], resource);
    "#,
    );

    // PUT a file
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/cedar-test.txt")
                .header("content-length", "4")
                .body(Body::from("test"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // GET the file
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/cedar-test.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_cedar_restrictive_denies_write() {
    // Only allow read and list, no write
    let app = make_app_with_cedar(
        r#"
        @id("readonly")
        permit (principal, action in [Action::"read", Action::"list"], resource);
    "#,
    );

    // PUT should be denied (403)
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/no-write.txt")
                .header("content-length", "4")
                .body(Body::from("test"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    // PROPFIND (list) should still work
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PROPFIND")
                .uri("/")
                .header("Depth", "1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::MULTI_STATUS);
}

#[tokio::test]
async fn test_cedar_exempt_paths_bypass_authorization() {
    // Policy denies everything
    let app = make_app_with_cedar(
        r#"
        @id("deny_all")
        forbid (principal, action, resource);
    "#,
    );

    // /api/policies should still be accessible (exempt)
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/policies")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // /api/config should still be accessible (exempt)
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/config")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // / should be denied
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_cedar_no_cedar_configured_passes_through() {
    // make_app() has no Cedar configured — all requests should pass
    let app = make_app();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/no-cedar.txt")
                .header("content-length", "4")
                .body(Body::from("test"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
}

// ── WOPI integration tests ────────────────────────────────────────────

#[tokio::test]
async fn test_wopi_discovery_returns_xml() {
    let app = make_app();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/hosting/discovery")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let ct = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok());
    assert!(ct.unwrap().contains("application/xml"));

    let body = body_string(resp).await;
    assert!(body.contains("<wopi-discovery>"));
    assert!(body.contains("action name=\"edit\""));
    assert!(body.contains("action name=\"view\""));
}

#[tokio::test]
async fn test_wopi_check_file_info_requires_token() {
    let app = make_app();

    // No access_token → 401
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/wopi/files/test.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // Empty access_token → 401
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/wopi/files/test.txt?access_token=")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_wopi_check_file_info_with_token() {
    let app = make_app();

    // First upload a file
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/wopi-test.docx")
                .header("content-length", "5")
                .body(Body::from("hello"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // CheckFileInfo with valid token
    let token = make_wopi_token("/wopi-test.docx");
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/wopi/files/wopi-test.docx?access_token={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = body_string(resp).await;
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["base_file_name"].as_str().unwrap(), "wopi-test.docx");
    assert_eq!(json["size"].as_u64().unwrap(), 5);
    assert!(json["user_can_write"].as_bool().unwrap());
    assert!(json["supports_update"].as_bool().unwrap());
    assert!(json["supports_locks"].as_bool().unwrap());
}

#[tokio::test]
async fn test_wopi_get_file_contents() {
    let app = make_app();

    // Upload a file
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/wopi-contents.txt")
                .header("content-length", "9")
                .body(Body::from("contents!"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Get file contents via WOPI
    let token = make_wopi_token("/wopi-contents.txt");
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/wopi/files/wopi-contents.txt/contents?access_token={token}"
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_string(resp).await;
    assert_eq!(body, "contents!");
}

#[tokio::test]
async fn test_wopi_file_not_found() {
    let app = make_app();

    // CheckFileInfo for nonexistent file
    let token = make_wopi_token("/nonexistent.txt");
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/wopi/files/nonexistent.txt?access_token={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // GetContents for nonexistent file
    let token = make_wopi_token("/nonexistent.txt");
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/wopi/files/nonexistent.txt/contents?access_token={token}"
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// ── REST file listing ─────────────────────────────────────────────────

#[tokio::test]
async fn test_rest_list_files_root() {
    let app = make_app();
    // Seed some files
    for name in ["hello.txt", "subdir/"] {
        if name.ends_with('/') {
            let _ = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("MKCOL")
                        .uri(format!("/{}", name.trim_end_matches('/')))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await;
        } else {
            let _ = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("PUT")
                        .uri(format!("/{}", name))
                        .body(Body::from("hello world"))
                        .unwrap(),
                )
                .await;
        }
    }

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/files")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_string(resp).await;
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    let entries = json["entries"].as_array().unwrap();
    assert!(
        !entries.is_empty(),
        "Expected at least 1 entry, got {}",
        entries.len()
    );

    let hello = entries
        .iter()
        .find(|e| e["name"] == "hello.txt")
        .expect("hello.txt should be listed");
    assert_eq!(hello["size"], 11);
    assert_eq!(hello["is_collection"], false);
    assert!(!hello["content_hash"].as_str().unwrap().is_empty());
    assert!(!hello["etag"].as_str().unwrap().is_empty());
}

#[tokio::test]
async fn test_rest_list_files_nested_path() {
    let app = make_app();
    // Create nested structure
    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .method("MKCOL")
                .uri("/docs")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/docs/readme.md")
                .body(Body::from("# docs"))
                .unwrap(),
        )
        .await;

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/files?path=/docs")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_string(resp).await;
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    let entries = json["entries"].as_array().unwrap();
    let readme = entries
        .iter()
        .find(|e| e["name"] == "readme.md")
        .expect("readme.md should be listed");
    assert_eq!(readme["size"], 6);
    assert_eq!(readme["path"], "/docs/readme.md");
}

#[tokio::test]
async fn test_rest_list_files_depth_zero() {
    let app = make_app();
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/files?depth=0")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_string(resp).await;
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    let entries = json["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 0, "depth=0 should return empty entries");
}

#[tokio::test]
async fn test_rest_list_files_not_found() {
    let app = make_app();
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/files?path=/nonexistent")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_rest_list_files_on_file_returns_conflict() {
    let app = make_app();
    // Create a file at /list-me.txt
    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/list-me.txt")
                .body(Body::from("data"))
                .unwrap(),
        )
        .await;

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/files?path=/list-me.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CONFLICT);
}
