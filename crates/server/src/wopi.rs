use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use base64::Engine;
use hmac::Mac;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use tracing::info;

use crate::api_error::ApiError;
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct WopiQueryParams {
    pub access_token: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct WopiCheckFileInfoResponse {
    pub base_file_name: String,
    pub size: u64,
    pub version: String,
    pub last_modified_time: String,
    pub owner_id: String,
    pub user_can_write: bool,
    pub user_can_not_write_relative: bool,
    pub supports_update: bool,
    pub supports_locks: bool,
    pub supports_coauth: bool,
}

#[derive(Debug, Serialize)]
pub struct WopiLockResponse {
    pub lock_id: String,
    pub lock: String,
    pub expires: String,
    pub user: String,
}

fn split_contents_suffix(path: &str) -> Option<(&str, &str)> {
    let idx = path.rfind("/contents")?;
    if idx + 9 == path.len() {
        return Some((&path[..idx], "contents"));
    }
    None
}

/// Validate the WOPI access_token from query parameters.
/// Tokens are HMAC-SHA256 signed and contain an expiry timestamp.
#[allow(clippy::result_large_err)]
fn validate_access_token(state: &AppState, token: &Option<String>) -> Result<String, Response> {
    let t = match token {
        None => return Err(ApiError::unauthorized(ApiError::TOKEN_INVALID, "Missing access_token parameter")),
        Some(t) if t.is_empty() => return Err(ApiError::unauthorized(ApiError::TOKEN_INVALID, "Empty access_token")),
        Some(t) => t,
    };

    let decoded = match base64::engine::general_purpose::STANDARD.decode(t) {
        Ok(d) => d,
        Err(_) => return Err(ApiError::unauthorized(ApiError::TOKEN_INVALID, "Invalid access_token encoding")),
    };

    let decoded_str = match String::from_utf8(decoded) {
        Ok(s) => s,
        Err(_) => return Err(ApiError::unauthorized(ApiError::TOKEN_INVALID, "Invalid access_token")),
    };

    let sep_idx = match decoded_str.rfind(':') {
        Some(i) => i,
        None => return Err(ApiError::unauthorized(ApiError::TOKEN_INVALID, "Invalid access_token format")),
    };
    let (payload_str, sig_hex) = decoded_str.split_at(sep_idx);
    let sig_hex = &sig_hex[1..];

    let expected_sig = {
        let mut mac = hmac::Hmac::<Sha256>::new_from_slice(state.wopi_token_secret.as_bytes())
            .map_err(|_| ApiError::internal(ApiError::INTERNAL_ERROR, "HMAC error"))?;
        mac.update(payload_str.as_bytes());
        let sig = mac.finalize();
        hex::encode(sig.into_bytes())
    };

    if sig_hex != expected_sig {
        return Err(ApiError::unauthorized(ApiError::TOKEN_INVALID, "Invalid access_token signature"));
    }

    let payload: serde_json::Value = match serde_json::from_str(payload_str) {
        Ok(p) => p,
        Err(_) => return Err(ApiError::unauthorized(ApiError::TOKEN_INVALID, "Invalid access_token payload")),
    };

    if let Some(exp) = payload.get("exp").and_then(|v| v.as_i64()) {
        let now = chrono::Utc::now().timestamp();
        if now > exp {
            return Err(ApiError::unauthorized(ApiError::TOKEN_EXPIRED, "Access token expired"));
        }
    }

    Ok(t.clone())
}

/// WOPI Discovery endpoint.
///
/// Returns an XML document listing the WOPI operations this server supports.
/// Per MS-WOPI §7.2, the discovery document tells Office Online / Collabora
/// which URLs to use for viewing and editing files.
pub async fn wopi_discovery(State(state): State<AppState>) -> Response {
    let urlsrc = state.wopi_office_url.as_str();

    let discovery_xml = format!(
r#"<?xml version="1.0" encoding="utf-8"?>
<wopi-discovery>
  <net-zone name="external-https">
    <app name="Edit" favIconUrl="" checkLicense="true">
      <action name="edit" ext="odt" urlsrc="{urlsrc}"/>
      <action name="edit" ext="ods" urlsrc="{urlsrc}"/>
      <action name="edit" ext="odp" urlsrc="{urlsrc}"/>
      <action name="edit" ext="docx" urlsrc="{urlsrc}"/>
      <action name="edit" ext="xlsx" urlsrc="{urlsrc}"/>
      <action name="edit" ext="pptx" urlsrc="{urlsrc}"/>
      <action name="edit" ext="txt" urlsrc="{urlsrc}"/>
    </app>
    <app name="View" favIconUrl="" checkLicense="true">
      <action name="view" ext="pdf" urlsrc="{urlsrc}"/>
      <action name="view" ext="odt" urlsrc="{urlsrc}"/>
      <action name="view" ext="ods" urlsrc="{urlsrc}"/>
      <action name="view" ext="odp" urlsrc="{urlsrc}"/>
      <action name="view" ext="docx" urlsrc="{urlsrc}"/>
      <action name="view" ext="xlsx" urlsrc="{urlsrc}"/>
      <action name="view" ext="pptx" urlsrc="{urlsrc}"/>
      <action name="view" ext="txt" urlsrc="{urlsrc}"/>
    </app>
  </net-zone>
</wopi-discovery>"#);

    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        "Content-Type",
        axum::http::HeaderValue::from_static("application/xml; charset=utf-8"),
    );
    (StatusCode::OK, headers, discovery_xml).into_response()
}

pub async fn wopi_get(
    State(state): State<AppState>,
    Path(path): Path<String>,
    Query(params): Query<WopiQueryParams>,
) -> Response {
    // Validate access_token
    if let Err(resp) = validate_access_token(&state, &params.access_token) {
        return resp;
    }

    if let Some((file_path, _)) = split_contents_suffix(&path) {
        get_file_inner(&state, file_path).await
    } else {
        check_file_info_inner(&state, &path).await
    }
}

pub async fn wopi_post(
    State(state): State<AppState>,
    Path(path): Path<String>,
    headers: axum::http::HeaderMap,
    body: bytes::Bytes,
) -> Response {
    // Validate access_token from query params
    let access_token = headers
        .get("X-WOPI-AccessToken")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    if let Err(resp) = validate_access_token(&state, &access_token) {
        return resp;
    }

    let override_header = headers
        .get("X-WOPI-Override")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if let Some((file_path, _)) = split_contents_suffix(&path) {
        put_file_inner(&state, file_path, &headers, body).await
    } else if override_header == "LOCK" {
        lock_file_inner(&state, &path).await
    } else if override_header == "UNLOCK" {
        unlock_file_inner(&state, &path, &headers).await
    } else {
        lock_file_inner(&state, &path).await
    }
}

async fn check_file_info_inner(state: &AppState, path: &str) -> Response {
    let full_path = format!("/{}", path);

    let meta = match state.storage.head(&full_path).await {
        Ok(m) => m,
        Err(_) => return ApiError::not_found(ApiError::FILE_NOT_FOUND, "File not found"),

    };

    let file_name = meta
        .path
        .rsplit('/')
        .next()
        .unwrap_or("unknown")
        .to_string();

    let response = WopiCheckFileInfoResponse {
        base_file_name: file_name,
        size: meta.size,
        version: meta.etag.trim_matches('"').to_string(),
        last_modified_time: meta.modified_at.to_rfc3339(),
        owner_id: meta.owner,
        user_can_write: true,
        user_can_not_write_relative: false,
        supports_update: true,
        supports_locks: true,
        supports_coauth: false,
    };

    info!("WOPI CheckFileInfo: {}", full_path);
    (StatusCode::OK, axum::Json(response)).into_response()
}

async fn get_file_inner(state: &AppState, path: &str) -> Response {
    let full_path = format!("/{}", path);

    let content = match state.storage.get(&full_path).await {
        Ok(c) => c,
        Err(_) => return ApiError::not_found(ApiError::FILE_NOT_FOUND, "File not found"),
    };

    let meta = state
        .storage
        .head(&full_path)
        .await
        .unwrap_or_else(|_| {
            common::metadata::FileMetadata::new(
                full_path.clone(),
                common::metadata::ContentHash::new("0".repeat(64)),
                content.len() as u64,
                "anonymous".to_string(),
            )
        });

    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        "Content-Type",
        axum::http::HeaderValue::from_str(&meta.mime_type)
            .unwrap_or_else(|_| axum::http::HeaderValue::from_static("application/octet-stream")),
    );
    (StatusCode::OK, headers, axum::body::Body::from(content)).into_response()
}

