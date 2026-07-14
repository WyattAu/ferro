use crate::{WebDavCoreState, WebdavOpType};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use common::error::{FerroError, Result};
use common::path::normalize_path;

pub(crate) async fn handle_mkcol<S: WebDavCoreState>(state: S, path: &str) -> Result<Response> {
    let path = normalize_path(path);

    if !common::path::validate_path(&path) {
        return Err(FerroError::InvalidArgument(format!("Invalid path: {}", path)));
    }

    if state.storage().exists(&path).await? {
        // RFC 4918 Section 9.3.1: MKCOL on an existing resource returns 405
        return Ok(StatusCode::METHOD_NOT_ALLOWED.into_response());
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
