use base64::Engine;
use ferro_server::{AppState, build_router};
use std::net::TcpListener;
use tokio_util::sync::CancellationToken;

fn random_port() -> u16 {
    TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port()
}

async fn start_server() -> (String, CancellationToken) {
    let state = AppState::in_memory();
    let app = build_router(state);
    let port = random_port();
    let shutdown_token = CancellationToken::new();

    let token = shutdown_token.clone();
    tokio::spawn(async move {
        let addr = format!("127.0.0.1:{}", port);
        let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
        axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                token.cancelled().await;
            })
            .await
            .unwrap();
    });

    (format!("http://127.0.0.1:{}", port), shutdown_token)
}

async fn start_server_with_auth() -> (String, CancellationToken) {
    let state = AppState::in_memory()
        .with_admin_user(Some("admin".to_string()))
        .with_admin_password(Some("secret".to_string()));
    let app = build_router(state);
    let port = random_port();
    let shutdown_token = CancellationToken::new();

    let token = shutdown_token.clone();
    tokio::spawn(async move {
        let addr = format!("127.0.0.1:{}", port);
        let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
        axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                token.cancelled().await;
            })
            .await
            .unwrap();
    });

    (format!("http://127.0.0.1:{}", port), shutdown_token)
}

async fn wait_for_server(base_url: &str) {
    let client = reqwest::Client::new();
    for _ in 0..50 {
        if client.get(format!("{}/healthz", base_url)).send().await.is_ok() {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    panic!("Server at {} did not become ready", base_url);
}

fn basic_auth_header() -> String {
    let auth = base64::engine::general_purpose::STANDARD.encode("admin:secret");
    format!("Basic {}", auth)
}

// ============================================================
// Auth endpoints
// ============================================================

#[tokio::test]
async fn test_auth_info_returns_anonymous_without_auth() {
    let (base_url, ct) = start_server().await;
    wait_for_server(&base_url).await;

    let client = reqwest::Client::new();
    let resp = client.get(format!("{}/api/auth/info", base_url)).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["sub"], "anonymous");
    assert_eq!(body["auth_type"], "none");

    ct.cancel();
}

#[tokio::test]
async fn test_auth_info_returns_basic_with_admin_configured() {
    let (base_url, ct) = start_server_with_auth().await;
    wait_for_server(&base_url).await;

    let client = reqwest::Client::new();
    let resp = client.get(format!("{}/api/auth/info", base_url)).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["sub"], "anonymous");
    assert_eq!(body["auth_type"], "basic");

    ct.cancel();
}

#[tokio::test]
async fn test_auth_login_returns_503_without_oidc() {
    let (base_url, ct) = start_server().await;
    wait_for_server(&base_url).await;

    let client = reqwest::Client::new();
    let resp = client.get(format!("{}/api/auth/login", base_url)).send().await.unwrap();
    assert_eq!(resp.status(), 503);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["error"], "OIDC not configured");

    ct.cancel();
}

// ============================================================
// File operations — happy paths
// ============================================================

#[tokio::test]
async fn test_list_files_root_empty() {
    let (base_url, ct) = start_server().await;
    wait_for_server(&base_url).await;

    let client = reqwest::Client::new();
    let resp = client.get(format!("{}/api/v1/files", base_url)).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    let entries = body["entries"].as_array().unwrap();
    assert!(entries.is_empty());

    ct.cancel();
}

#[tokio::test]
async fn test_put_file_returns_201() {
    let (base_url, ct) = start_server().await;
    wait_for_server(&base_url).await;

    let client = reqwest::Client::new();
    let resp = client
        .put(format!("{}/api/v1/files/test-file.txt", base_url))
        .body("hello world")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["path"], "/test-file.txt");
    assert_eq!(body["size"], 11);

    ct.cancel();
}

