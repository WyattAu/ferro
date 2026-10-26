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

    let existed = state.storage().exists(&path).await.unwrap_or(false);

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

    let status = if existed {
        StatusCode::OK
    } else {
        // LOCK on an unmapped URL creates an empty resource (RFC 4918 §7.3).
        StatusCode::CREATED
    };
    Ok((status, resp_headers, Body::from(xml)).into_response())
}

pub(crate) async fn handle_unlock<S: WebDavCoreState>(state: S, path: &str, headers: &HeaderMap) -> Result<Response> {
    let lock_token = headers
        .get("Lock-Token")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("<").and_then(|r| r.strip_suffix(">")))
        .ok_or_else(|| FerroError::InvalidArgument("Missing Lock-Token header".to_string()))?;

    // RFC 4918 §9.11: the request-URI MUST identify the resource the lock
    // was taken on (precondition lock-token-matches-request-uri). Never
    // release a lock whose path does not match the request path.
    let locks = state.lock_manager().all_locks().await;
    let lock = locks
        .iter()
        .find(|l| l.token.as_str() == lock_token || l.token.as_opaque() == lock_token);
    let Some(lock) = lock else {
        return Ok((
            StatusCode::CONFLICT,
            [("Allow", "LOCK, UNLOCK")],
            Body::from("no active lock for the submitted token"),
        )
            .into_response());
    };
    let normalized = normalize_path(path);
    if lock.path != normalized && lock.path != path {
        return Ok((
            StatusCode::CONFLICT,
            [("Allow", "LOCK, UNLOCK")],
            Body::from("lock-token-matches-request-uri precondition failed"),
        )
            .into_response());
    }

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
/// RFC 4918 §10.4: full parse; the first non-negated token refreshes.
fn extract_lock_token_from_if(if_header: &HeaderValue) -> Option<String> {
    let val = if_header.to_str().ok()?;
    crate::ifheader::parse(val)
        .map(|h| h.submitted_tokens())
        .and_then(|tokens| tokens.into_iter().next())
}
