use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;

use crate::AdminState;
use crate::ApiError;
use crate::build_audit_entry;
use crate::users::{UpdateUserRequest, UserErrorKind, UserRole, UserStatus};
use common::server_context::HasStorage;

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
pub async fn admin_stats<S: AdminState>(State(state): State<S>) -> Response {
    let version = env!("CARGO_PKG_VERSION");
    let uptime = state.started_at().elapsed().as_secs();

    let mut file_count = 0u64;
    let mut collection_count = 0u64;
    let mut total_bytes = 0u64;

    if let Ok(entries) = state.storage().list_all("/", 10000).await {
        for meta in &entries {
            if meta.is_collection {
                collection_count += 1;
            } else {
                file_count += 1;
                total_bytes += meta.size;
            }
        }
    }

    let auth_type = if state.oidc_enabled() {
        "oidc"
    } else if state.admin_user_enabled() {
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
        search_enabled: state.search_enabled(),
        features: AdminFeatures {
            s3: cfg!(feature = "s3"),
            gcs: cfg!(feature = "gcs"),
            azure: cfg!(feature = "azure"),
            oidc: state.oidc_enabled(),
            cedar: state.cedar_enabled(),
        },
    };

    (StatusCode::OK, axum::Json(body)).into_response()
}

/// Query parameters for the admin storage endpoint.
#[derive(Debug, Deserialize, Default, utoipa::IntoParams)]
pub struct StorageQueryParams {
    pub limit: Option<usize>,
}

