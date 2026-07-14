use axum::extract::{Path as AxumPath, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use base64::Engine;
use common::auth::Claims;
use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::AppState;
use crate::api_error::ApiError;
use ferro_server_state::ServerState as _;

pub use ferro_server_storage_ops::api::{
    CopyMoveResponse, FileEntryJson, ListFilesParams, ListFilesResponse, MkdirResponse, PutFileResponse,
    copy_file_impl, list_files_impl, mkdir_impl, move_file_rest_impl, normalize_api_path,
};

/// Maximum file size (in bytes) eligible for read cache.
const READ_CACHE_FILE_SIZE_LIMIT: u64 = 10 * 1024 * 1024; // 10 MiB

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct AuthInfoResponse {
    pub sub: String,
    pub iss: String,
    pub aud: String,
    pub email: Option<String>,
    pub name: Option<String>,
    pub groups: Vec<String>,
    pub auth_type: String,
}

/// GET /api/auth/info — return current user info from OIDC claims.
#[utoipa::path(
    get,
    path = "/api/auth/info",
    responses(
        (status = 200, description = "Auth info", body = AuthInfoResponse),
    ),
    tags = ["auth"],
)]
#[instrument(name = "auth_info", skip(state, claims))]
pub async fn auth_info(claims: Option<axum::Extension<Claims>>, State(state): State<AppState>) -> Response {
    auth_info_impl(&state, claims).await
}

async fn auth_info_impl<S: ferro_server_state::ServerState>(state: &S, claims: Option<axum::Extension<Claims>>) -> Response {
    let auth_type = if state.oidc().is_some() {
        "oidc"
    } else if state.admin_user().is_some() {
        "basic"
    } else {
        "none"
    };

    let body = match &claims {
        Some(c) => AuthInfoResponse {
            sub: c.sub.clone(),
            iss: c.iss.clone(),
            aud: c.aud.clone(),
            email: c.email.clone(),
            name: c.name.clone(),
            groups: c.groups.clone().unwrap_or_default(),
            auth_type: auth_type.to_string(),
        },
        None => AuthInfoResponse {
            sub: "anonymous".to_string(),
            iss: "ferro".to_string(),
            aud: "ferro".to_string(),
            email: None,
            name: None,
            groups: Vec::new(),
            auth_type: auth_type.to_string(),
        },
    };
    (StatusCode::OK, axum::Json(body)).into_response()
}

/// GET /api/auth/login — redirect to OIDC provider with PKCE.
#[utoipa::path(
    get,
    path = "/api/auth/login",
    params(LoginParams),
    responses(
        (status = 200, description = "Authorization URL for OIDC redirect"),
        (status = 503, description = "OIDC not configured", body = ApiError),
    ),
    tags = ["auth"],
)]
pub async fn auth_login(State(state): State<AppState>, Query(params): Query<LoginParams>) -> Response {
    auth_login_impl(&state, params).await
}

async fn auth_login_impl<S: ferro_server_state::ServerState>(state: &S, params: LoginParams) -> Response {
    let oidc = match state.oidc() {
        Some(v) => v,
        None => {
            return ApiError::service_unavailable(ApiError::NOT_CONFIGURED, "OIDC not configured");
        }
    };

    let config = oidc.config();
    let redirect_uri = params.redirect.unwrap_or_else(|| "/ui/".to_string());
    let callback_url = format!("{}/api/auth/callback?redirect={}", state.external_url(), redirect_uri);

    let code_verifier = generate_code_verifier();
    let code_challenge = generate_code_challenge(&code_verifier);
    let state_param = uuid::Uuid::new_v4().to_string();

    let auth_url = format!(
        "{}/authorize?response_type=code&client_id={}&redirect_uri={}&scope=openid%20profile%20email&state={}&code_challenge={}&code_challenge_method=S256",
        config.issuer,
        urlencoding(&config.client_id),
        urlencoding(&callback_url),
        urlencoding(&state_param),
        urlencoding(&code_challenge),
    );

    oidc.store_pkce_session(&state_param, &code_verifier, &redirect_uri, &callback_url)
        .await;

    (
        StatusCode::OK,
        axum::Json(serde_json::json!({
            "authorization_url": auth_url,
            "state": state_param,
        })),
    )
        .into_response()
}

