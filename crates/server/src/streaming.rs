//! Video streaming endpoint with Range header support.

use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::header::{self, HeaderMap, HeaderValue};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use bytes::Bytes;

use crate::api::normalize_api_path;
use crate::AppState;

/// Range header specification for byte range requests.
struct RangeHeader {
    start: u64,
    end: Option<u64>,
}

impl RangeHeader {
    fn parse(header_value: &str, total_size: u64) -> Option<Self> {
        let value = header_value.strip_prefix("bytes=")?;
        let parts: Vec<&str> = value.split(',').collect();
        if parts.is_empty() {
            return None;
        }

        let range_str = parts[0].trim();
        if let Some(suffix) = range_str.strip_prefix('-') {
            let suffix_len: u64 = suffix.parse().ok()?;
            if suffix_len == 0 {
                return None;
            }
            let start = total_size.saturating_sub(suffix_len);
            Some(RangeHeader {
                start,
                end: Some(total_size.saturating_sub(1)),
            })
        } else if let Some(dash_pos) = range_str.find('-') {
            let start_str = &range_str[..dash_pos];
            let end_str = &range_str[dash_pos + 1..];

            let start = if start_str.is_empty() {
                0
            } else {
                start_str.parse().ok()?
            };

            let end = if end_str.is_empty() {
                None
            } else {
                let e: u64 = end_str.parse().ok()?;
                Some(e)
            };

            Some(RangeHeader { start, end })
        } else {
            None
        }
    }

    fn resolve(&self, total_size: u64) -> Option<(u64, u64)> {
        let start = self.start;
        let end = self.end.unwrap_or(total_size.saturating_sub(1));

        if start > end || start >= total_size {
            return None;
        }

        let end = end.min(total_size.saturating_sub(1));
        Some((start, end))
    }
}

/// Guess MIME type from file path for video streaming.
fn guess_video_mime(path: &str) -> &'static str {
    let lower = path.to_lowercase();
    if lower.ends_with(".mp4") {
        "video/mp4"
    } else if lower.ends_with(".webm") {
        "video/webm"
    } else if lower.ends_with(".ogg") || lower.ends_with(".ogv") {
        "video/ogg"
    } else if lower.ends_with(".mov") {
        "video/quicktime"
    } else if lower.ends_with(".m4v") {
        "video/x-m4v"
    } else if lower.ends_with(".mkv") {
        "video/x-matroska"
    } else if lower.ends_with(".avi") {
        "video/x-msvideo"
    } else {
        "application/octet-stream"
    }
}

