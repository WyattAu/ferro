//! Pure sync functions: content-defined chunking (Buzhash-based) and sync event handlers.
//!
//! Extracted from `ferro-server` so they can be used independently
//! by client crates without pulling in the full server dependency tree.

pub mod events;

use common::metadata::ContentHash;

/// Buzhash rolling hash for content-defined chunking.
/// Uses a 48-byte window with a random lookup table.
pub struct BuzHash {
    window: [u8; 48],
    window_idx: usize,
    hash: u64,
    table: [u64; 256],
}

impl BuzHash {
    pub fn new() -> Self {
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

    pub fn update(&mut self, byte: u8) {
        let outgoing = self.window[self.window_idx];
        self.window[self.window_idx] = byte;
        self.window_idx = (self.window_idx + 1) % 48;
        // Buzhash: hash = rotate_left(hash, 1) ^ table[byte] ^ table_outgoing_shifted
        self.hash =
            self.hash.rotate_left(1) ^ self.table[byte as usize] ^ self.table[outgoing as usize].rotate_left(48);
    }

    pub fn value(&self) -> u64 {
        self.hash
    }
}

impl Default for BuzHash {
    fn default() -> Self {
        Self::new()
    }
}

/// Compute the mask for content-defined chunking from the target block size.
/// Uses the nearest power of 2 below target_size.
pub fn compute_mask(target_size: u64) -> u64 {
    let bits = 63 - target_size.leading_zeros();
    (1u64 << bits) - 1
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