/// POST /api/auth/change-password — change admin password.
pub async fn auth_change_password(State(state): State<AppState>, req: axum::extract::Request) -> Response {
    auth_change_password_impl(&state, req).await
}

async fn auth_change_password_impl<S: ferro_server_state::ServerState>(state: &S, req: axum::extract::Request) -> Response {
    let (parts, body) = req.into_parts();
    let body_bytes = match axum::body::to_bytes(body, 1024 * 1024).await {
        Ok(b) => b,
        Err(_) => {
            return ApiError::bad_request(ApiError::INVALID_BODY, "Failed to read request body");
        }
    };

    if body_bytes.len() > 1024 * 1024 {
        return ApiError::bad_request(ApiError::INVALID_BODY, "Request body too large");
    }

    let user_info = parts.extensions.get::<ferro_auth::users::UserInfo>().cloned();

    let password = match serde_json::from_slice::<serde_json::Value>(&body_bytes) {
        Ok(v) => match v.get("password").and_then(|s| s.as_str()) {
            Some(p) => p.to_string(),
            None => {
                return ApiError::bad_request(ApiError::MISSING_FIELD, "Request body must contain 'password' field");
            }
        },
        Err(_) => {
            return ApiError::bad_request(ApiError::INVALID_JSON, "Request body must be valid JSON");
        }
    };

    use crate::security;

    if password.len() < 8 {
        return ApiError::bad_request(ApiError::WEAK_PASSWORD, "Password must be at least 8 characters");
    }
    if security::is_default_password(&password) {
        return ApiError::bad_request(
            ApiError::WEAK_PASSWORD,
            "New password matches a known weak/default password. Choose a stronger value.",
        );
    }

    let username = match user_info {
        Some(ref info) => info.username.clone(),
        None => state.admin_user().unwrap_or("admin").to_string(),
    };

    let new_hash = match ferro_auth::users::hash_password(&password) {
        Ok(h) => h,
        Err(e) => {
            tracing::error!("Failed to hash password: {:?}", e);
            return ApiError::internal(ApiError::INTERNAL_ERROR, "Password hashing failed").into_response();
        }
    };
    state
        .admin_password_rotated()
        .store(true, std::sync::atomic::Ordering::Release);

    match state.user_store().get_user_by_username(&username).await {
        Ok(u) => {
            if let Err(e) = state.user_store().set_password(&u.id, &new_hash).await {
                tracing::error!("Failed to update password in user store: {:?}", e);
            }
        }
        Err(_) => {
            let admin_user = ferro_auth::users::User {
                id: uuid::Uuid::new_v4().to_string(),
                username: username.clone(),
                display_name: "Administrator".to_string(),
                email: String::new(),
                role: ferro_auth::users::UserRole::Admin,
                created_at: chrono::Utc::now(),
                last_login: None,
                status: ferro_auth::users::UserStatus::Active,
                storage_quota_bytes: None,
                storage_used_bytes: 0,
                is_ldap: false,
                password_hash: Some(ferro_auth::users::ZeroizeString::new(new_hash)),
                totp_secret: None,
                totp_enabled: false,
            };
            if let Err(e) = state.user_store().create_user(admin_user).await {
                tracing::error!(error = ?e, "Failed to create admin user after password change");
            }
        }
    }

    tracing::info!("Admin password changed for user '{}'", username);

    (
        StatusCode::OK,
        axum::Json(serde_json::json!({
            "status": "ok",
            "message": "Password changed successfully. Default password restrictions lifted."
        })),
    )
        .into_response()
}

