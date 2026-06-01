use crate::*;
use std::time::Duration;

struct MockProbe {
    probe_name: String,
    probe_type: ProbeType,
    result_status: HealthStatus,
}

impl MockProbe {
    fn new(name: &str, probe_type: ProbeType, status: HealthStatus) -> Self {
        Self {
            probe_name: name.to_string(),
            probe_type,
            result_status: status,
        }
    }
}

#[async_trait::async_trait]
impl HealthProbe for MockProbe {
    fn name(&self) -> &str {
        &self.probe_name
    }

    async fn check(&self) -> ProbeResult {
        ProbeResult::healthy(&self.probe_name, Duration::from_micros(10))
            .with_status(self.result_status)
    }

    fn probe_type(&self) -> ProbeType {
        self.probe_type
    }
}

#[tokio::test]
async fn register_and_check_probe() {
    let checker = HealthChecker::new("1.0.0");
    checker
        .register(Box::new(MockProbe::new(
            "test",
            ProbeType::Liveness,
            HealthStatus::Healthy,
        )))
        .unwrap();
    assert_eq!(checker.probe_count(), 1);

    let response = checker.check_all().await;
    assert_eq!(response.status, HealthStatus::Healthy);
    assert_eq!(response.checks.len(), 1);
    assert_eq!(response.checks[0].name, "test");
}

#[tokio::test]
async fn duplicate_registration_fails() {
    let checker = HealthChecker::new("1.0.0");
    checker
        .register(Box::new(MockProbe::new(
            "dup",
            ProbeType::Liveness,
            HealthStatus::Healthy,
        )))
        .unwrap();
    let result = checker.register(Box::new(MockProbe::new(
        "dup",
        ProbeType::Readiness,
        HealthStatus::Healthy,
    )));
    assert!(result.is_err());
    assert_eq!(checker.probe_count(), 1);
}

#[tokio::test]
async fn unregister_probe() {
    let checker = HealthChecker::new("1.0.0");
    checker
        .register(Box::new(MockProbe::new(
            "rem",
            ProbeType::Liveness,
            HealthStatus::Healthy,
        )))
        .unwrap();
    assert_eq!(checker.probe_count(), 1);

    checker.unregister("rem").unwrap();
    assert_eq!(checker.probe_count(), 0);

    let err = checker.unregister("rem");
    assert!(err.is_err());
}

#[tokio::test]
async fn unregister_nonexistent_fails() {
    let checker = HealthChecker::new("1.0.0");
    let err = checker.unregister("nope");
    assert!(err.is_err());
    assert!(matches!(err, Err(HealthError::ProbeNotFound { .. })));
}

#[tokio::test]
async fn liveness_readiness_startup_separation() {
    let checker = HealthChecker::new("1.0.0");
    checker
        .register(Box::new(MockProbe::new(
            "live",
            ProbeType::Liveness,
            HealthStatus::Healthy,
        )))
        .unwrap();
    checker
        .register(Box::new(MockProbe::new(
            "ready",
            ProbeType::Readiness,
            HealthStatus::Degraded,
        )))
        .unwrap();
    checker
        .register(Box::new(MockProbe::new(
            "start",
            ProbeType::Startup,
            HealthStatus::Unhealthy,
        )))
        .unwrap();

    let liveness = checker.check_liveness().await;
    assert_eq!(liveness.status, HealthStatus::Healthy);
    assert_eq!(liveness.checks.len(), 1);
    assert_eq!(liveness.checks[0].name, "live");

    let readiness = checker.check_readiness().await;
    assert_eq!(readiness.status, HealthStatus::Degraded);
    assert_eq!(readiness.checks.len(), 1);

    let startup = checker.check_startup().await;
    assert_eq!(startup.status, HealthStatus::Unhealthy);
    assert_eq!(startup.checks.len(), 1);
}

#[tokio::test]
async fn check_all_includes_everything() {
    let checker = HealthChecker::new("1.0.0");
    checker
        .register(Box::new(MockProbe::new(
            "a",
            ProbeType::Liveness,
            HealthStatus::Healthy,
        )))
        .unwrap();
    checker
        .register(Box::new(MockProbe::new(
            "b",
            ProbeType::Readiness,
            HealthStatus::Healthy,
        )))
        .unwrap();
    checker
        .register(Box::new(MockProbe::new(
            "c",
            ProbeType::Startup,
            HealthStatus::Healthy,
        )))
        .unwrap();

    let response = checker.check_all().await;
    assert_eq!(response.status, HealthStatus::Healthy);
    assert_eq!(response.checks.len(), 3);
}

