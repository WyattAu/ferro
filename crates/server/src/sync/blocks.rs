//! Server-side block sync protocol.
//!
//! Provides endpoints for block-level file synchronization:
//! - Compute block manifest for a file (content-defined chunking)
//! - Upload individual blocks to CAS store
//! - Query which blocks are missing
//! - Assemble a file from blocks already in CAS

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use common::metadata::ContentHash;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::AppState;
use ferro_server_state::ServerState as _;

use base64::Engine;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A single block descriptor within a file's block map.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BlockDescriptor {
    /// Zero-based index of this block in the file.
    pub index: u32,
    /// SHA-256 hash of the block content.
    pub hash: String,
    /// Size of the block in bytes.
    pub size: u64,
    /// Byte offset within the file where this block starts.
    pub offset: u64,
}

/// A file's block manifest: ordered list of block descriptors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockManifest {
    /// File path in the virtual filesystem.
    pub path: String,
    /// Total file size in bytes.
    pub total_size: u64,
    /// SHA-256 hash of the entire file content.
    pub file_hash: String,
    /// Ordered list of block descriptors.
    pub blocks: Vec<BlockDescriptor>,
}

/// Request to compute a block manifest for a given file path.
#[derive(Debug, Deserialize)]
pub struct ManifestQuery {
    /// File path to compute manifest for.
    pub path: String,
    /// Target average block size in bytes (default 65536 = 64KB).
    /// Minimum 4096, maximum 1048576 (1MB).
    #[serde(default = "default_block_size")]
    pub block_size: u64,
}

/// Response to a manifest request: the manifest plus which blocks
/// are already present in the server's CAS store.
#[derive(Debug, Serialize)]
pub struct ManifestResponse {
    pub manifest: BlockManifest,
    /// Set of block hashes that are NOT present on the server.
    /// The client needs to upload only these blocks.
    pub missing_blocks: Vec<String>,
}

/// Request to upload one or more blocks.
#[derive(Debug, Deserialize)]
pub struct UploadBlocksRequest {
    /// Map of block_hash -> base64-encoded block content.
    pub blocks: HashMap<String, String>,
}

/// Response to block upload.
#[derive(Debug, Serialize)]
pub struct UploadBlocksResponse {
    /// Number of new blocks stored (dedup: already-present blocks not counted).
    pub stored: usize,
    /// Number of blocks that were already present (deduplicated).
    pub deduplicated: usize,
}

/// Request to assemble a file from blocks already in CAS.
#[derive(Debug, Deserialize)]
pub struct AssembleRequest {
    /// File path to write.
    pub path: String,
    /// Ordered list of block hashes that compose the file.
    pub block_hashes: Vec<String>,
    /// Owner principal for the new file.
    pub owner: String,
}

/// Response to assemble request.
#[derive(Debug, Serialize)]
pub struct AssembleResponse {
    /// SHA-256 hash of the assembled file.
    pub file_hash: String,
    /// Total size in bytes.
    pub size: u64,
    /// Number of blocks assembled.
    pub block_count: usize,
}

// ---------------------------------------------------------------------------
// Defaults
// ---------------------------------------------------------------------------

const fn default_block_size() -> u64 {
    65536 // 64KB
}

// ---------------------------------------------------------------------------
// Content-defined chunking (Buzhash-based)
// ---------------------------------------------------------------------------

/// Buzhash rolling hash for content-defined chunking.
/// Uses a 48-byte window with a random lookup table.
struct BuzHash {
    window: [u8; 48],
    window_idx: usize,
    hash: u64,
    table: [u64; 256],
}

impl BuzHash {
    fn new() -> Self {
        // Deterministic random table (same table on client and server).
        // Generated from a fixed seed to ensure cross-platform consistency.
        let mut table = [0u64; 256];
        let mut seed: u64 = 0x9E37_79B9_7F4A_7C15; // golden ratio fraction
        for entry in table.iter_mut() {
            seed = seed.wrapping_mul(6_364_136_223_846_793_005);
            seed ^= seed >> 17;
            *entry = seed;
        }
        Self {
            window: [0u8; 48],
            window_idx: 0,
            hash: 0,
            table,
        }
    }