/// GET /api/auth/callback — handle OIDC callback.
#[utoipa::path(
    get,
    path = "/api/auth/callback",
    params(CallbackParams),
    responses(
        (status = 200, description = "Token exchange result"),
        (status = 503, description = "OIDC not configured", body = ApiError),
    ),
    tags = ["auth"],
)]
pub async fn auth_callback(State(state): State<AppState>, Query(params): Query<CallbackParams>) -> Response {
    auth_callback_impl(&state, params).await
}

async fn auth_callback_impl<S: ferro_server_state::ServerState>(state: &S, params: CallbackParams) -> Response {
    let oidc = match state.oidc() {
        Some(v) => v,
        None => {
            return ApiError::service_unavailable(ApiError::NOT_CONFIGURED, "OIDC not configured");
        }
    };

    let session = match oidc.consume_pkce_session(&params.state).await {
        Some(s) => s,
        None => {
            return ApiError::bad_request(ApiError::BAD_REQUEST, "Invalid or expired state parameter");
        }
    };

    let token_response = match oidc
        .exchange_code(&params.code, &session.code_verifier, &session.callback_url)
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("Token exchange failed: {}", e);
            return ApiError::with_details(
                StatusCode::BAD_GATEWAY,
                ApiError::TOKEN_INVALID,
                "Token exchange failed",
                e.to_string(),
            );
        }
    };

    let id_token_str = token_response.get("id_token").and_then(|v| v.as_str()).unwrap_or("");
    let claims = match oidc.validate_token(id_token_str).await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Token validation failed: {}", e);
            return ApiError::unauthorized(ApiError::TOKEN_INVALID, "Token validation failed");
        }
    };

    let access_token = token_response
        .get("access_token")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let token_type = token_response
        .get("token_type")
        .and_then(|v| v.as_str())
        .unwrap_or("Bearer")
        .to_string();
    let expires_in = token_response
        .get("expires_in")
        .and_then(|v| v.as_u64())
        .unwrap_or(3600);
    (
        StatusCode::OK,
        axum::Json(serde_json::json!({
            "access_token": access_token,
            "token_type": token_type,
            "expires_in": expires_in,
            "user": {
                "sub": claims.sub,
                "email": claims.email,
                "name": claims.name,
            },
            "redirect": session.redirect_uri,
        })),
    )
        .into_response()
}

/// POST /api/auth/refresh — exchange a refresh token for a new access token.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

#[utoipa::path(
    post,
    path = "/api/auth/refresh",
    request_body = RefreshTokenRequest,
    responses(
        (status = 200, description = "New access token"),
        (status = 401, description = "Invalid or expired refresh token"),
        (status = 503, description = "OIDC not configured"),
    ),
    tags = ["auth"],
)]
pub async fn auth_refresh_token(
    State(state): State<AppState>,
    axum::Json(body): axum::Json<RefreshTokenRequest>,
) -> Response {
    auth_refresh_token_impl(&state, body).await
}

