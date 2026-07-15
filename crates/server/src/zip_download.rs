use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use serde::Deserialize;
use std::io::Write;
use tracing::instrument;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

use crate::AppState;
use crate::api_error::ApiError;

#[derive(Debug, Deserialize)]
pub struct ZipDownloadRequest {
    pub paths: Vec<String>,
}

#[instrument(name = "zip_download", skip(state))]
pub async fn zip_download(State(state): State<AppState>, axum::Json(body): axum::Json<ZipDownloadRequest>) -> Response {
    if body.paths.is_empty() {
        return ApiError::bad_request(ApiError::BAD_REQUEST, "No paths provided");
    }

    let mut zip_buffer = Vec::new();
    {
        let mut writer = ZipWriter::new(std::io::Cursor::new(&mut zip_buffer));
        let options = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .compression_level(Some(6));

        for path in &body.paths {
            let path = path.trim_start_matches('/');
            if path.is_empty() {
                continue;
            }

            match state.storage.get(path).await {
                Ok(content) => {
                    let name = path.to_string();
                    if let Err(e) = writer.start_file(name, options) {
                        tracing::error!("Failed to start ZIP entry: {}", e);
                        continue;
                    }
                    if let Err(e) = writer.write_all(&content) {
                        tracing::error!("Failed to write ZIP entry: {}", e);
                        continue;
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to read file '{}' for ZIP: {}", path, e);
                }
            }
        }

        if let Err(e) = writer.finish() {
            return ApiError::internal(ApiError::INTERNAL_ERROR, format!("Failed to finalize ZIP: {}", e));
        }
    }

    let zip_bytes = Bytes::from(zip_buffer);
    let filename = if body.paths.len() == 1 {
        let path = body.paths[0].trim_start_matches('/');
        let name = path.rsplit('/').next().unwrap_or(path);
        format!("{}.zip", name)
    } else {
        "download.zip".to_string()
    };

    (
        StatusCode::OK,
        [
            (axum::http::header::CONTENT_TYPE, "application/zip"),
            (
                axum::http::header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{}\"", filename).as_str(),
            ),
        ],
        zip_bytes,
    )
        .into_response()
}
