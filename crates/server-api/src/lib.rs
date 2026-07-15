use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use base64::Engine;
use common::auth::Claims;
use serde::{Deserialize, Serialize};

use ferro_server_security_middleware::api_error::ApiError;

pub use ferro_server_storage_ops::api::{
    CopyMoveResponse, FileEntryJson, ListFilesParams, ListFilesResponse, MkdirResponse, PutFileResponse,
    copy_file_impl, list_files_impl, mkdir_impl, move_file_rest_impl, normalize_api_path,
};

const READ_CACHE_FILE_SIZE_LIMIT: u64 = 10 * 1024 * 1024;

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

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct LoginParams {
    pub redirect: Option<String>,
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct CallbackParams {
    pub code: String,
    pub state: String,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

pub async fn auth_info_impl<S: ferro_server_state::ServerState>(
    state: &S,
    claims: Option<axum::Extension<Claims>>,
) -> Response {
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

pub async fn auth_login_impl<S: ferro_server_state::ServerState>(state: &S, params: LoginParams) -> Response {
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

pub async fn auth_change_password_impl<S: ferro_server_state::ServerState>(
    state: &S,
    req: axum::extract::Request,
) -> Response {
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

    if password.len() < 8 {
        return ApiError::bad_request(ApiError::WEAK_PASSWORD, "Password must be at least 8 characters");
    }
    if ferro_server_security::security::is_default_password(&password) {
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
                wipe_pending: false,
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

pub async fn auth_callback_impl<S: ferro_server_state::ServerState>(state: &S, params: CallbackParams) -> Response {
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

pub async fn auth_refresh_token_impl<S: ferro_server_state::ServerState>(
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

pub async fn get_file_impl<S: ferro_server_state::ServerState>(
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

// ── PKCE helpers ──────────────────────────────────────────────────────────

pub(crate) fn generate_code_verifier() -> String {
    use rand::Rng;
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";
    let random_bytes: Vec<u8> = (0..64)
        .map(|_| CHARS[rand::rng().random_range(0..CHARS.len())])
        .collect();
    String::from_utf8(random_bytes).unwrap_or_default()
}

pub(crate) fn generate_code_challenge(verifier: &str) -> String {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(verifier.as_bytes());
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hash)
}

pub(crate) fn urlencoding(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}

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
