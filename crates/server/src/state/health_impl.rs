use super::AppState;
use ferro_server_health::HealthState;
use ferro_server_state::ServerState as _;

#[async_trait::async_trait]
impl HealthState for AppState {
    fn is_started(&self) -> bool {
        self.startup_complete.load(std::sync::atomic::Ordering::Relaxed)
    }

    async fn storage_list(&self, prefix: &str) -> Result<(), String> {
        self.storage().list(prefix).await.map(|_| ()).map_err(|e| e.to_string())
    }

    fn has_metadata_store(&self) -> bool {
        self.metadata_store.is_some()
    }

    fn has_cas_store(&self) -> bool {
        self.cas_store.is_some()
    }

    fn has_wasm_runtime(&self) -> bool {
        self.wasm_runtime.is_some()
    }

    fn has_search(&self) -> bool {
        self.search.is_some()
    }

    fn has_oidc(&self) -> bool {
        self.oidc.is_some()
    }

    async fn check_database(&self) -> bool {
        match &self.db {
            Some(db) => db
                .lock()
                .ok()
                .and_then(|conn| conn.execute_batch("SELECT 1;").ok())
                .is_some(),
            None => true, // No DB configured, not a failure.
        }
    }

    async fn check_search(&self) -> bool {
        match &self.search {
            Some(search) => search.try_read().is_ok(),
            None => true, // No search configured, not a failure.
        }
    }

    fn uptime(&self) -> std::time::Duration {
        self.started_at.elapsed()
    }
}
