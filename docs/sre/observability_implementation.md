# Ferro Advanced Observability Implementation Guide

> **Version:** 1.0  
> **Author:** SRE Team  
> **Created:** 2026-07-12  
> **Status:** Proposed  
> **Stack:** OpenTelemetry + Prometheus + Grafana + Loki + Alertmanager

---

## Executive Summary

This guide provides step-by-step integration of OpenTelemetry-based observability into Ferro's Rust server. It covers metrics, traces, and logs with concrete code snippets, Prometheus configuration, Grafana dashboards, and alert rules. The current codebase has a custom `MetricsRegistry` with manual Prometheus formatting (`crates/server/src/prometheus_metrics.rs`) — this plan migrates to the `opentelemetry` crate ecosystem while preserving backward compatibility.

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────┐
│                      Ferro Server                       │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐              │
│  │ Metrics  │  │  Traces  │  │   Logs   │              │
│  │ (OTel)   │  │ (OTel)   │  │ (tracing)│              │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘              │
│       │              │              │                    │
│       └──────────────┼──────────────┘                   │
│                      ▼                                  │
│            OTel SDK (OTLP gRPC)                         │
└──────────────────┬──────────────────────────────────────┘
                   │ :4317
                   ▼
         ┌─────────────────┐
         │  OTel Collector │
         └────────┬────────┘
                  │
      ┌───────────┼───────────┐
      ▼           ▼           ▼
┌──────────┐ ┌─────────┐ ┌────────┐
│Prometheus│ │  Loki   │ │Tempo   │
│ (metrics)│ │ (logs)  │ │(traces)│
└────┬─────┘ └────┬────┘ └───┬────┘
     │            │           │
     └────────────┼───────────┘
                  ▼
           ┌───────────┐
           │  Grafana   │
           └───────────┘
```

---

## Step 1: OpenTelemetry SDK Setup

### 1.1 Add Dependencies

**File:** `crates/observability/Cargo.toml`

```toml
[package]
name = "ferro-observability"
version.workspace = true
edition.workspace = true
description = "Observability for Ferro (OpenTelemetry, Prometheus, Loki)"
license.workspace = true

[dependencies]
axum = { workspace = true, optional = true }
serde = { workspace = true }
serde_json = { workspace = true }
chrono = { version = "0.4", features = ["serde"] }
parking_lot = "0.12"

# OpenTelemetry dependencies
opentelemetry = { version = "0.29", features = ["metrics", "trace"] }
opentelemetry_sdk = { version = "0.29", features = ["rt-tokio", "metrics", "trace"] }
opentelemetry-otlp = { version = "0.29", features = ["grpc-tonic", "trace", "metrics"] }
opentelemetry-stdout = { version = "0.5", features = ["trace", "metrics"] }
opentelemetry-prometheus = "0.22"
opentelemetry-semantic-conventions = "0.27"

# Tracing integration
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json", "registry"] }
tracing-opentelemetry = "0.30"
tracing-loki = { version = "0.3", optional = true }

# Prometheus exporter
prometheus = "0.13"

[features]
default = ["endpoints", "prometheus-exporter"]
endpoints = ["axum"]
loki = ["tracing-loki"]
prometheus-exporter = ["dep:prometheus"]
otlp = ["dep:opentelemetry-otlp"]
```

**File:** `crates/server/Cargo.toml` — update otel feature:

```toml
[features]
otel = [
    "dep:opentelemetry",
    "dep:opentelemetry-otlp",
    "dep:opentelemetry_sdk",
    "dep:tracing-opentelemetry",
    "ferro-observability/otlp",
]
```

### 1.2 Initialize OpenTelemetry Provider

**New File:** `crates/observability/src/otel_init.rs`

```rust
use opentelemetry::global;
use opentelemetry::metrics::{Meter, MeterProvider};
use opentelemetry::trace::TracerProvider;
use opentelemetry_otlp::{ExportConfig, Protocol, TonicExporterBuilder, WithExportConfig};
use opentelemetry_sdk::metrics::SdkMeterProvider;
use opentelemetry_sdk::runtime;
use opentelemetry_sdk::trace::SdkTracerProvider;
use std::time::Duration;

pub struct ObservabilityConfig {
    pub otlp_endpoint: String,
    pub service_name: String,
    pub service_version: String,
    pub meter_name: String,
    pub trace_sample_ratio: f64,
    pub metric_export_interval: Duration,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            otlp_endpoint: "http://localhost:4317".to_string(),
            service_name: "ferro-server".to_string(),
            service_version: env!("CARGO_PKG_VERSION").to_string(),
            meter_name: "ferro".to_string(),
            trace_sample_ratio: 1.0,
            metric_export_interval: Duration::from_secs(15),
        }
    }
}

pub fn init_tracing_provider(config: &ObservabilityConfig) -> TracerProvider {
    let exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint(&config.otlp_endpoint)
        .with_protocol(Protocol::Grpc)
        .with_timeout(Duration::from_secs(5));

    let trace_config = opentelemetry_sdk::trace::Config::default()
        .with_resource(opentelemetry_sdk::Resource::builder()
            .with_service_name(&config.service_name)
            .with_attribute(opentelemetry::KeyValue::new(
                "service.version",
                config.service_version.clone(),
            ))
            .build())
        .with_sampler(opentelemetry_sdk::trace::Sampler::ParentBased(
            opentelemetry_sdk::trace::Sampler::TraceIdRatioBased(config.trace_sample_ratio),
        ));

    let provider = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(exporter)
        .with_trace_config(trace_config)
        .install_batch(runtime::Tokio)
        .expect("Failed to install OTLP trace pipeline");

    global::set_tracer_provider(provider.clone());
    provider
}

