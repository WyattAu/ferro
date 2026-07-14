use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;

use ferro_server::AppState;
use ferro_server::api_error::ApiError;
use ferro_server::audit::build_audit_entry;
use ferro_server::users;

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct AdminStatsResponse {
    pub version: String,
    pub uptime_seconds: u64,
    pub total_files: u64,
    pub total_directories: u64,
    pub total_bytes: u64,
    pub storage_backend: String,
    pub auth_type: String,
    pub wasm_workers_loaded: u32,
    pub search_enabled: bool,
    pub features: AdminFeatures,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct AdminFeatures {
    pub s3: bool,
    pub gcs: bool,
    pub azure: bool,
    pub oidc: bool,
    pub cedar: bool,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct AdminStorageResponse {
    pub backend: String,
    pub total_bytes: u64,
    pub file_count: u64,
    pub directory_count: u64,
    pub largest_file: serde_json::Value,
    pub recent_files: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct AdminAuditResponse {
    pub entries: Vec<serde_json::Value>,
    pub total: usize,
    pub limit: usize,
    pub offset: usize,
}

/// Return server statistics (version, uptime, file counts).
#[utoipa::path(
    get,
    path = "/api/admin/stats",
    responses(
        (status = 200, description = "Server statistics", body = AdminStatsResponse),
    ),
    tags = ["admin"],
)]
pub async fn admin_stats(State(state): State<AppState>) -> Response {
    let version = env!("CARGO_PKG_VERSION");
    let uptime = state.started_at.elapsed().as_secs();

    let mut file_count = 0u64;
    let mut collection_count = 0u64;
    let mut total_bytes = 0u64;

    if let Ok(entries) = state.storage.list_all("/", 10000).await {
        for meta in &entries {
            if meta.is_collection {
                collection_count += 1;
            } else {
                file_count += 1;
                total_bytes += meta.size;
            }
        }
    }

    let auth_type = if state.oidc.is_some() {
        "oidc"
    } else if state.admin_user.is_some() {
        "basic"
    } else {
        "none"
    };

    let wasm_workers_loaded = 0u32;

    let body = AdminStatsResponse {
        version: version.to_string(),
        uptime_seconds: uptime,
        total_files: file_count,
        total_directories: collection_count,
        total_bytes,
        storage_backend: "memory".to_string(),
        auth_type: auth_type.to_string(),
        wasm_workers_loaded,
        search_enabled: state.search.is_some(),
        features: AdminFeatures {
            #[allow(unexpected_cfgs)]
            s3: cfg!(feature = "s3"),
            #[allow(unexpected_cfgs)]
            gcs: cfg!(feature = "gcs"),
            #[allow(unexpected_cfgs)]
            azure: cfg!(feature = "azure"),
            oidc: state.oidc.is_some(),
            cedar: state.cedar.is_some(),
        },
    };

    (StatusCode::OK, axum::Json(body)).into_response()
}

/// Query parameters for the admin storage endpoint.
#[derive(Debug, Deserialize, Default, utoipa::IntoParams)]
pub struct StorageQueryParams {
    pub limit: Option<usize>,
}

/// Return detailed storage statistics.
#[utoipa::path(
    get,
    path = "/api/admin/storage",
    params(StorageQueryParams),
    responses(
        (status = 200, description = "Storage statistics", body = AdminStorageResponse),
    ),
    tags = ["admin"],
)]
pub async fn admin_storage(State(state): State<AppState>, Query(_params): Query<StorageQueryParams>) -> Response {
    let mut file_count = 0u64;
    let mut collection_count = 0u64;
    let mut total_bytes = 0u64;
    let mut largest_file_path: String = String::new();
    let mut largest_file_size: u64 = 0;
    let mut recent_files: Vec<serde_json::Value> = Vec::new();

    if let Ok(entries) = state.storage.list_all("/", 10000).await {
        for meta in &entries {
            if meta.is_collection {
                collection_count += 1;
            } else {
                file_count += 1;
                total_bytes += meta.size;

                if meta.size > largest_file_size {
                    largest_file_size = meta.size;
                    largest_file_path = meta.path.clone();
                }

                recent_files.push(serde_json::json!({
                    "path": meta.path,
                    "size": meta.size,
                    "modified_at": meta.modified_at.to_rfc3339(),
                }));
            }
        }
    }

    recent_files.sort_by(|a, b| {
        let a_time = a["modified_at"].as_str().unwrap_or("");
        let b_time = b["modified_at"].as_str().unwrap_or("");
        b_time.cmp(a_time)
    });
    recent_files.truncate(10);

    let largest_file = if largest_file_path.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::json!({
            "path": largest_file_path,
            "size": largest_file_size,
        })
    };

    let body = AdminStorageResponse {
        backend: "memory".to_string(),
        total_bytes,
        file_count,
        directory_count: collection_count,
        largest_file,
        recent_files,
    };

    (StatusCode::OK, axum::Json(body)).into_response()
}

