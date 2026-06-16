//! ClamAV antivirus scanning API endpoints.
//!
//! Connects to ClamAV daemon via TCP socket (default 127.0.0.1:3310)
//! using the INSTREAM protocol for file scanning.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::RwLock;

use crate::AppState;

/// Maximum number of scan results to retain in history.
const MAX_SCAN_HISTORY: usize = 100;

/// In-memory scan history store.
pub struct ScanHistory {
    results: Arc<RwLock<VecDeque<ScanResultEntry>>>,
}

impl ScanHistory {
    pub fn new() -> Self {
        Self {
            results: Arc::new(RwLock::new(VecDeque::new())),
        }
    }

    pub async fn record(&self, entry: ScanResultEntry) {
        let mut results = self.results.write().await;
        if results.len() >= MAX_SCAN_HISTORY {
            results.pop_front();
        }
        results.push_back(entry);
    }

    pub async fn list(&self) -> Vec<ScanResultEntry> {
        let results = self.results.read().await;
        results.iter().cloned().collect()
    }
}

impl Default for ScanHistory {
    fn default() -> Self {
        Self::new()
    }
}

/// A single scan result entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResultEntry {
    pub id: String,
    pub file_path: String,
    pub clean: bool,
    pub threat_name: Option<String>,
    pub scan_time_ms: u64,
    pub scanned_at: String,
    pub file_size: u64,
}

/// Response body for antivirus status.
#[derive(Debug, Serialize)]
pub struct AntivirusStatusResponse {
    pub connected: bool,
    pub host: String,
    pub port: u16,
    pub total_scans: usize,
    pub infected_count: usize,
    pub clean_count: usize,
}

/// Request body for scanning a directory.
#[derive(Debug, Deserialize)]
pub struct ScanDirectoryRequest {
    pub directory: String,
}

/// Response body for a single file scan.
#[derive(Debug, Serialize)]
pub struct ScanFileResponse {
    pub file_path: String,
    pub clean: bool,
    pub threat_name: Option<String>,
    pub scan_time_ms: u64,
    pub scanned_at: String,
}

/// ClamAV TCP scanner configuration.
#[derive(Debug, Clone)]
pub struct ClamavTcpConfig {
    pub host: String,
    pub port: u16,
    pub timeout_ms: u64,
}

impl Default for ClamavTcpConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 3310,
            timeout_ms: 30_000,
        }
    }
}

/// Scan a file via ClamAV TCP INSTREAM protocol.
async fn scan_file_tcp(
    config: &ClamavTcpConfig,
    _file_path: &str,
    content: &[u8],
) -> Result<(bool, Option<String>, u64), String> {
    let start = std::time::Instant::now();

    let addr = format!("{}:{}", config.host, config.port);
    let mut stream = tokio::time::timeout(
        std::time::Duration::from_millis(config.timeout_ms),
        tokio::net::TcpStream::connect(&addr),
    )
    .await
    .map_err(|_| format!("Timeout connecting to ClamAV at {}", addr))?
    .map_err(|e| format!("Failed to connect to ClamAV at {}: {}", addr, e))?;

    // Send INSTREAM command
    stream
        .write_all(b"zINSTREAM\0")
        .await
        .map_err(|e| format!("Failed to send INSTREAM command: {}", e))?;

    // Send chunk length + data
    let len_bytes = (content.len() as u32).to_be_bytes();
    stream
        .write_all(&len_bytes)
        .await
        .map_err(|e| format!("Failed to write chunk length: {}", e))?;
    stream
        .write_all(content)
        .await
        .map_err(|e| format!("Failed to write chunk data: {}", e))?;

    // Send zero-length terminator
    stream
        .write_all(&0u32.to_be_bytes())
        .await
        .map_err(|e| format!("Failed to send end-of-stream marker: {}", e))?;

    // Read response
    let mut response = Vec::new();
    tokio::time::timeout(
        std::time::Duration::from_millis(config.timeout_ms),
        stream.read_to_end(&mut response),
    )
    .await
    .map_err(|_| "Timeout reading ClamAV response".to_string())?
    .map_err(|e| format!("Failed to read ClamAV response: {}", e))?;

    let scan_time_ms = start.elapsed().as_millis() as u64;
    let response_str = String::from_utf8(response)
        .map_err(|_| "Invalid UTF-8 in ClamAV response".to_string())?;
    let response_str = response_str.trim_end_matches('\0');

    if response_str.ends_with("OK") {
        Ok((true, None, scan_time_ms))
    } else if response_str.ends_with("FOUND") {
        let virus_name = response_str
            .strip_prefix("stream: ")
            .and_then(|s| s.strip_suffix(" FOUND"))
            .map(|s| s.to_string());
        Ok((false, virus_name, scan_time_ms))
    } else if response_str.contains("ERROR") {
        Err(format!("ClamAV scan error: {}", response_str))
    } else {
        Err(format!("Unexpected ClamAV response: {}", response_str))
    }
}

