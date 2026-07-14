use serde::{Deserialize, Serialize};
#[cfg(unix)]
use tokio::io::{AsyncReadExt, AsyncWriteExt};
#[cfg(unix)]
use tokio::net::UnixStream;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClamavConfig {
    pub enabled: bool,
    pub socket_path: String,
    pub max_file_size: u64,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClamavScanRequest {
    pub file_path: String,
    pub file_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClamavScanResult {
    pub clean: bool,
    pub threat_name: Option<String>,
    pub scan_time_ms: u64,
}

const CHUNK_SIZE: usize = 4096;

#[cfg(unix)]
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

    let scan_start = std::time::Instant::now();

    let stream = tokio::time::timeout(
        std::time::Duration::from_millis(config.timeout_ms),
        UnixStream::connect(&config.socket_path),
    )
    .await
    .map_err(|_| format!("Timeout connecting to ClamAV daemon at {}", config.socket_path))?
    .map_err(|e| format!("Failed to connect to ClamAV daemon: {e}"))?;

    let (mut reader, mut writer) = stream.into_split();

    writer
        .write_all(b"zINSTREAM\0")
        .await
        .map_err(|e| format!("Failed to send INSTREAM command: {e}"))?;

    let mut file = tokio::fs::File::open(&request.file_path)
        .await
        .map_err(|e| format!("Failed to open file for scanning: {e}"))?;

    let mut buffer = vec![0u8; CHUNK_SIZE];
    loop {
        let n = file
            .read(&mut buffer)
            .await
            .map_err(|e| format!("Failed to read file for scanning: {e}"))?;

        if n == 0 {
            break;
        }

        let len_bytes = (n as u32).to_be_bytes();
        writer
            .write_all(&len_bytes)
            .await
            .map_err(|e| format!("Failed to write chunk length: {e}"))?;
        writer
            .write_all(&buffer[..n])
            .await
            .map_err(|e| format!("Failed to write chunk data: {e}"))?;
    }

    writer
        .write_all(&0u32.to_be_bytes())
        .await
        .map_err(|e| format!("Failed to send end-of-stream marker: {e}"))?;

    let mut response = Vec::new();
    tokio::time::timeout(
        std::time::Duration::from_millis(config.timeout_ms),
        reader.read_to_end(&mut response),
    )
    .await
    .map_err(|_| "Timeout reading ClamAV scan response".to_string())?
    .map_err(|e| format!("Failed to read ClamAV response: {e}"))?;

    let scan_time_ms = scan_start.elapsed().as_millis() as u64;

    let response_str = String::from_utf8(response).map_err(|_| "Invalid UTF-8 in ClamAV response".to_string())?;
    let response_str = response_str.trim_end_matches('\0');

    if response_str.ends_with("OK") {
        Ok(ClamavScanResult {
            clean: true,
            threat_name: None,
            scan_time_ms,
        })
    } else if response_str.ends_with("FOUND") {
        let virus_name = response_str
            .strip_prefix("stream: ")
            .and_then(|s| s.strip_suffix(" FOUND"))
            .map(std::string::ToString::to_string);
        Ok(ClamavScanResult {
            clean: false,
            threat_name: virus_name,
            scan_time_ms,
        })
    } else if response_str.contains("ERROR") {
        Err(format!("ClamAV scan error: {response_str}"))
    } else {
        Err(format!("Unexpected ClamAV response: {response_str}"))
    }
}

#[cfg(test)]
#[cfg(unix)]
mod tests {
    use super::*;

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

    #[tokio::test]
    async fn test_scan_file_too_large() {
        let config = ClamavConfig {
            enabled: true,
            max_file_size: 100,
            ..Default::default()
        };
        let request = ClamavScanRequest {
            file_path: "/tmp/testfile.txt".to_string(),
            file_hash: "abc123".to_string(),
        };

        let err = scan_file(&config, &request, 1024).await.unwrap_err();
        assert!(err.contains("exceeds maximum scan size"));
    }

    #[tokio::test]
    async fn test_scan_file_no_daemon_running() {
        let config = ClamavConfig {
            enabled: true,
            socket_path: "/tmp/nonexistent-clamd.sock".to_string(),
            ..Default::default()
        };
        let request = ClamavScanRequest {
            file_path: "/tmp/testfile.txt".to_string(),
            file_hash: "abc123".to_string(),
        };

        let result = scan_file(&config, &request, 0).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_config_defaults() {
        let config = ClamavConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.socket_path, "/var/run/clamav/clamd.sock");
        assert_eq!(config.max_file_size, 26_214_400);
        assert_eq!(config.timeout_ms, 30_000);
    }
}
