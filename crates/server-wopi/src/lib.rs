use axum::extract::{Extension, Path, Query};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use base64::Engine;
use hmac::{Hmac, Mac};
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

#[derive(Debug, Deserialize)]
pub struct TokenQueryParams {
    pub path: Option<String>,
    /// `edit` (default) or `view` — view tokens cannot PutFile/LOCK.
    pub scope: Option<String>,
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
fn validate_access_token(state: &WopiState, token: &Option<String>) -> Result<WopiTokenClaims, Response> {
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

    // Constant-time verification (DC-WOPI-003): compare full signature bytes,
    // never a truncated prefix and never via early-exit string equality.
    let sig_bytes = match hex::decode(sig_hex) {
        Ok(b) => b,
        Err(_) => {
            return Err(wopi_error(
                StatusCode::UNAUTHORIZED,
                "TOKEN_INVALID",
                "Invalid access_token signature",
            ));
        }
    };
    {
        let mut mac = Hmac::<Sha256>::new_from_slice(state.wopi_token_secret.as_bytes())
            .map_err(|_| wopi_error(StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", "HMAC error"))?;
        mac.update(payload_str.as_bytes());
        if mac.verify_slice(&sig_bytes).is_err() {
            return Err(wopi_error(
                StatusCode::UNAUTHORIZED,
                "TOKEN_INVALID",
                "Invalid access_token signature",
            ));
        }
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

    // Tokens issued before scope existed are edit-capable (compat: view-only
    // sessions did not exist pre-YP-COLLAB-WOPI-001).
    let scope = payload
        .get("scope")
        .and_then(|v| v.as_str())
        .unwrap_or(WOPI_SCOPE_EDIT)
        .to_string();

    Ok(WopiTokenClaims {
        path: payload
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        user: payload
            .get("user")
            .and_then(|v| v.as_str())
            .unwrap_or("anonymous")
            .to_string(),
        exp: payload.get("exp").and_then(|v| v.as_i64()).unwrap_or(0),
        scope,
    })
}

/// Parsed, signature-verified WOPI access token claims (DC-WOPI-001/002/003).
#[derive(Debug, Clone)]
pub struct WopiTokenClaims {
    /// Exact virtual path (`/...`) this token is bound to. Requests for any
    /// other path are rejected — a stolen token is useless for other files.
    pub path: String,
    pub user: String,
    pub exp: i64,
    /// `edit` or `view` (THM-WOPI-SCOPE).
    pub scope: String,
}

impl WopiTokenClaims {
    pub fn can_edit(&self) -> bool {
        self.scope == WOPI_SCOPE_EDIT
    }
}

const WOPI_SCOPE_EDIT: &str = "edit";
const WOPI_SCOPE_VIEW: &str = "view";

/// Proof-key freshness window (MS-WOPI: 20 minutes).
const WOPI_PROOF_TTL_SECS: i64 = 20 * 60;

/// Derive a WOPI proof key from the token secret (domain-separated via HKDF).
/// Two generations exist so key rotation never breaks in-flight sessions:
/// `gen 0` is the old key, `gen 1` the current one.
#[allow(clippy::result_large_err)]
fn wopi_proof_key(state: &WopiState, key_gen: u8) -> Result<Vec<u8>, Response> {
    if state.wopi_token_secret.is_empty() {
        return Err(wopi_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "WOPI_TOKEN_SECRET_NOT_SET",
            "WOPI token secret is not configured",
        ));
    }
    let salt = b"ferro-wopi-proof-key-v1";
    let info = [b"proof".as_slice(), [key_gen].as_slice()].concat();
    let mut okm = [0u8; 32];
    hkdf::Hkdf::<Sha256>::new(Some(salt), state.wopi_token_secret.as_bytes())
        .expand(&info, &mut okm)
        .map_err(|_| wopi_error(StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", "HKDF error"))?;
    Ok(okm.to_vec())
}

/// Parse an X-WOPI-TimeStamp value: MS-WOPI FILETIME ticks (100 ns since
/// 1601-01-01) or an RFC 3339 string. Returns Unix seconds.
fn parse_wopi_timestamp(raw: &str) -> Option<i64> {
    if let Ok(ticks) = raw.trim().parse::<u64>() {
        // FILETIME epoch (1601-01-01) is 11_644_473_600 s before the Unix epoch.
        const FILETIME_TO_UNIX_SECS: u64 = 11_644_473_600;
        const TICKS_PER_SEC: u64 = 10_000_000;
        let secs = ticks / TICKS_PER_SEC;
        return i64::try_from(secs.checked_sub(FILETIME_TO_UNIX_SECS)?).ok();
    }
    chrono::DateTime::parse_from_rfc3339(raw.trim())
        .ok()
        .map(|dt| dt.timestamp())
}

/// Verify the WOPI request proof headers when present (YP-COLLAB-WOPI-001,
/// THM-WOPI-PROOF-SAFETY / THM-WOPI-NO-FALLTHROUGH).
///
/// Ferro's proof profile: `X-WOPI-Proof` carries `base64( HMAC-SHA256(key,
/// request_url || "\n" || access_token || "\n" || timestamp) )`; the MAC must
/// verify under the CURRENT proof key or the OLD one (rotation tolerance), and
/// the timestamp must be fresh within ±20 minutes.
///
/// Absence of both headers is accepted: the deployed office client
/// (Collabora) implements token-over-TLS but not the MS-WOPI proof —
/// rejecting all proofless traffic would break office editing. A *present
/// but invalid* proof is always fatal (no fallthrough to token-only auth).
#[allow(clippy::result_large_err)]
fn validate_wopi_proof(
    state: &WopiState,
    headers: &axum::http::HeaderMap,
    request_url: &str,
    access_token: &str,
) -> Result<(), Response> {
    let proof = headers.get("X-WOPI-Proof").and_then(|v| v.to_str().ok());
    let ts_raw = headers.get("X-WOPI-TimeStamp").and_then(|v| v.to_str().ok());

    match (proof, ts_raw) {
        (None, None) => Ok(()),
        (Some(_), None) | (None, Some(_)) => Err(wopi_error(
            StatusCode::UNAUTHORIZED,
            "PROOF_INVALID",
            "Incomplete WOPI proof: both X-WOPI-Proof and X-WOPI-TimeStamp are required",
        )),
        (Some(proof), Some(ts_raw)) => {
            let ts = parse_wopi_timestamp(ts_raw)
                .ok_or_else(|| wopi_error(StatusCode::UNAUTHORIZED, "PROOF_INVALID", "Malformed X-WOPI-TimeStamp"))?;
            let now = chrono::Utc::now().timestamp();
            if (now - ts).abs() > WOPI_PROOF_TTL_SECS {
                return Err(wopi_error(
                    StatusCode::UNAUTHORIZED,
                    "PROOF_EXPIRED",
                    "WOPI proof timestamp outside freshness window",
                ));
            }

            let proof_input = format!("{}\n{}\n{}", request_url, access_token, ts_raw);
            let proof_bytes = base64::engine::general_purpose::STANDARD
                .decode(proof)
                .map_err(|_| wopi_error(StatusCode::UNAUTHORIZED, "PROOF_INVALID", "Malformed X-WOPI-Proof"))?;

            let mut verified = false;
            for key_gen in [1u8, 0u8] {
                let key = wopi_proof_key(state, key_gen)?;
                if let Ok(mut mac) = Hmac::<Sha256>::new_from_slice(&key) {
                    mac.update(proof_input.as_bytes());
                    if mac.verify_slice(&proof_bytes).is_ok() {
                        verified = true;
                        break;
                    }
                }
            }
            if !verified {
                return Err(wopi_error(
                    StatusCode::UNAUTHORIZED,
                    "PROOF_INVALID",
                    "WOPI proof verification failed",
                ));
            }
            Ok(())
        }
    }
}

/// Enforce token↔path binding (THM-WOPI-SCOPE corollary): the token is bound
/// to the exact file path it was issued for.
#[allow(clippy::result_large_err)]
fn validate_token_path(claims: &WopiTokenClaims, full_path: &str) -> Result<(), Response> {
    if claims.path != full_path {
        return Err(wopi_error(
            StatusCode::UNAUTHORIZED,
            "TOKEN_PATH_MISMATCH",
            "Access token is not valid for this path",
        ));
    }
    Ok(())
}

/// Enforce the edit scope for mutating WOPI operations (THM-WOPI-SCOPE).
#[allow(clippy::result_large_err)]
fn validate_edit_scope(claims: &WopiTokenClaims) -> Result<(), Response> {
    if !claims.can_edit() {
        return Err(wopi_error(
            StatusCode::FORBIDDEN,
            "READ_ONLY_SESSION",
            "This WOPI session is view-only",
        ));
    }
    Ok(())
}

pub fn routes<S: Clone + Send + Sync + 'static>() -> axum::Router<S> {
    axum::Router::new()
        .route("/files/*path", axum::routing::get(wopi_get).post(wopi_post))
        .route("/office-discovery", axum::routing::get(wopi_office_discovery_proxy))
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

/// Relay the office suite's real WOPI discovery document to the frontend.
///
/// The browser cannot fetch `https://collabora.../hosting/discovery` itself
/// (no CORS headers), and our static mirror lacks the versioned
/// `browser/<id>/cool.html` urlsrc templates the editor iframe needs. This
/// passthrough is authenticated (sits behind the Bearer auth layer) and
/// simply relays the XML.
pub async fn wopi_office_discovery_proxy(Extension(state): Extension<WopiState>) -> Response {
    if state.wopi_office_url.is_empty() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            axum::Json(serde_json::json!({
                "error": "WOPI not configured",
                "error_code": "NOT_CONFIGURED",
                "message": "Set FERRO_WOPI_OFFICE_URL to enable office editing."
            })),
        )
            .into_response();
    }

    let discovery_url = format!("{}/hosting/discovery", state.wopi_office_url.trim_end_matches('/'));
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return wopi_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "HTTP_CLIENT_ERROR",
                format!("Failed to build HTTP client: {e}"),
            );
        }
    };
    match client.get(&discovery_url).send().await {
        Ok(resp) => {
            let status = StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
            let content_type = resp
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("text/xml")
                .to_string();
            match resp.bytes().await {
                Ok(body) => {
                    let mut headers = axum::http::HeaderMap::new();
                    if let Ok(v) = axum::http::HeaderValue::from_str(&content_type) {
                        headers.insert("Content-Type", v);
                    }
                    // Cache for an hour — discovery only changes on office upgrades.
                    headers.insert(
                        "Cache-Control",
                        axum::http::HeaderValue::from_static("public, max-age=3600"),
                    );
                    (status, headers, body).into_response()
                }
                Err(e) => wopi_error(
                    StatusCode::BAD_GATEWAY,
                    "DISCOVERY_READ_ERROR",
                    format!("Office discovery read failed: {e}"),
                ),
            }
        }
        Err(e) => wopi_error(
            StatusCode::BAD_GATEWAY,
            "OFFICE_UNREACHABLE",
            format!("Office server unreachable at {discovery_url}: {e}"),
        ),
    }
}

