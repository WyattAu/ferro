use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;

// ---------------------------------------------------------------------------
// Canonical chunking parameters (DC-DEDUP-002, YP-DEDUP-CDC-001)
//
// Client and server MUST source these from here. Parameter drift between the
// desktop client and the server is silent and corrupting: identical content
// would be cut at different boundaries, so dedup silently stops working and
// diffs become wrong. Previously the literals (65536/4096/1048576) were
// duplicated at four call sites.
//
// Legacy manifests cut with Buzhash (buzhash-v1, 4 KiB/64 KiB/1 MiB) remain
// verifiable; new manifests MUST use FastCDC (fastcdc-v1) below.
// ---------------------------------------------------------------------------

/// Target average block size (determines the hash mask; a power of two).
pub const CHUNK_TARGET_SIZE: u64 = 65_536;

/// Minimum block size. Bytes below this are never a boundary test
/// (FastCDC "cut-point skipping").
pub const CHUNK_MIN_SIZE: u64 = 4_096;

/// Maximum block size; a hard cut is forced at this length.
pub const CHUNK_MAX_SIZE: u64 = 1_048_576;

/// Rolling-hash window length in bytes.
pub const BUZHASH_WINDOW: usize = 48;

/// Algorithm tag for legacy Buzhash manifests.
pub const CHUNK_ALGO_BUZHASH_V1: &str = "buzhash-v1";

/// Algorithm tag for FastCDC manifests (Gear hash, normalized chunking).
pub const CHUNK_ALGO_FASTCDC_V1: &str = "fastcdc-v1";

// --- FastCDC (ATC'16) — YP-DEDUP-CDC-001 revision 1.1 ----------------------

/// FastCDC minimum chunk size (256 KiB).
pub const FASTCDC_MIN_SIZE: u64 = 262_144;

/// FastCDC average chunk size (1 MiB) — sets the base mask.
pub const FASTCDC_TARGET_SIZE: u64 = 1_048_576;

/// FastCDC maximum chunk size (8 MiB, hard cut).
pub const FASTCDC_MAX_SIZE: u64 = 8_388_608;

/// Gear hash loses contributions older than 64 bytes (64 shifts).
pub const GEAR_WINDOW_BYTES: u64 = 64;

/// Normalization strength: extra mask bits in the normalized region.
pub const FASTCDC_NORM_BITS: u32 = 2;

/// Deterministic 256-entry Gear table (ATC'16 §Gear-based CDC).
///
/// Seeded with the golden-ratio constant and advanced by the Knuth MMIX
/// LCG so the table is identical in every process — the chunk boundary
/// pattern is a pure function of the data alone.
fn gear_table() -> [u64; 256] {
    let mut table = [0u64; 256];
    let mut seed: u64 = 0x9E37_79B9_7F4A_7C15;
    for slot in table.iter_mut() {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        seed ^= seed >> 17;
        seed = seed.wrapping_mul(0x2545F4914F6CDD1D);
        seed ^= seed << 31;
        *slot = seed;
    }
    table
}

/// Gear hash accumulator: `fp' = (fp << 1) + table[byte]`.
///
/// After 64 shifts the contribution of any byte older than the last 64 has
/// fully rotated out, so the fingerprint is a pure function of the trailing
/// 64-byte window — the locality property that makes chunk boundaries
/// resynchronize after insertions elsewhere in the stream.
pub struct GearHash {
    fp: u64,
}

impl GearHash {
    #[must_use]
    pub fn new() -> Self {
        Self { fp: 0 }
    }

    pub fn update(&mut self, table: &[u64; 256], byte: u8) {
        self.fp = self.fp.wrapping_shl(1).wrapping_add(table[byte as usize]);
    }

    #[must_use]
    pub fn value(&self) -> u64 {
        self.fp
    }
}

impl Default for GearHash {
    fn default() -> Self {
        Self::new()
    }
}

fn mask_bits(avg_size: u64) -> u32 {
    63 - avg_size.max(1).leading_zeros()
}

fn mask(bits: u32) -> u64 {
    if bits >= 64 { u64::MAX } else { (1u64 << bits) - 1 }
}

