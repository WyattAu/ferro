//! WARNING: WebAuthn API endpoints are stubs. They do NOT perform real cryptographic verification and MUST NOT be exposed in production.

use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};

use crate::AppState;
use crate::auth::webauthn::{WebAuthnConfig, WebAuthnCredential};
use base64::Engine;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterBeginRequest {
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterBeginResponse {
    pub challenge: String,
    pub rp_id: String,
    pub rp_name: String,
    pub challenge_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterFinishRequest {
    pub username: String,
    pub credential_id: String,
    pub public_key: String,
    pub device_name: Option<String>,
    pub challenge_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterFinishResponse {
    pub success: bool,
    pub credential_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginBeginRequest {
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginBeginResponse {
    pub challenge: String,
    pub rp_id: String,
    pub challenge_id: String,
    pub credentials: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginFinishRequest {
    pub username: String,
    pub credential_id: String,
    pub challenge_id: String,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginFinishResponse {
    pub success: bool,
    pub token: Option<String>,
}

pub async fn webauthn_register_begin(
    State(state): State<AppState>,
    Json(request): Json<RegisterBeginRequest>,
) -> Json<RegisterBeginResponse> {
    tracing::warn!("WebAuthn stub endpoint called -- NO cryptographic verification performed");
    let config = WebAuthnConfig::default();
    let challenge = format!("stub-challenge-{}", uuid::Uuid::new_v4());
    let challenge_id = challenge.clone();

    {
        let mut store = state.webauthn_store.write().await;
        store.store_registration_challenge(
            &challenge_id,
            &request.username,
            challenge_id.as_bytes().to_vec(),
        );
    }

    Json(RegisterBeginResponse {
        challenge,
        rp_id: config.rp_id,
        rp_name: config.rp_name,
        challenge_id,
    })
}

pub async fn webauthn_register_finish(
    State(state): State<AppState>,
    Json(request): Json<RegisterFinishRequest>,
) -> Json<RegisterFinishResponse> {
    tracing::warn!("WebAuthn stub endpoint called -- NO cryptographic verification performed");
    let mut store = state.webauthn_store.write().await;

    if store
        .consume_registration_challenge(&request.challenge_id, 300)
        .is_err()
    {
        return Json(RegisterFinishResponse {
            success: false,
            credential_id: String::new(),
        });
    }

    let public_key_bytes =
        match base64::engine::general_purpose::STANDARD.decode(&request.public_key) {
            Ok(bytes) => bytes,
            Err(_) => {
                return Json(RegisterFinishResponse {
                    success: false,
                    credential_id: String::new(),
                });
            }
        };

    let now = chrono::Utc::now().timestamp();
    let cred = WebAuthnCredential {
        credential_id: request.credential_id.clone(),
        public_key_cose: public_key_bytes,
        sign_count: 0,
        device_name: request
            .device_name
            .unwrap_or_else(|| "Unknown Device".to_string()),
        registered_at: now,
        last_used_at: now,
        attestation_format: "none".to_string(),
        user_verified: false,
    };

    store.register_credential(&request.credential_id, cred);

    Json(RegisterFinishResponse {
        success: true,
        credential_id: request.credential_id,
    })
}

pub async fn webauthn_login_begin(
    State(state): State<AppState>,
    Json(request): Json<LoginBeginRequest>,
) -> Json<LoginBeginResponse> {
    tracing::warn!("WebAuthn stub endpoint called -- NO cryptographic verification performed");
    let config = WebAuthnConfig::default();
    let challenge = format!("stub-challenge-{}", uuid::Uuid::new_v4());
    let challenge_id = challenge.clone();

    {
        let mut store = state.webauthn_store.write().await;
        store.store_authentication_challenge(
            &challenge_id,
            &request.username,
            challenge_id.as_bytes().to_vec(),
            Vec::new(),
        );
    }

    let store = state.webauthn_store.read().await;
    let creds = store.get_credentials(&request.username);
    let credential_ids: Vec<String> = creds.iter().map(|c| c.credential_id.clone()).collect();

    Json(LoginBeginResponse {
        challenge,
        rp_id: config.rp_id,
        challenge_id,
        credentials: credential_ids,
    })
}

pub async fn webauthn_login_finish(
    State(state): State<AppState>,
    Json(request): Json<LoginFinishRequest>,
) -> Json<LoginFinishResponse> {
    tracing::warn!("WebAuthn stub endpoint called -- NO cryptographic verification performed");
    let mut store = state.webauthn_store.write().await;

    if store
        .consume_authentication_challenge(&request.challenge_id, 300)
        .is_err()
    {
        return Json(LoginFinishResponse {
            success: false,
            token: None,
        });
    }

    let creds = store.get_credentials(&request.username);
    let valid = creds
        .iter()
        .any(|c| c.credential_id == request.credential_id);

    if valid {
        let token = format!("webauthn-session-{}", uuid::Uuid::new_v4());
        Json(LoginFinishResponse {
            success: true,
            token: Some(token),
        })
    } else {
        Json(LoginFinishResponse {
            success: false,
            token: None,
        })
    }
}
