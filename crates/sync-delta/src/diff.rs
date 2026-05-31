use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use crate::chunker::ChunkInfo;

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

pub fn compute_block_diff(request: &BlockDiffRequest) -> BlockDiffResult {
    let local_hashes: HashSet<[u8; 32]> = request
        .local_chunks
        .iter()
        .map(|c| c.hash)
        .collect();

    let new_hashes: HashSet<[u8; 32]> = request
        .new_chunks
        .iter()
        .map(|c| c.hash)
        .collect();

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
        let new_chunks = vec![
            make_chunk(10, 0, 100, 0),
            make_chunk(20, 100, 200, 1),
        ];
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
        let local = vec![
            make_chunk(1, 0, 100, 0),
            make_chunk(2, 100, 200, 1),
        ];
        let new_chunks = vec![
            make_chunk(3, 0, 150, 0),
            make_chunk(4, 150, 100, 1),
        ];
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
}
