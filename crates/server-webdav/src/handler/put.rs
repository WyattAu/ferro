use crate::WebdavEventType;
use crate::WebdavFileEvent;
use crate::WebdavOpType;
use crate::handler::WebdavHandlerContext;
use crate::handler::check_conditional_if_match;
use crate::handler::sniff_content_type;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use common::error::FerroError;
use common::error::Result;
use common::path::normalize_path;
use ferro_offline::change_queue::ChangeQueueStore;
use tracing::{debug, info, warn};

pub(crate) async fn handle_put<S: crate::WebdavAppState>(
    state: &S,
    path: &str,
    headers: &HeaderMap,
    body: Bytes,
) -> Result<Response> {
    let ctx = WebdavHandlerContext::new(state, normalize_path(path).to_string(), headers);
    ctx.validate_path()?;

    if !state.is_online()
        && let Some(queue) = state.offline_queue().as_ref()
    {
        let content_hash = Some(common::metadata::ContentHash::compute(&body).as_str().to_string());
        let content_size = Some(body.len() as u64);
        let op = ferro_offline::change_queue::QueuedOperation::put(&ctx.path, content_hash, content_size, &ctx.owner);
        match queue.enqueue(op).await {
            Ok(()) => {
                let mut cache = state.offline_cache().write().await;
                cache.put(&ctx.path, body.to_vec());
                debug!("OFFLINE PUT: queued write for {}", ctx.path);
                let mut resp_headers = HeaderMap::new();
                resp_headers.insert(
                    "ETag",
                    HeaderValue::from_str(&format!(
                        "\"offline-{}\"",
                        common::metadata::ContentHash::compute(&body).as_str()
                    ))
                    .map_err(|e| FerroError::Internal(e.to_string()))?,
                );
                return Ok((StatusCode::CREATED, resp_headers, "").into_response());
            }
            Err(e) => {
                tracing::warn!("Offline queue enqueue failed for {}: {}", ctx.path, e);
                return Err(FerroError::Internal(format!(
                    "Offline queue full or unavailable: {}",
                    e
                )));
            }
        }
    }

    if let Some(queue) = state.offline_queue().as_ref() {
        let pending = queue.pending().await;
        if !pending.is_empty() {
            info!(
                "Syncing {} pending offline operations before handling PUT",
                pending.len()
            );
            let mut synced = 0u32;
            for op in &pending {
                let result: std::result::Result<(), FerroError> = match op.op {
                    ferro_offline::change_queue::OperationType::Put => {
                        state.storage().head(&op.source_path).await.map(|_| ())
                    }
                    ferro_offline::change_queue::OperationType::Delete => state.storage().delete(&op.source_path).await,
                    ferro_offline::change_queue::OperationType::Move => {
                        if let Some(ref dest) = op.dest_path {
                            state.storage().move_path(&op.source_path, dest).await
                        } else {
                            Ok(())
                        }
                    }
                    ferro_offline::change_queue::OperationType::Copy => {
                        if let Some(ref dest) = op.dest_path {
                            state.storage().copy(&op.source_path, dest).await
                        } else {
                            Ok(())
                        }
                    }
                    ferro_offline::change_queue::OperationType::CreateCollection => state
                        .storage()
                        .create_collection(&op.source_path, &op.owner)
                        .await
                        .map(|_| ()),
                };
                if result.is_ok() {
                    let _ = queue.mark_synced(&op.id).await;
                    synced += 1;
                }
            }
            if synced > 0 {
                info!("Synced {} offline operations", synced);
            }
        }
    }

    if let Some(lock) = state.lock_manager().check_lock(&ctx.path).await {
        let lock_token = headers
            .get("If")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("(<").and_then(|r| r.strip_suffix(">)")));
        if let Some(token) = lock_token {
            if lock.token.as_str() != token {
                return Err(FerroError::LockConflict(format!(
                    "Resource locked by {}",
                    lock.principal
                )));
            }
        } else {
            return Err(FerroError::LockConflict(format!(
                "Resource locked by {}",
                lock.principal
            )));
        }
    }

    if let Some(_if_match) = headers.get("If-Match").and_then(|v| v.to_str().ok()) {
        let current = state.storage().head(&ctx.path).await?;
        check_conditional_if_match(headers, &current.etag)?;
    }

    if let Some(if_none_match) = headers.get("If-None-Match").and_then(|v| v.to_str().ok())
        && if_none_match.trim() == "*"
        && state.storage().exists(&ctx.path).await?
    {
        return Err(FerroError::PreconditionFailed(
            "If-None-Match: resource already exists".to_string(),
        ));
    }

    if let Some(cas) = state.cas_store() {
        let hash = common::metadata::ContentHash::compute(&body);
        if cas.dedup_check(&hash).await? {
            debug!(
                "CAS DEDUP: {} already stored (hash: {})",
                ctx.path,
                &hash.as_str()[..16]
            );
            let meta = match state.storage().head(&ctx.path).await {
                Ok(m) => m,
                Err(_) => state.storage().put(&ctx.path, body.clone(), &ctx.owner).await?,
            };
            let mut resp_headers = HeaderMap::new();
            resp_headers.insert(
                "ETag",
                HeaderValue::from_str(&meta.etag).map_err(|e| FerroError::Internal(e.to_string()))?,
            );
            return Ok((StatusCode::NO_CONTENT, resp_headers, "").into_response());
        }
    }

    let already_existed = state.storage().exists(&ctx.path).await?;

    if already_existed && state.is_worm_protected(&ctx.path) {
        return Err(FerroError::WormProtected(ctx.path.clone()));
    }

    if already_existed
        && state.max_file_versions() > 0
        && let Ok(prev) = state.storage().get(&ctx.path).await
    {
        let ver_state = ferro_server_versioning::VersioningState {
            data_dir: state.data_dir(),
            admin_user: state.admin_user(),
            storage: state.storage().clone(),
            max_file_versions: state.max_file_versions(),
        };
        let ver_path = ctx.path.clone();
        tokio::spawn(async move {
            ferro_server_versioning::auto_version(&ver_state, &ver_path, prev).await;
        });
    }

    let content_type = headers
        .get("Content-Type")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| sniff_content_type(&body, &ctx.path));

    if let Some(declared) = headers.get("Content-Type").and_then(|v| v.to_str().ok())
        && let Some(detected) = state.verify_content_type(declared, &body)
    {
        tracing::warn!(
            path = %ctx.path,
            declared = %declared,
            detected = %detected,
            "Content-Type mismatch in WebDAV PUT"
        );
    }

    let body_for_index = body.clone();
    let mut meta = state.storage().put(&ctx.path, body, &ctx.owner).await?;
    meta.mime_type = content_type.clone();

    let mut resp_headers = HeaderMap::new();
    resp_headers.insert(
        "ETag",
        HeaderValue::from_str(&meta.etag).map_err(|e| FerroError::Internal(e.to_string()))?,
    );

    let status = if already_existed {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::CREATED
    };

    if let Some(meta_store) = state.metadata_store()
        && let Err(e) = meta_store.put(meta.clone()).await
    {
        warn!("Failed to write metadata for {}: {}", ctx.path, e);
    }

    state.index_file_with_content(&meta, &body_for_index);

    state.dispatch_wasm_workers(&ctx.path);

    ctx.dispatch_event(WebdavFileEvent {
        op_type: "put",
        path: ctx.path.clone(),
        new_path: None,
        size: Some(meta.size),
        mime_type: Some(meta.mime_type.clone()),
        owner: ctx.owner.clone(),
        etag: Some(meta.etag.clone()),
        already_existed,
    })
    .await;

    ctx.record_sync(
        WebdavOpType::Update,
        None,
        meta.size,
        Some(&meta.mime_type),
        meta.content_hash.as_str(),
    );

    if already_existed {
        ctx.fire_triggers(WebdavEventType::FileModified).await;
    } else {
        ctx.fire_triggers(WebdavEventType::FileUploaded).await;
    }

    {
        let mut cache = state.offline_cache().write().await;
        cache.put(&ctx.path, body_for_index.to_vec());
    }

    Ok((status, resp_headers, "").into_response())
}