/// Query parameters for the admin audit endpoint.
#[derive(Debug, Deserialize, Default, utoipa::IntoParams)]
pub struct AuditQueryParams {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub user_id: Option<String>,
    pub action: Option<String>,
    pub path: Option<String>,
    pub since: Option<String>,
    pub until: Option<String>,
}

/// Return paginated audit log entries.
#[utoipa::path(
    get,
    path = "/api/admin/audit",
    params(AuditQueryParams),
    responses(
        (status = 200, description = "Audit log entries", body = AdminAuditResponse),
    ),
    tags = ["admin"],
)]
pub async fn admin_audit(State(state): State<AppState>, Query(params): Query<AuditQueryParams>) -> Response {
    let limit: usize = params.limit.unwrap_or(100);
    let offset: usize = params.offset.unwrap_or(0);

    let all_entries = state.audit_log.entries().await;

    let filtered: Vec<_> = all_entries
        .into_iter()
        .filter(|e| {
            if let Some(ref uid) = params.user_id
                && &e.user != uid
            {
                return false;
            }
            if let Some(ref action) = params.action
                && &e.method != action
            {
                return false;
            }
            if let Some(ref pf) = params.path
                && !e.path.contains(pf.as_str())
            {
                return false;
            }
            if let Some(ref since) = params.since
                && let Ok(since_ts) = chrono::DateTime::parse_from_rfc3339(since)
                && let Ok(entry_ts) = chrono::DateTime::parse_from_rfc3339(&e.timestamp)
                && entry_ts.with_timezone(&chrono::Utc) < since_ts.with_timezone(&chrono::Utc)
            {
                return false;
            }
            if let Some(ref until) = params.until
                && let Ok(until_ts) = chrono::DateTime::parse_from_rfc3339(until)
                && let Ok(entry_ts) = chrono::DateTime::parse_from_rfc3339(&e.timestamp)
                && entry_ts.with_timezone(&chrono::Utc) > until_ts.with_timezone(&chrono::Utc)
            {
                return false;
            }
            true
        })
        .collect();

    let total = filtered.len();
    let entries_json: Vec<serde_json::Value> = filtered
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(|e| serde_json::to_value(&e).unwrap_or_default())
        .collect();

    (
        StatusCode::OK,
        axum::Json(AdminAuditResponse {
            entries: entries_json,
            total,
            limit,
            offset,
        }),
    )
        .into_response()
}

/// GET /api/admin/maintenance — check current maintenance mode status.
/// POST /api/admin/maintenance — toggle maintenance mode on/off.
/// The body should be `{ "enabled": true }` or `{ "enabled": false }`.
pub async fn admin_maintenance(State(state): State<AppState>, req: axum::extract::Request) -> Response {
    let method = req.method().clone();

    if method == axum::http::Method::GET {
        let enabled = state.maintenance_mode.load(std::sync::atomic::Ordering::Relaxed);
        return (
            StatusCode::OK,
            axum::Json(serde_json::json!({
                "maintenance_mode": enabled,
            })),
        )
            .into_response();
    }

    // POST: toggle maintenance mode
    let (_, body) = req.into_parts();
    let bytes = match axum::body::to_bytes(body, 1024).await {
        Ok(b) => b,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                axum::Json(serde_json::json!({
                    "error": "bad_request",
                    "message": "Failed to read request body",
                })),
            )
                .into_response();
        }
    };

    let input: serde_json::Value = match serde_json::from_slice(&bytes) {
        Ok(v) => v,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                axum::Json(serde_json::json!({
                    "error": "bad_request",
                    "message": "Request body must be JSON with 'enabled' boolean field",
                })),
            )
                .into_response();
        }
    };

    let enabled = input["enabled"].as_bool().unwrap_or(false);
    state
        .maintenance_mode
        .store(enabled, std::sync::atomic::Ordering::Relaxed);

    if enabled {
        tracing::warn!("Maintenance mode ENABLED — write operations are blocked");
    } else {
        tracing::info!("Maintenance mode DISABLED — normal operations resumed");
    }

    (
        StatusCode::OK,
        axum::Json(serde_json::json!({
            "maintenance_mode": enabled,
        })),
    )
        .into_response()
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct GdprErasureResponse {
    pub user_id: String,
    pub deleted_files: u64,
    pub deleted_shares: u64,
    pub deleted_favorites: u64,
    pub deleted_tags: u64,
    pub user_disabled: bool,
}

