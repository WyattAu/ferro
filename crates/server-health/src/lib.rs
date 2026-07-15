use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

/// Abstraction over the server state methods needed by health endpoints.
/// Implemented for `AppState` in the `ferro-server` crate.
#[async_trait::async_trait]
pub trait HealthState: Send + Sync + Clone + 'static {
    /// Returns true once the server has completed all startup checks.
    fn is_started(&self) -> bool;

    /// List objects at the given prefix in storage. Used for liveness.
    async fn storage_list(&self, prefix: &str) -> Result<(), String>;

    /// Returns whether a persistent metadata store is configured.
    fn has_metadata_store(&self) -> bool;

    /// Returns whether a CAS (content-addressable storage) store is configured.
    fn has_cas_store(&self) -> bool;

    /// Returns whether a WASM runtime is configured.
    fn has_wasm_runtime(&self) -> bool;

    /// Returns whether a search engine is configured.
    fn has_search(&self) -> bool;

    /// Returns whether OIDC authentication is configured.
    fn has_oidc(&self) -> bool;

    /// Check if the SQLite database (if configured) is reachable.
    async fn check_database(&self) -> bool;

    /// Check if the search index (if configured) is reachable.
    async fn check_search(&self) -> bool;

    /// Time since the server started.
    fn uptime(&self) -> std::time::Duration;
}

// ---------------------------------------------------------------------------
// Impl functions (called by Axum handlers in the server crate)
// ---------------------------------------------------------------------------

/// Kubernetes-style startup probe.
/// Returns 200 once `is_started()` is true, 503 otherwise.
pub async fn startup_impl<S: HealthState>(state: &S) -> Response {
    if state.is_started() {
        (StatusCode::OK, axum::Json(serde_json::json!({"status": "ok"}))).into_response()
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            axum::Json(serde_json::json!({"status": "starting"})),
        )
            .into_response()
    }
}

/// Readiness probe: checks storage, metadata store, database, and search index.
pub async fn readiness_impl<S: HealthState>(state: &S) -> Response {
    let mut subsystems = serde_json::Map::new();
    let mut healthy = true;

    let storage_ok = state.storage_list("/").await.is_ok();
    subsystems.insert(
        "storage".to_string(),
        serde_json::json!(if storage_ok { "ok" } else { "error" }),
    );
    if !storage_ok {
        healthy = false;
    }

    subsystems.insert(
        "metadata".to_string(),
        serde_json::json!(if state.has_metadata_store() {
            "persistent"
        } else {
            "in-memory"
        }),
    );

    let db_ok = state.check_database().await;
    subsystems.insert(
        "database".to_string(),
        serde_json::json!(if db_ok { "ok" } else { "error" }),
    );
    if !db_ok {
        healthy = false;
    }

    let search_ok = state.check_search().await;
    subsystems.insert(
        "search".to_string(),
        serde_json::json!(if search_ok { "ok" } else { "error" }),
    );
    if !search_ok {
        healthy = false;
    }

    let status = if healthy { "ok" } else { "degraded" };
    let code = if healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    let body = serde_json::json!({
        "status": status,
        "subsystems": subsystems,
    });
    (code, axum::Json(body)).into_response()
}

/// Full health check endpoint: reports status of all subsystems.
pub async fn health_check_impl<S: HealthState>(state: &S) -> Response {
    let mut subsystems = serde_json::Map::new();
    let mut healthy = true;

    let storage_ok = state.storage_list("/").await.is_ok();
    subsystems.insert(
        "storage".to_string(),
        serde_json::json!(if storage_ok { "ok" } else { "error" }),
    );
    if !storage_ok {
        healthy = false;
    }

    subsystems.insert(
        "metadata".to_string(),
        serde_json::json!(if state.has_metadata_store() {
            "persistent"
        } else {
            "in-memory"
        }),
    );

    subsystems.insert(
        "wasm".to_string(),
        serde_json::json!(if state.has_wasm_runtime() { "ok" } else { "disabled" }),
    );

    subsystems.insert(
        "search".to_string(),
        serde_json::json!(if state.has_search() { "ok" } else { "disabled" }),
    );

    subsystems.insert(
        "auth".to_string(),
        serde_json::json!(if state.has_oidc() { "configured" } else { "disabled" }),
    );

    subsystems.insert(
        "cas".to_string(),
        serde_json::json!(if state.has_cas_store() {
            "enabled"
        } else {
            "disabled"
        }),
    );

    let status = if healthy { "ok" } else { "degraded" };
    let code = if healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    let body = serde_json::json!({
        "status": status,
        "version": env!("CARGO_PKG_VERSION"),
        "uptime_seconds": state.uptime().as_secs(),
        "subsystems": subsystems,
    });
    (code, axum::Json(body)).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct MockHealthState {
        started: bool,
    }

    #[async_trait::async_trait]
    impl HealthState for MockHealthState {
        fn is_started(&self) -> bool {
            self.started
        }
        async fn storage_list(&self, _prefix: &str) -> Result<(), String> {
            Ok(())
        }
        fn has_metadata_store(&self) -> bool {
            false
        }
        fn has_cas_store(&self) -> bool {
            false
        }
        fn has_wasm_runtime(&self) -> bool {
            false
        }
        fn has_search(&self) -> bool {
            false
        }
        fn has_oidc(&self) -> bool {
            false
        }
        async fn check_database(&self) -> bool {
            true
        }
        async fn check_search(&self) -> bool {
            true
        }
        fn uptime(&self) -> std::time::Duration {
            std::time::Duration::from_secs(42)
        }
    }

    #[tokio::test]
    async fn test_startup_not_started() {
        let state = MockHealthState { started: false };
        let resp = startup_impl(&state).await;
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn test_startup_started() {
        let state = MockHealthState { started: true };
        let resp = startup_impl(&state).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_readiness_ok() {
        let state = MockHealthState { started: false };
        let resp = readiness_impl(&state).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_health_check_ok() {
        let state = MockHealthState { started: false };
        let resp = health_check_impl(&state).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
