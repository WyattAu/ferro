use axum::extract::{Extension, Path, Query};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use base64::Engine;
use hmac::{KeyInit, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::sync::Arc;
use tracing::info;

use common::storage::LockManagerTrait;
use common::storage::StorageEngine;

#[derive(Clone)]
pub struct WopiState {
    pub storage: Arc<dyn StorageEngine>,
    pub lock_manager: Arc<dyn LockManagerTrait>,
    pub wopi_token_secret: String,
    pub wopi_office_url: String,
}

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

fn wopi_error(status: StatusCode, code: &str, message: impl Into<String>) -> Response {
    let body = serde_json::json!({
        "error": message.into(),
        "error_code": code.to_string(),
    });
    (status, axum::Json(body)).into_response()
}

fn wopi_error_with_details(
    status: StatusCode,
    code: &str,
    message: impl Into<String>,
    details: impl Into<String>,
) -> Response {
    let body = serde_json::json!({
        "error": message.into(),
        "error_code": code.to_string(),
        "details": details.into(),
    });
    (status, axum::Json(body)).into_response()
}

#[allow(clippy::result_large_err)]
fn validate_access_token(state: &WopiState, token: &Option<String>) -> Result<String, Response> {
    let t = match token {
        None => {
            return Err(wopi_error(
                StatusCode::UNAUTHORIZED,
                "TOKEN_INVALID",
                "Missing access_token parameter",
            ));
        }
        Some(t) if t.is_empty() => {
            return Err(wopi_error(
                StatusCode::UNAUTHORIZED,
                "TOKEN_INVALID",
                "Empty access_token",
            ));
        }
        Some(t) => t,
    };

    let decoded = match base64::engine::general_purpose::STANDARD.decode(t) {
        Ok(d) => d,
        Err(_) => {
            return Err(wopi_error(
                StatusCode::UNAUTHORIZED,
                "TOKEN_INVALID",
                "Invalid access_token encoding",
            ));
        }
    };

    let decoded_str = match String::from_utf8(decoded) {
        Ok(s) => s,
        Err(_) => {
            return Err(wopi_error(
                StatusCode::UNAUTHORIZED,
                "TOKEN_INVALID",
                "Invalid access_token",
            ));
        }
    };

    let sep_idx = match decoded_str.rfind(':') {
        Some(i) => i,
        None => {
            return Err(wopi_error(
                StatusCode::UNAUTHORIZED,
                "TOKEN_INVALID",
                "Invalid access_token format",
            ));
        }
    };
    let (payload_str, sig_hex) = decoded_str.split_at(sep_idx);
    let sig_hex = &sig_hex[1..];

    let expected_sig = {
        let mut mac = hmac::Hmac::<Sha256>::new_from_slice(state.wopi_token_secret.as_bytes())
            .map_err(|_| {
                wopi_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_ERROR",
                    "HMAC error",
                )
            })?;
        mac.update(payload_str.as_bytes());
        let sig = mac.finalize();
        hex::encode(sig.into_bytes())
    };

    if sig_hex != expected_sig {
        return Err(wopi_error(
            StatusCode::UNAUTHORIZED,
            "TOKEN_INVALID",
            "Invalid access_token signature",
        ));
    }

    let payload: serde_json::Value = match serde_json::from_str(payload_str) {
        Ok(p) => p,
        Err(_) => {
            return Err(wopi_error(
                StatusCode::UNAUTHORIZED,
                "TOKEN_INVALID",
                "Invalid access_token payload",
            ));
        }
    };

    if let Some(exp) = payload.get("exp").and_then(|v| v.as_i64()) {
        let now = chrono::Utc::now().timestamp();
        if now > exp {
            return Err(wopi_error(
                StatusCode::UNAUTHORIZED,
                "TOKEN_EXPIRED",
                "Access token expired",
            ));
        }
    }

    Ok(t.clone())
}

pub fn routes<S: Clone + Send + Sync + 'static>() -> axum::Router<S> {
    axum::Router::new()
        .route("/files/*path", axum::routing::get(wopi_get).post(wopi_post))
        .route("/files/{path}/token", axum::routing::post(wopi_issue_token))
}