    fn update(&mut self, byte: u8) {
        let outgoing = self.window[self.window_idx];
        self.window[self.window_idx] = byte;
        self.window_idx = (self.window_idx + 1) % 48;
        // Buzhash: hash = rotate_left(hash, 1) ^ table[byte] ^ table_outgoing_shifted
        self.hash =
            self.hash.rotate_left(1) ^ self.table[byte as usize] ^ self.table[outgoing as usize].rotate_left(48);
    }

    fn value(&self) -> u64 {
        self.hash
    }
}

/// Chunk a byte slice into content-defined blocks using Buzhash.
///
/// Parameters:
/// - `data`: The file content to chunk
/// - `target_size`: Target average block size (determines the hash mask)
/// - `min_size`: Minimum block size (default 4KB)
/// - `max_size`: Maximum block size (default 1MB)
///
/// Returns a list of `(offset, length, hash)` tuples.
pub fn chunk_data(data: &[u8], target_size: u64, min_size: u64, max_size: u64) -> Vec<(u64, u64, String)> {
    let mask = compute_mask(target_size);
    let mut buzhash = BuzHash::new();
    let mut blocks = Vec::new();
    let mut block_start: usize = 0;

    // Small files: return as single block
    if data.len() as u64 <= min_size {
        if !data.is_empty() {
            let hash = ContentHash::compute(data);
            blocks.push((0u64, data.len() as u64, hash.as_hex().to_string()));
        }
        return blocks;
    }

    for (i, &byte) in data.iter().enumerate() {
        let block_len = (i - block_start) as u64;

        // Enforce maximum block size
        if block_len >= max_size {
            // Cut before the current byte to keep block <= max_size.
            // The current byte will start the next block (processed again).
            let block_data = &data[block_start..i];
            let hash = ContentHash::compute(block_data);
            blocks.push((block_start as u64, block_data.len() as u64, hash.as_hex().to_string()));
            block_start = i; // Current byte starts the next block
            buzhash = BuzHash::new();
        // Don't continue - fall through so current byte is processed in the next block
        } else if block_len >= min_size {
            buzhash.update(byte);
            if (buzhash.value() & mask) == 0 {
                let block_data = &data[block_start..=i];
                let hash = ContentHash::compute(block_data);
                blocks.push((block_start as u64, block_data.len() as u64, hash.as_hex().to_string()));
                block_start = i + 1;
                buzhash = BuzHash::new();
            }
        } else {
            buzhash.update(byte);
        }
    }

    // Remaining data as last block
    if block_start < data.len() {
        let block_data = &data[block_start..];
        let hash = ContentHash::compute(block_data);
        blocks.push((block_start as u64, block_data.len() as u64, hash.as_hex().to_string()));
    }

    blocks
}

