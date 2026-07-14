use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};

use ferro_auth::webauthn::{
    AuthenticationParams, AuthenticationResult, RegistrationResult, WebAuthnConfig, WebAuthnCredential,
};

use crate::SecurityAppState;
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

async fn get_existing_credential_ids<S: SecurityAppState>(state: &S, username: &str) -> Vec<String> {
    let store = state.webauthn_store().read().await;
    store
        .get_credentials(username)
        .iter()
        .map(|c| c.credential_id.clone())
        .collect()
}

async fn begin_registration(
    state: &S,
    config: &WebAuthnConfig,
    username: &str,
    existing_ids: &[String],
) -> (String, ferro_auth::webauthn::RegistrationOptions) {
    let store = state.webauthn_store().read().await;
    store.generate_registration_challenge(config, username, username, existing_ids)
}

fn decode_challenge(challenge_b64: &str) -> Vec<u8> {
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(challenge_b64)
        .unwrap_or_default()
}

async fn save_registration_challenge<S: SecurityAppState>(
    state: &S,
    challenge_id: &str,
    username: &str,
    challenge_bytes: Vec<u8>,
) {
    let mut store = state.webauthn_store().write().await;
    store.store_registration_challenge(challenge_id, username, challenge_bytes);
}

async fn prepare_registration<S: SecurityAppState>(
    state: &S,
    challenge_id: &str,
    credential_id: &str,
    timeout_secs: u64,
) -> Option<(String, Vec<u8>, String)> {
    let (username, challenge_bytes) = {
        let mut store = state.webauthn_store().write().await;
        store.consume_registration_challenge(challenge_id, timeout_secs).ok()?
    };
    let existing_credential_id = {
        let store = state.webauthn_store().read().await;
        store
            .get_credentials(&username)
            .iter()
            .find(|c| c.credential_id == credential_id)
            .map(|c| c.credential_id.clone())
            .unwrap_or_default()
    };
    Some((username, challenge_bytes, existing_credential_id))
}

fn verify_registration_call(
    challenge_bytes: &[u8],
    client_data_json_b64: &str,
    attestation_object_b64: &str,
    existing_credential_id: &str,
    rp_id: &str,
    rp_origins: &[String],
) -> Result<RegistrationResult, ferro_auth::webauthn::WebAuthnError> {
    ferro_auth::webauthn::verify_registration(
        challenge_bytes,
        client_data_json_b64,
        attestation_object_b64,
        existing_credential_id,
        rp_id,
        rp_origins,
    )
}

async fn store_credential<S: SecurityAppState>(state: &S, username: &str, credential: WebAuthnCredential) {
    let mut store = state.webauthn_store().write().await;
    store.register_credential(username, credential);
}

async fn handle_registration_result<S: SecurityAppState>(
    state: &S,
    username: &str,
    device_name: Option<String>,
    reg_result: RegistrationResult,
) -> Json<RegisterFinishResponse> {
    let now = chrono::Utc::now().timestamp();
    let credential = WebAuthnCredential {
        credential_id: reg_result.credential_id.clone(),
        public_key_cose: Vec::new(),
        sign_count: 0,
        device_name: device_name.unwrap_or(reg_result.device_name),
        registered_at: now,
        last_used_at: now,
        attestation_format: reg_result.attestation_format,
        user_verified: reg_result.user_verified,
    };
    store_credential(state, username, credential).await;
    Json(RegisterFinishResponse {
        success: true,
        credential_id: reg_result.credential_id,
    })
}

async fn get_allowed_credentials<S: SecurityAppState>(state: &S, username: &str) -> Vec<String> {
    let store = state.webauthn_store().read().await;
    store
        .get_credentials(username)
        .iter()
        .map(|c| c.credential_id.clone())
        .collect()
}

async fn begin_authentication(
    state: &S,
    config: &WebAuthnConfig,
    credential_ids: Vec<String>,
) -> (String, ferro_auth::webauthn::AuthenticationOptions) {
    let store = state.webauthn_store().read().await;
    store.generate_authentication_challenge(config, credential_ids)
}

async fn save_auth_challenge<S: SecurityAppState>(
    state: &S,
    challenge_id: &str,
    username: &str,
    challenge_bytes: Vec<u8>,
    allowed_credential_ids: Vec<String>,
) {
    let mut store = state.webauthn_store().write().await;
    store.store_authentication_challenge(challenge_id, username, challenge_bytes, allowed_credential_ids);
}

async fn prepare_authentication<S: SecurityAppState>(
    state: &S,
    challenge_id: &str,
    credential_id: &str,
    timeout_secs: u64,
) -> Option<(String, Vec<u8>, Vec<String>, u32, Vec<u8>)> {
    let (username, challenge_bytes, allowed_credential_ids) = {
        let mut store = state.webauthn_store().write().await;
        store
            .consume_authentication_challenge(challenge_id, timeout_secs)
            .ok()?
    };
    let (current_sign_count, public_key_cose) = {
        let store = state.webauthn_store().read().await;
        match store.find_credential(credential_id) {
            Some((cred_user, cred)) if cred_user == username => (cred.sign_count, cred.public_key_cose.clone()),
            _ => return None,
        }
    };
    Some((
        username,
        challenge_bytes,
        allowed_credential_ids,
        current_sign_count,
        public_key_cose,
    ))
}

