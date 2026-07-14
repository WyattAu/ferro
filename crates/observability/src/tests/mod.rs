use std::collections::HashMap;

use crate::*;

#[test]
fn test_counter_basic() {
    let c = Counter::new("test_counter", "A test counter");
    assert_eq!(c.get(), 0);
    c.inc();
    assert_eq!(c.get(), 1);
    c.inc_by(10);
    assert_eq!(c.get(), 11);
}

#[test]
fn test_gauge_basic() {
    let g = Gauge::new("test_gauge", "A test gauge");
    assert_eq!(g.get(), 0);
    g.set(42);
    assert_eq!(g.get(), 42);
    g.inc();
    assert_eq!(g.get(), 43);
    g.dec();
    assert_eq!(g.get(), 42);
}

#[test]
fn test_histogram_basic() {
    let h = Histogram::new("test_hist", "A test histogram", vec![0.1, 0.5, 1.0, 5.0]);
    h.observe(0.05);
    h.observe(0.3);
    h.observe(0.8);
    h.observe(3.0);
    h.observe(10.0);
    assert_eq!(h.total_count(), 5);
}

#[test]
fn test_histogram_cumulative_buckets() {
    let h = Histogram::new("test_hist", "A test histogram", vec![1.0, 5.0, 10.0]);
    h.observe(0.5);
    h.observe(3.0);
    h.observe(7.0);
    h.observe(15.0);

    assert_eq!(h.bucket_count(0), 1);
    assert_eq!(h.bucket_count(1), 2);
    assert_eq!(h.bucket_count(2), 3);
    assert_eq!(h.total_count(), 4);
}

#[test]
fn test_labels() {
    let mut labels = Labels::new();
    labels.push("method", "GET");
    labels.push("path", "/api/files");
    let s = labels.as_str();
    assert!(s.contains("method=\"GET\""));
    assert!(s.contains("path=\"/api/files\""));
}

#[test]
fn test_labels_with() {
    let labels = Labels::with(vec![("app", "ferro"), ("env", "prod")]);
    assert_eq!(labels.len(), 2);
}

#[test]
fn test_registry() {
    let reg = MetricsRegistry::new();
    let c = reg.register_counter("requests_total", "Total requests");
    let g = reg.register_gauge("active_connections", "Active connections");
    let h = reg.register_histogram("request_duration", "Request duration", vec![0.01, 0.05, 0.1, 0.5, 1.0]);

    c.inc();
    g.set(10);
    h.observe(0.03);

    let output = export_prometheus(&reg);
    assert!(output.contains("requests_total"));
    assert!(output.contains("active_connections"));
    assert!(output.contains("request_duration"));
}

#[test]
fn test_log_buffer() {
    let buffer = LogBuffer::new(100);
    assert_eq!(buffer.len(), 0);

    buffer.push(LogEntry {
        timestamp: 1000,
        line: "test message".to_string(),
        labels: HashMap::new(),
        level: "info".to_string(),
        source: "test".to_string(),
    });
    assert_eq!(buffer.len(), 1);

    let results = buffer.query(Some("info"), 10);
    assert_eq!(results.len(), 1);

    let results = buffer.query(Some("error"), 10);
    assert_eq!(results.len(), 0);
}

#[test]
fn test_log_buffer_ring() {
    let buffer = LogBuffer::new(3);
    for i in 0..5 {
        buffer.push(LogEntry {
            timestamp: i as i64,
            line: format!("msg {}", i),
            labels: HashMap::new(),
            level: "info".to_string(),
            source: "test".to_string(),
        });
    }
    assert_eq!(buffer.len(), 3);
    let results = buffer.query(None, 10);
    assert_eq!(results[0].line, "msg 4");
    assert_eq!(results[2].line, "msg 2");
}

#[test]
fn test_prometheus_export() {
    let reg = MetricsRegistry::new();
    let c = reg.register_counter("http_requests", "Total HTTP requests");
    c.inc_by(42);

    let output = export_prometheus(&reg);
    assert!(output.contains("# HELP http_requests Total HTTP requests"));
    assert!(output.contains("# TYPE http_requests counter"));
    assert!(output.contains("http_requests_total 42"));
}

#[test]
fn test_prometheus_export_gauge() {
    let reg = MetricsRegistry::new();
    let g = reg.register_gauge("temperature", "Current temperature");
    g.set(72);

    let output = export_prometheus(&reg);
    assert!(output.contains("# TYPE temperature gauge"));
    assert!(output.contains("temperature 72"));
}

#[test]
fn test_prometheus_export_histogram() {
    let reg = MetricsRegistry::new();
    let h = reg.register_histogram("request_duration_seconds", "Request duration", vec![0.1, 0.5, 1.0]);
    h.observe(0.05);
    h.observe(0.3);
    h.observe(0.8);

    let output = export_prometheus(&reg);
    assert!(output.contains("# TYPE request_duration_seconds histogram"));
    assert!(output.contains("request_duration_seconds_bucket{le=\"0.1\"} 1"));
    assert!(output.contains("request_duration_seconds_bucket{le=\"+Inf\"} 3"));
    assert!(output.contains("request_duration_seconds_count 3"));
}

#[test]
fn test_global_registry() {
    let reg = global_registry();
    let _ = reg.register_counter("global_test_counter", "Global registry test");
}

#[test]
fn test_log_buffer_clear() {
    let buffer = LogBuffer::new(100);
    buffer.push(LogEntry {
        timestamp: 1,
        line: "test".to_string(),
        labels: HashMap::new(),
        level: "info".to_string(),
        source: "test".to_string(),
    });
    assert_eq!(buffer.len(), 1);
    buffer.clear();
    assert_eq!(buffer.len(), 0);
    assert!(buffer.is_empty());
}

#[test]
fn test_log_buffer_is_empty() {
    let buffer = LogBuffer::new(100);
    assert!(buffer.is_empty());
    buffer.push(LogEntry {
        timestamp: 1,
        line: "test".to_string(),
        labels: HashMap::new(),
        level: "info".to_string(),
        source: "test".to_string(),
    });
    assert!(!buffer.is_empty());
}

#[test]
fn test_log_buffer_query_limit() {
    let buffer = LogBuffer::new(100);
    for i in 0..10 {
        buffer.push(LogEntry {
            timestamp: i as i64,
            line: format!("msg {}", i),
            labels: HashMap::new(),
            level: "info".to_string(),
            source: "test".to_string(),
        });
    }
    let results = buffer.query(None, 3);
    assert_eq!(results.len(), 3);
    assert_eq!(results[0].line, "msg 9");
    assert_eq!(results[1].line, "msg 8");
    assert_eq!(results[2].line, "msg 7");
}

#[test]
fn test_registry_default() {
    let reg = MetricsRegistry::default();
    let c = reg.register_counter("default_test", "Test");
    c.inc();
    assert_eq!(c.get(), 1);
}

#[test]
fn test_labels_default() {
    let labels = Labels::default();
    assert!(labels.is_empty());
}
