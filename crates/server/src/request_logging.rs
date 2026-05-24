use axum::body::Body;
use axum::http::Request;
use axum::middleware::Next;
use axum::response::Response;
use std::sync::atomic::Ordering;
use std::time::Instant;

/// Middleware that logs each request with method, path, status, duration, and request ID.
/// Also records Prometheus-compatible histogram buckets and status-code counters,
/// and tracks storage operation counts by HTTP method.
pub async fn request_logging_middleware(
    request_count: std::sync::Arc<std::sync::atomic::AtomicU64>,
    duration_buckets: std::sync::Arc<[std::sync::atomic::AtomicU64; 11]>,
    duration_sum_ms: std::sync::Arc<std::sync::atomic::AtomicU64>,
    status_counts: std::sync::Arc<[std::sync::atomic::AtomicU64; 4]>,
    storage_op_counts: Option<std::sync::Arc<[std::sync::atomic::AtomicU64; 6]>>,
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

    // Record duration into histogram buckets (upper bounds in ms).
    let ms = duration.as_millis() as u64;
    let bucket_idx = match ms {
        0..=0 => 0,       // <1ms (bucket for 0ms)
        1..=4 => 1,       // <5ms
        5..=9 => 2,       // <10ms
        10..=24 => 3,     // <25ms
        25..=49 => 4,     // <50ms
        50..=99 => 5,     // <100ms
        100..=249 => 6,   // <250ms
        250..=499 => 7,   // <500ms
        500..=999 => 8,   // <1s
        1000..=4999 => 9, // <5s
        _ => 10,          // >=5s
    };
    duration_buckets[bucket_idx].fetch_add(1, Ordering::Relaxed);
    duration_sum_ms.fetch_add(ms, Ordering::Relaxed);

    // Record per-status-class counter.
    let status_idx = match status.as_u16() {
        200..=299 => 0,
        300..=399 => 1,
        400..=499 => 2,
        _ => 3,
    };
    status_counts[status_idx].fetch_add(1, Ordering::Relaxed);

    // Track storage operations: PUT=0, GET=1, DELETE=2, LIST(PROPFIND)=3, COPY=4, MOVE=5
    if let Some(ref ops) = storage_op_counts {
        let op_idx: Option<usize> = match method.as_str() {
            "PUT" => Some(0),
            "GET" | "HEAD" => Some(1),
            "DELETE" => Some(2),
            "PROPFIND" => Some(3),
            "COPY" => Some(4),
            "MOVE" => Some(5),
            _ => None,
        };
        if let Some(idx) = op_idx {
            ops[idx].fetch_add(1, Ordering::Relaxed);
        }
    }

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