async fn auth_refresh_token_impl<S: ferro_server_state::ServerState>(
    state: &S,
    body: RefreshTokenRequest,
) -> Response {
    let oidc = match state.oidc() {
        Some(v) => v,
        None => {
            return ApiError::service_unavailable(ApiError::NOT_CONFIGURED, "OIDC not configured");
        }
    };

    if body.refresh_token.is_empty() {
        return ApiError::bad_request(ApiError::BAD_REQUEST, "refresh_token is required");
    }

    match oidc.refresh_access_token(&body.refresh_token).await {
        Ok(token_response) => {
            let access_token = token_response
                .get("access_token")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let token_type = token_response
                .get("token_type")
                .and_then(|v| v.as_str())
                .unwrap_or("Bearer")
                .to_string();
            let expires_in = token_response
                .get("expires_in")
                .and_then(|v| v.as_u64())
                .unwrap_or(3600);
            let new_refresh_token = token_response
                .get("refresh_token")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let mut response_body = serde_json::json!({
                "access_token": access_token,
                "token_type": token_type,
                "expires_in": expires_in,
            });
            if let Some(rt) = new_refresh_token {
                response_body["refresh_token"] = serde_json::Value::String(rt);
            }

            (StatusCode::OK, axum::Json(response_body)).into_response()
        }
        Err(_) => ApiError::unauthorized(ApiError::TOKEN_EXPIRED, "Refresh token expired or invalid"),
    }
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct LoginParams {
    pub redirect: Option<String>,
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct CallbackParams {
    pub code: String,
    pub state: String,
}

// ── PKCE helpers ──────────────────────────────────────────────────────────

fn generate_code_verifier() -> String {
    use rand::Rng;
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";
    let random_bytes: Vec<u8> = (0..64)
        .map(|_| CHARS[rand::rng().random_range(0..CHARS.len())])
        .collect();
    String::from_utf8(random_bytes).unwrap_or_default()
}

fn generate_code_challenge(verifier: &str) -> String {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(verifier.as_bytes());
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hash)
}

fn urlencoding(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_code_verifier_length() {
        let verifier = generate_code_verifier();
        assert!(
            verifier.len() >= 43,
            "Verifier must be at least 43 chars, got {}",
            verifier.len()
        );
        assert!(verifier.len() <= 128);
        for c in verifier.chars() {
            assert!(c.is_ascii_alphanumeric() || "-._~".contains(c), "Invalid char: {}", c);
        }
    }

    #[test]
    fn test_code_challenge_deterministic() {
        let verifier = "test-verifier-123";
        let challenge = generate_code_challenge(verifier);
        assert!(!challenge.contains('+'));
        assert!(!challenge.contains('/'));
        assert!(!challenge.contains('='));
    }

    #[test]
    fn test_code_challenge_matches_known_value() {
        let challenge = generate_code_challenge("test");
        assert_eq!(challenge, "n4bQgYhMfWWaL-qgxVrQFaO_TxsrC4Is0V1sFbDwCgg");
    }
}

// ── File listing (REST) ─────────────────────────────────────────────────

/// GET /api/v1/files — JSON file listing (alternative to WebDAV PROPFIND).
#[utoipa::path(
    get,
    path = "/api/v1/files",
    params(ListFilesParams),
    responses(
        (status = 200, description = "File listing", body = ListFilesResponse),
        (status = 409, description = "Not a collection", body = ApiError),
        (status = 404, description = "Path not found", body = ApiError),
        (status = 500, description = "List failed", body = ApiError),
    ),
    tags = ["files"],
)]
#[instrument(name = "list_files", skip(state, params))]
pub async fn list_files(State(state): State<AppState>, Query(params): Query<ListFilesParams>) -> Response {
    list_files_impl(&state, &params).await
}

/// GET /api/v1/files/{path} — download file content or get collection metadata.
#[utoipa::path(
    get,
    path = "/api/v1/files/{path}",
    responses(
        (status = 200, description = "File content or collection metadata", body = FileEntryJson),
        (status = 304, description = "Not modified"),
        (status = 404, description = "Not found", body = ApiError),
    ),
    tags = ["files"],
)]
#[instrument(name = "get_file", skip(state, headers), fields(path = %path))]
pub async fn get_file(
    State(state): State<AppState>,
    AxumPath(path): AxumPath<String>,
    headers: axum::http::HeaderMap,
) -> Response {
    get_file_impl(&state, path, headers).await
}