pub async fn wopi_get(
    Extension(state): Extension<WopiState>,
    // Collabora echoes the WOPISrc we issued, which single-segment-encodes
    // the virtual path (%2F for '/'). Axum 0.7 hands the RAW segment —
    // decode once so storage keys are the real paths.
    Path(raw): Path<String>,
    headers: axum::http::HeaderMap,
    Query(params): Query<WopiQueryParams>,
) -> Response {
    let claims = match validate_access_token(&state, &params.access_token) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    let path = common::path::decode_percent(&raw).into_owned();
    let full_path = format!("/{}", path);
    // Bind the token to the underlying FILE: `/contents` sub-requests target
    // the same file the token was issued for.
    let binding_path = match split_contents_suffix(&path) {
        Some((file_path, _)) => format!("/{}", file_path),
        None => full_path.clone(),
    };
    if let Err(resp) = validate_token_path(&claims, &binding_path) {
        return resp;
    }
    // Request URI as the office client saw it (WOPI proof input binding).
    let request_url = headers
        .get("X-Forwarded-Proto")
        .and_then(|v| v.to_str().ok())
        .map(|p| {
            format!(
                "{}://{}",
                p,
                headers.get("host").and_then(|v| v.to_str().ok()).unwrap_or("")
            )
        })
        .map(|authority| format!("{}{}", authority, full_path))
        .unwrap_or_else(|| full_path.clone());
    if let Err(resp) = validate_wopi_proof(
        &state,
        &headers,
        &request_url,
        params.access_token.as_deref().unwrap_or(""),
    ) {
        return resp;
    }

    if let Some((file_path, _)) = split_contents_suffix(&path) {
        get_file_inner(&state, file_path).await
    } else {
        check_file_info_inner(&state, &path, &claims).await
    }
}

