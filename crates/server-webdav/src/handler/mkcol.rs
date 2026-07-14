use crate::WebdavFileEvent;
use crate::WebdavOpType;
use crate::handler::WebdavHandlerContext;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use common::error::Result;
use common::path::normalize_path;

pub(crate) async fn handle_mkcol<S: crate::WebdavAppState>(state: &S, path: &str) -> Result<Response> {
    let empty_headers = HeaderMap::new();
    let ctx = WebdavHandlerContext::new(state, normalize_path(path).to_string(), &empty_headers);
    ctx.validate_path()?;

    if state.storage().exists(&ctx.path).await? {
        return Ok(StatusCode::METHOD_NOT_ALLOWED.into_response());
    }

    state.storage().create_collection(&ctx.path, "anonymous").await?;

    ctx.dispatch_event(WebdavFileEvent {
        op_type: "mkcol",
        path: ctx.path.clone(),
        new_path: None,
        size: None,
        mime_type: None,
        owner: "anonymous".to_string(),
        etag: None,
        already_existed: false,
    })
    .await;

    ctx.record_sync(WebdavOpType::Create, None, 0, None, "anonymous");

    Ok(StatusCode::CREATED.into_response())
}