pub fn init_metrics_provider(config: &ObservabilityConfig) -> SdkMeterProvider {
    let exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint(&config.otlp_endpoint)
        .with_protocol(Protocol::Grpc)
        .with_timeout(Duration::from_secs(5));

    let resource = opentelemetry_sdk::Resource::builder()
        .with_service_name(&config.service_name)
        .with_attribute(opentelemetry::KeyValue::new(
            "service.version",
            config.service_version.clone(),
        ))
        .build();

    let provider = opentelemetry_otlp::new_pipeline()
        .metrics(opentelemetry_sdk::runtime::Tokio)
        .with_exporter(exporter)
        .with_resource(resource)
        .with_period(config.metric_export_interval)
        .build()
        .expect("Failed to install OTLP metrics pipeline");

    global::set_meter_provider(provider.clone());
    provider
}
```

### 1.3 Wire into Server Startup

**File:** `crates/server/src/startup.rs` — add at top of `main()`:

```rust
#[cfg(feature = "otel")]
{
    use ferro_observability::otel_init::{ObservabilityConfig, init_tracing_provider, init_metrics_provider};
    let otel_config = ObservabilityConfig::default();
    let _tracer_provider = init_tracing_provider(&otel_config);
    let _meter_provider = init_metrics_provider(&otel_config);
    tracing::info!("OpenTelemetry initialized (OTLP endpoint: {})", otel_config.otlp_endpoint);
}
```

---

## Step 2: Metrics Integration

### 2.1 Define Metric Instruments

**New File:** `crates/observability/src/instruments.rs`

```rust
use opentelemetry::global;
use opentelemetry::metrics::{Counter, Histogram, Meter, UpDownCounter};
use std::sync::OnceLock;

pub struct FerroMetrics {
    // HTTP metrics
    pub http_requests_total: Counter<u64>,
    pub http_request_duration: Histogram<f64>,
    pub http_request_body_size: Histogram<u64>,
    pub http_responses_total: Counter<u64>,

    // Storage metrics
    pub storage_operations_total: Counter<u64>,
    pub storage_operation_duration: Histogram<f64>,
    pub storage_bytes_total: Counter<u64>,

    // Cache metrics
    pub cache_hits_total: Counter<u64>,
    pub cache_misses_total: Counter<u64>,
    pub cache_evictions_total: Counter<u64>,
    pub cache_size: UpDownCounter<i64>,

    // WASM metrics
    pub wasm_dispatches_total: Counter<u64>,
    pub wasm_errors_total: Counter<u64>,
    pub wasm_fuel_consumed: Counter<u64>,
    pub wasm_worker_count: UpDownCounter<i64>,

    // Database metrics
    pub db_queries_total: Counter<u64>,
    pub db_query_duration: Histogram<f64>,
    pub db_connections_active: UpDownCounter<i64>,

    // Sync metrics
    pub sync_operations_total: Counter<u64>,
    pub sync_clock_drift: Histogram<f64>,

    // System metrics
    pub server_uptime: opentelemetry::metrics::Gauge<f64>,
    pub server_file_count: opentelemetry::metrics::Gauge<u64>,
    pub server_storage_bytes: opentelemetry::metrics::Gauge<u64>,
}

static METRICS: OnceLock<FerroMetrics> = OnceLock::new();