/// GET /api/stream/{path} — Stream video with Range header support (206 Partial Content).
pub async fn stream_video(
    State(state): State<AppState>,
    Path(path): Path<String>,
    headers: HeaderMap,
) -> Response {
    let path = match normalize_api_path(&path) {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                axum::Json(serde_json::json!({
                    "error": "invalid_path",
                    "message": e,
                })),
            )
                .into_response();
        }
    };

    let meta = match state.storage.head(&path).await {
        Ok(m) => m,
        Err(e) => {
            return (
                StatusCode::NOT_FOUND,
                axum::Json(serde_json::json!({
                    "error": "not_found",
                    "message": e.to_string(),
                })),
            )
                .into_response();
        }
    };

    let total_size = meta.size;
    let content_type = if meta.mime_type == "application/octet-stream" {
        guess_video_mime(&path)
    } else {
        &meta.mime_type
    };

    let mut response_headers = HeaderMap::new();
    response_headers.insert(header::CONTENT_TYPE, HeaderValue::from_str(content_type).unwrap());
    response_headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!(
            "inline; filename=\"{}\"",
            path.rsplit('/').next().unwrap_or("video")
        ))
        .unwrap(),
    );
    response_headers.insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    response_headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=3600"),
    );

    // Check for Range header
    if let Some(range_value) = headers.get(header::RANGE) {
        let range_str = range_value.to_str().unwrap_or("");
        if let Some(range) = RangeHeader::parse(range_str, total_size) {
            if let Some((start, end)) = range.resolve(total_size) {
                let content_length = end - start + 1;

                let mut partial_headers = response_headers.clone();
                partial_headers.insert(
                    header::CONTENT_LENGTH,
                    HeaderValue::from_str(&content_length.to_string()).unwrap(),
                );
                partial_headers.insert(
                    header::CONTENT_RANGE,
                    HeaderValue::from_str(&format!(
                        "bytes {}-{}/{}",
                        start,
                        end,
                        total_size
                    ))
                    .unwrap(),
                );

                // Stream the requested range
                match state.storage.get_stream(&path).await {
                    Ok(mut reader) => {
                        // Read the requested range into memory
                        let mut data = Vec::with_capacity(content_length as usize);
                        let mut buf = [0u8; 64 * 1024];
                        let mut remaining = content_length;
                        loop {
                            if remaining == 0 {
                                break;
                            }
                            let to_read = std::cmp::min(remaining, buf.len() as u64) as usize;
                            match tokio::io::AsyncReadExt::read(&mut reader, &mut buf[..to_read])
                                .await
                            {
                                Ok(0) => break,
                                Ok(n) => {
                                    remaining -= n as u64;
                                    data.extend_from_slice(&buf[..n]);
                                }
                                Err(_) => break,
                            }
                        }

                        (
                            partial_headers,
                            Bytes::from(data),
                        )
                            .into_response()
                    }
                    Err(e) => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        axum::Json(serde_json::json!({
                            "error": "storage_error",
                            "message": e.to_string(),
                        })),
                    )
                        .into_response(),
                }
            } else {
                (StatusCode::RANGE_NOT_SATISFIABLE).into_response()
            }
        } else {
            // Invalid Range header, serve full content
            serve_full_content(state, path, content_type, total_size, response_headers).await
        }
    } else {
        // No Range header, serve full content
        serve_full_content(state, path, content_type, total_size, response_headers).await
    }
}

async fn serve_full_content(
    state: AppState,
    path: String,
    _content_type: &str,
    total_size: u64,
    mut headers: HeaderMap,
) -> Response {
    headers.insert(
        header::CONTENT_LENGTH,
        HeaderValue::from_str(&total_size.to_string()).unwrap(),
    );

    match state.storage.get_stream(&path).await {
        Ok(reader) => {
            let stream = tokio_util::io::ReaderStream::new(reader);
            let body = Body::from_stream(stream);

            (headers, body).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(serde_json::json!({
                "error": "storage_error",
                "message": e.to_string(),
            })),
        )
            .into_response(),
    }
}

/// GET /api/stream/{path}/manifest.m3u8 — HLS manifest stub (returns 501 if no transcoding).
pub async fn hls_manifest(
    State(_state): State<AppState>,
    Path(_path): Path<String>,
) -> Response {
    // HLS transcoding is not yet implemented
    (
        StatusCode::NOT_IMPLEMENTED,
        axum::Json(serde_json::json!({
            "error": "not_implemented",
            "message": "HLS transcoding is not yet available",
        })),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_range_header_parse_simple() {
        let range = RangeHeader::parse("bytes=0-499", 1000).unwrap();
        assert_eq!(range.start, 0);
        assert_eq!(range.end, Some(499));
    }

    #[test]
    fn test_range_header_parse_from_offset() {
        let range = RangeHeader::parse("bytes=500-", 1000).unwrap();
        assert_eq!(range.start, 500);
        assert_eq!(range.end, None);
    }

    #[test]
    fn test_range_header_parse_suffix() {
        let range = RangeHeader::parse("bytes=-500", 1000).unwrap();
        assert_eq!(range.start, 500);
        assert_eq!(range.end, Some(999));
    }

    #[test]
    fn test_range_header_resolve() {
        let range = RangeHeader {
            start: 100,
            end: Some(499),
        };
        assert_eq!(range.resolve(1000), Some((100, 499)));
    }

    #[test]
    fn test_range_header_resolve_beyond_size() {
        let range = RangeHeader {
            start: 0,
            end: Some(2000),
        };
        assert_eq!(range.resolve(1000), Some((0, 999)));
    }

    #[test]
    fn test_guess_video_mime() {
        assert_eq!(guess_video_mime("test.mp4"), "video/mp4");
        assert_eq!(guess_video_mime("test.webm"), "video/webm");
        assert_eq!(guess_video_mime("test.mov"), "video/quicktime");
    }
}