/// Check ClamAV connection via PING/PONG.
async fn check_connection(config: &ClamavTcpConfig) -> bool {
    let addr = format!("{}:{}", config.host, config.port);
    let stream = tokio::time::timeout(
        std::time::Duration::from_millis(5000),
        tokio::net::TcpStream::connect(&addr),
    )
    .await;

    match stream {
        Ok(Ok(mut s)) => {
            let _ = s.write_all(b"zPING\0").await;
            let mut buf = vec![0u8; 64];
            match tokio::time::timeout(std::time::Duration::from_secs(5), s.read(&mut buf)).await {
                Ok(Ok(n)) => {
                    let resp = String::from_utf8_lossy(&buf[..n]);
                    resp.contains("PONG")
                }
                _ => false,
            }
        }
        _ => false,
    }
}

/// POST /api/antivirus/scan/{path} — Scan a file.
pub async fn scan_file(
    State(state): State<AppState>,
    Path(file_path): Path<String>,
) -> Response {
    let content = match state.storage.get(&file_path).await {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": format!("File not found: {}", e),
                })),
            )
                .into_response();
        }
    };

    let config = ClamavTcpConfig::default();
    let scanned_at = chrono::Utc::now().to_rfc3339();
    let file_size = content.len() as u64;

    match scan_file_tcp(&config, &file_path, &content).await {
        Ok((clean, threat_name, scan_time_ms)) => {
            let entry = ScanResultEntry {
                id: uuid::Uuid::new_v4().to_string(),
                file_path: file_path.clone(),
                clean,
                threat_name: threat_name.clone(),
                scan_time_ms,
                scanned_at: scanned_at.clone(),
                file_size,
            };

            let history = ScanHistory::new();
            history.record(entry).await;

            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "file_path": file_path,
                    "clean": clean,
                    "threat_name": threat_name,
                    "scan_time_ms": scan_time_ms,
                    "scanned_at": scanned_at,
                    "file_size": file_size,
                })),
            )
                .into_response()
        }
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({
                "error": format!("ClamAV scan failed: {}", e),
            })),
        )
            .into_response(),
    }
}

/// GET /api/antivirus/status — ClamAV connection status.
pub async fn antivirus_status(State(_state): State<AppState>) -> Response {
    let config = ClamavTcpConfig::default();
    let connected = check_connection(&config).await;

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "connected": connected,
            "host": config.host,
            "port": config.port,
            "timeout_ms": config.timeout_ms,
        })),
    )
        .into_response()
}

/// POST /api/antivirus/scan-all — Scan all files in a directory.
pub async fn scan_all(
    State(state): State<AppState>,
    Json(req): Json<ScanDirectoryRequest>,
) -> Response {
    let prefix = if req.directory.is_empty() {
        "/".to_string()
    } else if req.directory.ends_with('/') {
        req.directory.clone()
    } else {
        format!("{}/", req.directory)
    };

    let entries = match state.storage.list_all(&prefix, 10000).await {
        Ok(e) => e,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": format!("Failed to list directory: {}", e),
                })),
            )
                .into_response();
        }
    };

    let config = ClamavTcpConfig::default();
    let mut results = Vec::new();
    let mut scanned = 0u32;
    let mut infected = 0u32;
    let mut errors = 0u32;

    for meta in &entries {
        if meta.is_collection {
            continue;
        }

        let content = match state.storage.get(&meta.path).await {
            Ok(c) => c,
            Err(_) => {
                errors += 1;
                continue;
            }
        };

        let scanned_at = chrono::Utc::now().to_rfc3339();
        match scan_file_tcp(&config, &meta.path, &content).await {
            Ok((clean, threat_name, scan_time_ms)) => {
                results.push(serde_json::json!({
                    "file_path": meta.path,
                    "clean": clean,
                    "threat_name": threat_name,
                    "scan_time_ms": scan_time_ms,
                    "scanned_at": scanned_at,
                    "file_size": content.len() as u64,
                }));
                scanned += 1;
                if !clean {
                    infected += 1;
                }
            }
            Err(e) => {
                results.push(serde_json::json!({
                    "file_path": meta.path,
                    "error": e,
                }));
                errors += 1;
            }
        }
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "directory": req.directory,
            "scanned": scanned,
            "infected": infected,
            "clean": scanned - infected,
            "errors": errors,
            "results": results,
        })),
    )
        .into_response()
}

/// GET /api/antivirus/history — Scan history with results.
pub async fn scan_history(State(_state): State<AppState>) -> Response {
    let history = ScanHistory::new();
    let entries = history.list().await;
    let total = entries.len();
    let infected = entries.iter().filter(|e| !e.clean).count();

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "history": entries,
            "total": total,
            "infected_count": infected,
            "clean_count": total - infected,
        })),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AppState;

    #[tokio::test]
    async fn test_antivirus_status() {
        let state = AppState::in_memory();
        let response = antivirus_status(State(state)).await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_scan_file_not_found() {
        let state = AppState::in_memory();
        let response = scan_file(State(state), Path("/nonexistent.txt".to_string())).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_scan_history_empty() {
        let state = AppState::in_memory();
        let response = scan_history(State(state)).await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_scan_all_empty_directory() {
        let state = AppState::in_memory();
        let response = scan_all(
            State(state),
            Json(ScanDirectoryRequest {
                directory: "/".to_string(),
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn test_clamav_config_defaults() {
        let config = ClamavTcpConfig::default();
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 3310);
        assert_eq!(config.timeout_ms, 30000);
    }

    #[tokio::test]
    async fn test_check_connection_no_daemon() {
        let config = ClamavTcpConfig::default();
        assert!(!check_connection(&config).await);
    }
}
