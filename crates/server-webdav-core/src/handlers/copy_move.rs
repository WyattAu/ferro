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

/// RFC 4918 §9.8.4/§9.9.3 shared COPY/MOVE preflight: `Overwrite: F`
/// forbids clobbering an existing destination (412), the destination's
/// parent collection must exist (409), and lock checks are token-aware —
/// the client's submitted If-header tokens are honored on both ends (§7.1).
async fn copy_move_preflight<S: WebDavCoreState>(state: &S, path: &str, dest: &str, headers: &HeaderMap) -> Result<()> {
    let overwrite_forbidden = headers
        .get("Overwrite")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.trim().eq_ignore_ascii_case("f"))
        .unwrap_or(false);
    let dest_exists = state.storage().exists(dest).await?;
    if overwrite_forbidden && dest_exists {
        return Err(FerroError::PreconditionFailed(
            "Overwrite: F and destination exists".to_string(),
        ));
    }

    // Destination parent must exist (a COPY/MOVE cannot create collections).
    // Ancestors at or above the per-user root are implicit (see mkcol).
    if !dest_exists && let Some((parent, _)) = dest.rsplit_once('/') {
        let parent = if parent.is_empty() { "/" } else { parent };
        let implicit_root = parent == "/users" || (parent.starts_with("/users/") && parent.matches('/').count() <= 2);
        if parent != "/" && !implicit_root && !state.storage().exists(parent).await? {
            return Err(FerroError::Conflict(format!(
                "Destination collection {} does not exist",
                parent
            )));
        }
    }

    let submitted: Vec<String> = headers
        .get("If")
        .and_then(|v| v.to_str().ok())
        .and_then(crate::ifheader::parse)
        .map(|h| h.submitted_tokens())
        .unwrap_or_default();
    if let Err(e) = state
        .lock_manager()
        .check_lock_for_write_with_token(path, &submitted)
        .await
    {
        return Err(FerroError::LockConflict(format!("Source locked: {}", e)));
    }
    if let Err(e) = state
        .lock_manager()
        .check_lock_for_write_with_token(dest, &submitted)
        .await
    {
        return Err(FerroError::LockConflict(format!("Destination locked: {}", e)));
    }
    Ok(())
}

/// Resolve a Destination-header path through the same per-user root
/// mapping the dispatcher applies to request paths (`/users/<sub>/...`).
/// Without this, COPY/MOVE destinations bypassed user-root isolation —
/// a Destination header could address another principal's subtree
/// directly (R6 fix, security-relevant).
fn resolve_destination(dest: &str, headers: &HeaderMap) -> String {
    let sub = headers.get("X-Ferro-User").and_then(|v| v.to_str().ok());
    match sub {
        Some(sub) if sub != "anonymous" && !dest.starts_with("/_spaces") => {
            let user_root = format!("/users/{}", sub);
            if dest == "/" || dest.is_empty() {
                user_root
            } else if dest.starts_with(&user_root) {
                dest.to_string()
            } else {
                format!("{}{}", user_root, dest)
            }
        }
        _ => dest.to_string(),
    }
}

/// Depth-infinity collection copy (RFC 4918 §9.8.3): walk the source tree
/// and recreate every descendant under `dest`.
fn copy_collection_recursive<'a, S: WebDavCoreState>(
    state: &'a S,
    src: &'a str,
    dest: &'a str,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
    Box::pin(copy_collection_recursive_inner(state, src, dest))
}

async fn copy_collection_recursive_inner<S: WebDavCoreState>(state: &S, src: &str, dest: &str) -> Result<()> {
    for child in state.storage().list(src).await? {
        let child_name = child
            .path
            .rsplit_once('/')
            .map(|(_, n)| n.to_string())
            .unwrap_or_else(|| child.path.clone());
        let child_dest = format!("{}/{}", dest.trim_end_matches('/'), child_name);
        if child.is_collection {
            state.storage().create_collection(&child_dest, &child.owner).await?;
            copy_collection_recursive(state, child.path.as_str(), child_dest.as_str()).await?;
        } else {
            state.storage().copy(&child.path, &child_dest).await?;
        }
    }
    Ok(())
}

pub(crate) async fn handle_copy<S: WebDavCoreState>(state: S, path: &str, headers: &HeaderMap) -> Result<Response> {
    let path = normalize_path(path);

    let destination = headers
        .get("Destination")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| FerroError::InvalidArgument("Missing Destination header".to_string()))?;

    // WebDAV Destination header is a full URI (RFC 4918 §10.4); extract just the path.
    let dest = strip_uri_authority(destination);
    // Headers are never URI-decoded by the framework — decode to match storage keys.
    let dest = common::path::decode_percent(&dest).into_owned();
    let dest = normalize_path(&dest);
    let dest = resolve_destination(&dest, headers);

    if !common::path::validate_path(&path) || !common::path::validate_path(&dest) {
        return Err(FerroError::InvalidArgument("Invalid path".to_string()));
    }

    if !state.storage().exists(&path).await? {
        return Err(FerroError::NotFound(path.to_string()));
    }

    if state.is_worm_protected(&path) {
        return Err(FerroError::WormProtected(path.to_string()));
    }

    copy_move_preflight(&state, &path, &dest, headers).await?;

    // RFC 4918 §9.8.3: COPY of a collection copies the tree (depth
    // infinity by default). The storage engine's flat copy cannot copy a
    // directory — recurse: create the destination collection, then copy
    // every descendant (litmus copymove 502s were this gap).
    let head = state.storage().head(&path).await?;
    if head.is_collection {
        state.storage().create_collection(&dest, &head.owner).await?;
        copy_collection_recursive(&state, &path, &dest).await?;
    } else {
        state.storage().copy(&path, &dest).await?;
    }

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
    // Headers are never URI-decoded by the framework — decode to match storage keys.
    let dest = common::path::decode_percent(&dest).into_owned();
    let dest = normalize_path(&dest);
    let dest = resolve_destination(&dest, headers);

    if !common::path::validate_path(&path) || !common::path::validate_path(&dest) {
        return Err(FerroError::InvalidArgument("Invalid path".to_string()));
    }

    if !state.storage().exists(&path).await? {
        return Err(FerroError::NotFound(path.to_string()));
    }

    if state.is_worm_protected(&path) {
        return Err(FerroError::WormProtected(path.to_string()));
    }

    copy_move_preflight(&state, &path, &dest, headers).await?;

    // Overwrite: replace an existing destination subtree before the rename.
    if state.storage().exists(&dest).await? {
        super::delete::delete_recursive(&state, &dest).await?;
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
