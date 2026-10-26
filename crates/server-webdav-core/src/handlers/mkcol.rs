use crate::{WebDavCoreState, WebdavOpType};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use common::error::{FerroError, Result};
use common::path::normalize_path;

pub(crate) async fn handle_mkcol<S: WebDavCoreState>(state: S, path: &str, body: &[u8]) -> Result<Response> {
    let path = normalize_path(path);

    if !common::path::validate_path(&path) {
        return Err(FerroError::InvalidArgument(format!("Invalid path: {}", path)));
    }

    // RFC 4918 §9.3.1: MKCOL with a request body must fail with 415 —
    // collections cannot carry a body at creation.
    if !body.is_empty() {
        return Err(FerroError::UnsupportedMediaType(
            "MKCOL with a request body is not allowed".to_string(),
        ));
    }

    if state.storage().exists(&path).await? {
        // RFC 4918 Section 9.3.1: MKCOL on an existing resource returns 405
        return Ok(StatusCode::METHOD_NOT_ALLOWED.into_response());
    }

    // RFC 4918 §9.3.1 (and RFC 5689 strictness): all intermediate
    // collections must exist — MKCOL creates exactly one collection.
    // Ancestors at or above the per-user root (`/users/<sub>`) are implicit
    // and always exist; only intermediates strictly below it count.
    let implicit_root = |p: &str| p == "/users" || (p.starts_with("/users/") && p.matches('/').count() <= 2);
    if let Some((parent, _)) = path.rsplit_once('/') {
        let parent = if parent.is_empty() { "/" } else { parent };
        if parent != "/" && !implicit_root(parent) && !state.storage().exists(parent).await? {
            return Err(FerroError::Conflict(format!(
                "Intermediate collection {} does not exist",
                parent
            )));
        }
    }

    state.storage().create_collection(&path, "anonymous").await?;

    state
        .dispatch_file_event(crate::WebdavFileEvent {
            op_type: "mkcol",
            path: path.to_string(),
            new_path: None,
            size: None,
            mime_type: None,
            owner: "anonymous".to_string(),
            etag: None,
            already_existed: false,
        })
        .await;

    state.record_sync_op(WebdavOpType::Create, &path, None, 0, None, "anonymous", "");

    Ok(StatusCode::CREATED.into_response())
}