#[utoipa::path(
    get,
    path = "/api/admin/users/{id}/export",
    responses(
        (status = 200, description = "ZIP archive of user data"),
        (status = 404, description = "User not found"),
    ),
    tags = ["admin"],
)]
pub async fn admin_export_user_data(State(state): State<AppState>, Path(user_id): Path<String>) -> Response {
    let user = match state.user_store.get_user(&user_id).await {
        Ok(u) => u,
        Err(_) => {
            return ApiError::not_found(ApiError::USER_NOT_FOUND, "User not found");
        }
    };

    let tmp_dir = match tempfile::tempdir() {
        Ok(d) => d,
        Err(e) => {
            tracing::warn!(error = %e, "failed to create temp dir for GDPR export");
            return ApiError::internal(ApiError::INTERNAL_ERROR, "Failed to create export directory");
        }
    };

    let user_json = serde_json::to_string_pretty(&serde_json::json!({
        "id": user.id,
        "username": user.username,
        "display_name": user.display_name,
        "email": user.email,
        "role": format!("{:?}", user.role),
        "created_at": user.created_at.to_rfc3339(),
        "last_login": user.last_login.map(|t| t.to_rfc3339()),
        "status": format!("{:?}", user.status),
        "storage_quota_bytes": user.storage_quota_bytes,
        "storage_used_bytes": user.storage_used_bytes,
        "is_ldap": user.is_ldap,
    }))
    .unwrap_or_else(|_| "{}".to_string());
    std::fs::write(tmp_dir.path().join("user_metadata.json"), &user_json).ok();

    let files_list: Vec<serde_json::Value> = match state.storage.list_all("/", 50000).await {
        Ok(entries) => entries
            .into_iter()
            .filter_map(|meta| {
                if meta.is_collection {
                    None
                } else {
                    Some(serde_json::json!({
                        "path": meta.path,
                        "size": meta.size,
                        "mime_type": meta.mime_type,
                        "modified_at": meta.modified_at.to_rfc3339(),
                    }))
                }
            })
            .collect(),
        Err(_) => Vec::new(),
    };
    let files_json = serde_json::to_string_pretty(&files_list).unwrap_or_else(|_| "[]".to_string());
    std::fs::write(tmp_dir.path().join("files.json"), &files_json).ok();

    let audit_entries: Vec<serde_json::Value> = {
        let all = state.audit_log.entries().await;
        all.into_iter()
            .filter(|e| e.user == user.username || e.user == user.id)
            .map(|e| {
                serde_json::json!({
                    "timestamp": e.timestamp,
                    "method": e.method,
                    "path": e.path,
                    "status": e.status,
                    "client_ip": e.client_ip,
                    "user_agent": e.user_agent,
                })
            })
            .collect()
    };
    let audit_json = serde_json::to_string_pretty(&audit_entries).unwrap_or_else(|_| "[]".to_string());
    std::fs::write(tmp_dir.path().join("audit_log.json"), &audit_json).ok();

    let shares_list: Vec<serde_json::Value> = {
        let all = state.share_store.list().await;
        all.into_iter()
            .filter(|s| s.created_by == user.username || s.created_by == user.id)
            .map(|s| {
                serde_json::json!({
                    "token": s.token,
                    "path": s.path,
                    "expires_at": s.expires_at.to_rfc3339(),
                    "max_downloads": s.max_downloads,
                    "download_count": s.download_count,
                    "created_by": s.created_by,
                    "allow_download": s.allow_download,
                    "allow_upload": s.allow_upload,
                })
            })
            .collect()
    };
    let shares_json = serde_json::to_string_pretty(&shares_list).unwrap_or_else(|_| "[]".to_string());
    std::fs::write(tmp_dir.path().join("shares.json"), &shares_json).ok();

    let favorites_list: Vec<String> = state.favorites.list().await;
    let fav_json = serde_json::to_string_pretty(&favorites_list).unwrap_or_else(|_| "[]".to_string());
    std::fs::write(tmp_dir.path().join("favorites.json"), &fav_json).ok();

    let tags_list: Vec<serde_json::Value> = state
        .tags
        .entries
        .iter()
        .map(|entry| {
            let (path, tags) = entry.pair();
            serde_json::json!({
                "path": path,
                "tags": tags.iter().collect::<Vec<_>>(),
            })
        })
        .collect();
    let tags_json = serde_json::to_string_pretty(&tags_list).unwrap_or_else(|_| "[]".to_string());
    std::fs::write(tmp_dir.path().join("tags.json"), &tags_json).ok();

    let zip_buf = Vec::new();
    let zip_writer = zip::ZipWriter::new(std::io::Cursor::new(zip_buf));
    let mut zip_writer = zip_writer;

    let options = zip::write::FileOptions::<()>::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .compression_level(Some(6));

    for entry in std::fs::read_dir(tmp_dir.path()).unwrap_or_else(|_| std::fs::read_dir(tmp_dir.path()).unwrap()) {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
        if let Err(e) = zip_writer.start_file(&name, options) {
            tracing::warn!(error = %e, file = %name, "failed to start zip entry");
            continue;
        }
        if let Ok(data) = std::fs::read(&path)
            && let Err(e) = zip_writer.write_all(&data)
        {
            tracing::warn!(error = %e, file = %name, "failed to write zip entry");
        }
    }

    let cursor = zip_writer.finish().unwrap_or_else(|e| {
        tracing::warn!(error = %e, "failed to finalize ZIP");
        std::io::Cursor::new(Vec::new())
    });
    let zip_bytes = cursor.into_inner();

    if zip_bytes.is_empty() {
        return ApiError::internal(ApiError::INTERNAL_ERROR, "Export archive is empty");
    }

    state
        .audit_log
        .log(build_audit_entry(
            "GET",
            &format!("/api/admin/users/{}/export", user_id),
            "admin",
            200,
            None,
            None,
        ))
        .await;

    let filename = format!("gdpr-export-{}.zip", user.username);
    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static("application/zip"),
    );
    headers.insert(
        axum::http::header::CONTENT_DISPOSITION,
        axum::http::HeaderValue::from_str(&format!("attachment; filename=\"{}\"", filename))
            .unwrap_or_else(|_| axum::http::HeaderValue::from_static("attachment; filename=\"export.zip\"")),
    );
    (headers, axum::body::Body::from(zip_bytes)).into_response()
}

