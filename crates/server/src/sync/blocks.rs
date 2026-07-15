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
use ferro_server_sync_handlers::chunk_data;

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

// Content-defined chunking functions are imported from ferro-server-sync-handlers

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

// Pure function tests (chunk_data, compute_mask, BuzHash) are in ferro-server-sync-handlers
