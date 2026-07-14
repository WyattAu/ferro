use dashmap::DashMap;
use wasmtime::{Engine, Linker, Store, Val};

use crate::api::add_host_functions;
use crate::error::WasmHostError;
use crate::plugin::{LoadedPlugin, PluginHandle};
use crate::sandbox::{WasmHostConfig, WasmState};

pub struct WasmHost {
    pub(crate) engine: Engine,
    pub(crate) config: WasmHostConfig,
    pub(crate) plugins: DashMap<String, LoadedPlugin>,
}

impl WasmHost {
    pub fn new(config: WasmHostConfig) -> Result<Self, WasmHostError> {
        let mut engine_config = wasmtime::Config::new();
        engine_config.consume_fuel(config.fuel_enabled);

        let engine = Engine::new(&engine_config).map_err(|e| WasmHostError::CompileFailed(e.to_string()))?;

        Ok(Self {
            engine,
            config,
            plugins: DashMap::new(),
        })
    }

    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    pub fn config(&self) -> &WasmHostConfig {
        &self.config
    }

    pub fn create_store(&self, plugin_name: &str) -> Result<Store<WasmState>, WasmHostError> {
        let mut store = Store::new(
            &self.engine,
            WasmState::for_plugin(plugin_name, self.config.host_config.clone()),
        );

        if self.config.fuel_enabled {
            store
                .set_fuel(self.config.fuel_limit)
                .map_err(|e| WasmHostError::RuntimeError(e.to_string()))?;
        }

        Ok(store)
    }

    pub fn create_linker(&self) -> Result<Linker<WasmState>, WasmHostError> {
        let mut linker = Linker::new(&self.engine);
        add_host_functions(&mut linker)?;
        Ok(linker)
    }

    pub fn call(&self, handle: &PluginHandle, func: &str, args: &[Val]) -> Result<Vec<Val>, WasmHostError> {
        let plugin = self
            .plugins
            .get(&handle.name)
            .ok_or_else(|| WasmHostError::NotFound(handle.name.clone()))?;

        let mut store = self.create_store(&handle.name)?;
        let linker = self.create_linker()?;
        let instance = linker
            .instantiate(&mut store, &plugin.module)
            .map_err(|e| WasmHostError::InstantiationFailed(e.to_string()))?;

        let func = instance
            .get_func(&mut store, func)
            .ok_or_else(|| WasmHostError::RuntimeError(format!("Function '{}' not found", func)))?;

        let mut results = vec![Val::I32(0); func.ty(&store).results().len()];
        func.call(&mut store, args, &mut results).map_err(WasmHostError::from)?;

        Ok(results)
    }