pub fn init_metrics() -> &'static FerroMetrics {
    METRICS.get_or_init(|| {
        let meter = global::meter("ferro");

        FerroMetrics {
            http_requests_total: meter.u64_counter("ferro.http.requests.total")
                .with_description("Total HTTP requests")
                .build(),
            http_request_duration: meter.f64_histogram("ferro.http.request.duration")
                .with_description("HTTP request duration in seconds")
                .with_unit("s")
                .build(),
            http_request_body_size: meter.u64_histogram("ferro.http.request.body.size")
                .with_description("HTTP request body size in bytes")
                .with_unit("By")
                .build(),
            http_responses_total: meter.u64_counter("ferro.http.responses.total")
                .with_description("Total HTTP responses by status code")
                .build(),
            storage_operations_total: meter.u64_counter("ferro.storage.operations.total")
                .with_description("Storage operations by type")
                .build(),
            storage_operation_duration: meter.f64_histogram("ferro.storage.operation.duration")
                .with_description("Storage operation duration in seconds")
                .with_unit("s")
                .build(),
            storage_bytes_total: meter.u64_counter("ferro.storage.bytes.total")
                .with_description("Total bytes stored")
                .build(),
            cache_hits_total: meter.u64_counter("ferro.cache.hits.total")
                .with_description("Cache hit count")
                .build(),
            cache_misses_total: meter.u64_counter("ferro.cache.misses.total")
                .with_description("Cache miss count")
                .build(),
            cache_evictions_total: meter.u64_counter("ferro.cache.evictions.total")
                .with_description("Cache eviction count")
                .build(),
            cache_size: meter.i64_up_down_counter("ferro.cache.size")
                .with_description("Current cache entry count")
                .build(),
            wasm_dispatches_total: meter.u64_counter("ferro.wasm.dispatches.total")
                .with_description("WASM worker dispatches")
                .build(),
            wasm_errors_total: meter.u64_counter("ferro.wasm.errors.total")
                .with_description("WASM worker errors")
                .build(),
            wasm_fuel_consumed: meter.u64_counter("ferro.wasm.fuel.consumed")
                .with_description("WASM fuel consumed")
                .build(),
            wasm_worker_count: meter.i64_up_down_counter("ferro.wasm.workers")
                .with_description("Active WASM workers")
                .build(),
            db_queries_total: meter.u64_counter("ferro.db.queries.total")
                .with_description("Database queries")
                .build(),
            db_query_duration: meter.f64_histogram("ferro.db.query.duration")
                .with_description("Database query duration")
                .with_unit("s")
                .build(),
            db_connections_active: meter.i64_up_down_counter("ferro.db.connections.active")
                .with_description("Active DB connections")
                .build(),
            sync_operations_total: meter.u64_counter("ferro.sync.operations.total")
                .with_description("Sync operations")
                .build(),
            sync_clock_drift: meter.f64_histogram("ferro.sync.clock.drift")
                .with_description("Sync clock drift in milliseconds")
                .with_unit("ms")
                .build(),
            server_uptime: meter.f64_gauge("ferro.server.uptime")
                .with_description("Server uptime in seconds")
                .with_unit("s")
                .build(),
            server_file_count: meter.u64_gauge("ferro.server.files")
                .with_description("Total file count")
                .build(),
            server_storage_bytes: meter.u64_gauge("ferro.server.storage.bytes")
                .with_description("Total storage bytes")
                .build(),
        }
    })
}
```

### 2.2 Instrument HTTP Handlers

**File:** `crates/server/src/request_logging.rs` — enhance with OTel metrics:

```rust
use axum::extract::{MatchedRequestParts, Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::Response;
use std::time::Instant;

pub async fn otel_metrics_middleware(
    State(_state): State<crate::AppState>,
    request: Request,
    next: Next,
) -> Response {
    let start = Instant::now();
    let method = request.method().to_string();
    let uri = request.uri().path().to_string();

    let response = next.run(request).await;
    let duration = start.elapsed().as_secs_f64();
    let status = response.status().as_u16();

    let metrics = crate::otel_init::init_metrics();

    // Record request count
    metrics.http_requests_total.add(1, &[
        opentelemetry::KeyValue::new("method", method.clone()),
        opentelemetry::KeyValue::new("status", status as i64),
    ]);

    // Record duration
    metrics.http_request_duration.record(duration, &[
        opentelemetry::KeyValue::new("method", method),
        opentelemetry::KeyValue::new("status", status as i64),
    ]);

    // Record response
    metrics.http_responses_total.add(1, &[
        opentelemetry::KeyValue::new("status", status as i64),
    ]);

    response
}
```

### 2.3 Instrument Storage Operations

**File:** `crates/server/src/storage.rs` — add timing to storage calls:

```rust
use opentelemetry::global;
use opentelemetry::trace::{Span, TraceContextExt, Tracer};

pub async fn instrumented_put(
    storage: &Arc<dyn StorageEngine>,
    path: &str,
    body: Bytes,
    owner: &str,
) -> Result<FileMetadata, StorageError> {
    let metrics = crate::otel_init::init_metrics();
    let start = std::time::Instant::now();

    let result = storage.put(path, body.clone(), owner).await;

    let duration = start.elapsed().as_secs_f64();
    let op_type = "put";

    metrics.storage_operations_total.add(1, &[
        opentelemetry::KeyValue::new("operation", op_type),
        opentelemetry::KeyValue::new("success", result.is_ok()),
    ]);

    metrics.storage_operation_duration.record(duration, &[
        opentelemetry::KeyValue::new("operation", op_type),
    ]);

    if let Ok(_) = &result {
        metrics.storage_bytes_total.add(body.len() as u64, &[
            opentelemetry::KeyValue::new("operation", op_type),
        ]);
    }

    result
}

pub async fn instrumented_get(
    storage: &Arc<dyn StorageEngine>,
    path: &str,
) -> Result<Bytes, StorageError> {
    let metrics = crate::otel_init::init_metrics();
    let start = std::time::Instant::now();

    let result = storage.get(path).await;

    let duration = start.elapsed().as_secs_f64();

    metrics.storage_operations_total.add(1, &[
        opentelemetry::KeyValue::new("operation", "get"),
        opentelemetry::KeyValue::new("success", result.is_ok()),
    ]);

    metrics.storage_operation_duration.record(duration, &[
        opentelemetry::KeyValue::new("operation", "get"),
    ]);

    result
}
```

### 2.4 Instrument Cache

**File:** `crates/cache/src/cache.rs` — add metrics hooks:

```rust
// In CacheStore impl for TimedCache
fn get(&self, key: &K) -> Option<V> {
    let metrics = ferro_observability::instruments::init_metrics();

    let mut entry = match self.entries.get_mut(key) {
        Some(e) => e,
        None => {
            self.stats.record_miss();
            metrics.cache_misses_total.add(1, &[]);
            return None;
        }
    };

    if entry.is_expired() {
        drop(entry);
        if let Some((_, removed)) = self.entries.remove(key) {
            self.stats.sub_size(removed.size_bytes);
            if let Some(ref lru) = self.lru {
                lru.record_remove(key);
            }
        }
        self.stats.record_miss();
        metrics.cache_misses_total.add(1, &[]);
        return None;
    }

    entry.touch();
    if let Some(ref lru) = self.lru {
        lru.record_access(key);
    }
    self.stats.record_hit();
    metrics.cache_hits_total.add(1, &[]);
    Some(entry.value.clone())
}
```

### 2.5 Instrument WASM Runtime

**File:** `crates/core/src/wasm.rs` — add dispatch metrics:

```rust
pub async fn dispatch_with_metrics(
    &self,
    worker_path: &str,
    input: &[u8],
) -> Result<Vec<u8>, WasmError> {
    let metrics = ferro_observability::instruments::init_metrics();
    let start = std::time::Instant::now();

    let result = self.dispatch(worker_path, input).await;

    let duration = start.elapsed().as_secs_f64();

    metrics.wasm_dispatches_total.add(1, &[
        opentelemetry::KeyValue::new("worker", worker_path.to_string()),
        opentelemetry::KeyValue::new("success", result.is_ok()),
    ]);

    if let Err(ref e) = result {
        metrics.wasm_errors_total.add(1, &[
            opentelemetry::KeyValue::new("worker", worker_path.to_string()),
            opentelemetry::KeyValue::new("error_type", format!("{:?}", e)),
        ]);
    }

    result
}
```

### 2.6 Instrument Database Queries

**File:** `crates/server/src/db.rs` — add query timing:

```rust
pub async fn instrumented_query(
    pool: &SqlitePool,
    query: &str,
) -> Result<Vec<SqliteRow>, sqlx::Error> {
    let metrics = ferro_observability::instruments::init_metrics();
    let start = std::time::Instant::now();

    metrics.db_connections_active.add(1, &[]);

    let result = sqlx::query(query).fetch_all(pool).await;

    metrics.db_connections_active.add(-1, &[]);

    let duration = start.elapsed().as_secs_f64();

    metrics.db_queries_total.add(1, &[
        opentelemetry::KeyValue::new("success", result.is_ok()),
    ]);

    metrics.db_query_duration.record(duration, &[
        opentelemetry::KeyValue::new("success", result.is_ok()),
    ]);

    result
}
```

---

## Step 3: Distributed Tracing Integration

### 3.1 Tracing Subscriber Setup

**File:** `crates/server/src/startup.rs` — configure tracing subscriber:

```rust
pub fn init_tracing() {
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::EnvFilter;

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,ferro_server=debug"));

    let otel_layer = tracing_opentelemetry::layer()
        .with_tracer(global::tracer("ferro"))
        .with_filter(filter);

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_thread_ids(true);

    tracing_subscriber::registry()
        .with(otel_layer)
        .with(fmt_layer)
        .init();
}
```

### 3.2 Add Tracing Spans to Critical Paths

**File:** `crates/server/src/handlers.rs` — add spans:

```rust
#[tracing::instrument(
    name = "webdav_put",
    skip(state, body),
    fields(
        path = %path,
        content_length = body.len(),
        owner = tracing::field::Empty,
    )
)]
pub async fn handle_put(
    State(state): State<AppState>,
    Path(path): Path<String>,
    body: Bytes,
) -> Result<impl IntoResponse, ApiError> {
    tracing::Span::current().record("owner", &tracing::field::display("anonymous"));

    // ... handler logic
}

