use crate::{WebDavCoreState, WebdavEventType, WebdavOpType};
use axum::body::Body;
use axum::extract::{Path as AxumPath, State};
use axum::http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use common::error::FerroError;
use common::error::Result;
use common::path::normalize_path;
use common::webdav::LockDepth;
use ferro_offline::change_queue::ChangeQueueStore;
use ferro_server_storage_ops::streaming_upload::StreamingUploadWriter;
use ferro_server_webdav::sanitize_path;
use ferro_webdav_handler::escape_xml;
use http_body_util::BodyExt;
use tokio::io::AsyncReadExt;
use tracing::{debug, info, warn};

/// Maximum recursion depth for PROPFIND depth:infinity to prevent DoS.
const MAX_PROPFIND_DEPTH: u32 = 100;
const MAX_RECENTLY_PROCESSED: usize = 10_000;

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

pub async fn handle_any<S: WebDavCoreState>(
    method: Method,
    uri: axum::http::Uri,
    State(state): State<S>,
    path: Option<AxumPath<String>>,
    headers: HeaderMap,
    body: Body,
) -> Response {
    let raw_path = match path {
        Some(AxumPath(p)) => format!("/{}", p),
        None => uri.path().to_string(),
    };

    let path_str = match sanitize_path(&raw_path) {
        Ok(p) => p,
        Err(e) => {
            warn!("Path sanitization failed for '{}': {}", raw_path, e);
            let status = StatusCode::from_u16(e.status_code()).unwrap_or(StatusCode::BAD_REQUEST);
            return (
                status,
                axum::Json(serde_json::json!({
                    "error": e.to_string(),
                })),
            )
                .into_response();
        }
    };
    debug!("{} {}", method, path_str);

    // Enforce body size limit using Content-Length header (defense-in-depth;
    // axum's DefaultBodyLimit layer is the primary enforcement).
    if let Some(content_len) = headers
        .get("Content-Length")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
        && content_len > state.max_body_size()
    {
        return (
            StatusCode::PAYLOAD_TOO_LARGE,
            axum::Json(serde_json::json!({
                "error": "Request body too large",
                "size": content_len,
                "max": state.max_body_size(),
            })),
        )
            .into_response();
    }

    let user_sub = headers.get("X-Ferro-User").and_then(|v| v.to_str().ok());
    let resolved_path = match user_sub {
        Some(sub) if sub != "anonymous" => {
            let user_root = format!("/users/{}", sub);
            if path_str == "/" || path_str.is_empty() {
                user_root
            } else {
                format!("{}{}", user_root, path_str)
            }
        }
        _ => path_str.clone(),
    };

    let result: Result<Response> = async {
        // Quota enforcement for PUT (best-effort pre-check using Content-Length header)
        if method.as_str() == "PUT"
            && let Some(content_len) = headers
                .get("Content-Length")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok())
            && let Err(quota_resp) = state.enforce_quota(content_len).await
        {
            return Ok(quota_resp);
        }

        match method.as_str() {
            "OPTIONS" => handle_options(&resolved_path).await,
            "PROPFIND" => handle_propfind(state, &resolved_path, &headers).await,
            "GET" => handle_get(state, &resolved_path, &headers).await,
            "HEAD" => handle_head(state, &resolved_path, &headers).await,
            "PUT" => handle_put_dispatch(state, &resolved_path, &headers, body).await,
            "DELETE" => handle_delete(state, &resolved_path, &headers).await,
            "MKCOL" => handle_mkcol(state, &resolved_path).await,
            "COPY" => handle_copy(state, &resolved_path, &headers).await,
            "MOVE" => handle_move(state, &resolved_path, &headers).await,
            "LOCK" => {
                let bytes = body
                    .collect()
                    .await
                    .map_err(|e| FerroError::Internal(format!("body read: {}", e)))?
                    .to_bytes();
                handle_lock(state, &resolved_path, &headers, &bytes).await
            }
            "UNLOCK" => handle_unlock(state, &resolved_path, &headers).await,
            "PROPPATCH" => {
                let bytes = body
                    .collect()
                    .await
                    .map_err(|e| FerroError::Internal(format!("body read: {}", e)))?
                    .to_bytes();
                handle_proppatch(state, &resolved_path, &headers, &bytes).await
            }
            "MKCALENDAR" | "REPORT" if resolved_path.starts_with("/dav/cal") => {
                let bytes = body
                    .collect()
                    .await
                    .map_err(|e| FerroError::Internal(format!("body read: {}", e)))?
                    .to_bytes();
                let m = method.clone();
                Ok(crate::dav::dispatch_caldav(state, &m, &resolved_path, bytes).await)
            }
            "REPORT" if resolved_path.starts_with("/dav/card") => {
                let bytes = body
                    .collect()
                    .await
                    .map_err(|e| FerroError::Internal(format!("body read: {}", e)))?
                    .to_bytes();
                let m = method.clone();
                Ok(crate::dav::dispatch_carddav(state, &m, &resolved_path, bytes).await)
            }
            _ => Err(FerroError::InvalidArgument(format!(
                "Method {} not supported",
                method
            ))),
        }
    }
    .await;

    match result {
        Ok(response) => response,
        Err(e) => {
            warn!("Error handling {} {}: {}", method, path_str, e);
            let status =
                StatusCode::from_u16(e.status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            let xml = format!(
                r#"<?xml version="1.0" encoding="utf-8"?><d:error xmlns:d="DAV:"><s:message>{}</s:message></d:error>"#,
                escape_xml(&e.to_string())
            );
            (status, xml).into_response()
        }
    }
}

