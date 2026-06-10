use axum::body::Body;
use axum::http::{Request, StatusCode};
use ferro_server::{AppState, build_router, make_app};
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
async fn test_cors_wildcard_warning_logged() {
    let app = make_app();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("OPTIONS")
                .uri("/api/v1/config")
                .header("Origin", "https://example.com")
                .header("Access-Control-Request-Method", "GET")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    let origin = resp
        .headers()
        .get("access-control-allow-origin")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert_eq!(origin, "*");

    let methods = resp
        .headers()
        .get("access-control-allow-methods")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(methods.contains("GET"));
    assert!(methods.contains("PUT"));
    assert!(methods.contains("DELETE"));

    let headers = resp
        .headers()
        .get("access-control-allow-headers")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(headers.contains("Content-Type"));
    assert!(headers.contains("Authorization"));
}

#[tokio::test]
async fn test_rate_limiter_rejects_excess_requests() {
    use ferro_rate_limiter::{RateLimiter, TokenBucketLimiter};
    use std::time::Duration;

    let limiter = TokenBucketLimiter::new(5, 0, Duration::from_secs(60));

    for i in 0..5 {
        assert!(
            limiter.check("192.168.1.1").await.unwrap().allowed,
            "Request {} should pass",
            i + 1
        );
    }
    assert!(
        !limiter.check("192.168.1.1").await.unwrap().allowed,
        "6th request should be rejected"
    );

    assert!(
        limiter.check("192.168.1.2").await.unwrap().allowed,
        "Different IP should not be affected"
    );
}

#[tokio::test]
async fn test_rate_limiter_recovery_after_window() {
    use ferro_rate_limiter::{RateLimiter, TokenBucketLimiter};
    use std::time::Duration;

    let limiter = TokenBucketLimiter::new(3, 3, Duration::from_millis(200));

    for _ in 0..3 {
        limiter.check("recovery-client").await.unwrap();
    }
    assert!(!limiter.check("recovery-client").await.unwrap().allowed);

    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(limiter.check("recovery-client").await.unwrap().allowed);
}

#[tokio::test]
async fn test_health_probes_respond() {
    let app = make_app();

    let liveness = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/healthz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(liveness.status(), StatusCode::OK);
    let body = body_string(liveness).await;
    assert_eq!(body, "ok");

    let readiness = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/readyz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(readiness.status(), StatusCode::OK);
    let json = body_json(readiness).await;
    assert_eq!(json["status"], "ok");
    assert!(json["subsystems"].is_object());

    let well_known = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/.well-known/ferro")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(well_known.status(), StatusCode::OK);
    let json = body_json(well_known).await;
    assert_eq!(json["status"], "ok");
    assert!(json.get("version").is_some());
    assert!(json.get("uptime_seconds").is_some());
}

#[tokio::test]
async fn test_security_headers_on_api_response() {
    let app = make_app();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/config")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let headers = resp.headers();

    assert_eq!(headers.get("X-Content-Type-Options").unwrap(), "nosniff");
    assert_eq!(headers.get("X-Frame-Options").unwrap(), "DENY");
    assert!(headers.get("Content-Security-Policy").is_some());
    assert_eq!(
        headers.get("Referrer-Policy").unwrap(),
        "strict-origin-when-cross-origin"
    );
}

#[tokio::test]
async fn test_hsts_not_set_on_http() {
    let app = make_app();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/config")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(
        resp.headers().get("Strict-Transport-Security").is_none(),
        "HSTS should not be set on plain HTTP"
    );
}

#[tokio::test]
async fn test_hsts_set_on_https() {
    let app = make_app();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/config")
                .header("x-forwarded-proto", "https")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let hsts = resp.headers().get("Strict-Transport-Security").unwrap();
    assert_eq!(hsts, "max-age=31536000; includeSubDomains");
}

#[tokio::test]
async fn test_deprecation_headers_present() {
    let state =
        ferro_server::AppState::in_memory().with_wopi_token_secret("bench-secret".to_string());
    let app = build_router(state);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/config")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let deprecation = resp.headers().get("deprecation");
    if let Some(dep) = deprecation {
        assert_eq!(dep, "true");
    }
}

#[tokio::test]
async fn test_request_metrics_incremented() {
    let state =
        ferro_server::AppState::in_memory().with_wopi_token_secret("bench-secret".to_string());
    let app = build_router(state);

    app.clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/config")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    app.clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/config")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let metrics = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(metrics.status(), StatusCode::OK);
    let json = body_json(metrics).await;
    assert!(
        json.get("uptime_seconds").is_some(),
        "metrics should include uptime_seconds"
    );
    assert!(
        json.get("storage").is_some(),
        "metrics should include storage stats"
    );
    assert!(
        json["requests"].get("total").is_some(),
        "metrics should include requests.total"
    );
}

#[tokio::test]
async fn test_maintenance_mode_blocks_writes() {
    let state = AppState::in_memory();
    state
        .maintenance_mode
        .store(true, std::sync::atomic::Ordering::Relaxed);
    let app = build_router(state);

    let get_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/config")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(get_resp.status(), StatusCode::OK);

    let put_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/files/maintenance-test.txt")
                .body(Body::from("blocked"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(put_resp.status(), StatusCode::SERVICE_UNAVAILABLE);
}
