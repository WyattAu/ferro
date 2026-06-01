use ferro_health::{HealthChecker, MemoryProbe};

pub fn create_health_checker() -> HealthChecker {
    HealthChecker::new(env!("CARGO_PKG_VERSION"))
}

pub fn create_health_checker_with_memory_probe(threshold_percent: f64) -> HealthChecker {
    let checker = create_health_checker();
    let probe = MemoryProbe::new(threshold_percent);
    let _ = checker.register(Box::new(probe));
    checker
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_health_checker() {
        let checker = create_health_checker();
        assert_eq!(checker.probe_count(), 0);
    }

    #[tokio::test]
    async fn test_health_checker_with_probe() {
        let checker = create_health_checker_with_memory_probe(90.0);
        assert_eq!(checker.probe_count(), 1);
        let response = checker.check_liveness().await;
        assert!(!response.checks.is_empty());
    }

    #[tokio::test]
    async fn test_health_checker_readiness_empty() {
        let checker = create_health_checker();
        let response = checker.check_readiness().await;
        assert!(response.checks.is_empty());
    }
}
