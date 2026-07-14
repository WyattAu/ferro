use crate::{
    WebDavCoreState,
    handlers::{
        copy_move::{handle_copy, handle_move},
        delete::handle_delete,
        get::{handle_get, handle_head},
        lock::{handle_lock, handle_unlock},
        mkcol::handle_mkcol,
        options::handle_options,
        propfind::handle_propfind,
        proppatch::handle_proppatch,
        put::handle_put_dispatch,
    },
};
use axum::body::Body;
use axum::extract::{Path as AxumPath, State};
use axum::http::{HeaderMap, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use common::error::{FerroError, Result};
use common::path::normalize_path;
use ferro_webdav_handler::escape_xml;
use http_body_util::BodyExt;
use tracing::{debug, instrument, warn};

pub fn sanitize_path(path: &str) -> Result<String> {
    if path.contains('\0') {
        return Err(FerroError::InvalidArgument("Path contains null bytes".to_string()));
    }

    for component in std::path::Path::new(path).components() {
        match component {
            std::path::Component::ParentDir => {
                return Err(FerroError::InvalidArgument(
                    "Path traversal detected: '..' not allowed".to_string(),
                ));
            }
            std::path::Component::CurDir => {
                return Err(FerroError::InvalidArgument("Path contains '.' component".to_string()));
            }
            _ => {}
        }
    }

    let normalized = normalize_path(path);
    Ok(normalized.to_string())
}

/// Maximum recursion depth for PROPFIND depth:infinity to prevent DoS.
pub(crate) const MAX_PROPFIND_DEPTH: u32 = 100;
pub(crate) const MAX_RECENTLY_PROCESSED: usize = 10_000;

#[instrument(name = "webdav", skip(state, headers, body), fields(method = %method, uri = %uri))]
pub async fn handle_any<S: WebDavCoreState>(
    method: Method,
    uri: axum::http::Uri,
    State(state): State<S>,
    path: Option<AxumPath<String>>,
    headers: HeaderMap,
    body: Body,
) -> Response {
    let raw_path = match path {
        Some(AxumPath(p)) => format!("/{}", p),
        None => uri.path().to_string(),
    };

    let path_str = match sanitize_path(&raw_path) {
        Ok(p) => p,
        Err(e) => {
            warn!("Path sanitization failed for '{}': {}", raw_path, e);
            let status = StatusCode::from_u16(e.status_code()).unwrap_or(StatusCode::BAD_REQUEST);
            return (
                status,
                axum::Json(serde_json::json!({
                    "error": e.to_string(),
                })),
            )
                .into_response();
        }
    };
    debug!("{} {}", method, path_str);

    // Enforce body size limit using Content-Length header (defense-in-depth;
    // axum's DefaultBodyLimit layer is the primary enforcement).
    if let Some(content_len) = headers
        .get("Content-Length")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
        && content_len > state.max_body_size()
    {
        return (
            StatusCode::PAYLOAD_TOO_LARGE,
            axum::Json(serde_json::json!({
                "error": "Request body too large",
                "size": content_len,
                "max": state.max_body_size(),
            })),
        )
            .into_response();
    }

    let user_sub = headers.get("X-Ferro-User").and_then(|v| v.to_str().ok());
    let resolved_path = match user_sub {
        Some(sub) if sub != "anonymous" => {
            let user_root = format!("/users/{}", sub);
            if path_str == "/" || path_str.is_empty() {
                user_root
            } else {
                format!("{}{}", user_root, path_str)
            }
        }
        _ => path_str.clone(),
    };

    let result: Result<Response> = async {
        // Quota enforcement for PUT (best-effort pre-check using Content-Length header)
        if method.as_str() == "PUT"
            && let Some(content_len) = headers
                .get("Content-Length")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok())
            && let Err(quota_resp) = state.enforce_quota(content_len).await
        {
            return Ok(quota_resp);
        }

        match method.as_str() {
            "OPTIONS" => handle_options(&resolved_path).await,
            "PROPFIND" => handle_propfind(state, &resolved_path, &headers).await,
            "GET" => handle_get(state, &resolved_path, &headers).await,
            "HEAD" => handle_head(state, &resolved_path, &headers).await,
            "PUT" => handle_put_dispatch(state, &resolved_path, &headers, body).await,
            "DELETE" => handle_delete(state, &resolved_path, &headers).await,
            "MKCOL" => handle_mkcol(state, &resolved_path).await,
            "COPY" => handle_copy(state, &resolved_path, &headers).await,
            "MOVE" => handle_move(state, &resolved_path, &headers).await,
            "LOCK" => {
                let bytes = body
                    .collect()
                    .await
                    .map_err(|e| FerroError::Internal(format!("body read: {}", e)))?
                    .to_bytes();
                handle_lock(state, &resolved_path, &headers, &bytes).await
            }
            "UNLOCK" => handle_unlock(state, &resolved_path, &headers).await,
            "PROPPATCH" => {
                let bytes = body
                    .collect()
                    .await
                    .map_err(|e| FerroError::Internal(format!("body read: {}", e)))?
                    .to_bytes();
                handle_proppatch(state, &resolved_path, &headers, &bytes).await
            }
            "MKCALENDAR" | "REPORT" if resolved_path.starts_with("/dav/cal") => {
                let bytes = body
                    .collect()
                    .await
                    .map_err(|e| FerroError::Internal(format!("body read: {}", e)))?
                    .to_bytes();
                let m = method.clone();
                Ok(crate::dav::dispatch_caldav(state, &m, &resolved_path, bytes).await)
            }
            "REPORT" if resolved_path.starts_with("/dav/card") => {
                let bytes = body
                    .collect()
                    .await
                    .map_err(|e| FerroError::Internal(format!("body read: {}", e)))?
                    .to_bytes();
                let m = method.clone();
                Ok(crate::dav::dispatch_carddav(state, &m, &resolved_path, bytes).await)
            }
            _ => Err(FerroError::InvalidArgument(format!("Method {} not supported", method))),
        }
    }
    .await;

    match result {
        Ok(response) => response,
        Err(e) => {
            warn!("Error handling {} {}: {}", method, path_str, e);
            let status = StatusCode::from_u16(e.status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            let xml = format!(
                r#"<?xml version="1.0" encoding="utf-8"?><d:error xmlns:d="DAV:"><s:message>{}</s:message></d:error>"#,
                escape_xml(&e.to_string())
            );
            (status, xml).into_response()
        }
    }
}

/// Extract the owner from the request: either from X-Ferro-User header
/// (set by auth middleware) or from Claims extension, or "anonymous".
pub(crate) fn extract_owner(headers: &HeaderMap, claims: Option<&common::auth::Claims>) -> String {
    if let Some(user) = headers.get("X-Ferro-User").and_then(|v| v.to_str().ok()) {
        return user.to_string();
    }
    if let Some(c) = claims {
        return c.sub.clone();
    }
    "anonymous".to_string()
}

/// Check conditional headers (If-Match, If-None-Match) against the current ETag.
/// Returns Ok(()) if the request should proceed, or Err(PreconditionFailed).
/// For GET/HEAD, the caller should check If-None-Match separately to return 304.
pub(crate) fn check_conditional_if_match(headers: &HeaderMap, etag: &str) -> Result<()> {
    // If-Match: proceed only if ETag matches one of the values
    if let Some(if_match) = headers.get("If-Match").and_then(|v| v.to_str().ok()) {
        let trimmed = if_match.trim();
        if trimmed == "*" {
            // Must exist (already checked before calling this)
        } else {
            let tags: Vec<&str> = trimmed.split(',').map(|t| t.trim()).collect();
            if !tags.contains(&etag) {
                return Err(FerroError::PreconditionFailed(format!(
                    "If-Match: expected one of {}, got {}",
                    trimmed, etag
                )));
            }
        }
    }
    Ok(())
}

/// Check If-None-Match: returns true if the current ETag matches (caller should return 304).
pub(crate) fn check_if_none_match(headers: &HeaderMap, etag: &str) -> bool {
    if let Some(if_none_match) = headers.get("If-None-Match").and_then(|v| v.to_str().ok()) {
        let trimmed = if_none_match.trim();
        if trimmed == "*" || trimmed == etag {
            return true;
        }
        let tags: Vec<&str> = trimmed.split(',').map(|t| t.trim()).collect();
        if tags.contains(&etag) {
            return true;
        }
    }
    false
}
