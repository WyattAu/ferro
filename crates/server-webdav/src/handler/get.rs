use crate::handler::WebdavHandlerContext;
use crate::handler::sniff_content_type;
use crate::range_get;
use axum::body::Body;
use axum::http::{HeaderMap, HeaderName, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use common::error::FerroError;
use common::error::Result;
use common::path::normalize_path;
use tokio::io::AsyncReadExt;
use tracing::debug;

pub(crate) async fn handle_get<S: crate::WebdavAppState>(
    state: &S,
    path: &str,
    headers: &HeaderMap,
) -> Result<Response> {
    let ctx = WebdavHandlerContext::new(state, normalize_path(path).to_string(), headers);
    ctx.validate_path()?;

    if !state.is_online() {
        let mut cache = state.offline_cache().write().await;
        if let Some(cached_data) = cache.get(&ctx.path) {
            debug!("OFFLINE GET: serving cached content for {}", ctx.path);
            let content_type = sniff_content_type(&cached_data, &ctx.path);
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
                range_get::accept_ranges_header(),
            );
            return Ok((StatusCode::OK, resp_headers, Body::from(cached_data)).into_response());
        }
        return Err(FerroError::NotFound(format!(
            "Resource not available offline: {}",
            ctx.path
        )));
    }

    let meta = state.storage().head(&ctx.path).await?;
    if meta.is_collection {
        return Err(FerroError::InvalidArgument("Cannot GET a collection".to_string()));
    }

    ctx.check_if_match(&meta.etag)?;

    if ctx.check_if_none_match(&meta.etag) {
        let mut resp_headers = HeaderMap::new();
        resp_headers.insert(
            "ETag",
            HeaderValue::from_str(&meta.etag).map_err(|e| FerroError::Internal(e.to_string()))?,
        );
        return Ok((StatusCode::NOT_MODIFIED, resp_headers, "").into_response());
    }

    let content_type = if meta.mime_type == "application/octet-stream" {
        sniff_content_type(&[], &ctx.path)
    } else {
        meta.mime_type.clone()
    };

    let etag_val = HeaderValue::from_str(&meta.etag).map_err(|e| FerroError::Internal(e.to_string()))?;
    let last_modified_val = HeaderValue::from_str(&meta.modified_at.format("%a, %d %b %Y %H:%M:%S GMT").to_string())
        .map_err(|e| FerroError::Internal(e.to_string()))?;
    let content_type_val = HeaderValue::from_str(&content_type).map_err(|e| FerroError::Internal(e.to_string()))?;

    if let Some(range_req) = range_get::parse_range_header(headers, meta.size)
        && let Some(spec) = range_req.ranges.first()
    {
        if let Some((start, end)) = spec.resolve(meta.size) {
            let mut reader = state.storage().get_stream(&ctx.path).await?;
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
            let range_headers = range_get::build_range_headers(start, end, meta.size);
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

    let reader = state.storage().get_stream(&ctx.path).await?;
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
        range_get::accept_ranges_header(),
    );

    Ok((StatusCode::OK, resp_headers, body).into_response())
}

pub(crate) async fn handle_head<S: crate::WebdavAppState>(
    state: &S,
    path: &str,
    headers: &HeaderMap,
) -> Result<Response> {
    let ctx = WebdavHandlerContext::new(state, normalize_path(path).to_string(), headers);
    ctx.validate_path()?;

    let meta = state.storage().head(&ctx.path).await?;

    ctx.check_if_match(&meta.etag)?;

    if ctx.check_if_none_match(&meta.etag) {
        let mut resp_headers = HeaderMap::new();
        resp_headers.insert(
            "ETag",
            HeaderValue::from_str(&meta.etag).map_err(|e| FerroError::Internal(e.to_string()))?,
        );
        return Ok((StatusCode::NOT_MODIFIED, resp_headers, "").into_response());
    }

    let mut resp_headers = HeaderMap::new();
    let content_type = if meta.mime_type == "application/octet-stream" {
        sniff_content_type(&[], &ctx.path)
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
        range_get::accept_ranges_header(),
    );

    Ok((StatusCode::OK, resp_headers, "").into_response())
}
