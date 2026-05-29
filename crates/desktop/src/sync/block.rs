//! Block-level content chunking for delta sync.
//!
//! Implements the same Buzhash-based content-defined chunking algorithm
//! as the server, ensuring identical chunking on both sides for the same
//! file content.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;

/// A single block descriptor.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BlockDescriptor {
    /// Zero-based block index.
    pub index: u32,
    /// SHA-256 hash of the block content (hex).
    pub hash: String,
    /// Block size in bytes.
    pub size: u64,
    /// Byte offset within the file.
    pub offset: u64,
}

/// A file's block manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockManifest {
    /// Relative path of the file.
    pub path: String,
    /// Total file size.
    pub total_size: u64,
    /// SHA-256 hash of the entire file.
    pub file_hash: String,
    /// Ordered list of blocks.
    pub blocks: Vec<BlockDescriptor>,
}

/// Chunk a local file into content-defined blocks.
pub fn chunk_file(
    path: &Path,
    relative_path: &str,
    target_block_size: u64,
) -> Result<BlockManifest> {
    let data = std::fs::read(path)?;
    let file_hash = compute_hash(&data);
    let total_size = data.len() as u64;

    let raw_blocks = chunk_data(&data, target_block_size, 4096, 1_048_576);

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

    Ok(BlockManifest {
        path: relative_path.to_string(),
        total_size,
        file_hash,
        blocks,
    })
}

/// Compute the SHA-256 hash of data, returning hex string.
pub fn compute_hash(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

// ---------------------------------------------------------------------------
// Buzhash content-defined chunking (same algorithm as server)
// ---------------------------------------------------------------------------

struct BuzHash {
    window: [u8; 48],
    window_idx: usize,
    hash: u64,
    table: [u64; 256],
}

impl BuzHash {
    fn new() -> Self {
        let mut table = [0u64; 256];
        let mut seed: u64 = 0x9E37_79B9_7F4A_7C15;
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
        self.hash = self.hash.rotate_left(1)
            ^ self.table[byte as usize]
            ^ self.table[outgoing as usize].rotate_left(48);
    }
}

/// Chunk data into content-defined blocks using Buzhash.
/// Returns a list of `(offset, length, hash)` tuples.
///
/// This MUST produce identical results to the server's `chunk_data` function
/// given the same input data and parameters.
pub fn chunk_data(
    data: &[u8],
    target_size: u64,
    min_size: u64,
    max_size: u64,
) -> Vec<(u64, u64, String)> {
    let mask = compute_mask(target_size);
    let mut buzhash = BuzHash::new();
    let mut blocks = Vec::new();
    let mut block_start: usize = 0;

    if data.len() as u64 <= min_size {
        if !data.is_empty() {
            blocks.push((0u64, data.len() as u64, compute_hash(data)));
        }
        return blocks;
    }

    for (i, &byte) in data.iter().enumerate() {
        let block_len = (i - block_start) as u64;

        if block_len >= max_size {
            let block_data = &data[block_start..i];
            blocks.push((
                block_start as u64,
                block_data.len() as u64,
                compute_hash(block_data),
            ));
            block_start = i;
            buzhash = BuzHash::new();
        } else if block_len >= min_size {
            buzhash.update(byte);
            if (buzhash.hash & mask) == 0 {
                let block_data = &data[block_start..=i];
                blocks.push((
                    block_start as u64,
                    block_data.len() as u64,
                    compute_hash(block_data),
                ));
                block_start = i + 1;
                buzhash = BuzHash::new();
            }
        } else {
            buzhash.update(byte);
        }
    }

    if block_start < data.len() {
        let block_data = &data[block_start..];
        blocks.push((
            block_start as u64,
            block_data.len() as u64,
            compute_hash(block_data),
        ));
    }

    blocks
}

fn compute_mask(target_size: u64) -> u64 {
    let bits = 63 - target_size.leading_zeros();
    (1u64 << bits) - 1
}

/// Compare two block manifests and return the hashes of blocks that differ.
pub fn diff_manifests(local: &BlockManifest, remote: &BlockManifest) -> Vec<String> {
    let local_hashes: std::collections::HashSet<&str> =
        local.blocks.iter().map(|b| b.hash.as_str()).collect();
    let remote_hashes: std::collections::HashSet<&str> =
        remote.blocks.iter().map(|b| b.hash.as_str()).collect();

    // Blocks in remote but not in local (need to upload)
    remote_hashes
        .iter()
        .filter(|h| !local_hashes.contains(*h))
        .map(|h| h.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_chunk_file() {
        let dir = std::env::temp_dir().join("ferro-block-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let file_path = dir.join("test.bin");
        let data: Vec<u8> = (0..100_000).map(|i| (i % 256) as u8).collect();
        std::fs::write(&file_path, &data).unwrap();

        let manifest = chunk_file(&file_path, "test.bin", 65536).unwrap();
        assert_eq!(manifest.total_size, 100_000);
        assert!(!manifest.blocks.is_empty());

        // Verify blocks cover entire file
        let total: u64 = manifest.blocks.iter().map(|b| b.size).sum();
        assert_eq!(total, 100_000);

        // Verify determinism
        let manifest2 = chunk_file(&file_path, "test.bin", 65536).unwrap();
        assert_eq!(manifest.blocks, manifest2.blocks);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_chunk_small_file() {
        let data = b"hello";
        let blocks = chunk_data(data, 65536, 4096, 1_048_576);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].1, 5); // length
    }

    #[test]
    fn test_compute_hash_deterministic() {
        let data = b"test content";
        let h1 = compute_hash(data);
        let h2 = compute_hash(data);
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64); // SHA-256 hex = 64 chars
    }
}
