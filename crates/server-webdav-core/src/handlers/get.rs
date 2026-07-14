use crate::WebDavCoreState;
use axum::body::Body;
use axum::http::{HeaderMap, HeaderName, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use common::error::{FerroError, Result};
use common::path::normalize_path;
use tokio::io::AsyncReadExt;
use tracing::debug;

use super::{check_conditional_if_match, check_if_none_match};

pub(crate) async fn handle_get<S: WebDavCoreState>(state: S, path: &str, headers: &HeaderMap) -> Result<Response> {
    let path = normalize_path(path);

    if !common::path::validate_path(&path) {
        return Err(FerroError::InvalidArgument(format!("Invalid path: {}", path)));
    }

    // Offline-first: check content cache before hitting storage
    if !state.is_online() {
        let mut cache = state.offline_cache().write().await;
        if let Some(cached_data) = cache.get(&path) {
            debug!("OFFLINE GET: serving cached content for {}", path);
            let content_type = common::mime::sniff_content_type(&cached_data, &path);
            let etag = format!("\"{}\"", common::metadata::ContentHash::compute(&cached_data).as_str());
            let mut resp_headers = HeaderMap::new();
            resp_headers.insert(
                "Content-Type",
                HeaderValue::from_str(&content_type).map_err(|e| FerroError::Internal(e.to_string()))?,
            );
            resp_headers.insert(
                "Content-Length",
                HeaderValue::from_str(&cached_data.len().to_string())
                    .map_err(|e| FerroError::Internal(e.to_string()))?,
            );
            resp_headers.insert(
                "ETag",
                HeaderValue::from_str(&etag).map_err(|e| FerroError::Internal(e.to_string()))?,
            );
            resp_headers.insert(
                HeaderName::from_static("accept-ranges"),
                ferro_server_storage_ops::range_get::accept_ranges_header(),
            );
            return Ok((StatusCode::OK, resp_headers, Body::from(cached_data)).into_response());
        }
        return Err(FerroError::NotFound(format!(
            "Resource not available offline: {}",
            path
        )));
    }

    let meta = state.storage().head(&path).await?;
    if meta.is_collection {
        return Err(FerroError::InvalidArgument("Cannot GET a collection".to_string()));
    }

    // Conditional GET: If-Match
    check_conditional_if_match(headers, &meta.etag)?;

    // Check If-None-Match for 304 Not Modified (must be before reading content)
    if check_if_none_match(headers, &meta.etag) {
        let mut resp_headers = HeaderMap::new();
        resp_headers.insert(
            "ETag",
            HeaderValue::from_str(&meta.etag).map_err(|e| FerroError::Internal(e.to_string()))?,
        );
        return Ok((StatusCode::NOT_MODIFIED, resp_headers, "").into_response());
    }

    let content_type = if meta.mime_type == "application/octet-stream" {
        common::mime::sniff_content_type(&[], &path)
    } else {
        meta.mime_type.clone()
    };

    let etag_val = HeaderValue::from_str(&meta.etag).map_err(|e| FerroError::Internal(e.to_string()))?;
    let last_modified_val = HeaderValue::from_str(&meta.modified_at.format("%a, %d %b %Y %H:%M:%S GMT").to_string())
        .map_err(|e| FerroError::Internal(e.to_string()))?;
    let content_type_val = HeaderValue::from_str(&content_type).map_err(|e| FerroError::Internal(e.to_string()))?;

    if let Some(range_req) = ferro_server_storage_ops::range_get::parse_range_header(headers, meta.size)
        && let Some(spec) = range_req.ranges.first()
    {
        if let Some((start, end)) = spec.resolve(meta.size) {
            let mut reader = state.storage().get_stream(&path).await?;
            {
                let mut buf = [0u8; 8192];
                let mut remaining = start;
                while remaining > 0 {
                    let n = std::cmp::min(remaining, buf.len() as u64);
                    reader
                        .read_exact(&mut buf[..n as usize])
                        .await
                        .map_err(|e| FerroError::Internal(e.to_string()))?;
                    remaining -= n;
                }
            }
            let take_reader = reader.take(end - start + 1);
            let stream = tokio_util::io::ReaderStream::new(take_reader);
            let body = Body::from_stream(stream);

            let mut resp_headers = HeaderMap::new();
            resp_headers.insert("Content-Type", content_type_val);
            resp_headers.insert("ETag", etag_val);
            resp_headers.insert("Last-Modified", last_modified_val);
            let range_headers = ferro_server_storage_ops::range_get::build_range_headers(start, end, meta.size);
            for (k, v) in range_headers.iter() {
                resp_headers.insert(k.clone(), v.clone());
            }
            return Ok((StatusCode::PARTIAL_CONTENT, resp_headers, body).into_response());
        } else {
            let mut resp_headers = HeaderMap::new();
            resp_headers.insert(
                HeaderName::from_static("content-range"),
                HeaderValue::from_str(&format!("bytes */{}", meta.size))
                    .unwrap_or_else(|_| HeaderValue::from_static("bytes */0")),
            );
            return Ok((StatusCode::RANGE_NOT_SATISFIABLE, resp_headers, "").into_response());
        }
    }

    let reader = state.storage().get_stream(&path).await?;
    let stream = tokio_util::io::ReaderStream::new(reader);
    let body = Body::from_stream(stream);

    let mut resp_headers = HeaderMap::new();
    resp_headers.insert("Content-Type", content_type_val);
    resp_headers.insert(
        "Content-Length",
        HeaderValue::from_str(&meta.size.to_string()).map_err(|e| FerroError::Internal(e.to_string()))?,
    );
    resp_headers.insert("ETag", etag_val);
    resp_headers.insert("Last-Modified", last_modified_val);
    resp_headers.insert(
        HeaderName::from_static("accept-ranges"),
        ferro_server_storage_ops::range_get::accept_ranges_header(),
    );

    Ok((StatusCode::OK, resp_headers, body).into_response())
}

pub(crate) async fn handle_head<S: WebDavCoreState>(state: S, path: &str, headers: &HeaderMap) -> Result<Response> {
    let path = normalize_path(path);

    if !common::path::validate_path(&path) {
        return Err(FerroError::InvalidArgument(format!("Invalid path: {}", path)));
    }

    let meta = state.storage().head(&path).await?;

    // Conditional HEAD
    check_conditional_if_match(headers, &meta.etag)?;

    if check_if_none_match(headers, &meta.etag) {
        let mut resp_headers = HeaderMap::new();
        resp_headers.insert(
            "ETag",
            HeaderValue::from_str(&meta.etag).map_err(|e| FerroError::Internal(e.to_string()))?,
        );
        return Ok((StatusCode::NOT_MODIFIED, resp_headers, "").into_response());
    }

    let mut resp_headers = HeaderMap::new();
    // Re-detect MIME from extension if stored value is the generic default.
    let content_type = if meta.mime_type == "application/octet-stream" {
        common::mime::sniff_content_type(&[], &path)
    } else {
        meta.mime_type.clone()
    };
    resp_headers.insert(
        "Content-Type",
        HeaderValue::from_str(&content_type).map_err(|e| FerroError::Internal(e.to_string()))?,
    );
    resp_headers.insert(
        "Content-Length",
        HeaderValue::from_str(&meta.size.to_string()).map_err(|e| FerroError::Internal(e.to_string()))?,
    );
    resp_headers.insert(
        "ETag",
        HeaderValue::from_str(&meta.etag).map_err(|e| FerroError::Internal(e.to_string()))?,
    );
    resp_headers.insert(
        "Last-Modified",
        HeaderValue::from_str(&meta.modified_at.format("%a, %d %b %Y %H:%M:%S GMT").to_string())
            .map_err(|e| FerroError::Internal(e.to_string()))?,
    );
    resp_headers.insert(
        HeaderName::from_static("accept-ranges"),
        ferro_server_storage_ops::range_get::accept_ranges_header(),
    );

    Ok((StatusCode::OK, resp_headers, "").into_response())
}