#[tracing::instrument(
    name = "webdav_get",
    skip(state),
    fields(path = %path)
)]
pub async fn handle_get(
    State(state): State<AppState>,
    Path(path): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    // ... handler logic
}
```

### 3.3 Propagate Trace Context in Storage Operations

**File:** `crates/core/src/storage.rs` — propagate context:

```rust
pub async fn get_with_context(
    &self,
    path: &str,
    cx: &opentelemetry::trace::Context,
) -> Result<Bytes, StorageError> {
    let mut span = global::tracer("ferro.storage").start_with_context("storage.get", cx);
    span.set_attribute(opentelemetry::KeyValue::new("storage.path", path.to_string()));

    let result = self.get(path).await;

    if let Err(ref e) = result {
        span.set_status(opentelemetry::trace::Status::error(format!("{}", e)));
    }

    span.end();
    result
}
```

### 3.4 Create Span for WebSocket Connections

**File:** `crates/server/src/ws.rs` — instrument WebSocket:

```rust
pub async fn ws_handler(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
) -> Response {
    let span = global::tracer("ferro").start("websocket.connection");
    let _guard = tracing_opentelemetry::set_span_from_context(span.context());

    ws.on_upgrade(|socket| handle_socket(socket, state))
}
```

---

## Step 4: Structured Logging with Loki

### 4.1 Configure Loki Exporter

**File:** `crates/observability/src/loki.rs` (enhance existing):

```rust
use tracing_loki::Layer;
use tracing_subscriber::layer::SubscriberExt;

pub fn init_loki subscriber(endpoint: &str) -> impl SubscriberExt {
    let (loki_layer, task) = Layer::builder()
        .http_client(reqwest::Client::new())
        .endpoint(endpoint)
        .label("service", "ferro-server")
        .label("host", hostname::get().unwrap().to_string_lossy().to_string())
        .build()
        .expect("Failed to create Loki layer");

    tokio::spawn(task);
    loki_layer
}
```

### 4.2 Structured Log Format

**File:** `crates/server/src/json_logging.rs` — add OTel context:

```rust
pub fn format_with_otel_context(
    event: &tracing::Event,
    fields: &BTreeMap<String, serde_json::Value>,
) -> serde_json::Value {
    let mut json = serde_json::Map::new();

    // Add standard fields
    json.insert("timestamp".into(), serde_json::to_value(chrono::Utc::now()).unwrap());
    json.insert("level".into(), serde_json::Value::String(format!("{}", event.metadata().level())));

    // Add OTel context if available
    if let Some(cx) = opentelemetry::Context::current().span_context() {
        json.insert("trace_id".into(), serde_json::Value::String(cx.trace_id().to_string()));
        json.insert("span_id".into(), serde_json::Value::String(cx.span_id().to_string()));
    }

    // Add custom fields
    for (key, value) in fields {
        json.insert(key.clone(), value.clone());
    }

    serde_json::Value::Object(json)
}
```

### 4.3 Log Levels by Component

| Component | Default Level | Debug Level |
|-----------|--------------|-------------|
| HTTP handlers | info | debug |
| Storage operations | info | trace |
| Cache | warn | debug |
| WASM runtime | info | debug |
| Database | warn | info |
| Auth | info | debug |
| Federation | info | debug |

---

## Step 5: Prometheus Configuration

### 5.1 Updated Prometheus Config

**File:** `monitoring/prometheus/prometheus.yml`

```yaml
global:
  scrape_interval: 15s
  evaluation_interval: 15s
  scrape_timeout: 10s

  # External labels for federation
  external_labels:
    cluster: 'ferro-production'
    environment: 'production'

rule_files:
  - "alerts.yml"
  - "recording_rules.yml"

alerting:
  alertmanagers:
    - static_configs:
        - targets:
          - alertmanager:9093

scrape_configs:
  # Ferro server metrics (OTel exporter)
  - job_name: 'ferro-server'
    static_configs:
      - targets: ['ferro-server:8080']
    metrics_path: '/metrics'
    scrape_interval: 10s
    scrape_timeout: 5s
    metric_relabel_configs:
      # Drop high-cardinality metrics
      - source_labels: [__name__]
        regex: 'ferro_http_request_duration_seconds_bucket\{le=".*"\}'
        action: keep
      # Add instance label
      - source_labels: [__address__]
        target_label: instance
        regex: '(.+):\d+'
        replacement: '$1'

  # OTel Collector metrics
  - job_name: 'otel-collector'
    static_configs:
      - targets: ['otel-collector:8888']
    scrape_interval: 10s

  # Prometheus self-monitoring
  - job_name: 'prometheus'
    static_configs:
      - targets: ['localhost:9090']

  # Kubernetes pod scraping
  - job_name: 'kubernetes-pods'
    kubernetes_sd_configs:
      - role: pod
    relabel_configs:
      - source_labels: [__meta_kubernetes_pod_annotation_prometheus_io_scrape]
        action: keep
        regex: true
      - source_labels: [__meta_kubernetes_pod_annotation_prometheus_io_path]
        action: replace
        target_label: __metrics_path__
        regex: (.+)
      - source_labels: [__address__, __meta_kubernetes_pod_annotation_prometheus_io_port]
        action: replace
        target_label: __address__
        regex: ([^:]+)(?::\d+)?;(\d+)
        replacement: $1:$2

  # Service discovery for federated Ferro instances
  - job_name: 'ferro-federation'
    consul_sd_configs:
      - server: 'consul:8500'
        services:
          - 'ferro-server'
    relabel_configs:
      - source_labels: [__meta_consul_service_metadata_prometheus_port]
        target_label: __address__
