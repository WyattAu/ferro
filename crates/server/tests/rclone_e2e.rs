//! Real rclone E2E integration test.
//!
//! Spawns an actual ferro-server process, mounts it via rclone, and verifies
//! file operations work correctly through the WebDAV protocol.
//!
//! These tests are ignored by default because they require `rclone` to be
//! installed. Run with: cargo test -p ferro-server --test rclone_e2e -- --ignored

use std::net::TcpListener;
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tokio::time::sleep;

/// Find a free port on localhost.
fn find_free_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap().port()
}

/// Spawn the ferro-server binary with a given port.
fn spawn_server(port: u16) -> Child {
    Command::new(env!("CARGO_BIN_EXE_ferro-server"))
        .env("RUST_LOG", "warn")
        .args(["--host", "127.0.0.1", "--port", &port.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to spawn ferro-server (run `cargo build -p ferro-server` first)")
}

/// Wait for the server to be ready by polling the health endpoint.
async fn wait_for_server(port: u16, max_wait: Duration) {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(1))
        .build()
        .unwrap();

    let url = format!("http://127.0.0.1:{}/.well-known/ferro", port);
    let start = std::time::Instant::now();

    loop {
        if client.get(&url).send().await.is_ok_and(|r| r.status().is_success()) {
            return;
        }
        if start.elapsed() > max_wait {
            panic!("Server did not start within {:?}", max_wait);
        }
        sleep(Duration::from_millis(100)).await;
    }
}

/// Run a set of WebDAV operations using reqwest against a real server.
async fn webdav_operations(port: u16) {
    let base = format!("http://127.0.0.1:{}", port);
    let client = reqwest::Client::new();

    // 1. OPTIONS — verify DAV header
    let resp = client
        .request(reqwest::Method::from_bytes(b"OPTIONS").unwrap(), &base)
        .send()
        .await
        .unwrap();
    assert!(resp.status().is_success());
    let dav = resp.headers().get("DAV").unwrap().to_str().unwrap();
    assert!(dav.contains("1"), "DAV header missing or wrong: {}", dav);

    // 2. MKCOL
    let resp = client
        .request(reqwest::Method::from_bytes(b"MKCOL").unwrap(), &format!("{}/e2e-test", base))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 201, "MKCOL failed");

    // 3. PUT
    let resp = client
        .put(&format!("{}/e2e-test/hello.txt", base))
        .header("Content-Type", "text/plain")
        .body("Hello from rclone E2E test!")
        .send()
        .await
        .unwrap();
    assert!(resp.status().is_success(), "PUT failed: {}", resp.status());

    // 4. GET
    let resp = client
        .get(&format!("{}/e2e-test/hello.txt", base))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 200);
    let body = resp.text().await.unwrap();
    assert_eq!(body, "Hello from rclone E2E test!");

    // 5. HEAD
    let resp = client
        .head(&format!("{}/e2e-test/hello.txt", base))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 200);
    assert!(resp.headers().contains_key("etag"));
    assert!(resp.headers().contains_key("content-length"));

    // 6. PROPFIND depth:0
    let resp = client
        .request(reqwest::Method::from_bytes(b"PROPFIND").unwrap(), &format!("{}/e2e-test", base))
        .header("Depth", "0")
        .header("Content-Type", "application/xml")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 207);
    let body = resp.text().await.unwrap();
    assert!(body.contains("e2e-test"), "PROPFIND should contain collection href");
    assert!(body.contains("<D:collection/>"), "Should be marked as collection");

    // 7. PROPFIND depth:1
    let resp = client
        .request(reqwest::Method::from_bytes(b"PROPFIND").unwrap(), &format!("{}/e2e-test", base))
        .header("Depth", "1")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 207);
    let body = resp.text().await.unwrap();
    assert!(body.contains("hello.txt"), "PROPFIND depth:1 should list files");

    // 8. COPY
    let resp = client
        .request(reqwest::Method::from_bytes(b"COPY").unwrap(), &format!("{}/e2e-test/hello.txt", base))
        .header("Destination", &format!("{}/e2e-test/hello-copy.txt", base))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 201);

    let resp = client
        .get(&format!("{}/e2e-test/hello-copy.txt", base))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 200);

    // 9. MOVE
    let resp = client
        .request(reqwest::Method::from_bytes(b"MOVE").unwrap(), &format!("{}/e2e-test/hello-copy.txt", base))
        .header("Destination", &format!("{}/e2e-test/hello-moved.txt", base))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 201);

    // Original should be gone
    let resp = client
        .get(&format!("{}/e2e-test/hello-copy.txt", base))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 404);

    // Moved file should exist
    let resp = client
        .get(&format!("{}/e2e-test/hello-moved.txt", base))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 200);

    // 10. DELETE
    let resp = client
        .delete(&format!("{}/e2e-test/hello-moved.txt", base))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 204);

    // 11. Conditional GET (If-None-Match → 304)
    let resp = client
        .put(&format!("{}/e2e-test/cached.txt", base))
        .body("cache test")
        .send()
        .await
        .unwrap();
    let etag = resp.headers().get("etag").unwrap().to_str().unwrap().to_string();

    let resp = client
        .get(&format!("{}/e2e-test/cached.txt", base))
        .header("If-None-Match", &etag)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 304, "Should return Not Modified");

    // 12. PROPPATCH
    let proppatch_body = r#"<?xml version="1.0" encoding="utf-8"?>
<D:propertyupdate xmlns:D="DAV:">
    <D:set>
        <D:prop>
            <D:displayname>Test Directory</D:displayname>
        </D:prop>
    </D:set>
</D:propertyupdate>"#;

    let resp = client
        .request(reqwest::Method::from_bytes(b"PROPPATCH").unwrap(), &format!("{}/e2e-test", base))
        .header("Content-Type", "application/xml")
        .body(proppatch_body)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 207, "PROPPATCH should return 207");

    // 13. LOCK + UNLOCK
    let resp = client
        .request(reqwest::Method::from_bytes(b"LOCK").unwrap(), &format!("{}/e2e-test/cached.txt", base))
        .header("Depth", "0")
        .header("Content-Type", "application/xml")
        .body(r#"<D:lockinfo xmlns:D="DAV:"><D:locktype><D:write/></D:locktype></D:lockinfo>"#)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 200);
    let lock_token = resp.headers().get("lock-token").unwrap().to_str().unwrap().to_string();

    // DELETE should fail without lock token
    let resp = client
        .delete(&format!("{}/e2e-test/cached.txt", base))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 423, "Should be locked");

    // UNLOCK
    let resp = client
        .request(reqwest::Method::from_bytes(b"UNLOCK").unwrap(), &format!("{}/e2e-test/cached.txt", base))
        .header("Lock-Token", &lock_token)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 204);

    // Now DELETE should succeed
    let resp = client
        .delete(&format!("{}/e2e-test/cached.txt", base))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 204);
}

#[tokio::test]
async fn test_real_server_e2e() {
    let port = find_free_port();
    let mut server = spawn_server(port);

    // Give the server time to start
    wait_for_server(port, Duration::from_secs(10)).await;

    // Run WebDAV operations
    webdav_operations(port).await;

    // Cleanup
    let _ = server.kill();
    let _ = server.wait();
}
