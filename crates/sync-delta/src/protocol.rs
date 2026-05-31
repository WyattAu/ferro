use serde::{Deserialize, Serialize};
use crate::chunker::ChunkInfo;
use crate::diff::BlockDiffResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncMessage {
    ChunkRequest {
        chunk_hashes: Vec<[u8; 32]>,
    },
    ChunkResponse {
        chunks: Vec<(Vec<u8>, ChunkInfo)>,
    },
    DiffRequest {
        local_chunks: Vec<ChunkInfo>,
    },
    DiffResponse {
        diff: BlockDiffResult,
    },
    VersionCheck {
        file_path: String,
        chunk_count: u32,
        total_size: u64,
    },
}
