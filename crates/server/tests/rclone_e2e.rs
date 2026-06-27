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
        if client
            .get(&url)
            .send()
            .await
            .is_ok_and(|r| r.status().is_success())
        {
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
        .request(
            reqwest::Method::from_bytes(b"MKCOL").unwrap(),
            format!("{}/e2e-test", base),
        )
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 201, "MKCOL failed");

    // 3. PUT
    let resp = client
        .put(format!("{}/e2e-test/hello.txt", base))
        .header("Content-Type", "text/plain")
        .body("Hello from rclone E2E test!")
        .send()
        .await
        .unwrap();
    assert!(resp.status().is_success(), "PUT failed: {}", resp.status());

    // 4. GET
    let resp = client
        .get(format!("{}/e2e-test/hello.txt", base))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 200);
    let body = resp.text().await.unwrap();
    assert_eq!(body, "Hello from rclone E2E test!");

    // 5. HEAD
    let resp = client
        .head(format!("{}/e2e-test/hello.txt", base))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 200);
    assert!(resp.headers().contains_key("etag"));
    assert!(resp.headers().contains_key("content-length"));

    // 6. PROPFIND depth:0
    let resp = client
        .request(
            reqwest::Method::from_bytes(b"PROPFIND").unwrap(),
            format!("{}/e2e-test", base),
        )
        .header("Depth", "0")
        .header("Content-Type", "application/xml")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 207);
    let body = resp.text().await.unwrap();
    assert!(
        body.contains("e2e-test"),
        "PROPFIND should contain collection href"
    );
    assert!(
        body.contains("<D:collection/>"),
        "Should be marked as collection"
    );

    // 7. PROPFIND depth:1
    let resp = client
        .request(
            reqwest::Method::from_bytes(b"PROPFIND").unwrap(),
            format!("{}/e2e-test", base),
        )
        .header("Depth", "1")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 207);
    let body = resp.text().await.unwrap();
    assert!(
        body.contains("hello.txt"),
        "PROPFIND depth:1 should list files"
    );

    // 8. COPY
    let resp = client
        .request(
            reqwest::Method::from_bytes(b"COPY").unwrap(),
            format!("{}/e2e-test/hello.txt", base),
        )
        .header("Destination", format!("{}/e2e-test/hello-copy.txt", base))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 201);

    let resp = client
        .get(format!("{}/e2e-test/hello-copy.txt", base))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 200);

    // 9. MOVE
    let resp = client
        .request(
            reqwest::Method::from_bytes(b"MOVE").unwrap(),
            format!("{}/e2e-test/hello-copy.txt", base),
        )
        .header("Destination", format!("{}/e2e-test/hello-moved.txt", base))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 201);

    // Original should be gone
    let resp = client
        .get(format!("{}/e2e-test/hello-copy.txt", base))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 404);

    // Moved file should exist
    let resp = client
        .get(format!("{}/e2e-test/hello-moved.txt", base))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 200);

    // 10. DELETE
    let resp = client
        .delete(format!("{}/e2e-test/hello-moved.txt", base))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 204);

    // 11. Conditional GET (If-None-Match → 304)
    let resp = client
        .put(format!("{}/e2e-test/cached.txt", base))
        .body("cache test")
        .send()
        .await
        .unwrap();
    let etag = resp
        .headers()
        .get("etag")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    let resp = client
        .get(format!("{}/e2e-test/cached.txt", base))
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
        .request(
            reqwest::Method::from_bytes(b"PROPPATCH").unwrap(),
            format!("{}/e2e-test", base),
        )
        .header("Content-Type", "application/xml")
        .body(proppatch_body)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 207, "PROPPATCH should return 207");

    // 13. LOCK + UNLOCK
    let resp = client
        .request(
            reqwest::Method::from_bytes(b"LOCK").unwrap(),
            format!("{}/e2e-test/cached.txt", base),
        )
        .header("Depth", "0")
        .header("Content-Type", "application/xml")
        .body(r#"<D:lockinfo xmlns:D="DAV:"><D:locktype><D:write/></D:locktype></D:lockinfo>"#)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 200);
    let lock_token = resp
        .headers()
        .get("lock-token")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    // DELETE should fail without lock token
    let resp = client
        .delete(format!("{}/e2e-test/cached.txt", base))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 423, "Should be locked");

    // UNLOCK
    let resp = client
        .request(
            reqwest::Method::from_bytes(b"UNLOCK").unwrap(),
            format!("{}/e2e-test/cached.txt", base),
        )
        .header("Lock-Token", &lock_token)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 204);

    // Now DELETE should succeed
    let resp = client
        .delete(format!("{}/e2e-test/cached.txt", base))
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

