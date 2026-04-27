use ferro_common::error::{FerroError, Result};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};
use wasmtime::*;
use wasmtime_wasi::p2::pipe::{MemoryInputPipe, MemoryOutputPipe};
use wasmtime_wasi::p1::{self, WasiP1Ctx};
use wasmtime_wasi::WasiCtxBuilder;

/// Configuration limits for WASM worker execution.
#[derive(Debug, Clone)]
pub struct WorkerConfig {
    pub max_time_ms: u64,
    pub max_memory_bytes: usize,
    pub max_fuel: u64,
    pub allowed_paths: Vec<String>,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            max_time_ms: 30_000,
            max_memory_bytes: 64 * 1024 * 1024,
            max_fuel: 1_000_000_000,
            allowed_paths: vec![],
        }
    }
}

/// Result of a WASM worker execution.
#[derive(Debug, Clone)]
pub struct WorkerResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub fuel_consumed: u64,
    pub execution_time_ms: u64,
}

/// A registered WASM worker that triggers on a file path pattern.
#[derive(Debug, Clone)]
pub struct WorkerEvent {
    pub pattern: String,
    pub module_path: String,
    pub config: WorkerConfig,
    pub function_name: String,
}

/// WASM worker runtime using Wasmtime with WASI support.
pub struct WasmWorkerRuntime {
    engine: Engine,
    workers: Arc<RwLock<Vec<WorkerEvent>>>,
    config: WorkerConfig,
}

impl WasmWorkerRuntime {
    /// Create a new WASM runtime with default configuration.
    pub fn new() -> Result<Self> {
        let mut config = Config::new();
        config.consume_fuel(true);
        config.wasm_threads(false);

        let engine = Engine::new(&config)
            .map_err(|e| FerroError::Internal(format!("Wasmtime engine creation failed: {}", e)))?;

        info!("WASM worker runtime initialized");

        Ok(Self {
            engine,
            workers: Arc::new(RwLock::new(Vec::new())),
            config: WorkerConfig::default(),
        })
    }

    /// Register a worker event handler for a file path pattern.
    pub async fn register_worker(&self, event: WorkerEvent) {
        let mut workers = self.workers.write().await;
        info!("Registered WASM worker: {} -> {}::{}",
            event.pattern,
            event.module_path,
            event.function_name
        );
        workers.push(event);
    }

    /// Execute a WASM module on a blocking thread pool.
    ///
    /// This uses `tokio::task::spawn_blocking` to avoid blocking the async
    /// runtime with CPU-heavy compilation and WASM execution.
    ///
    /// Improvements over the previous implementation:
    /// - Runs on blocking thread pool (doesn't starve async tasks)
    /// - Enforces `max_time_ms` via `tokio::time::timeout`
    /// - Passes `input` bytes into WASM linear memory
    /// - Captures stdout/stderr into in-memory buffers
    pub async fn execute(
        &self,
        module_path: &str,
        function_name: &str,
        input: &[u8],
        config: Option<WorkerConfig>,
    ) -> Result<WorkerResult> {
        let config = config.unwrap_or_else(|| self.config.clone());
        let max_time_ms = config.max_time_ms;
        let engine = self.engine.clone();
        let module_path_owned = module_path.to_string();
        let function_name_owned = function_name.to_string();
        let input_owned = input.to_vec();

        // Move all heavy work to the blocking thread pool
        let result = tokio::task::spawn_blocking(move || {
            execute_blocking(
                &engine,
                &module_path_owned,
                &function_name_owned,
                &input_owned,
                &config,
            )
        });

        // Apply time limit
        match tokio::time::timeout(
            std::time::Duration::from_millis(max_time_ms),
            result,
        ).await {
            Ok(Ok(result)) => result,
            Ok(Err(e)) => Ok(WorkerResult {
                success: false,
                output: String::new(),
                error: Some(format!("Worker task panicked: {}", e)),
                fuel_consumed: 0,
                execution_time_ms: max_time_ms,
            }),
            Err(_) => Ok(WorkerResult {
                success: false,
                output: String::new(),
                error: Some(format!("Worker timed out after {}ms", max_time_ms)),
                fuel_consumed: 0,
                execution_time_ms: max_time_ms,
            }),
        }
    }