```

### 5.2 Recording Rules

**New File:** `monitoring/prometheus/recording_rules.yml`

```yaml
groups:
  - name: ferro_recording_rules
    interval: 15s
    rules:
      # Request rate (per second)
      - record: ferro:http_requests:rate5m
        expr: rate(ferro_http_requests_total[5m])

      # Error rate
      - record: ferro:http_errors:rate5m
        expr: rate(ferro_http_responses_total{status=~"5.."}[5m])

      # Error ratio
      - record: ferro:http_error_ratio:rate5m
        expr: |
          rate(ferro_http_responses_total{status=~"5.."}[5m])
          /
          rate(ferro_http_responses_total[5m])

      # p50 latency
      - record: ferro:http_latency:p50
        expr: histogram_quantile(0.50, rate(ferro_http_request_duration_seconds_bucket[5m]))

      # p95 latency
      - record: ferro:http_latency:p95
        expr: histogram_quantile(0.95, rate(ferro_http_request_duration_seconds_bucket[5m]))

      # p99 latency
      - record: ferro:http_latency:p99
        expr: histogram_quantile(0.99, rate(ferro_http_request_duration_seconds_bucket[5m]))

      # Cache hit ratio
      - record: ferro:cache_hit_ratio:rate5m
        expr: |
          rate(ferro_cache_hits_total[5m])
          /
          (rate(ferro_cache_hits_total[5m]) + rate(ferro_cache_misses_total[5m]))

      # Storage throughput (bytes/sec)
      - record: ferro:storage_throughput:rate5m
        expr: rate(ferro_storage_bytes_total[5m])

      # WASM error ratio
      - record: ferro:wasm_error_ratio:rate5m
        expr: |
          rate(ferro_wasm_errors_total[5m])
          /
          rate(ferro_wasm_dispatches_total[5m])