pub async fn wopi_post(
    Extension(state): Extension<WopiState>,
    Path(raw): Path<String>,
    headers: axum::http::HeaderMap,
    body: bytes::Bytes,
) -> Response {
    let path = common::path::decode_percent(&raw).into_owned();
    let access_token = headers
        .get("X-WOPI-AccessToken")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let claims = match validate_access_token(&state, &access_token) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    let full_path = format!("/{}", path);
    let binding_path = match split_contents_suffix(&path) {
        Some((file_path, _)) => format!("/{}", file_path),
        None => full_path.clone(),
    };
    if let Err(resp) = validate_token_path(&claims, &binding_path) {
        return resp;
    }
    let request_url = headers
        .get("X-Forwarded-Proto")
        .and_then(|v| v.to_str().ok())
        .map(|p| {
            format!(
                "{}://{}",
                p,
                headers.get("host").and_then(|v| v.to_str().ok()).unwrap_or("")
            )
        })
        .map(|authority| format!("{}{}", authority, full_path))
        .unwrap_or_else(|| full_path.clone());
    if let Err(resp) = validate_wopi_proof(&state, &headers, &request_url, access_token.as_deref().unwrap_or("")) {
        return resp;
    }

    let override_header = headers
        .get("X-WOPI-Override")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if let Some((file_path, _)) = split_contents_suffix(&path) {
        // PutFile mutates the document: requires an edit-scoped session.
        if let Err(resp) = validate_edit_scope(&claims) {
            return resp;
        }
        put_file_inner(&state, file_path, &headers, body, &claims).await
    } else if override_header == "LOCK" || override_header == "UNLOCK" {
        if let Err(resp) = validate_edit_scope(&claims) {
            return resp;
        }
        if override_header == "LOCK" {
            lock_file_inner(&state, &path).await
        } else {
            unlock_file_inner(&state, &path, &headers).await
        }
    } else {
        if let Err(resp) = validate_edit_scope(&claims) {
            return resp;
        }
        lock_file_inner(&state, &path).await
    }
}