async fn put_file_inner(
    state: &AppState,
    path: &str,
    headers: &axum::http::HeaderMap,
    body: bytes::Bytes,
) -> Response {
    let full_path = format!("/{}", path);

    if let Some(lock) = state.lock_manager.check_lock(&full_path).await {
        let lock_token = headers.get("X-WOPI-Lock").and_then(|v| v.to_str().ok());
        if let Some(token) = lock_token {
            if lock.token.as_opaque() != token {
                return ApiError::conflict(ApiError::FILE_LOCKED, "File locked by another user");
            }
        } else {
            return ApiError::with_details(
                StatusCode::CONFLICT,
                ApiError::FILE_LOCKED,
                "File locked by another user",
                lock.principal.clone(),
            );
        }
    }

    let owner = headers
        .get("X-WOPI-Override")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("anonymous");

    match state.storage.put(&full_path, body, owner).await {
        Ok(_) => {
            info!("WOPI PutFile: {}", full_path);
            (StatusCode::OK, "").into_response()
        }
        Err(e) => ApiError::internal(ApiError::INTERNAL_ERROR, e.to_string()),
    }
}

async fn lock_file_inner(state: &AppState, path: &str) -> Response {
    let full_path = format!("/{}", path);
    let principal = "wopi-editor".to_string();

    match state.lock_manager.acquire_lock(
        &full_path,
        &principal,
        common::webdav::LockScope::Exclusive,
        common::webdav::LockDepth::Zero,
        None,
    ).await {
        Ok(lock) => {
            let response = WopiLockResponse {
                lock_id: lock.token.as_str().to_string(),
                lock: "exclusive".to_string(),
                expires: format!("{}s", lock.timeout_seconds),
                user: principal,
            };
            (StatusCode::OK, axum::Json(response)).into_response()
        }
        Err(e) => ApiError::conflict(ApiError::FILE_LOCKED, e.to_string()),
    }
}