async fn get_file_impl<S: ferro_server_state::ServerState>(
    state: &S,
    path: String,
    headers: axum::http::HeaderMap,
) -> Response {
    let path = match normalize_api_path(&path) {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                axum::Json(serde_json::json!({
                    "error": "invalid_path", "message": e,
                })),
            )
                .into_response();
        }
    };

    let meta = match state.storage().head(&path).await {
        Ok(m) => m,
        Err(e) => {
            return (
                StatusCode::NOT_FOUND,
                axum::Json(serde_json::json!({
                    "error": "not_found",
                    "message": e.to_string(),
                })),
            )
                .into_response();
        }
    };

    let etag = meta.etag.clone();

    if headers
        .get("if-none-match")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v == etag || v == "*")
    {
        return (StatusCode::NOT_MODIFIED, [(axum::http::header::ETAG, etag)]).into_response();
    }

    if meta.is_collection {
        let entry = FileEntryJson {
            name: path.rsplit('/').next().unwrap_or(&path).to_string(),
            path: meta.path,
            size: meta.size,
            is_collection: true,
            mime_type: meta.mime_type,
            etag: meta.etag,
            content_hash: meta.content_hash.as_str().to_string(),
            modified_at: meta.modified_at.to_rfc3339(),
            created_at: meta.created_at.to_rfc3339(),
        };
        (StatusCode::OK, axum::Json(serde_json::json!(entry))).into_response()
    } else {
        let content_type = if meta.mime_type == "application/octet-stream" {
            common::mime::sniff_content_type(&[], &path)
        } else {
            meta.mime_type.clone()
        };
        let content_length = meta.size;
        let filename = path.rsplit('/').next().unwrap_or("file").to_string();
        let etag_for_cache = meta.etag.clone();
        let path_for_cache = path.clone();

        if content_length <= READ_CACHE_FILE_SIZE_LIMIT
            && let Some(cached) = state.read_cache().get(&path_for_cache, &etag_for_cache)
        {
            return (
                [
                    (axum::http::header::CONTENT_TYPE, content_type),
                    (axum::http::header::ETAG, etag_for_cache),
                    (axum::http::header::CONTENT_LENGTH, content_length.to_string()),
                    (
                        axum::http::header::CONTENT_DISPOSITION,
                        format!("inline; filename=\"{filename}\""),
                    ),
                ],
                cached,
            )
                .into_response();
        }

        match state.storage().get_stream(&path).await {
            Ok(reader) => {
                let stream = tokio_util::io::ReaderStream::new(reader);
                let body = axum::body::Body::from_stream(stream);
                (
                    [
                        (axum::http::header::CONTENT_TYPE, content_type),
                        (axum::http::header::ETAG, etag_for_cache),
                        (axum::http::header::CONTENT_LENGTH, content_length.to_string()),
                        (
                            axum::http::header::CONTENT_DISPOSITION,
                            format!("inline; filename=\"{filename}\""),
                        ),
                    ],
                    body,
                )
                    .into_response()
            }
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(serde_json::json!({
                    "error": "read_failed",
                    "message": e.to_string(),
                })),
            )
                .into_response(),
        }
    }
}