```

---

## Step 6: Grafana Dashboards

### 6.1 Overview Dashboard

**New File:** `monitoring/grafana/dashboards/ferro-overview.json`

```json
{
  "dashboard": {
    "title": "Ferro Overview",
    "uid": "ferro-overview",
    "tags": ["ferro", "sre"],
    "timezone": "utc",
    "panels": [
      {
        "title": "Request Rate",
        "type": "timeseries",
        "gridPos": { "h": 8, "w": 12, "x": 0, "y": 0 },
        "targets": [
          {
            "expr": "sum(rate(ferro_http_requests_total[5m]))",
            "legendFormat": "Total RPS",
            "refId": "A"
          },
          {
            "expr": "sum(rate(ferro_http_requests_total{status=~\"5..\"}[5m]))",
            "legendFormat": "Error RPS",
            "refId": "B"
          }
        ],
        "fieldConfig": {
          "defaults": {
            "unit": "reqps",
            "custom": {
              "drawStyle": "line",
              "lineWidth": 2,
              "fillOpacity": 20
            }
          }
        }
      },
      {
        "title": "Latency Distribution",
        "type": "timeseries",
        "gridPos": { "h": 8, "w": 12, "x": 12, "y": 0 },
        "targets": [
          {
            "expr": "histogram_quantile(0.50, rate(ferro_http_request_duration_seconds_bucket[5m]))",
            "legendFormat": "p50",
            "refId": "A"
          },
          {
            "expr": "histogram_quantile(0.95, rate(ferro_http_request_duration_seconds_bucket[5m]))",
            "legendFormat": "p95",
            "refId": "B"
          },
          {
            "expr": "histogram_quantile(0.99, rate(ferro_http_request_duration_seconds_bucket[5m]))",
            "legendFormat": "p99",
            "refId": "C"
          }
        ],
        "fieldConfig": {
          "defaults": {
            "unit": "s",
            "custom": {
              "drawStyle": "line",
              "lineWidth": 2,
              "fillOpacity": 10
            }
          }
        }
      },
      {
        "title": "Error Rate",
        "type": "stat",
        "gridPos": { "h": 4, "w": 6, "x": 0, "y": 8 },
        "targets": [
          {
            "expr": "sum(rate(ferro_http_responses_total{status=~\"5..\"}[5m])) / sum(rate(ferro_http_responses_total[5m])) * 100",
            "refId": "A"
          }
        ],
        "fieldConfig": {
          "defaults": {
            "unit": "percent",
            "thresholds": {
              "steps": [
                { "value": 0, "color": "green" },
                { "value": 1, "color": "yellow" },
                { "value": 5, "color": "red" }
              ]
            }
          }
        }
      },
      {
        "title": "Cache Hit Ratio",
        "type": "gauge",
        "gridPos": { "h": 4, "w": 6, "x": 6, "y": 8 },
        "targets": [
          {
            "expr": "rate(ferro_cache_hits_total[5m]) / (rate(ferro_cache_hits_total[5m]) + rate(ferro_cache_misses_total[5m])) * 100",
            "refId": "A"
          }
        ],
        "fieldConfig": {
          "defaults": {
            "unit": "percent",
            "min": 0,
            "max": 100,
            "thresholds": {
              "steps": [
                { "value": 0, "color": "red" },
                { "value": 70, "color": "yellow" },
                { "value": 90, "color": "green" }
              ]
            }
          }
        }
      },
      {
        "title": "Storage Operations",
        "type": "timeseries",
        "gridPos": { "h": 8, "w": 12, "x": 0, "y": 12 },
        "targets": [
          {
            "expr": "sum by (operation) (rate(ferro_storage_operations_total[5m]))",
            "legendFormat": "{{operation}}",
            "refId": "A"
          }
        ],
        "fieldConfig": {
          "defaults": {
            "unit": "ops",
            "custom": {
              "drawStyle": "bars",
              "fillOpacity": 80,
              "stacking": { "mode": "normal" }
            }
          }
        }
      },
      {
        "title": "WASM Workers",
        "type": "stat",
        "gridPos": { "h": 4, "w": 6, "x": 12, "y": 12 },
        "targets": [
          {
            "expr": "ferro_wasm_workers",
            "refId": "A"
          }
        ]
      },
      {
        "title": "Active DB Connections",
        "type": "stat",
        "gridPos": { "h": 4, "w": 6, "x": 18, "y": 12 },
        "targets": [
          {
            "expr": "ferro_db_connections_active",
            "refId": "A"
          }
        ]
      }
    ],
    "time": {
      "from": "now-1h",
      "to": "now"
    },
    "refresh": "10s"
  }
}
```

### 6.2 Storage Dashboard

**New File:** `monitoring/grafana/dashboards/ferro-storage.json`

```json
{
  "dashboard": {
    "title": "Ferro Storage",
    "uid": "ferro-storage",
    "panels": [
      {
        "title": "Storage Operation Latency",
        "type": "timeseries",
        "targets": [
          {
            "expr": "histogram_quantile(0.99, rate(ferro_storage_operation_duration_seconds_bucket[5m]))",
            "legendFormat": "p99 {{operation}}",
            "refId": "A"
          }
        ]
      },
      {
        "title": "Total Storage Bytes",
        "type": "stat",
        "targets": [
          {
            "expr": "ferro_server_storage_bytes",
            "refId": "A"
          }
        ],
        "fieldConfig": { "defaults": { "unit": "bytes" } }
      },
      {
        "title": "File Count",
        "type": "stat",
        "targets": [
          {
            "expr": "ferro_server_files",
            "refId": "A"
          }
        ]
      }
    ]
  }
}
```

---

## Step 7: Alert Rules

### 7.1 Updated Alert Rules

**File:** `monitoring/prometheus/alerts.yml`

```yaml
groups:
  - name: ferro_alerts
    rules:
      # === Availability Alerts ===
      - alert: FerroDown
        expr: up{job="ferro-server"} == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "Ferro server is down"
          description: "Ferro server {{ $labels.instance }} has been unreachable for 1 minute"

      - alert: HighErrorRate
        expr: |
          sum(rate(ferro_http_responses_total{status=~"5.."}[5m]))
          /
          sum(rate(ferro_http_responses_total[5m]))
          > 0.05
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "High 5xx error rate"
          description: "Error rate is {{ $value | humanizePercentage }} (threshold: 5%)"

      # === Latency Alerts ===
      - alert: HighLatencyP99
        expr: |
          histogram_quantile(0.99, rate(ferro_http_request_duration_seconds_bucket[5m])) > 2
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "p99 latency exceeds 2s"
          description: "Current p99: {{ $value }}s"

      - alert: LatencySLOViolation
        expr: |
          histogram_quantile(0.99, rate(ferro_http_request_duration_seconds_bucket[5m])) > 0.5
        for: 15m
        labels:
          severity: critical
        annotations:
          summary: "Latency SLO violation"
          description: "p99 latency {{ $value }}s exceeds 500ms SLO"

      # === Storage Alerts ===
      - alert: HighDiskUsage
        expr: |
          (node_filesystem_avail_bytes{mountpoint="/"} / node_filesystem_size_bytes{mountpoint="/"}) < 0.2
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Disk usage above 80%"
          description: "Available: {{ $value | humanizePercentage }}"

      - alert: StorageDegraded
        expr: ferro_storage_health_status == 0
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Storage health degraded"
          description: "Storage health check failing"

      # === Cache Alerts ===
      - alert: CacheHitRateLow
        expr: |
          rate(ferro_cache_hits_total[5m])
          / (rate(ferro_cache_hits_total[5m]) + rate(ferro_cache_misses_total[5m]))
          < 0.7
        for: 15m
        labels:
          severity: warning
        annotations:
          summary: "Cache hit rate below 70%"
          description: "Current hit rate: {{ $value | humanizePercentage }}"

      # === Database Alerts ===
      - alert: DatabaseConnectionPoolExhausted
        expr: ferro_db_connections_active > 80
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "Database connection pool near exhaustion"
          description: "Active connections: {{ $value }}"

      - alert: SlowDatabaseQueries
        expr: |
          rate(ferro_db_query_duration_seconds_sum[5m])
          / rate(ferro_db_query_duration_seconds_count[5m])
          > 0.1
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "Average DB query time > 100ms"
          description: "Average: {{ $value }}s"

      # === WASM Alerts ===
      - alert: WasmErrorRateHigh
        expr: |
          rate(ferro_wasm_errors_total[5m])
          / rate(ferro_wasm_dispatches_total[5m])
          > 0.1
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "WASM error rate above 10%"
          description: "Error ratio: {{ $value | humanizePercentage }}"

      # === Memory Alerts ===
      - alert: HighMemoryUsage
        expr: |
          process_resident_memory_bytes / process_virtual_memory_max_bytes > 0.8
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Memory usage above 80%"
          description: "RSS: {{ $value | humanizePercentage }}"

      # === Pod Alerts (Kubernetes) ===
      - alert: PodCrashLooping
        expr: rate(kube_pod_container_status_restarts_total[15m]) > 0
        for: 15m
        labels:
          severity: critical
        annotations:
          summary: "Pod crash looping"
          description: "Pod {{ $labels.pod }} restarting"

      - alert: PodNotReady
        expr: kube_pod_status_ready{condition="true"} == 0
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Pod not ready"
          description: "Pod {{ $labels.pod }} not ready for 5 minutes"