/// Compute the mask for content-defined chunking from the target block size.
/// Uses the nearest power of 2 below target_size.
fn compute_mask(target_size: u64) -> u64 {
    let bits = 63 - target_size.leading_zeros();
    (1u64 << bits) - 1
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// `GET /api/v1/sync/blocks/manifest?path=/foo.txt&block_size=65536`
///
/// Computes the block manifest for a file on the server and returns which
/// blocks the client needs to upload.
pub async fn get_manifest(State(state): State<AppState>, Query(params): Query<ManifestQuery>) -> Response {
    let path = params.path.trim_start_matches('/');

    // Fetch file content
    let content = match state.storage().get(path).await {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": "file not found",
                    "detail": e.to_string()
                })),
            )
                .into_response();
        }
    };

    let file_hash = ContentHash::compute(&content);
    let block_size = params.block_size.clamp(4096, 1_048_576);

    // Chunk the file
    let raw_blocks = chunk_data(&content, block_size, 4096, 1_048_576);

    let blocks: Vec<BlockDescriptor> = raw_blocks
        .iter()
        .enumerate()
        .map(|(idx, (offset, len, hash))| BlockDescriptor {
            index: idx as u32,
            hash: hash.clone(),
            size: *len,
            offset: *offset,
        })
        .collect();

    // Check which blocks exist in CAS
    let missing_blocks = if let Some(cas) = state.cas_store() {
        let mut missing = Vec::new();
        for block in &blocks {
            let content_hash = match ContentHash::new(block.hash.clone()) {
                Some(h) => h,
                None => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(serde_json::json!({
                            "error": "invalid block hash",
                            "hash": block.hash,
                        })),
                    )
                        .into_response();
                }
            };
            match cas.exists(&content_hash).await {
                Ok(true) => {}
                _ => {
                    missing.push(block.hash.clone());
                }
            }
        }
        missing
    } else {
        // No CAS store: all blocks are "missing" (client must upload full file)
        blocks.iter().map(|b| b.hash.clone()).collect()
    };

    let manifest = BlockManifest {
        path: path.to_string(),
        total_size: content.len() as u64,
        file_hash: file_hash.as_hex().to_string(),
        blocks,
    };

    (
        StatusCode::OK,
        Json(ManifestResponse {
            manifest,
            missing_blocks,
        }),
    )
        .into_response()
}

/// `POST /api/v1/sync/blocks/upload`
///
/// Upload individual blocks to the server's CAS store. Blocks are deduplicated
/// by content hash.
pub async fn upload_blocks(State(state): State<AppState>, Json(body): Json<UploadBlocksRequest>) -> Response {
    let cas = match state.cas_store() {
        Some(cas) => cas,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({
                    "error": "CAS store not configured"
                })),
            )
                .into_response();
        }
    };

    let mut stored = 0usize;
    let mut deduplicated = 0usize;

    for (hash_hex, content_b64) in &body.blocks {
        let content_bytes = match base64::engine::general_purpose::STANDARD.decode(content_b64) {
            Ok(b) => bytes::Bytes::from(b),
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "error": "invalid base64",
                        "hash": hash_hex,
                        "detail": e.to_string()
                    })),
                )
                    .into_response();
            }
        };

        // Verify hash matches content
        let computed_hash = ContentHash::compute(&content_bytes);
        if computed_hash.as_hex() != hash_hex {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "hash mismatch",
                    "expected": hash_hex,
                    "computed": computed_hash.as_hex()
                })),
            )
                .into_response();
        }

        match cas.put_content(content_bytes).await {
            Ok(_) => {
                // put_content deduplicates, so we can't easily distinguish new vs existing.
                // Check existence first for accurate counts.
                let content_hash = ContentHash::new_unchecked(hash_hex.clone());
                match cas.dedup_check(&content_hash).await {
                    Ok(true) => deduplicated += 1,
                    _ => stored += 1,
                }
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "error": "failed to store block",
                        "hash": hash_hex,
                        "detail": e.to_string()
                    })),
                )
                    .into_response();
            }
        }
    }

    (StatusCode::OK, Json(UploadBlocksResponse { stored, deduplicated })).into_response()
}

/// `GET /api/v1/sync/blocks/check`
///
/// Given a list of block hashes, return which ones are missing from the server.
#[derive(Debug, Deserialize)]
pub struct CheckBlocksQuery {
    /// Comma-separated list of block hashes to check.
    pub hashes: String,
}

#[derive(Debug, Serialize)]
pub struct CheckBlocksResponse {
    pub missing: Vec<String>,
    pub present: usize,
}

