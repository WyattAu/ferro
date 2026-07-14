use base64::Engine;
use ferro_server::{AppState, build_router};
use std::net::TcpListener;
use tokio_util::sync::CancellationToken;

fn random_port() -> u16 {
    TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port()
}

async fn start_server() -> (u16, CancellationToken) {
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

    (port, shutdown_token)
}

async fn start_server_with_auth() -> (u16, CancellationToken) {
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

    (port, shutdown_token)
}

async fn wait_for_server(port: u16) {
    let client = reqwest::Client::new();
    for _ in 0..50 {
        if client
            .get(format!("http://127.0.0.1:{}/health", port))
            .send()
            .await
            .is_ok()
        {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    panic!("Server on port {} did not become ready", port);
}

#[tokio::test]
async fn test_server_starts_and_responds_to_health_check() {
    let (port, shutdown_token) = start_server().await;
    wait_for_server(port).await;

    let client = reqwest::Client::new();

    let resp = client
        .get(format!("http://127.0.0.1:{}/health", port))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "Healthy");

    let resp = client
        .get(format!("http://127.0.0.1:{}/healthz", port))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert_eq!(body, "ok");

    shutdown_token.cancel();
}

#[tokio::test]
async fn test_server_handles_webdav_requests() {
    let (port, shutdown_token) = start_server().await;
    wait_for_server(port).await;

    let client = reqwest::Client::new();

    let resp = client
        .request(
            reqwest::Method::from_bytes(b"MKCOL").unwrap(),
            format!("http://127.0.0.1:{}/webdav-test", port),
        )
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    let resp = client
        .request(
            reqwest::Method::from_bytes(b"PROPFIND").unwrap(),
            format!("http://127.0.0.1:{}/webdav-test", port),
        )
        .header("Depth", "0")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 207);

    let resp = client
        .put(format!("http://127.0.0.1:{}/webdav-test/lifecycle-test.txt", port))
        .body("hello lifecycle")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    let resp = client
        .get(format!("http://127.0.0.1:{}/webdav-test/lifecycle-test.txt", port))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert_eq!(body, "hello lifecycle");

    shutdown_token.cancel();
}

#[tokio::test]
async fn test_server_rejects_unauthorized_requests() {
    let (port, shutdown_token) = start_server_with_auth().await;
    wait_for_server(port).await;

    let client = reqwest::Client::new();

    let resp = client
        .get(format!("http://127.0.0.1:{}/api/v1/admin/stats", port))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);

    let auth = base64::engine::general_purpose::STANDARD.encode("admin:secret");
    let resp = client
        .get(format!("http://127.0.0.1:{}/api/v1/admin/stats", port))
        .header("Authorization", format!("Basic {}", auth))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    shutdown_token.cancel();
}

#[tokio::test]
async fn test_concurrent_requests_do_not_deadlock() {
    let (port, shutdown_token) = start_server().await;
    wait_for_server(port).await;

    let mut handles = vec![];

    for i in 0..10 {
        let handle = tokio::spawn(async move {
            let client = reqwest::Client::new();
            let path = format!("/concurrent-{}.txt", i);
            let content = format!("content-{}", i);

            let resp = client
                .put(format!("http://127.0.0.1:{}{}", port, path))
                .body(content.clone())
                .send()
                .await
                .unwrap();
            assert_eq!(resp.status(), 201);

            let resp = client
                .get(format!("http://127.0.0.1:{}{}", port, path))
                .send()
                .await
                .unwrap();
            assert_eq!(resp.status(), 200);
            let body = resp.text().await.unwrap();
            assert_eq!(body, content);
        });
        handles.push(handle);
    }

    let result = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        for h in handles {
            h.await.unwrap();
        }
    })
    .await;

    assert!(result.is_ok(), "Concurrent requests deadlocked within 5s");

    shutdown_token.cancel();
}