async fn unlock_file_inner(
    state: &AppState,
    path: &str,
    headers: &axum::http::HeaderMap,
) -> Response {
    let _full_path = format!("/{}", path);

    let lock_token = headers
        .get("X-WOPI-LockId")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    match state.lock_manager.release_lock(lock_token).await {
        Ok(()) => (StatusCode::OK, "").into_response(),
        Err(_) => ApiError::not_found(ApiError::NOT_FOUND, "Lock not found"),
    }
}

/// POST /wopi/files/:path/token — issue a time-limited WOPI access token.
///
/// The token is a base64-encoded JSON payload containing the file path,
/// user, expiry timestamp, and an HMAC signature for validation.
/// This allows Office Online to access files without re-authentication.
pub async fn wopi_issue_token(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<common::auth::Claims>,
    Path(path): Path<String>,
) -> Response {
    let full_path = format!("/{}", path.trim_matches('/'));

    // Check file exists
    if !state.storage.exists(&full_path).await.unwrap_or(false) {
        return ApiError::not_found(ApiError::FILE_NOT_FOUND, "File not found");
    }

    // Issue token valid for 8 hours
    let expires = chrono::Utc::now().timestamp() + (8 * 3600);
    let token_payload = serde_json::json!({
        "path": full_path,
        "user": claims.sub,
        "exp": expires,
    });

    let payload_str = serde_json::to_string(&token_payload).unwrap_or_default();
    let mut mac = hmac::Hmac::<Sha256>::new_from_slice(state.wopi_token_secret.as_bytes()).unwrap();
    mac.update(payload_str.as_bytes());
    let signature = mac.finalize();
    let sig_hex = hex::encode(signature.into_bytes());

    let token = base64::engine::general_purpose::STANDARD.encode(format!("{}:{}", payload_str, sig_hex));

    (StatusCode::OK, axum::Json(serde_json::json!({
        "access_token": token,
        "expires_in": 28800,
        "token_ttl": 28800,
    }))).into_response()
}

#[cfg(test)]
mod tests {
    use crate::AppState;
    use base64::Engine;
    use hmac::Mac;

