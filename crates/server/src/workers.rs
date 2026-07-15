use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};

use crate::AppState;
use ferro_server_state::ServerState as _;

/// Core logic for listing WASM workers.
async fn list_workers_impl<S: ferro_server_state::ServerState>(state: &S) -> Response {
    match state.wasm_runtime() {
        Some(runtime) => {
            let workers = runtime.list_workers().await;
            let items: Vec<serde_json::Value> = workers
                .into_iter()
                .map(|w| {
                    serde_json::json!({
                        "pattern": w.pattern,
                        "module_path": w.module_path,
                        "function_name": w.function_name,
                        "max_fuel": w.config.max_fuel,
                        "max_memory_bytes": w.config.max_memory_bytes,
                    })
                })
                .collect();

            let body = serde_json::json!({
                "workers": items,
            });
            (StatusCode::OK, axum::Json(body)).into_response()
        }
        None => {
            let body = serde_json::json!({
                "workers": [],
                "configured": false,
            });
            (StatusCode::OK, axum::Json(body)).into_response()
        }
    }
}

/// GET /api/workers — list registered WASM workers.
pub async fn list_workers(State(state): State<AppState>) -> Response {
    list_workers_impl(&state).await
}

/// POST /api/workers — register a new WASM worker.
/// Request body for registering a WASM worker.
#[derive(Debug, Deserialize)]
pub struct RegisterWorkerRequest {
    pub pattern: String,
    pub module_path: String,
    pub function_name: String,
    pub max_fuel: Option<u64>,
    pub max_memory_bytes: Option<usize>,
}

/// Response after registering a WASM worker.
#[derive(Debug, Serialize)]
pub struct RegisterWorkerResponse {
    pub status: String,
    pub pattern: String,
    pub module_path: String,
    pub function_name: String,
}

/// Core logic for registering a WASM worker.
async fn register_worker_impl<S: ferro_server_state::ServerState>(state: &S, req: RegisterWorkerRequest) -> Response {
    match state.wasm_runtime() {
        Some(runtime) => {
            let event = ferro_core::wasm::WorkerEvent {
                pattern: req.pattern.clone(),
                module_path: req.module_path.clone(),
                function_name: req.function_name.clone(),
                config: ferro_core::wasm::WorkerConfig {
                    max_time_ms: 30_000,
                    max_memory_bytes: req.max_memory_bytes.unwrap_or(64 * 1024 * 1024),
                    max_fuel: req.max_fuel.unwrap_or(1_000_000_000),
                    allowed_paths: vec![],
                },
            };

            runtime.register_worker(event).await;

            let body = RegisterWorkerResponse {
                status: "registered".to_string(),
                pattern: req.pattern,
                module_path: req.module_path,
                function_name: req.function_name,
            };
            (StatusCode::CREATED, axum::Json(body)).into_response()
        }
        None => {
            let body = serde_json::json!({
                "error": "WASM worker runtime is not configured",
            });
            (StatusCode::SERVICE_UNAVAILABLE, axum::Json(body)).into_response()
        }
    }
}

/// POST /api/workers — register a new WASM worker.
pub async fn register_worker(
    State(state): State<AppState>,
    axum::Json(req): axum::Json<RegisterWorkerRequest>,
) -> Response {
    register_worker_impl(&state, req).await
}