pub fn discovery_route<S: Clone + Send + Sync + 'static>() -> axum::Router<S> {
    axum::Router::new().route("/discovery", axum::routing::get(wopi_discovery))
}

pub async fn wopi_discovery(Extension(state): Extension<WopiState>) -> Response {
    let urlsrc = state.wopi_office_url.as_str();

    if urlsrc.is_empty() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            axum::Json(serde_json::json!({
                "error": "WOPI not configured",
                "error_code": "NOT_CONFIGURED",
                "message": "Set --wopi-office-url to enable WOPI discovery."
            })),
        )
            .into_response();
    }

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
</wopi-discovery>"#
    );

    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        "Content-Type",
        axum::http::HeaderValue::from_static("application/xml; charset=utf-8"),
    );
    (StatusCode::OK, headers, discovery_xml).into_response()
}

pub async fn wopi_get(
    Extension(state): Extension<WopiState>,
    Path(path): Path<String>,
    Query(params): Query<WopiQueryParams>,
) -> Response {
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
    Extension(state): Extension<WopiState>,
    Path(path): Path<String>,
    headers: axum::http::HeaderMap,
    body: bytes::Bytes,
) -> Response {
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

async fn check_file_info_inner(state: &WopiState, path: &str) -> Response {
    let full_path = format!("/{}", path);

    let meta = match state.storage.head(&full_path).await {
        Ok(m) => m,
        Err(_) => return wopi_error(StatusCode::NOT_FOUND, "FILE_NOT_FOUND", "File not found"),
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

async fn get_file_inner(state: &WopiState, path: &str) -> Response {
    let full_path = format!("/{}", path);

    let content = match state.storage.get(&full_path).await {
        Ok(c) => c,
        Err(_) => return wopi_error(StatusCode::NOT_FOUND, "FILE_NOT_FOUND", "File not found"),
    };

    let meta = state.storage.head(&full_path).await.unwrap_or_else(|_| {
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
    state: &WopiState,
    path: &str,
    headers: &axum::http::HeaderMap,
    body: bytes::Bytes,
) -> Response {
    let full_path = format!("/{}", path);

    if let Some(lock) = state.lock_manager.check_lock(&full_path).await {
        let lock_token = headers.get("X-WOPI-Lock").and_then(|v| v.to_str().ok());
        if let Some(token) = lock_token {
            if lock.token.as_opaque() != token {
                return wopi_error(
                    StatusCode::CONFLICT,
                    "FILE_LOCKED",
                    "File locked by another user",
                );
            }
        } else {
            return wopi_error_with_details(
                StatusCode::CONFLICT,
                "FILE_LOCKED",
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
        Err(e) => wopi_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "INTERNAL_ERROR",
            e.to_string(),
        ),
    }
}

async fn lock_file_inner(state: &WopiState, path: &str) -> Response {
    let full_path = format!("/{}", path);
    let principal = "wopi-editor".to_string();

    match state
        .lock_manager
        .acquire_lock(
            &full_path,
            &principal,
            common::webdav::LockScope::Exclusive,
            common::webdav::LockDepth::Zero,
            None,
        )
        .await
    {
        Ok(lock) => {
            let response = WopiLockResponse {
                lock_id: lock.token.as_str().to_string(),
                lock: "exclusive".to_string(),
                expires: format!("{}s", lock.timeout_seconds),
                user: principal,
            };
            (StatusCode::OK, axum::Json(response)).into_response()
        }
        Err(e) => wopi_error(StatusCode::CONFLICT, "FILE_LOCKED", e.to_string()),
    }
}

async fn unlock_file_inner(
    state: &WopiState,
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
        Err(_) => wopi_error(StatusCode::NOT_FOUND, "NOT_FOUND", "Lock not found"),
    }
}

pub async fn wopi_issue_token(
    Extension(state): Extension<WopiState>,
    Extension(claims): Extension<common::auth::Claims>,
    Path(path): Path<String>,
) -> Response {
    if state.wopi_token_secret.is_empty() {
        tracing::error!(
            "WOPI token secret is not configured. Set --wopi-token-secret to a strong random value."
        );
        return wopi_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "WOPI_TOKEN_SECRET_NOT_SET",
            "WOPI token secret is not configured. Set --wopi-token-secret to a strong random value.",
        );
    }

    let full_path = format!("/{}", path.trim_matches('/'));

    if !state.storage.exists(&full_path).await.unwrap_or(false) {
        return wopi_error(StatusCode::NOT_FOUND, "FILE_NOT_FOUND", "File not found");
    }

    let expires = chrono::Utc::now().timestamp() + (8 * 3600);
    let token_payload = serde_json::json!({
        "path": full_path,
        "user": claims.sub,
        "exp": expires,
    });

    let payload_str = serde_json::to_string(&token_payload).unwrap_or_default();
    let mut mac = match hmac::Hmac::<Sha256>::new_from_slice(state.wopi_token_secret.as_bytes()) {
        Ok(m) => m,
        Err(_) => {
            return wopi_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "HMAC_KEY_ERROR",
                "WOPI token secret is too long for HMAC key",
            );
        }
    };
    mac.update(payload_str.as_bytes());
    let signature = mac.finalize();
    let sig_hex = hex::encode(signature.into_bytes());

    let token =
        base64::engine::general_purpose::STANDARD.encode(format!("{}:{}", payload_str, sig_hex));

    (
        StatusCode::OK,
        axum::Json(serde_json::json!({
            "access_token": token,
            "expires_in": 28800,
            "token_ttl": 28800,
        })),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use hmac::{KeyInit, Mac};

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

    use common::error::FerroError;
    use common::metadata::ContentHash;
    use common::metadata::FileMetadata;
    use common::storage::StorageEngine;
    use common::webdav::{LockDepth, LockInfo, LockScope, LockType};

    #[derive(Default)]
    struct MockStorage {
        files: tokio::sync::RwLock<std::collections::HashMap<String, bytes::Bytes>>,
    }

    #[async_trait::async_trait]
    impl StorageEngine for MockStorage {
        async fn head(&self, path: &str) -> common::error::Result<FileMetadata> {
            let files = self.files.read().await;
            if !files.contains_key(path) {
                return Err(FerroError::NotFound(path.to_string()));
            }
            let size = files.get(path).map(|b| b.len() as u64).unwrap_or(0);
            Ok(FileMetadata::new(
                path.to_string(),
                ContentHash::new("0".repeat(64)),
                size,
                "owner".to_string(),
            ))
        }

        async fn get(&self, path: &str) -> common::error::Result<bytes::Bytes> {
            let files = self.files.read().await;
            files
                .get(path)
                .cloned()
                .ok_or_else(|| FerroError::NotFound(path.to_string()))
        }

        async fn put(
            &self,
            path: &str,
            content: bytes::Bytes,
            _owner: &str,
        ) -> common::error::Result<FileMetadata> {
            self.files.write().await.insert(path.to_string(), content);
            self.head(path).await
        }

        async fn delete(&self, path: &str) -> common::error::Result<()> {
            self.files.write().await.remove(path);
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
        async fn exists(&self, path: &str) -> common::error::Result<bool> {
            Ok(self.files.read().await.contains_key(path))
        }

        async fn create_collection(
            &self,
            path: &str,
            _owner: &str,
        ) -> common::error::Result<FileMetadata> {
            self.files
                .write()
                .await
                .insert(path.to_string(), bytes::Bytes::new());
            self.head(path).await
        }

        async fn list_all(
            &self,
            _path: &str,
            _max_depth: u32,
        ) -> common::error::Result<Vec<FileMetadata>> {
            Ok(vec![])
        }
    }

    #[derive(Default)]
    struct MockLockManager {
        locks: tokio::sync::RwLock<std::collections::HashMap<String, LockInfo>>,
    }

    #[async_trait::async_trait]
    impl LockManagerTrait for MockLockManager {
        async fn check_lock(&self, path: &str) -> Option<LockInfo> {
            self.locks.read().await.get(path).cloned()
        }

        async fn check_lock_for_write(&self, _path: &str) -> common::error::Result<()> {
            Ok(())
        }

        async fn acquire_lock(
            &self,
            path: &str,
            principal: &str,
            scope: LockScope,
            depth: LockDepth,
            timeout_secs: Option<u32>,
        ) -> common::error::Result<LockInfo> {
            let info = LockInfo {
                token: common::webdav::LockToken::new(),
                path: path.to_string(),
                principal: principal.to_string(),
                scope,
                lock_type: LockType::Write,
                depth,
                timeout_seconds: timeout_secs.unwrap_or(60),
                created_at: chrono::Utc::now(),
                refresh_count: 0,
            };
            self.locks
                .write()
                .await
                .insert(path.to_string(), info.clone());
            Ok(info)
        }

        async fn release_lock(&self, token: &str) -> common::error::Result<()> {
            let mut locks = self.locks.write().await;
            let key = locks
                .iter()
                .find(|(_, v)| v.token.as_str() == token)
                .map(|(k, _)| k.clone());
            if let Some(k) = key {
                locks.remove(&k);
                Ok(())
            } else {
                Err(FerroError::NotFound("lock not found".to_string()))
            }
        }

        async fn refresh_lock(
            &self,
            _token: &str,
            _timeout_secs: Option<u32>,
        ) -> common::error::Result<LockInfo> {
            Err(FerroError::NotFound("lock not found".to_string()))
        }

        async fn all_locks(&self) -> Vec<LockInfo> {
            self.locks.read().await.values().cloned().collect()
        }
    }

    fn make_wopi_state(secret: &str) -> WopiState {
        WopiState {
            storage: Arc::new(MockStorage::default()),
            lock_manager: Arc::new(MockLockManager::default()),
            wopi_token_secret: secret.to_string(),
            wopi_office_url: "http://localhost:9980".to_string(),
        }
    }

    #[test]
    fn test_token_issued_with_custom_secret() {
        let secret_a = "my-custom-secret-abc";
        let secret_b = "different-secret-xyz";

        let payload = build_token_payload("/test.txt", "user1", 9999999999);

        let token_a = sign_token(&payload, secret_a);
        let token_b = sign_token(&payload, secret_b);

        assert_ne!(
            token_a, token_b,
            "Tokens signed with different secrets must differ"
        );

        let state_a = make_wopi_state(secret_a);
        let state_b = make_wopi_state(secret_b);

        assert_eq!(state_a.wopi_token_secret, secret_a);
        assert_eq!(state_b.wopi_token_secret, secret_b);
    }

    #[test]
    fn test_token_signature_verifiable() {
        let secret = "test-secret-key";
        let payload = build_token_payload("/docs/report.odt", "alice", 4000000000);
        let token = sign_token(&payload, secret);

        let decoded = base64::engine::general_purpose::STANDARD
            .decode(&token)
            .unwrap();
        let decoded_str = String::from_utf8(decoded).unwrap();
        let sep_idx = decoded_str.rfind(':').unwrap();
        let (recovered_payload, recovered_sig) = decoded_str.split_at(sep_idx);
        let recovered_sig = &recovered_sig[1..];
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
        let state = make_wopi_state(secret);
        let result = validate_access_token(&state, &Some(token));
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_wrong_secret_rejected() {
        let payload = build_token_payload("/file.txt", "user", 9999999999);
        let token = sign_token(&payload, "correct-secret");
        let state = make_wopi_state("wrong-secret");
        let result = validate_access_token(&state, &Some(token));
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_expired_token_rejected() {
        let secret = "test-secret";
        let past_exp = chrono::Utc::now().timestamp() - 3600;
        let payload = build_token_payload("/file.txt", "user", past_exp);
        let token = sign_token(&payload, secret);
        let state = make_wopi_state(secret);
        let result = validate_access_token(&state, &Some(token));
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_missing_token_rejected() {
        let state = make_wopi_state("test");
        let result = validate_access_token(&state, &None);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_empty_token_rejected() {
        let state = make_wopi_state("test");
        let result = validate_access_token(&state, &Some(String::new()));
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_garbage_token_rejected() {
        let state = make_wopi_state("test");
        let result = validate_access_token(&state, &Some("not-a-valid-token".to_string()));
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_wopi_discovery_returns_xml() {
        use http_body_util::BodyExt;
        use tower::ServiceExt;

        let state = make_wopi_state("test");
        let app = discovery_route::<()>().layer(axum::Extension(state));

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/discovery")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let content_type = response.headers().get("content-type").unwrap();
        assert!(content_type.to_str().unwrap().contains("application/xml"));

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        assert!(body_str.contains("<wopi-discovery>"));
        assert!(body_str.contains("urlsrc=\"http://localhost:9980\""));
    }

    #[tokio::test]
    async fn test_wopi_check_file_info() {
        use http_body_util::BodyExt;
        use tower::ServiceExt;

        let secret = "test-secret";
        let state = make_wopi_state(secret);

        state
            .storage
            .put(
                "/hello.txt",
                bytes::Bytes::from_static(b"hello world"),
                "owner",
            )
            .await
            .unwrap();

        let token_payload = build_token_payload("/hello.txt", "user1", 9999999999);
        let token = sign_token(&token_payload, secret);

        let app = routes::<()>().layer(axum::Extension(state));

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri(format!("/files/hello.txt?access_token={token}"))
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["base_file_name"], "hello.txt");
        assert_eq!(json["size"], 11);
        assert!(json["supports_update"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn test_wopi_get_file_contents() {
        use http_body_util::BodyExt;
        use tower::ServiceExt;

        let secret = "test-secret";
        let state = make_wopi_state(secret);

        state
            .storage
            .put(
                "/doc.txt",
                bytes::Bytes::from_static(b"content here"),
                "owner",
            )
            .await
            .unwrap();

        let token_payload = build_token_payload("/doc.txt/contents", "user1", 9999999999);
        let token = sign_token(&token_payload, secret);

        let app = routes::<()>().layer(axum::Extension(state));

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri(format!("/files/doc.txt/contents?access_token={token}"))
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(&body[..], b"content here");
    }

    #[tokio::test]
    async fn test_wopi_check_file_not_found() {
        use tower::ServiceExt;

        let secret = "test-secret";
        let state = make_wopi_state(secret);

        let token_payload = build_token_payload("/nonexistent.txt", "user1", 9999999999);
        let token = sign_token(&token_payload, secret);

        let app = routes::<()>().layer(axum::Extension(state));

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri(format!("/files/nonexistent.txt?access_token={token}"))
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_wopi_lock_and_unlock() {
        use http_body_util::BodyExt;
        use tower::ServiceExt;

        let secret = "test-secret";
        let state = make_wopi_state(secret);

        state
            .storage
            .put("/file.odt", bytes::Bytes::from_static(b"data"), "owner")
            .await
            .unwrap();

        let token_payload = build_token_payload("/file.odt", "user1", 9999999999);
        let token = sign_token(&token_payload, secret);

        let app = routes::<()>().layer(axum::Extension(state));

        let lock_response = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/files/file.odt")
                    .header("X-WOPI-AccessToken", &token)
                    .header("X-WOPI-Override", "LOCK")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(lock_response.status(), StatusCode::OK);
        let lock_body = lock_response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        let lock_json: serde_json::Value = serde_json::from_slice(&lock_body).unwrap();
        let lock_id = lock_json["lock_id"].as_str().unwrap().to_string();

        let unlock_response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/files/file.odt")
                    .header("X-WOPI-AccessToken", &token)
                    .header("X-WOPI-Override", "UNLOCK")
                    .header("X-WOPI-LockId", &lock_id)
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(unlock_response.status(), StatusCode::OK);
    }
}
