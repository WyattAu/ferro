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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use axum::routing::get;
    use tower::ServiceExt;

    struct TestMetrics {
        request_count: std::sync::Arc<std::sync::atomic::AtomicU64>,
        duration_buckets: std::sync::Arc<[std::sync::atomic::AtomicU64; 11]>,
        duration_sum_ms: std::sync::Arc<std::sync::atomic::AtomicU64>,
        status_counts: std::sync::Arc<[std::sync::atomic::AtomicU64; 4]>,
        storage_op_counts: std::sync::Arc<[std::sync::atomic::AtomicU64; 6]>,
    }

    impl TestMetrics {
        fn new() -> Self {
            Self {
                request_count: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
                duration_buckets: std::sync::Arc::new(std::array::from_fn(|_| std::sync::atomic::AtomicU64::new(0))),
                duration_sum_ms: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
                status_counts: std::sync::Arc::new(std::array::from_fn(|_| std::sync::atomic::AtomicU64::new(0))),
                storage_op_counts: std::sync::Arc::new(std::array::from_fn(|_| std::sync::atomic::AtomicU64::new(0))),
            }
        }

        fn make_app_with_method(&self, _method: &str) -> axum::Router {
            let rc = self.request_count.clone();
            let db = self.duration_buckets.clone();
            let ds = self.duration_sum_ms.clone();
            let sc = self.status_counts.clone();
            let so = self.storage_op_counts.clone();

            axum::Router::new()
                .route("/test", get(|| async { axum::http::StatusCode::OK }))
                .layer(axum::middleware::from_fn(move |req, next| {
                    let rc = rc.clone();
                    let db = db.clone();
                    let ds = ds.clone();
                    let sc = sc.clone();
                    let so = so.clone();
                    async move { request_logging_middleware(rc, db, ds, sc, Some(so), req, next).await }
                }))
        }

        fn make_app(&self) -> axum::Router {
            self.make_app_with_method("GET")
        }

        fn request_count(&self) -> u64 {
            self.request_count.load(Ordering::Relaxed)
        }

        fn status_2xx(&self) -> u64 {
            self.status_counts[0].load(Ordering::Relaxed)
        }

        #[allow(dead_code)]
        fn status_4xx(&self) -> u64 {
            self.status_counts[2].load(Ordering::Relaxed)
        }

        #[allow(dead_code)]
        fn status_5xx(&self) -> u64 {
            self.status_counts[3].load(Ordering::Relaxed)
        }

        #[allow(dead_code)]
        fn storage_put(&self) -> u64 {
            self.storage_op_counts[0].load(Ordering::Relaxed)
        }

        fn storage_get(&self) -> u64 {
            self.storage_op_counts[1].load(Ordering::Relaxed)
        }

        #[allow(dead_code)]
        fn storage_delete(&self) -> u64 {
            self.storage_op_counts[2].load(Ordering::Relaxed)
        }
    }

    #[tokio::test]
    async fn test_request_logging_increments_counter() {
        let metrics = TestMetrics::new();
        let app = metrics.make_app();
        let resp = app
            .oneshot(Request::builder().uri("/test").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(metrics.request_count(), 1);
        assert_eq!(metrics.status_2xx(), 1);
        assert_eq!(metrics.storage_get(), 1);
    }

    #[tokio::test]
    async fn test_request_logging_extracts_client_ip() {
        let metrics = TestMetrics::new();
        let app = metrics.make_app();
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/test")
                    .header("x-forwarded-for", "10.0.0.1, 10.0.0.2")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(metrics.request_count(), 1);
    }

    #[tokio::test]
    async fn test_request_logging_without_forwarded_for() {
        let metrics = TestMetrics::new();
        let app = metrics.make_app();
        let resp = app
            .oneshot(Request::builder().uri("/test").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(metrics.request_count(), 1);
    }

    #[tokio::test]
    async fn test_request_logging_records_duration() {
        let metrics = TestMetrics::new();
        let app = metrics.make_app();
        let _ = app
            .oneshot(Request::builder().uri("/test").body(Body::empty()).unwrap())
            .await
            .unwrap();

        let _sum = metrics.duration_sum_ms.load(Ordering::Relaxed);
    }
}