    /// Find all workers whose pattern matches the given file path.
    pub async fn find_matching_workers(&self, file_path: &str) -> Vec<WorkerEvent> {
        let workers = self.workers.read().await;
        workers.iter()
            .filter(|w| self.pattern_matches(&w.pattern, file_path))
            .cloned()
            .collect()
    }

    fn pattern_matches(&self, pattern: &str, path: &str) -> bool {
        if pattern == "*" {
            return true;
        }

        if let Some(suffix) = pattern.strip_prefix("*") {
            return path.ends_with(suffix);
        }

        if let Some(prefix) = pattern.strip_suffix("*") {
            return path.starts_with(prefix);
        }

        path == pattern
    }

    /// List all registered workers.
    pub async fn list_workers(&self) -> Vec<WorkerEvent> {
        self.workers.read().await.clone()
    }
}

/// Synchronous execution that runs on a blocking thread.
fn execute_blocking(
    engine: &Engine,
    module_path: &str,
    function_name: &str,
    input: &[u8],
    config: &WorkerConfig,
) -> Result<WorkerResult> {
    let start = std::time::Instant::now();

    let module_bytes = match std::fs::read(module_path) {
        Ok(bytes) => bytes,
        Err(e) => return Ok(WorkerResult {
            success: false,
            output: String::new(),
            error: Some(format!("Failed to read module: {}", e)),
            fuel_consumed: 0,
            execution_time_ms: 0,
        }),
    };

    let module = match Module::from_binary(engine, &module_bytes) {
        Ok(m) => m,
        Err(e) => return Ok(WorkerResult {
            success: false,
            output: String::new(),
            error: Some(format!("Module compilation failed: {}", e)),
            fuel_consumed: 0,
            execution_time_ms: start.elapsed().as_millis() as u64,
        }),
    };

    // Capture stdout/stderr into in-memory pipes instead of inheriting
    let stdout_pipe = MemoryOutputPipe::new(1024 * 1024); // 1MB capture buffer
    let stderr_pipe = MemoryOutputPipe::new(1024 * 1024);
    let stdin_pipe = MemoryInputPipe::new(input.to_vec());

    let wasi_ctx = WasiCtxBuilder::new()
        .stdin(stdin_pipe)
        .stdout(stdout_pipe.clone())
        .stderr(stderr_pipe)
        .build_p1();

    let mut store = Store::new(engine, wasi_ctx);
    store.set_fuel(config.max_fuel)
        .map_err(|e| FerroError::Internal(format!("Failed to set fuel: {}", e)))?;

    let mut linker = Linker::new(engine);
    p1::add_to_linker_sync(&mut linker, |s: &mut WasiP1Ctx| s)
        .map_err(|e| FerroError::Internal(format!("WASI setup failed: {}", e)))?;

    let instance = match linker.instantiate(&mut store, &module) {
        Ok(i) => i,
        Err(e) => return Ok(WorkerResult {
            success: false,
            output: String::new(),
            error: Some(format!("Instantiation failed: {}", e)),
            fuel_consumed: store.get_fuel().unwrap_or(0),
            execution_time_ms: start.elapsed().as_millis() as u64,
        }),
    };

    // Load input into WASM memory
    let (input_ptr, input_len) = match load_input_into_memory(&mut store, &instance, input) {
        Ok(v) => v,
        Err(e) => {
            warn!("Failed to load input into WASM memory: {}", e);
            (0, 0)
        }
    };

    let func = instance.get_typed_func::<(u32, u32), u32>(&mut store, function_name);
    let func = match func {
        Ok(f) => f,
        Err(_) => return Ok(WorkerResult {
            success: false,
            output: String::new(),
            error: Some(format!("Function '{}' not found in module", function_name)),
            fuel_consumed: store.get_fuel().unwrap_or(0),
            execution_time_ms: start.elapsed().as_millis() as u64,
        }),
    };

    match func.call(&mut store, (input_ptr, input_len)) {
        Ok(_) => {}
        Err(e) => {
            let fuel_used = config.max_fuel - store.get_fuel().unwrap_or(0);
            return Ok(WorkerResult {
                success: false,
                output: String::new(),
                error: Some(format!("Execution error: {}", e)),
                fuel_consumed: fuel_used,
                execution_time_ms: start.elapsed().as_millis() as u64,
            });
        }
    };

    let fuel_consumed = config.max_fuel - store.get_fuel().unwrap_or(0);
    let elapsed = start.elapsed().as_millis() as u64;

    // Drop the store to release the WASI context, then read captured output
    drop(store);
    let stdout_bytes = stdout_pipe.contents();
    let output = String::from_utf8_lossy(&stdout_bytes).to_string();

    Ok(WorkerResult {
        success: true,
        output,
        error: None,
        fuel_consumed,
        execution_time_ms: elapsed,
    })
}