/// PUT /api/v1/files/{path} — upload/replace file content.
#[utoipa::path(
    put,
    path = "/api/v1/files/{path}",
    request_body(content = [u8], description = "Raw file content (binary)"),
    responses(
        (status = 201, description = "File created/updated", body = PutFileResponse),
        (status = 409, description = "Precondition failed", body = ApiError),
        (status = 500, description = "Upload failed", body = ApiError),
    ),
    tags = ["files"],
)]
#[instrument(name = "put_file", skip(state, headers, body), fields(path = %path))]
pub async fn put_file(
    State(state): State<AppState>,
    AxumPath(path): AxumPath<String>,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    let path = match normalize_api_path(&path) {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                axum::Json(serde_json::json!({
                    "error": "invalid_path", "message": e,
                })),
            )
                .into_response();
        }
    };

    if let Err(reason) = crate::security::validate_path(&path) {
        return (
            StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({
                "error": "invalid_path",
                "message": reason,
            })),
        )
            .into_response();
    }

    if let Some(declared) = headers.get("content-type").and_then(|v| v.to_str().ok())
        && let Some(detected) = crate::security::verify_content_type(declared, &body)
    {
        return (
            StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({
                "error": "content_type_mismatch",
                "message": format!(
                    "Declared Content-Type '{}' does not match detected type '{}'",
                    declared, detected
                ),
            })),
        )
            .into_response();
    }

    #[allow(clippy::collapsible_if)]
    if let Some(if_match) = headers.get("if-match").and_then(|v| v.to_str().ok()) {
        if let Ok(existing) = state.storage().head(&path).await {
            if if_match != existing.etag && if_match != "*" {
                return (
                    StatusCode::PRECONDITION_FAILED,
                    axum::Json(serde_json::json!({
                        "error": "precondition_failed",
                        "message": "ETag does not match",
                        "current_etag": existing.etag,
                    })),
                )
                    .into_response();
            }
        }
    }

    let owner = "anonymous".to_string();
    match state.storage().put(&path, body.clone(), &owner).await {
        Ok(meta) => {
            let etag = meta.etag.clone();
            let size = meta.size;
            let mime_type = meta.mime_type.clone();
            crate::events::dispatch_post_op(
                &state,
                crate::events::FileEvent {
                    op_type: "put",
                    path: path.clone(),
                    new_path: None,
                    size: Some(size),
                    mime_type: Some(mime_type),
                    owner: owner.clone(),
                    etag: Some(etag.clone()),
                    already_existed: false,
                },
            )
            .await;
            (
                StatusCode::CREATED,
                [
                    (axum::http::header::ETAG, etag.clone()),
                    (axum::http::header::LOCATION, path.clone()),
                ],
                axum::Json(PutFileResponse {
                    path: meta.path,
                    size,
                    etag,
                    content_hash: meta.content_hash.as_str().to_string(),
                    created_at: meta.created_at.to_rfc3339(),
                    modified_at: meta.modified_at.to_rfc3339(),
                }),
            )
                .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(serde_json::json!({
                "error": "upload_failed",
                "message": e.to_string(),
            })),
        )
            .into_response(),
    }
}

/// DELETE /api/v1/files/{path} — delete a file or collection.
#[utoipa::path(
    delete,
    path = "/api/v1/files/{path}",
    responses(
        (status = 204, description = "File deleted"),
        (status = 404, description = "Not found", body = ApiError),
    ),
    tags = ["files"],
)]
#[instrument(name = "delete_file", skip(state), fields(path = %path))]
pub async fn delete_file(State(state): State<AppState>, AxumPath(path): AxumPath<String>) -> Response {
    let path = match normalize_api_path(&path) {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                axum::Json(serde_json::json!({
                    "error": "invalid_path", "message": e,
                })),
            )
                .into_response();
        }
    };
    match state.storage().delete(&path).await {
        Ok(()) => {
            crate::events::dispatch_post_op(
                &state,
                crate::events::FileEvent {
                    op_type: "delete",
                    path: path.clone(),
                    new_path: None,
                    size: None,
                    mime_type: None,
                    owner: "anonymous".to_string(),
                    etag: None,
                    already_existed: true,
                },
            )
            .await;
            (StatusCode::NO_CONTENT, "").into_response()
        }
        Err(e) => (
            StatusCode::NOT_FOUND,
            axum::Json(serde_json::json!({
                "error": "delete_failed",
                "message": e.to_string(),
            })),
        )
            .into_response(),
    }
}

/// POST /api/v1/files/mkdir — create a directory/collection.
#[utoipa::path(
    post,
    path = "/api/v1/files/mkdir",
    request_body(content = serde_json::Value, description = "JSON with 'path' field"),
    responses(
        (status = 201, description = "Directory created", body = MkdirResponse),
        (status = 409, description = "Already exists", body = ApiError),
        (status = 500, description = "Mkdir failed", body = ApiError),
    ),
    tags = ["files"],
)]
pub async fn mkdir(State(state): State<AppState>, body: axum::Json<serde_json::Value>) -> Response {
    let path = body.get("path").and_then(|v| v.as_str()).unwrap_or("/");
    mkdir_impl(&state, path).await
}

