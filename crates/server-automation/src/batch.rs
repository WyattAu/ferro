use axum::extract::Extension;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use std::sync::Arc;

use crate::AutomationState;

#[derive(Debug, Deserialize)]
pub struct BatchCopyMoveRequest {
    pub operations: Vec<BatchOperation>,
}

#[derive(Debug, Deserialize)]
pub struct BatchOperation {
    pub from: String,
    pub to: String,
}

pub async fn batch_copy(
    Extension(state): Extension<Arc<AutomationState>>,
    axum::Json(body): axum::Json<BatchCopyMoveRequest>,
) -> Response {
    let mut results: Vec<serde_json::Value> = Vec::new();

    for op in &body.operations {
        let from = common::path::normalize_path(&op.from);
        let to = common::path::normalize_path(&op.to);

        if !common::path::validate_path(&from) || !common::path::validate_path(&to) {
            results.push(serde_json::json!({
                "from": op.from,
                "to": op.to,
                "status": "error",
                "error": "Invalid path",
            }));
            continue;
        }

        if from == to {
            results.push(serde_json::json!({
                "from": op.from,
                "to": op.to,
                "status": "error",
                "error": "Source and destination are the same",
            }));
            continue;
        }

        match state.storage.head(&from).await {
            Ok(_) => match state.storage.copy(&from, &to).await {
                Ok(()) => {
                    results.push(serde_json::json!({
                        "from": op.from,
                        "to": op.to,
                        "status": "ok",
                    }));
                }
                Err(e) => {
                    results.push(serde_json::json!({
                        "from": op.from,
                        "to": op.to,
                        "status": "error",
                        "error": e.to_string(),
                    }));
                }
            },
            Err(_) => {
                results.push(serde_json::json!({
                    "from": op.from,
                    "to": op.to,
                    "status": "error",
                    "error": "Source not found",
                }));
            }
        }
    }

    (StatusCode::OK, axum::Json(serde_json::json!({ "results": results }))).into_response()
}

pub async fn batch_move(
    Extension(state): Extension<Arc<AutomationState>>,
    axum::Json(body): axum::Json<BatchCopyMoveRequest>,
) -> Response {
    let mut results: Vec<serde_json::Value> = Vec::new();

    for op in &body.operations {
        let from = common::path::normalize_path(&op.from);
        let to = common::path::normalize_path(&op.to);

        if !common::path::validate_path(&from) || !common::path::validate_path(&to) {
            results.push(serde_json::json!({
                "from": op.from,
                "to": op.to,
                "status": "error",
                "error": "Invalid path",
            }));
            continue;
        }

        if from == to {
            results.push(serde_json::json!({
                "from": op.from,
                "to": op.to,
                "status": "error",
                "error": "Source and destination are the same",
            }));
            continue;
        }

        match state.storage.head(&from).await {
            Ok(_) => match state.storage.move_path(&from, &to).await {
                Ok(()) => {
                    results.push(serde_json::json!({
                        "from": op.from,
                        "to": op.to,
                        "status": "ok",
                    }));
                }
                Err(e) => {
                    results.push(serde_json::json!({
                        "from": op.from,
                        "to": op.to,
                        "status": "error",
                        "error": e.to_string(),
                    }));
                }
            },
            Err(_) => {
                results.push(serde_json::json!({
                    "from": op.from,
                    "to": op.to,
                    "status": "error",
                    "error": "Source not found",
                }));
            }
        }
    }

    (StatusCode::OK, axum::Json(serde_json::json!({ "results": results }))).into_response()
}
