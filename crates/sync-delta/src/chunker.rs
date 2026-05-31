use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkConfig {
    pub min_chunk_size: usize,
    pub max_chunk_size: usize,
    pub target_chunk_size: usize,
    pub window_size: usize,
}

impl Default for ChunkConfig {
    fn default() -> Self {
        Self {
            min_chunk_size: 64 * 1024,
            max_chunk_size: 1024 * 1024,
            target_chunk_size: 256 * 1024,
            window_size: 48,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkInfo {
    pub hash: [u8; 32],
    pub offset: u64,
    pub size: u32,
    pub index: u32,
}

struct RollingHash {
    window: Vec<u8>,
    window_idx: usize,
    hash: u64,
    table: [u64; 256],
}

impl RollingHash {
    fn new(window_size: usize) -> Self {
        let mut table = [0u64; 256];
        let mut seed: u64 = 0x9E37_79B9_7F4A_7C15;
        for entry in table.iter_mut() {
            seed = seed.wrapping_mul(6_364_136_223_846_793_005);
            seed ^= seed >> 17;
            *entry = seed;
        }
        Self {
            window: vec![0; window_size],
            window_idx: 0,
            hash: 0,
            table,
        }
    }

    fn update(&mut self, byte: u8, window_size: usize) {
        let outgoing = self.window[self.window_idx];
        self.window[self.window_idx] = byte;
        self.window_idx = (self.window_idx + 1) % self.window.len();
        self.hash = self.hash.rotate_left(1)
            ^ self.table[byte as usize]
            ^ self.table[outgoing as usize].rotate_left(window_size as u32);
    }

    fn reset(&mut self, window_size: usize) {
        self.window = vec![0; window_size];
        self.window_idx = 0;
        self.hash = 0;
    }
}

fn compute_mask(target_chunk_size: usize) -> u64 {
    let target = target_chunk_size as u64;
    let bits = 63 - target.leading_zeros();
    (1u64 << bits) - 1
}

fn sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

pub struct Chunker {
    config: ChunkConfig,
}

impl Chunker {
    pub fn new(config: ChunkConfig) -> Self {
        Self { config }
    }

    pub fn chunk_bytes(&mut self, data: &[u8]) -> Vec<ChunkInfo> {
        let mut chunks = Vec::new();
        if data.is_empty() {
            return chunks;
        }

        if data.len() <= self.config.min_chunk_size {
            chunks.push(ChunkInfo {
                hash: sha256(data),
                offset: 0,
                size: data.len() as u32,
                index: 0,
            });
            return chunks;
        }

        let mask = compute_mask(self.config.target_chunk_size);
        let mut rolling = RollingHash::new(self.config.window_size);
        let mut chunk_start: usize = 0;
        let mut chunk_index: u32 = 0;

        for (i, &byte) in data.iter().enumerate() {
            let chunk_len = i - chunk_start;

            if chunk_len >= self.config.max_chunk_size {
                let chunk_data = &data[chunk_start..i];
                chunks.push(ChunkInfo {
                    hash: sha256(chunk_data),
                    offset: chunk_start as u64,
                    size: chunk_data.len() as u32,
                    index: chunk_index,
                });
                chunk_index += 1;
                chunk_start = i;
                rolling.reset(self.config.window_size);
            } else if chunk_len >= self.config.min_chunk_size {
                rolling.update(byte, self.config.window_size);
                if (rolling.hash & mask) == 0 {
                    let chunk_data = &data[chunk_start..=i];
                    chunks.push(ChunkInfo {
                        hash: sha256(chunk_data),
                        offset: chunk_start as u64,
                        size: chunk_data.len() as u32,
                        index: chunk_index,
                    });
                    chunk_index += 1;
                    chunk_start = i + 1;
                    rolling.reset(self.config.window_size);
                }
            } else {
                rolling.update(byte, self.config.window_size);
            }
        }

        if chunk_start < data.len() {
            let chunk_data = &data[chunk_start..];
            chunks.push(ChunkInfo {
                hash: sha256(chunk_data),
                offset: chunk_start as u64,
                size: chunk_data.len() as u32,
                index: chunk_index,
            });
        }

        chunks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_input() {
        let mut chunker = Chunker::new(ChunkConfig::default());
        let chunks = chunker.chunk_bytes(&[]);
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_small_file() {
        let config = ChunkConfig {
            min_chunk_size: 1024,
            max_chunk_size: 4096,
            target_chunk_size: 2048,
            window_size: 48,
        };
        let mut chunker = Chunker::new(config);
        let data = vec![0xABu8; 512];
        let chunks = chunker.chunk_bytes(&data);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].size, 512);
        assert_eq!(chunks[0].offset, 0);
        assert_eq!(chunks[0].index, 0);
    }

    #[test]
    fn test_large_file() {
        let config = ChunkConfig {
            min_chunk_size: 64,
            max_chunk_size: 256,
            target_chunk_size: 128,
            window_size: 48,
        };
        let mut chunker = Chunker::new(config.clone());
        let data: Vec<u8> = (0..2000).map(|i| (i % 256) as u8).collect();
        let chunks = chunker.chunk_bytes(&data);
        assert!(chunks.len() > 1, "expected multiple chunks, got {}", chunks.len());

        let total_size: u32 = chunks.iter().map(|c| c.size).sum();
        assert_eq!(total_size, data.len() as u32);

        for chunk in &chunks {
            assert!(chunk.size <= config.max_chunk_size as u32);
        }

        let mut expected_index = 0u32;
        let mut expected_offset = 0u64;
        for chunk in &chunks {
            assert_eq!(chunk.index, expected_index);
            assert_eq!(chunk.offset, expected_offset);
            expected_index += 1;
            expected_offset += chunk.size as u64;
        }
    }

    #[test]
    fn test_deterministic_chunking() {
        let config = ChunkConfig {
            min_chunk_size: 64,
            max_chunk_size: 512,
            target_chunk_size: 256,
            window_size: 48,
        };
        let data: Vec<u8> = (0..5000).map(|i| (i % 256) as u8).collect();

        let mut chunker1 = Chunker::new(config.clone());
        let chunks1 = chunker1.chunk_bytes(&data);

        let mut chunker2 = Chunker::new(config);
        let chunks2 = chunker2.chunk_bytes(&data);

        assert_eq!(chunks1, chunks2);
    }

    #[test]
    fn test_boundary_conditions() {
        let config = ChunkConfig {
            min_chunk_size: 100,
            max_chunk_size: 200,
            target_chunk_size: 150,
            window_size: 48,
        };

        let mut chunker = Chunker::new(config.clone());
        let exactly_min: Vec<u8> = (0..100).map(|i| (i % 256) as u8).collect();
        let chunks = chunker.chunk_bytes(&exactly_min);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].size, 100);

        let mut chunker = Chunker::new(config.clone());
        let just_over_min: Vec<u8> = (0..101).map(|i| (i % 256) as u8).collect();
        let chunks = chunker.chunk_bytes(&just_over_min);
        assert_eq!(chunks.len(), 1);

        let mut chunker = Chunker::new(config.clone());
        let exactly_max: Vec<u8> = (0..200).map(|i| (i % 256) as u8).collect();
        let chunks = chunker.chunk_bytes(&exactly_max);
        assert!(chunks.len() >= 1);
        for chunk in &chunks {
            assert!(chunk.size <= 200);
        }
    }
}
