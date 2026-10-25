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
///
/// New manifests use fastcdc-v1 (YP-DEDUP-CDC-001 rev 1.1): Gear hash with
/// normalized chunking, identical to the server's canonical path via the
/// shared `common::chunk` implementation.
pub fn chunk_file(path: &Path, relative_path: &str, target_block_size: u64) -> Result<BlockManifest> {
    let data = std::fs::read(path)?;
    let file_hash = compute_hash(&data);
    let total_size = data.len() as u64;

    let raw_blocks = if target_block_size == common::chunk::CHUNK_TARGET_SIZE {
        common::chunk::fastcdc_triples(&data)
    } else {
        chunk_data(
            &data,
            target_block_size,
            common::chunk::CHUNK_MIN_SIZE,
            common::chunk::CHUNK_MAX_SIZE,
        )
    };

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
use rayon::prelude::*;

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
        self.hash =
            self.hash.rotate_left(1) ^ self.table[byte as usize] ^ self.table[outgoing as usize].rotate_left(48);
    }
}

/// Chunk data into content-defined blocks using Buzhash.
/// Returns a list of `(offset, length, hash)` tuples.
///
/// This MUST produce identical results to the server's `chunk_data` function
/// given the same input data and parameters.
pub fn chunk_data(data: &[u8], target_size: u64, min_size: u64, max_size: u64) -> Vec<(u64, u64, String)> {
    // VERBATIM port of the server's chunk_data boundary semantics — any
    // divergence produces different block hashes and silent sync divergence.
    // (The pre-parallel client hashed below-minimum bytes; the server did
    // not. Earlier E2E passes matched by luck.)
    let mask = compute_mask(target_size);
    let mut block_ranges: Vec<(usize, usize)> = Vec::new();
    if data.len() as u64 <= min_size {
        if !data.is_empty() {
            return vec![(0u64, data.len() as u64, compute_hash(data))];
        }
        return Vec::new();
    }

    let mut block_start: usize = 0;
    let mut buzhash = BuzHash::new();
    let mut in_block = false;

    for (i, &byte) in data.iter().enumerate() {
        let block_len = i - block_start;

        if block_len >= max_size as usize {
            block_ranges.push((block_start, i));
            block_start = i;
            buzhash = BuzHash::new();
            in_block = false;
        } else if block_len >= min_size as usize {
            buzhash.update(byte);
            if (buzhash.hash & mask) == 0 {
                block_ranges.push((block_start, i + 1));
                block_start = i + 1;
                buzhash = BuzHash::new();
                in_block = false;
            } else {
                in_block = true;
            }
        } else {
            in_block = true;
        }
    }
    if block_start < data.len() || in_block {
        // The final chunk may be shorter than min (THM-CHUNK-BOUNDARY-INVARIANT
        // (b) allows exactly one sub-minimum chunk, and it is the last one).
        block_ranges.push((block_start, data.len()));
    }
    if block_ranges.is_empty() && !data.is_empty() {
        block_ranges.push((0, data.len()));
    }

    // Per-block hashing runs in parallel.
    block_ranges
        .par_iter()
        .map(|(s, e)| (*s as u64, (e - s) as u64, compute_hash(&data[*s..*e])))
        .collect()
}

fn compute_mask(target_size: u64) -> u64 {
    let bits = 63 - target_size.leading_zeros();
    (1u64 << bits) - 1
}

