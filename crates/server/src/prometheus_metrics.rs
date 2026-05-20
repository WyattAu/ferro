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
ferro_http_request_duration_seconds_sum 0
ferro_http_request_duration_seconds_count {request_count}
# HELP ferro_http_responses_total HTTP responses by status class
# TYPE ferro_http_responses_total counter
ferro_http_responses_total{{status_class="2xx"}} {status_2xx}
ferro_http_responses_total{{status_class="3xx"}} {status_3xx}
ferro_http_responses_total{{status_class="4xx"}} {status_4xx}
ferro_http_responses_total{{status_class="5xx"}} {status_5xx}
"#,
        uptime = uptime,
        file_count = file_count,
        total_bytes = total_bytes,
        request_count = request_count,
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