async fn check_file_info_inner(state: &WopiState, path: &str, claims: &WopiTokenClaims) -> Response {
    let full_path = format!("/{}", path);

    let meta = match state.storage.head(&full_path).await {
        Ok(m) => m,
        Err(_) => return wopi_error(StatusCode::NOT_FOUND, "FILE_NOT_FOUND", "File not found"),
    };

    let file_name = meta.path.rsplit('/').next().unwrap_or("unknown").to_string();

    let response = WopiCheckFileInfoResponse {
        base_file_name: file_name,
        size: meta.size,
        version: meta.etag.trim_matches('"').to_string(),
        last_modified_time: meta.modified_at.to_rfc3339(),
        owner_id: meta.owner,
        // Session capability, not a constant (THM-WOPI-SCOPE).
        user_can_write: claims.can_edit(),
        user_can_not_write_relative: false,
        supports_update: claims.can_edit(),
        supports_locks: claims.can_edit(),
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
            common::metadata::ContentHash::new_unchecked("0".repeat(64)),
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
    claims: &WopiTokenClaims,
) -> Response {
    let full_path = format!("/{}", path);

    if let Some(lock) = state.lock_manager.check_lock(&full_path).await {
        let lock_token = headers.get("X-WOPI-Lock").and_then(|v| v.to_str().ok());
        if let Some(token) = lock_token {
            if lock.token.as_opaque() != token {
                return wopi_error(StatusCode::CONFLICT, "FILE_LOCKED", "File locked by another user");
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

    // The document owner is the token's user — NOT a client-controlled header
    // (the old code took X-WOPI-Override here, which is the operation verb).
    let owner = claims.user.clone();

    match state.storage.put(&full_path, body, &owner).await {
        Ok(_) => {
            info!("WOPI PutFile: {}", full_path);
            (StatusCode::OK, "").into_response()
        }
        Err(e) => wopi_error(StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", e.to_string()),
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

async fn unlock_file_inner(state: &WopiState, path: &str, headers: &axum::http::HeaderMap) -> Response {
    let _full_path = format!("/{}", path);

    let lock_token = headers.get("X-WOPI-LockId").and_then(|v| v.to_str().ok()).unwrap_or("");

    match state.lock_manager.release_lock(lock_token).await {
        Ok(()) => (StatusCode::OK, "").into_response(),
        Err(_) => wopi_error(StatusCode::NOT_FOUND, "NOT_FOUND", "Lock not found"),
    }
}

/// Issue a WOPI access token for `?path=/users/<sub>/<file>`.
///
/// The path rides in the QUERY STRING on purpose: matchit 0.7.3 refuses to
/// match a `:param` segment containing %2F (escaped-slash traversal guard),
/// so a path-segment form could never express nested files.
pub async fn wopi_issue_token(
    Extension(state): Extension<WopiState>,
    Extension(claims): Extension<common::auth::Claims>,
    axum::extract::Query(params): axum::extract::Query<TokenQueryParams>,
) -> Response {
    let path = match params.path {
        Some(p) if !p.is_empty() => p,
        _ => {
            return wopi_error(
                StatusCode::BAD_REQUEST,
                "PATH_REQUIRED",
                "path query parameter is required",
            );
        }
    };
    if state.wopi_token_secret.is_empty() {
        tracing::error!("WOPI token secret is not configured. Set --wopi-token-secret to a strong random value.");
        return wopi_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "WOPI_TOKEN_SECRET_NOT_SET",
            "WOPI token secret is not configured. Set --wopi-token-secret to a strong random value.",
        );
    }

    let full_path = format!("/{}", path.trim_matches('/'));

    if !state.storage.exists(&full_path).await.unwrap_or(false) {
        return wopi_error(
            StatusCode::NOT_FOUND,
            "FILE_NOT_FOUND",
            format!("File not found: {full_path}"),
        );
    }

    let expires = chrono::Utc::now().timestamp() + (8 * 3600);
    // Scope: `edit` (default, backward compatible) or `view`. View sessions get
    // read-only CheckFileInfo and cannot PutFile/LOCK (THM-WOPI-SCOPE).
    let scope = params
        .scope
        .as_deref()
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| s == WOPI_SCOPE_EDIT || s == WOPI_SCOPE_VIEW)
        .unwrap_or_else(|| WOPI_SCOPE_EDIT.to_string());
    let token_payload = serde_json::json!({
        "path": full_path,
        "user": claims.sub,
        "scope": scope,
        "exp": expires,
    });

    let payload_str = serde_json::to_string(&token_payload).unwrap_or_default();
    let mut mac = match Hmac::<Sha256>::new_from_slice(state.wopi_token_secret.as_bytes()) {
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

    let token = base64::engine::general_purpose::STANDARD.encode(format!("{}:{}", payload_str, sig_hex));

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
    use hmac::{Hmac, Mac};

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
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
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
                ContentHash::new_unchecked("0".repeat(64)),
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

        async fn put(&self, path: &str, content: bytes::Bytes, _owner: &str) -> common::error::Result<FileMetadata> {
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

        async fn create_collection(&self, path: &str, _owner: &str) -> common::error::Result<FileMetadata> {
            self.files.write().await.insert(path.to_string(), bytes::Bytes::new());
            self.head(path).await
        }

        async fn list_all(&self, _path: &str, _max_depth: u32) -> common::error::Result<Vec<FileMetadata>> {
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
            self.locks.write().await.insert(path.to_string(), info.clone());
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

        async fn refresh_lock(&self, _token: &str, _timeout_secs: Option<u32>) -> common::error::Result<LockInfo> {
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

        assert_ne!(token_a, token_b, "Tokens signed with different secrets must differ");

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

        let decoded = base64::engine::general_purpose::STANDARD.decode(&token).unwrap();
        let decoded_str = String::from_utf8(decoded).unwrap();
        let sep_idx = decoded_str.rfind(':').unwrap();
        let (recovered_payload, recovered_sig) = decoded_str.split_at(sep_idx);
        let recovered_sig = &recovered_sig[1..];
        assert_eq!(recovered_payload, payload);

        use sha2::Sha256;
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
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
            .put("/hello.txt", bytes::Bytes::from_static(b"hello world"), "owner")
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
            .put("/doc.txt", bytes::Bytes::from_static(b"content here"), "owner")
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
        let lock_body = lock_response.into_body().collect().await.unwrap().to_bytes();
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

    // -----------------------------------------------------------------------
    // WOPI proof verification (YP-COLLAB-WOPI-001, TV-WOPI-001..006)
    // -----------------------------------------------------------------------

    fn proof_headers(
        state: &WopiState,
        url: &str,
        token: &str,
        timestamp_raw: &str,
        key_gen: u8,
        key_override: Option<&str>,
    ) -> axum::http::HeaderMap {
        use base64::Engine;
        let key = match key_override {
            Some(k) => k.as_bytes().to_vec(),
            None => wopi_proof_key(state, key_gen).unwrap(),
        };
        let input = format!("{}\n{}\n{}", url, token, timestamp_raw);
        let mut mac = Hmac::<Sha256>::new_from_slice(&key).unwrap();
        mac.update(input.as_bytes());
        let sig = base64::engine::general_purpose::STANDARD.encode(mac.finalize().into_bytes());
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("X-WOPI-Proof", axum::http::HeaderValue::from_str(&sig).unwrap());
        headers.insert(
            "X-WOPI-TimeStamp",
            axum::http::HeaderValue::from_str(timestamp_raw).unwrap(),
        );
        headers
    }

    #[test]
    fn test_wopi_proof_absent_headers_accepted() {
        let state = make_wopi_state("test-secret");
        let headers = axum::http::HeaderMap::new();
        // Collabora sends no proof headers; token-over-TLS remains the base layer.
        assert!(validate_wopi_proof(&state, &headers, "/wopi/files/x", "tok").is_ok());
    }

    #[test]
    fn test_wopi_proof_half_present_rejected() {
        let state = make_wopi_state("test-secret");
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("X-WOPI-Proof", axum::http::HeaderValue::from_static("AAAA"));
        assert!(validate_wopi_proof(&state, &headers, "/wopi/files/x", "tok").is_err());

        let mut headers2 = axum::http::HeaderMap::new();
        headers2.insert(
            "X-WOPI-TimeStamp",
            axum::http::HeaderValue::from_static("133000000000000000"),
        );
        assert!(validate_wopi_proof(&state, &headers2, "/wopi/files/x", "tok").is_err());
    }

    #[test]
    fn test_wopi_proof_valid_current_and_old_key() {
        let state = make_wopi_state("test-secret");
        let url = "/wopi/files/docs%2Freport.odt";
        let token = "some-access-token";

        // Current key (gen 1), fresh FILETIME timestamp.
        let now_ticks = (chrono::Utc::now().timestamp() + 11_644_473_600) as u64 * 10_000_000;
        let headers = proof_headers(&state, url, token, &now_ticks.to_string(), 1, None);
        assert!(validate_wopi_proof(&state, &headers, url, token).is_ok());

        // Old key (gen 0) also verifies — rotation tolerance.
        let headers_old = proof_headers(&state, url, token, &now_ticks.to_string(), 0, None);
        assert!(validate_wopi_proof(&state, &headers_old, url, token).is_ok());

        // RFC 3339 timestamp form is accepted too.
        let ts_rfc = chrono::Utc::now().to_rfc3339();
        let headers_rfc = proof_headers(&state, url, token, &ts_rfc, 1, None);
        assert!(validate_wopi_proof(&state, &headers_rfc, url, token).is_ok());
    }

    #[test]
    fn test_wopi_proof_wrong_key_rejected() {
        let state = make_wopi_state("test-secret");
        let url = "/wopi/files/x";
        let now_ticks = (chrono::Utc::now().timestamp() + 11_644_473_600) as u64 * 10_000_000;
        // Signed under an unrelated key — must never verify.
        let headers = proof_headers(&state, url, "tok", &now_ticks.to_string(), 1, Some("attacker-key"));
        assert!(validate_wopi_proof(&state, &headers, url, "tok").is_err());
    }

    /// TV-WOPI-002: timestamp freshness boundary (±20 min).
    #[test]
    fn test_wopi_proof_stale_timestamp_rejected() {
        let state = make_wopi_state("test-secret");
        let url = "/wopi/files/x";

        let mk_ticks =
            |age_secs: i64| ((chrono::Utc::now().timestamp() - age_secs) + 11_644_473_600) as u64 * 10_000_000;

        // 19 minutes old: inside the window.
        let fresh = proof_headers(&state, url, "tok", &mk_ticks(19 * 60).to_string(), 1, None);
        assert!(validate_wopi_proof(&state, &fresh, url, "tok").is_ok());

        // 21 minutes old: outside the window.
        let stale = proof_headers(&state, url, "tok", &mk_ticks(21 * 60).to_string(), 1, None);
        assert!(validate_wopi_proof(&state, &stale, url, "tok").is_err());
    }

    /// TV-WOPI-001: proof over a different URL/token fails (binding).
    #[test]
    fn test_wopi_proof_url_binding() {
        let state = make_wopi_state("test-secret");
        let now_ticks = (chrono::Utc::now().timestamp() + 11_644_473_600) as u64 * 10_000_000;
        let headers = proof_headers(&state, "/wopi/files/legit", "tok", &now_ticks.to_string(), 1, None);
        // Same proof replayed against a different URL must fail.
        assert!(validate_wopi_proof(&state, &headers, "/wopi/files/other", "tok").is_err());
        // Same URL, different token must fail.
        assert!(validate_wopi_proof(&state, &headers, "/wopi/files/legit", "other-tok").is_err());
    }

    /// TV-WOPI-005 (THM-WOPI-SCOPE): view-scoped tokens cannot mutate.
    #[test]
    fn test_view_scope_cannot_edit() {
        let claims = WopiTokenClaims {
            path: "/docs/report.odt".into(),
            user: "alice".into(),
            exp: 9999999999,
            scope: WOPI_SCOPE_VIEW.into(),
        };
        assert!(!claims.can_edit());
        assert!(validate_edit_scope(&claims).is_err());

        let edit_claims = WopiTokenClaims {
            scope: WOPI_SCOPE_EDIT.into(),
            ..claims
        };
        assert!(validate_edit_scope(&edit_claims).is_ok());
    }

    /// Token↔path binding: a token issued for one file is useless for another.
    #[test]
    fn test_token_path_binding() {
        let claims = WopiTokenClaims {
            path: "/docs/report.odt".into(),
            user: "alice".into(),
            exp: 9999999999,
            scope: WOPI_SCOPE_EDIT.into(),
        };
        assert!(validate_token_path(&claims, "/docs/report.odt").is_ok());
        assert!(validate_token_path(&claims, "/docs/other.odt").is_err());
        assert!(validate_token_path(&claims, "/docs/report.odt/contents").is_err());
    }

    /// Legacy tokens (no scope field) default to edit — backward compatible.
    #[test]
    fn test_legacy_token_defaults_to_edit_scope() {
        let secret = "test-secret";
        let payload = build_token_payload("/file.txt", "user", 9999999999);
        let token = sign_token(&payload, secret);
        let state = make_wopi_state(secret);
        let claims = validate_access_token(&state, &Some(token)).unwrap();
        assert!(claims.can_edit());
        assert_eq!(claims.path, "/file.txt");
        assert_eq!(claims.user, "user");
    }
}