#[tokio::test]
async fn aggregation_healthy_plus_unhealthy_equals_degraded() {
    let checker = HealthChecker::new("1.0.0");
    checker
        .register(Box::new(MockProbe::new(
            "ok",
            ProbeType::Liveness,
            HealthStatus::Healthy,
        )))
        .unwrap();
    checker
        .register(Box::new(MockProbe::new(
            "bad",
            ProbeType::Liveness,
            HealthStatus::Unhealthy,
        )))
        .unwrap();

    let response = checker.check_liveness().await;
    assert_eq!(response.status, HealthStatus::Degraded);
}

#[tokio::test]
async fn aggregation_all_healthy() {
    let checker = HealthChecker::new("1.0.0");
    checker
        .register(Box::new(MockProbe::new(
            "a",
            ProbeType::Liveness,
            HealthStatus::Healthy,
        )))
        .unwrap();
    checker
        .register(Box::new(MockProbe::new(
            "b",
            ProbeType::Liveness,
            HealthStatus::Healthy,
        )))
        .unwrap();

    let response = checker.check_liveness().await;
    assert_eq!(response.status, HealthStatus::Healthy);
}

#[tokio::test]
async fn aggregation_mixed_degraded_and_unhealthy() {
    let checker = HealthChecker::new("1.0.0");
    checker
        .register(Box::new(MockProbe::new(
            "a",
            ProbeType::Liveness,
            HealthStatus::Healthy,
        )))
        .unwrap();
    checker
        .register(Box::new(MockProbe::new(
            "b",
            ProbeType::Liveness,
            HealthStatus::Degraded,
        )))
        .unwrap();

    let response = checker.check_liveness().await;
    assert_eq!(response.status, HealthStatus::Degraded);
}

#[tokio::test]
async fn aggregation_all_unknown() {
    let checker = HealthChecker::new("1.0.0");
    checker
        .register(Box::new(MockProbe::new(
            "a",
            ProbeType::Liveness,
            HealthStatus::Unknown,
        )))
        .unwrap();
    checker
        .register(Box::new(MockProbe::new(
            "b",
            ProbeType::Liveness,
            HealthStatus::Unknown,
        )))
        .unwrap();

    let response = checker.check_liveness().await;
    assert_eq!(response.status, HealthStatus::Unknown);
}

#[test]
fn probe_result_serialization() {
    let result = ProbeResult::healthy("test-probe", Duration::from_millis(42))
        .with_status(HealthStatus::Degraded)
        .with_message("something is off")
        .with_detail("key", serde_json::json!("value"));

    let json = serde_json::to_string(&result).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed["name"], "test-probe");
    assert_eq!(parsed["status"], "Degraded");
    assert_eq!(parsed["message"], "something is off");
    assert!(parsed["details"]["key"].is_string());
}

#[test]
fn probe_result_deserialization_roundtrip() {
    let result = ProbeResult::healthy("roundtrip", Duration::from_nanos(123))
        .with_status(HealthStatus::Unhealthy)
        .with_message("failed");

    let json = serde_json::to_string(&result).unwrap();
    let de: ProbeResult = serde_json::from_str(&json).unwrap();

    assert_eq!(de.name, "roundtrip");
    assert_eq!(de.status, HealthStatus::Unhealthy);
    assert_eq!(de.message, Some("failed".to_string()));
    assert_eq!(de.duration, Duration::from_nanos(123));
    assert!(de.details.is_empty());
}