#[utoipa::path(
    delete,
    path = "/api/admin/users/{id}/data",
    responses(
        (status = 200, description = "Erasure confirmation", body = GdprErasureResponse),
        (status = 404, description = "User not found"),
        (status = 403, description = "Cannot erase admin"),
    ),
    tags = ["admin"],
)]
pub async fn admin_erase_user_data(State(state): State<AppState>, Path(user_id): Path<String>) -> Response {
    let user = match state.user_store.get_user(&user_id).await {
        Ok(u) => u,
        Err(_) => {
            return ApiError::not_found(ApiError::USER_NOT_FOUND, "User not found");
        }
    };

    if user.is_admin() {
        return (
            StatusCode::FORBIDDEN,
            axum::Json(serde_json::json!({
                "error": "Cannot erase admin account data"
            })),
        )
            .into_response();
    }

    let mut deleted_files = 0u64;
    let mut deleted_shares = 0u64;

    if let Ok(entries) = state.storage.list_all("/", 50000).await {
        for meta in entries {
            if !meta.is_collection && state.storage.delete(&meta.path).await.is_ok() {
                deleted_files += 1;
            }
        }
    }

    let all_shares = state.share_store.list().await;
    for share in &all_shares {
        if share.created_by == user.username || share.created_by == user.id {
            state.share_store.delete(&share.token).await;
            deleted_shares += 1;
        }
    }

    let deleted_favorites = {
        let all_favs = state.favorites.list().await;
        let count = all_favs.len() as u64;
        for fav_path in all_favs {
            state.favorites.remove(&fav_path).await;
        }
        count
    };

    let deleted_tags = {
        let tag_entries: Vec<(String, String)> = state
            .tags
            .entries
            .iter()
            .flat_map(|entry| {
                let (path, tags) = entry.pair();
                tags.iter().map(|tag| (path.clone(), tag.clone())).collect::<Vec<_>>()
            })
            .collect();
        let count = tag_entries.len() as u64;
        for (path, tag) in tag_entries {
            state.tags.remove_tag(&path, &tag);
        }
        count
    };

    let user_disabled = if let Err(e) = state
        .user_store
        .update_user(
            &user_id,
            users::UpdateUserRequest {
                status: Some(users::UserStatus::Disabled),
                ..Default::default()
            },
        )
        .await
    {
        tracing::warn!(error = %e.message, "failed to disable user during erasure");
        false
    } else {
        true
    };

    state
        .audit_log
        .log(build_audit_entry(
            "DELETE",
            &format!("/api/admin/users/{}/data", user_id),
            "admin",
            200,
            None,
            None,
        ))
        .await;

    (
        StatusCode::OK,
        axum::Json(GdprErasureResponse {
            user_id: user_id.clone(),
            deleted_files,
            deleted_shares,
            deleted_favorites,
            deleted_tags,
            user_disabled,
        }),
    )
        .into_response()
}

