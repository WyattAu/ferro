use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use rusqlite;
use serde::{Deserialize, Serialize};
use subtle::ConstantTimeEq;

use ferro_auth::totp;

use crate::SecurityAppState;

#[derive(Debug, Deserialize)]
pub struct TotpSetupRequest {
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct TotpSetupResponse {
    pub secret: String,
    pub otpauth_uri: String,
}

#[derive(Debug, Deserialize)]
pub struct TotpVerifyRequest {
    pub password: String,
    pub code: String,
}

#[derive(Debug, Serialize)]
pub struct TotpVerifyResponse {
    pub verified: bool,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TotpStatusResponse {
    pub enabled: bool,
    pub has_secret: bool,
}

fn extract_username<S: SecurityAppState>(state: &S) -> Option<String> {
    state.admin_user().clone()
}

async fn verify_user_password<S: SecurityAppState>(
    state: &S,
    username: &str,
    password: &str,
) -> bool {
    #[allow(clippy::collapsible_if)]
    if let (Some(admin_pw), Some(admin_user)) = (state.admin_password(), state.admin_user()) {
        if username == admin_user.as_str() {
            return password.as_bytes().ct_eq(admin_pw.as_bytes()).into();
        }
    }
    state
        .user_store()
        .authenticate(username, password)
        .await
        .is_ok()
}

async fn get_user_id<S: SecurityAppState>(state: &S, username: &str) -> Option<String> {
    #[allow(clippy::collapsible_if)]
    if let Some(admin_user) = state.admin_user() {
        if username == admin_user.as_str() {
            return state
                .user_store()
                .get_user_by_username_blocking(username)
                .ok()
                .map(|u| u.id);
        }
    }
    state
        .user_store()
        .get_user_by_username_blocking(username)
        .ok()
        .map(|u| u.id)
}

async fn get_totp_secret<S: SecurityAppState>(state: &S, username: &str) -> Option<String> {
    state
        .user_store()
        .get_user_by_username_blocking(username)
        .ok()
        .and_then(|u| u.totp_secret)
}

async fn is_totp_enabled<S: SecurityAppState>(state: &S, username: &str) -> bool {
    state
        .user_store()
        .get_user_by_username_blocking(username)
        .ok()
        .map(|u| u.totp_enabled)
        .unwrap_or(false)
}

pub async fn totp_setup<S: SecurityAppState>(
    State(state): State<S>,
    Json(body): Json<TotpSetupRequest>,
) -> Response {
    let username = match extract_username(&state) {
        Some(u) => u,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(TotpVerifyResponse {
                    verified: false,
                    error: Some("authentication required".to_string()),
                }),
            )
                .into_response();
        }
    };

    if !verify_user_password(&state, &username, &body.password).await {
        return (
            StatusCode::UNAUTHORIZED,
            Json(TotpVerifyResponse {
                verified: false,
                error: Some("invalid password".to_string()),
            }),
        )
            .into_response();
    }

    let secret_bytes = totp::generate_secret();
    let secret_b32 = totp::encode_secret_base32(&secret_bytes);
    let otpauth_uri = totp::generate_otpauth_uri("Ferro", &username, &secret_b32, 6, 30);

    let user_id = match get_user_id(&state, &username).await {
        Some(id) => id,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(TotpVerifyResponse {
                    verified: false,
                    error: Some("user not found".to_string()),
                }),
            )
                .into_response();
        }
    };

    if let Some(db) = state.db() {
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        if let Err(e) = conn.execute(
            "UPDATE users SET totp_secret = ?1 WHERE id = ?2",
            rusqlite::params![secret_b32, user_id],
        ) {
            tracing::warn!(error = %e, "failed to store TOTP secret");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(TotpVerifyResponse {
                    verified: false,
                    error: Some("failed to store TOTP secret".to_string()),
                }),
            )
                .into_response();
        }
    }

    (
        StatusCode::OK,
        Json(TotpSetupResponse {
            secret: secret_b32,
            otpauth_uri,
        }),
    )
        .into_response()
}