/// A content-defined chunk: byte range plus content hash.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FastChunk {
    pub offset: u64,
    pub size: u64,
    pub hash: [u8; 32],
}

/// FastCDC chunking (ATC'16): Gear hash judgment, sub-minimum cut-point
/// skipping, and normalized chunk-size distribution via two cutoff masks.
///
/// Regions per chunk: `[0, min)` — no test; `[min, min+normal)` — hard mask
/// (`base+norm` bits); `[min+normal, max)` — easy mask (`base−norm` bits);
/// hard cut at `max`. Every chunk except the last satisfies
/// `min ≤ size ≤ max`; the last chunk is `1 ≤ size ≤ max`.
#[must_use]
pub fn fastcdc_chunk_with(data: &[u8], min_size: u64, avg_size: u64, max_size: u64, norm_bits: u32) -> Vec<FastChunk> {
    if data.is_empty() {
        return Vec::new();
    }
    let table = gear_table();
    let base_bits = mask_bits(avg_size);
    let mask_hard = mask(base_bits + norm_bits);
    let mask_easy = mask(base_bits.saturating_sub(norm_bits));
    let normal_region = (min_size / 2).max(1);

    let len = data.len() as u64;
    let mut chunks = Vec::new();
    let mut start: u64 = 0;
    let mut fp = GearHash::new();

    while start < len {
        let remaining = len - start;
        if remaining <= min_size {
            let bytes = &data[start as usize..];
            chunks.push(FastChunk {
                offset: start,
                size: remaining,
                hash: Sha256::digest(bytes).into(),
            });
            break;
        }

        // Skip the sub-minimum region (cut-point skipping), updating fp.
        let skip_end = (start + min_size).min(len);
        for &b in &data[start as usize..skip_end as usize] {
            fp.update(&table, b);
        }

        let hard_end = (start + min_size + normal_region).min(len);
        let end = (start + max_size).min(len);
        let mut cut: Option<u64> = None;

        // Normalized region: hard mask (fewer cuts, pushes size up).
        let mut pos = skip_end;
        while pos < hard_end {
            fp.update(&table, data[pos as usize]);
            if fp.value() & mask_hard == 0 {
                cut = Some(pos + 1);
                break;
            }
            pos += 1;
        }

        // Easy-mask region (more cuts, pulls size back toward avg).
        if cut.is_none() {
            let mut pos = hard_end;
            while pos < end {
                fp.update(&table, data[pos as usize]);
                if fp.value() & mask_easy == 0 {
                    cut = Some(pos + 1);
                    break;
                }
                pos += 1;
            }
        }

        let chunk_end = cut.unwrap_or(end);
        let bytes = &data[start as usize..chunk_end as usize];
        chunks.push(FastChunk {
            offset: start,
            size: chunk_end - start,
            hash: Sha256::digest(bytes).into(),
        });
        start = chunk_end;
    }

    chunks
}

/// FastCDC with the canonical Ferro parameters (256 KiB / 1 MiB / 8 MiB).
#[must_use]
pub fn fastcdc_chunk(data: &[u8]) -> Vec<FastChunk> {
    fastcdc_chunk_with(
        data,
        FASTCDC_MIN_SIZE,
        FASTCDC_TARGET_SIZE,
        FASTCDC_MAX_SIZE,
        FASTCDC_NORM_BITS,
    )
}

