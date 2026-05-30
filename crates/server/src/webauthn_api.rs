//! WebAuthn/FIDO2 API endpoints.
//!
//! TODO: integrate webauthn-rs crate for actual cryptographic operations

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
    Json(_request): Json<RegisterBeginRequest>,
) -> Json<RegisterBeginResponse> {
    let config = WebAuthnConfig::default();
    let challenge_id = uuid::Uuid::new_v4().to_string();
    let challenge = format!("random-challenge-{}", challenge_id);

    {
        let mut store = state.webauthn_store.write().await;
        store.store_challenge(&challenge_id, &challenge);
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
    let mut store = state.webauthn_store.write().await;

    if store
        .get_and_remove_challenge(&request.challenge_id)
        .is_none()
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
        public_key: public_key_bytes,
        sign_count: 0,
        device_name: request
            .device_name
            .unwrap_or_else(|| "Unknown Device".to_string()),
        registered_at: now,
        last_used_at: now,
    };

    store.register_credential(&request.username, cred);

    Json(RegisterFinishResponse {
        success: true,
        credential_id: request.credential_id,
    })
}

pub async fn webauthn_login_begin(
    State(state): State<AppState>,
    Json(request): Json<LoginBeginRequest>,
) -> Json<LoginBeginResponse> {
    let config = WebAuthnConfig::default();
    let challenge_id = uuid::Uuid::new_v4().to_string();
    let challenge = format!("random-challenge-{}", challenge_id);

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
    let mut store = state.webauthn_store.write().await;

    if store
        .get_and_remove_challenge(&request.challenge_id)
        .is_none()
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