    pub fn call_with_input(&self, handle: &PluginHandle, func: &str, input: &[u8]) -> Result<Vec<u8>, WasmHostError> {
        let plugin = self
            .plugins
            .get(&handle.name)
            .ok_or_else(|| WasmHostError::NotFound(handle.name.clone()))?;

        let mut store = self.create_store(&handle.name)?;
        let linker = self.create_linker()?;
        let instance = linker
            .instantiate(&mut store, &plugin.module)
            .map_err(|e| WasmHostError::InstantiationFailed(e.to_string()))?;

        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or_else(|| WasmHostError::RuntimeError("Plugin has no memory export".into()))?;

        let alloc_fn = instance
            .get_typed_func::<u32, u32>(&mut store, "alloc")
            .map_err(|e| WasmHostError::RuntimeError(format!("No alloc function: {e}")))?;

        let dealloc_fn = instance
            .get_typed_func::<(u32, u32), ()>(&mut store, "dealloc")
            .map_err(|e| WasmHostError::RuntimeError(format!("No dealloc function: {e}")))?;

        let len = input.len() as u32;
        let ptr = alloc_fn
            .call(&mut store, len)
            .map_err(|e| WasmHostError::RuntimeError(e.to_string()))?;
        memory.data_mut(&mut store)[ptr as usize..ptr as usize + len as usize].copy_from_slice(input);

        let entry = instance.get_typed_func::<u32, u32>(&mut store, func).map_err(|e| {
            WasmHostError::RuntimeError(format!("Function '{}' not found or wrong signature: {e}", func))
        })?;

        let result_ptr = entry.call(&mut store, ptr).map_err(WasmHostError::from)?;

        let memory_data = memory.data(&store);
        let len_ptr = result_ptr as usize;
        let len = if len_ptr + 4 <= memory_data.len() {
            u32::from_le_bytes(memory_data[len_ptr..len_ptr + 4].try_into().unwrap_or([0; 4]))
        } else {
            0
        };
        let data_ptr = (result_ptr + 4) as usize;
        let output = if data_ptr + len as usize <= memory_data.len() {
            memory_data[data_ptr..data_ptr + len as usize].to_vec()
        } else {
            Vec::new()
        };

        dealloc_fn
            .call(&mut store, (ptr, len))
            .map_err(|e| WasmHostError::RuntimeError(e.to_string()))?;

        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use wasmtime::Val;

    const SIMPLE_ADD_WAT: &str = r#"
        (module
            (func $add (param i32 i32) (result i32)
                local.get 0
                local.get 1
                i32.add
            )
            (export "add" (func $add))
        )
    "#;

    const CONST_RETURN_WAT: &str = r#"
        (module
            (func $answer (result i32)
                i32.const 42
            )
            (export "answer" (func $answer))
        )
    "#;

    fn compile_wat(wat: &str) -> Vec<u8> {
        wat::parse_str(wat).expect("failed to parse WAT")
    }

    fn make_host() -> WasmHost {
        let config = WasmHostConfig {
            fuel_enabled: false,
            ..Default::default()
        };
        WasmHost::new(config).expect("failed to create host")
    }

    fn make_host_fueled(fuel: u64) -> WasmHost {
        let config = WasmHostConfig {
            fuel_enabled: true,
            fuel_limit: fuel,
            ..Default::default()
        };
        WasmHost::new(config).expect("failed to create host")
    }

    #[test]
    fn test_host_creation_default_config() {
        let config = WasmHostConfig::default();
        let host = WasmHost::new(config);
        assert!(host.is_ok());
    }

    #[test]
    fn test_host_creation_custom_config() {
        let config = WasmHostConfig {
            max_memory_bytes: 128 * 1024 * 1024,
            max_execution_time: std::time::Duration::from_secs(10),
            max_instances: 5,
            fuel_enabled: true,
            fuel_limit: 500_000_000,
            ..Default::default()
        };
        let host = WasmHost::new(config).expect("failed to create host");
        assert_eq!(host.config().max_memory_bytes, 128 * 1024 * 1024);
        assert_eq!(host.config().max_instances, 5);
        assert!(host.config().fuel_enabled);
        assert_eq!(host.config().fuel_limit, 500_000_000);
    }

    #[test]
    fn test_host_creation_with_host_config() {
        let mut host_config = HashMap::new();
        host_config.insert("key1".to_string(), "value1".to_string());
        let config = WasmHostConfig {
            host_config,
            ..Default::default()
        };
        let host = WasmHost::new(config).expect("failed to create host");
        assert_eq!(host.config().host_config.get("key1").unwrap(), "value1");
    }

    #[test]
    fn test_load_simple_plugin() {
        let host = make_host();
        let wasm = compile_wat(SIMPLE_ADD_WAT);
        let handle = host.load_plugin("add-plugin", &wasm);
        assert!(handle.is_ok());
        let handle = handle.unwrap();
        assert_eq!(handle.name, "add-plugin");
        assert_eq!(host.plugin_count(), 1);
    }

    #[test]
    fn test_call_add_function() {
        let host = make_host();
        let wasm = compile_wat(SIMPLE_ADD_WAT);
        let handle = host.load_plugin("add-plugin", &wasm).unwrap();

        let results = host.call(&handle, "add", &[Val::I32(3), Val::I32(7)]);
        assert!(results.is_ok());
        let results = results.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].unwrap_i32(), 10);
    }

    #[test]
    fn test_call_const_return() {
        let host = make_host();
        let wasm = compile_wat(CONST_RETURN_WAT);
        let handle = host.load_plugin("answer-plugin", &wasm).unwrap();

        let results = host.call(&handle, "answer", &[]);
        assert!(results.is_ok());
        assert_eq!(results.unwrap()[0].unwrap_i32(), 42);
    }

    #[test]
    fn test_invalid_wasm_rejected() {
        let host = make_host();
        let bad_bytes = b"not valid wasm at all";

        let result = host.load_plugin("bad", bad_bytes);
        assert!(result.is_err());
        match result.unwrap_err() {
            WasmHostError::InvalidPlugin(_) => {}
            other => panic!("expected InvalidPlugin, got {:?}", other),
        }
    }

    #[test]
    fn test_empty_wasm_rejected() {
        let host = make_host();
        let result = host.load_plugin("empty", &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_truncated_wasm_rejected() {
        let host = make_host();
        let bytes = [0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x01];
        let result = host.load_plugin("truncated", &bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_duplicate_plugin_rejected() {
        let host = make_host();
        let wasm = compile_wat(SIMPLE_ADD_WAT);
        host.load_plugin("add-plugin", &wasm).unwrap();

        let result = host.load_plugin("add-plugin", &wasm);
        assert!(result.is_err());
        match result.unwrap_err() {
            WasmHostError::AlreadyLoaded(name) => assert_eq!(name, "add-plugin"),
            other => panic!("expected AlreadyLoaded, got {:?}", other),
        }
    }

    #[test]
    fn test_unload_plugin() {
        let host = make_host();
        let wasm = compile_wat(SIMPLE_ADD_WAT);
        let handle = host.load_plugin("add-plugin", &wasm).unwrap();
        assert_eq!(host.plugin_count(), 1);

        let result = host.unload_plugin(&handle);
        assert!(result.is_ok());
        assert_eq!(host.plugin_count(), 0);
    }

    #[test]
    fn test_unload_nonexistent_plugin() {
        let host = make_host();
        let handle = PluginHandle {
            id: 999,
            name: "nonexistent".to_string(),
            checksum: "none".to_string(),
        };
        let result = host.unload_plugin(&handle);
        assert!(result.is_err());
        match result.unwrap_err() {
            WasmHostError::NotFound(name) => assert_eq!(name, "nonexistent"),
            other => panic!("expected NotFound, got {:?}", other),
        }
    }

    #[test]
    fn test_call_nonexistent_function() {
        let host = make_host();
        let wasm = compile_wat(SIMPLE_ADD_WAT);
        let handle = host.load_plugin("add-plugin", &wasm).unwrap();

        let result = host.call(&handle, "nonexistent", &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_call_nonexistent_plugin() {
        let host = make_host();
        let handle = PluginHandle {
            id: 999,
            name: "nonexistent".to_string(),
            checksum: "none".to_string(),
        };
        let result = host.call(&handle, "add", &[]);
        assert!(result.is_err());
        match result.unwrap_err() {
            WasmHostError::NotFound(name) => assert_eq!(name, "nonexistent"),
            other => panic!("expected NotFound, got {:?}", other),
        }
    }

    #[test]
    fn test_multiple_plugins() {
        let host = make_host();
        let wasm_add = compile_wat(SIMPLE_ADD_WAT);
        let wasm_answer = compile_wat(CONST_RETURN_WAT);

        let h1 = host.load_plugin("add-plugin", &wasm_add).unwrap();
        let h2 = host.load_plugin("answer-plugin", &wasm_answer).unwrap();

        assert_eq!(host.plugin_count(), 2);

        let r1 = host.call(&h1, "add", &[Val::I32(10), Val::I32(20)]).unwrap();
        assert_eq!(r1[0].unwrap_i32(), 30);

        let r2 = host.call(&h2, "answer", &[]).unwrap();
        assert_eq!(r2[0].unwrap_i32(), 42);
    }

    #[test]
    fn test_validate_plugin_bytes() {
        let wasm = compile_wat(SIMPLE_ADD_WAT);
        let result = WasmHost::validate_plugin_bytes(&wasm);
        assert!(result.is_ok());
        let v = result.unwrap();
        assert!(v.valid);
        assert!(v.size > 0);
        assert!(v.exports.contains(&"add".to_string()));
        assert!(v.warnings.is_empty());
    }

    #[test]
    fn test_validate_invalid_bytes() {
        let result = WasmHost::validate_plugin_bytes(b"garbage");
        assert!(result.is_err());
    }

    #[test]
    fn test_plugin_handle_checksum() {
        let host = make_host();
        let wasm = compile_wat(SIMPLE_ADD_WAT);
        let handle = host.load_plugin("checksum-plugin", &wasm).unwrap();
        assert!(!handle.checksum.is_empty());
        assert_eq!(handle.checksum.len(), 64);
    }

    #[test]
    fn test_fuel_enforcement() {
        let infinite_wat = r#"
            (module
                (func $loop (export "loop")
                    (loop $again
                        br $again
                    )
                )
            )
        "#;
        let host = make_host_fueled(1000);
        let wasm = compile_wat(infinite_wat);
        let handle = host.load_plugin("infinite", &wasm).unwrap();

        let result = host.call(&handle, "loop", &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_instance_limit() {
        let config = WasmHostConfig {
            max_instances: 2,
            fuel_enabled: false,
            ..Default::default()
        };
        let host = WasmHost::new(config).expect("failed to create host");
        let wasm = compile_wat(SIMPLE_ADD_WAT);

        host.load_plugin("p1", &wasm).unwrap();
        host.load_plugin("p2", &wasm).unwrap();

        let result = host.load_plugin("p3", &wasm);
        assert!(result.is_err());
    }

    #[test]
    fn test_config_propagation_to_store() {
        let mut host_cfg = HashMap::new();
        host_cfg.insert("theme".to_string(), "dark".to_string());
        let config = WasmHostConfig {
            host_config: host_cfg.clone(),
            fuel_enabled: false,
            ..Default::default()
        };
        let host = WasmHost::new(config).unwrap();
        let store = host.create_store("test-plugin").unwrap();
        assert_eq!(store.data().host_config.get("theme").unwrap(), "dark");
        assert_eq!(store.data().plugin_name, "test-plugin");
    }

    #[test]
    fn test_get_plugin() {
        let host = make_host();
        let wasm = compile_wat(SIMPLE_ADD_WAT);
        let loaded = host.load_plugin("add-plugin", &wasm).unwrap();

        let retrieved = host.get_plugin("add-plugin");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, loaded.name);

        assert!(host.get_plugin("nonexistent").is_none());
    }

    #[test]
    fn test_create_linker_has_host_functions() {
        let host = make_host();
        let linker = host.create_linker().unwrap();
        let engine = host.engine();
        let module = wasmtime::Module::new(engine, compile_wat(SIMPLE_ADD_WAT)).unwrap();
        let mut store = host.create_store("test").unwrap();
        let instance = linker.instantiate(&mut store, &module).unwrap();
        assert!(instance.get_func(&mut store, "add").is_some());
    }
}