/// Handler for `/api/v1/files/{*path}` — dispatches GET/PUT/DELETE.
pub async fn files_content_handler(
    method: axum::http::Method,
    uri: axum::http::Uri,
    State(state): State<AppState>,
    headers: HeaderMap,
    path: Option<AxumPath<String>>,
    body: axum::body::Bytes,
) -> Response {
    let file_path = match path {
        Some(AxumPath(p)) => p,
        None => {
            let path_str = uri.path();
            match path_str
                .strip_prefix("/api/v1/files/")
                .or_else(|| path_str.strip_prefix("/api/files/"))
            {
                Some(p) if !p.is_empty() => p.to_string(),
                _ => {
                    return (
                        StatusCode::NOT_FOUND,
                        axum::Json(serde_json::json!({
                            "error": "not_found",
                            "message": "Unknown API endpoint",
                        })),
                    )
                        .into_response();
                }
            }
        }
    };

    match method {
        axum::http::Method::GET => get_file(State(state), AxumPath(file_path), headers).await,
        axum::http::Method::PUT => put_file(State(state), AxumPath(file_path), headers, body).await,
        axum::http::Method::DELETE => delete_file(State(state), AxumPath(file_path)).await,
        _ => (
            StatusCode::METHOD_NOT_ALLOWED,
            axum::Json(serde_json::json!({
                "error": "method_not_allowed",
                "message": "Only GET, PUT, and DELETE are supported for file operations",
            })),
        )
            .into_response(),
    }
}

/// POST /api/v1/files/copy — copy a file or directory.
#[utoipa::path(
    post,
    path = "/api/v1/files/copy",
    request_body(content = serde_json::Value, description = "JSON with 'from' and 'to' fields"),
    responses(
        (status = 201, description = "File copied", body = CopyMoveResponse),
        (status = 400, description = "Missing parameters", body = ApiError),
        (status = 404, description = "Copy failed", body = ApiError),
    ),
    tags = ["files"],
)]
pub async fn copy_file(State(state): State<AppState>, body: axum::Json<serde_json::Value>) -> Response {
    let from = body.get("from").and_then(|v| v.as_str()).unwrap_or("");
    let to = body.get("to").and_then(|v| v.as_str()).unwrap_or("");

    if from.is_empty() || to.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({
                "error": "missing_params",
                "message": "Both 'from' and 'to' are required",
            })),
        )
            .into_response();
    }

    let from = match normalize_api_path(from) {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                axum::Json(serde_json::json!({
                    "error": "invalid_path", "message": e,
                })),
            )
                .into_response();
        }
    };
    let to = match normalize_api_path(to) {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                axum::Json(serde_json::json!({
                    "error": "invalid_path", "message": e,
                })),
            )
                .into_response();
        }
    };

    copy_file_impl(&state, &from, &to).await
}

/// POST /api/v1/files/move — move/rename a file or directory.
#[utoipa::path(
    post,
    path = "/api/v1/files/move",
    request_body(content = serde_json::Value, description = "JSON with 'from' and 'to' fields"),
    responses(
        (status = 201, description = "File moved", body = CopyMoveResponse),
        (status = 400, description = "Missing parameters", body = ApiError),
        (status = 404, description = "Move failed", body = ApiError),
    ),
    tags = ["files"],
)]
pub async fn move_file_rest(State(state): State<AppState>, body: axum::Json<serde_json::Value>) -> Response {
    let from = body.get("from").and_then(|v| v.as_str()).unwrap_or("");
    let to = body.get("to").and_then(|v| v.as_str()).unwrap_or("");

    if from.is_empty() || to.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({
                "error": "missing_params",
                "message": "Both 'from' and 'to' are required",
            })),
        )
            .into_response();
    }

    let from = match normalize_api_path(from) {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                axum::Json(serde_json::json!({
                    "error": "invalid_path", "message": e,
                })),
            )
                .into_response();
        }
    };
    let to = match normalize_api_path(to) {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                axum::Json(serde_json::json!({
                    "error": "invalid_path", "message": e,
                })),
            )
                .into_response();
        }
    };

    move_file_rest_impl(&state, &from, &to).await
}