#[tokio::test]
async fn test_put_then_get_file_roundtrip() {
    let (base_url, ct) = start_server().await;
    wait_for_server(&base_url).await;

    let client = reqwest::Client::new();
    let content = "roundtrip content 123";

    // PUT
    let resp = client
        .put(format!("{}/api/v1/files/roundtrip.txt", base_url))
        .body(content)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    // GET
    let resp = client
        .get(format!("{}/api/v1/files/roundtrip.txt", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert_eq!(body, content);

    ct.cancel();
}

#[tokio::test]
async fn test_delete_file_returns_204() {
    let (base_url, ct) = start_server().await;
    wait_for_server(&base_url).await;

    let client = reqwest::Client::new();

    // Create
    let resp = client
        .put(format!("{}/api/v1/files/delete-me.txt", base_url))
        .body("delete me")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    // Delete
    let resp = client
        .delete(format!("{}/api/v1/files/delete-me.txt", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 204);

    ct.cancel();
}

#[tokio::test]
async fn test_get_deleted_file_returns_404() {
    let (base_url, ct) = start_server().await;
    wait_for_server(&base_url).await;

    let client = reqwest::Client::new();

    // Create
    let resp = client
        .put(format!("{}/api/v1/files/ghost.txt", base_url))
        .body("ephemeral")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    // Delete
    let resp = client
        .delete(format!("{}/api/v1/files/ghost.txt", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 204);

    // GET should 404
    let resp = client
        .get(format!("{}/api/v1/files/ghost.txt", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);

    ct.cancel();
}

#[tokio::test]
async fn test_put_file_in_subdirectory() {
    let (base_url, ct) = start_server().await;
    wait_for_server(&base_url).await;

    let client = reqwest::Client::new();

    let resp = client
        .put(format!("{}/api/v1/files/dir/nested.txt", base_url))
        .body("nested content")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["path"], "/dir/nested.txt");

    ct.cancel();
}

#[tokio::test]
async fn test_list_files_subdirectory() {
    let (base_url, ct) = start_server().await;
    wait_for_server(&base_url).await;

    let client = reqwest::Client::new();

    // Create a collection first
    client
        .request(
            reqwest::Method::from_bytes(b"MKCOL").unwrap(),
            format!("{}/mydir/", base_url),
        )
        .send()
        .await
        .unwrap();

    // Create files in the subdirectory
    client
        .put(format!("{}/api/v1/files/mydir/a.txt", base_url))
        .body("aaa")
        .send()
        .await
        .unwrap();
    client
        .put(format!("{}/api/v1/files/mydir/b.txt", base_url))
        .body("bbb")
        .send()
        .await
        .unwrap();

    // List the subdirectory
    let resp = client
        .get(format!("{}/api/v1/files?path=/mydir", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    let entries = body["entries"].as_array().unwrap();
    let names: Vec<&str> = entries.iter().map(|e| e["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"a.txt"));
    assert!(names.contains(&"b.txt"));

    ct.cancel();
}

#[tokio::test]
async fn test_list_files_after_put() {
    let (base_url, ct) = start_server().await;
    wait_for_server(&base_url).await;

    let client = reqwest::Client::new();

    client
        .put(format!("{}/api/v1/files/listed.txt", base_url))
        .body("listed")
        .send()
        .await
        .unwrap();

    let resp = client.get(format!("{}/api/v1/files", base_url)).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    let entries = body["entries"].as_array().unwrap();
    let names: Vec<&str> = entries.iter().map(|e| e["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"listed.txt"));

    ct.cancel();
}

#[tokio::test]
async fn test_put_file_overwrite() {
    let (base_url, ct) = start_server().await;
    wait_for_server(&base_url).await;

    let client = reqwest::Client::new();

    // First write
    let resp = client
        .put(format!("{}/api/v1/files/overwrite.txt", base_url))
        .body("version1")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    // Overwrite
    let resp = client
        .put(format!("{}/api/v1/files/overwrite.txt", base_url))
        .body("version2")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    // Read back
    let resp = client
        .get(format!("{}/api/v1/files/overwrite.txt", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert_eq!(body, "version2");

    ct.cancel();
}

// ============================================================
// Batch operations
// ============================================================

#[tokio::test]
async fn test_batch_copy() {
    let (base_url, ct) = start_server().await;
    wait_for_server(&base_url).await;

    let client = reqwest::Client::new();

    // Create source file
    client
        .put(format!("{}/api/v1/files/source.txt", base_url))
        .body("batch copy source")
        .send()
        .await
        .unwrap();

    // Batch copy
    let resp = client
        .post(format!("{}/api/v1/batch/copy", base_url))
        .header("content-type", "application/json")
        .body(
            serde_json::json!({
                "operations": [
                    {"from": "/source.txt", "to": "/dest.txt"}
                ]
            })
            .to_string(),
        )
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    let results = body["results"].as_array().unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["status"], "ok");

    // Verify both exist
    let resp = client
        .get(format!("{}/api/v1/files/dest.txt", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let content = resp.text().await.unwrap();
    assert_eq!(content, "batch copy source");

    ct.cancel();
}

#[tokio::test]
async fn test_batch_move() {
    let (base_url, ct) = start_server().await;
    wait_for_server(&base_url).await;

    let client = reqwest::Client::new();

    // Create source file
    client
        .put(format!("{}/api/v1/files/move-src.txt", base_url))
        .body("batch move source")
        .send()
        .await
        .unwrap();

    // Batch move
    let resp = client
        .post(format!("{}/api/v1/batch/move", base_url))
        .header("content-type", "application/json")
        .body(
            serde_json::json!({
                "operations": [
                    {"from": "/move-src.txt", "to": "/move-dst.txt"}
                ]
            })
            .to_string(),
        )
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    let results = body["results"].as_array().unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["status"], "ok");

    // Verify destination exists with correct content
    let resp = client
        .get(format!("{}/api/v1/files/move-dst.txt", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let content = resp.text().await.unwrap();
    assert_eq!(content, "batch move source");

    // Verify source is gone
    let resp = client
        .get(format!("{}/api/v1/files/move-src.txt", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);

    ct.cancel();
}

#[tokio::test]
async fn test_batch_delete() {
    let (base_url, ct) = start_server().await;
    wait_for_server(&base_url).await;

    let client = reqwest::Client::new();

    // Create files
    client
        .put(format!("{}/api/v1/files/del-a.txt", base_url))
        .body("aaa")
        .send()
        .await
        .unwrap();
    client
        .put(format!("{}/api/v1/files/del-b.txt", base_url))
        .body("bbb")
        .send()
        .await
        .unwrap();

    // Batch delete
    let resp = client
        .post(format!("{}/api/v1/batch/delete", base_url))
        .header("content-type", "application/json")
        .body(
            serde_json::json!({
                "paths": ["/del-a.txt", "/del-b.txt"]
            })
            .to_string(),
        )
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    let succeeded = body["succeeded"].as_array().unwrap();
    assert_eq!(succeeded.len(), 2);

    // Verify both are gone
    let resp = client
        .get(format!("{}/api/v1/files/del-a.txt", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);

    let resp = client
        .get(format!("{}/api/v1/files/del-b.txt", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);

    ct.cancel();
}

// ============================================================
// REST copy and move (single file)
// ============================================================

#[tokio::test]
async fn test_rest_copy_file() {
    let (base_url, ct) = start_server().await;
    wait_for_server(&base_url).await;

    let client = reqwest::Client::new();

    client
        .put(format!("{}/api/v1/files/copy-src.txt", base_url))
        .body("copy me")
        .send()
        .await
        .unwrap();

    let resp = client
        .post(format!("{}/api/v1/files/copy", base_url))
        .header("content-type", "application/json")
        .body(
            serde_json::json!({
                "source": "/copy-src.txt",
                "destination": "/copy-dst.txt"
            })
            .to_string(),
        )
        .send()
        .await
        .unwrap();
    assert!(resp.status().is_success(), "copy should succeed, got {}", resp.status());

    // Verify destination
    let resp = client
        .get(format!("{}/api/v1/files/copy-dst.txt", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let content = resp.text().await.unwrap();
    assert_eq!(content, "copy me");

    ct.cancel();
}

#[tokio::test]
async fn test_rest_move_file() {
    let (base_url, ct) = start_server().await;
    wait_for_server(&base_url).await;

    let client = reqwest::Client::new();

    client
        .put(format!("{}/api/v1/files/move-rest.txt", base_url))
        .body("move me")
        .send()
        .await
        .unwrap();

    let resp = client
        .post(format!("{}/api/v1/files/move", base_url))
        .header("content-type", "application/json")
        .body(
            serde_json::json!({
                "source": "/move-rest.txt",
                "destination": "/moved-rest.txt"
            })
            .to_string(),
        )
        .send()
        .await
        .unwrap();
    assert!(resp.status().is_success(), "move should succeed, got {}", resp.status());

    // Verify destination
    let resp = client
        .get(format!("{}/api/v1/files/moved-rest.txt", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let content = resp.text().await.unwrap();
    assert_eq!(content, "move me");

    // Verify source is gone
    let resp = client
        .get(format!("{}/api/v1/files/move-rest.txt", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);

    ct.cancel();
}

// ============================================================
// WebDAV operations
// ============================================================

#[tokio::test]
async fn test_webdav_propfind_collection_depth_one() {
    let (base_url, ct) = start_server().await;
    wait_for_server(&base_url).await;

    let client = reqwest::Client::new();

    // Create a collection first
    let resp = client
        .request(
            reqwest::Method::from_bytes(b"MKCOL").unwrap(),
            format!("{}/propfind-test/", base_url),
        )
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    // PROPFIND on the collection with Depth: 1
    let resp = client
        .request(
            reqwest::Method::from_bytes(b"PROPFIND").unwrap(),
            format!("{}/propfind-test/", base_url),
        )
        .header("Depth", "1")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 207);

    ct.cancel();
}

#[tokio::test]
async fn test_webdav_mkcol_put_get_delete() {
    let (base_url, ct) = start_server().await;
    wait_for_server(&base_url).await;

    let client = reqwest::Client::new();

    // MKCOL
    let resp = client
        .request(
            reqwest::Method::from_bytes(b"MKCOL").unwrap(),
            format!("{}/webtest/", base_url),
        )
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    // PUT
    let resp = client
        .put(format!("{}/webtest/file.txt", base_url))
        .body("webdav content")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    // GET
    let resp = client
        .get(format!("{}/webtest/file.txt", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert_eq!(body, "webdav content");

    // DELETE
    let resp = client
        .delete(format!("{}/webtest/file.txt", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 204);

    ct.cancel();
}

#[tokio::test]
async fn test_webdav_propfind_collection_after_delete() {
    let (base_url, ct) = start_server().await;
    wait_for_server(&base_url).await;

    let client = reqwest::Client::new();

    // Create collection with a file
    client
        .request(
            reqwest::Method::from_bytes(b"MKCOL").unwrap(),
            format!("{}/emptytest/", base_url),
        )
        .send()
        .await
        .unwrap();

    client
        .put(format!("{}/emptytest/temp.txt", base_url))
        .body("temp")
        .send()
        .await
        .unwrap();

    // Delete the file
    client
        .delete(format!("{}/emptytest/temp.txt", base_url))
        .send()
        .await
        .unwrap();

    // PROPFIND the now-empty collection
    let resp = client
        .request(
            reqwest::Method::from_bytes(b"PROPFIND").unwrap(),
            format!("{}/emptytest/", base_url),
        )
        .header("Depth", "0")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 207);

    ct.cancel();
}

// ============================================================
// Error handling
// ============================================================

#[tokio::test]
async fn test_get_nonexistent_file_returns_404() {
    let (base_url, ct) = start_server().await;
    wait_for_server(&base_url).await;

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/api/v1/files/nonexistent.txt", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["error"], "not_found");

    ct.cancel();
}

#[tokio::test]
async fn test_delete_nonexistent_file_returns_404() {
    let (base_url, ct) = start_server().await;
    wait_for_server(&base_url).await;

    let client = reqwest::Client::new();
    let resp = client
        .delete(format!("{}/api/v1/files/never-existed.txt", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);

    ct.cancel();
}

#[tokio::test]
async fn test_admin_stats_requires_auth() {
    let (base_url, ct) = start_server_with_auth().await;
    wait_for_server(&base_url).await;

    let client = reqwest::Client::new();

    // Without auth — should return 401
    let resp = client
        .get(format!("{}/api/v1/admin/stats", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);

    // With valid auth — should return 200
    let resp = client
        .get(format!("{}/api/v1/admin/stats", base_url))
        .header("Authorization", basic_auth_header())
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    ct.cancel();
}

#[tokio::test]
async fn test_admin_stats_without_auth_configured_returns_200() {
    let (base_url, ct) = start_server().await;
    wait_for_server(&base_url).await;

    let client = reqwest::Client::new();

    // No auth configured, so admin stats should be accessible
    let resp = client
        .get(format!("{}/api/v1/admin/stats", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    ct.cancel();
}

#[tokio::test]
async fn test_health_endpoints() {
    let (base_url, ct) = start_server().await;
    wait_for_server(&base_url).await;

    let client = reqwest::Client::new();

    let resp = client.get(format!("{}/health", base_url)).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "Healthy");

    let resp = client.get(format!("{}/healthz", base_url)).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let text = resp.text().await.unwrap();
    assert_eq!(text, "ok");

    ct.cancel();
}

// ============================================================
// Server config endpoint
// ============================================================

#[tokio::test]
async fn test_api_config_all_fields_present() {
    let (base_url, ct) = start_server().await;
    wait_for_server(&base_url).await;

    let client = reqwest::Client::new();
    let resp = client.get(format!("{}/api/v1/config", base_url)).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    let expected_fields = ["version", "auth_enabled", "search_enabled", "wasm_enabled", "storage"];
    for field in &expected_fields {
        assert!(body.get(*field).is_some(), "Missing field: {}", field);
    }

    ct.cancel();
}