// ---------------------------------------------------------------------------
// User Management
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct AdminUserSummary {
    pub id: String,
    pub username: String,
    pub role: String,
    pub created_at: String,
    pub last_login: Option<String>,
    pub file_count: u64,
    pub total_size: u64,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct AdminUsersResponse {
    pub users: Vec<AdminUserSummary>,
}

#[utoipa::path(
    get,
    path = "/api/admin/users",
    responses(
        (status = 200, description = "List all users with file counts", body = AdminUsersResponse),
    ),
    tags = ["admin"],
)]
pub async fn admin_list_users(State(state): State<AppState>) -> Response {
    let users = state.user_store.list_users().await;
    let summaries: Vec<AdminUserSummary> = users
        .iter()
        .map(|u| AdminUserSummary {
            id: u.id.clone(),
            username: u.username.clone(),
            role: format!("{:?}", u.role),
            created_at: u.created_at.to_rfc3339(),
            last_login: u.last_login.map(|t| t.to_rfc3339()),
            file_count: 0,
            total_size: u.storage_used_bytes,
        })
        .collect();

    (StatusCode::OK, axum::Json(AdminUsersResponse { users: summaries })).into_response()
}

#[utoipa::path(
    get,
    path = "/api/admin/users/{id}",
    responses(
        (status = 200, description = "Single user details"),
        (status = 404, description = "User not found"),
    ),
    tags = ["admin"],
)]
pub async fn admin_get_user(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    match state.user_store.get_user(&id).await {
        Ok(u) => {
            let mut v = serde_json::to_value(&u).unwrap_or_default();
            if let Some(obj) = v.as_object_mut() {
                obj.remove("password_hash");
                obj.insert("file_count".to_string(), serde_json::json!(0u64));
            }
            (StatusCode::OK, axum::Json(v)).into_response()
        }
        Err(e) => match e.kind {
            ferro_server::users::UserErrorKind::NotFound => ApiError::not_found(ApiError::USER_NOT_FOUND, e.message),
            _ => ApiError::internal(ApiError::USER_ERROR, e.message),
        },
    }
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct SetUserRoleRequest {
    pub role: String,
}

#[utoipa::path(
    put,
    path = "/api/admin/users/{id}/role",
    request_body = SetUserRoleRequest,
    responses(
        (status = 200, description = "User role updated"),
        (status = 404, description = "User not found"),
        (status = 400, description = "Invalid role"),
    ),
    tags = ["admin"],
)]
pub async fn admin_set_user_role(
    State(state): State<AppState>,
    Path(id): Path<String>,
    axum::Json(body): axum::Json<SetUserRoleRequest>,
) -> Response {
    let role = match body.role.to_lowercase().as_str() {
        "admin" => users::UserRole::Admin,
        "user" => users::UserRole::User,
        "guest" | "readonly" => users::UserRole::ReadOnly,
        _ => {
            return ApiError::bad_request(
                ApiError::INVALID_INPUT,
                "Invalid role. Must be 'admin', 'user', or 'guest'",
            );
        }
    };

    match state
        .user_store
        .update_user(
            &id,
            users::UpdateUserRequest {
                role: Some(role),
                ..Default::default()
            },
        )
        .await
    {
        Ok(u) => {
            let mut v = serde_json::to_value(&u).unwrap_or_default();
            if let Some(obj) = v.as_object_mut() {
                obj.remove("password_hash");
            }
            (StatusCode::OK, axum::Json(v)).into_response()
        }
        Err(e) => match e.kind {
            ferro_server::users::UserErrorKind::NotFound => ApiError::not_found(ApiError::USER_NOT_FOUND, e.message),
            _ => ApiError::internal(ApiError::USER_ERROR, e.message),
        },
    }
}