```

---

## Step 8: OpenTelemetry Collector Configuration

### 8.1 Collector Config

**New File:** `monitoring/otel-collector/otel-collector-config.yaml`

```yaml
receivers:
  otlp:
    protocols:
      grpc:
        endpoint: 0.0.0.0:4317
      http:
        endpoint: 0.0.0.0:4318

  prometheus:
    config:
      scrape_configs:
        - job_name: 'otel-collector'
          static_configs:
            - targets: ['localhost:8888']

processors:
  batch:
    timeout: 5s
    send_batch_size: 1000

  memory_limiter:
    check_interval: 1s
    limit_mib: 512
    spike_limit_mib: 128

  # Add k8s metadata
  k8sattributes:
    extract:
      metadata:
        - k8s.pod.name
        - k8s.namespace.name
        - k8s.deployment.name
    pod_association:
      - sources:
          - from: resource_attribute
            name: k8s.pod.ip

  # Filter high-cardinality metrics
  filter:
    metrics:
      include:
        match_type: regexp
        metric_names:
          - 'ferro_.*'

exporters:
  prometheus:
    endpoint: 0.0.0.0:8889
    namespace: ferro
    const_labels:
      environment: production

  loki:
    endpoint: http://loki:3100/loki/api/v1/push

  otlp/tempo:
    endpoint: tempo:4317
    tls:
      insecure: true

  debug:
    verbosity: basic

service:
  pipelines:
    metrics:
      receivers: [otlp]
      processors: [memory_limiter, batch]
      exporters: [prometheus]

    traces:
      receivers: [otlp]
      processors: [memory_limiter, k8sattributes, batch]
      exporters: [otlp/tempo, debug]

    logs:
      receivers: [otlp]
      processors: [memory_limiter, batch]
      exporters: [loki, debug]
```

---

## Step 9: Testing Procedures

### 9.1 Unit Tests for Metrics

**New File:** `crates/observability/src/tests/metrics_tests.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use opentelemetry::global;
    use opentelemetry_sdk::metrics::SdkMeterProvider;
    use opentelemetry_sdk::export::metrics::MetricExporter;
    use opentelemetry_sdk::runtime;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_counter_instrument() {
        let exporter = opentelemetry_stdout::MetricsExporter::default();
        let provider = SdkMeterProvider::builder()
            .with_periodic_exporter(exporter)
            .build();

        global::set_meter_provider(provider.clone());
        let meter = global::meter("test");
        let counter = meter.u64_counter("test.counter").build();

        counter.add(1, &[]);
        counter.add(5, &[]);

        // Verify via exporter
        provider.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_histogram_instrument() {
        let meter = global::meter("test");
        let histogram = meter.f64_histogram("test.histogram").build();

        histogram.record(0.1, &[]);
        histogram.record(0.5, &[]);
        histogram.record(1.0, &[]);
    }

    #[tokio::test]
    async fn test_labeled_metrics() {
        let meter = global::meter("test");
        let counter = meter.u64_counter("test.labeled").build();

        counter.add(1, &[
            opentelemetry::KeyValue::new("method", "GET"),
            opentelemetry::KeyValue::new("status", 200),
        ]);

        counter.add(1, &[
            opentelemetry::KeyValue::new("method", "POST"),
            opentelemetry::KeyValue::new("status", 201),
        ]);
    }
}
```

### 9.2 Integration Tests

**New File:** `tests/observability_integration.rs`

```rust
use reqwest;
use std::time::Duration;

#[tokio::test]
async fn test_metrics_endpoint() {
    let resp = reqwest::get("http://localhost:8080/metrics")
        .await
        .expect("Failed to reach metrics endpoint");

    assert!(resp.status().is_success());

    let body = resp.text().await.unwrap();
    assert!(body.contains("ferro_http_requests_total"));
    assert!(body.contains("ferro_http_request_duration_seconds"));
}

#[tokio::test]
async fn test_prometheus_format() {
    let resp = reqwest::get("http://localhost:8080/metrics/prometheus")
        .await
        .expect("Failed to reach prometheus endpoint");

    let body = resp.text().await.unwrap();
    assert!(body.contains("# HELP ferro_"));
    assert!(body.contains("# TYPE ferro_"));
}

#[tokio::test]
async fn test_trace_propagation() {
    let client = reqwest::Client::new();
    let resp = client
        .get("http://localhost:8080/test.txt")
        .header("traceparent", "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01")
        .send()
        .await
        .expect("Failed to send request");

    // Verify trace headers in response
    assert!(resp.headers().contains_key("server-timing"));
}
```

### 9.3 Load Testing with Observability

**File:** `benchmarks/k6/observability_load.js`

```javascript
import http from 'k6/http';
import { check, sleep } from 'k6';
import { Counter, Rate, Trend } from 'k6/metrics';

const errorRate = new Rate('errors');
const requestDuration = new Trend('request_duration');

export const options = {
  stages: [
    { duration: '1m', target: 100 },   // Ramp up
    { duration: '5m', target: 100 },   // Steady state
    { duration: '2m', target: 500 },   // Spike
    { duration: '5m', target: 500 },   // Sustained spike
    { duration: '2m', target: 0 },     // Ramp down
  ],
  thresholds: {
    http_req_duration: ['p(99)<500'],  // 99% of requests under 500ms
    errors: ['rate<0.05'],              // Error rate under 5%
  },
};