pub async fn totp_enable<S: SecurityAppState>(
    State(state): State<S>,
    Json(body): Json<TotpVerifyRequest>,
) -> Response {
    let username = match extract_username(&state) {
        Some(u) => u,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(TotpVerifyResponse {
                    verified: false,
                    error: Some("authentication required".to_string()),
                }),
            )
                .into_response();
        }
    };

    if !verify_user_password(&state, &username, &body.password).await {
        return (
            StatusCode::UNAUTHORIZED,
            Json(TotpVerifyResponse {
                verified: false,
                error: Some("invalid password".to_string()),
            }),
        )
            .into_response();
    }

    let secret_b32 = match get_totp_secret(&state, &username).await {
        Some(s) if !s.is_empty() => s,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(TotpVerifyResponse {
                    verified: false,
                    error: Some("TOTP not set up. Call /api/v1/auth/totp/setup first.".to_string()),
                }),
            )
                .into_response();
        }
    };

    let secret_bytes = match totp::decode_secret_base32(&secret_b32) {
        Some(s) => s,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(TotpVerifyResponse {
                    verified: false,
                    error: Some("invalid TOTP secret encoding".to_string()),
                }),
            )
                .into_response();
        }
    };

    let code: u32 = match body.code.trim().parse() {
        Ok(c) => c,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(TotpVerifyResponse {
                    verified: false,
                    error: Some("invalid TOTP code (must be 6 digits)".to_string()),
                }),
            )
                .into_response();
        }
    };

    let timestamp = chrono::Utc::now().timestamp() as u64;
    if !totp::verify_totp(&secret_bytes, code, timestamp, 6, 30, 0, 1) {
        return (
            StatusCode::BAD_REQUEST,
            Json(TotpVerifyResponse {
                verified: false,
                error: Some("invalid TOTP code".to_string()),
            }),
        )
            .into_response();
    }

    let user_id = match get_user_id(&state, &username).await {
        Some(id) => id,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(TotpVerifyResponse {
                    verified: false,
                    error: Some("user not found".to_string()),
                }),
            )
                .into_response();
        }
    };

    if let Some(db) = state.db() {
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        if let Err(e) = conn.execute(
            "UPDATE users SET totp_enabled = 1 WHERE id = ?1",
            rusqlite::params![user_id],
        ) {
            tracing::warn!(error = %e, "failed to enable TOTP");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(TotpVerifyResponse {
                    verified: false,
                    error: Some("failed to enable TOTP".to_string()),
                }),
            )
                .into_response();
        }
    }

    (
        StatusCode::OK,
        Json(TotpVerifyResponse {
            verified: true,
            error: None,
        }),
    )
        .into_response()
}

pub async fn totp_disable<S: SecurityAppState>(
    State(state): State<S>,
    Json(body): Json<TotpVerifyRequest>,
) -> Response {
    let username = match extract_username(&state) {
        Some(u) => u,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(TotpVerifyResponse {
                    verified: false,
                    error: Some("authentication required".to_string()),
                }),
            )
                .into_response();
        }
    };

    if !verify_user_password(&state, &username, &body.password).await {
        return (
            StatusCode::UNAUTHORIZED,
            Json(TotpVerifyResponse {
                verified: false,
                error: Some("invalid password".to_string()),
            }),
        )
            .into_response();
    }

    let user_id = match get_user_id(&state, &username).await {
        Some(id) => id,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(TotpVerifyResponse {
                    verified: false,
                    error: Some("user not found".to_string()),
                }),
            )
                .into_response();
        }
    };

    if let Some(db) = state.db() {
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        if let Err(e) = conn.execute(
            "UPDATE users SET totp_enabled = 0, totp_secret = NULL WHERE id = ?1",
            rusqlite::params![user_id],
        ) {
            tracing::warn!(error = %e, "failed to disable TOTP");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(TotpVerifyResponse {
                    verified: false,
                    error: Some("failed to disable TOTP".to_string()),
                }),
            )
                .into_response();
        }
    }

    (
        StatusCode::OK,
        Json(TotpVerifyResponse {
            verified: true,
            error: None,
        }),
    )
        .into_response()
}

pub async fn totp_status<S: SecurityAppState>(State(state): State<S>) -> Response {
    let username = match extract_username(&state) {
        Some(u) => u,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(TotpStatusResponse {
                    enabled: false,
                    has_secret: false,
                }),
            )
                .into_response();
        }
    };

    let secret = get_totp_secret(&state, &username).await.unwrap_or_default();
    let enabled = is_totp_enabled(&state, &username).await;

    (
        StatusCode::OK,
        Json(TotpStatusResponse {
            enabled,
            has_secret: !secret.is_empty(),
        }),
    )
        .into_response()
}
