# Advanced Observability

**Document:** OpenTelemetry Tracing and Custom Metrics  
**Version:** 1.0.0  
**Status:** Active  
**Last Updated:** 2026-07-12  

---

## Overview

Full observability stack: distributed tracing, custom metrics, and intelligent alerting. Built on OpenTelemetry for vendor-neutral instrumentation.

---

## Architecture

```
Ferro Server
    |
    +---> OpenTelemetry SDK
              |
              +---> Traces (Jaeger/Tempo)
              +---> Metrics (Prometheus)
              +---> Logs (Loki)
```

---

## Distributed Tracing

### Configuration

```toml
# Cargo.toml
[dependencies]
opentelemetry = "0.24"
opentelemetry-otlp = "0.17"
opentelemetry_sdk = { version = "0.24", features = ["rt-tokio"] }
tracing-opentelemetry = "0.25"
```

### Initialization

```rust
use opentelemetry::trace::TracerProvider;
use opentelemetry_otlp::WithExportConfig;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{layer::SubscriberExt, Registry};

fn init_tracing(endpoint: &str) {
    let provider = opentelemetry_otlp::new_pipeline()
        .tonic()
        .with_endpoint(endpoint)
        .install_batch(opentelemetry_sdk::runtime::Tokio)
        .expect("Failed to create OTLP provider");
    
    let tracer = provider.tracer("ferro-server");
    
    let telemetry = OpenTelemetryLayer::new(tracer);
    
    let subscriber = Registry::default()
        .with(tracing_subscriber::fmt::layer().json())
        .with(telemetry);
    
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set subscriber");
}
```

### Span Propagation

```rust
// Automatic span creation for request handling
#[axum::debug_handler]
async fn handle_put(
    State(state): State<AppState>,
    Path(path): Path<String>,
    request: Request,
) -> Response {
    let span = tracing::info_span!("webdav_put", path = %path);
    let _guard = span.enter();
    
    // All log events within this scope include the span
    tracing::info!("Processing PUT request");
    
    // Child spans are automatically linked
    let result = process_upload(&state, &path, request).await;
    
    result
}
```

### Trace Context Propagation

```rust
// Propagate trace context across service boundaries
use opentelemetry::context::Propagation;

fn inject_headers(headers: &mut HeaderMap) {
    let propagator = opentelemetry_sdk::propagation::TraceContextPropagator::new();
    let context = opentelemetry::Context::current();
    propagator.inject_context(&context, headers);
}

fn extract_headers(headers: &HeaderMap) -> opentelemetry::Context {
    let propagator = opentelemetry_sdk::propagation::TraceContextPropagator::new();
    propagator.extract(headers)
}
```

---

## Custom Metrics

### Metric Types

| Metric | Type | Description | Labels |
|--------|------|-------------|--------|
| `ferro_http_requests_total` | Counter | Total HTTP requests | method, path, status |
| `ferro_http_request_duration_seconds` | Histogram | Request latency | method, path |
| `ferro_http_request_size_bytes` | Histogram | Request body size | method, path |
| `ferro_http_response_size_bytes` | Histogram | Response body size | method, path |
| `ferro_webdav_operations_total` | Counter | WebDAV operations | operation, status |
| `ferro_storage_operations_total` | Counter | Storage operations | backend, operation, status |
| `ferro_storage_latency_seconds` | Histogram | Storage operation latency | backend, operation |
| `ferro_auth_attempts_total` | Counter | Auth attempts | method, status |
| `ferro_cache_hits_total` | Counter | Cache hits | cache_name |
| `ferro_cache_misses_total` | Counter | Cache misses | cache_name |
| `ferro_active_connections` | Gauge | Active connections | - |
| `ferro_wasm_workers_active` | Gauge | Active WASM workers | - |
| `ferro_file_uploads_total` | Counter | File uploads | status |
| `ferro_file_downloads_total` | Counter | File downloads | status |
| `ferro_share_links_active` | Gauge | Active share links | - |

### Implementation

```rust
use prometheus::{Encoder, Histogram, HistogramOpts, IntCounter, IntGauge, Registry};
use std::time::Instant;

lazy_static::lazy_static! {
    static ref REGISTRY: Registry = Registry::new();
    
    static ref HTTP_REQUESTS_TOTAL: IntCounter = 
        IntCounter::with_opts(
            prometheus::opts!("ferro_http_requests_total", "Total HTTP requests")
                .variable_label("method")
                .variable_label("status")
        ).unwrap();
    
    static ref HTTP_REQUEST_DURATION: Histogram = 
        Histogram::with_opts(
            HistogramOpts::new("ferro_http_request_duration_seconds", "Request latency")
                .buckets(vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0])
        ).unwrap();
    
    static ref ACTIVE_CONNECTIONS: IntGauge = 
        IntGauge::new("ferro_active_connections", "Active connections").unwrap();
}

pub fn register_metrics() {
    REGISTRY.register(Box::new(HTTP_REQUESTS_TOTAL.clone())).unwrap();
    REGISTRY.register(Box::new(HTTP_REQUEST_DURATION.clone())).unwrap();
    REGISTRY.register(Box::new(ACTIVE_CONNECTIONS.clone())).unwrap();
}

pub fn record_request(method: &str, status: u16, duration: std::time::Duration) {
    HTTP_REQUESTS_TOTAL.with_label_values(&[method, &status.to_string()]).inc();
    HTTP_REQUEST_DURATION.observe(duration.as_secs_f64());
}

pub fn metrics_handler() -> impl IntoResponse {
    let encoder = prometheus::TextEncoder::new();
    let metric_families = REGISTRY.gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer).unwrap();
    (StatusCode::OK, [(header::CONTENT_TYPE, "text/plain; version=0.0.4")], buffer)
}
```

