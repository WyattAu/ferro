use crate::AppState;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};

/// GET /metrics/prometheus — return server metrics in Prometheus format.
pub async fn prometheus_metrics_handler(State(state): State<AppState>) -> Response {
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

    let request_count = state
        .request_count
        .load(std::sync::atomic::Ordering::Relaxed);

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
ferro_wasm_workers_loaded 0
# HELP ferro_http_requests_total Total HTTP requests
# TYPE ferro_http_requests_total counter
ferro_http_requests_total {request_count}
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
