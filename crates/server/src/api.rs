use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use base64::Engine;
use common::auth::Claims;
use serde::Deserialize;

use crate::AppState;
use crate::api_error::ApiError;

/// GET /api/auth/info — return current user info from OIDC claims.
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

    match claims {
        Some(c) => {
            let body = serde_json::json!({
                "sub": c.sub,
                "iss": c.iss,
                "aud": c.aud,
                "email": c.email,
                "name": c.name,
                "groups": c.groups,
                "auth_type": auth_type,
            });
            (StatusCode::OK, axum::Json(body)).into_response()
        }
        None => {
            let body = serde_json::json!({
                "sub": "anonymous",
                "iss": "ferro",
                "aud": "ferro",
                "auth_type": auth_type,
            });
            (StatusCode::OK, axum::Json(body)).into_response()
        }
    }
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
pub async fn auth_login(
    State(state): State<AppState>,
    Query(params): Query<LoginParams>,
) -> Response {
    let oidc = match &state.oidc {
        Some(v) => v,
        None => {
            return ApiError::service_unavailable("NOT_CONFIGURED", "OIDC not configured");
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

/// GET /api/auth/callback — handle OIDC callback.
///
/// Exchanges the authorization code for tokens, validates the ID token,
/// and returns the user info. The frontend can then store the access token
/// for subsequent API calls.
pub async fn auth_callback(
    State(state): State<AppState>,
    Query(params): Query<CallbackParams>,
) -> Response {
    let oidc = match &state.oidc {
        Some(v) => v,
        None => {
            return ApiError::service_unavailable("NOT_CONFIGURED", "OIDC not configured");
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

#[derive(Debug, Deserialize)]
pub struct LoginParams {
    pub redirect: Option<String>,
}

#[derive(Debug, Deserialize)]
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
}