---

## Grafana Dashboards

### Dashboard: Ferro Overview

| Panel | Query | Type |
|-------|-------|------|
| Request Rate | `rate(ferro_http_requests_total[5m])` | Time series |
| Error Rate | `rate(ferro_http_requests_total{status=~"5.."}[5m]) / rate(ferro_http_requests_total[5m])` | Time series |
| p50/p95/p99 Latency | `histogram_quantile(0.5/0.95/0.99, rate(ferro_http_request_duration_seconds_bucket[5m]))` | Time series |
| Active Connections | `ferro_active_connections` | Gauge |
| Storage Operations | `rate(ferro_storage_operations_total[5m])` | Time series |
| Cache Hit Rate | `rate(ferro_cache_hits_total[5m]) / (rate(ferro_cache_hits_total[5m]) + rate(ferro_cache_misses_total[5m]))` | Gauge |

### Dashboard: Per-Tenant

| Panel | Query | Type |
|-------|-------|------|
| Requests by Tenant | `sum by (tenant_id) (rate(ferro_http_requests_total[5m]))` | Time series |
| Storage by Tenant | `ferro_storage_usage_bytes by (tenant_id)` | Bar chart |
| Error Rate by Tenant | `sum by (tenant_id) (rate(ferro_http_requests_total{status=~"5.."}[5m]))` | Time series |

### Dashboard: WebDAV Operations

| Panel | Query | Type |
|-------|-------|------|
| PROPFIND Rate | `rate(ferro_webdav_operations_total{operation="PROPFIND"}[5m])` | Time series |
| PUT Rate | `rate(ferro_webdav_operations_total{operation="PUT"}[5m])` | Time series |
| GET Rate | `rate(ferro_webdav_operations_total{operation="GET"}[5m])` | Time series |
| Lock Operations | `rate(ferro_webdav_operations_total{operation=~"LOCK|UNLOCK"}[5m])` | Time series |

---

## Alerting Rules

```yaml
# prometheus/alerts.yml
groups:
  - name: ferro-slo
    rules:
      - alert: ErrorBudgetLow
        expr: |
          (
            1 - (
              sum(rate(ferro_http_requests_total{status!~"5.."}[30d]))
              / sum(rate(ferro_http_requests_total[30d]))
            )
          ) > 0.001
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Error budget below 99.9% SLO"
          runbook_url: "https://github.com/WyattAu/ferro/blob/main/docs/sre/slo-definition.md"
      
      - alert: HighLatencyP99
        expr: |
          histogram_quantile(0.99, rate(ferro_http_request_duration_seconds_bucket[5m])) > 0.5
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "p99 latency > 500ms"
      
      - alert: HighErrorRate
        expr: |
          rate(ferro_http_requests_total{status=~"5.."}[5m])
          / rate(ferro_http_requests_total[5m]) > 0.01
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "Error rate > 1%"
      
      - alert: StorageSpaceLow
        expr: |
          (node_filesystem_avail_bytes{mountpoint="/data"} / node_filesystem_size_bytes{mountpoint="/data"}) < 0.2
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "Storage space < 20%"
```

---

## Anomaly Detection

### Automated Baselining

Use Prometheus recording rules to compute rolling baselines:

```yaml
groups:
  - name: ferro-baselines
    interval: 5m
    rules:
      - record: ferro:request_rate:baseline
        expr: |
          avg_over_time(rate(ferro_http_requests_total[5m])[7d:5m])
      
      - record: ferro:request_rate:deviation
        expr: |
          stddev_over_time(rate(ferro_http_requests_total[5m])[7d:5m])
      
      - record: ferro:latency_p99:baseline
        expr: |
          avg_over_time(histogram_quantile(0.99, rate(ferro_http_request_duration_seconds_bucket[5m]))[7d:5m])
```

### Anomaly Alerts

```yaml
- alert: RequestRateAnomaly
  expr: |
    abs(rate(ferro_http_requests_total[5m]) - ferro:request_rate:baseline) > 3 * ferro:request_rate:deviation
  for: 10m
  labels:
    severity: warning
  annotations:
    summary: "Request rate anomaly detected (3 sigma)"

- alert: LatencyAnomaly
  expr: |
    histogram_quantile(0.99, rate(ferro_http_request_duration_seconds_bucket[5m])) > 2 * ferro:latency_p99:baseline
  for: 10m
  labels:
    severity: warning
  annotations:
    summary: "p99 latency anomaly detected (2x baseline)"
```

---

## Implementation Roadmap

| Phase | Timeline | Components |
|-------|----------|------------|
| Phase 1 | Weeks 1-2 | OpenTelemetry SDK, basic tracing |
| Phase 2 | Weeks 3-4 | Custom metrics, Prometheus endpoint |
| Phase 3 | Weeks 5-6 | Grafana dashboards |
| Phase 4 | Weeks 7-8 | Alerting rules, anomaly detection |
| Phase 5 | Weeks 9-12 | Per-tenant metrics, advanced dashboards |
