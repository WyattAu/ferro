use crate::WebdavEventType;
use crate::WebdavFileEvent;
use crate::WebdavOpType;
use crate::handler::WebdavHandlerContext;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use common::error::Result;
use common::path::normalize_path;

fn delete_recursive<'a, S: crate::WebdavAppState>(
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
        state.remove_file_from_index(path);
        Ok(())
    })
}

pub(crate) async fn handle_delete<S: crate::WebdavAppState>(
    state: &S,
    path: &str,
    headers: &HeaderMap,
) -> Result<Response> {
    let ctx = WebdavHandlerContext::new(state, normalize_path(path).to_string(), headers);
    ctx.validate_path()?;
    ctx.check_lock().await?;
    ctx.check_worm()?;

    delete_recursive(state, &ctx.path).await?;

    ctx.dispatch_event(WebdavFileEvent {
        op_type: "delete",
        path: ctx.path.clone(),
        new_path: None,
        size: None,
        mime_type: None,
        owner: ctx.owner.clone(),
        etag: None,
        already_existed: true,
    })
    .await;

    ctx.record_sync(WebdavOpType::Delete, None, 0, None, "");

    ctx.fire_triggers(WebdavEventType::FileDeleted).await;

    Ok(StatusCode::NO_CONTENT.into_response())
}
