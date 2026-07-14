use crate::WebDavCoreState;
use axum::body::Body;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use common::error::{FerroError, Result};
use common::path::normalize_path;
use common::webdav::LockDepth;
use tracing::debug;

pub(crate) async fn handle_lock<S: WebDavCoreState>(
    state: S,
    path: &str,
    headers: &HeaderMap,
    body: &Bytes,
) -> Result<Response> {
    let path = normalize_path(path);

    if !common::path::validate_path(&path) {
        return Err(FerroError::InvalidArgument(format!("Invalid path: {}", path)));
    }

    // RFC 4918 §9.10.2: If the request includes an If header with a lock token,
    // this is a lock refresh request, not a new lock acquisition.
    if let Some(if_header) = headers.get("If")
        && let Some(lock_token) = extract_lock_token_from_if(if_header)
    {
        return handle_lock_refresh(state, &path, &lock_token, headers, body).await;
    }

    let lock_request = ferro_webdav_handler::LockRequest::parse(body);

    let depth = headers
        .get("Depth")
        .and_then(|v| v.to_str().ok())
        .map(LockDepth::from_header)
        .unwrap_or(lock_request.depth);

    let principal = lock_request.owner.clone().unwrap_or_else(|| {
        headers
            .get("X-Ferro-User")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("anonymous")
            .to_string()
    });

    let lock = state
        .lock_manager()
        .acquire_lock(&path, &principal, lock_request.scope, depth, lock_request.timeout_hint)
        .await?;

    let lock_token = lock.token.as_str();
    let xml = ferro_webdav_handler::build_lock_response_xml(
        &lock_token,
        depth.to_header(),
        &principal,
        lock.timeout_seconds,
        &path,
    );

    let mut resp_headers = HeaderMap::new();
    resp_headers.insert(
        "Content-Type",
        HeaderValue::from_static("application/xml; charset=utf-8"),
    );
    resp_headers.insert(
        "Lock-Token",
        HeaderValue::from_str(&format!("<{}>", lock_token)).map_err(|e| FerroError::Internal(e.to_string()))?,
    );

    Ok((StatusCode::OK, resp_headers, Body::from(xml)).into_response())
}

pub(crate) async fn handle_unlock<S: WebDavCoreState>(state: S, _path: &str, headers: &HeaderMap) -> Result<Response> {
    let lock_token = headers
        .get("Lock-Token")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("<").and_then(|r| r.strip_suffix(">")))
        .ok_or_else(|| FerroError::InvalidArgument("Missing Lock-Token header".to_string()))?;

    state.lock_manager().release_lock(lock_token).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

/// LOCK refresh (RFC 4918 §9.10.2): client sends LOCK with If header containing lock token.
async fn handle_lock_refresh<S: WebDavCoreState>(
    state: S,
    path: &str,
    lock_token: &str,
    headers: &HeaderMap,
    body: &Bytes,
) -> Result<Response> {
    let lock_request = ferro_webdav_handler::LockRequest::parse(body);

    match state
        .lock_manager()
        .refresh_lock(lock_token, lock_request.timeout_hint)
        .await
    {
        Ok(lock) => {
            let principal = lock.principal.clone();
            let xml = ferro_webdav_handler::build_lock_response_xml(
                lock_token,
                lock.depth.to_header(),
                &principal,
                lock.timeout_seconds,
                path,
            );

            let mut resp_headers = HeaderMap::new();
            resp_headers.insert(
                "Content-Type",
                HeaderValue::from_static("application/xml; charset=utf-8"),
            );
            resp_headers.insert(
                "Lock-Token",
                HeaderValue::from_str(&format!("<{}>", lock_token)).map_err(|e| FerroError::Internal(e.to_string()))?,
            );

            debug!("LOCK refreshed: {} token={}", path, lock_token);
            Ok((StatusCode::OK, resp_headers, Body::from(xml)).into_response())
        }
        Err(_) => {
            // Lock not found or expired — treat as new lock request
            debug!("LOCK refresh failed for {}, treating as new lock", lock_token);
            let lock_request = ferro_webdav_handler::LockRequest::parse(body);
            let depth = headers
                .get("Depth")
                .and_then(|v| v.to_str().ok())
                .map(common::webdav::LockDepth::from_header)
                .unwrap_or(lock_request.depth);

            let principal = lock_request.owner.clone().unwrap_or_else(|| {
                headers
                    .get("X-Ferro-User")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("anonymous")
                    .to_string()
            });

            let lock = state
                .lock_manager()
                .acquire_lock(path, &principal, lock_request.scope, depth, lock_request.timeout_hint)
                .await?;

            let lock_token = lock.token.as_str();
            let xml = ferro_webdav_handler::build_lock_response_xml(
                &lock_token,
                depth.to_header(),
                &principal,
                lock.timeout_seconds,
                path,
            );

            let mut resp_headers = HeaderMap::new();
            resp_headers.insert(
                "Content-Type",
                HeaderValue::from_static("application/xml; charset=utf-8"),
            );
            resp_headers.insert(
                "Lock-Token",
                HeaderValue::from_str(&format!("<{}>", lock_token)).map_err(|e| FerroError::Internal(e.to_string()))?,
            );

            Ok((StatusCode::OK, resp_headers, Body::from(xml)).into_response())
        }
    }
}

/// Extract a lock token from an If header value.
/// Format: `(<lock-token>)` or `( <lock-token> )`
fn extract_lock_token_from_if(if_header: &HeaderValue) -> Option<String> {
    let val = if_header.to_str().ok()?;
    let trimmed = val.trim();
    // Extract content between angle brackets
    let start = trimmed.find('<')?;
    let end = trimmed.find('>')?;
    if end > start {
        Some(trimmed[start + 1..end].to_string())
    } else {
        None
    }
}
