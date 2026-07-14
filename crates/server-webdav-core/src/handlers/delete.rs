use crate::{WebDavCoreState, WebdavEventType, WebdavOpType};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use common::error::{FerroError, Result};
use common::path::normalize_path;

use super::extract_owner;

/// Recursively delete a path and all its children (RFC 4918 §9.6.1).
/// For collections, deletes all descendants depth-first, then the collection itself.
fn delete_recursive<'a, S: WebDavCoreState>(
    state: &'a S,
    path: &'a str,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
    Box::pin(async move {
        if matches!(
            state.storage().head(path).await,
            Ok(meta) if meta.is_collection
        ) {
            let children = state.storage().list(path).await?;
            for child in &children {
                delete_recursive(state, &child.path).await?;
            }
        }
        state.storage().delete(path).await?;
        state.thumbnail_cache_invalidate(path);
        state.remove_file_from_index(path).await;
        Ok(())
    })
}

pub(crate) async fn handle_delete<S: WebDavCoreState>(state: S, path: &str, headers: &HeaderMap) -> Result<Response> {
    let path = normalize_path(path);

    if !common::path::validate_path(&path) {
        return Err(FerroError::InvalidArgument(format!("Invalid path: {}", path)));
    }

    if let Some(lock) = state.lock_manager().check_lock(&path).await {
        return Err(FerroError::LockConflict(format!(
            "Resource locked by {}",
            lock.principal
        )));
    }

    if state.is_worm_protected(&path) {
        return Err(FerroError::WormProtected(path.to_string()));
    }

    // RFC 4918 §9.6.1: DELETE on a collection removes the collection and all
    // its members recursively.
    delete_recursive(&state, &path).await?;

    let owner = extract_owner(headers, None);

    state
        .dispatch_file_event(crate::WebdavFileEvent {
            op_type: "delete",
            path: path.to_string(),
            new_path: None,
            size: None,
            mime_type: None,
            owner: owner.clone(),
            etag: None,
            already_existed: true,
        })
        .await;

    state.record_sync_op(WebdavOpType::Delete, &path, None, 0, None, &owner, "");

    state
        .fire_event_triggers(WebdavEventType::FileDeleted, &path, &owner)
        .await;

    Ok(StatusCode::NO_CONTENT.into_response())
}
