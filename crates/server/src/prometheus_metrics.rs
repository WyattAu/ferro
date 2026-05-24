use crate::AppState;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};

/// GET /metrics/prometheus — return server metrics in Prometheus format.
pub async fn prometheus_metrics_handler(State(state): State<AppState>) -> Response {
    use std::sync::atomic::Ordering;

    let uptime = state.started_at.elapsed().as_secs_f64();

    let mut file_count = 0u64;
    let mut total_bytes = 0u64;
    if let Ok(entries) = state.storage.list_all("/", 10000).await {
        for meta in &entries {
            if !meta.is_collection {
                file_count += 1;
                total_bytes += meta.size;
            }
        }
    }

    let request_count = state.request_count.load(Ordering::Relaxed);
    let request_duration_sum =
        state.request_duration_sum_ms.load(Ordering::Relaxed) as f64 / 1000.0;

    // Read histogram buckets.
    let buckets = &state.request_duration_buckets;
    let le_001ms = buckets[0].load(Ordering::Relaxed);
    let le_005ms = buckets[1].load(Ordering::Relaxed);
    let le_010ms = buckets[2].load(Ordering::Relaxed);
    let le_025ms = buckets[3].load(Ordering::Relaxed);
    let le_050ms = buckets[4].load(Ordering::Relaxed);
    let le_100ms = buckets[5].load(Ordering::Relaxed);
    let le_250ms = buckets[6].load(Ordering::Relaxed);
    let le_500ms = buckets[7].load(Ordering::Relaxed);
    let le_1s = buckets[8].load(Ordering::Relaxed);
    let le_5s = buckets[9].load(Ordering::Relaxed);
    let le_inf = buckets[10].load(Ordering::Relaxed);

    // Read per-status-class counters.
    let statuses = &state.request_status_counts;
    let status_2xx = statuses[0].load(Ordering::Relaxed);
    let status_3xx = statuses[1].load(Ordering::Relaxed);
    let status_4xx = statuses[2].load(Ordering::Relaxed);
    let status_5xx = statuses[3].load(Ordering::Relaxed);

    // Read actual WASM worker count (0 if not configured)
    let wasm_workers = match &state.wasm_runtime {
        Some(rt) => rt.worker_count().await,
        None => 0,
    };

    // Read storage operation counters
    let storage_ops = &state.storage_op_counts;
    let storage_puts = storage_ops[0].load(Ordering::Relaxed);
    let storage_gets = storage_ops[1].load(Ordering::Relaxed);
    let storage_deletes = storage_ops[2].load(Ordering::Relaxed);
    let storage_lists = storage_ops[3].load(Ordering::Relaxed);
    let storage_copies = storage_ops[4].load(Ordering::Relaxed);
    let storage_moves = storage_ops[5].load(Ordering::Relaxed);

    // Read cache stats
    let cache_stats = state.read_cache.stats();
    let cache_hits = cache_stats.hits;
    let cache_misses = cache_stats.misses;
    let cache_evictions = cache_stats.evictions;

    // Read WASM worker metrics
    let wasm_dispatches = state.wasm_dispatch_count.load(Ordering::Relaxed);
    let wasm_errors = state.wasm_error_count.load(Ordering::Relaxed);
    let wasm_fuel = state.wasm_fuel_total.load(Ordering::Relaxed);

    let mut headers = HeaderMap::new();
    headers.insert(
        "Content-Type",
        "text/plain; version=0.0.4; charset=utf-8"
            .parse()
            .expect("static MIME type must parse"),
    );

    let output = format!(
        r#"# HELP ferro_uptime_seconds Server uptime in seconds
# TYPE ferro_uptime_seconds gauge
ferro_uptime_seconds {uptime}
# HELP ferro_files_total Total number of files
# TYPE ferro_files_total gauge
ferro_files_total {file_count}
# HELP ferro_storage_bytes_total Total storage used in bytes
# TYPE ferro_storage_bytes_total gauge
ferro_storage_bytes_total {total_bytes}
# HELP ferro_wasm_workers_loaded Number of loaded WASM workers
# TYPE ferro_wasm_workers_loaded gauge
ferro_wasm_workers_loaded {wasm_workers}
# HELP ferro_http_requests_total Total HTTP requests
# TYPE ferro_http_requests_total counter
ferro_http_requests_total {request_count}
# HELP ferro_http_request_duration_seconds Request duration histogram
# TYPE ferro_http_request_duration_seconds histogram
ferro_http_request_duration_seconds_bucket{{le="0.001"}} {le_001ms}
ferro_http_request_duration_seconds_bucket{{le="0.005"}} {le_005ms}
ferro_http_request_duration_seconds_bucket{{le="0.010"}} {le_010ms}
ferro_http_request_duration_seconds_bucket{{le="0.025"}} {le_025ms}
ferro_http_request_duration_seconds_bucket{{le="0.050"}} {le_050ms}
ferro_http_request_duration_seconds_bucket{{le="0.100"}} {le_100ms}
ferro_http_request_duration_seconds_bucket{{le="0.250"}} {le_250ms}
ferro_http_request_duration_seconds_bucket{{le="0.500"}} {le_500ms}
ferro_http_request_duration_seconds_bucket{{le="1.0"}} {le_1s}
ferro_http_request_duration_seconds_bucket{{le="5.0"}} {le_5s}
ferro_http_request_duration_seconds_bucket{{le="+Inf"}} {le_inf}
ferro_http_request_duration_seconds_sum {request_duration_sum}
ferro_http_request_duration_seconds_count {request_count}
# HELP ferro_http_responses_total HTTP responses by status class
# TYPE ferro_http_responses_total counter
ferro_http_responses_total{{status_class="2xx"}} {status_2xx}
ferro_http_responses_total{{status_class="3xx"}} {status_3xx}
ferro_http_responses_total{{status_class="4xx"}} {status_4xx}
ferro_http_responses_total{{status_class="5xx"}} {status_5xx}
# HELP ferro_storage_operations_total Storage operations by type
# TYPE ferro_storage_operations_total counter
ferro_storage_operations_total{{operation="put"}} {storage_puts}
ferro_storage_operations_total{{operation="get"}} {storage_gets}
ferro_storage_operations_total{{operation="delete"}} {storage_deletes}
ferro_storage_operations_total{{operation="list"}} {storage_lists}
ferro_storage_operations_total{{operation="copy"}} {storage_copies}
ferro_storage_operations_total{{operation="move"}} {storage_moves}
# HELP ferro_read_cache_hits_total Read cache hit count
# TYPE ferro_read_cache_hits_total counter
ferro_read_cache_hits_total {cache_hits}
# HELP ferro_read_cache_misses_total Read cache miss count
# TYPE ferro_read_cache_misses_total counter
ferro_read_cache_misses_total {cache_misses}
# HELP ferro_read_cache_evictions_total Read cache eviction count
# TYPE ferro_read_cache_evictions_total counter
ferro_read_cache_evictions_total {cache_evictions}
# HELP ferro_wasm_dispatch_total Total WASM worker dispatches
# TYPE ferro_wasm_dispatch_total counter
ferro_wasm_dispatch_total {wasm_dispatches}
# HELP ferro_wasm_errors_total Total WASM worker execution errors
# TYPE ferro_wasm_errors_total counter
ferro_wasm_errors_total {wasm_errors}
# HELP ferro_wasm_fuel_consumed_total Total fuel consumed by WASM workers
# TYPE ferro_wasm_fuel_consumed_total counter
ferro_wasm_fuel_consumed_total {wasm_fuel}
"#,
        uptime = uptime,
        file_count = file_count,
        total_bytes = total_bytes,
        request_count = request_count,
        request_duration_sum = request_duration_sum,
    );

    (StatusCode::OK, headers, output).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AppState;
    use crate::build_router;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    async fn body_text(response: axum::response::Response) -> String {
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    #[tokio::test]
    async fn test_prometheus_endpoint_returns_text_plain() {
        let app = build_router(AppState::in_memory());
        let resp = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/metrics/prometheus")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let ct = resp
            .headers()
            .get("Content-Type")
            .unwrap()
            .to_str()
            .unwrap();
        assert!(ct.starts_with("text/plain"));
    }

    #[tokio::test]
    async fn test_prometheus_output_contains_required_metrics() {
        let app = build_router(AppState::in_memory());
        let resp = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/metrics/prometheus")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let text = body_text(resp).await;
        assert!(text.contains("# HELP ferro_uptime_seconds"));
        assert!(text.contains("# TYPE ferro_uptime_seconds gauge"));
        assert!(text.contains("ferro_uptime_seconds "));
        assert!(text.contains("# HELP ferro_files_total"));
        assert!(text.contains("# TYPE ferro_files_total gauge"));
        assert!(text.contains("ferro_files_total "));
        assert!(text.contains("# HELP ferro_storage_bytes_total"));
        assert!(text.contains("# TYPE ferro_storage_bytes_total gauge"));
        assert!(text.contains("ferro_storage_bytes_total "));
        assert!(text.contains("# HELP ferro_http_requests_total"));
        assert!(text.contains("# TYPE ferro_http_requests_total counter"));
        assert!(text.contains("ferro_http_requests_total "));
    }

    #[tokio::test]
    async fn test_prometheus_shows_file_count_after_upload() {
        let app = build_router(AppState::in_memory());

        app.clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("PUT")
                    .uri("/prom-test.txt")
                    .body(axum::body::Body::from("data"))
                    .unwrap(),
            )
            .await
            .unwrap();

        let resp = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/metrics/prometheus")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let text = body_text(resp).await;
        assert!(text.contains("ferro_files_total 1"));
    }
}
