use std::collections::HashMap;

use crate::error::WasmHostError;
use crate::plugin::ValidationResult;

const MAX_WASM_SIZE: usize = 100 * 1024 * 1024;

/// The import allow-list (THM-PLUGIN-IMPORT-GATE): fixed at host build
/// time, immutable per instance — the Lean model's declared imports.
pub const ALLOWED_IMPORTS: &[&str] = &[
    "host::log",
    "host::storage_get",
    "host::storage_set",
    "host::get_config",
];

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
        return Err(WasmHostError::InvalidPlugin("WASM module too small to be valid".into()));
    }

    let magic = &wasm_bytes[0..4];
    if magic != [0x00, 0x61, 0x73, 0x6d] {
        return Err(WasmHostError::InvalidPlugin("Invalid WASM magic number".into()));
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

    // THM-PLUGIN-IMPORT-GATE (YP-TENANCY 1.2, PluginContainment.lean):
    // fail closed on any import outside the declared allow-list. The ABI
    // exposes exactly the `host` namespace below; wasi_* and any other
    // namespace are rejected before instantiation.
    let mut disallowed: Vec<String> = imports
        .iter()
        .filter(|i| !ALLOWED_IMPORTS.contains(&i.as_str()))
        .cloned()
        .collect();
    if !disallowed.is_empty() {
        disallowed.sort();
        return Err(WasmHostError::InvalidPlugin(format!(
            "imports outside the allowed set: {}",
            disallowed.join(", ")
        )));
    }

    Ok(ValidationResult {
        valid: true,
        size: wasm_bytes.len(),
        imports,
        exports,
        warnings,
    })
}

pub fn check_instance_limit(current_count: usize, config: &WasmHostConfig) -> Result<(), WasmHostError> {
    if current_count >= config.max_instances {
        Err(WasmHostError::InvalidPlugin(format!(
            "Max instances ({}) reached",
            config.max_instances
        )))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_import_gate_rejects_undeclared_import() {
        // THM-PLUGIN-IMPORT-GATE: an import outside the allow-list must be
        // rejected before instantiation, fail-closed.
        let wat = r#"
            (module
                (import "env" "evil" (func $evil))
                (func (export "run") (call $evil))
            )"#;
        let bytes = wat::parse_str(wat).expect("wat");
        let err = validate_plugin(&bytes).expect_err("must reject");
        assert!(err.to_string().contains("env::evil"), "err: {err}");
    }

    #[test]
    fn test_import_gate_accepts_abi_imports() {
        let wat = r#"
            (module
                (import "host" "log" (func $log (param i32 i32 i32)))
                (func (export "run") (call $log (i32.const 0) (i32.const 0) (i32.const 0)))
            )"#;
        let bytes = wat::parse_str(wat).expect("wat");
        let result = validate_plugin(&bytes).expect("ABI imports allowed");
        assert!(result.valid);
        assert!(result.imports.iter().any(|i| i == "host::log"));
    }

    #[test]
    fn test_import_gate_rejects_wasi() {
        let wat = r#"
            (module
                (import "wasi_snapshot_preview1" "random_get"
                  (func $random_get (param i32 i32) (result i32)))
                (func (export "run") (result i32) (call $random_get (i32.const 0) (i32.const 0)))
            )"#;
        let bytes = wat::parse_str(wat).expect("wat");
        let err = validate_plugin(&bytes).expect_err("wasi must be rejected");
        assert!(err.to_string().contains("wasi_snapshot_preview1::random_get"));
    }

    #[test]
    fn test_allowed_imports_list_is_immutable_abi() {
        // The allow-list is exactly the ABI v1 host surface.
        assert_eq!(
            ALLOWED_IMPORTS,
            &[
                "host::log",
                "host::storage_get",
                "host::storage_set",
                "host::get_config",
            ]
        );
    }
}