    fn build_token_payload(path: &str, user: &str, exp: i64) -> String {
        let payload = serde_json::json!({
            "path": path,
            "user": user,
            "exp": exp,
        });
        serde_json::to_string(&payload).unwrap()
    }

    fn sign_token(payload_str: &str, secret: &str) -> String {
        use sha2::Sha256;
        let mut mac = hmac::Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(payload_str.as_bytes());
        let signature = mac.finalize();
        let sig_hex = hex::encode(signature.into_bytes());
        base64::engine::general_purpose::STANDARD.encode(format!("{}:{}", payload_str, sig_hex))
    }

    #[test]
    fn test_token_issued_with_custom_secret() {
        let secret_a = "my-custom-secret-abc";
        let secret_b = "different-secret-xyz";

        let payload = build_token_payload("/test.txt", "user1", 9999999999);

        let token_a = sign_token(&payload, secret_a);
        let token_b = sign_token(&payload, secret_b);

        assert_ne!(token_a, token_b, "Tokens signed with different secrets must differ");

        let state_a = AppState::in_memory().with_wopi_token_secret(secret_a.to_string());
        let state_b = AppState::in_memory().with_wopi_token_secret(secret_b.to_string());

        assert_eq!(state_a.wopi_token_secret, secret_a);
        assert_eq!(state_b.wopi_token_secret, secret_b);
    }

    #[test]
    fn test_token_signature_verifiable() {
        let secret = "test-secret-key";
        let payload = build_token_payload("/docs/report.odt", "alice", 4000000000);
        let token = sign_token(&payload, secret);

        let decoded = base64::engine::general_purpose::STANDARD.decode(&token).unwrap();
        let decoded_str = String::from_utf8(decoded).unwrap();
        // The format is "{json_payload}:{hex_signature}".
        // Since JSON contains ':', split from the right (hex sig has no ':').
        let sep_idx = decoded_str.rfind(':').unwrap();
        let (recovered_payload, recovered_sig) = decoded_str.split_at(sep_idx);
        let recovered_sig = &recovered_sig[1..]; // skip the ':'
        assert_eq!(recovered_payload, payload);

        use sha2::Sha256;
        let mut mac = hmac::Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(payload.as_bytes());
        let signature = mac.finalize();
        let expected_sig = hex::encode(signature.into_bytes());
        assert_eq!(recovered_sig, expected_sig);
    }

    #[test]
    fn test_default_secret_differs_from_custom() {
        let default = "ferro-wopi-token-secret-change-me";
        let custom = "my-production-secret";
        assert_ne!(default, custom);

        let payload = build_token_payload("/file.txt", "user", 9999999999);
        let token_default = sign_token(&payload, default);
        let token_custom = sign_token(&payload, custom);
        assert_ne!(token_default, token_custom);
    }

    #[test]
    fn test_validate_valid_token() {
        let secret = "test-secret";
        let payload = build_token_payload("/file.txt", "user", 9999999999);
        let token = sign_token(&payload, secret);
        let state = AppState::in_memory().with_wopi_token_secret(secret.to_string());
        let result = super::validate_access_token(&state, &Some(token));
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_wrong_secret_rejected() {
        let payload = build_token_payload("/file.txt", "user", 9999999999);
        let token = sign_token(&payload, "correct-secret");
        let state = AppState::in_memory().with_wopi_token_secret("wrong-secret".to_string());
        let result = super::validate_access_token(&state, &Some(token));
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_expired_token_rejected() {
        let secret = "test-secret";
        let past_exp = chrono::Utc::now().timestamp() - 3600;
        let payload = build_token_payload("/file.txt", "user", past_exp);
        let token = sign_token(&payload, secret);
        let state = AppState::in_memory().with_wopi_token_secret(secret.to_string());
        let result = super::validate_access_token(&state, &Some(token));
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_missing_token_rejected() {
        let state = AppState::in_memory();
        let result = super::validate_access_token(&state, &None);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_empty_token_rejected() {
        let state = AppState::in_memory();
        let result = super::validate_access_token(&state, &Some(String::new()));
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_garbage_token_rejected() {
        let state = AppState::in_memory();
        let result = super::validate_access_token(&state, &Some("not-a-valid-token".to_string()));
        assert!(result.is_err());
    }
}
