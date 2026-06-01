use std::collections::HashMap;

use crate::error::WasmHostError;
use crate::plugin::ValidationResult;

const MAX_WASM_SIZE: usize = 100 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct WasmHostConfig {
    pub max_memory_bytes: u64,
    pub max_execution_time: std::time::Duration,
    pub max_instances: usize,
    pub fuel_enabled: bool,
    pub fuel_limit: u64,
    pub allow_network: bool,
    pub allow_filesystem: bool,
    pub wasi_enabled: bool,
    pub host_config: HashMap<String, String>,
}

impl Default for WasmHostConfig {
    fn default() -> Self {
        Self {
            max_memory_bytes: 256 * 1024 * 1024,
            max_execution_time: std::time::Duration::from_secs(30),
            max_instances: 10,
            fuel_enabled: true,
            fuel_limit: 1_000_000_000,
            allow_network: false,
            allow_filesystem: false,
            wasi_enabled: true,
            host_config: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct WasmState {
    pub plugin_name: String,
    pub storage: HashMap<String, Vec<u8>>,
    pub host_config: HashMap<String, String>,
    pub remaining_fuel: u64,
    pub log_messages: Vec<(u32, String)>,
}

impl WasmState {
    pub fn for_plugin(name: &str, host_config: HashMap<String, String>) -> Self {
        Self {
            plugin_name: name.to_string(),
            host_config,
            ..Default::default()
        }
    }
}

pub fn validate_plugin(wasm_bytes: &[u8]) -> Result<ValidationResult, WasmHostError> {
    if wasm_bytes.len() > MAX_WASM_SIZE {
        return Err(WasmHostError::InvalidPlugin(format!(
            "WASM module too large: {} bytes (max {})",
            wasm_bytes.len(),
            MAX_WASM_SIZE
        )));
    }

    if wasm_bytes.len() < 8 {
        return Err(WasmHostError::InvalidPlugin(
            "WASM module too small to be valid".into(),
        ));
    }

    let magic = &wasm_bytes[0..4];
    if magic != [0x00, 0x61, 0x73, 0x6d] {
        return Err(WasmHostError::InvalidPlugin(
            "Invalid WASM magic number".into(),
        ));
    }

    let engine = wasmtime::Engine::default();
    let module = match wasmtime::Module::new(&engine, wasm_bytes) {
        Ok(m) => m,
        Err(e) => return Err(WasmHostError::CompileFailed(e.to_string())),
    };

    let imports: Vec<String> = module
        .imports()
        .map(|i| format!("{}::{}", i.module(), i.name()))
        .collect();

    let exports: Vec<String> = module.exports().map(|e| e.name().to_string()).collect();

    let mut warnings = Vec::new();
    for imp in &imports {
        if imp.contains("i64") {
            warnings.push(format!("Floating i64 import detected: {}", imp));
        }
    }

    Ok(ValidationResult {
        valid: true,
        size: wasm_bytes.len(),
        imports,
        exports,
        warnings,
    })
}

pub fn check_instance_limit(
    current_count: usize,
    config: &WasmHostConfig,
) -> Result<(), WasmHostError> {
    if current_count >= config.max_instances {
        Err(WasmHostError::InvalidPlugin(format!(
            "Max instances ({}) reached",
            config.max_instances
        )))
    } else {
        Ok(())
    }
}
