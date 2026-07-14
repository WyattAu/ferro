use wasmtime::Module;

use crate::WasmHost;
use crate::error::WasmHostError;
use crate::sandbox::{check_instance_limit, validate_plugin};

#[derive(Debug)]
pub struct ValidationResult {
    pub valid: bool,
    pub size: usize,
    pub imports: Vec<String>,
    pub exports: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct PluginHandle {
    pub id: u64,
    pub name: String,
    pub checksum: String,
}

#[derive(Debug)]
pub struct LoadedPlugin {
    pub handle: PluginHandle,
    pub module: Module,
    pub exports: Vec<String>,
}

static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

impl WasmHost {
    pub fn load_plugin(&self, name: &str, wasm_bytes: &[u8]) -> Result<PluginHandle, WasmHostError> {
        if self.plugins.contains_key(name) {
            return Err(WasmHostError::AlreadyLoaded(name.to_string()));
        }

        check_instance_limit(self.plugins.len(), &self.config)?;

        let validation = validate_plugin(wasm_bytes)?;
        if !validation.valid {
            return Err(WasmHostError::InvalidPlugin("Validation failed".into()));
        }

        let module = Module::new(&self.engine, wasm_bytes).map_err(|e| WasmHostError::CompileFailed(e.to_string()))?;

        let checksum = compute_checksum(wasm_bytes);

        let id = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let handle = PluginHandle {
            id,
            name: name.to_string(),
            checksum,
        };

        let loaded = LoadedPlugin {
            handle: handle.clone(),
            module,
            exports: validation.exports,
        };

        self.plugins.insert(name.to_string(), loaded);
        Ok(handle)
    }

    pub fn unload_plugin(&self, handle: &PluginHandle) -> Result<(), WasmHostError> {
        if self.plugins.remove(&handle.name).is_none() {
            return Err(WasmHostError::NotFound(handle.name.clone()));
        }
        Ok(())
    }

    pub fn validate_plugin_bytes(wasm_bytes: &[u8]) -> Result<ValidationResult, WasmHostError> {
        validate_plugin(wasm_bytes)
    }

    pub fn get_plugin(&self, name: &str) -> Option<PluginHandle> {
        self.plugins.get(name).map(|p| p.handle.clone())
    }

    pub fn plugin_count(&self) -> usize {
        self.plugins.len()
    }
}

fn compute_checksum(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}