/// FastCDC as `(offset, size, hex_hash)` triples — the shape the sync
/// manifests consume. Canonical for all NEW manifests (fastcdc-v1).
#[must_use]
pub fn fastcdc_triples(data: &[u8]) -> Vec<(u64, u64, String)> {
    fastcdc_chunk(data)
        .into_iter()
        .map(|c| (c.offset, c.size, hex::encode(c.hash)))
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkInfo {
    pub hash: [u8; 32],
    pub offset: u64,
    pub size: u32,
    pub index: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockDiffRequest {
    pub local_chunks: Vec<ChunkInfo>,
    pub new_chunks: Vec<ChunkInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockDiffResult {
    pub chunks_to_upload: Vec<ChunkInfo>,
    pub chunks_to_download: Vec<ChunkInfo>,
    pub chunks_common: Vec<ChunkInfo>,
}

#[must_use]
pub fn compute_block_diff(request: &BlockDiffRequest) -> BlockDiffResult {
    let local_hashes: HashSet<[u8; 32]> = request.local_chunks.iter().map(|c| c.hash).collect();

    let new_hashes: HashSet<[u8; 32]> = request.new_chunks.iter().map(|c| c.hash).collect();

    let mut chunks_to_upload = Vec::new();
    let mut chunks_to_download = Vec::new();
    let mut chunks_common = Vec::new();

    for chunk in &request.new_chunks {
        if local_hashes.contains(&chunk.hash) {
            chunks_common.push(chunk.clone());
        } else {
            chunks_to_upload.push(chunk.clone());
        }
    }

    for chunk in &request.local_chunks {
        if !new_hashes.contains(&chunk.hash) {
            chunks_to_download.push(chunk.clone());
        }
    }

    BlockDiffResult {
        chunks_to_upload,
        chunks_to_download,
        chunks_common,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_chunk(i: u8, offset: u64, size: u32, index: u32) -> ChunkInfo {
        let mut hash = [0u8; 32];
        hash[0] = i;
        ChunkInfo {
            hash,
            offset,
            size,
            index,
        }
    }

    #[test]
    fn test_no_changes() {
        let chunks = vec![
            make_chunk(1, 0, 100, 0),
            make_chunk(2, 100, 200, 1),
            make_chunk(3, 300, 150, 2),
        ];
        let request = BlockDiffRequest {
            local_chunks: chunks.clone(),
            new_chunks: chunks.clone(),
        };
        let result = compute_block_diff(&request);
        assert!(result.chunks_to_upload.is_empty());
        assert!(result.chunks_to_download.is_empty());
        assert_eq!(result.chunks_common.len(), 3);
    }

    #[test]
    fn test_new_file() {
        let local = vec![];
        let new_chunks = vec![make_chunk(10, 0, 100, 0), make_chunk(20, 100, 200, 1)];
        let request = BlockDiffRequest {
            local_chunks: local,
            new_chunks: new_chunks.clone(),
        };
        let result = compute_block_diff(&request);
        assert_eq!(result.chunks_to_upload.len(), 2);
        assert!(result.chunks_to_download.is_empty());
        assert!(result.chunks_common.is_empty());
    }

    #[test]
    fn test_reversed() {
        let local = vec![make_chunk(1, 0, 100, 0), make_chunk(2, 100, 200, 1)];
        let new_chunks = vec![make_chunk(3, 0, 150, 0), make_chunk(4, 150, 100, 1)];
        let request = BlockDiffRequest {
            local_chunks: local,
            new_chunks,
        };
        let result = compute_block_diff(&request);
        assert_eq!(result.chunks_to_upload.len(), 2);
        assert_eq!(result.chunks_to_download.len(), 2);
        assert!(result.chunks_common.is_empty());
    }

    #[test]
    fn test_partial_overlap() {
        let local = vec![
            make_chunk(1, 0, 100, 0),
            make_chunk(2, 100, 200, 1),
            make_chunk(3, 300, 150, 2),
        ];
        let new_chunks = vec![
            make_chunk(2, 0, 200, 0),
            make_chunk(4, 200, 100, 1),
            make_chunk(3, 300, 150, 2),
        ];
        let request = BlockDiffRequest {
            local_chunks: local,
            new_chunks,
        };
        let result = compute_block_diff(&request);
        assert_eq!(result.chunks_common.len(), 2);
        assert_eq!(result.chunks_to_upload.len(), 1);
        assert_eq!(result.chunks_to_upload[0].hash[0], 4);
        assert_eq!(result.chunks_to_download.len(), 1);
        assert_eq!(result.chunks_to_download[0].hash[0], 1);
    }

    #[test]
    fn test_empty_request() {
        let request = BlockDiffRequest {
            local_chunks: vec![],
            new_chunks: vec![],
        };
        let result = compute_block_diff(&request);
        assert!(result.chunks_to_upload.is_empty());
        assert!(result.chunks_to_download.is_empty());
        assert!(result.chunks_common.is_empty());
    }

    #[test]
    fn test_single_chunk_same() {
        let chunk = make_chunk(1, 0, 100, 0);
        let request = BlockDiffRequest {
            local_chunks: vec![chunk.clone()],
            new_chunks: vec![chunk],
        };
        let result = compute_block_diff(&request);
        assert_eq!(result.chunks_common.len(), 1);
        assert!(result.chunks_to_upload.is_empty());
        assert!(result.chunks_to_download.is_empty());
    }

    #[test]
    fn test_single_chunk_different() {
        let local = make_chunk(1, 0, 100, 0);
        let new = make_chunk(2, 0, 100, 0);
        let request = BlockDiffRequest {
            local_chunks: vec![local],
            new_chunks: vec![new],
        };
        let result = compute_block_diff(&request);
        assert!(result.chunks_common.is_empty());
        assert_eq!(result.chunks_to_upload.len(), 1);
        assert_eq!(result.chunks_to_download.len(), 1);
    }

    #[test]
    fn test_chunk_info_debug() {
        let chunk = make_chunk(1, 0, 100, 0);
        let debug = format!("{:?}", chunk);
        assert!(debug.contains("ChunkInfo"));
    }

    #[test]
    fn test_chunk_info_clone() {
        let chunk1 = make_chunk(1, 0, 100, 0);
        let chunk2 = chunk1.clone();
        assert_eq!(chunk1.hash, chunk2.hash);
        assert_eq!(chunk1.offset, chunk2.offset);
        assert_eq!(chunk1.size, chunk2.size);
        assert_eq!(chunk1.index, chunk2.index);
    }

    #[test]
    fn test_chunk_info_serialize_deserialize() {
        let chunk = make_chunk(1, 0, 100, 0);
        let json = serde_json::to_string(&chunk).unwrap();
        let deserialized: ChunkInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(chunk.hash, deserialized.hash);
        assert_eq!(chunk.offset, deserialized.offset);
    }

    #[test]
    fn test_block_diff_request_debug() {
        let request = BlockDiffRequest {
            local_chunks: vec![],
            new_chunks: vec![],
        };
        let debug = format!("{:?}", request);
        assert!(debug.contains("BlockDiffRequest"));
    }

    #[test]
    fn test_block_diff_result_debug() {
        let result = BlockDiffResult {
            chunks_to_upload: vec![],
            chunks_to_download: vec![],
            chunks_common: vec![],
        };
        let debug = format!("{:?}", result);
        assert!(debug.contains("BlockDiffResult"));
    }

    #[test]
    fn test_block_diff_result_clone() {
        let result1 = BlockDiffResult {
            chunks_to_upload: vec![make_chunk(1, 0, 100, 0)],
            chunks_to_download: vec![make_chunk(2, 100, 200, 1)],
            chunks_common: vec![make_chunk(3, 300, 150, 2)],
        };
        let result2 = result1.clone();
        assert_eq!(result1.chunks_to_upload.len(), result2.chunks_to_upload.len());
        assert_eq!(result1.chunks_to_download.len(), result2.chunks_to_download.len());
        assert_eq!(result1.chunks_common.len(), result2.chunks_common.len());
    }

    #[test]
    fn test_block_diff_result_serialize_deserialize() {
        let result = BlockDiffResult {
            chunks_to_upload: vec![make_chunk(1, 0, 100, 0)],
            chunks_to_download: vec![],
            chunks_common: vec![],
        };
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: BlockDiffResult = serde_json::from_str(&json).unwrap();
        assert_eq!(result.chunks_to_upload.len(), deserialized.chunks_to_upload.len());
    }

    // === FastCDC (ATC'16) tests — YP-DEDUP-CDC-001 rev 1.1 ===

    fn pseudo_random(n: usize) -> Vec<u8> {
        let mut state = 0x1234_5678_9abc_def0u64;
        (0..n)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                (state >> 24) as u8
            })
            .collect()
    }

    #[test]
    fn test_gear_table_deterministic() {
        assert_eq!(gear_table(), gear_table());
        assert!(gear_table().iter().any(|&v| v != 0));
    }

    #[test]
    fn test_fastcdc_empty() {
        assert!(fastcdc_chunk(&[]).is_empty());
    }

    #[test]
    fn test_fastcdc_single_chunk_below_min() {
        let data = pseudo_random(1_000);
        let chunks = fastcdc_chunk(&data);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].offset, 0);
        assert_eq!(chunks[0].size, 1_000);
    }

    #[test]
    fn test_fastcdc_size_bounds() {
        let data = pseudo_random(64 * 1024 * 1024);
        let chunks = fastcdc_chunk(&data);
        assert!(chunks.len() > 8, "expected a real chunk pattern, got {}", chunks.len());
        let last = chunks.len() - 1;
        for (i, c) in chunks.iter().enumerate() {
            assert!(c.size <= FASTCDC_MAX_SIZE, "chunk {i} exceeds max: {}", c.size);
            if i != last {
                assert!(c.size >= FASTCDC_MIN_SIZE, "chunk {i} below min: {}", c.size);
            }
            let expect: [u8; 32] = Sha256::digest(&data[c.offset as usize..(c.offset + c.size) as usize]).into();
            assert_eq!(c.hash, expect);
        }
    }

    #[test]
    fn test_fastcdc_determinism() {
        let data = pseudo_random(32 * 1024 * 1024);
        assert_eq!(fastcdc_chunk(&data), fastcdc_chunk(&data));
    }

    #[test]
    fn test_fastcdc_hash_integrity() {
        let data = pseudo_random(8 * 1024 * 1024);
        for c in fastcdc_chunk(&data) {
            assert_eq!(
                c.hash,
                Sha256::digest(&data[c.offset as usize..(c.offset + c.size) as usize])
            );
        }
    }

    #[test]
    fn test_fastcdc_resynchronization_after_prefix_insert() {
        let data = pseudo_random(48 * 1024 * 1024);
        let prefix = pseudo_random(3_000);
        let shifted = [prefix.clone(), data.clone()].concat();

        let base = fastcdc_chunk(&data);
        let after = fastcdc_chunk(&shifted);

        let base_hashes: std::collections::HashSet<[u8; 32]> = base.iter().map(|c| c.hash).collect();
        let reused = after.iter().filter(|c| base_hashes.contains(&c.hash)).count();
        let ratio = reused as f64 / base.len() as f64;
        assert!(
            ratio > 0.5,
            "expected >50% chunk reuse after prefix insert, got {ratio:.2}"
        );

        let tail_base: Vec<[u8; 32]> = base.iter().rev().take(3).map(|c| c.hash).collect();
        let tail_after: Vec<[u8; 32]> = after.iter().rev().take(3).map(|c| c.hash).collect();
        assert_eq!(tail_base, tail_after, "tail must resynchronize");
    }

    #[test]
    fn test_fastcdc_tail_append_does_not_reshape_head() {
        let data = pseudo_random(24 * 1024 * 1024);
        let extended = [data.clone(), pseudo_random(5 * 1024 * 1024)].concat();

        let head_before = fastcdc_chunk(&data);
        let all = fastcdc_chunk(&extended);

        // Every chunk except the final one (which absorbs the appended tail)
        // must be byte-identical: append-only streams keep their history.
        let n = head_before.len() - 1;
        assert!(all.len() > n);
        for (i, c) in head_before.iter().take(n).enumerate() {
            assert_eq!(all[i], *c, "chunk {i} changed after a pure tail append");
        }
    }

    #[test]
    fn test_fastcdc_contiguous_coverage() {
        let data = pseudo_random(32 * 1024 * 1024);
        let chunks = fastcdc_chunk(&data);
        let mut expected_offset = 0u64;
        for c in &chunks {
            assert_eq!(c.offset, expected_offset);
            expected_offset += c.size;
        }
        assert_eq!(expected_offset, data.len() as u64);
    }
}