// ── Enhanced rclone E2E tests ────────────────────────────────────────
//
// These tests exercise rclone CLI commands against a live ferro-server.
// They are ignored by default and require `rclone` to be installed.
// Run with: cargo test -p ferro-server --test rclone_e2e -- --ignored

fn rclone_config_name() -> &'static str {
    "ferro-e2e"
}

fn rclone_env(port: u16) -> Vec<(String, String)> {
    let config = format!(
        "[{}]\n\
         type = webdav\n\
         url = http://127.0.0.1:{}/\n\
         vendor = other\n\
         user = \n\
         pass = \n",
        rclone_config_name(),
        port
    );
    let config_path = format!("/tmp/ferro-rclone-e2e-{}.conf", port);
    std::fs::write(&config_path, &config).unwrap();
    vec![("RCLONE_CONFIG".to_string(), config_path)]
}

fn rclone_cmd(port: u16) -> Command {
    let mut cmd = Command::new("rclone");
    let env = rclone_env(port);
    for (k, v) in &env {
        cmd.env(k, v);
    }
    cmd.env("RCLONE_VERBOSE", "0");
    cmd
}

fn rclone_success(label: &str, output: &std::process::Output) {
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!(
            "{} failed (exit {:?}): {}",
            label,
            output.status.code(),
            stderr
        );
    }
}

fn setup_remote_dir(port: u16, path: &str) {
    let client = reqwest::Client::new();
    let base = format!("http://127.0.0.1:{}", port);

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let mkcol = reqwest::Method::from_bytes(b"MKCOL").unwrap();
        let resp = client
            .request(mkcol, format!("{}{}", base, path))
            .send()
            .await
            .unwrap();
        assert!(
            resp.status().as_u16() == 201 || resp.status().as_u16() == 405,
            "MKCOL {} should succeed (201) or already exist (405), got {}",
            path,
            resp.status()
        );
    });
}

fn put_remote_file(port: u16, remote_path: &str, content: &str) {
    let client = reqwest::Client::new();
    let base = format!("http://127.0.0.1:{}", port);

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let resp = client
            .put(format!("{}{}", base, remote_path))
            .body(content.to_string())
            .send()
            .await
            .unwrap();
        assert!(
            resp.status().is_success(),
            "PUT {} failed: {}",
            remote_path,
            resp.status()
        );
    });
}

fn get_remote_file(port: u16, remote_path: &str) -> String {
    let client = reqwest::Client::new();
    let base = format!("http://127.0.0.1:{}", port);

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let resp = client
            .get(format!("{}{}", base, remote_path))
            .send()
            .await
            .unwrap();
        assert_eq!(
            resp.status().as_u16(),
            200,
            "GET {} should return 200",
            remote_path
        );
        resp.text().await.unwrap()
    })
}

#[allow(dead_code)]
fn delete_remote_file(port: u16, remote_path: &str) {
    let client = reqwest::Client::new();
    let base = format!("http://127.0.0.1:{}", port);

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let _ = client
            .delete(format!("{}{}", base, remote_path))
            .send()
            .await;
    });
}

#[tokio::test]
#[ignore]
async fn test_rclone_copy() {
    let port = find_free_port();
    let mut server = spawn_server(port);
    wait_for_server(port, Duration::from_secs(10)).await;

    let local_dir = tempfile::tempdir().unwrap();
    let local_file = local_dir.path().join("copy-me.txt");
    std::fs::write(&local_file, "rclone copy test content").unwrap();

    setup_remote_dir(port, "/rclone-copy");

    let remote = format!("{}:/rclone-copy/", rclone_config_name());
    let output = rclone_cmd(port)
        .args(["copy", local_dir.path().to_str().unwrap(), &remote])
        .output()
        .unwrap();
    rclone_success("rclone copy", &output);

    let content = get_remote_file(port, "/rclone-copy/copy-me.txt");
    assert_eq!(content, "rclone copy test content");

    let _ = server.kill();
    let _ = server.wait();
}

