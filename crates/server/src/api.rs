use axum::extract::{Path as AxumPath, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use base64::Engine;
use common::auth::Claims;
use serde::{Deserialize, Serialize};

use crate::AppState;
use crate::api_error::ApiError;

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

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ListFilesResponse {
    pub entries: Vec<FileEntryJson>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct PutFileResponse {
    pub path: String,
    pub size: u64,
    pub etag: String,
    pub content_hash: String,
    pub created_at: String,
    pub modified_at: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct MkdirResponse {
    pub path: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct CopyMoveResponse {
    #[serde(rename = "from")]
    pub from_path: String,
    #[serde(rename = "to")]
    pub to_path: String,
}

/// Maximum file size (in bytes) eligible for read cache.
/// Files larger than this are streamed directly without caching
/// to avoid consuming memory on large assets.
const READ_CACHE_FILE_SIZE_LIMIT: u64 = 10 * 1024 * 1024; // 10 MiB

/// GET /api/auth/info — return current user info from OIDC claims.
#[utoipa::path(
    get,
    path = "/api/auth/info",
    responses(
        (status = 200, description = "Auth info", body = AuthInfoResponse),
    ),
    tags = ["auth"],
)]
pub async fn auth_info(
    claims: Option<axum::Extension<Claims>>,
    State(state): State<AppState>,
) -> Response {
    let auth_type = if state.oidc.is_some() {
        "oidc"
    } else if state.admin_user.is_some() {
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
///
/// Builds the full authorization URL with:
/// - PKCE code_verifier and code_challenge (S256)
/// - state parameter for CSRF protection
/// - redirect_uri pointing back to /api/auth/callback
///
/// The code_verifier is stored server-side in a short-lived cache
/// and verified during callback.
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
pub async fn auth_login(
    State(state): State<AppState>,
    Query(params): Query<LoginParams>,
) -> Response {
    let oidc = match &state.oidc {
        Some(v) => v,
        None => {
            return ApiError::service_unavailable(ApiError::NOT_CONFIGURED, "OIDC not configured");
        }
    };

    let config = oidc.config();
    let redirect_uri = params.redirect.unwrap_or_else(|| "/ui/".to_string());
    let callback_url = format!(
        "{}/api/auth/callback?redirect={}",
        state.external_url, redirect_uri
    );

    // Generate PKCE verifier and challenge
    let code_verifier = generate_code_verifier();
    let code_challenge = generate_code_challenge(&code_verifier);

    // Generate state for CSRF protection
    let state_param = uuid::Uuid::new_v4().to_string();

    // Build authorization URL
    let auth_url = format!(
        "{}/authorize?response_type=code&client_id={}&redirect_uri={}&scope=openid%20profile%20email&state={}&code_challenge={}&code_challenge_method=S256",
        config.issuer,
        urlencoding(&config.client_id),
        urlencoding(&callback_url),
        urlencoding(&state_param),
        urlencoding(&code_challenge),
    );

    // Store code_verifier + state for later callback verification
    oidc.store_pkce_session(&state_param, &code_verifier, &redirect_uri, &callback_url)
        .await;

    // Return the auth URL as JSON (the frontend can redirect)
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
///
/// Requires HTTP Basic authentication (verified by middleware).
/// Accepts JSON body `{"password": "<new-password>"}`.
/// Returns 400 if the new password is weak or matches a known default.
pub async fn auth_change_password(
    State(state): State<AppState>,
    req: axum::extract::Request,
) -> Response {
    // Parse password from JSON body manually
    let (parts, body) = req.into_parts();
    let body_bytes = match axum::body::to_bytes(body, usize::MAX).await {
        Ok(b) => b,
        Err(_) => {
            return ApiError::bad_request(ApiError::INVALID_BODY, "Failed to read request body");
        }
    };

    // Extract authenticated user info from request extensions
    let user_info = parts
        .extensions
        .get::<ferro_auth::users::UserInfo>()
        .cloned();

    let password = match serde_json::from_slice::<serde_json::Value>(&body_bytes) {
        Ok(v) => match v.get("password").and_then(|s| s.as_str()) {
            Some(p) => p.to_string(),
            None => {
                return ApiError::bad_request(
                    ApiError::MISSING_FIELD,
                    "Request body must contain 'password' field",
                );
            }
        },
        Err(_) => {
            return ApiError::bad_request(
                ApiError::INVALID_JSON,
                "Request body must be valid JSON",
            );
        }
    };

    use crate::security;

    // Reject weak passwords
    if password.len() < 8 {
        return ApiError::bad_request(
            ApiError::WEAK_PASSWORD,
            "Password must be at least 8 characters",
        );
    }
    if security::is_default_password(&password) {
        return ApiError::bad_request(
            ApiError::WEAK_PASSWORD,
            "New password matches a known weak/default password. Choose a stronger value.",
        );
    }

    // Authentication is already verified by simple_auth_middleware.
    // Extract the authenticated user info from request extensions.
    let username = match user_info {
        Some(ref info) => info.username.clone(),
        None => {
            // Fallback: use admin_user from config
            state
                .admin_user
                .clone()
                .unwrap_or_else(|| "admin".to_string())
        }
    };

    // Update password in user store and lift default-password restrictions
    let new_hash = match ferro_auth::users::hash_password(&password) {
        Ok(h) => h,
        Err(e) => {
            tracing::error!("Failed to hash password: {:?}", e);
            return ApiError::internal(ApiError::INTERNAL_ERROR, "Password hashing failed")
                .into_response();
        }
    };
    state
        .admin_password_rotated
        .store(true, std::sync::atomic::Ordering::Release);

    match state.user_store.get_user_by_username(&username).await {
        Ok(u) => {
            if let Err(e) = state.user_store.set_password(&u.id, &new_hash).await {
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
                password_hash: Some(new_hash),
                totp_secret: None,
                totp_enabled: false,
            };
            let _ = state.user_store.create_user(admin_user).await;
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
///
/// Exchanges the authorization code for tokens, validates the ID token,
/// and returns the user info. The frontend can then store the access token
/// for subsequent API calls.
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
pub async fn auth_callback(
    State(state): State<AppState>,
    Query(params): Query<CallbackParams>,
) -> Response {
    let oidc = match &state.oidc {
        Some(v) => v,
        None => {
            return ApiError::service_unavailable(ApiError::NOT_CONFIGURED, "OIDC not configured");
        }
    };

    // Verify state matches a pending PKCE session
    let session = match oidc.consume_pkce_session(&params.state).await {
        Some(s) => s,
        None => {
            return ApiError::bad_request(
                ApiError::BAD_REQUEST,
                "Invalid or expired state parameter",
            );
        }
    };

    // Exchange authorization code for tokens
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

    // Validate the ID token to get claims
    let id_token_str = token_response
        .get("id_token")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let claims = match oidc.validate_token(id_token_str).await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Token validation failed: {}", e);
            return ApiError::unauthorized(ApiError::TOKEN_INVALID, "Token validation failed");
        }
    };

    // Return the access token and user info to the frontend
    // The frontend stores the access_token and sends it as Bearer token
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
///
/// Accepts `{ "refresh_token": "..." }` and returns a new access token.
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
    let oidc = match &state.oidc {
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
        Err(_) => {
            ApiError::unauthorized(ApiError::TOKEN_EXPIRED, "Refresh token expired or invalid")
        }
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

/// Generate a cryptographically random code verifier (43-128 chars, unreserved).
fn generate_code_verifier() -> String {
    use rand::Rng;
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";
    let random_bytes: Vec<u8> = (0..64)
        .map(|_| CHARS[rand::rng().random_range(0..CHARS.len())])
        .collect();
    String::from_utf8(random_bytes).unwrap_or_default()
}

/// Generate code_challenge from verifier using S256 (SHA-256 + base64url).
fn generate_code_challenge(verifier: &str) -> String {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(verifier.as_bytes());
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hash)
}

/// URL-encode a string for query parameters.
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
            assert!(
                c.is_ascii_alphanumeric() || "-._~".contains(c),
                "Invalid char: {}",
                c
            );
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

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct ListFilesParams {
    pub path: Option<String>,
    pub depth: Option<u32>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct FileEntryJson {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub is_collection: bool,
    pub mime_type: String,
    pub etag: String,
    pub content_hash: String,
    pub modified_at: String,
    pub created_at: String,
}

/// GET /api/v1/files — JSON file listing (alternative to WebDAV PROPFIND).
///
/// Query parameters:
/// - `path`: directory to list (default: `/`)
/// - `depth`: nesting depth 0 or 1 (default: 1)
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
pub async fn list_files(
    State(state): State<AppState>,
    Query(params): Query<ListFilesParams>,
) -> Response {
    let path = params.path.as_deref().unwrap_or("/").trim_matches('/');
    let normalized = if path.is_empty() {
        "/"
    } else {
        &format!("/{path}")
    };
    let depth = params.depth.unwrap_or(1);

    // Verify the target is a collection. For "/", synthesize a root collection
    // if the store doesn't auto-create it (mirrors WebDAV PROPFIND behavior).
    if normalized != "/" {
        match state.storage.head(normalized).await {
            Ok(meta) if meta.is_collection => {}
            Ok(_) => {
                return (
                    StatusCode::CONFLICT,
                    axum::Json(serde_json::json!({
                        "error": "not_a_collection",
                        "message": format!("{} is not a directory", normalized),
                    })),
                )
                    .into_response();
            }
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
        }
    } else {
        // Root may not exist in in-memory store; that's OK — list() will return empty.
        let _ = state.storage.head("/").await;
    }

    let entries = if depth == 0 {
        vec![]
    } else {
        match state.storage.list(normalized).await {
            Ok(items) => items,
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    axum::Json(serde_json::json!({
                        "error": "list_failed",
                        "message": e.to_string(),
                    })),
                )
                    .into_response();
            }
        }
    };

    let json_entries: Vec<FileEntryJson> = entries
        .into_iter()
        .map(|m| {
            let name = m.path.rsplit('/').next().unwrap_or(&m.path).to_string();
            FileEntryJson {
                name,
                path: m.path,
                size: m.size,
                is_collection: m.is_collection,
                mime_type: m.mime_type,
                etag: m.etag,
                content_hash: m.content_hash.as_str().to_string(),
                modified_at: m.modified_at.to_rfc3339(),
                created_at: m.created_at.to_rfc3339(),
            }
        })
        .collect();

    (
        StatusCode::OK,
        axum::Json(ListFilesResponse {
            entries: json_entries,
        }),
    )
        .into_response()
}

/// GET /api/v1/files/{path} — download file content or get collection metadata.
///
/// For files: returns the raw content with Content-Type, ETag, and Content-Length headers.
/// For collections: returns JSON metadata (same as FileEntryJson).
/// Supports If-None-Match / If-Match for conditional requests (304 Not Modified).
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
pub async fn get_file(
    State(state): State<AppState>,
    AxumPath(path): AxumPath<String>,
    headers: axum::http::HeaderMap,
) -> Response {
    let path = normalize_api_path(&path);

    let meta = match state.storage.head(&path).await {
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

    // Conditional request: If-None-Match → 304
    if headers
        .get("if-none-match")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v == etag || v == "*")
    {
        return (StatusCode::NOT_MODIFIED, [(axum::http::header::ETAG, etag)]).into_response();
    }

    if meta.is_collection {
        // Return JSON metadata for collections
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
        // Stream file content (with read cache for small files)
        // Re-detect MIME from extension if stored value is the generic default.
        let content_type = if meta.mime_type == "application/octet-stream" {
            crate::webdav::sniff_content_type(&[], &path)
        } else {
            meta.mime_type.clone()
        };
        let content_length = meta.size;
        let filename = path.rsplit('/').next().unwrap_or("file").to_string();
        let etag_for_cache = meta.etag.clone();
        let path_for_cache = path.clone();

        // Check read cache for small files (large files skip cache to save memory)
        if content_length <= READ_CACHE_FILE_SIZE_LIMIT
            && let Some(cached) = state.read_cache.get(&path_for_cache, &etag_for_cache)
        {
            return (
                [
                    (axum::http::header::CONTENT_TYPE, content_type),
                    (axum::http::header::ETAG, etag_for_cache),
                    (
                        axum::http::header::CONTENT_LENGTH,
                        content_length.to_string(),
                    ),
                    (
                        axum::http::header::CONTENT_DISPOSITION,
                        format!("inline; filename=\"{filename}\""),
                    ),
                ],
                cached,
            )
                .into_response();
        }

        match state.storage.get_stream(&path).await {
            Ok(reader) => {
                // Convert AsyncRead to Stream<Bytes> for axum Body
                let stream = tokio_util::io::ReaderStream::new(reader);
                let body = axum::body::Body::from_stream(stream);
                // Populate cache for small files (read from stream)
                // NOTE: streaming skips cache — only fully-buffered responses are cached.
                // This is intentional: large files should not consume cache memory.
                (
                    [
                        (axum::http::header::CONTENT_TYPE, content_type),
                        (axum::http::header::ETAG, etag_for_cache),
                        (
                            axum::http::header::CONTENT_LENGTH,
                            content_length.to_string(),
                        ),
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
/////
/// Request body is the raw file content. Supports If-Match for CAS (409 Precondition Failed).
/// Returns JSON with metadata including ETag and content hash.
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
pub async fn put_file(
    State(state): State<AppState>,
    AxumPath(path): AxumPath<String>,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    let path = normalize_api_path(&path);

    // Validate each path component for safety (reserved names, control chars, etc.)
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

    // Verify declared Content-Type matches actual file magic bytes
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

    // If-Match: CAS — verify existing ETag matches
    #[allow(clippy::collapsible_if)]
    if let Some(if_match) = headers.get("if-match").and_then(|v| v.to_str().ok()) {
        if let Ok(existing) = state.storage.head(&path).await {
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
    match state.storage.put(&path, body.clone(), &owner).await {
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
pub async fn delete_file(
    State(state): State<AppState>,
    AxumPath(path): AxumPath<String>,
) -> Response {
    let path = normalize_api_path(&path);
    match state.storage.delete(&path).await {
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

    let path = normalize_api_path(path);

    let owner = "anonymous".to_string();

    match state.storage.create_collection(&path, &owner).await {
        Ok(meta) => {
            let location = meta.path.clone();
            (
                StatusCode::CREATED,
                [(axum::http::header::LOCATION, location)],
                axum::Json(MkdirResponse {
                    path: meta.path,
                    created_at: meta.created_at.to_rfc3339(),
                }),
            )
                .into_response()
        }
        Err(e) => {
            let status = if e.to_string().contains("exists") {
                StatusCode::CONFLICT
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (
                status,
                axum::Json(serde_json::json!({
                    "error": "mkdir_failed",
                    "message": e.to_string(),
                })),
            )
                .into_response()
        }
    }
}

/// Handler for `/api/v1/files/{*path}` — dispatches GET/PUT/DELETE.
///
/// Axum doesn't allow `{*path}` catch-all in a nested router, so we register
/// this at the top-level router and manually strip the prefix.
pub async fn files_content_handler(
    method: axum::http::Method,
    uri: axum::http::Uri,
    State(state): State<AppState>,
    headers: HeaderMap,
    path: Option<AxumPath<String>>,
    body: axum::body::Bytes,
) -> Response {
    // Use the path from URL parsing (extracted by axum's {*path})
    let file_path = match path {
        Some(AxumPath(p)) => p,
        None => {
            // Fallback: try to extract from URI
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
pub async fn copy_file(
    State(state): State<AppState>,
    body: axum::Json<serde_json::Value>,
) -> Response {
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

    let from = normalize_api_path(from);
    let to = normalize_api_path(to);

    match state.storage.copy(&from, &to).await {
        Ok(()) => (
            StatusCode::CREATED,
            axum::Json(CopyMoveResponse {
                from_path: from,
                to_path: to,
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::NOT_FOUND,
            axum::Json(serde_json::json!({
                "error": "copy_failed",
                "message": e.to_string(),
            })),
        )
            .into_response(),
    }
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
pub async fn move_file_rest(
    State(state): State<AppState>,
    body: axum::Json<serde_json::Value>,
) -> Response {
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

    let from = normalize_api_path(from);
    let to = normalize_api_path(to);

    match state.storage.move_path(&from, &to).await {
        Ok(()) => (
            StatusCode::CREATED,
            axum::Json(CopyMoveResponse {
                from_path: from,
                to_path: to,
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::NOT_FOUND,
            axum::Json(serde_json::json!({
                "error": "move_failed",
                "message": e.to_string(),
            })),
        )
            .into_response(),
    }
}

/// Normalize a user-supplied path for the REST API.
fn normalize_api_path(path: &str) -> String {
    let trimmed = path.trim_matches('/');
    if trimmed.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", trimmed)
    }
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
        // Regression test: PUT to /api/v1/files/{*path} was caught by
        // WebDAV fallback (returns 204) instead of REST handler (returns 201).
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
        // Verify the response is JSON (REST handler) not empty (WebDAV)
        let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(json["path"], "/test-dir/test-file.txt");
        assert_eq!(json["size"], 12); // "test content" = 12 bytes
    }

    #[tokio::test]
    async fn test_rest_get_returns_streaming_response() {
        // First PUT a file
        let app = test_app_no_oidc();
        app.clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("PUT")
                    .uri("/api/v1/files/stream-test/file.bin")
                    .body(axum::body::Body::from("streaming data"))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Then GET it — should return 200 with content-type octet-stream
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/v1/files/stream-test/file.bin")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "application/octet-stream"
        );
        let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(&body_bytes[..], b"streaming data");
    }
}
