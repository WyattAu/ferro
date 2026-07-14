use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use dashmap::DashMap;
use std::sync::Arc;

use crate::AppState;

#[derive(Debug, Clone, serde::Serialize)]
pub struct StorageHealth {
    pub backend_type: String,
    pub healthy: bool,
    pub last_check: String,
    pub latency_ms: u64,
    pub error: Option<String>,
    pub total_checks: u64,
    pub failed_checks: u64,
}

#[derive(Debug)]
pub struct StorageHealthMonitor {
    statuses: Arc<DashMap<String, StorageHealth>>,
}

impl StorageHealthMonitor {
    pub fn new() -> Self {
        Self {
            statuses: Arc::new(DashMap::new()),
        }
    }

    pub fn record_success(&self, backend: &str, latency_ms: u64) {
        let mut entry = self
            .statuses
            .entry(backend.to_string())
            .or_insert_with(|| StorageHealth {
                backend_type: backend.to_string(),
                healthy: true,
                last_check: chrono::Utc::now().to_rfc3339(),
                latency_ms,
                error: None,
                total_checks: 0,
                failed_checks: 0,
            });
        entry.healthy = true;
        entry.last_check = chrono::Utc::now().to_rfc3339();
        entry.latency_ms = latency_ms;
        entry.error = None;
        entry.total_checks += 1;
    }

    pub fn record_failure(&self, backend: &str, error: &str) {
        let mut entry = self
            .statuses
            .entry(backend.to_string())
            .or_insert_with(|| StorageHealth {
                backend_type: backend.to_string(),
                healthy: false,
                last_check: chrono::Utc::now().to_rfc3339(),
                latency_ms: 0,
                error: Some(error.to_string()),
                total_checks: 0,
                failed_checks: 0,
            });
        entry.healthy = false;
        entry.last_check = chrono::Utc::now().to_rfc3339();
        entry.error = Some(error.to_string());
        entry.total_checks += 1;
        entry.failed_checks += 1;
    }

    pub fn get_health(&self, backend: &str) -> Option<StorageHealth> {
        self.statuses.get(backend).map(|e| e.value().clone())
    }

    pub fn get_all_health(&self) -> Vec<StorageHealth> {
        self.statuses.iter().map(|e| e.value().clone()).collect()
    }

    pub fn any_unhealthy(&self) -> bool {
        self.statuses.iter().any(|e| !e.value().healthy)
    }
}

impl Default for StorageHealthMonitor {
    fn default() -> Self {
        Self::new()
    }
}

pub async fn storage_health_handler(State(state): State<AppState>) -> Response {
    let health = state.storage_health.get_all_health();
    (
        StatusCode::OK,
        axum::Json(serde_json::json!({
            "backends": health,
            "any_unhealthy": state.storage_health.any_unhealthy(),
        })),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_success() {
        let monitor = StorageHealthMonitor::new();
        monitor.record_success("local", 5);
        let health = monitor.get_health("local").unwrap();
        assert!(health.healthy);
        assert_eq!(health.latency_ms, 5);
        assert_eq!(health.total_checks, 1);
        assert_eq!(health.failed_checks, 0);
    }

    #[test]
    fn test_record_failure() {
        let monitor = StorageHealthMonitor::new();
        monitor.record_failure("s3", "connection timeout");
        let health = monitor.get_health("s3").unwrap();
        assert!(!health.healthy);
        assert!(health.error.as_ref().unwrap().contains("timeout"));
        assert_eq!(health.failed_checks, 1);
    }

    #[test]
    fn test_recovery() {
        let monitor = StorageHealthMonitor::new();
        monitor.record_failure("local", "error");
        assert!(!monitor.get_health("local").unwrap().healthy);
        monitor.record_success("local", 10);
        assert!(monitor.get_health("local").unwrap().healthy);
        assert_eq!(monitor.get_health("local").unwrap().total_checks, 2);
    }

    #[test]
    fn test_any_unhealthy() {
        let monitor = StorageHealthMonitor::new();
        monitor.record_success("local", 1);
        assert!(!monitor.any_unhealthy());
        monitor.record_failure("s3", "error");
        assert!(monitor.any_unhealthy());
    }

    #[test]
    fn test_get_all_health() {
        let monitor = StorageHealthMonitor::new();
        monitor.record_success("local", 1);
        monitor.record_failure("s3", "error");
        assert_eq!(monitor.get_all_health().len(), 2);
    }
}