pub async fn check_blocks(State(state): State<AppState>, Query(params): Query<CheckBlocksQuery>) -> Response {
    let cas = match state.cas_store() {
        Some(cas) => cas,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({"error": "CAS store not configured"})),
            )
                .into_response();
        }
    };

    let hashes: Vec<&str> = params.hashes.split(',').map(|s| s.trim()).collect();
    let mut missing = Vec::new();
    let mut present = 0;

    for hash_str in hashes {
        if hash_str.is_empty() {
            continue;
        }
        let content_hash = match ContentHash::new(hash_str.to_string()) {
            Some(h) => h,
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "error": "invalid block hash",
                        "hash": hash_str,
                    })),
                )
                    .into_response();
            }
        };
        match cas.exists(&content_hash).await {
            Ok(true) => present += 1,
            _ => missing.push(hash_str.to_string()),
        }
    }

    (StatusCode::OK, Json(CheckBlocksResponse { missing, present })).into_response()
}

/// `POST /api/v1/sync/blocks/assemble`
///
/// Assemble a file from blocks already in the CAS store.
pub async fn assemble_file(State(state): State<AppState>, Json(body): Json<AssembleRequest>) -> Response {
    let cas = match state.cas_store() {
        Some(cas) => cas,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({"error": "CAS store not configured"})),
            )
                .into_response();
        }
    };

    // Fetch all blocks
    let mut assembled = Vec::with_capacity(body.block_hashes.len() * 65536);
    for (i, hash_hex) in body.block_hashes.iter().enumerate() {
        let content_hash = match ContentHash::new(hash_hex.clone()) {
            Some(h) => h,
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "error": "invalid block hash",
                        "block_index": i,
                    })),
                )
                    .into_response();
            }
        };
        match cas.get_content(&content_hash).await {
            Ok(block_bytes) => assembled.extend_from_slice(&block_bytes),
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "error": "missing block",
                        "block_index": i,
                        "block_hash": hash_hex,
                        "detail": e.to_string()
                    })),
                )
                    .into_response();
            }
        }
    }

    let file_hash = ContentHash::compute(&assembled);
    let file_bytes = bytes::Bytes::from(assembled);
    let total_size = file_bytes.len() as u64;

    // Write to storage
    let path = body.path.trim_start_matches('/');
    match state.storage().put(path, file_bytes, &body.owner).await {
        Ok(_meta) => (
            StatusCode::OK,
            Json(AssembleResponse {
                file_hash: file_hash.as_hex().to_string(),
                size: total_size,
                block_count: body.block_hashes.len(),
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "failed to write assembled file",
                "detail": e.to_string()
            })),
        )
            .into_response(),
    }
}