#[test]
fn health_response_serialization() {
    let response = HealthResponse::new("2.0.0".to_string(), Duration::from_secs(3600))
        .with_info("env", "production")
        .with_checks(vec![ProbeResult::healthy("db", Duration::from_millis(5))]);

    let json = serde_json::to_string(&response).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed["version"], "2.0.0");
    assert_eq!(parsed["status"], "Healthy");
    assert_eq!(parsed["info"]["env"], "production");
    assert_eq!(parsed["checks"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn global_status_override() {
    let checker = HealthChecker::new("1.0.0");
    checker
        .register(Box::new(MockProbe::new(
            "live",
            ProbeType::Liveness,
            HealthStatus::Healthy,
        )))
        .unwrap();

    let before = checker.check_liveness().await;
    assert_eq!(before.status, HealthStatus::Healthy);

    checker
        .set_global_status(HealthStatus::Unhealthy, "shutting down")
        .await;

    let after = checker.check_liveness().await;
    assert_eq!(after.status, HealthStatus::Unhealthy);
    assert_eq!(after.info.get("override_message").unwrap(), "shutting down");
}

#[tokio::test]
async fn global_status_applies_to_all_checks() {
    let checker = HealthChecker::new("1.0.0");
    checker
        .register(Box::new(MockProbe::new(
            "a",
            ProbeType::Liveness,
            HealthStatus::Healthy,
        )))
        .unwrap();
    checker
        .register(Box::new(MockProbe::new(
            "b",
            ProbeType::Readiness,
            HealthStatus::Healthy,
        )))
        .unwrap();

    checker
        .set_global_status(HealthStatus::Degraded, "partial outage")
        .await;

    assert_eq!(
        checker.check_liveness().await.status,
        HealthStatus::Degraded
    );
    assert_eq!(
        checker.check_readiness().await.status,
        HealthStatus::Degraded
    );
    assert_eq!(checker.check_all().await.status, HealthStatus::Degraded);
}

#[tokio::test]
async fn component_status_lookup() {
    let checker = HealthChecker::new("1.0.0");
    checker
        .register(Box::new(MockProbe::new(
            "db",
            ProbeType::Liveness,
            HealthStatus::Healthy,
        )))
        .unwrap();

    let status = checker.component_status("db");
    assert!(status.is_some());

    let missing = checker.component_status("nonexistent");
    assert!(missing.is_none());
}

#[tokio::test]
async fn database_probe() {
    let probe = DatabaseProbe::new("postgres").with_status(HealthStatus::Healthy, "connected");
    let result = probe.check().await;
    assert_eq!(result.name, "postgres");
    assert_eq!(result.status, HealthStatus::Healthy);
    assert_eq!(probe.probe_type(), ProbeType::Readiness);
}

#[tokio::test]
async fn database_probe_unhealthy() {
    let probe =
        DatabaseProbe::new("mysql").with_status(HealthStatus::Unhealthy, "connection refused");
    let result = probe.check().await;
    assert_eq!(result.status, HealthStatus::Unhealthy);
    assert_eq!(result.message, Some("connection refused".to_string()));
}

#[tokio::test]
async fn memory_probe_runs() {
    let probe = MemoryProbe::new(99.9).with_name("mem");
    let result = probe.check().await;
    assert_eq!(result.name, "mem");
    assert_eq!(probe.probe_type(), ProbeType::Liveness);
    assert!(result.details.contains_key("usage_percent"));
    assert!(result.details.contains_key("threshold_percent"));
}

#[tokio::test]
async fn disk_space_probe_runs() {
    let probe = DiskSpaceProbe::new("/tmp", 1);
    let result = probe.check().await;
    assert!(result.name.starts_with("disk-"));
    assert_eq!(probe.probe_type(), ProbeType::Liveness);
    assert!(result.details.contains_key("available_bytes"));
}

#[tokio::test]
async fn custom_probe_with_closure() {
    let probe = CustomProbe::new("custom", ProbeType::Startup, || async move {
        ProbeResult::healthy("custom", Duration::from_millis(1))
            .with_status(HealthStatus::Degraded)
            .with_message("custom check")
    });

    let result = probe.check().await;
    assert_eq!(result.name, "custom");
    assert_eq!(result.status, HealthStatus::Degraded);
    assert_eq!(probe.probe_type(), ProbeType::Startup);

    let checker = HealthChecker::new("1.0.0");
    checker.register(Box::new(probe)).unwrap();
    let response = checker.check_startup().await;
    assert_eq!(response.status, HealthStatus::Degraded);
}

#[tokio::test]
async fn empty_checker_returns_unknown() {
    let checker = HealthChecker::new("1.0.0");
    let response = checker.check_all().await;
    assert_eq!(response.status, HealthStatus::Unknown);
    assert!(response.checks.is_empty());
}

#[tokio::test]
async fn response_has_version_and_uptime() {
    let checker = HealthChecker::new("3.5.0");
    checker
        .register(Box::new(MockProbe::new(
            "p",
            ProbeType::Liveness,
            HealthStatus::Healthy,
        )))
        .unwrap();

    let response = checker.check_all().await;
    assert_eq!(response.version, "3.5.0");
    assert!(response.uptime > Duration::ZERO);
}

#[test]
fn health_status_serde_roundtrip() {
    let statuses = [
        HealthStatus::Healthy,
        HealthStatus::Degraded,
        HealthStatus::Unhealthy,
        HealthStatus::Unknown,
    ];
    for status in &statuses {
        let json = serde_json::to_string(status).unwrap();
        let de: HealthStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(*status, de);
    }
}

#[test]
fn probe_type_serde_roundtrip() {
    let types = [
        ProbeType::Liveness,
        ProbeType::Readiness,
        ProbeType::Startup,
    ];
    for pt in &types {
        let json = serde_json::to_string(pt).unwrap();
        let de: ProbeType = serde_json::from_str(&json).unwrap();
        assert_eq!(*pt, de);
    }
}
