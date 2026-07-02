use dashmap::DashMap;
use std::path::PathBuf;
use std::sync::Arc;

pub mod plugin_marketplace_api;
pub mod plugin_permissions;
pub mod wasm_upload;
pub mod workers;

/// Trait abstracting the server state needed by plugin and worker handlers.
pub trait PluginState: Clone + Send + Sync + 'static {
    fn plugin_registry(&self) -> &Arc<DashMap<String, plugin_permissions::PluginManifest>>;
    fn workers_dir(&self) -> Option<&PathBuf>;
    fn wasm_runtime(&self) -> Option<&Arc<ferro_core::wasm::WasmWorkerRuntime>>;
}
