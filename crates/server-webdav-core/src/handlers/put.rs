use crate::webdav::MAX_RECENTLY_PROCESSED;
use crate::{WebDavCoreState, WebdavEventType, WebdavOpType};
use axum::body::Body;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use common::error::{FerroError, Result};
use common::path::normalize_path;
use ferro_offline::change_queue::ChangeQueueStore;
use tracing::{debug, info, warn};

use super::{check_conditional_if_match, extract_owner};

pub(crate) async fn handle_put_dispatch<S: WebDavCoreState>(
    state: S,
    path: &str,
    headers: &HeaderMap,
    body: Body,
) -> Result<Response> {
    let content_len = headers
        .get("Content-Length")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok());

    if content_len.is_some_and(|len| len <= state.streaming_upload_threshold()) {
        let bytes = body
            .collect()
            .await
            .map_err(|e| FerroError::Internal(format!("body read: {}", e)))?
            .to_bytes();
        handle_put(state, path, headers, bytes).await
    } else {
        handle_put_streaming(state, path, headers, body).await
    }
}

async fn handle_put_streaming<S: WebDavCoreState>(
    state: S,
    path: &str,
    headers: &HeaderMap,
    body: Body,
) -> Result<Response> {
    let path = normalize_path(path);

    if !common::path::validate_path(&path) {
        return Err(FerroError::InvalidArgument(format!("Invalid path: {}", path)));
    }

    if let Some(lock) = state.lock_manager().check_lock(&path).await {
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
        let current = state.storage().head(&path).await?;
        check_conditional_if_match(headers, &current.etag)?;
    }

    if let Some(if_none_match) = headers.get("If-None-Match").and_then(|v| v.to_str().ok())
        && if_none_match.trim() == "*"
        && state.storage().exists(&path).await?
    {
        return Err(FerroError::PreconditionFailed(
            "If-None-Match: resource already exists".to_string(),
        ));
    }

    let owner = extract_owner(headers, None);

    let mut writer = StreamingUploadWriter::new(state.data_dir())
        .await
        .map_err(|e| FerroError::Internal(format!("temp file create: {}", e)))?;

    let mut data_stream = body.into_data_stream();
    use futures::StreamExt;
    while let Some(chunk) = data_stream.next().await {
        let chunk = chunk.map_err(|e| FerroError::Internal(format!("body stream: {}", e)))?;
        writer
            .write_chunk(&chunk)
            .await
            .map_err(|e| FerroError::Internal(format!("temp file write: {}", e)))?;
    }

    let body = writer
        .finalize()
        .await
        .map_err(|e| FerroError::Internal(format!("temp file read: {}", e)))?;

    if let Some(cas) = &state.cas_store() {
        let hash = common::metadata::ContentHash::compute(&body);
        if cas.dedup_check(&hash).await? {
            debug!("CAS DEDUP: {} already stored (hash: {})", path, &hash.as_str()[..16]);
            let meta = match state.storage().head(&path).await {
                Ok(m) => m,
                Err(_) => state.storage().put(&path, body.clone(), &owner).await?,
            };
            let mut resp_headers = HeaderMap::new();
            resp_headers.insert(
                "ETag",
                HeaderValue::from_str(&meta.etag).map_err(|e| FerroError::Internal(e.to_string()))?,
            );
            return Ok((StatusCode::NO_CONTENT, resp_headers, "").into_response());
        }
    }

    let already_existed = state.storage().exists(&path).await?;

    if already_existed && state.is_worm_protected(&path) {
        return Err(FerroError::WormProtected(path.to_string()));
    }

    if already_existed
        && state.max_file_versions() > 0
        && let Ok(prev) = state.storage().get(&path).await
    {
        let ver_state = ferro_server_versioning::VersioningState {
            data_dir: state.data_dir().map(|s| s.to_string()),
            admin_user: state.admin_user().map(|s| s.to_string()),
            storage: state.storage().clone(),
            max_file_versions: state.max_file_versions(),
        };
        let ver_path = path.to_string();
        tokio::spawn(async move {
            ferro_server_versioning::auto_version(&ver_state, &ver_path, prev).await;
        });
    }

    let content_type = headers
        .get("Content-Type")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| common::mime::sniff_content_type(&body, &path));

    if let Some(declared) = headers.get("Content-Type").and_then(|v| v.to_str().ok())
        && let Some(detected) = ferro_server_security::security::verify_content_type(declared, &body)
    {
        tracing::warn!(
            path = %path,
            declared = %declared,
            detected = %detected,
            "Content-Type mismatch in WebDAV PUT (streaming)"
        );
    }

    let body_for_index = body.clone();
    let use_multipart = body.len() > 10 * 1024 * 1024 && state.storage().supports_multipart();
    let mut meta = if use_multipart {
        state.storage().put_multipart(&path, body, &owner).await?
    } else {
        state.storage().put(&path, body, &owner).await?
    };
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

    if let Some(meta_store) = &state.metadata_store()
        && let Err(e) = meta_store.put(meta.clone()).await
    {
        warn!("Failed to write metadata for {}: {}", path, e);
    }

    state.index_file_with_content(&meta, &body_for_index).await;

    if let Some(runtime) = state.wasm_runtime() {
        let runtime = runtime.clone();
        let storage = state.storage().clone();
        let path = path.to_string();
        let dispatch_count = state.wasm_dispatch_count().clone();
        let error_count = state.wasm_error_count().clone();
        let fuel_total = state.wasm_fuel_total().clone();
        state.recently_processed().insert(path.clone());
        if state.recently_processed().len() > MAX_RECENTLY_PROCESSED {
            let to_remove: Vec<String> = state
                .recently_processed()
                .iter()
                .take(MAX_RECENTLY_PROCESSED / 2)
                .map(|r| r.key().clone())
                .collect();
            for key in to_remove {
                state.recently_processed().remove(&key);
            }
        }
        tokio::spawn(async move {
            let workers = runtime.find_matching_workers(&path).await;
            for worker in workers {
                if let Ok(content) = storage.get(&path).await {
                    tracing::info!("Triggering worker {} for {}", worker.pattern, path);
                    dispatch_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    match runtime
                        .execute(
                            &worker.module_path,
                            &worker.function_name,
                            &content,
                            Some(worker.config.clone()),
                        )
                        .await
                    {
                        Ok(result) => {
                            fuel_total.fetch_add(result.fuel_consumed, std::sync::atomic::Ordering::Relaxed);
                            if !result.success {
                                error_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            }
                        }
                        Err(_) => {
                            error_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        }
                    }
                }
            }
        });
    }

    state
        .dispatch_file_event(crate::WebdavFileEvent {
            op_type: "put",
            path: path.to_string(),
            new_path: None,
            size: Some(meta.size),
            mime_type: Some(meta.mime_type.clone()),
            owner: owner.clone(),
            etag: Some(meta.etag.clone()),
            already_existed,
        })
        .await;

    state.record_sync_op(
        WebdavOpType::Update,
        &path,
        None,
        meta.size,
        Some(&meta.mime_type),
        &owner,
        meta.content_hash.as_str(),
    );

    if already_existed {
        state
            .fire_event_triggers(WebdavEventType::FileModified, &path, &owner)
            .await;
    } else {
        state
            .fire_event_triggers(WebdavEventType::FileUploaded, &path, &owner)
            .await;
    }

    Ok((status, resp_headers, "").into_response())
}

