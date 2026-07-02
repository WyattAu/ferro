use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};

use crate::PluginState;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginCapabilities {
    pub read_files: bool,
    pub write_files: bool,
    pub network: bool,
    pub admin_api: bool,
}

impl PluginCapabilities {
    pub fn all() -> Self {
        Self {
            read_files: true,
            write_files: true,
            network: true,
            admin_api: true,
        }
    }

    pub fn none() -> Self {
        Self::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub module_path: String,
    #[serde(default)]
    pub capabilities: PluginCapabilities,
}

pub async fn list_plugins<S: PluginState>(State(state): State<S>) -> Response {
    let mut plugins: Vec<serde_json::Value> = state
        .plugin_registry()
        .iter()
        .map(|entry| {
            let manifest = entry.value();
            serde_json::json!({
                "name": manifest.name,
                "version": manifest.version,
                "module_path": manifest.module_path,
                "capabilities": manifest.capabilities,
            })
        })
        .collect();

    plugins.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));

    (
        StatusCode::OK,
        axum::Json(serde_json::json!({
            "plugins": plugins,
        })),
    )
        .into_response()
}