#[tokio::test]
#[ignore]
async fn test_rclone_sync() {
    let port = find_free_port();
    let mut server = spawn_server(port);
    wait_for_server(port, Duration::from_secs(10)).await;

    let local_dir = tempfile::tempdir().unwrap();
    std::fs::write(local_dir.path().join("a.txt"), "aaa").unwrap();
    std::fs::write(local_dir.path().join("b.txt"), "bbb").unwrap();

    setup_remote_dir(port, "/rclone-sync");
    put_remote_file(port, "/rclone-sync/stale.txt", "should be removed");

    let remote = format!("{}:/rclone-sync/", rclone_config_name());
    let output = rclone_cmd(port)
        .args(["sync", local_dir.path().to_str().unwrap(), &remote])
        .output()
        .unwrap();
    rclone_success("rclone sync", &output);

    assert_eq!(get_remote_file(port, "/rclone-sync/a.txt"), "aaa");
    assert_eq!(get_remote_file(port, "/rclone-sync/b.txt"), "bbb");

    let _ = server.kill();
    let _ = server.wait();
}

#[tokio::test]
#[ignore]
async fn test_rclone_move() {
    let port = find_free_port();
    let mut server = spawn_server(port);
    wait_for_server(port, Duration::from_secs(10)).await;

    let local_dir = tempfile::tempdir().unwrap();
    std::fs::write(local_dir.path().join("move-me.txt"), "moved content").unwrap();

    setup_remote_dir(port, "/rclone-move");

    let remote = format!("{}:/rclone-move/", rclone_config_name());
    let output = rclone_cmd(port)
        .args(["move", local_dir.path().to_str().unwrap(), &remote])
        .output()
        .unwrap();
    rclone_success("rclone move", &output);

    assert_eq!(
        get_remote_file(port, "/rclone-move/move-me.txt"),
        "moved content"
    );
    assert!(
        !local_dir.path().join("move-me.txt").exists(),
        "Local file should be deleted after rclone move"
    );

    let _ = server.kill();
    let _ = server.wait();
}

#[tokio::test]
#[ignore]
async fn test_rclone_ls() {
    let port = find_free_port();
    let mut server = spawn_server(port);
    wait_for_server(port, Duration::from_secs(10)).await;

    setup_remote_dir(port, "/rclone-ls");
    put_remote_file(port, "/rclone-ls/file1.txt", "one");
    put_remote_file(port, "/rclone-ls/file2.txt", "two");

    let remote = format!("{}:/rclone-ls/", rclone_config_name());
    let output = rclone_cmd(port).args(["ls", &remote]).output().unwrap();
    rclone_success("rclone ls", &output);

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    assert!(
        stdout.contains("file1.txt"),
        "ls output should contain file1.txt"
    );
    assert!(
        stdout.contains("file2.txt"),
        "ls output should contain file2.txt"
    );

    let _ = server.kill();
    let _ = server.wait();
}

#[tokio::test]
#[ignore]
async fn test_rclone_size() {
    let port = find_free_port();
    let mut server = spawn_server(port);
    wait_for_server(port, Duration::from_secs(10)).await;

    setup_remote_dir(port, "/rclone-size");
    put_remote_file(port, "/rclone-size/small.txt", "12345");

    let remote = format!("{}:/rclone-size/", rclone_config_name());
    let output = rclone_cmd(port).args(["size", &remote]).output().unwrap();
    rclone_success("rclone size", &output);

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    assert!(
        stdout.contains("5"),
        "size output should contain the byte count (5)"
    );

    let _ = server.kill();
    let _ = server.wait();
}

#[tokio::test]
#[ignore]
async fn test_rclone_check() {
    let port = find_free_port();
    let mut server = spawn_server(port);
    wait_for_server(port, Duration::from_secs(10)).await;

    let local_dir = tempfile::tempdir().unwrap();
    std::fs::write(local_dir.path().join("check.txt"), "identical content").unwrap();

    setup_remote_dir(port, "/rclone-check");
    put_remote_file(port, "/rclone-check/check.txt", "identical content");

    let remote = format!("{}:/rclone-check/", rclone_config_name());
    let output = rclone_cmd(port)
        .args(["check", local_dir.path().to_str().unwrap(), &remote])
        .output()
        .unwrap();
    rclone_success("rclone check", &output);

    let _ = server.kill();
    let _ = server.wait();
}