#[cfg(test)]
mod auth_tests {
    use super::*;
    use crate::AppState;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    fn test_app_no_oidc() -> axum::Router {
        crate::build_router(AppState::in_memory())
    }

    async fn body_json(response: axum::response::Response) -> serde_json::Value {
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn test_auth_login_without_oidc_returns_503() {
        let app = test_app_no_oidc();
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/auth/login")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        let json = body_json(response).await;
        assert_eq!(json["error"], "OIDC not configured");
    }

    #[tokio::test]
    async fn test_auth_callback_without_oidc_returns_503() {
        let app = test_app_no_oidc();
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/auth/callback?code=test&state=invalid")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        let json = body_json(response).await;
        assert_eq!(json["error"], "OIDC not configured");
    }

    #[tokio::test]
    async fn test_auth_info_returns_anonymous() {
        let app = test_app_no_oidc();
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/auth/info")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert_eq!(json["sub"], "anonymous");
        assert_eq!(json["iss"], "ferro");
        assert_eq!(json["aud"], "ferro");
    }

    #[tokio::test]
    async fn test_api_config_all_fields_present() {
        let app = test_app_no_oidc();
        let resp = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/config")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        let expected_fields = [
            "version",
            "auth_enabled",
            "search_enabled",
            "wasm_enabled",
            "wasm_workers_enabled",
            "cedar_enabled",
            "metadata_persistent",
            "cas_enabled",
            "storage",
            "external_url",
            "wopi_configured",
        ];
        for field in &expected_fields {
            assert!(json.get(*field).is_some(), "Missing field: {}", field);
        }
    }

    #[tokio::test]
    async fn test_health_check_format() {
        let app = test_app_no_oidc();
        let resp = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/.well-known/ferro")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["status"], "ok");
        assert!(json.get("version").is_some());
        assert!(json.get("uptime_seconds").is_some());
        assert!(json.get("subsystems").is_some());
        assert!(json["subsystems"].is_object());
        assert!(json["subsystems"].get("storage").is_some());
        assert!(json["subsystems"].get("auth").is_some());
        assert!(json["subsystems"].get("search").is_some());
        assert!(json["subsystems"].get("wasm").is_some());
        assert!(json["subsystems"].get("metadata").is_some());
        assert!(json["subsystems"].get("cas").is_some());
    }

    #[tokio::test]
    async fn test_metrics_endpoint_format() {
        let app = test_app_no_oidc();
        let resp = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/metrics")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert!(json.get("uptime_seconds").is_some());
        assert!(json.get("storage").is_some());
        assert!(json["storage"].is_object());
        assert!(json["storage"].get("files").is_some());
        assert!(json["storage"].get("total_bytes").is_some());
        assert!(json.get("requests").is_some());
        assert!(json["requests"].is_object());
    }

    #[tokio::test]
    async fn test_security_headers_present() {
        let app = test_app_no_oidc();
        let resp = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/config")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let headers = resp.headers();
        assert!(
            headers.get("X-Content-Type-Options").is_some(),
            "Missing X-Content-Type-Options header"
        );
        assert!(
            headers.get("X-Frame-Options").is_some(),
            "Missing X-Frame-Options header"
        );
        assert!(
            headers.get("Referrer-Policy").is_some(),
            "Missing Referrer-Policy header"
        );
    }

    #[tokio::test]
    async fn test_rest_put_returns_201_not_204() {
        let app = test_app_no_oidc();
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("PUT")
                    .uri("/api/v1/files/test-dir/test-file.txt")
                    .header("content-type", "text/plain")
                    .body(axum::body::Body::from("test content"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(json["path"], "/test-dir/test-file.txt");
        assert_eq!(json["size"], 12);
    }
}