#[utoipa::path(
    delete,
    path = "/api/admin/users/{id}",
    responses(
        (status = 200, description = "User account disabled"),
        (status = 404, description = "User not found"),
        (status = 403, description = "Cannot disable admin"),
    ),
    tags = ["admin"],
)]
pub async fn admin_delete_user(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    let user = match state.user_store.get_user(&id).await {
        Ok(u) => u,
        Err(e) => match e.kind {
            ferro_server::users::UserErrorKind::NotFound => {
                return ApiError::not_found(ApiError::USER_NOT_FOUND, e.message);
            }
            _ => return ApiError::internal(ApiError::USER_ERROR, e.message),
        },
    };

    if user.is_admin() {
        return (
            StatusCode::FORBIDDEN,
            axum::Json(serde_json::json!({
                "error": "Cannot disable admin account"
            })),
        )
            .into_response();
    }

    match state
        .user_store
        .update_user(
            &id,
            users::UpdateUserRequest {
                status: Some(users::UserStatus::Disabled),
                ..Default::default()
            },
        )
        .await
    {
        Ok(_) => {
            state
                .audit_log
                .log(build_audit_entry(
                    "DELETE",
                    &format!("/api/admin/users/{}", id),
                    "admin",
                    200,
                    None,
                    None,
                ))
                .await;
            (
                StatusCode::OK,
                axum::Json(serde_json::json!({
                    "ok": true,
                    "disabled": true
                })),
            )
                .into_response()
        }
        Err(e) => ApiError::internal(ApiError::USER_ERROR, e.message),
    }
}

// ---------------------------------------------------------------------------
// Enhanced Storage Statistics
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct AdminStorageStatsResponse {
    pub total_files: u64,
    pub total_size: u64,
    pub size_by_type: HashMap<String, u64>,
    pub top_10_largest_files: Vec<serde_json::Value>,
    pub growth_last_7_days: Vec<serde_json::Value>,
}

#[utoipa::path(
    get,
    path = "/api/admin/storage/stats",
    responses(
        (status = 200, description = "Detailed storage statistics", body = AdminStorageStatsResponse),
    ),
    tags = ["admin"],
)]
pub async fn admin_storage_stats(State(state): State<AppState>) -> Response {
    let mut total_files = 0u64;
    let mut total_size = 0u64;
    let mut size_by_type: HashMap<String, u64> = HashMap::new();
    let mut all_files: Vec<(String, u64, String)> = Vec::new();

    if let Ok(entries) = state.storage.list_all("/", 50000).await {
        for meta in &entries {
            if meta.is_collection {
                continue;
            }
            total_files += 1;
            total_size += meta.size;
            let ext = std::path::Path::new(&meta.path)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("other");
            *size_by_type.entry(ext.to_lowercase()).or_insert(0) += meta.size;
            all_files.push((meta.path.clone(), meta.size, meta.mime_type.clone()));
        }
    }

    all_files.sort_by_key(|b| std::cmp::Reverse(b.1));
    let top_10: Vec<serde_json::Value> = all_files
        .iter()
        .take(10)
        .map(|(path, size, mime)| {
            serde_json::json!({
                "path": path,
                "size": size,
                "mime_type": mime,
            })
        })
        .collect();

    let seven_days_ago = chrono::Utc::now() - chrono::Duration::days(7);
    let entries = state.audit_log.entries().await;
    let mut daily_ops: HashMap<String, i64> = HashMap::new();

    for entry in &entries {
        if let Ok(ts) = chrono::DateTime::parse_from_rfc3339(&entry.timestamp) {
            let ts_utc = ts.with_timezone(&chrono::Utc);
            if ts_utc < seven_days_ago {
                continue;
            }
            let day = ts_utc.format("%Y-%m-%d").to_string();
            let delta = match entry.method.as_str() {
                "PUT" | "POST" => 1i64,
                "DELETE" => -1i64,
                _ => continue,
            };
            *daily_ops.entry(day).or_insert(0) += delta;
        }
    }

    let mut growth: Vec<serde_json::Value> = daily_ops
        .into_iter()
        .map(|(day, ops)| {
            serde_json::json!({
                "date": day,
                "net_operations": ops,
            })
        })
        .collect();
    growth.sort_by(|a, b| a["date"].as_str().cmp(&b["date"].as_str()));

    let body = AdminStorageStatsResponse {
        total_files,
        total_size,
        size_by_type,
        top_10_largest_files: top_10,
        growth_last_7_days: growth,
    };

    (StatusCode::OK, axum::Json(body)).into_response()
}

// ---------------------------------------------------------------------------
// Audit Log Summary
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct AdminAuditSummaryResponse {
    pub by_action: HashMap<String, u64>,
    pub by_user: HashMap<String, u64>,
    pub by_day: HashMap<String, u64>,
}

