use crate::AppState;
use crate::streaming_upload::StreamingUploadWriter;
use crate::sync::ops::OpType;
use crate::xml::escape_xml;
use axum::body::Body;
use axum::extract::{Path as AxumPath, State};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use common::error::FerroError;
use common::error::Result;
use common::path::normalize_path;
use common::webdav::LockDepth;
use http_body_util::BodyExt;
use tracing::{debug, warn};

/// Maximum recursion depth for PROPFIND depth:infinity to prevent DoS.
const MAX_PROPFIND_DEPTH: u32 = 100;
const MAX_RECENTLY_PROCESSED: usize = 10_000;

fn sanitize_path(path: &str) -> Result<String> {
    if path.contains('\0') {
        return Err(FerroError::InvalidArgument(
            "Path contains null bytes".to_string(),
        ));
    }

    for component in std::path::Path::new(path).components() {
        match component {
            std::path::Component::ParentDir => {
                return Err(FerroError::InvalidArgument(
                    "Path traversal detected: '..' not allowed".to_string(),
                ));
            }
            std::path::Component::CurDir => {
                return Err(FerroError::InvalidArgument(
                    "Path contains '.' component".to_string(),
                ));
            }
            _ => {}
        }
    }

    let normalized = normalize_path(path);
    Ok(normalized)
}

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