pub(crate) async fn handle_put<S: WebDavCoreState>(
    state: S,
    path: &str,
    headers: &HeaderMap,
    body: Bytes,
) -> Result<Response> {
    let path = normalize_path(path);

    if !common::path::validate_path(&path) {
        return Err(FerroError::InvalidArgument(format!("Invalid path: {}", path)));
    }

    // Offline-first: if offline and queue is enabled, queue the write operation
    if !state.is_online()
        && let Some(queue) = state.offline_queue()
    {
        let owner = extract_owner(headers, None);
        let content_hash = Some(common::metadata::ContentHash::compute(&body).as_str().to_string());
        let content_size = Some(body.len() as u64);
        let op = ferro_offline::change_queue::QueuedOperation::put(&path, content_hash, content_size, &owner);
        match queue.enqueue(op).await {
            Ok(()) => {
                // Cache the content for later sync
                let mut cache = state.offline_cache().write().await;
                cache.put(&path, body.to_vec());
                debug!("OFFLINE PUT: queued write for {}", path);
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
                tracing::warn!("Offline queue enqueue failed for {}: {}", path, e);
                return Err(FerroError::Internal(format!(
                    "Offline queue full or unavailable: {}",
                    e
                )));
            }
        }
    }

    // Online path: if we have an offline queue with pending ops, attempt sync
    if let Some(queue) = state.offline_queue() {
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

    if let Some(lock) = state.lock_manager().check_lock(&path).await {
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

    // Conditional PUT: If-Match on existing resource
    if let Some(_if_match) = headers.get("If-Match").and_then(|v| v.to_str().ok()) {
        let current = state.storage().head(&path).await?;
        check_conditional_if_match(headers, &current.etag)?;
    }

    // If-None-Match on PUT: fail if resource already exists
    if let Some(if_none_match) = headers.get("If-None-Match").and_then(|v| v.to_str().ok())
        && if_none_match.trim() == "*"
        && state.storage().exists(&path).await?
    {
        return Err(FerroError::PreconditionFailed(
            "If-None-Match: resource already exists".to_string(),
        ));
    }

    let owner = extract_owner(headers, None);

    if let Some(cas) = &state.cas_store() {
        let hash = common::metadata::ContentHash::compute(&body);
        if cas.dedup_check(&hash).await? {
            debug!("CAS DEDUP: {} already stored (hash: {})", path, &hash.as_str()[..16]);
            let meta = match state.storage().head(&path).await {
                Ok(m) => m,
                Err(_) => state.storage().put(&path, body.clone(), &owner).await?,
            };
            let mut resp_headers = HeaderMap::new();
            resp_headers.insert(
                "ETag",
                HeaderValue::from_str(&meta.etag).map_err(|e| FerroError::Internal(e.to_string()))?,
            );
            return Ok((StatusCode::NO_CONTENT, resp_headers, "").into_response());
        }
    }

    let already_existed = state.storage().exists(&path).await?;

    if already_existed && state.is_worm_protected(&path) {
        return Err(FerroError::WormProtected(path.to_string()));
    }

    // Auto-version: snapshot previous content before overwrite
    if already_existed
        && state.max_file_versions() > 0
        && let Ok(prev) = state.storage().get(&path).await
    {
        let ver_state = ferro_server_versioning::VersioningState {
            data_dir: state.data_dir().map(|s| s.to_string()),
            admin_user: state.admin_user().map(|s| s.to_string()),
            storage: state.storage().clone(),
            max_file_versions: state.max_file_versions(),
        };
        let ver_path = path.to_string();
        tokio::spawn(async move {
            ferro_server_versioning::auto_version(&ver_state, &ver_path, prev).await;
        });
    }

    // Determine Content-Type before storing — sniff from extension/content if not provided
    let content_type = headers
        .get("Content-Type")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| common::mime::sniff_content_type(&body, &path));

    // Verify Content-Type matches actual file content (magic bytes check)
    if let Some(declared) = headers.get("Content-Type").and_then(|v| v.to_str().ok())
        && let Some(detected) = ferro_server_security::security::verify_content_type(declared, &body)
    {
        tracing::warn!(
            path = %path,
            declared = %declared,
            detected = %detected,
            "Content-Type mismatch in WebDAV PUT"
        );
    }

    let body_for_index = body.clone();
    let use_multipart = body.len() > 10 * 1024 * 1024 && state.storage().supports_multipart();
    let mut meta = if use_multipart {
        state.storage().put_multipart(&path, body, &owner).await?
    } else {
        state.storage().put(&path, body, &owner).await?
    };
    // Update the metadata with the sniffed content-type.
    // Note: This only persists if a metadata_store is configured. Without one,
    // the in-memory backend retains the default mime_type from put().
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

    if let Some(meta_store) = &state.metadata_store()
        && let Err(e) = meta_store.put(meta.clone()).await
    {
        warn!("Failed to write metadata for {}: {}", path, e);
    }

    // Auto-index the file for search
    state.index_file_with_content(&meta, &body_for_index).await;

    if let Some(runtime) = state.wasm_runtime() {
        let runtime = runtime.clone();
        let storage = state.storage().clone();
        let path = path.to_string();
        let dispatch_count = state.wasm_dispatch_count().clone();
        let error_count = state.wasm_error_count().clone();
        let fuel_total = state.wasm_fuel_total().clone();
        state.recently_processed().insert(path.clone());
        if state.recently_processed().len() > MAX_RECENTLY_PROCESSED {
            let to_remove: Vec<String> = state
                .recently_processed()
                .iter()
                .take(MAX_RECENTLY_PROCESSED / 2)
                .map(|r| r.key().clone())
                .collect();
            for key in to_remove {
                state.recently_processed().remove(&key);
            }
        }
        tokio::spawn(async move {
            let workers = runtime.find_matching_workers(&path).await;
            for worker in workers {
                if let Ok(content) = storage.get(&path).await {
                    tracing::info!("Triggering worker {} for {}", worker.pattern, path);
                    dispatch_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    match runtime
                        .execute(
                            &worker.module_path,
                            &worker.function_name,
                            &content,
                            Some(worker.config.clone()),
                        )
                        .await
                    {
                        Ok(result) => {
                            fuel_total.fetch_add(result.fuel_consumed, std::sync::atomic::Ordering::Relaxed);
                            if !result.success {
                                error_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            }
                        }
                        Err(_) => {
                            error_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        }
                    }
                }
            }
        });
    }

    state
        .dispatch_file_event(crate::WebdavFileEvent {
            op_type: "put",
            path: path.to_string(),
            new_path: None,
            size: Some(meta.size),
            mime_type: Some(meta.mime_type.clone()),
            owner: owner.clone(),
            etag: Some(meta.etag.clone()),
            already_existed,
        })
        .await;

    state.record_sync_op(
        WebdavOpType::Update,
        &path,
        None,
        meta.size,
        Some(&meta.mime_type),
        &owner,
        meta.content_hash.as_str(),
    );

    if already_existed {
        state
            .fire_event_triggers(WebdavEventType::FileModified, &path, &owner)
            .await;
    } else {
        state
            .fire_event_triggers(WebdavEventType::FileUploaded, &path, &owner)
            .await;
    }

    // Update offline content cache so it's available if connectivity drops
    {
        let mut cache = state.offline_cache().write().await;
        cache.put(&path, body_for_index.to_vec());
    }

    Ok((status, resp_headers, "").into_response())
}

use ferro_server_storage_ops::streaming_upload::StreamingUploadWriter;
use http_body_util::BodyExt;