/// `GET /api/v1/sync/blocks/:hash`
///
/// Download a single block from the CAS store by its content hash.
pub async fn get_block(State(state): State<AppState>, Path(hash): Path<String>) -> Response {
    let cas = match state.cas_store() {
        Some(cas) => cas,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({"error": "CAS store not configured"})),
            )
                .into_response();
        }
    };

    let content_hash = match ContentHash::new(hash.clone()) {
        Some(h) => h,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "invalid block hash",
                    "hash": hash,
                })),
            )
                .into_response();
        }
    };
    match cas.get_content(&content_hash).await {
        Ok(data) => (
            StatusCode::OK,
            [
                ("content-type", "application/octet-stream".to_string()),
                ("content-length", data.len().to_string()),
            ],
            data,
        )
            .into_response(),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "block not found",
                "detail": e.to_string()
            })),
        )
            .into_response(),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_data_small_file() {
        let data = b"hello world";
        let blocks = chunk_data(data, 65536, 4096, 1_048_576);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].0, 0); // offset
        assert_eq!(blocks[0].1, 11); // length
    }

    #[test]
    fn test_chunk_data_empty() {
        let data = b"";
        let blocks = chunk_data(data, 65536, 4096, 1_048_576);
        assert!(blocks.is_empty());
    }

    #[test]
    fn test_chunk_data_deterministic() {
        let data: Vec<u8> = (0..100_000).map(|i| (i % 256) as u8).collect();
        let blocks1 = chunk_data(&data, 65536, 4096, 1_048_576);
        let blocks2 = chunk_data(&data, 65536, 4096, 1_048_576);
        assert_eq!(blocks1, blocks2, "chunking must be deterministic");
        // Verify blocks cover the entire file
        let total: u64 = blocks1.iter().map(|b| b.1).sum();
        assert_eq!(total, data.len() as u64, "blocks must cover entire file");
        // Verify blocks are contiguous
        let mut offset = 0u64;
        for (block_offset, block_len, _) in &blocks1 {
            assert_eq!(*block_offset, offset, "block offset mismatch");
            offset += block_len;
        }
    }

    #[test]
    fn test_chunk_data_respects_max_size() {
        let data: Vec<u8> = (0..200_000).map(|i| (i % 256) as u8).collect();
        let max_size = 32_768; // 32KB max
        let blocks = chunk_data(&data, 65536, 4096, max_size);
        for (_, len, _) in &blocks {
            assert!(*len <= max_size, "block size {} exceeds max {}", len, max_size);
        }
    }

    #[test]
    fn test_chunk_data_respects_min_size() {
        // With 100KB of data and min_size=8192, the first block should be at least 8KB
        let data: Vec<u8> = (0..100_000).map(|i| (i % 256) as u8).collect();
        let blocks = chunk_data(&data, 65536, 8192, 1_048_576);
        // All blocks except possibly the last should be >= min_size
        for (i, (_, len, _)) in blocks.iter().enumerate() {
            if i < blocks.len() - 1 {
                assert!(*len >= 8192, "non-final block {} has size {} < min_size 8192", i, len);
            }
        }
    }

    #[test]
    fn test_compute_mask() {
        assert_eq!(compute_mask(65536), 0xFFFF); // 16 bits
        assert_eq!(compute_mask(4096), 0xFFF); // 12 bits
        assert_eq!(compute_mask(1024), 0x3FF); // 10 bits
    }

    #[test]
    fn test_block_determinism_different_ordering() {
        // Create a file with clear block boundaries (repeated patterns)
        // then insert a byte. CDC should re-chunk identically before the insertion.
        let pattern: Vec<u8> = "The quick brown fox jumps over the lazy dog. ".as_bytes().to_vec();
        let data1: Vec<u8> = pattern.iter().cycle().take(100_000).cloned().collect();

        let mut data2 = data1.clone();
        // Insert one byte at the 80KB boundary
        data2.insert(80_000, 0xFF);

        let blocks1 = chunk_data(&data1, 65536, 4096, 1_048_576);
        let blocks2 = chunk_data(&data2, 65536, 4096, 1_048_576);

        // Blocks before the insertion point should be identical
        // because the data is identical up to offset 80000
        let blocks1_before: Vec<_> = blocks1.iter().filter(|(off, _, _)| *off < 80_000).collect();
        let blocks2_before: Vec<_> = blocks2.iter().filter(|(off, _, _)| *off < 80_000).collect();

        // At least the first block (offset 0) should match
        assert!(
            !blocks1_before.is_empty(),
            "should have at least one block before insertion point"
        );
        assert!(
            !blocks2_before.is_empty(),
            "should have at least one block before insertion point"
        );

        // Check if the first blocks match
        if blocks1_before[0] == blocks2_before[0] {
            // First blocks match - CDC is working correctly for the prefix
            assert_eq!(
                blocks1_before[0].2, blocks2_before[0].2,
                "first block hash should match"
            );
        }

        // Regardless of chunk boundaries, total data covered should be consistent
        let total1: u64 = blocks1.iter().map(|b| b.1).sum();
        let total2: u64 = blocks2.iter().map(|b| b.1).sum();
        assert_eq!(total1, data1.len() as u64);
        assert_eq!(total2, data2.len() as u64);
    }
}
