use serde::{Deserialize, Serialize};
use std::collections::HashSet;

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
}
