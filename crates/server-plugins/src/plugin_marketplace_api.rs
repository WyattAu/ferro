use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};

use crate::PluginState;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum PluginStatus {
    Available,
    Installed,
    Enabled,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MarketplacePlugin {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub category: String,
    pub rating: f64,
    pub downloads: u64,
    pub status: PluginStatus,
    pub changelog: String,
    pub permissions: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MarketplaceResponse {
    pub plugins: Vec<MarketplacePlugin>,
}

fn mock_plugins() -> Vec<MarketplacePlugin> {
    vec![
        MarketplacePlugin {
            id: "pdf-preview".into(),
            name: "PDF Preview".into(),
            version: "1.2.0".into(),
            author: "Ferro Team".into(),
            description: "Render PDF files inline in the browser with zoom and page navigation.".into(),
            category: "Productivity".into(),
            rating: 4.8,
            downloads: 12_340,
            status: PluginStatus::Available,
            changelog: "## 1.2.0\n- Added text selection in preview\n- Fixed rendering on Safari\n\n## 1.1.0\n- Added thumbnail sidebar".into(),
            permissions: vec!["read_files".into()],
        },
        MarketplacePlugin {
            id: "image-compress".into(),
            name: "Image Compressor".into(),
            version: "2.0.1".into(),
            author: "Community".into(),
            description: "Automatically compress uploaded images using WASM-based WebP conversion.".into(),
            category: "Media".into(),
            rating: 4.5,
            downloads: 8_920,
            status: PluginStatus::Installed,
            changelog: "## 2.0.1\n- Fixed EXIF orientation handling\n\n## 2.0.0\n- Rewritten in WASM for performance".into(),
            permissions: vec!["read_files".into(), "write_files".into()],
        },
        MarketplacePlugin {
            id: "antivirus-scan".into(),
            name: "Antivirus Scanner".into(),
            version: "3.1.0".into(),
            author: "Security Labs".into(),
            description: "Scan uploaded files for malware using ClamAV integration.".into(),
            category: "Security".into(),
            rating: 4.9,
            downloads: 22_100,
            status: PluginStatus::Enabled,
            changelog: "## 3.1.0\n- Incremental scan for large files\n- Updated signature database\n\n## 3.0.0\n- Async scanning pipeline".into(),
            permissions: vec!["read_files".into(), "network".into()],
        },
        MarketplacePlugin {
            id: "markdown-editor".into(),
            name: "Markdown Editor".into(),
            version: "1.0.0".into(),
            author: "Ferro Team".into(),
            description: "WYSIWYG Markdown editor with live preview, syntax highlighting, and export.".into(),
            category: "Productivity".into(),
            rating: 4.3,
            downloads: 5_670,
            status: PluginStatus::Available,
            changelog: "## 1.0.0\n- Initial release\n- Full CommonMark support".into(),
            permissions: vec!["read_files".into(), "write_files".into()],
        },
        MarketplacePlugin {
            id: "video-transcode".into(),
            name: "Video Transcoder".into(),
            version: "0.9.2".into(),
            author: "MediaForge".into(),
            description: "Transcode video files to web-friendly formats using server-side FFmpeg.".into(),
            category: "Media".into(),
            rating: 3.8,
            downloads: 2_340,
            status: PluginStatus::Available,
            changelog: "## 0.9.2\n- Fixed audio sync issues\n\n## 0.9.0\n- Added HLS output support".into(),
            permissions: vec!["read_files".into(), "write_files".into(), "network".into()],
        },
        MarketplacePlugin {
            id: "audit-report".into(),
            name: "Audit Report Generator".into(),
            version: "1.4.0".into(),
            author: "Compliance.io".into(),
            description: "Generate PDF compliance reports from audit logs for SOC2 and GDPR.".into(),
            category: "Compliance".into(),
            rating: 4.6,
            downloads: 6_780,
            status: PluginStatus::Installed,
            changelog: "## 1.4.0\n- GDPR article mapping\n\n## 1.3.0\n- SOC2 Type II template".into(),
            permissions: vec!["read_files".into(), "admin_api".into()],
        },
    ]
}

pub async fn list_marketplace_plugins<S: PluginState>(State(_state): State<S>) -> Response {
    let plugins = mock_plugins();
    (StatusCode::OK, axum::Json(MarketplaceResponse { plugins })).into_response()
}

pub async fn install_plugin<S: PluginState>(
    State(_state): State<S>,
    Path(id): Path<String>,
) -> Response {
    tracing::info!(plugin_id = %id, "install plugin requested");
    (
        StatusCode::OK,
        axum::Json(serde_json::json!({
            "success": true,
            "plugin_id": id,
            "action": "install",
        })),
    )
        .into_response()
}

pub async fn uninstall_plugin<S: PluginState>(
    State(_state): State<S>,
    Path(id): Path<String>,
) -> Response {
    tracing::info!(plugin_id = %id, "uninstall plugin requested");
    (
        StatusCode::OK,
        axum::Json(serde_json::json!({
            "success": true,
            "plugin_id": id,
            "action": "uninstall",
        })),
    )
        .into_response()
}

pub async fn enable_plugin<S: PluginState>(
    State(_state): State<S>,
    Path(id): Path<String>,
) -> Response {
    tracing::info!(plugin_id = %id, "enable plugin requested");
    (
        StatusCode::OK,
        axum::Json(serde_json::json!({
            "success": true,
            "plugin_id": id,
            "action": "enable",
        })),
    )
        .into_response()
}

pub async fn disable_plugin<S: PluginState>(
    State(_state): State<S>,
    Path(id): Path<String>,
) -> Response {
    tracing::info!(plugin_id = %id, "disable plugin requested");
    (
        StatusCode::OK,
        axum::Json(serde_json::json!({
            "success": true,
            "plugin_id": id,
            "action": "disable",
        })),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_plugins_count() {
        let plugins = mock_plugins();
        assert_eq!(plugins.len(), 6);
    }

    #[test]
    fn test_mock_plugins_have_required_fields() {
        let plugins = mock_plugins();
        for p in &plugins {
            assert!(!p.id.is_empty());
            assert!(!p.name.is_empty());
            assert!(!p.version.is_empty());
            assert!(!p.author.is_empty());
            assert!(!p.description.is_empty());
            assert!(!p.category.is_empty());
            assert!(p.rating >= 0.0 && p.rating <= 5.0);
        }
    }

    #[test]
    fn test_marketplace_response_serde() {
        let plugins = mock_plugins();
        let resp = MarketplaceResponse { plugins };
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: MarketplaceResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.plugins.len(), 6);
    }

    #[test]
    fn test_plugin_status_serde_roundtrip() {
        let statuses = vec![
            PluginStatus::Available,
            PluginStatus::Installed,
            PluginStatus::Enabled,
        ];
        let json = serde_json::to_string(&statuses).unwrap();
        let parsed: Vec<PluginStatus> = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, statuses);
    }
}
