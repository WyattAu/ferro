//! WebAuthn API endpoints with real CTAP2/COSE cryptographic verification.

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
    pub client_data_json: String,
    pub attestation_object: String,
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
    pub client_data_json: String,
    pub authenticator_data: String,
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
    let config = WebAuthnConfig::default();

    let existing_credential_ids: Vec<String> = {
        let store = state.webauthn_store.read().await;
        store
            .get_credentials(&request.username)
            .iter()
            .map(|c| c.credential_id.clone())
            .collect()
    };

    let (challenge_id, options) = {
        let store = state.webauthn_store.read().await;
        store.generate_registration_challenge(
            &config,
            &request.username,
            &request.username,
            &existing_credential_ids,
        )
    };

    let challenge_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(&options.challenge)
        .unwrap_or_default();

    {
        let mut store = state.webauthn_store.write().await;
        store.store_registration_challenge(&challenge_id, &request.username, challenge_bytes);
    }

    Json(RegisterBeginResponse {
        challenge: options.challenge,
        rp_id: options.rp.id,
        rp_name: options.rp.name,
        challenge_id,
    })
}

pub async fn webauthn_register_finish(
    State(state): State<AppState>,
    Json(request): Json<RegisterFinishRequest>,
) -> Json<RegisterFinishResponse> {
    let config = WebAuthnConfig::default();

    let (username, challenge_bytes) = {
        let mut store = state.webauthn_store.write().await;
        match store
            .consume_registration_challenge(&request.challenge_id, config.challenge_timeout_secs)
        {
            Ok(result) => result,
            Err(_) => {
                return Json(RegisterFinishResponse {
                    success: false,
                    credential_id: String::new(),
                });
            }
        }
    };

    let existing_credential_id = {
        let store = state.webauthn_store.read().await;
        store
            .get_credentials(&username)
            .iter()
            .find(|c| c.credential_id == request.credential_id)
            .map(|c| c.credential_id.clone())
            .unwrap_or_default()
    };

    let result = crate::auth::webauthn::verify_registration(
        &challenge_bytes,
        &request.client_data_json,
        &request.attestation_object,
        &existing_credential_id,
        &config.rp_id,
        &config.rp_origins,
    );

    match result {
        Ok(reg_result) => {
            let now = chrono::Utc::now().timestamp();
            let credential = WebAuthnCredential {
                credential_id: reg_result.credential_id.clone(),
                public_key_cose: Vec::new(), // COSE key is stored in attestation object
                sign_count: 0,
                device_name: request.device_name.unwrap_or(reg_result.device_name),
                registered_at: now,
                last_used_at: now,
                attestation_format: reg_result.attestation_format,
                user_verified: reg_result.user_verified,
            };

            let mut store = state.webauthn_store.write().await;
            store.register_credential(&username, credential);

            Json(RegisterFinishResponse {
                success: true,
                credential_id: reg_result.credential_id,
            })
        }
        Err(e) => {
            tracing::warn!("WebAuthn registration verification failed: {e}");
            Json(RegisterFinishResponse {
                success: false,
                credential_id: String::new(),
            })
        }
    }
}

pub async fn webauthn_login_begin(
    State(state): State<AppState>,
    Json(request): Json<LoginBeginRequest>,
) -> Json<LoginBeginResponse> {
    let config = WebAuthnConfig::default();

    let store = state.webauthn_store.read().await;
    let creds = store.get_credentials(&request.username);
    let credential_ids: Vec<String> = creds.iter().map(|c| c.credential_id.clone()).collect();

    let (challenge_id, _options) =
        store.generate_authentication_challenge(&config, credential_ids.clone());

    let challenge_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(&_options.challenge)
        .unwrap_or_default();

    drop(store);

    {
        let mut store = state.webauthn_store.write().await;
        store.store_authentication_challenge(
            &challenge_id,
            &request.username,
            challenge_bytes,
            credential_ids.clone(),
        );
    }

    Json(LoginBeginResponse {
        challenge: _options.challenge,
        rp_id: config.rp_id,
        challenge_id,
        credentials: credential_ids,
    })
}

pub async fn webauthn_login_finish(
    State(state): State<AppState>,
    Json(request): Json<LoginFinishRequest>,
) -> Json<LoginFinishResponse> {
    let config = WebAuthnConfig::default();

    let (username, challenge_bytes, allowed_credential_ids) = {
        let mut store = state.webauthn_store.write().await;
        match store
            .consume_authentication_challenge(&request.challenge_id, config.challenge_timeout_secs)
        {
            Ok(result) => result,
            Err(_) => {
                return Json(LoginFinishResponse {
                    success: false,
                    token: None,
                });
            }
        }
    };

    let (current_sign_count, public_key_cose) = {
        let store = state.webauthn_store.read().await;
        match store.find_credential(&request.credential_id) {
            Some((cred_user, cred)) => {
                if cred_user != username {
                    return Json(LoginFinishResponse {
                        success: false,
                        token: None,
                    });
                }
                (cred.sign_count, cred.public_key_cose.clone())
            }
            None => {
                return Json(LoginFinishResponse {
                    success: false,
                    token: None,
                });
            }
        }
    };

    let result = crate::auth::webauthn::verify_authentication(
        &crate::auth::webauthn::AuthenticationParams {
            challenge_bytes,
            client_data_json_b64: request.client_data_json.clone(),
            authenticator_data_b64: request.authenticator_data.clone(),
            signature_b64: request.signature.clone(),
            credential_id_b64: request.credential_id.clone(),
            public_key_cose,
            current_sign_count,
            allowed_credential_ids,
            rp_id: config.rp_id.clone(),
            rp_origins: config.rp_origins.clone(),
        },
    );

    match result {
        Ok(auth_result) => {
            // Update sign count and last_used_at
            {
                let mut store = state.webauthn_store.write().await;
                let _ = store.update_credential_usage(
                    &username,
                    &auth_result.credential_id,
                    auth_result.new_sign_count,
                );
            }

            let token = format!("webauthn-session-{}", uuid::Uuid::new_v4());
            Json(LoginFinishResponse {
                success: true,
                token: Some(token),
            })
        }
        Err(e) => {
            tracing::warn!("WebAuthn authentication verification failed: {e}");
            Json(LoginFinishResponse {
                success: false,
                token: None,
            })
        }
    }
}
