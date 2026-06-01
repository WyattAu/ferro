use thiserror::Error;

#[derive(Error, Debug)]
pub enum WasmHostError {
    #[error("WASM compilation failed: {0}")]
    CompileFailed(String),

    #[error("WASM instantiation failed: {0}")]
    InstantiationFailed(String),

    #[error("WASM runtime error: {0}")]
    RuntimeError(String),

    #[error("Plugin execution timed out after {0:?}")]
    Timeout(std::time::Duration),

    #[error("Memory limit exceeded: requested {requested}, limit {limit}")]
    MemoryLimit { requested: u64, limit: u64 },

    #[error("Fuel exhausted")]
    FuelExhausted,

    #[error("Invalid plugin: {0}")]
    InvalidPlugin(String),

    #[error("Plugin not found: {0}")]
    NotFound(String),

    #[error("Plugin already loaded: {0}")]
    AlreadyLoaded(String),

    #[error("Serialization failed: {0}")]
    SerializationFailed(String),
}

impl From<wasmtime::Error> for WasmHostError {
    fn from(err: wasmtime::Error) -> Self {
        let msg = err.to_string();
        if msg.contains("fuel") || msg.contains("out of fuel") {
            WasmHostError::FuelExhausted
        } else if msg.contains("memory") {
            WasmHostError::MemoryLimit {
                requested: 0,
                limit: 0,
            }
        } else {
            WasmHostError::RuntimeError(msg)
        }
    }
}