fn verify_authentication_call(
    params: &AuthenticationParams,
) -> Result<AuthenticationResult, ferro_auth::webauthn::WebAuthnError> {
    ferro_auth::webauthn::verify_authentication(params)
}

async fn update_credential_usage<S: SecurityAppState>(
    state: &S,
    username: &str,
    credential_id: &str,
    new_sign_count: u32,
) {
    let mut store = state.webauthn_store().write().await;
    let _ = store.update_credential_usage(username, credential_id, new_sign_count);
}

async fn handle_auth_success<S: SecurityAppState>(
    state: &S,
    username: &str,
    auth_result: AuthenticationResult,
) -> Json<LoginFinishResponse> {
    update_credential_usage(state, username, &auth_result.credential_id, auth_result.new_sign_count).await;
    let token = format!("webauthn-session-{}", uuid::Uuid::new_v4());
    Json(LoginFinishResponse {
        success: true,
        token: Some(token),
    })
}

pub async fn webauthn_register_begin<S: SecurityAppState>(
    State(state): State<S>,
    Json(request): Json<RegisterBeginRequest>,
) -> Json<RegisterBeginResponse> {
    let config = WebAuthnConfig::default();
    let existing_ids = get_existing_credential_ids(&state, &request.username).await;
    let (challenge_id, options) = begin_registration(&state, &config, &request.username, &existing_ids).await;
    let challenge_bytes = decode_challenge(&options.challenge);
    save_registration_challenge(&state, &challenge_id, &request.username, challenge_bytes).await;

    Json(RegisterBeginResponse {
        challenge: options.challenge,
        rp_id: options.rp.id,
        rp_name: options.rp.name,
        challenge_id,
    })
}

pub async fn webauthn_register_finish<S: SecurityAppState>(
    State(state): State<S>,
    Json(request): Json<RegisterFinishRequest>,
) -> Json<RegisterFinishResponse> {
    let config = WebAuthnConfig::default();
    let Some((username, challenge_bytes, existing_credential_id)) = prepare_registration(
        &state,
        &request.challenge_id,
        &request.credential_id,
        config.challenge_timeout_secs,
    )
    .await
    else {
        return Json(RegisterFinishResponse {
            success: false,
            credential_id: String::new(),
        });
    };
    let result = verify_registration_call(
        &challenge_bytes,
        &request.client_data_json,
        &request.attestation_object,
        &existing_credential_id,
        &config.rp_id,
        &config.rp_origins,
    );

    match result {
        Ok(reg_result) => handle_registration_result(&state, &username, request.device_name, reg_result).await,
        Err(e) => {
            tracing::warn!("WebAuthn registration verification failed: {e}");
            Json(RegisterFinishResponse {
                success: false,
                credential_id: String::new(),
            })
        }
    }
}

pub async fn webauthn_login_begin<S: SecurityAppState>(
    State(state): State<S>,
    Json(request): Json<LoginBeginRequest>,
) -> Json<LoginBeginResponse> {
    let config = WebAuthnConfig::default();
    let allowed_creds = get_allowed_credentials(&state, &request.username).await;
    let (challenge_id, options) = begin_authentication(&state, &config, allowed_creds.clone()).await;
    let challenge_bytes = decode_challenge(&options.challenge);
    save_auth_challenge(
        &state,
        &challenge_id,
        &request.username,
        challenge_bytes,
        allowed_creds.clone(),
    )
    .await;

    Json(LoginBeginResponse {
        challenge: options.challenge,
        rp_id: config.rp_id,
        challenge_id,
        credentials: allowed_creds,
    })
}

pub async fn webauthn_login_finish<S: SecurityAppState>(
    State(state): State<S>,
    Json(request): Json<LoginFinishRequest>,
) -> Json<LoginFinishResponse> {
    let config = WebAuthnConfig::default();
    let Some((username, challenge_bytes, allowed_credential_ids, current_sign_count, public_key_cose)) =
        prepare_authentication(
            &state,
            &request.challenge_id,
            &request.credential_id,
            config.challenge_timeout_secs,
        )
        .await
    else {
        return Json(LoginFinishResponse {
            success: false,
            token: None,
        });
    };

    let params = AuthenticationParams {
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
    };
    let result = verify_authentication_call(&params);

    match result {
        Ok(auth_result) => handle_auth_success(&state, &username, auth_result).await,
        Err(e) => {
            tracing::warn!("WebAuthn authentication verification failed: {e}");
            Json(LoginFinishResponse {
                success: false,
                token: None,
            })
        }
    }
}