export default function () {
  const params = {
    headers: {
      'Content-Type': 'application/json',
    },
  };

  // Mix of operations
  const rand = Math.random();
  let res;

  if (rand < 0.6) {
    // 60% reads
    res = http.get(`http://ferro-server:8080/test-${__VU}.txt`, params);
  } else if (rand < 0.8) {
    // 20% writes
    res = http.put(`http://ferro-server:8080/test-${__VU}.txt`, 'test data', params);
  } else if (rand < 0.9) {
    // 10% listings
    res = http.get('http://ferro-server:8080/', params);
  } else {
    // 10% metrics
    res = http.get('http://ferro-server:8080/metrics/prometheus', params);
  }

  check(res, {
    'status is 2xx': (r) => r.status >= 200 && r.status < 300,
    'response time < 500ms': (r) => r.timings.duration < 500,
  });

  errorRate.add(res.status >= 400);
  requestDuration.add(res.timings.duration);

  sleep(0.1);
}
```

### 9.4 Chaos Testing with Observability

**File:** `tests/chaos_observability.rs`

```rust
#[tokio::test]
async fn test_metrics_survive_pod_restart() {
    // Start server
    // Generate some traffic
    // Kill the server
    // Restart the server
    // Verify metrics reset correctly (counters start from 0)
}

#[tokio::test]
async fn test_traces_across_network_partition() {
    // Simulate network partition between server and collector
    // Verify traces are buffered locally
    // Restore connection
    // Verify buffered traces are sent
}

#[tokio::test]
async fn test_metrics_under_memory_pressure() {
    // Fill memory to 90%
    // Generate high traffic
    // Verify metrics still update (no OOM)
    // Verify memory_limiter in collector activates
}
```

---

## Step 10: Deployment Procedures

### 10.1 Docker Compose (Local Dev)

**File:** `monitoring/docker-compose.yml` (update existing):

```yaml
version: '3.8'

services:
  # OpenTelemetry Collector
  otel-collector:
    image: otel/opentelemetry-collector-contrib:latest
    command: ["--config=/etc/otelcol/config.yaml"]
    volumes:
      - ./otel-collector/otel-collector-config.yaml:/etc/otelcol/config.yaml
    ports:
      - "4317:4317"   # OTLP gRPC
      - "4318:4318"   # OTLP HTTP
      - "8888:8888"   # Collector metrics
      - "8889:8889"   # Prometheus export
    depends_on:
      - prometheus
      - loki

  # Prometheus
  prometheus:
    image: prom/prometheus:latest
    volumes:
      - ./prometheus/prometheus.yml:/etc/prometheus/prometheus.yml
      - ./prometheus/alerts.yml:/etc/prometheus/alerts.yml
      - ./prometheus/recording_rules.yml:/etc/prometheus/recording_rules.yml
    ports:
      - "9090:9090"

  # Grafana
  grafana:
    image: grafana/grafana:latest
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=admin
    volumes:
      - ./grafana/dashboards:/var/lib/grafana/dashboards
      - ./grafana/provisioning:/etc/grafana/provisioning
    ports:
      - "3000:3000"
    depends_on:
      - prometheus

  # Loki
  loki:
    image: grafana/loki:latest
    ports:
      - "3100:3100"

  # Alertmanager
  alertmanager:
    image: prom/alertmanager:latest
    volumes:
      - ./alertmanager/alertmanager.yml:/etc/alertmanager/alertmanager.yml
    ports:
      - "9093:9093"

  # Tempo (distributed traces)
  tempo:
    image: grafana/tempo:latest
    ports:
      - "3200:3200"
```

### 10.2 Kubernetes Deployment

**New File:** `k8s/production/otel-collector.yaml`

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: otel-collector
  namespace: monitoring
spec:
  replicas: 2
  selector:
    matchLabels:
      app: otel-collector
  template:
    metadata:
      labels:
        app: otel-collector
    spec:
      containers:
        - name: otel-collector
          image: otel/opentelemetry-collector-contrib:latest
          args:
            - --config=/etc/otelcol/config.yaml
          ports:
            - containerPort: 4317
              name: otlp-grpc
            - containerPort: 4318
              name: otlp-http
          resources:
            requests:
              memory: "256Mi"
              cpu: "250m"
            limits:
              memory: "512Mi"
              cpu: "500m"
          volumeMounts:
            - name: config
              mountPath: /etc/otelcol
      volumes:
        - name: config
          configMap:
            name: otel-collector-config
---
apiVersion: v1
kind: Service
metadata:
  name: otel-collector
  namespace: monitoring
spec:
  selector:
    app: otel-collector
  ports:
    - port: 4317
      name: otlp-grpc
    - port: 4318
      name: otlp-http
  type: ClusterIP
```

---

## Appendix A: Metric Naming Convention

| Category | Prefix | Example |
|----------|--------|---------|
| HTTP | `ferro.http.` | `ferro.http.requests.total` |
| Storage | `ferro.storage.` | `ferro.storage.operations.total` |
| Cache | `ferro.cache.` | `ferro.cache.hits.total` |
| WASM | `ferro.wasm.` | `ferro.wasm.dispatches.total` |
| Database | `ferro.db.` | `ferro.db.queries.total` |
| Sync | `ferro.sync.` | `ferro.sync.operations.total` |
| Server | `ferro.server.` | `ferro.server.uptime` |

## Appendix B: Label Cardinality Limits

| Metric | Max Labels | Max Series |
|--------|-----------|------------|
| `ferro.http.requests.total` | 4 (method, status, path, tenant) | 10,000 |
| `ferro.storage.operations.total` | 3 (operation, success, tier) | 100 |
| `ferro.cache.hits.total` | 2 (cache_name, tier) | 100 |
| `ferro.wasm.dispatches.total` | 2 (worker, success) | 1,000 |
| `ferro.db.queries.total` | 2 (query_type, success) | 100 |

## Appendix C: Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `OTEL_EXPORTER_OTLP_ENDPOINT` | `http://localhost:4317` | OTLP collector endpoint |
| `OTEL_SERVICE_NAME` | `ferro-server` | Service name |
| `OTEL_TRACE_SAMPLE_RATIO` | `1.0` | Trace sampling ratio (0.0-1.0) |
| `OTEL_METRIC_EXPORT_INTERVAL` | `15000` | Metric export interval (ms) |
| `FERRO_LOG_LEVEL` | `info` | Log level filter |
| `FERRO_LOG_FORMAT` | `json` | Log format (json/text) |
| `LOKI_ENDPOINT` | `http://localhost:3100` | Loki push endpoint |
