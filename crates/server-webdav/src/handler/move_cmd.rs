use crate::WebdavFileEvent;
use crate::WebdavOpType;
use crate::handler::WebdavHandlerContext;
use crate::handler::strip_uri_authority;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use common::error::FerroError;
use common::error::Result;
use common::path::normalize_path;

pub(crate) async fn handle_move<S: crate::WebdavAppState>(
    state: &S,
    path: &str,
    headers: &HeaderMap,
) -> Result<Response> {
    let ctx = WebdavHandlerContext::new(state, normalize_path(path).to_string(), headers);

    let destination = headers
        .get("Destination")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| FerroError::InvalidArgument("Missing Destination header".to_string()))?;

    let dest = strip_uri_authority(destination);
    let dest = normalize_path(&dest);

    if !common::path::validate_path(&ctx.path) || !common::path::validate_path(&dest) {
        return Err(FerroError::InvalidArgument("Invalid path".to_string()));
    }

    if !state.storage().exists(&ctx.path).await? {
        return Err(FerroError::NotFound(ctx.path.to_string()));
    }

    ctx.check_worm()?;
    ctx.check_lock_for_write(&ctx.path).await?;
    ctx.check_lock_for_write(&dest).await?;

    state.storage().move_path(&ctx.path, &dest).await?;

    ctx.dispatch_event(WebdavFileEvent {
        op_type: "move",
        path: ctx.path.clone(),
        new_path: Some(dest.to_string()),
        size: None,
        mime_type: None,
        owner: ctx.owner.clone(),
        etag: None,
        already_existed: true,
    })
    .await;

    ctx.record_sync(WebdavOpType::Rename, Some(&dest), 0, None, "");

    let mut resp_headers = HeaderMap::new();
    resp_headers.insert(
        "Location",
        HeaderValue::from_str(&dest).map_err(|e| FerroError::Internal(e.to_string()))?,
    );
    Ok((StatusCode::CREATED, resp_headers, "").into_response())
}