#[tokio::test]
#[ignore]
async fn test_rclone_large_file() {
    let port = find_free_port();
    let mut server = spawn_server(port);
    wait_for_server(port, Duration::from_secs(10)).await;

    let local_dir = tempfile::tempdir().unwrap();
    let large_content = "A".repeat(10 * 1024 * 1024);
    let large_file = local_dir.path().join("large.dat");
    std::fs::write(&large_file, &large_content).unwrap();

    setup_remote_dir(port, "/rclone-large");

    let remote = format!("{}:/rclone-large/", rclone_config_name());
    let output = rclone_cmd(port)
        .args(["copy", local_dir.path().to_str().unwrap(), &remote])
        .output()
        .unwrap();
    rclone_success("rclone copy large file", &output);

    let content = get_remote_file(port, "/rclone-large/large.dat");
    assert_eq!(
        content.len(),
        10 * 1024 * 1024,
        "Large file size should match"
    );

    let _ = server.kill();
    let _ = server.wait();
}

#[tokio::test]
#[ignore]
async fn test_rclone_special_characters() {
    let port = find_free_port();
    let mut server = spawn_server(port);
    wait_for_server(port, Duration::from_secs(10)).await;

    let local_dir = tempfile::tempdir().unwrap();
    let special_names = [
        "file with spaces.txt",
        "file-with-dashes.txt",
        "file_with_underscores.txt",
        "file.many.dots.txt",
        "file(1).txt",
    ];

    for name in &special_names {
        std::fs::write(local_dir.path().join(name), format!("content of {}", name)).unwrap();
    }

    setup_remote_dir(port, "/rclone-special");

    let remote = format!("{}:/rclone-special/", rclone_config_name());
    let output = rclone_cmd(port)
        .args(["copy", local_dir.path().to_str().unwrap(), &remote])
        .output()
        .unwrap();
    rclone_success("rclone copy special chars", &output);

    for name in &special_names {
        let encoded = name
            .replace(' ', "%20")
            .replace('(', "%28")
            .replace(')', "%29");
        let content = get_remote_file(port, &format!("/rclone-special/{}", encoded));
        assert_eq!(
            content,
            format!("content of {}", name),
            "File '{}' content mismatch",
            name
        );
    }

    let _ = server.kill();
    let _ = server.wait();
}

#[tokio::test]
#[ignore]
async fn test_rclone_concurrent_operations() {
    let port = find_free_port();
    let mut server = spawn_server(port);
    wait_for_server(port, Duration::from_secs(10)).await;

    setup_remote_dir(port, "/rclone-concurrent");

    let mut handles = Vec::new();
    for i in 0..3 {
        handles.push(std::thread::spawn(move || {
            let local_dir = tempfile::tempdir().unwrap();
            std::fs::write(
                local_dir.path().join(format!("worker-{}.txt", i)),
                format!("data from worker {}", i),
            )
            .unwrap();

            let remote = format!("{}:/rclone-concurrent/worker-{}/", rclone_config_name(), i);
            let output = rclone_cmd(port)
                .args(["copy", local_dir.path().to_str().unwrap(), &remote])
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "Concurrent rclone copy {} failed: {}",
                i,
                String::from_utf8_lossy(&output.stderr)
            );
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    for i in 0..3 {
        let content = get_remote_file(
            port,
            &format!("/rclone-concurrent/worker-{}/worker-{}.txt", i, i),
        );
        assert_eq!(content, format!("data from worker {}", i));
    }

    let _ = server.kill();
    let _ = server.wait();
}

#[tokio::test]
#[ignore]
async fn test_rclone_copy_progress() {
    let port = find_free_port();
    let mut server = spawn_server(port);
    wait_for_server(port, Duration::from_secs(10)).await;

    let local_dir = tempfile::tempdir().unwrap();
    std::fs::write(local_dir.path().join("progress.txt"), "progress test").unwrap();

    setup_remote_dir(port, "/rclone-progress");

    let remote = format!("{}:/rclone-progress/", rclone_config_name());
    let output = rclone_cmd(port)
        .args([
            "copy",
            "--progress",
            local_dir.path().to_str().unwrap(),
            &remote,
        ])
        .output()
        .unwrap();
    rclone_success("rclone copy --progress", &output);

    let content = get_remote_file(port, "/rclone-progress/progress.txt");
    assert_eq!(content, "progress test");

    let _ = server.kill();
    let _ = server.wait();
}