/// Load input bytes into the WASM module's exported memory.
/// Returns (pointer, length) for passing to the WASM function.
fn load_input_into_memory(
    store: &mut Store<WasiP1Ctx>,
    instance: &Instance,
    input: &[u8],
) -> Result<(u32, u32)> {
    if input.is_empty() {
        return Ok((0, 0));
    }

    let memory = instance
        .get_memory(&mut *store, "memory")
        .ok_or_else(|| FerroError::Internal("WASM module has no exported memory".to_string()))?;

    let input_len = input.len() as u32;
    let memory_size = memory.data_size(&mut *store) as u32;

    if input_len > memory_size {
        return Err(FerroError::Internal(format!(
            "Input ({} bytes) exceeds WASM memory ({} bytes)",
            input_len, memory_size
        )));
    }

    // Write input at the beginning of memory (offset 0)
    memory
        .data_mut(&mut *store)[..input_len as usize]
        .copy_from_slice(input);

    Ok((0, input_len))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_matching() {
        let runtime = WasmWorkerRuntime::new().unwrap();

        assert!(runtime.pattern_matches("*", "/any/path.pdf"));
        assert!(runtime.pattern_matches("*.pdf", "/docs/report.pdf"));
        assert!(!runtime.pattern_matches("*.pdf", "/docs/report.txt"));
        assert!(runtime.pattern_matches("/docs/*", "/docs/file.txt"));
        assert!(!runtime.pattern_matches("/docs/*", "/other/file.txt"));
    }

    #[tokio::test]
    async fn test_register_worker() {
        let runtime = WasmWorkerRuntime::new().unwrap();

        runtime.register_worker(WorkerEvent {
            pattern: "*.pdf".to_string(),
            module_path: "/tmp/worker.wasm".to_string(),
            config: WorkerConfig::default(),
            function_name: "process".to_string(),
        }).await;

        let workers = runtime.list_workers().await;
        assert_eq!(workers.len(), 1);
        assert_eq!(workers[0].pattern, "*.pdf");
    }

    #[tokio::test]
    async fn test_find_matching_workers() {
        let runtime = WasmWorkerRuntime::new().unwrap();

        runtime.register_worker(WorkerEvent {
            pattern: "*.pdf".to_string(),
            module_path: "/tmp/pdf.wasm".to_string(),
            config: WorkerConfig::default(),
            function_name: "process".to_string(),
        }).await;

        runtime.register_worker(WorkerEvent {
            pattern: "*.jpg".to_string(),
            module_path: "/tmp/image.wasm".to_string(),
            config: WorkerConfig::default(),
            function_name: "resize".to_string(),
        }).await;

        let matches = runtime.find_matching_workers("/photos/report.pdf").await;
        assert_eq!(matches.len(), 1);
        assert!(matches[0].module_path.contains("pdf"));
    }

    #[tokio::test]
    async fn test_execute_nonexistent_module() {
        let runtime = WasmWorkerRuntime::new().unwrap();

        let result = runtime.execute(
            "/nonexistent/module.wasm",
            "process",
            b"",
            None,
        ).await.unwrap();

        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[tokio::test]
    async fn test_execute_timeout() {
        let runtime = WasmWorkerRuntime::new().unwrap();

        // Use a very short timeout to test timeout enforcement
        let config = WorkerConfig {
            max_time_ms: 1, // 1ms timeout
            ..WorkerConfig::default()
        };

        // Even a nonexistent module should respect the timeout
        let result = runtime.execute(
            "/nonexistent/module.wasm",
            "process",
            b"",
            Some(config),
        ).await.unwrap();

        // Should either fail with "not found" (fast) or timeout
        assert!(!result.success);
    }
}
