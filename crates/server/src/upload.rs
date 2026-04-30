use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::AppState;

#[derive(Debug, Clone)]
pub struct ChunkedUpload {
    pub path: String,
    pub chunk_size: usize,
    pub received_chunks: HashMap<usize, Vec<u8>>,
    pub total_chunks: Option<usize>,
    pub created_at: std::time::Instant,
}

pub type UploadStore = Arc<RwLock<HashMap<String, ChunkedUpload>>>;

#[derive(Serialize)]
pub struct InitUploadResponse {
    upload_id: String,
    chunk_size: usize,
}

#[derive(Deserialize)]
pub struct InitUploadRequest {
    path: String,
    total_size: Option<u64>,
    chunk_size: Option<usize>,
}

#[derive(Deserialize)]
pub struct CompleteUploadRequest {
    path: Option<String>,
}

pub async fn init_upload(
    State(state): State<AppState>,
    Json(req): Json<InitUploadRequest>,
) -> (StatusCode, Json<InitUploadResponse>) {
    let chunk_size = req.chunk_size.unwrap_or(5 * 1024 * 1024);
    let upload_id = format!("ul_{}", uuid::Uuid::new_v4().simple());

    let upload = ChunkedUpload {
        path: req.path,
        chunk_size,
        received_chunks: HashMap::new(),
        total_chunks: req
            .total_size
            .map(|s| (s as usize).div_ceil(chunk_size)),
        created_at: std::time::Instant::now(),
    };

    state.upload_store.write().await.insert(upload_id.clone(), upload);

    (StatusCode::OK, Json(InitUploadResponse { upload_id, chunk_size }))
}

pub async fn upload_chunk(
    State(state): State<AppState>,
    Path((upload_id, chunk_index)): Path<(String, usize)>,
    bytes: axum::body::Bytes,
) -> StatusCode {
    let mut store = state.upload_store.write().await;

    match store.get_mut(&upload_id) {
        Some(upload) => {
            if bytes.len() > upload.chunk_size {
                return StatusCode::PAYLOAD_TOO_LARGE;
            }
            upload.received_chunks.insert(chunk_index, bytes.to_vec());
            StatusCode::OK
        }
        None => StatusCode::NOT_FOUND,
    }
}

pub async fn complete_upload(
    State(state): State<AppState>,
    Path(upload_id): Path<String>,
    Json(req): Json<CompleteUploadRequest>,
) -> StatusCode {
    let mut store = state.upload_store.write().await;

    match store.remove(&upload_id) {
        Some(upload) => {
            let path = req.path.unwrap_or(upload.path);

            let max_chunk = upload.received_chunks.keys().copied().max().unwrap_or(0);
            let total_chunks = upload.total_chunks.unwrap_or(max_chunk + 1);

            let mut data = Vec::with_capacity(total_chunks * upload.chunk_size);
            for i in 0..total_chunks {
                match upload.received_chunks.get(&i) {
                    Some(chunk) => data.extend_from_slice(chunk),
                    None => return StatusCode::BAD_REQUEST,
                }
            }

            match state.storage.put(&path, bytes::Bytes::from(data), "anonymous").await {
                Ok(_) => StatusCode::CREATED,
                Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
            }
        }
        None => StatusCode::NOT_FOUND,
    }
}

pub async fn cancel_upload(
    State(state): State<AppState>,
    Path(upload_id): Path<String>,
) -> StatusCode {
    state.upload_store.write().await.remove(&upload_id);
    StatusCode::NO_CONTENT
}

pub async fn list_uploads(State(state): State<AppState>) -> Json<Vec<serde_json::Value>> {
    let store = state.upload_store.read().await;
    let uploads: Vec<_> = store
        .iter()
        .map(|(id, upload)| {
            serde_json::json!({
                "upload_id": id,
                "path": upload.path,
                "chunk_size": upload.chunk_size,
                "received": upload.received_chunks.len(),
                "total_chunks": upload.total_chunks,
                "elapsed_secs": upload.created_at.elapsed().as_secs(),
            })
        })
        .collect();
    Json(uploads)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunked_upload_struct() {
        let upload = ChunkedUpload {
            path: "/test.bin".to_string(),
            chunk_size: 1024,
            received_chunks: HashMap::new(),
            total_chunks: Some(3),
            created_at: std::time::Instant::now(),
        };

        assert_eq!(upload.chunk_size, 1024);
        assert_eq!(upload.total_chunks, Some(3));
        assert!(upload.received_chunks.is_empty());
    }

    #[tokio::test]
    async fn test_upload_store_lifecycle() {
        let store: UploadStore = Arc::new(RwLock::new(HashMap::new()));

        {
            let mut s = store.write().await;
            s.insert(
                "ul_test".to_string(),
                ChunkedUpload {
                    path: "/test".to_string(),
                    chunk_size: 1024,
                    received_chunks: HashMap::new(),
                    total_chunks: Some(2),
                    created_at: std::time::Instant::now(),
                },
            );
        }

        {
            let s = store.read().await;
            assert!(s.contains_key("ul_test"));
        }

        {
            let mut s = store.write().await;
            s.remove("ul_test");
        }

        {
            let s = store.read().await;
            assert!(!s.contains_key("ul_test"));
        }
    }

    #[test]
    fn test_chunk_size_default() {
        let req = InitUploadRequest {
            path: "/test".to_string(),
            total_size: Some(15 * 1024 * 1024),
            chunk_size: None,
        };
        assert_eq!(req.chunk_size, None);
    }

    #[test]
    fn test_total_chunks_calculation() {
        let total_size = 15_000_000u64;
        let chunk_size = 5_000_000usize;
        let total_chunks = (total_size as usize + chunk_size - 1) / chunk_size;
        assert_eq!(total_chunks, 3);
    }

    #[test]
    fn test_exact_chunk_boundary() {
        let total_size = 10_000_000u64;
        let chunk_size = 5_000_000usize;
        let total_chunks = (total_size as usize + chunk_size - 1) / chunk_size;
        assert_eq!(total_chunks, 2);
    }

    #[test]
    fn test_small_file_single_chunk() {
        let total_size = 1000u64;
        let chunk_size = 5_000_000usize;
        let total_chunks = (total_size as usize + chunk_size - 1) / chunk_size;
        assert_eq!(total_chunks, 1);
    }
}