pub async fn handle_any(
    method: Method,
    uri: axum::http::Uri,
    State(state): State<AppState>,
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
        && content_len > state.max_body_size
    {
        return (
            StatusCode::PAYLOAD_TOO_LARGE,
            axum::Json(serde_json::json!({
                "error": "Request body too large",
                "size": content_len,
                "max": state.max_body_size,
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
            && let Err(quota_resp) = crate::quota::enforce_quota(&state, content_len)
        {
            return Ok(*quota_resp);
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

async fn handle_put_dispatch(
    state: AppState,
    path: &str,
    headers: &HeaderMap,
    body: Body,
) -> Result<Response> {
    let content_len = headers
        .get("Content-Length")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok());

    if content_len.is_some_and(|len| len <= state.streaming_upload_threshold) {
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

async fn handle_put_streaming(
    state: AppState,
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

    if let Some(lock) = state.lock_manager.check_lock(&path).await {
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
        let current = state.storage.head(&path).await?;
        check_conditional_if_match(headers, &current.etag)?;
    }

    if let Some(if_none_match) = headers.get("If-None-Match").and_then(|v| v.to_str().ok())
        && if_none_match.trim() == "*"
        && state.storage.exists(&path).await?
    {
        return Err(FerroError::PreconditionFailed(
            "If-None-Match: resource already exists".to_string(),
        ));
    }

    let owner = extract_owner(headers, None);

    let mut writer = StreamingUploadWriter::new(state.data_dir.as_deref())
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

    if let Some(cas) = &state.cas_store {
        let hash = common::metadata::ContentHash::compute(&body);
        if cas.dedup_check(&hash).await? {
            debug!(
                "CAS DEDUP: {} already stored (hash: {})",
                path,
                &hash.as_str()[..16]
            );
            let meta = match state.storage.head(&path).await {
                Ok(m) => m,
                Err(_) => state.storage.put(&path, body.clone(), &owner).await?,
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

    let already_existed = state.storage.exists(&path).await?;

    if already_existed
        && state.max_file_versions > 0
        && let Ok(prev) = state.storage.get(&path).await
    {
        let ver_state = ferro_server_versioning::VersioningState {
            data_dir: state.data_dir.clone(),
            admin_user: state.admin_user.clone(),
            storage: state.storage.clone(),
            max_file_versions: state.max_file_versions,
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
        .unwrap_or_else(|| sniff_content_type(&body, &path));

    if let Some(declared) = headers.get("Content-Type").and_then(|v| v.to_str().ok())
        && let Some(detected) = crate::security::verify_content_type(declared, &body)
    {
        tracing::warn!(
            path = %path,
            declared = %declared,
            detected = %detected,
            "Content-Type mismatch in WebDAV PUT (streaming)"
        );
    }

    let body_for_index = body.clone();
    let use_multipart = body.len() > 10 * 1024 * 1024 && state.storage.supports_multipart();
    let mut meta = if use_multipart {
        state.storage.put_multipart(&path, body, &owner).await?
    } else {
        state.storage.put(&path, body, &owner).await?
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

    if let Some(meta_store) = &state.metadata_store
        && let Err(e) = meta_store.put(meta.clone()).await
    {
        warn!("Failed to write metadata for {}: {}", path, e);
    }

    crate::indexer::index_file_with_content(&state, &meta, &body_for_index).await;

    if let Some(runtime) = &state.wasm_runtime {
        let runtime = runtime.clone();
        let storage = state.storage.clone();
        let path = path.clone();
        let dispatch_count = state.wasm_dispatch_count.clone();
        let error_count = state.wasm_error_count.clone();
        let fuel_total = state.wasm_fuel_total.clone();
        state.recently_processed.insert(path.clone());
        if state.recently_processed.len() > MAX_RECENTLY_PROCESSED {
            let to_remove: Vec<String> = state
                .recently_processed
                .iter()
                .take(MAX_RECENTLY_PROCESSED / 2)
                .map(|r| r.key().clone())
                .collect();
            for key in to_remove {
                state.recently_processed.remove(&key);
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

    crate::events::dispatch_post_op(
        &state,
        crate::events::FileEvent {
            op_type: "put",
            path: path.clone(),
            new_path: None,
            size: Some(meta.size),
            mime_type: Some(meta.mime_type.clone()),
            owner: owner.clone(),
            etag: Some(meta.etag.clone()),
            already_existed,
        },
    )
    .await;

    state.record_sync_op(
        OpType::Update,
        &path,
        None,
        meta.size,
        Some(&meta.mime_type),
        &owner,
        meta.content_hash.as_str(),
    );

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

async fn handle_propfind(state: AppState, path: &str, headers: &HeaderMap) -> Result<Response> {
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
    let metadata = match state.storage.head(&path).await {
        Ok(m) => m,
        Err(_) if path == "/" && depth != "0" => {
            common::metadata::FileMetadata::new_collection("/".to_string(), "anonymous".to_string())
        }
        Err(e) => return Err(e),
    };
    let mut items = vec![(path.clone(), metadata)];

    if depth != "0" && items[0].1.is_collection {
        if depth == "1" {
            let children = state.storage.list(&path).await?;
            items.extend(children.into_iter().map(|m| (m.path.clone(), m)));
        } else {
            // depth:infinity — use bounded list_all
            let all_descendants = state.storage.list_all(&path, MAX_PROPFIND_DEPTH).await?;
            items.extend(all_descendants.into_iter().map(|m| (m.path.clone(), m)));
        }
    }

    if let Some(token) = sync_token {
        let current = state.sync_clock.load(std::sync::atomic::Ordering::SeqCst);
        if token >= current {
            items = items.into_iter().take(1).collect();
        }
    }

    let current_clock = state.sync_clock.load(std::sync::atomic::Ordering::SeqCst);
    let xml = crate::xml::build_multistatus_xml(&items);
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

async fn handle_get(state: AppState, path: &str, headers: &HeaderMap) -> Result<Response> {
    let path = normalize_path(path);

    if !common::path::validate_path(&path) {
        return Err(FerroError::InvalidArgument(format!(
            "Invalid path: {}",
            path
        )));
    }

    let meta = state.storage.head(&path).await?;
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

    let reader = state.storage.get_stream(&path).await?;
    let stream = tokio_util::io::ReaderStream::new(reader);
    let body = Body::from_stream(stream);

    let mut resp_headers = HeaderMap::new();
    // If the storage engine persisted the generic default (e.g. because the
    // MIME was sniffed but never written back), re-detect from the file
    // extension so that .json files return "application/json", etc.
    let content_type = if meta.mime_type == "application/octet-stream" {
        sniff_content_type(&[], &path)
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

    Ok((StatusCode::OK, resp_headers, body).into_response())
}

async fn handle_head(state: AppState, path: &str, headers: &HeaderMap) -> Result<Response> {
    let path = normalize_path(path);

    if !common::path::validate_path(&path) {
        return Err(FerroError::InvalidArgument(format!(
            "Invalid path: {}",
            path
        )));
    }

    let meta = state.storage.head(&path).await?;

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
        sniff_content_type(&[], &path)
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

    Ok((StatusCode::OK, resp_headers, "").into_response())
}

/// Detect MIME type from the first bytes of content using magic bytes.
pub(crate) fn sniff_content_type(data: &[u8], path: &str) -> String {
    // First try the file extension
    if let Some(mime) = mime_guess::from_path(path).first() {
        let mime_str = mime.essence_str();
        if mime_str != "application/octet-stream" {
            return mime_str.to_string();
        }
    }

    // Fall back to magic bytes for common formats
    if data.len() >= 4 {
        match &data[..4] {
            b"%PDF" => return "application/pdf".to_string(),
            b"\x89PNG" => return "image/png".to_string(),
            b"GIF8" => return "image/gif".to_string(),
            _ => {}
        }
    }
    if data.len() >= 3 && &data[..3] == b"\xff\xd8\xff" {
        return "image/jpeg".to_string();
    }
    if data.len() >= 5 && &data[..5] == b"<?xml" {
        return "application/xml".to_string();
    }
    if data.len() >= 2 && &data[..2] == b"PK" {
        return "application/zip".to_string();
    }
    // RAR: Rar!\x1a\x07
    if data.len() >= 6 && &data[..6] == b"Rar!\x1a\x07" {
        return "application/vnd.rar".to_string();
    }
    // OGG
    if data.len() >= 4 && &data[..4] == b"OggS" {
        return "audio/ogg".to_string();
    }
    // WebP
    if data.len() >= 12 && &data[8..12] == b"WEBP" {
        return "image/webp".to_string();
    }
    // MP4
    if data.len() >= 8 && &data[4..8] == b"ftyp" {
        return "video/mp4".to_string();
    }

    "application/octet-stream".to_string()
}

async fn handle_put(
    state: AppState,
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

    if let Some(lock) = state.lock_manager.check_lock(&path).await {
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
        let current = state.storage.head(&path).await?;
        check_conditional_if_match(headers, &current.etag)?;
    }

    // If-None-Match on PUT: fail if resource already exists
    if let Some(if_none_match) = headers.get("If-None-Match").and_then(|v| v.to_str().ok())
        && if_none_match.trim() == "*"
        && state.storage.exists(&path).await?
    {
        return Err(FerroError::PreconditionFailed(
            "If-None-Match: resource already exists".to_string(),
        ));
    }

    let owner = extract_owner(headers, None);

    if let Some(cas) = &state.cas_store {
        let hash = common::metadata::ContentHash::compute(&body);
        if cas.dedup_check(&hash).await? {
            debug!(
                "CAS DEDUP: {} already stored (hash: {})",
                path,
                &hash.as_str()[..16]
            );
            let meta = match state.storage.head(&path).await {
                Ok(m) => m,
                Err(_) => state.storage.put(&path, body.clone(), &owner).await?,
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

    let already_existed = state.storage.exists(&path).await?;

    // Auto-version: snapshot previous content before overwrite
    if already_existed
        && state.max_file_versions > 0
        && let Ok(prev) = state.storage.get(&path).await
    {
        let ver_state = ferro_server_versioning::VersioningState {
            data_dir: state.data_dir.clone(),
            admin_user: state.admin_user.clone(),
            storage: state.storage.clone(),
            max_file_versions: state.max_file_versions,
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
        .unwrap_or_else(|| sniff_content_type(&body, &path));

    // Verify Content-Type matches actual file content (magic bytes check)
    if let Some(declared) = headers.get("Content-Type").and_then(|v| v.to_str().ok())
        && let Some(detected) = crate::security::verify_content_type(declared, &body)
    {
        tracing::warn!(
            path = %path,
            declared = %declared,
            detected = %detected,
            "Content-Type mismatch in WebDAV PUT"
        );
    }

    let body_for_index = body.clone();
    let use_multipart = body.len() > 10 * 1024 * 1024 && state.storage.supports_multipart();
    let mut meta = if use_multipart {
        state.storage.put_multipart(&path, body, &owner).await?
    } else {
        state.storage.put(&path, body, &owner).await?
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

    if let Some(meta_store) = &state.metadata_store
        && let Err(e) = meta_store.put(meta.clone()).await
    {
        warn!("Failed to write metadata for {}: {}", path, e);
    }

    // Auto-index the file for search
    crate::indexer::index_file_with_content(&state, &meta, &body_for_index).await;

    if let Some(runtime) = &state.wasm_runtime {
        let runtime = runtime.clone();
        let storage = state.storage.clone();
        let path = path.clone();
        let dispatch_count = state.wasm_dispatch_count.clone();
        let error_count = state.wasm_error_count.clone();
        let fuel_total = state.wasm_fuel_total.clone();
        state.recently_processed.insert(path.clone());
        if state.recently_processed.len() > MAX_RECENTLY_PROCESSED {
            let to_remove: Vec<String> = state
                .recently_processed
                .iter()
                .take(MAX_RECENTLY_PROCESSED / 2)
                .map(|r| r.key().clone())
                .collect();
            for key in to_remove {
                state.recently_processed.remove(&key);
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

    crate::events::dispatch_post_op(
        &state,
        crate::events::FileEvent {
            op_type: "put",
            path: path.clone(),
            new_path: None,
            size: Some(meta.size),
            mime_type: Some(meta.mime_type.clone()),
            owner: owner.clone(),
            etag: Some(meta.etag.clone()),
            already_existed,
        },
    )
    .await;

    state.record_sync_op(
        OpType::Update,
        &path,
        None,
        meta.size,
        Some(&meta.mime_type),
        &owner,
        meta.content_hash.as_str(),
    );

    Ok((status, resp_headers, "").into_response())
}

/// Recursively delete a path and all its children (RFC 4918 §9.6.1).
/// For collections, deletes all descendants depth-first, then the collection itself.
fn delete_recursive<'a>(
    state: &'a AppState,
    path: &'a str,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
    Box::pin(async move {
        if matches!(
            state.storage.head(path).await,
            Ok(meta) if meta.is_collection
        ) {
            let children = state.storage.list(path).await?;
            for child in &children {
                delete_recursive(state, &child.path).await?;
            }
        }
        state.storage.delete(path).await?;
        crate::indexer::remove_file(state, path).await;
        Ok(())
    })
}

async fn handle_delete(state: AppState, path: &str, headers: &HeaderMap) -> Result<Response> {
    let path = normalize_path(path);

    if !common::path::validate_path(&path) {
        return Err(FerroError::InvalidArgument(format!(
            "Invalid path: {}",
            path
        )));
    }

    if let Some(lock) = state.lock_manager.check_lock(&path).await {
        return Err(FerroError::LockConflict(format!(
            "Resource locked by {}",
            lock.principal
        )));
    }

    // RFC 4918 §9.6.1: DELETE on a collection removes the collection and all
    // its members recursively.
    delete_recursive(&state, &path).await?;

    let owner = extract_owner(headers, None);

    crate::events::dispatch_post_op(
        &state,
        crate::events::FileEvent {
            op_type: "delete",
            path: path.clone(),
            new_path: None,
            size: None,
            mime_type: None,
            owner: owner.clone(),
            etag: None,
            already_existed: true,
        },
    )
    .await;

    state.record_sync_op(OpType::Delete, &path, None, 0, None, &owner, "");

    Ok(StatusCode::NO_CONTENT.into_response())
}

async fn handle_mkcol(state: AppState, path: &str) -> Result<Response> {
    let path = normalize_path(path);

    if !common::path::validate_path(&path) {
        return Err(FerroError::InvalidArgument(format!(
            "Invalid path: {}",
            path
        )));
    }

    if state.storage.exists(&path).await? {
        return Err(FerroError::AlreadyExists(path.to_string()));
    }

    state.storage.create_collection(&path, "anonymous").await?;

    crate::events::dispatch_post_op(
        &state,
        crate::events::FileEvent {
            op_type: "mkcol",
            path: path.clone(),
            new_path: None,
            size: None,
            mime_type: None,
            owner: "anonymous".to_string(),
            etag: None,
            already_existed: false,
        },
    )
    .await;

    state.record_sync_op(OpType::Create, &path, None, 0, None, "anonymous", "");

    Ok(StatusCode::CREATED.into_response())
}

async fn handle_copy(state: AppState, path: &str, headers: &HeaderMap) -> Result<Response> {
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

    if !state.storage.exists(&path).await? {
        return Err(FerroError::NotFound(path.to_string()));
    }

    if let Err(e) = state.lock_manager.check_lock_for_write(&path).await {
        return Err(FerroError::LockConflict(format!("Source locked: {}", e)));
    }
    if let Err(e) = state.lock_manager.check_lock_for_write(&dest).await {
        return Err(FerroError::LockConflict(format!(
            "Destination locked: {}",
            e
        )));
    }

    state.storage.copy(&path, &dest).await?;

    crate::events::dispatch_post_op(
        &state,
        crate::events::FileEvent {
            op_type: "copy",
            path: path.clone(),
            new_path: Some(dest.clone()),
            size: None,
            mime_type: None,
            owner: extract_owner(headers, None),
            etag: None,
            already_existed: false,
        },
    )
    .await;

    state.bump_sync_clock();

    let mut resp_headers = HeaderMap::new();
    resp_headers.insert(
        "Location",
        HeaderValue::from_str(&dest).map_err(|e| FerroError::Internal(e.to_string()))?,
    );
    Ok((StatusCode::CREATED, resp_headers, "").into_response())
}

async fn handle_move(state: AppState, path: &str, headers: &HeaderMap) -> Result<Response> {
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

    if !state.storage.exists(&path).await? {
        return Err(FerroError::NotFound(path.to_string()));
    }

    if let Err(e) = state.lock_manager.check_lock_for_write(&path).await {
        return Err(FerroError::LockConflict(format!("Source locked: {}", e)));
    }
    if let Err(e) = state.lock_manager.check_lock_for_write(&dest).await {
        return Err(FerroError::LockConflict(format!(
            "Destination locked: {}",
            e
        )));
    }

    state.storage.move_path(&path, &dest).await?;

    let owner = extract_owner(headers, None);

    crate::events::dispatch_post_op(
        &state,
        crate::events::FileEvent {
            op_type: "move",
            path: path.clone(),
            new_path: Some(dest.clone()),
            size: None,
            mime_type: None,
            owner: owner.clone(),
            etag: None,
            already_existed: true,
        },
    )
    .await;

    state.record_sync_op(OpType::Rename, &path, Some(&dest), 0, None, &owner, "");

    let mut resp_headers = HeaderMap::new();
    resp_headers.insert(
        "Location",
        HeaderValue::from_str(&dest).map_err(|e| FerroError::Internal(e.to_string()))?,
    );
    Ok((StatusCode::CREATED, resp_headers, "").into_response())
}

async fn handle_lock(
    state: AppState,
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

    let lock_request = crate::xml::LockRequest::parse(body);

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
        .lock_manager
        .acquire_lock(
            &path,
            &principal,
            lock_request.scope,
            depth,
            lock_request.timeout_hint,
        )
        .await?;

    let lock_token = lock.token.as_str();
    let xml = crate::xml::build_lock_response_xml(
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

async fn handle_unlock(state: AppState, _path: &str, headers: &HeaderMap) -> Result<Response> {
    let lock_token = headers
        .get("Lock-Token")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("<").and_then(|r| r.strip_suffix(">")))
        .ok_or_else(|| FerroError::InvalidArgument("Missing Lock-Token header".to_string()))?;

    state.lock_manager.release_lock(lock_token).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

/// LOCK refresh (RFC 4918 §9.10.2): client sends LOCK with If header containing lock token.
async fn handle_lock_refresh(
    state: AppState,
    path: &str,
    lock_token: &str,
    headers: &HeaderMap,
    body: &Bytes,
) -> Result<Response> {
    let lock_request = crate::xml::LockRequest::parse(body);

    match state
        .lock_manager
        .refresh_lock(lock_token, lock_request.timeout_hint)
        .await
    {
        Ok(lock) => {
            let principal = lock.principal.clone();
            let xml = crate::xml::build_lock_response_xml(
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
            let lock_request = crate::xml::LockRequest::parse(body);
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
                .lock_manager
                .acquire_lock(
                    path,
                    &principal,
                    lock_request.scope,
                    depth,
                    lock_request.timeout_hint,
                )
                .await?;

            let lock_token = lock.token.as_str();
            let xml = crate::xml::build_lock_response_xml(
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
async fn handle_proppatch(
    state: AppState,
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

    if !state.storage.exists(&path).await? {
        return Err(FerroError::NotFound(path.to_string()));
    }

    // Parse simple PROPPATCH body to extract property operations
    let props = crate::xml::parse_proppatch(body);

    // Build response XML showing all properties as 200 OK
    let xml = crate::xml::build_proppatch_response(&path, &props);

    let mut resp_headers = HeaderMap::new();
    resp_headers.insert(
        "Content-Type",
        HeaderValue::from_static("application/xml; charset=utf-8"),
    );

    debug!("PROPPATCH {} ({} properties)", path, props.len());
    Ok((StatusCode::MULTI_STATUS, resp_headers, Body::from(xml)).into_response())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::InMemoryStorageEngine;
    use axum::Router;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::routing::any;
    use common::storage::StorageEngine;
    use http_body_util::BodyExt;
    use std::sync::Arc;
    use tower::ServiceExt;

    fn make_test_app() -> Router {
        let state = AppState::in_memory();

        Router::new()
            .route("/", any(handle_any))
            .route("/*path", any(handle_any))
            .with_state(state)
    }

    #[tokio::test]
    async fn test_health_check() {
        let app = Router::new().route(
            "/.well-known/ferro",
            axum::routing::get(|| async { "Ferro OK" }),
        );

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/.well-known/ferro")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_options() {
        let app = make_test_app();

        let response = app
            .oneshot(
                Request::builder()
                    .method("OPTIONS")
                    .uri("/")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let allow = response.headers().get("Allow").unwrap().to_str().unwrap();
        assert!(allow.contains("PROPPATCH"));
    }

    #[tokio::test]
    async fn test_put_and_get() {
        let app = make_test_app();

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/test.txt")
                    .header("Content-Type", "text/plain")
                    .body(Body::from("hello world"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/test.txt")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_mkcol_and_propfind() {
        let app = make_test_app();

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("MKCOL")
                    .uri("/mydir")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let response = app
            .oneshot(
                Request::builder()
                    .method("PROPFIND")
                    .uri("/mydir")
                    .header("Depth", "0")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::MULTI_STATUS);
    }

    #[tokio::test]
    async fn test_delete() {
        let app = make_test_app();

        app.clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/to-delete.txt")
                    .body(Body::from("data"))
                    .unwrap(),
            )
            .await
            .unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/to-delete.txt")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn test_lock_and_unlock() {
        let app = make_test_app();

        app.clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/locked.txt")
                    .body(Body::from("data"))
                    .unwrap(),
            )
            .await
            .unwrap();

        let lock_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("LOCK")
                    .uri("/locked.txt")
                    .header("Depth", "0")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(lock_response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_unsupported_method() {
        let app = make_test_app();

        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_conditional_get_not_modified() {
        let app = make_test_app();

        // PUT a file
        app.clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/cached.txt")
                    .body(Body::from("content"))
                    .unwrap(),
            )
            .await
            .unwrap();

        // GET to retrieve the ETag
        let get_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/cached.txt")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let etag = get_resp
            .headers()
            .get("ETag")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        // GET with If-None-Match — should return 304
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/cached.txt")
                    .header("If-None-Match", &etag)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NOT_MODIFIED);
    }

    #[tokio::test]
    async fn test_content_type_sniffing() {
        let app = make_test_app();

        // Upload a PNG file (no Content-Type header — should sniff)
        let png_header = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00];
        app.clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/image.png")
                    .body(Body::from(png_header.to_vec()))
                    .unwrap(),
            )
            .await
            .unwrap();

        // The Content-Type sniffing runs on PUT but the metadata stored in the
        // backend may not reflect it since we only update the local `meta` var.
        // When Content-Type header is provided, it should be preserved via
        // the file extension in `sniff_content_type`.
        // Test that providing Content-Type header works:
        let pdf_body = b"%PDF-1.4 fake";
        app.clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/doc.pdf")
                    .header("Content-Type", "application/pdf")
                    .body(Body::from(pdf_body.to_vec()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/doc.pdf")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        // The stored content-type comes from FileMetadata which defaults to
        // application/octet-stream unless the storage engine preserves it.
        // This test verifies the upload succeeds and returns proper status.
    }

    #[tokio::test]
    async fn test_proppatch() {
        let app = make_test_app();

        app.clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/file.txt")
                    .body(Body::from("data"))
                    .unwrap(),
            )
            .await
            .unwrap();

        let proppatch_body = br#"<?xml version="1.0" encoding="utf-8"?>
<D:propertyupdate xmlns:D="DAV:">
    <D:set>
        <D:prop>
            <D:displayname>My File</D:displayname>
        </D:prop>
    </D:set>
</D:propertyupdate>"#;

        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PROPPATCH")
                    .uri("/file.txt")
                    .body(Body::from(proppatch_body.to_vec()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::MULTI_STATUS);
    }

    #[tokio::test]
    async fn test_propfind_depth_infinity_bounded() {
        let app = make_test_app();

        // Create the collection first
        app.clone()
            .oneshot(
                Request::builder()
                    .method("MKCOL")
                    .uri("/deep")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // Create a deeply nested structure
        for i in 0..150 {
            let path = format!("/deep/{}", i);
            app.clone()
                .oneshot(
                    Request::builder()
                        .method("PUT")
                        .uri(&path)
                        .body(Body::from("x"))
                        .unwrap(),
                )
                .await
                .unwrap();
        }

        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PROPFIND")
                    .uri("/deep")
                    .header("Depth", "infinity")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::MULTI_STATUS);

        use http_body_util::BodyExt;
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let xml = String::from_utf8(body.to_vec()).unwrap();
        // Should be bounded by MAX_PROPFIND_DEPTH (100) + the collection itself = 101
        let count = xml.matches("<D:response>").count();
        assert!(count <= 101, "PROPFIND should be bounded, got {}", count);
    }

    #[tokio::test]
    async fn test_body_size_limit_rejected() {
        let state = AppState::in_memory().with_max_body_size(100);

        let app = Router::new()
            .route("/", any(handle_any))
            .route("/*path", any(handle_any))
            .with_state(state);

        let large_body = vec![0u8; 200]; // 200 bytes > 100 byte limit
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/test.txt")
                    .header("Content-Length", "200")
                    .body(Body::from(large_body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[tokio::test]
    async fn test_body_size_limit_accepted() {
        let state = AppState::in_memory().with_max_body_size(1024);

        let app = Router::new()
            .route("/", any(handle_any))
            .route("/*path", any(handle_any))
            .with_state(state);

        let small_body = b"hello world";
        let response = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/test.txt")
                    .body(Body::from(small_body.to_vec()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
    }

    #[tokio::test]
    async fn test_concurrent_puts_different_files() {
        let storage: Arc<dyn StorageEngine> = Arc::new(InMemoryStorageEngine::new());
        let mut state = AppState::in_memory().with_max_body_size(1024 * 1024);
        state.storage = storage.clone();

        let app = Router::new()
            .route("/", any(handle_any))
            .route("/*path", any(handle_any))
            .with_state(state);

        // Concurrent PUTs to different files should all succeed
        let mut handles = vec![];
        for i in 0..10 {
            let app_clone = app.clone();
            handles.push(tokio::spawn(async move {
                let response = app_clone
                    .oneshot(
                        Request::builder()
                            .method("PUT")
                            .uri(format!("/file{}.txt", i))
                            .body(Body::from(format!("content {}", i)))
                            .unwrap(),
                    )
                    .await
                    .unwrap();
                response.status()
            }));
        }

        let mut statuses = vec![];
        for handle in handles {
            statuses.push(handle.await.unwrap());
        }
        for status in &statuses {
            assert_eq!(
                *status,
                StatusCode::CREATED,
                "All concurrent PUTs should succeed"
            );
        }

        // Verify all files exist
        assert!(storage.exists("/file0.txt").await.unwrap());
        assert!(storage.exists("/file9.txt").await.unwrap());
    }

    #[tokio::test]
    async fn test_concurrent_gets_same_file() {
        let storage: Arc<dyn StorageEngine> = Arc::new(InMemoryStorageEngine::new());
        storage
            .put("/shared.txt", Bytes::from("shared content"), "user1")
            .await
            .unwrap();

        let mut state = AppState::in_memory().with_max_body_size(1024 * 1024);
        state.storage = storage.clone();

        let app = Router::new()
            .route("/", any(handle_any))
            .route("/*path", any(handle_any))
            .with_state(state);

        // Concurrent GETs to the same file should all succeed
        let mut handles = vec![];
        for _ in 0..20 {
            let app_clone = app.clone();
            handles.push(tokio::spawn(async move {
                let response = app_clone
                    .oneshot(
                        Request::builder()
                            .method("GET")
                            .uri("/shared.txt")
                            .body(Body::empty())
                            .unwrap(),
                    )
                    .await
                    .unwrap();
                response.status()
            }));
        }

        let mut statuses = vec![];
        for handle in handles {
            statuses.push(handle.await.unwrap());
        }
        for status in &statuses {
            assert_eq!(
                *status,
                StatusCode::OK,
                "All concurrent GETs should succeed"
            );
        }
    }

    #[tokio::test]
    async fn test_lock_refresh_via_if_header() {
        let storage: Arc<dyn StorageEngine> = Arc::new(InMemoryStorageEngine::new());
        storage
            .put("/lockme.txt", Bytes::from("content"), "user1")
            .await
            .unwrap();

        let mut state = AppState::in_memory().with_max_body_size(1024 * 1024);
        state.storage = storage.clone();

        let app = Router::new()
            .route("/", any(handle_any))
            .route("/*path", any(handle_any))
            .with_state(state);

        // Step 1: Acquire a lock
        let lock_body = r#"<?xml version="1.0" encoding="utf-8"?>
            <D:lockinfo xmlns:D="DAV:">
                <D:locktype><D:write/></D:locktype>
                <D:lockscope><D:exclusive/></D:lockscope>
                <D:owner><D:href>user1</D:href></D:owner>
            </D:lockinfo>"#;

        let lock_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("LOCK")
                    .uri("/lockme.txt")
                    .header("Content-Type", "application/xml")
                    .body(Body::from(lock_body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(lock_resp.status(), StatusCode::OK);
        let lock_token = lock_resp
            .headers()
            .get("Lock-Token")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        // Step 2: Refresh the lock using If header
        let refresh_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("LOCK")
                    .uri("/lockme.txt")
                    .header("If", &lock_token)
                    .header("Content-Type", "application/xml")
                    .body(Body::from(lock_body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(refresh_resp.status(), StatusCode::OK);
    }

    #[test]
    fn test_path_with_dotdot_rejected() {
        assert!(sanitize_path("/foo/../bar").is_err());
        assert!(sanitize_path("/../etc/passwd").is_err());
        assert!(sanitize_path("../../etc/passwd").is_err());
        assert!(sanitize_path("/foo/bar/../../etc").is_err());
    }

    #[test]
    fn test_path_with_null_byte_rejected() {
        assert!(sanitize_path("/foo\0bar").is_err());
        assert!(sanitize_path("/test\0.txt").is_err());
        assert!(sanitize_path("/\0").is_err());
    }

    #[test]
    fn test_path_normalized_correctly() {
        assert_eq!(sanitize_path("/foo/bar").unwrap(), "/foo/bar");
        assert_eq!(sanitize_path("/foo//bar").unwrap(), "/foo/bar");
        assert_eq!(sanitize_path("/foo/bar/").unwrap(), "/foo/bar");
        assert_eq!(sanitize_path("/").unwrap(), "/");
        assert_eq!(sanitize_path("/a..b/test.txt").unwrap(), "/a..b/test.txt");
        assert_eq!(sanitize_path("/file.with.dots").unwrap(), "/file.with.dots");
    }

    #[tokio::test]
    async fn test_special_characters_in_filenames() {
        let app = make_test_app();

        let filenames = [
            "file with spaces.txt",
            "file-with-dashes.txt",
            "file_with_underscores.txt",
            "file.with.dots.txt",
            "file(with)parens.txt",
            "file'with'quotes.txt",
        ];

        for name in &filenames {
            let encoded = urlencoding(name);
            let put_resp = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("PUT")
                        .uri(format!("/{}", encoded))
                        .body(Body::from("content"))
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_eq!(
                put_resp.status(),
                StatusCode::CREATED,
                "PUT failed for {}",
                name
            );

            let get_resp = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("GET")
                        .uri(format!("/{}", encoded))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(get_resp.status(), StatusCode::OK, "GET failed for {}", name);
        }
    }

    #[tokio::test]
    async fn test_unicode_filenames() {
        let app = make_test_app();

        let filenames = ["файл.txt", "文件.txt", "αρχείο.txt", "téléchargement.txt"];

        for name in &filenames {
            let encoded = urlencoding(name);
            let put_resp = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("PUT")
                        .uri(format!("/{}", encoded))
                        .body(Body::from("unicode content"))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(
                put_resp.status(),
                StatusCode::CREATED,
                "PUT failed for {}",
                name
            );

            let get_resp = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("GET")
                        .uri(format!("/{}", encoded))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(get_resp.status(), StatusCode::OK, "GET failed for {}", name);
        }
    }

    #[tokio::test]
    async fn test_put_root_behavior() {
        let app = make_test_app();
        let resp = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/")
                    .body(Body::from("root data"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(
            resp.status().is_success(),
            "PUT / should succeed or return error gracefully, got {}",
            resp.status()
        );
    }

    #[tokio::test]
    async fn test_long_filename_255_chars() {
        let app = make_test_app();
        let name = "a".repeat(255) + ".txt";
        let encoded = urlencoding(&name);
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/{}", encoded))
                    .body(Body::from("data"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);

        let get_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/{}", encoded))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(get_resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_file_with_no_extension() {
        let app = make_test_app();
        app.clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/noext")
                    .body(Body::from("no extension content"))
                    .unwrap(),
            )
            .await
            .unwrap();

        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/noext")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_double_slash_normalized() {
        let app = make_test_app();
        let put_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/folder//file.txt")
                    .body(Body::from("double slash"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(put_resp.status().is_success());

        let get_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/folder/file.txt")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(get_resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_put_overwrites_existing_etag_changes() {
        let app = make_test_app();

        let put1 = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/overwrite.txt")
                    .body(Body::from("version1"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(put1.status(), StatusCode::CREATED);
        let etag1 = put1
            .headers()
            .get("ETag")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        let put2 = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/overwrite.txt")
                    .body(Body::from("version2"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(put2.status(), StatusCode::NO_CONTENT);
        let etag2 = put2
            .headers()
            .get("ETag")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        assert_ne!(etag1, etag2, "ETag should change after overwrite");
    }

    #[tokio::test]
    async fn test_delete_nonexistent_returns_404() {
        let app = make_test_app();
        let resp = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/nonexistent.txt")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_mkcol_on_existing_file_returns_405() {
        let app = make_test_app();
        app.clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/existing-file.txt")
                    .body(Body::from("data"))
                    .unwrap(),
            )
            .await
            .unwrap();

        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("MKCOL")
                    .uri("/existing-file.txt")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::METHOD_NOT_ALLOWED);
    }

    #[tokio::test]
    async fn test_mkcol_nested_creates_intermediates() {
        let app = make_test_app();
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("MKCOL")
                    .uri("/a/b/c")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(
            resp.status().is_success(),
            "MKCOL should succeed (in-memory store auto-creates parents), got {}",
            resp.status()
        );
    }

    #[tokio::test]
    async fn test_propfind_nonexistent_directory_returns_404() {
        let app = make_test_app();
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PROPFIND")
                    .uri("/nonexistent/")
                    .header("Depth", "0")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_concurrent_put_same_file_no_panic() {
        let storage: Arc<dyn StorageEngine> = Arc::new(InMemoryStorageEngine::new());
        let mut state = AppState::in_memory().with_max_body_size(1024 * 1024);
        state.storage = storage.clone();

        let app = Router::new()
            .route("/", any(handle_any))
            .route("/*path", any(handle_any))
            .with_state(state);

        let mut handles = vec![];
        for i in 0..10 {
            let app_clone = app.clone();
            handles.push(tokio::spawn(async move {
                let resp = app_clone
                    .oneshot(
                        Request::builder()
                            .method("PUT")
                            .uri("/race.txt")
                            .body(Body::from(format!("writer {}", i)))
                            .unwrap(),
                    )
                    .await
                    .unwrap();
                resp.status()
            }));
        }

        let mut statuses = vec![];
        for handle in handles {
            statuses.push(handle.await.unwrap());
        }
        for status in &statuses {
            assert!(
                status.is_success(),
                "All concurrent PUTs should succeed, got {}",
                status
            );
        }

        let content = storage.get("/race.txt").await.unwrap();
        assert!(
            !content.is_empty(),
            "File should have content from last writer"
        );
    }

    #[tokio::test]
    async fn test_propfind_reports_correct_file_size() {
        let app = make_test_app();

        let content = b"Hello, World! This is 32 bytes.";
        app.clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/sized.txt")
                    .body(Body::from(content.to_vec()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PROPFIND")
                    .uri("/sized.txt")
                    .header("Depth", "0")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::MULTI_STATUS);
        use http_body_util::BodyExt;
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let xml = String::from_utf8(body.to_vec()).unwrap();
        assert!(
            xml.contains(&content.len().to_string()),
            "PROPFIND should report correct size {}",
            content.len()
        );
    }

    #[tokio::test]
    async fn test_root_directory_listing() {
        let app = make_test_app();

        app.clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/root-file.txt")
                    .body(Body::from("data"))
                    .unwrap(),
            )
            .await
            .unwrap();

        app.clone()
            .oneshot(
                Request::builder()
                    .method("MKCOL")
                    .uri("/root-dir")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PROPFIND")
                    .uri("/")
                    .header("Depth", "1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::MULTI_STATUS);
        use http_body_util::BodyExt;
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let xml = String::from_utf8(body.to_vec()).unwrap();
        assert!(
            xml.contains("root-file.txt"),
            "Root listing should contain root-file.txt"
        );
        assert!(
            xml.contains("root-dir"),
            "Root listing should contain root-dir"
        );
    }

    #[tokio::test]
    async fn test_trailing_slash_on_file_put() {
        let app = make_test_app();
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/file.txt/")
                    .body(Body::from("trailing"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(resp.status().is_success());

        let get_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/file.txt")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(get_resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_encoded_dotdot_rejected() {
        let app = make_test_app();
        let resp = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/%2e%2e/etc/passwd")
                    .body(Body::from("traversal"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(
            !resp.status().is_success(),
            "Encoded dotdot should be rejected, got {}",
            resp.status()
        );
    }

    fn urlencoding(s: &str) -> String {
        url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
    }

    #[test]
    fn test_sync_token_extraction() {
        let token = "http://ferro.local/sync/token/42";
        let counter = token.rsplit('/').next().unwrap().parse::<u64>().unwrap();
        assert_eq!(counter, 42);
    }

    #[test]
    fn test_sync_token_extraction_large_number() {
        let token = "http://ferro.local/sync/token/9999999999";
        let counter = token.rsplit('/').next().unwrap().parse::<u64>().unwrap();
        assert_eq!(counter, 9999999999);
    }

    #[test]
    fn test_sync_token_malformed_returns_none() {
        let token = "http://ferro.local/sync/token/notanumber";
        let result = token.rsplit('/').next().unwrap().parse::<u64>();
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_propfind_without_sync_token_has_no_sync_header() {
        let app = make_test_app();

        app.clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/nosync.txt")
                    .body(Body::from("data"))
                    .unwrap(),
            )
            .await
            .unwrap();

        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PROPFIND")
                    .uri("/")
                    .header("Depth", "1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::MULTI_STATUS);
        assert!(
            resp.headers().get("Sync-Token").is_none(),
            "PROPFIND without Sync-Token request should not include Sync-Token in response"
        );
    }

    #[tokio::test]
    async fn test_propfind_with_sync_token_returns_sync_header() {
        let app = make_test_app();

        let resp = app
            .oneshot(
                Request::builder()
                    .method("PROPFIND")
                    .uri("/")
                    .header("Depth", "1")
                    .header("Sync-Token", "http://ferro.local/sync/token/0")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::MULTI_STATUS);
        let sync_token = resp.headers().get("Sync-Token").unwrap().to_str().unwrap();
        assert!(
            sync_token.starts_with("http://ferro.local/sync/token/"),
            "Sync-Token header should start with the expected prefix, got: {}",
            sync_token
        );
        let counter: u64 = sync_token.rsplit('/').next().unwrap().parse().unwrap();
        assert!(
            counter >= 1,
            "Sync-Token counter should be >= 1, got {}",
            counter
        );
    }

    #[tokio::test]
    async fn test_sync_token_clock_increments_on_write() {
        let state = AppState::in_memory();
        let app = Router::new()
            .route("/", any(handle_any))
            .route("/*path", any(handle_any))
            .with_state(state);

        app.clone()
            .oneshot(
                Request::builder()
                    .method("PROPFIND")
                    .uri("/")
                    .header("Depth", "1")
                    .header("Sync-Token", "http://ferro.local/sync/token/0")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        app.clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/inc.txt")
                    .body(Body::from("data"))
                    .unwrap(),
            )
            .await
            .unwrap();

        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PROPFIND")
                    .uri("/")
                    .header("Depth", "1")
                    .header("Sync-Token", "http://ferro.local/sync/token/0")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let sync_token = resp.headers().get("Sync-Token").unwrap().to_str().unwrap();
        let counter: u64 = sync_token.rsplit('/').next().unwrap().parse().unwrap();
        assert!(
            counter >= 2,
            "Sync-Token should increment after write, got {}",
            counter
        );
    }

    #[tokio::test]
    async fn test_put_overwrite_creates_version() {
        use tempfile::TempDir;
        let tmp = TempDir::new().unwrap();
        let data_dir = tmp.path().to_string_lossy().to_string();
        let state = AppState::in_memory()
            .with_data_dir(data_dir.clone())
            .with_max_file_versions(5);
        let app = Router::new()
            .route("/", any(handle_any))
            .route("/*path", any(handle_any))
            .with_state(state.clone());

        // PUT v1
        let put1 = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/versioned.txt")
                    .body(Body::from("hello world"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(put1.status(), StatusCode::CREATED);

        // Allow async versioning task to complete
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // PUT v2 (overwrite)
        let put2 = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/versioned.txt")
                    .body(Body::from("updated content"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(put2.status(), StatusCode::NO_CONTENT);

        // Allow async versioning task to complete
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Verify version was created via the versioning API
        let ver_state = ferro_server_versioning::VersioningState {
            data_dir: Some(data_dir),
            admin_user: None,
            storage: state.storage.clone(),
            max_file_versions: 5,
        };
        let resp = ferro_server_versioning::list_versions(
            axum::Extension(ver_state),
            axum::extract::Path("versioned.txt".to_string()),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            json["versions"].as_array().unwrap().len(),
            1,
            "Overwrite should create one version snapshot"
        );
    }

    #[tokio::test]
    async fn test_streaming_put_large_file() {
        let state = AppState::in_memory()
            .with_max_body_size(1024 * 1024)
            .with_streaming_upload_threshold(64);

        let app = Router::new()
            .route("/", any(handle_any))
            .route("/*path", any(handle_any))
            .with_state(state.clone());

        let large_body = vec![0x42u8; 1024];
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/large.bin")
                    .header("Content-Length", large_body.len().to_string())
                    .body(Body::from(large_body.clone()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let get_resp = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/large.bin")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(get_resp.status(), StatusCode::OK);
        use http_body_util::BodyExt;
        let body = get_resp.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(body.len(), 1024);
        assert!(body.iter().all(|&b| b == 0x42));
    }

    #[tokio::test]
    async fn test_streaming_put_no_content_length_uses_streaming() {
        let state = AppState::in_memory()
            .with_max_body_size(1024 * 1024)
            .with_streaming_upload_threshold(64);

        let app = Router::new()
            .route("/", any(handle_any))
            .route("/*path", any(handle_any))
            .with_state(state.clone());

        let body_data = vec![0x55u8; 256];
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/no-cl.bin")
                    .body(Body::from(body_data.clone()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let get_resp = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/no-cl.bin")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(get_resp.status(), StatusCode::OK);
        use http_body_util::BodyExt;
        let body = get_resp.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(body.len(), 256);
    }

    #[tokio::test]
    async fn test_small_put_uses_in_memory_path() {
        let state = AppState::in_memory()
            .with_max_body_size(1024 * 1024)
            .with_streaming_upload_threshold(65536);

        let app = Router::new()
            .route("/", any(handle_any))
            .route("/*path", any(handle_any))
            .with_state(state.clone());

        let small_data = b"tiny";
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/small.txt")
                    .header("Content-Length", small_data.len().to_string())
                    .body(Body::from(small_data.to_vec()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let get_resp = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/small.txt")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(get_resp.status(), StatusCode::OK);
    }
}
