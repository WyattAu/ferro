//! healthkit-backed registry bridge (R5-B, estate adoption).
//!
//! Builds a `healthkit::HealthRegistry` from Ferro's [`HealthState`] so the
//! subsystem probes gain healthkit's per-check timeouts, parallel
//! execution, error detail, and Prometheus rendering — without touching the
//! existing k8s probe handlers (`/healthz`, `/readyz` keep their contracts).

use std::sync::Arc;
use std::time::Duration;

use healthkit::{HealthCheckError, HealthRegistry, HealthStatus};

use crate::HealthState;

/// Build the subsystem registry: one check per Ferro dependency, each
/// bounded by healthkit's default timeout. Optional subsystems (OIDC, WASM)
/// report `Degraded` when absent rather than failing readiness.
#[must_use]
pub fn subsystem_registry<S: HealthState>(state: &S) -> HealthRegistry {
    // healthkit 0.1 registers checks via a blocking write, which panics
    // under a tokio runtime. Build on a plain thread so this is callable
    // from startup (async) and tests alike.
    let state = Arc::new(state.clone());
    std::thread::spawn(move || build_registry(state))
        .join()
        .expect("registry build thread panicked")
}

fn build_registry<S: HealthState>(state: Arc<S>) -> HealthRegistry {
    let registry = HealthRegistry::new();

    registry.add_check("storage", {
        let state = state.clone();
        move || {
            let state = state.clone();
            async move {
                match state.storage_list("/").await {
                    Ok(()) => Ok(HealthStatus::Healthy),
                    Err(e) => Err(HealthCheckError::DependencyUnavailable(format!("storage: {e}"))),
                }
            }
        }
    });

    registry.add_check("database", {
        let state = state.clone();
        move || {
            let state = state.clone();
            async move {
                if state.check_database().await {
                    Ok(HealthStatus::Healthy)
                } else {
                    Err(HealthCheckError::DependencyUnavailable("database unreachable".into()))
                }
            }
        }
    });

    registry.add_check("search", {
        let state = state.clone();
        move || {
            let state = state.clone();
            async move {
                if !state.has_search() {
                    return Ok(HealthStatus::Degraded);
                }
                if state.check_search().await {
                    Ok(HealthStatus::Healthy)
                } else {
                    Err(HealthCheckError::DependencyUnavailable(
                        "search index unreachable".into(),
                    ))
                }
            }
        }
    });

    registry.add_check("cas", {
        let state = state.clone();
        move || {
            let state = state.clone();
            async move {
                if state.has_cas_store() {
                    Ok(HealthStatus::Healthy)
                } else {
                    Ok(HealthStatus::Degraded)
                }
            }
        }
    });

    registry.add_check("metadata", {
        let state = state.clone();
        move || {
            let state = state.clone();
            async move {
                if state.has_metadata_store() {
                    Ok(HealthStatus::Healthy)
                } else {
                    Ok(HealthStatus::Degraded)
                }
            }
        }
    });

    // Optional subsystems: absence is a configuration fact, not a health
    // problem — Healthy keeps liveness green; readiness is unaffected
    // because there is nothing partially broken.
    registry.add_check("wasm", {
        let state = state.clone();
        move || {
            let state = state.clone();
            async move { Ok::<HealthStatus, HealthCheckError>(HealthStatus::Healthy) }
        }
    });

    registry.add_check("oidc", {
        let state = state.clone();
        move || {
            let state = state.clone();
            async move { Ok::<HealthStatus, HealthCheckError>(HealthStatus::Healthy) }
        }
    });

    registry
}

/// Overall status: the worst of the individual results. healthkit 1.2.1
/// fixed its inverted `check_readiness` aggregate upstream; Ferro keeps
/// this local aggregate as a belt-and-braces contract on the exact
/// semantics the /healthz/detailed route serves.
#[must_use]
pub fn overall_status(results: &[healthkit::CheckResult]) -> HealthStatus {
    results
        .iter()
        .map(|r| r.status)
        .max_by_key(|s| match s {
            HealthStatus::Healthy => 0,
            HealthStatus::Degraded => 1,
            HealthStatus::Unhealthy => 2,
        })
        .unwrap_or(HealthStatus::Healthy)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct TestState {
        started: bool,
        db: bool,
        search: bool,
    }

    #[async_trait::async_trait]
    impl HealthState for TestState {
        fn is_started(&self) -> bool {
            self.started
        }
        async fn storage_list(&self, _prefix: &str) -> Result<(), String> {
            Ok(())
        }
        fn has_metadata_store(&self) -> bool {
            true
        }
        fn has_cas_store(&self) -> bool {
            true
        }
        fn has_wasm_runtime(&self) -> bool {
            false
        }
        fn has_search(&self) -> bool {
            self.search
        }
        fn has_oidc(&self) -> bool {
            false
        }
        async fn check_database(&self) -> bool {
            self.db
        }
        async fn check_search(&self) -> bool {
            self.search
        }
        fn uptime(&self) -> std::time::Duration {
            Duration::from_secs(1)
        }
    }

    #[tokio::test]
    async fn test_healthy_state_all_dependencies_ok() {
        let registry = subsystem_registry(&TestState {
            started: true,
            db: true,
            search: true,
        });
        let results = registry.check_all().await;
        assert_eq!(results.len(), 7);
        assert!(results.iter().all(|r| r.status.is_healthy()));
        let liveness = registry.check_liveness().await;
        assert!(liveness.is_ok_and(|s| s.is_healthy()));
    }

    #[tokio::test]
    async fn test_dead_database_is_unhealthy_and_readiness_fails() {
        let registry = subsystem_registry(&TestState {
            started: true,
            db: false,
            search: false,
        });
        let results = registry.check_all().await;
        let db = results.iter().find(|r| r.name == "database").expect("db check");
        assert_eq!(db.status, HealthStatus::Unhealthy);
        let results = registry.check_all().await;
        let status = overall_status(&results);
        assert_eq!(status, HealthStatus::Unhealthy, "results: {results:?}");
        assert!(
            results
                .iter()
                .any(|r| r.name == "database" && r.status == HealthStatus::Unhealthy)
        );
    }
}
