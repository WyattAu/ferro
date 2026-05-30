//! ClamAV antivirus scanning WASM worker skeleton (G-15).
//!
//! Defines the interface for a ClamAV scanning worker. The actual scanning
//! is performed by a WASM module that communicates with a ClamAV daemon via
//! TCP socket (e.g. a Unix domain socket at `/var/run/clamav/clamd.sock`).

use serde::{Deserialize, Serialize};

/// Configuration for the ClamAV scanning worker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClamavConfig {
    /// Whether ClamAV scanning is enabled.
    pub enabled: bool,
    /// Path to the ClamAV daemon socket.
    pub socket_path: String,
    /// Maximum file size (in bytes) that will be scanned.
    pub max_file_size: u64,
    /// Timeout for the ClamAV daemon connection in milliseconds.
    pub timeout_ms: u64,
}

impl Default for ClamavConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            socket_path: "/var/run/clamav/clamd.sock".to_string(),
            max_file_size: 26_214_400,
            timeout_ms: 30_000,
        }
    }
}

/// Request payload for a file scan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClamavScanRequest {
    /// Absolute path to the file to scan.
    pub file_path: String,
    /// Pre-computed hash of the file contents (e.g. SHA-256).
    pub file_hash: String,
}

/// Result of a ClamAV file scan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClamavScanResult {
    /// `true` when no threat was detected.
    pub clean: bool,
    /// Name of the detected threat, if any.
    pub threat_name: Option<String>,
    /// Wall-clock time the scan took in milliseconds.
    pub scan_time_ms: u64,
}

/// Scan a file using the ClamAV daemon.
///
/// TODO: Replace the stub with actual ClamAV daemon communication.
///       The WASM worker should open a TCP connection to the configured
///       socket, send the INSTREAM command, stream the file contents,
///       and parse the response for virus signatures.
pub async fn scan_file(
    config: &ClamavConfig,
    request: &ClamavScanRequest,
    file_size: u64,
) -> Result<ClamavScanResult, String> {
    if !config.enabled {
        return Err("ClamAV scanning is disabled".to_string());
    }

    if file_size > config.max_file_size {
        return Err(format!(
            "File size {} exceeds maximum scan size {}",
            file_size, config.max_file_size
        ));
    }

    // TODO: Connect to ClamAV daemon via TCP socket and perform actual scan.
    let _ = (&config.socket_path, &request.file_path, &request.file_hash);

    Ok(ClamavScanResult {
        clean: true,
        threat_name: None,
        scan_time_ms: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_scan_file_clean() {
        let config = ClamavConfig {
            enabled: true,
            ..Default::default()
        };
        let request = ClamavScanRequest {
            file_path: "/tmp/testfile.txt".to_string(),
            file_hash: "abc123".to_string(),
        };

        let result = scan_file(&config, &request, 1024).await.unwrap();
        assert!(result.clean);
        assert!(result.threat_name.is_none());
        assert_eq!(result.scan_time_ms, 0);
    }

    #[tokio::test]
    async fn test_scan_file_disabled() {
        let config = ClamavConfig::default();
        let request = ClamavScanRequest {
            file_path: "/tmp/testfile.txt".to_string(),
            file_hash: "abc123".to_string(),
        };

        let err = scan_file(&config, &request, 1024).await.unwrap_err();
        assert!(err.contains("disabled"));
    }
}
