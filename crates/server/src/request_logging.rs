use axum::body::Body;
use axum::http::Request;
use axum::middleware::Next;
use axum::response::Response;
use std::sync::atomic::Ordering;
use std::time::Instant;

/// Middleware that logs each request with method, path, status, duration, and request ID.
pub async fn request_logging_middleware(
    request_count: std::sync::Arc<std::sync::atomic::AtomicU64>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let start = Instant::now();
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let client_ip = req
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "-".to_string());

    let response = next.run(req).await;

    request_count.fetch_add(1, Ordering::Relaxed);

    let duration = start.elapsed();
    let status = response.status();

    let request_id = response
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-");

    tracing::info!(
        request_id = %request_id,
        method = %method,
        path = %path,
        status = status.as_u16(),
        duration_ms = duration.as_millis() as u64,
        client_ip = %client_ip,
        "request"
    );

    response
}
