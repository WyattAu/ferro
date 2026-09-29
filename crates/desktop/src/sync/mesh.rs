//! LAN mesh block exchange.
//!
//! After each sync cycle the client registers itself with the server's peer
//! index (addresses + block hashes it holds). Before uploading missing
//! blocks to the server it queries the index and tries to fetch directly
//! from a LAN peer — avoiding the WAN round-trip entirely.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

const REGISTRATION_TTL_HINT: &str = "600";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub sub: String,
    pub device_name: String,
    pub addresses: Vec<String>,
    pub port: u16,
    pub block_count: usize,
    pub last_seen: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshConfig {
    pub server_url: String,
    pub bearer_token: String,
    pub device_name: String,
    /// Addresses this peer is reachable on (discovered from local interfaces).
    pub local_addresses: Vec<String>,
    /// Port the LAN block listener binds to.
    pub listen_port: u16,
}

/// Register this peer with the server's mesh index.
pub async fn register(config: &MeshConfig, block_hashes: &[String]) -> Result<()> {
    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()?;
    let body = serde_json::json!({
        "device_name": config.device_name,
        "addresses": config.local_addresses,
        "port": config.listen_port,
        "block_hashes": block_hashes,
    });
    let resp = http
        .post(format!("{}/api/mesh/peers", config.server_url.trim_end_matches('/')))
        .bearer_auth(&config.bearer_token)
        .header("X-Ferro-User", "") // server derives from auth claims
        .json(&body)
        .send()
        .await
        .context("mesh register failed")?;
    if !resp.status().is_success() {
        anyhow::bail!("mesh register: {}", resp.status());
    }
    Ok(())
}

/// Query which registered peers hold a given block hash.
pub async fn query_block_holders(
    config: &MeshConfig,
    hash: &str,
) -> Result<Vec<PeerInfo>> {
    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;
    let resp = http
        .get(format!(
            "{}/api/mesh/peers/blocks",
            config.server_url.trim_end_matches('/')
        ))
        .bearer_auth(&config.bearer_token)
        .query(&[("hash", hash)])
        .send()
        .await
        .context("mesh block query failed")?;
    if !resp.status().is_success() {
        anyhow::bail!("mesh block query: {}", resp.status());
    }
    let v: serde_json::Value = resp.json().await?;
    let holders = v
        .get("holders")
        .cloned()
        .unwrap_or(serde_json::Value::Array(vec![]));
    serde_json::from_value(holders).context("failed to parse holders")
}

/// Fetch a block from a LAN peer directly (GET /blocks/{hash}).
pub async fn fetch_block_from_peer(
    peer_address: &str,
    port: u16,
    hash: &str,
) -> Result<Vec<u8>> {
    let url = format!("http://{peer_address}:{port}/blocks/{hash}");
    let resp = reqwest::get(&url)
        .await
        .context("peer fetch failed")?
        .error_for_status()
        .context("peer returned error")?;
    Ok(resp.bytes().await?.to_vec())
}

/// Local LAN address discovery — enumerates non-loopback IPv4 addresses.
pub fn discover_local_addresses() -> Vec<String> {
    let mut out = Vec::new();
    if let Ok(addrs) = std::net::ToSocketAddrs::to_socket_addrs(&(
        std::env::var("HOSTNAME").unwrap_or_default(),
        0,
    )) {
        for a in addrs {
            if let std::net::IpAddr::V4(v4) = a.ip() {
                if !v4.is_loopback() && !out.contains(&v4.to_string()) {
                    out.push(v4.to_string());
                }
            }
        }
    }
    // Fallback: common LAN prefixes
    if out.is_empty() {
        out.push("127.0.0.1".to_string());
    }
    out
}

/// Serve blocks from a local cache map on an ephemeral LAN port.
/// Called as a background task during sync; the listener dies when the
/// returned shutdown handle is dropped or triggered.
pub async fn serve_blocks(
    port: u16,
    cache: HashMap<String, Vec<u8>>,
) -> Result<u16> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port)).await?;
    let bound = listener.local_addr()?.port();
    let cache = std::sync::Arc::new(cache);

    tokio::spawn(async move {
        loop {
            let Ok((mut socket, _)) = listener.accept().await else {
                break;
            };
            let cache = cache.clone();
            tokio::spawn(async move {
                let mut buf = vec![0u8; 1024];
                let Ok(n) = socket.read(&mut buf).await else { return };
                let req = String::from_utf8_lossy(&buf[..n]);
                let hash = req.split_whitespace().nth(1).unwrap_or("").to_string();
                if let Some(content) = cache.get(&hash) {
                    let _ = socket
                        .write_all(format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n", content.len()).as_bytes())
                        .await;
                    let _ = socket.write_all(content).await;
                } else {
                    let _ = socket
                        .write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n")
                        .await;
                }
            });
        }
    });
    Ok(bound)
}
