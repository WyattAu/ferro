use crate::{WebDavCoreState, WebdavOpType};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use common::error::{FerroError, Result};
use common::path::normalize_path;

use super::extract_owner;

/// Strip the scheme and authority from a WebDAV Destination URI, returning
/// just the path component. Per RFC 4918 §10.4, the Destination header is
/// always an absolute URI like `http://host:port/path/to/resource`.
fn strip_uri_authority(uri: &str) -> String {
    // Try to parse as URL and extract the path.
    if let Ok(parsed) = url::Url::parse(uri) {
        return parsed.path().to_string();
    }
    // Fallback: if it starts with /, return as-is; otherwise try to find the first /.
    if uri.starts_with('/') {
        uri.to_string()
    } else if let Some(idx) = uri.find('/') {
        if idx > 0 && uri[..idx].contains("://") {
            uri[idx..].to_string()
        } else {
            uri.to_string()
        }
    } else {
        uri.to_string()
    }
}

pub(crate) async fn handle_copy<S: WebDavCoreState>(state: S, path: &str, headers: &HeaderMap) -> Result<Response> {
    let path = normalize_path(path);

    let destination = headers
        .get("Destination")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| FerroError::InvalidArgument("Missing Destination header".to_string()))?;

    // WebDAV Destination header is a full URI (RFC 4918 §10.4); extract just the path.
    let dest = strip_uri_authority(destination);
    let dest = normalize_path(&dest);

    if !common::path::validate_path(&path) || !common::path::validate_path(&dest) {
        return Err(FerroError::InvalidArgument("Invalid path".to_string()));
    }

    if !state.storage().exists(&path).await? {
        return Err(FerroError::NotFound(path.to_string()));
    }

    if state.is_worm_protected(&path) {
        return Err(FerroError::WormProtected(path.to_string()));
    }

    if let Err(e) = state.lock_manager().check_lock_for_write(&path).await {
        return Err(FerroError::LockConflict(format!("Source locked: {}", e)));
    }
    if let Err(e) = state.lock_manager().check_lock_for_write(&dest).await {
        return Err(FerroError::LockConflict(format!("Destination locked: {}", e)));
    }

    state.storage().copy(&path, &dest).await?;

    state
        .dispatch_file_event(crate::WebdavFileEvent {
            op_type: "copy",
            path: path.to_string(),
            new_path: Some(dest.to_string()),
            size: None,
            mime_type: None,
            owner: extract_owner(headers, None),
            etag: None,
            already_existed: false,
        })
        .await;

    state.bump_sync_clock();

    let mut resp_headers = HeaderMap::new();
    resp_headers.insert(
        "Location",
        HeaderValue::from_str(&dest).map_err(|e| FerroError::Internal(e.to_string()))?,
    );
    Ok((StatusCode::CREATED, resp_headers, "").into_response())
}

pub(crate) async fn handle_move<S: WebDavCoreState>(state: S, path: &str, headers: &HeaderMap) -> Result<Response> {
    let path = normalize_path(path);

    let destination = headers
        .get("Destination")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| FerroError::InvalidArgument("Missing Destination header".to_string()))?;

    // WebDAV Destination header is a full URI (RFC 4918 §10.4); extract just the path.
    let dest = strip_uri_authority(destination);
    let dest = normalize_path(&dest);

    if !common::path::validate_path(&path) || !common::path::validate_path(&dest) {
        return Err(FerroError::InvalidArgument("Invalid path".to_string()));
    }

    if !state.storage().exists(&path).await? {
        return Err(FerroError::NotFound(path.to_string()));
    }

    if state.is_worm_protected(&path) {
        return Err(FerroError::WormProtected(path.to_string()));
    }

    if let Err(e) = state.lock_manager().check_lock_for_write(&path).await {
        return Err(FerroError::LockConflict(format!("Source locked: {}", e)));
    }
    if let Err(e) = state.lock_manager().check_lock_for_write(&dest).await {
        return Err(FerroError::LockConflict(format!("Destination locked: {}", e)));
    }

    state.storage().move_path(&path, &dest).await?;

    let owner = extract_owner(headers, None);

    state
        .dispatch_file_event(crate::WebdavFileEvent {
            op_type: "move",
            path: path.to_string(),
            new_path: Some(dest.to_string()),
            size: None,
            mime_type: None,
            owner: owner.clone(),
            etag: None,
            already_existed: true,
        })
        .await;

    state.record_sync_op(WebdavOpType::Rename, &path, Some(&dest), 0, None, &owner, "");

    let mut resp_headers = HeaderMap::new();
    resp_headers.insert(
        "Location",
        HeaderValue::from_str(&dest).map_err(|e| FerroError::Internal(e.to_string()))?,
    );
    Ok((StatusCode::CREATED, resp_headers, "").into_response())
}