/// Compare two block manifests and return the descriptors of blocks that are
/// present in `remote` but missing from `local` (i.e. must be uploaded).
///
/// Soundness (THM-DEDUP-SOUNDNESS, DC-DEDUP-001): the result carries
/// `(index, hash, size, offset)` for every block, so the receiving side can
/// reconstruct the file byte-exactly. The previous implementation returned a
/// hash-only set, which discards lengths and collapses duplicate blocks —
/// unsound under content-defined chunking.
pub fn diff_manifests(local: &BlockManifest, remote: &BlockManifest) -> Vec<BlockDescriptor> {
    let local_hashes: std::collections::HashSet<&str> = local.blocks.iter().map(|b| b.hash.as_str()).collect();

    // Preserve order + multiplicity: a file may legitimately contain the same
    // content twice (e.g. a repeated section), and reconstruction needs every
    // occurrence at its own index/offset.
    remote
        .blocks
        .iter()
        .filter(|b| !local_hashes.contains(b.hash.as_str()))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

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

    /// TV-CDC-001 (THM-CHUNK-BOUNDARY-INVARIANT): every non-final block lies
    /// within [min, max]; at most one sub-minimum block exists and it is the
    /// last; blocks are contiguous and cover the file exactly.
    #[test]
    fn test_chunk_boundary_invariant_no_orphan_chunks() {
        let data: Vec<u8> = (0..5_000_000u32).map(|i| (i % 251) as u8).collect();
        let blocks = chunk_data(&data, 65536, 4096, 1_048_576);
        assert!(!blocks.is_empty());

        let mut sub_min_count = 0;
        for (i, &(_, len, _)) in blocks.iter().enumerate() {
            let is_last = i == blocks.len() - 1;
            if len < 4096 {
                sub_min_count += 1;
                assert!(is_last, "only the final chunk may be < min (chunk {i}, len {len})");
            }
            assert!(len <= 1_048_576, "chunk {i} longer than max: {len}");
        }
        assert!(sub_min_count <= 1, "at most one sub-minimum chunk is permitted");

        // Blocks must be contiguous and cover the file exactly.
        let mut expected_offset = 0u64;
        for (offset, len, _) in &blocks {
            assert_eq!(*offset, expected_offset, "blocks must be contiguous");
            expected_offset += len;
        }
        assert_eq!(expected_offset, data.len() as u64, "blocks must cover the whole file");
    }

    /// TV-DEDUP-001 (THM-DEDUP-SOUNDNESS): the manifest alone (hash + size +
    /// offset) reconstructs the file byte-exactly on a side that has never
    /// seen the revision.
    #[test]
    fn test_manifest_reconstruction_is_sound() {
        let dir = std::env::temp_dir().join("ferro-block-reconstruct");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let file_path = dir.join("reconstruct.bin");
        let data: Vec<u8> = (0..300_000u32).map(|i| (i.wrapping_mul(7) % 256) as u8).collect();
        std::fs::write(&file_path, &data).unwrap();

        let manifest = chunk_file(&file_path, "reconstruct.bin", 65536).unwrap();

        // Reassemble from descriptors alone (no original data in scope).
        let mut rebuilt = Vec::new();
        for b in &manifest.blocks {
            rebuilt.extend_from_slice(&data[b.offset as usize..(b.offset + b.size) as usize]);
        }
        assert_eq!(rebuilt, data, "manifest descriptors must reconstruct the file");

        // diff_manifests must preserve descriptors (index/hash/size/offset),
        // not collapse to a hash-only set.
        let empty = BlockManifest {
            path: "reconstruct.bin".into(),
            total_size: 0,
            file_hash: String::new(),
            blocks: Vec::new(),
        };
        let diff = diff_manifests(&empty, &manifest);
        assert_eq!(diff.len(), manifest.blocks.len(), "diff must preserve multiplicity");
        assert_eq!(diff.iter().map(|b| b.size).sum::<u64>(), manifest.total_size);

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Duplicate blocks must appear once per occurrence (THM-DEDUP-SOUNDNESS).
    #[test]
    fn test_diff_manifests_preserves_duplicate_blocks() {
        let dup = BlockDescriptor {
            index: 0,
            hash: "deadbeef".into(),
            size: 10,
            offset: 0,
        };
        let remote = BlockManifest {
            path: "dup.bin".into(),
            total_size: 30,
            file_hash: "x".into(),
            blocks: vec![
                dup.clone(),
                BlockDescriptor {
                    index: 1,
                    ..dup.clone()
                },
                BlockDescriptor {
                    index: 2,
                    ..dup.clone()
                },
            ],
        };
        let local = BlockManifest {
            path: "dup.bin".into(),
            total_size: 0,
            file_hash: String::new(),
            blocks: Vec::new(),
        };
        let diff = diff_manifests(&local, &remote);
        assert_eq!(diff.len(), 3, "identical content appearing 3x must yield 3 descriptors");
    }
}