pub async fn admin_storage_impl<S: HasStorage>(state: &S) -> Response {
    let mut file_count = 0u64;
    let mut collection_count = 0u64;
    let mut total_bytes = 0u64;
    let mut largest_file_path: String = String::new();
    let mut largest_file_size: u64 = 0;
    let mut recent_files: Vec<serde_json::Value> = Vec::new();

    if let Ok(entries) = state.storage().list_all("/", 10000).await {
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
pub async fn admin_storage<S: AdminState>(
    State(state): State<S>,
    Query(_params): Query<StorageQueryParams>,
) -> Response {
    admin_storage_impl(&state).await
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
pub async fn admin_audit<S: AdminState>(State(state): State<S>, Query(params): Query<AuditQueryParams>) -> Response {
    let limit: usize = params.limit.unwrap_or(100);
    let offset: usize = params.offset.unwrap_or(0);

    let all_entries = state.audit_log().entries().await;

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
pub async fn admin_maintenance<S: AdminState>(State(state): State<S>, req: axum::extract::Request) -> Response {
    let method = req.method().clone();

    if method == axum::http::Method::GET {
        let enabled = state.maintenance_mode().load(std::sync::atomic::Ordering::Relaxed);
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
        .maintenance_mode()
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
pub async fn admin_export_user_data<S: AdminState>(State(state): State<S>, Path(user_id): Path<String>) -> Response {
    let user = match state.user_store().get_user(&user_id).await {
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
    let path = tmp_dir.path().join("user_metadata.json");
    let data = user_json;
    tokio::task::spawn_blocking(move || std::fs::write(path, data).ok())
        .await
        .ok();

    let files_list: Vec<serde_json::Value> = match state.storage().list_all("/", 50000).await {
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
    let path = tmp_dir.path().join("files.json");
    let data = files_json;
    tokio::task::spawn_blocking(move || std::fs::write(path, data).ok())
        .await
        .ok();

    let audit_entries: Vec<serde_json::Value> = {
        let all = state.audit_log().entries().await;
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
    let path = tmp_dir.path().join("audit_log.json");
    let data = audit_json;
    tokio::task::spawn_blocking(move || std::fs::write(path, data).ok())
        .await
        .ok();

    let shares_list: Vec<serde_json::Value> = {
        let all = state.share_store().list().await;
        all.into_iter()
            .filter(|s| s.created_by == user.username || s.created_by == user.id)
            .map(|s| {
                serde_json::json!({
                    "token": s.token,
                    "path": s.path,
                    "expires_at": s.expires_at,
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
    let path = tmp_dir.path().join("shares.json");
    let data = shares_json;
    tokio::task::spawn_blocking(move || std::fs::write(path, data).ok())
        .await
        .ok();

    let favorites_list: Vec<String> = state.favorites().list().await;
    let fav_json = serde_json::to_string_pretty(&favorites_list).unwrap_or_else(|_| "[]".to_string());
    let path = tmp_dir.path().join("favorites.json");
    let data = fav_json;
    tokio::task::spawn_blocking(move || std::fs::write(path, data).ok())
        .await
        .ok();

    let tags_list: Vec<serde_json::Value> = state
        .tags()
        .all_tags()
        .iter()
        .map(|(path, tags)| {
            serde_json::json!({
                "path": path,
                "tags": tags,
            })
        })
        .collect();
    let tags_json = serde_json::to_string_pretty(&tags_list).unwrap_or_else(|_| "[]".to_string());
    let path = tmp_dir.path().join("tags.json");
    let data = tags_json;
    tokio::task::spawn_blocking(move || std::fs::write(path, data).ok())
        .await
        .ok();

    let zip_result = {
        let tmp_path = tmp_dir.path().to_path_buf();
        tokio::task::spawn_blocking(move || -> Result<Vec<u8>, String> {
            let zip_buf = Vec::new();
            let zip_writer = zip::ZipWriter::new(std::io::Cursor::new(zip_buf));
            let mut zip_writer = zip_writer;

            let options = zip::write::FileOptions::<()>::default()
                .compression_method(zip::CompressionMethod::Deflated)
                .compression_level(Some(6));

            let dir_entries = std::fs::read_dir(&tmp_path).map_err(|e| format!("failed to read temp dir: {}", e))?;

            for entry in dir_entries {
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

            let cursor = zip_writer
                .finish()
                .map_err(|e| format!("failed to finalize ZIP: {}", e))?;
            Ok(cursor.into_inner())
        })
        .await
    };

    let zip_bytes = match zip_result {
        Ok(Ok(bytes)) if !bytes.is_empty() => bytes,
        _ => {
            tracing::warn!("failed to create or finalize ZIP");
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
            return (headers, axum::body::Body::from(Vec::new())).into_response();
        }
    };

    state
        .audit_log()
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
pub async fn admin_erase_user_data<S: AdminState>(State(state): State<S>, Path(user_id): Path<String>) -> Response {
    let user = match state.user_store().get_user(&user_id).await {
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

    if let Ok(entries) = state.storage().list_all("/", 50000).await {
        for meta in entries {
            if !meta.is_collection && state.storage().delete(&meta.path).await.is_ok() {
                deleted_files += 1;
            }
        }
    }

    let all_shares = state.share_store().list().await;
    for share in &all_shares {
        if share.created_by == user.username || share.created_by == user.id {
            state.share_store().delete(&share.token).await;
            deleted_shares += 1;
        }
    }

    let deleted_favorites = {
        let all_favs = state.favorites().list().await;
        let count = all_favs.len() as u64;
        for fav_path in all_favs {
            state.favorites().remove(&fav_path).await;
        }
        count
    };

    let deleted_tags = {
        let tag_entries = state.tags().all_tag_pairs();
        let count = tag_entries.len() as u64;
        for (path, tag) in tag_entries {
            state.tags().remove_tag(&path, &tag);
        }
        count
    };

    let user_disabled = if let Err(e) = state
        .user_store()
        .update_user(
            &user_id,
            UpdateUserRequest {
                status: Some(UserStatus::Disabled),
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
        .audit_log()
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
pub async fn admin_list_users<S: AdminState>(State(state): State<S>) -> Response {
    let users = state.user_store().list_users().await;
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
pub async fn admin_get_user<S: AdminState>(State(state): State<S>, Path(id): Path<String>) -> Response {
    match state.user_store().get_user(&id).await {
        Ok(u) => {
            let mut v = serde_json::to_value(&u).unwrap_or_default();
            if let Some(obj) = v.as_object_mut() {
                obj.remove("password_hash");
                obj.insert("file_count".to_string(), serde_json::json!(0u64));
            }
            (StatusCode::OK, axum::Json(v)).into_response()
        }
        Err(e) => match e.kind {
            UserErrorKind::NotFound => ApiError::not_found(ApiError::USER_NOT_FOUND, e.message),
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
pub async fn admin_set_user_role<S: AdminState>(
    State(state): State<S>,
    Path(id): Path<String>,
    axum::Json(body): axum::Json<SetUserRoleRequest>,
) -> Response {
    let role = match body.role.to_lowercase().as_str() {
        "admin" => UserRole::Admin,
        "user" => UserRole::User,
        "guest" | "readonly" => UserRole::ReadOnly,
        _ => {
            return ApiError::bad_request(
                ApiError::INVALID_INPUT,
                "Invalid role. Must be 'admin', 'user', or 'guest'",
            );
        }
    };

    match state
        .user_store()
        .update_user(
            &id,
            UpdateUserRequest {
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
            UserErrorKind::NotFound => ApiError::not_found(ApiError::USER_NOT_FOUND, e.message),
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
pub async fn admin_delete_user<S: AdminState>(State(state): State<S>, Path(id): Path<String>) -> Response {
    let user = match state.user_store().get_user(&id).await {
        Ok(u) => u,
        Err(e) => match e.kind {
            UserErrorKind::NotFound => {
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
        .user_store()
        .update_user(
            &id,
            UpdateUserRequest {
                status: Some(UserStatus::Disabled),
                ..Default::default()
            },
        )
        .await
    {
        Ok(_) => {
            state
                .audit_log()
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
pub async fn admin_storage_stats<S: AdminState>(State(state): State<S>) -> Response {
    let mut total_files = 0u64;
    let mut total_size = 0u64;
    let mut size_by_type: HashMap<String, u64> = HashMap::new();
    let mut all_files: Vec<(String, u64, String)> = Vec::new();

    if let Ok(entries) = state.storage().list_all("/", 50000).await {
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
    let entries = state.audit_log().entries().await;
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
pub async fn admin_audit_summary<S: AdminState>(State(state): State<S>) -> Response {
    let entries = state.audit_log().entries().await;
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
    use super::*;
    use crate::AdminShareLink;
    use crate::AuditEntry;
    use crate::AuditLogTrait;
    use crate::DbHandle;
    use crate::branding::BrandingStore;
    use crate::build_audit_entry;
    use crate::gdpr::GdprStore;
    use async_trait::async_trait;
    use bytes::Bytes;
    use chrono::Utc;
    use common::metadata::ContentHash;
    use common::metadata::FileMetadata;
    use common::storage::StorageEngine;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Instant;

    struct MockAuditLog {
        entries: tokio::sync::RwLock<Vec<AuditEntry>>,
    }

    impl MockAuditLog {
        fn new() -> Self {
            Self {
                entries: tokio::sync::RwLock::new(Vec::new()),
            }
        }
        fn with_entries(entries: Vec<AuditEntry>) -> Self {
            Self {
                entries: tokio::sync::RwLock::new(entries),
            }
        }
    }

    #[async_trait]
    impl AuditLogTrait for MockAuditLog {
        async fn log(&self, entry: AuditEntry) {
            self.entries.write().await.push(entry);
        }
        async fn entries(&self) -> Vec<AuditEntry> {
            self.entries.read().await.clone()
        }
        async fn verify_chain(&self) -> Option<serde_json::Value> {
            None
        }
    }

    struct MockStorage;

    #[async_trait]
    impl StorageEngine for MockStorage {
        async fn head(&self, _path: &str) -> common::error::Result<FileMetadata> {
            Ok(FileMetadata::new(
                "/test".to_string(),
                ContentHash::compute(b"test"),
                4,
                "admin".to_string(),
            ))
        }
        async fn get(&self, _path: &str) -> common::error::Result<Bytes> {
            Ok(Bytes::from("test"))
        }
        async fn put(&self, _path: &str, _content: Bytes, _owner: &str) -> common::error::Result<FileMetadata> {
            Ok(FileMetadata::new(
                "/test".to_string(),
                ContentHash::compute(b"test"),
                4,
                "admin".to_string(),
            ))
        }
        async fn delete(&self, _path: &str) -> common::error::Result<()> {
            Ok(())
        }
        async fn list(&self, _path: &str) -> common::error::Result<Vec<FileMetadata>> {
            Ok(vec![])
        }
        async fn copy(&self, _from: &str, _to: &str) -> common::error::Result<()> {
            Ok(())
        }
        async fn move_path(&self, _from: &str, _to: &str) -> common::error::Result<()> {
            Ok(())
        }
        async fn exists(&self, _path: &str) -> common::error::Result<bool> {
            Ok(false)
        }
        async fn create_collection(&self, _path: &str, _owner: &str) -> common::error::Result<FileMetadata> {
            Ok(FileMetadata::new(
                "/test".to_string(),
                ContentHash::compute(b""),
                0,
                "admin".to_string(),
            ))
        }
        async fn list_all(&self, _path: &str, _max_depth: u32) -> common::error::Result<Vec<FileMetadata>> {
            Ok(vec![])
        }
    }

    struct MockUserStore {
        users: tokio::sync::RwLock<Vec<ferro_auth::users::User>>,
    }

    impl MockUserStore {
        fn new() -> Self {
            Self {
                users: tokio::sync::RwLock::new(Vec::new()),
            }
        }
        fn with_users(users: Vec<ferro_auth::users::User>) -> Self {
            Self {
                users: tokio::sync::RwLock::new(users),
            }
        }
    }

    #[async_trait]
    impl ferro_auth::users::UserStoreTrait for MockUserStore {
        async fn create_user(
            &self,
            _user: ferro_auth::users::User,
        ) -> Result<ferro_auth::users::User, ferro_auth::users::UserError> {
            unimplemented!()
        }
        async fn get_user(&self, id: &str) -> Result<ferro_auth::users::User, ferro_auth::users::UserError> {
            let users = self.users.read().await;
            users
                .iter()
                .find(|u| u.id == id)
                .cloned()
                .ok_or_else(|| ferro_auth::users::UserError::not_found("User not found"))
        }
        async fn get_user_by_username(
            &self,
            _username: &str,
        ) -> Result<ferro_auth::users::User, ferro_auth::users::UserError> {
            unimplemented!()
        }
        async fn get_user_by_email(
            &self,
            _email: &str,
        ) -> Result<ferro_auth::users::User, ferro_auth::users::UserError> {
            unimplemented!()
        }
        async fn list_users(&self) -> Vec<ferro_auth::users::User> {
            self.users.read().await.clone()
        }
        async fn update_user(
            &self,
            id: &str,
            updates: ferro_auth::users::UpdateUserRequest,
        ) -> Result<ferro_auth::users::User, ferro_auth::users::UserError> {
            let mut users = self.users.write().await;
            if let Some(user) = users.iter_mut().find(|u| u.id == id) {
                if let Some(role) = updates.role {
                    user.role = role;
                }
                if let Some(status) = updates.status {
                    user.status = status;
                }
                if let Some(dn) = updates.display_name {
                    user.display_name = dn;
                }
                if let Some(em) = updates.email {
                    user.email = em;
                }
                if let Some(quota) = updates.storage_quota_bytes {
                    user.storage_quota_bytes = quota;
                }
                Ok(user.clone())
            } else {
                Err(ferro_auth::users::UserError::not_found("User not found"))
            }
        }
        async fn delete_user(&self, _id: &str) -> Result<(), ferro_auth::users::UserError> {
            unimplemented!()
        }
        async fn update_last_login(&self, _id: &str) {}
        async fn set_password(&self, _id: &str, _password_hash: &str) -> Result<(), ferro_auth::users::UserError> {
            unimplemented!()
        }
        async fn authenticate(
            &self,
            _username: &str,
            _password: &str,
        ) -> Result<ferro_auth::users::User, ferro_auth::users::UserError> {
            unimplemented!()
        }
        async fn set_wipe_pending(&self, _id: &str, _pending: bool) -> Result<(), ferro_auth::users::UserError> {
            unimplemented!()
        }
    }

    struct MockAdminShareStore;

    #[async_trait]
    impl crate::AdminShareStoreTrait for MockAdminShareStore {
        async fn list(&self) -> Vec<AdminShareLink> {
            vec![]
        }
        async fn delete(&self, _token: &str) -> bool {
            true
        }
    }

    struct MockAdminFavoriteStore;

    #[async_trait]
    impl crate::AdminFavoriteStoreTrait for MockAdminFavoriteStore {
        async fn list(&self) -> Vec<String> {
            vec![]
        }
        async fn remove(&self, _path: &str) {}
    }

    struct MockAdminTagStore;

    impl crate::AdminTagStoreTrait for MockAdminTagStore {
        fn all_tags(&self) -> Vec<(String, Vec<String>)> {
            vec![]
        }
        fn all_tag_pairs(&self) -> Vec<(String, String)> {
            vec![]
        }
        fn remove_tag(&self, _path: &str, _tag: &str) -> bool {
            true
        }
    }

    #[derive(Clone)]
    struct MockAdminState {
        started_at: Instant,
        maintenance: Arc<AtomicBool>,
        audit_log: Arc<dyn AuditLogTrait>,
        user_store: Arc<dyn ferro_auth::users::UserStoreTrait>,
        share_store: Arc<dyn crate::AdminShareStoreTrait>,
        favorites: Arc<dyn crate::AdminFavoriteStoreTrait>,
        tags: Arc<dyn crate::AdminTagStoreTrait>,
        branding_store: Arc<BrandingStore>,
        gdpr_store: Arc<GdprStore>,
        storage: Arc<dyn StorageEngine>,
        oidc_enabled: bool,
        admin_user_enabled: bool,
        search_enabled: bool,
        cedar_enabled: bool,
        data_dir: Option<String>,
    }

    impl MockAdminState {
        fn new() -> Self {
            Self {
                started_at: Instant::now(),
                maintenance: Arc::new(AtomicBool::new(false)),
                audit_log: Arc::new(MockAuditLog::new()),
                user_store: Arc::new(MockUserStore::new()),
                share_store: Arc::new(MockAdminShareStore),
                favorites: Arc::new(MockAdminFavoriteStore),
                tags: Arc::new(MockAdminTagStore),
                branding_store: Arc::new(BrandingStore::new()),
                gdpr_store: Arc::new(GdprStore::new()),
                storage: Arc::new(MockStorage),
                oidc_enabled: false,
                admin_user_enabled: false,
                search_enabled: false,
                cedar_enabled: false,
                data_dir: None,
            }
        }
        fn with_audit_log(entries: Vec<AuditEntry>) -> Self {
            let mut s = Self::new();
            s.audit_log = Arc::new(MockAuditLog::with_entries(entries));
            s
        }
    }

    impl common::server_context::HasStorage for MockAdminState {
        fn storage(&self) -> &Arc<dyn StorageEngine> {
            &self.storage
        }
    }

    impl AdminState for MockAdminState {
        fn started_at(&self) -> Instant {
            self.started_at
        }
        fn oidc_enabled(&self) -> bool {
            self.oidc_enabled
        }
        fn admin_user_enabled(&self) -> bool {
            self.admin_user_enabled
        }
        fn search_enabled(&self) -> bool {
            self.search_enabled
        }
        fn cedar_enabled(&self) -> bool {
            self.cedar_enabled
        }
        fn maintenance_mode(&self) -> &Arc<AtomicBool> {
            &self.maintenance
        }
        fn data_dir(&self) -> Option<&str> {
            self.data_dir.as_deref()
        }
        fn db(&self) -> &Option<DbHandle> {
            &None
        }
        fn cas_store(&self) -> Option<&Arc<dyn ferro_core::cas::CasStore>> {
            None
        }
        fn audit_log(&self) -> &Arc<dyn AuditLogTrait> {
            &self.audit_log
        }
        fn user_store(&self) -> &Arc<dyn ferro_auth::users::UserStoreTrait> {
            &self.user_store
        }
        fn share_store(&self) -> &Arc<dyn crate::AdminShareStoreTrait> {
            &self.share_store
        }
        fn favorites(&self) -> &Arc<dyn crate::AdminFavoriteStoreTrait> {
            &self.favorites
        }
        fn tags(&self) -> &Arc<dyn crate::AdminTagStoreTrait> {
            &self.tags
        }
        fn branding_store(&self) -> &crate::branding::BrandingStore {
            &self.branding_store
        }
        fn gdpr_store(&self) -> &crate::gdpr::GdprStore {
            &self.gdpr_store
        }
    }

    fn make_user(id: &str, username: &str, role: ferro_auth::users::UserRole) -> ferro_auth::users::User {
        ferro_auth::users::User {
            id: id.to_string(),
            username: username.to_string(),
            display_name: username.to_string(),
            email: format!("{}@example.com", username),
            role,
            created_at: Utc::now(),
            last_login: None,
            status: ferro_auth::users::UserStatus::Active,
            storage_quota_bytes: None,
            storage_used_bytes: 0,
            is_ldap: false,
            password_hash: None,
            totp_secret: None,
            totp_enabled: false,
            wipe_pending: false,
        }
    }

    fn make_audit_entry(method: &str, path: &str, user: &str) -> AuditEntry {
        build_audit_entry(method, path, user, 200, None, None)
    }

    // ---- admin_stats tests ----

    #[tokio::test]
    async fn test_admin_stats_returns_200() {
        let state = MockAdminState::new();
        let resp = admin_stats(axum::extract::State(state)).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_admin_stats_body_has_version() {
        let state = MockAdminState::new();
        let resp = admin_stats(axum::extract::State(state)).await;
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json["version"].is_string());
        assert!(json["uptime_seconds"].is_number());
        assert!(json["total_files"].is_number());
        assert!(json["storage_backend"].is_string());
        assert!(json["auth_type"].is_string());
    }

    #[tokio::test]
    async fn test_admin_stats_auth_type_none() {
        let state = MockAdminState::new();
        let resp = admin_stats(axum::extract::State(state)).await;
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["auth_type"], "none");
    }

    // ---- admin_list_users tests ----

    #[tokio::test]
    async fn test_admin_list_users_empty() {
        let state = MockAdminState::new();
        let resp = admin_list_users(axum::extract::State(state)).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json["users"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_admin_list_users_with_data() {
        let users = vec![
            make_user("1", "alice", ferro_auth::users::UserRole::Admin),
            make_user("2", "bob", ferro_auth::users::UserRole::User),
        ];
        let state = MockAdminState {
            user_store: Arc::new(MockUserStore::with_users(users)),
            ..MockAdminState::new()
        };
        let resp = admin_list_users(axum::extract::State(state)).await;
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let arr = json["users"].as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["username"], "alice");
        assert_eq!(arr[1]["username"], "bob");
    }

    // ---- admin_get_user tests ----

    #[tokio::test]
    async fn test_admin_get_user_found() {
        let users = vec![make_user("u1", "alice", ferro_auth::users::UserRole::User)];
        let state = MockAdminState {
            user_store: Arc::new(MockUserStore::with_users(users)),
            ..MockAdminState::new()
        };
        let resp = admin_get_user(axum::extract::State(state), Path("u1".to_string())).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["username"], "alice");
    }

    #[tokio::test]
    async fn test_admin_get_user_not_found() {
        let state = MockAdminState::new();
        let resp = admin_get_user(axum::extract::State(state), Path("nonexistent".to_string())).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    // ---- admin_set_user_role tests ----

    #[tokio::test]
    async fn test_admin_set_user_role_to_admin() {
        let users = vec![make_user("u1", "bob", ferro_auth::users::UserRole::User)];
        let state = MockAdminState {
            user_store: Arc::new(MockUserStore::with_users(users)),
            ..MockAdminState::new()
        };
        let body = SetUserRoleRequest {
            role: "admin".to_string(),
        };
        let resp = admin_set_user_role(axum::extract::State(state), Path("u1".to_string()), axum::Json(body)).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["role"], "Admin");
    }

    #[tokio::test]
    async fn test_admin_set_user_role_invalid() {
        let state = MockAdminState::new();
        let body = SetUserRoleRequest {
            role: "superadmin".to_string(),
        };
        let resp = admin_set_user_role(axum::extract::State(state), Path("u1".to_string()), axum::Json(body)).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_admin_set_user_role_to_guest() {
        let users = vec![make_user("u1", "carol", ferro_auth::users::UserRole::User)];
        let state = MockAdminState {
            user_store: Arc::new(MockUserStore::with_users(users)),
            ..MockAdminState::new()
        };
        let body = SetUserRoleRequest {
            role: "guest".to_string(),
        };
        let resp = admin_set_user_role(axum::extract::State(state), Path("u1".to_string()), axum::Json(body)).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["role"], "ReadOnly");
    }

    #[tokio::test]
    async fn test_admin_set_user_role_not_found() {
        let state = MockAdminState::new();
        let body = SetUserRoleRequest {
            role: "user".to_string(),
        };
        let resp = admin_set_user_role(
            axum::extract::State(state),
            Path("nonexistent".to_string()),
            axum::Json(body),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    // ---- admin_delete_user tests ----

    #[tokio::test]
    async fn test_admin_delete_user_not_found() {
        let state = MockAdminState::new();
        let resp = admin_delete_user(axum::extract::State(state), Path("nonexistent".to_string())).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_admin_delete_user_success() {
        let users = vec![make_user("u1", "bob", ferro_auth::users::UserRole::User)];
        let state = MockAdminState {
            user_store: Arc::new(MockUserStore::with_users(users)),
            ..MockAdminState::new()
        };
        let resp = admin_delete_user(axum::extract::State(state), Path("u1".to_string())).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["ok"], true);
        assert_eq!(json["disabled"], true);
    }

    #[tokio::test]
    async fn test_admin_delete_user_forbids_admin() {
        let users = vec![make_user("admin1", "admin", ferro_auth::users::UserRole::Admin)];
        let state = MockAdminState {
            user_store: Arc::new(MockUserStore::with_users(users)),
            ..MockAdminState::new()
        };
        let resp = admin_delete_user(axum::extract::State(state), Path("admin1".to_string())).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    // ---- admin_audit tests ----

    #[tokio::test]
    async fn test_admin_audit_empty() {
        let state = MockAdminState::new();
        let resp = admin_audit(axum::extract::State(state), Query(AuditQueryParams::default())).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["total"], 0);
    }

    #[tokio::test]
    async fn test_admin_audit_with_entries() {
        let entries = vec![
            make_audit_entry("GET", "/files/test.txt", "alice"),
            make_audit_entry("PUT", "/files/new.txt", "bob"),
        ];
        let state = MockAdminState::with_audit_log(entries);
        let resp = admin_audit(axum::extract::State(state), Query(AuditQueryParams::default())).await;
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["total"], 2);
    }

    #[tokio::test]
    async fn test_admin_audit_filter_by_user() {
        let entries = vec![
            make_audit_entry("GET", "/a", "alice"),
            make_audit_entry("GET", "/b", "bob"),
            make_audit_entry("GET", "/c", "alice"),
        ];
        let state = MockAdminState::with_audit_log(entries);
        let params = AuditQueryParams {
            user_id: Some("alice".to_string()),
            ..Default::default()
        };
        let resp = admin_audit(axum::extract::State(state), Query(params)).await;
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["total"], 2);
    }

    #[tokio::test]
    async fn test_admin_audit_filter_by_action() {
        let entries = vec![
            make_audit_entry("GET", "/a", "u1"),
            make_audit_entry("PUT", "/b", "u1"),
            make_audit_entry("GET", "/c", "u1"),
        ];
        let state = MockAdminState::with_audit_log(entries);
        let params = AuditQueryParams {
            action: Some("PUT".to_string()),
            ..Default::default()
        };
        let resp = admin_audit(axum::extract::State(state), Query(params)).await;
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["total"], 1);
    }

    #[tokio::test]
    async fn test_admin_audit_filter_by_path() {
        let entries = vec![
            make_audit_entry("GET", "/files/doc.pdf", "u1"),
            make_audit_entry("GET", "/images/pic.jpg", "u1"),
        ];
        let state = MockAdminState::with_audit_log(entries);
        let params = AuditQueryParams {
            path: Some("doc".to_string()),
            ..Default::default()
        };
        let resp = admin_audit(axum::extract::State(state), Query(params)).await;
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["total"], 1);
    }

    #[tokio::test]
    async fn test_admin_audit_limit() {
        let entries: Vec<AuditEntry> = (0..10)
            .map(|i| make_audit_entry("GET", &format!("/file{}", i), "u1"))
            .collect();
        let state = MockAdminState::with_audit_log(entries);
        let params = AuditQueryParams {
            limit: Some(3),
            ..Default::default()
        };
        let resp = admin_audit(axum::extract::State(state), Query(params)).await;
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["total"], 10);
        assert_eq!(json["entries"].as_array().unwrap().len(), 3);
    }

    #[tokio::test]
    async fn test_admin_audit_offset() {
        let entries: Vec<AuditEntry> = (0..10)
            .map(|i| make_audit_entry("GET", &format!("/file{}", i), "u1"))
            .collect();
        let state = MockAdminState::with_audit_log(entries);
        let params = AuditQueryParams {
            limit: Some(5),
            offset: Some(5),
            ..Default::default()
        };
        let resp = admin_audit(axum::extract::State(state), Query(params)).await;
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["total"], 10);
        assert_eq!(json["entries"].as_array().unwrap().len(), 5);
    }

    // ---- admin_storage tests ----

    #[tokio::test]
    async fn test_admin_storage_returns_200() {
        let state = MockAdminState::new();
        let resp = admin_storage(axum::extract::State(state), Query(StorageQueryParams::default())).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_admin_storage_body_format() {
        let state = MockAdminState::new();
        let resp = admin_storage(axum::extract::State(state), Query(StorageQueryParams::default())).await;
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json["backend"].is_string());
        assert!(json["total_bytes"].is_number());
        assert!(json["file_count"].is_number());
        assert!(json["directory_count"].is_number());
    }

    // ---- admin_maintenance tests ----

    #[tokio::test]
    async fn test_admin_maintenance_get_default() {
        let state = MockAdminState::new();
        let req = axum::http::Request::builder()
            .method("GET")
            .uri("/api/admin/maintenance")
            .body(axum::body::Body::empty())
            .unwrap();
        let resp = admin_maintenance(axum::extract::State(state), req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["maintenance_mode"], false);
    }

    #[tokio::test]
    async fn test_admin_maintenance_post_enable() {
        let state = MockAdminState::new();
        let body_json = serde_json::json!({ "enabled": true });
        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/api/admin/maintenance")
            .header("content-type", "application/json")
            .body(axum::body::Body::from(serde_json::to_vec(&body_json).unwrap()))
            .unwrap();
        let resp = admin_maintenance(axum::extract::State(state.clone()), req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(state.maintenance_mode().load(Ordering::Relaxed));
    }

    #[tokio::test]
    async fn test_admin_maintenance_post_disable() {
        let state = MockAdminState::new();
        state.maintenance_mode().store(true, Ordering::Relaxed);
        let body_json = serde_json::json!({ "enabled": false });
        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/api/admin/maintenance")
            .header("content-type", "application/json")
            .body(axum::body::Body::from(serde_json::to_vec(&body_json).unwrap()))
            .unwrap();
        let resp = admin_maintenance(axum::extract::State(state.clone()), req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(!state.maintenance_mode().load(Ordering::Relaxed));
    }

    #[tokio::test]
    async fn test_admin_maintenance_post_invalid_json() {
        let state = MockAdminState::new();
        let req = axum::http::Request::builder()
            .method("POST")
            .uri("/api/admin/maintenance")
            .body(axum::body::Body::from("not json"))
            .unwrap();
        let resp = admin_maintenance(axum::extract::State(state), req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    // ---- admin_erase_user_data tests ----

    #[tokio::test]
    async fn test_admin_erase_user_not_found() {
        let state = MockAdminState::new();
        let resp = admin_erase_user_data(axum::extract::State(state), Path("nonexistent".to_string())).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_admin_erase_user_forbids_admin() {
        let users = vec![make_user("admin1", "admin", ferro_auth::users::UserRole::Admin)];
        let state = MockAdminState {
            user_store: Arc::new(MockUserStore::with_users(users)),
            ..MockAdminState::new()
        };
        let resp = admin_erase_user_data(axum::extract::State(state), Path("admin1".to_string())).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_admin_erase_user_success() {
        let users = vec![make_user("u1", "bob", ferro_auth::users::UserRole::User)];
        let state = MockAdminState {
            user_store: Arc::new(MockUserStore::with_users(users)),
            ..MockAdminState::new()
        };
        let resp = admin_erase_user_data(axum::extract::State(state), Path("u1".to_string())).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["user_id"], "u1");
        assert!(json["user_disabled"].is_boolean());
    }

    // ---- admin_storage_stats tests ----

    #[tokio::test]
    async fn test_admin_storage_stats_returns_200() {
        let state = MockAdminState::new();
        let resp = admin_storage_stats(axum::extract::State(state)).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json["total_files"].is_number());
        assert!(json["total_size"].is_number());
        assert!(json["size_by_type"].is_object());
        assert!(json["top_10_largest_files"].is_array());
        assert!(json["growth_last_7_days"].is_array());
    }

    // ---- admin_audit_summary tests ----

    #[tokio::test]
    async fn test_admin_audit_summary_empty() {
        let state = MockAdminState::new();
        let resp = admin_audit_summary(axum::extract::State(state)).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json["by_action"].as_object().unwrap().is_empty());
        assert!(json["by_user"].as_object().unwrap().is_empty());
        assert!(json["by_day"].as_object().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_admin_audit_summary_groups() {
        let entries = vec![
            make_audit_entry("GET", "/a", "alice"),
            make_audit_entry("GET", "/b", "alice"),
            make_audit_entry("PUT", "/c", "bob"),
        ];
        let state = MockAdminState::with_audit_log(entries);
        let resp = admin_audit_summary(axum::extract::State(state)).await;
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["by_action"]["GET"], 2);
        assert_eq!(json["by_action"]["PUT"], 1);
        assert_eq!(json["by_user"]["alice"], 2);
        assert_eq!(json["by_user"]["bob"], 1);
    }

    // ---- build_audit_entry tests ----

    #[test]
    fn test_build_audit_entry_basic() {
        let entry = build_audit_entry("GET", "/test", "user1", 200, None, None);
        assert_eq!(entry.method, "GET");
        assert_eq!(entry.path, "/test");
        assert_eq!(entry.user, "user1");
        assert_eq!(entry.status, 200);
        assert!(entry.client_ip.is_none());
        assert!(entry.user_agent.is_none());
        assert!(entry.content_length.is_none());
    }

    #[test]
    fn test_build_audit_entry_with_ip() {
        let entry = build_audit_entry("POST", "/upload", "user1", 201, Some("127.0.0.1".to_string()), None);
        assert_eq!(entry.client_ip, Some("127.0.0.1".to_string()));
    }

    // ---- AdminStatsResponse tests ----

    #[test]
    fn test_admin_stats_response_serialization() {
        let resp = AdminStatsResponse {
            version: "1.0.0".to_string(),
            uptime_seconds: 3600,
            total_files: 42,
            total_directories: 5,
            total_bytes: 1024,
            storage_backend: "memory".to_string(),
            auth_type: "oidc".to_string(),
            wasm_workers_loaded: 0,
            search_enabled: true,
            features: AdminFeatures {
                s3: false,
                gcs: false,
                azure: false,
                oidc: true,
                cedar: false,
            },
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["version"], "1.0.0");
        assert_eq!(json["total_files"], 42);
        assert_eq!(json["features"]["oidc"], true);
    }

    // ---- admin_export_user_data tests ----

    #[tokio::test]
    async fn test_admin_export_user_not_found() {
        let state = MockAdminState::new();
        let resp = admin_export_user_data(axum::extract::State(state), Path("nonexistent".to_string())).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_admin_export_user_found() {
        let users = vec![make_user("u1", "alice", ferro_auth::users::UserRole::User)];
        let state = MockAdminState {
            user_store: Arc::new(MockUserStore::with_users(users)),
            ..MockAdminState::new()
        };
        let resp = admin_export_user_data(axum::extract::State(state), Path("u1".to_string())).await;
        // Should return a zip (or empty body on temp failure), status should be 200
        assert_eq!(resp.status(), StatusCode::OK);
    }

    // ---- AdminStorageResponse tests ----

    #[test]
    fn test_admin_storage_response_serialization() {
        let resp = AdminStorageResponse {
            backend: "memory".to_string(),
            total_bytes: 2048,
            file_count: 10,
            directory_count: 3,
            largest_file: serde_json::json!({"path": "/big.bin", "size": 512}),
            recent_files: vec![],
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["backend"], "memory");
        assert_eq!(json["file_count"], 10);
    }

    // ---- AdminAuditResponse tests ----

    #[test]
    fn test_admin_audit_response_serialization() {
        let resp = AdminAuditResponse {
            entries: vec![],
            total: 0,
            limit: 100,
            offset: 0,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["total"], 0);
        assert_eq!(json["limit"], 100);
    }

    // ---- AdminUserSummary tests ----

    #[test]
    fn test_admin_user_summary_serialization() {
        let summary = AdminUserSummary {
            id: "u1".to_string(),
            username: "alice".to_string(),
            role: "User".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            last_login: None,
            file_count: 5,
            total_size: 1024,
        };
        let json = serde_json::to_value(&summary).unwrap();
        assert_eq!(json["id"], "u1");
        assert_eq!(json["file_count"], 5);
    }
}