async fn handle_put_dispatch<S: WebDavCoreState>(
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
        return Err(FerroError::InvalidArgument(format!(
            "Invalid path: {}",
            path
        )));
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
            debug!(
                "CAS DEDUP: {} already stored (hash: {})",
                path,
                &hash.as_str()[..16]
            );
            let meta = match state.storage().head(&path).await {
                Ok(m) => m,
                Err(_) => state.storage().put(&path, body.clone(), &owner).await?,
            };
            let mut resp_headers = HeaderMap::new();
            resp_headers.insert(
                "ETag",
                HeaderValue::from_str(&meta.etag)
                    .map_err(|e| FerroError::Internal(e.to_string()))?,
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
        let ver_path = path.clone();
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
        && let Some(detected) =
            ferro_server_security::security::verify_content_type(declared, &body)
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
        let path = path.clone();
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
                            fuel_total.fetch_add(
                                result.fuel_consumed,
                                std::sync::atomic::Ordering::Relaxed,
                            );
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
            path: path.clone(),
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

async fn handle_options(_path: &str) -> Result<Response> {
    let mut headers = HeaderMap::new();
    headers.insert("DAV", HeaderValue::from_static("1, 2, 3"));
    headers.insert(
        "Allow",
        HeaderValue::from_static(
            "OPTIONS, GET, HEAD, PUT, DELETE, MKCOL, COPY, MOVE, PROPFIND, PROPPATCH, LOCK, UNLOCK",
        ),
    );
    headers.insert("MS-Author-Via", HeaderValue::from_static("DAV"));
    Ok((StatusCode::OK, headers, "").into_response())
}

pub async fn handle_propfind<S: WebDavCoreState>(
    state: S,
    path: &str,
    headers: &HeaderMap,
) -> Result<Response> {
    let path = normalize_path(path);

    if !common::path::validate_path(&path) {
        return Err(FerroError::InvalidArgument(format!(
            "Invalid path: {}",
            path
        )));
    }

    let sync_token = headers
        .get("Sync-Token")
        .and_then(|v| v.to_str().ok())
        .and_then(|t| t.rsplit('/').next())
        .and_then(|n| n.parse::<u64>().ok());

    let depth = headers
        .get("Depth")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("infinity");

    // Try to head the path. If it's not found but it's a depth>0 request
    // on "/", synthesize a root collection entry (the in-memory store doesn't
    // auto-create the root collection).
    let metadata = match state.storage().head(&path).await {
        Ok(m) => m,
        Err(_) if path == "/" && depth != "0" => {
            common::metadata::FileMetadata::new_collection("/".to_string(), "anonymous".to_string())
        }
        Err(e) => return Err(e),
    };
    let mut items = vec![(path.clone(), metadata)];

    if depth != "0" && items[0].1.is_collection {
        if depth == "1" {
            let children = state.storage().list(&path).await?;
            items.extend(children.into_iter().map(|m| (m.path.clone(), m)));
        } else {
            // depth:infinity — use bounded list_all
            let all_descendants = state.storage().list_all(&path, MAX_PROPFIND_DEPTH).await?;
            items.extend(all_descendants.into_iter().map(|m| (m.path.clone(), m)));
        }
    }

    if let Some(token) = sync_token {
        let current = state.sync_clock().load(std::sync::atomic::Ordering::SeqCst);
        if token >= current {
            items = items.into_iter().take(1).collect();
        }
    }

    let current_clock = state.sync_clock().load(std::sync::atomic::Ordering::SeqCst);
    let xml = ferro_webdav_handler::build_multistatus_xml(&items);
    if sync_token.is_some() {
        Ok(sync_token_multistatus_response(xml, current_clock))
    } else {
        Ok(multistatus_response(xml))
    }
}

fn multistatus_response(xml: Bytes) -> Response {
    let mut headers = HeaderMap::new();
    headers.insert(
        "Content-Type",
        HeaderValue::from_static("application/xml; charset=utf-8"),
    );
    (StatusCode::MULTI_STATUS, headers, Body::from(xml)).into_response()
}

fn sync_token_multistatus_response(xml: Bytes, clock: u64) -> Response {
    let mut headers = HeaderMap::new();
    headers.insert(
        "Content-Type",
        HeaderValue::from_static("application/xml; charset=utf-8"),
    );
    let token_value = format!("http://ferro.local/sync/token/{}", clock);
    if let Ok(val) = HeaderValue::from_str(&token_value) {
        headers.insert("Sync-Token", val);
    }
    (StatusCode::MULTI_STATUS, headers, Body::from(xml)).into_response()
}

/// Extract the owner from the request: either from X-Ferro-User header
/// (set by auth middleware) or from Claims extension, or "anonymous".
fn extract_owner(headers: &HeaderMap, claims: Option<&common::auth::Claims>) -> String {
    if let Some(user) = headers.get("X-Ferro-User").and_then(|v| v.to_str().ok()) {
        return user.to_string();
    }
    if let Some(c) = claims {
        return c.sub.clone();
    }
    "anonymous".to_string()
}

/// Check conditional headers (If-Match, If-None-Match) against the current ETag.
/// Returns Ok(()) if the request should proceed, or Err(PreconditionFailed).
/// For GET/HEAD, the caller should check If-None-Match separately to return 304.
fn check_conditional_if_match(headers: &HeaderMap, etag: &str) -> Result<()> {
    // If-Match: proceed only if ETag matches one of the values
    if let Some(if_match) = headers.get("If-Match").and_then(|v| v.to_str().ok()) {
        let trimmed = if_match.trim();
        if trimmed == "*" {
            // Must exist (already checked before calling this)
        } else {
            let tags: Vec<&str> = trimmed.split(',').map(|t| t.trim()).collect();
            if !tags.contains(&etag) {
                return Err(FerroError::PreconditionFailed(format!(
                    "If-Match: expected one of {}, got {}",
                    trimmed, etag
                )));
            }
        }
    }
    Ok(())
}

/// Check If-None-Match: returns true if the current ETag matches (caller should return 304).
fn check_if_none_match(headers: &HeaderMap, etag: &str) -> bool {
    if let Some(if_none_match) = headers.get("If-None-Match").and_then(|v| v.to_str().ok()) {
        let trimmed = if_none_match.trim();
        if trimmed == "*" || trimmed == etag {
            return true;
        }
        let tags: Vec<&str> = trimmed.split(',').map(|t| t.trim()).collect();
        if tags.contains(&etag) {
            return true;
        }
    }
    false
}

async fn handle_get<S: WebDavCoreState>(
    state: S,
    path: &str,
    headers: &HeaderMap,
) -> Result<Response> {
    let path = normalize_path(path);

    if !common::path::validate_path(&path) {
        return Err(FerroError::InvalidArgument(format!(
            "Invalid path: {}",
            path
        )));
    }

    // Offline-first: check content cache before hitting storage
    if !state.is_online() {
        let mut cache = state.offline_cache().write().await;
        if let Some(cached_data) = cache.get(&path) {
            debug!("OFFLINE GET: serving cached content for {}", path);
            let content_type = common::mime::sniff_content_type(&cached_data, &path);
            let etag = format!(
                "\"{}\"",
                common::metadata::ContentHash::compute(&cached_data).as_str()
            );
            let mut resp_headers = HeaderMap::new();
            resp_headers.insert(
                "Content-Type",
                HeaderValue::from_str(&content_type)
                    .map_err(|e| FerroError::Internal(e.to_string()))?,
            );
            resp_headers.insert(
                "Content-Length",
                HeaderValue::from_str(&cached_data.len().to_string())
                    .map_err(|e| FerroError::Internal(e.to_string()))?,
            );
            resp_headers.insert(
                "ETag",
                HeaderValue::from_str(&etag).map_err(|e| FerroError::Internal(e.to_string()))?,
            );
            resp_headers.insert(
                HeaderName::from_static("accept-ranges"),
                ferro_server_storage_ops::range_get::accept_ranges_header(),
            );
            return Ok((StatusCode::OK, resp_headers, Body::from(cached_data)).into_response());
        }
        return Err(FerroError::NotFound(format!(
            "Resource not available offline: {}",
            path
        )));
    }

    let meta = state.storage().head(&path).await?;
    if meta.is_collection {
        return Err(FerroError::InvalidArgument(
            "Cannot GET a collection".to_string(),
        ));
    }

    // Conditional GET: If-Match
    check_conditional_if_match(headers, &meta.etag)?;

    // Check If-None-Match for 304 Not Modified (must be before reading content)
    if check_if_none_match(headers, &meta.etag) {
        let mut resp_headers = HeaderMap::new();
        resp_headers.insert(
            "ETag",
            HeaderValue::from_str(&meta.etag).map_err(|e| FerroError::Internal(e.to_string()))?,
        );
        return Ok((StatusCode::NOT_MODIFIED, resp_headers, "").into_response());
    }

    let content_type = if meta.mime_type == "application/octet-stream" {
        common::mime::sniff_content_type(&[], &path)
    } else {
        meta.mime_type.clone()
    };

    let etag_val =
        HeaderValue::from_str(&meta.etag).map_err(|e| FerroError::Internal(e.to_string()))?;
    let last_modified_val = HeaderValue::from_str(
        &meta
            .modified_at
            .format("%a, %d %b %Y %H:%M:%S GMT")
            .to_string(),
    )
    .map_err(|e| FerroError::Internal(e.to_string()))?;
    let content_type_val =
        HeaderValue::from_str(&content_type).map_err(|e| FerroError::Internal(e.to_string()))?;

    if let Some(range_req) =
        ferro_server_storage_ops::range_get::parse_range_header(headers, meta.size)
        && let Some(spec) = range_req.ranges.first()
    {
        if let Some((start, end)) = spec.resolve(meta.size) {
            let mut reader = state.storage().get_stream(&path).await?;
            {
                let mut buf = [0u8; 8192];
                let mut remaining = start;
                while remaining > 0 {
                    let n = std::cmp::min(remaining, buf.len() as u64);
                    reader
                        .read_exact(&mut buf[..n as usize])
                        .await
                        .map_err(|e| FerroError::Internal(e.to_string()))?;
                    remaining -= n;
                }
            }
            let take_reader = reader.take(end - start + 1);
            let stream = tokio_util::io::ReaderStream::new(take_reader);
            let body = Body::from_stream(stream);

            let mut resp_headers = HeaderMap::new();
            resp_headers.insert("Content-Type", content_type_val);
            resp_headers.insert("ETag", etag_val);
            resp_headers.insert("Last-Modified", last_modified_val);
            let range_headers =
                ferro_server_storage_ops::range_get::build_range_headers(start, end, meta.size);
            for (k, v) in range_headers.iter() {
                resp_headers.insert(k.clone(), v.clone());
            }
            return Ok((StatusCode::PARTIAL_CONTENT, resp_headers, body).into_response());
        } else {
            let mut resp_headers = HeaderMap::new();
            resp_headers.insert(
                HeaderName::from_static("content-range"),
                HeaderValue::from_str(&format!("bytes */{}", meta.size))
                    .unwrap_or_else(|_| HeaderValue::from_static("bytes */0")),
            );
            return Ok((StatusCode::RANGE_NOT_SATISFIABLE, resp_headers, "").into_response());
        }
    }

    let reader = state.storage().get_stream(&path).await?;
    let stream = tokio_util::io::ReaderStream::new(reader);
    let body = Body::from_stream(stream);

    let mut resp_headers = HeaderMap::new();
    resp_headers.insert("Content-Type", content_type_val);
    resp_headers.insert(
        "Content-Length",
        HeaderValue::from_str(&meta.size.to_string())
            .map_err(|e| FerroError::Internal(e.to_string()))?,
    );
    resp_headers.insert("ETag", etag_val);
    resp_headers.insert("Last-Modified", last_modified_val);
    resp_headers.insert(
        HeaderName::from_static("accept-ranges"),
        ferro_server_storage_ops::range_get::accept_ranges_header(),
    );

    Ok((StatusCode::OK, resp_headers, body).into_response())
}

async fn handle_head<S: WebDavCoreState>(
    state: S,
    path: &str,
    headers: &HeaderMap,
) -> Result<Response> {
    let path = normalize_path(path);

    if !common::path::validate_path(&path) {
        return Err(FerroError::InvalidArgument(format!(
            "Invalid path: {}",
            path
        )));
    }

    let meta = state.storage().head(&path).await?;

    // Conditional HEAD
    check_conditional_if_match(headers, &meta.etag)?;

    if check_if_none_match(headers, &meta.etag) {
        let mut resp_headers = HeaderMap::new();
        resp_headers.insert(
            "ETag",
            HeaderValue::from_str(&meta.etag).map_err(|e| FerroError::Internal(e.to_string()))?,
        );
        return Ok((StatusCode::NOT_MODIFIED, resp_headers, "").into_response());
    }

    let mut resp_headers = HeaderMap::new();
    // Re-detect MIME from extension if stored value is the generic default.
    let content_type = if meta.mime_type == "application/octet-stream" {
        common::mime::sniff_content_type(&[], &path)
    } else {
        meta.mime_type.clone()
    };
    resp_headers.insert(
        "Content-Type",
        HeaderValue::from_str(&content_type).map_err(|e| FerroError::Internal(e.to_string()))?,
    );
    resp_headers.insert(
        "Content-Length",
        HeaderValue::from_str(&meta.size.to_string())
            .map_err(|e| FerroError::Internal(e.to_string()))?,
    );
    resp_headers.insert(
        "ETag",
        HeaderValue::from_str(&meta.etag).map_err(|e| FerroError::Internal(e.to_string()))?,
    );
    resp_headers.insert(
        "Last-Modified",
        HeaderValue::from_str(
            &meta
                .modified_at
                .format("%a, %d %b %Y %H:%M:%S GMT")
                .to_string(),
        )
        .map_err(|e| FerroError::Internal(e.to_string()))?,
    );
    resp_headers.insert(
        HeaderName::from_static("accept-ranges"),
        ferro_server_storage_ops::range_get::accept_ranges_header(),
    );

    Ok((StatusCode::OK, resp_headers, "").into_response())
}

async fn handle_put<S: WebDavCoreState>(
    state: S,
    path: &str,
    headers: &HeaderMap,
    body: Bytes,
) -> Result<Response> {
    let path = normalize_path(path);

    if !common::path::validate_path(&path) {
        return Err(FerroError::InvalidArgument(format!(
            "Invalid path: {}",
            path
        )));
    }

    // Offline-first: if offline and queue is enabled, queue the write operation
    if !state.is_online()
        && let Some(queue) = state.offline_queue()
    {
        let owner = extract_owner(headers, None);
        let content_hash = Some(
            common::metadata::ContentHash::compute(&body)
                .as_str()
                .to_string(),
        );
        let content_size = Some(body.len() as u64);
        let op = ferro_offline::change_queue::QueuedOperation::put(
            &path,
            content_hash,
            content_size,
            &owner,
        );
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
                    ferro_offline::change_queue::OperationType::Delete => {
                        state.storage().delete(&op.source_path).await
                    }
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
                    _ => {
                        warn!("Unhandled offline operation type: {:?}", op.op);
                        Ok(())
                    }
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
            debug!(
                "CAS DEDUP: {} already stored (hash: {})",
                path,
                &hash.as_str()[..16]
            );
            let meta = match state.storage().head(&path).await {
                Ok(m) => m,
                Err(_) => state.storage().put(&path, body.clone(), &owner).await?,
            };
            let mut resp_headers = HeaderMap::new();
            resp_headers.insert(
                "ETag",
                HeaderValue::from_str(&meta.etag)
                    .map_err(|e| FerroError::Internal(e.to_string()))?,
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
        let ver_path = path.clone();
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
        && let Some(detected) =
            ferro_server_security::security::verify_content_type(declared, &body)
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
        let path = path.clone();
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
                            fuel_total.fetch_add(
                                result.fuel_consumed,
                                std::sync::atomic::Ordering::Relaxed,
                            );
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
            path: path.clone(),
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

async fn handle_delete<S: WebDavCoreState>(
    state: S,
    path: &str,
    headers: &HeaderMap,
) -> Result<Response> {
    let path = normalize_path(path);

    if !common::path::validate_path(&path) {
        return Err(FerroError::InvalidArgument(format!(
            "Invalid path: {}",
            path
        )));
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
            path: path.clone(),
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

async fn handle_mkcol<S: WebDavCoreState>(state: S, path: &str) -> Result<Response> {
    let path = normalize_path(path);

    if !common::path::validate_path(&path) {
        return Err(FerroError::InvalidArgument(format!(
            "Invalid path: {}",
            path
        )));
    }

    if state.storage().exists(&path).await? {
        // RFC 4918 Section 9.3.1: MKCOL on an existing resource returns 405
        return Ok(StatusCode::METHOD_NOT_ALLOWED.into_response());
    }

    state
        .storage()
        .create_collection(&path, "anonymous")
        .await?;

    state
        .dispatch_file_event(crate::WebdavFileEvent {
            op_type: "mkcol",
            path: path.clone(),
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

async fn handle_copy<S: WebDavCoreState>(
    state: S,
    path: &str,
    headers: &HeaderMap,
) -> Result<Response> {
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
        return Err(FerroError::LockConflict(format!(
            "Destination locked: {}",
            e
        )));
    }

    state.storage().copy(&path, &dest).await?;

    state
        .dispatch_file_event(crate::WebdavFileEvent {
            op_type: "copy",
            path: path.clone(),
            new_path: Some(dest.clone()),
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

async fn handle_move<S: WebDavCoreState>(
    state: S,
    path: &str,
    headers: &HeaderMap,
) -> Result<Response> {
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
        return Err(FerroError::LockConflict(format!(
            "Destination locked: {}",
            e
        )));
    }

    state.storage().move_path(&path, &dest).await?;

    let owner = extract_owner(headers, None);

    state
        .dispatch_file_event(crate::WebdavFileEvent {
            op_type: "move",
            path: path.clone(),
            new_path: Some(dest.clone()),
            size: None,
            mime_type: None,
            owner: owner.clone(),
            etag: None,
            already_existed: true,
        })
        .await;

    state.record_sync_op(
        WebdavOpType::Rename,
        &path,
        Some(&dest),
        0,
        None,
        &owner,
        "",
    );

    let mut resp_headers = HeaderMap::new();
    resp_headers.insert(
        "Location",
        HeaderValue::from_str(&dest).map_err(|e| FerroError::Internal(e.to_string()))?,
    );
    Ok((StatusCode::CREATED, resp_headers, "").into_response())
}

async fn handle_lock<S: WebDavCoreState>(
    state: S,
    path: &str,
    headers: &HeaderMap,
    body: &Bytes,
) -> Result<Response> {
    let path = normalize_path(path);

    if !common::path::validate_path(&path) {
        return Err(FerroError::InvalidArgument(format!(
            "Invalid path: {}",
            path
        )));
    }

    // RFC 4918 §9.10.2: If the request includes an If header with a lock token,
    // this is a lock refresh request, not a new lock acquisition.
    if let Some(if_header) = headers.get("If")
        && let Some(lock_token) = extract_lock_token_from_if(if_header)
    {
        return handle_lock_refresh(state, &path, &lock_token, headers, body).await;
    }

    let lock_request = ferro_webdav_handler::LockRequest::parse(body);

    let depth = headers
        .get("Depth")
        .and_then(|v| v.to_str().ok())
        .map(LockDepth::from_header)
        .unwrap_or(lock_request.depth);

    let principal = lock_request.owner.clone().unwrap_or_else(|| {
        headers
            .get("X-Ferro-User")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("anonymous")
            .to_string()
    });

    let lock = state
        .lock_manager()
        .acquire_lock(
            &path,
            &principal,
            lock_request.scope,
            depth,
            lock_request.timeout_hint,
        )
        .await?;

    let lock_token = lock.token.as_str();
    let xml = ferro_webdav_handler::build_lock_response_xml(
        &lock_token,
        depth.to_header(),
        &principal,
        lock.timeout_seconds,
        &path,
    );

    let mut resp_headers = HeaderMap::new();
    resp_headers.insert(
        "Content-Type",
        HeaderValue::from_static("application/xml; charset=utf-8"),
    );
    resp_headers.insert(
        "Lock-Token",
        HeaderValue::from_str(&format!("<{}>", lock_token))
            .map_err(|e| FerroError::Internal(e.to_string()))?,
    );

    Ok((StatusCode::OK, resp_headers, Body::from(xml)).into_response())
}

async fn handle_unlock<S: WebDavCoreState>(
    state: S,
    _path: &str,
    headers: &HeaderMap,
) -> Result<Response> {
    let lock_token = headers
        .get("Lock-Token")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("<").and_then(|r| r.strip_suffix(">")))
        .ok_or_else(|| FerroError::InvalidArgument("Missing Lock-Token header".to_string()))?;

    state.lock_manager().release_lock(lock_token).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

/// LOCK refresh (RFC 4918 §9.10.2): client sends LOCK with If header containing lock token.
async fn handle_lock_refresh<S: WebDavCoreState>(
    state: S,
    path: &str,
    lock_token: &str,
    headers: &HeaderMap,
    body: &Bytes,
) -> Result<Response> {
    let lock_request = ferro_webdav_handler::LockRequest::parse(body);

    match state
        .lock_manager()
        .refresh_lock(lock_token, lock_request.timeout_hint)
        .await
    {
        Ok(lock) => {
            let principal = lock.principal.clone();
            let xml = ferro_webdav_handler::build_lock_response_xml(
                lock_token,
                lock.depth.to_header(),
                &principal,
                lock.timeout_seconds,
                path,
            );

            let mut resp_headers = HeaderMap::new();
            resp_headers.insert(
                "Content-Type",
                HeaderValue::from_static("application/xml; charset=utf-8"),
            );
            resp_headers.insert(
                "Lock-Token",
                HeaderValue::from_str(&format!("<{}>", lock_token))
                    .map_err(|e| FerroError::Internal(e.to_string()))?,
            );

            debug!("LOCK refreshed: {} token={}", path, lock_token);
            Ok((StatusCode::OK, resp_headers, Body::from(xml)).into_response())
        }
        Err(_) => {
            // Lock not found or expired — treat as new lock request
            debug!(
                "LOCK refresh failed for {}, treating as new lock",
                lock_token
            );
            let lock_request = ferro_webdav_handler::LockRequest::parse(body);
            let depth = headers
                .get("Depth")
                .and_then(|v| v.to_str().ok())
                .map(common::webdav::LockDepth::from_header)
                .unwrap_or(lock_request.depth);

            let principal = lock_request.owner.clone().unwrap_or_else(|| {
                headers
                    .get("X-Ferro-User")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("anonymous")
                    .to_string()
            });

            let lock = state
                .lock_manager()
                .acquire_lock(
                    path,
                    &principal,
                    lock_request.scope,
                    depth,
                    lock_request.timeout_hint,
                )
                .await?;

            let lock_token = lock.token.as_str();
            let xml = ferro_webdav_handler::build_lock_response_xml(
                &lock_token,
                depth.to_header(),
                &principal,
                lock.timeout_seconds,
                path,
            );

            let mut resp_headers = HeaderMap::new();
            resp_headers.insert(
                "Content-Type",
                HeaderValue::from_static("application/xml; charset=utf-8"),
            );
            resp_headers.insert(
                "Lock-Token",
                HeaderValue::from_str(&format!("<{}>", lock_token))
                    .map_err(|e| FerroError::Internal(e.to_string()))?,
            );

            Ok((StatusCode::OK, resp_headers, Body::from(xml)).into_response())
        }
    }
}

/// Extract a lock token from an If header value.
/// Format: `(<lock-token>)` or `( <lock-token> )`
fn extract_lock_token_from_if(if_header: &HeaderValue) -> Option<String> {
    let val = if_header.to_str().ok()?;
    let trimmed = val.trim();
    // Extract content between angle brackets
    let start = trimmed.find('<')?;
    let end = trimmed.find('>')?;
    if end > start {
        Some(trimmed[start + 1..end].to_string())
    } else {
        None
    }
}

/// PROPPATCH — modify dead properties on a resource.
/// Currently supports setting `displayname` and `owner` via simple XML parsing.
async fn handle_proppatch<S: WebDavCoreState>(
    state: S,
    path: &str,
    _headers: &HeaderMap,
    body: &Bytes,
) -> Result<Response> {
    let path = normalize_path(path);

    if !common::path::validate_path(&path) {
        return Err(FerroError::InvalidArgument(format!(
            "Invalid path: {}",
            path
        )));
    }

    if !state.storage().exists(&path).await? {
        return Err(FerroError::NotFound(path.to_string()));
    }

    // Parse simple PROPPATCH body to extract property operations
    let props = ferro_webdav_handler::parse_proppatch(body);

    // Build response XML showing all properties as 200 OK
    let xml = ferro_webdav_handler::build_proppatch_response(&path, &props);

    let mut resp_headers = HeaderMap::new();
    resp_headers.insert(
        "Content-Type",
        HeaderValue::from_static("application/xml; charset=utf-8"),
    );

    debug!("PROPPATCH {} ({} properties)", path, props.len());
    Ok((StatusCode::MULTI_STATUS, resp_headers, Body::from(xml)).into_response())
}

// Tests require AppState which is defined in ferro-server.
// These tests should be moved to the server crate's integration tests.