#[utoipa::path(
    get,
    path = "/api/admin/audit/summary",
    responses(
        (status = 200, description = "Audit log summary", body = AdminAuditSummaryResponse),
    ),
    tags = ["admin"],
)]
pub async fn admin_audit_summary(State(state): State<AppState>) -> Response {
    let entries = state.audit_log.entries().await;
    let mut by_action: HashMap<String, u64> = HashMap::new();
    let mut by_user: HashMap<String, u64> = HashMap::new();
    let mut by_day: HashMap<String, u64> = HashMap::new();

    for entry in &entries {
        *by_action.entry(entry.method.clone()).or_insert(0) += 1;
        *by_user.entry(entry.user.clone()).or_insert(0) += 1;
        if let Ok(ts) = chrono::DateTime::parse_from_rfc3339(&entry.timestamp) {
            let day = ts.with_timezone(&chrono::Utc).format("%Y-%m-%d").to_string();
            *by_day.entry(day).or_insert(0) += 1;
        }
    }

    (
        StatusCode::OK,
        axum::Json(AdminAuditSummaryResponse {
            by_action,
            by_user,
            by_day,
        }),
    )
        .into_response()
}

#[cfg(test)]
mod tests {

    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use base64::Engine;
    use ferro_server::AppState;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    fn admin_test_app() -> axum::Router {
        let state = AppState::in_memory()
            .with_admin_user(Some("admin".to_string()))
            .with_admin_password(Some("secret".to_string()));
        ferro_server::build_router(state)
    }

    use axum::Router;

    fn flat_admin_router() -> Router {
        let state = AppState::in_memory()
            .with_admin_user(Some("admin".to_string()))
            .with_admin_password(Some("secret".to_string()));
        flat_admin_router_with_store(state)
    }

    fn flat_admin_router_with_store(state: AppState) -> Router {
        Router::new()
            .route(
                "/admin/users/:id",
                axum::routing::get(super::admin_get_user).delete(super::admin_delete_user),
            )
            .route("/admin/users/:id/role", axum::routing::put(super::admin_set_user_role))
            .with_state(state)
    }

    #[allow(dead_code)] // Test helper
    fn no_auth_test_app() -> axum::Router {
        ferro_server::build_router(AppState::in_memory())
    }

    async fn body_json(response: axum::response::Response) -> serde_json::Value {
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    async fn seed_files(app: &axum::Router, creds: &str) {
        for i in 0..5 {
            app.clone()
                .oneshot(
                    Request::builder()
                        .method("PUT")
                        .uri(format!("/test{}.txt", i))
                        .header("Authorization", format!("Basic {}", creds))
                        .body(Body::from(format!("content {}", i)))
                        .unwrap(),
                )
                .await
                .unwrap();
        }
    }

    #[tokio::test]
    async fn test_admin_stats_requires_auth() {
        let state = AppState::in_memory()
            .with_admin_user(Some("admin".to_string()))
            .with_admin_password(Some("secret".to_string()));
        let app = ferro_server::build_router(state);

        let resp = app
            .oneshot(Request::builder().uri("/api/admin/stats").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_admin_stats_reports_correct_counts() {
        let app = admin_test_app();
        let creds = base64::engine::general_purpose::STANDARD.encode("admin:secret");

        seed_files(&app, &creds).await;

        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/admin/storage")
                    .header("Authorization", format!("Basic {}", creds))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;

        assert_eq!(json["backend"], "memory");
        assert!(json.get("total_bytes").is_some());
        assert_eq!(json["file_count"], 5);
        assert!(json.get("directory_count").is_some());
        assert!(json.get("largest_file").is_some());
        assert!(json.get("recent_files").is_some());
        assert!(json["recent_files"].is_array());
    }

    #[tokio::test]
    async fn test_admin_delete_user_not_found() {
        let app = flat_admin_router();

        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/admin/users/nonexistent-id")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_admin_list_users() {
        let app = admin_test_app();
        let creds = base64::engine::general_purpose::STANDARD.encode("admin:secret");

        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/admin/users")
                    .header("Authorization", format!("Basic {}", creds))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert!(json.get("users").is_some());
        assert!(json["users"].is_array());
    }

    #[tokio::test]
    async fn test_admin_storage_stats() {
        let app = admin_test_app();
        let creds = base64::engine::general_purpose::STANDARD.encode("admin:secret");

        seed_files(&app, &creds).await;

        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/admin/storage/stats")
                    .header("Authorization", format!("Basic {}", creds))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["total_files"], 5);
        assert!(json.get("total_size").is_some());
        assert!(json.get("size_by_type").is_some());
        assert!(json["top_10_largest_files"].is_array());
        assert!(json["growth_last_7_days"].is_array());
    }

    #[tokio::test]
    async fn test_admin_audit_summary() {
        let app = admin_test_app();
        let creds = base64::engine::general_purpose::STANDARD.encode("admin:secret");

        seed_files(&app, &creds).await;

        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/admin/audit/summary")
                    .header("Authorization", format!("Basic {}", creds))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert!(json.get("by_action").is_some());
        assert!(json.get("by_user").is_some());
        assert!(json.get("by_day").is_some());
    }

    #[tokio::test]
    async fn test_admin_set_user_role_not_found() {
        let app = flat_admin_router();

        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/admin/users/nonexistent-id/role")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"role":"admin"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_admin_set_user_role_bad_role() {
        let state = AppState::in_memory()
            .with_admin_user(Some("admin".to_string()))
            .with_admin_password(Some("secret".to_string()));
        let user = ferro_server::users::User {
            id: uuid::Uuid::new_v4().to_string(),
            username: "roleuser".to_string(),
            display_name: "Role User".to_string(),
            email: "role@example.com".to_string(),
            role: ferro_server::users::UserRole::User,
            created_at: chrono::Utc::now(),
            last_login: None,
            status: ferro_server::users::UserStatus::Active,
            storage_quota_bytes: None,
            storage_used_bytes: 0,
            is_ldap: false,
            password_hash: Some(ferro_server::users::ZeroizeString::new(
                ferro_server::users::hash_password("pw").unwrap(),
            )),
            totp_secret: None,
            totp_enabled: false,
        };
        let uid = user.id.clone();
        state.user_store.create_user(user).await.unwrap();
        let app = flat_admin_router_with_store(state);

        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/admin/users/{}/role", uid))
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"role":"superadmin"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_admin_delete_user_disables() {
        let state = AppState::in_memory()
            .with_admin_user(Some("admin".to_string()))
            .with_admin_password(Some("secret".to_string()));
        let user = ferro_server::users::User {
            id: uuid::Uuid::new_v4().to_string(),
            username: "deluser".to_string(),
            display_name: "Del User".to_string(),
            email: "del@example.com".to_string(),
            role: ferro_server::users::UserRole::User,
            created_at: chrono::Utc::now(),
            last_login: None,
            status: ferro_server::users::UserStatus::Active,
            storage_quota_bytes: None,
            storage_used_bytes: 0,
            is_ldap: false,
            password_hash: Some(ferro_server::users::ZeroizeString::new(
                ferro_server::users::hash_password("pw").unwrap(),
            )),
            totp_secret: None,
            totp_enabled: false,
        };
        let uid = user.id.clone();
        state.user_store.create_user(user).await.unwrap();
        let app = flat_admin_router_with_store(state);

        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/admin/users/{}", uid))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["disabled"], true);
    }

    #[tokio::test]
    async fn test_admin_set_user_role_guest_maps_readonly() {
        let state = AppState::in_memory()
            .with_admin_user(Some("admin".to_string()))
            .with_admin_password(Some("secret".to_string()));
        let user = ferro_server::users::User {
            id: uuid::Uuid::new_v4().to_string(),
            username: "guestuser".to_string(),
            display_name: "Guest User".to_string(),
            email: "guest@example.com".to_string(),
            role: ferro_server::users::UserRole::User,
            created_at: chrono::Utc::now(),
            last_login: None,
            status: ferro_server::users::UserStatus::Active,
            storage_quota_bytes: None,
            storage_used_bytes: 0,
            is_ldap: false,
            password_hash: Some(ferro_server::users::ZeroizeString::new(
                ferro_server::users::hash_password("pw").unwrap(),
            )),
            totp_secret: None,
            totp_enabled: false,
        };
        let uid = user.id.clone();
        state.user_store.create_user(user).await.unwrap();
        let app = flat_admin_router_with_store(state);

        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/admin/users/{}/role", uid))
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"role":"guest"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["role"], "ReadOnly");
    }

    #[tokio::test]
    async fn test_admin_audit_filtered_by_action() {
        let app = admin_test_app();
        let creds = base64::engine::general_purpose::STANDARD.encode("admin:secret");

        seed_files(&app, &creds).await;

        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/admin/audit?action=PUT")
                    .header("Authorization", format!("Basic {}", creds))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert!(json["entries"].is_array());
        for entry in json["entries"].as_array().unwrap() {
            assert_eq!(entry["method"], "PUT");
        }
    }
}
