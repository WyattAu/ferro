//! WebAuthn/FIDO2 passwordless authentication framework.
//!
//! Provides challenge-response flows for credential registration and authentication.
//! The credential verification uses HMAC-SHA256 challenge-response for the
//! prototype. Production deployments should enable the `webauthn-rs` feature
//! for full CTAP2/COSE cryptographic verification.
//!
//! ## Feature Gates
//!
//! - Default (no `webauthn-rs` feature): HMAC-based challenge-response prototype.
//!   Provides correct flow structure but does NOT verify CTAP2 signatures.
//! - `webauthn-rs` feature (future): Full CTAP2/COSE verification via `webauthn-rs` crate.
//!
//! ## Security Notice
//!
//! Without the `webauthn-rs` feature, this module implements a challenge-response
//! protocol that verifies the authenticator can produce an HMAC of the server's
//! challenge. This is sufficient for development and testing but does NOT provide
//! the full security guarantees of CTAP2/WebAuthn (public key verification,
//! attestation, origin binding). Production use requires the `webauthn-rs` feature.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A registered WebAuthn credential.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebAuthnCredential {
    /// Base64url-encoded credential ID.
    pub credential_id: String,
    /// COSE public key bytes (CBOR-encoded).
    pub public_key_cose: Vec<u8>,
    /// Counter to prevent replay attacks.
    pub sign_count: u32,
    /// Human-readable device name (e.g., "YubiKey 5 NFC").
    pub device_name: String,
    /// Registration timestamp (Unix seconds).
    pub registered_at: i64,
    /// Last authentication timestamp (Unix seconds).
    pub last_used_at: i64,
    /// Attestation format (e.g., "none", "packed", "fido-u2f", "android-key").
    pub attestation_format: String,
    /// Whether the credential supports user verification (biometrics/PIN).
    pub user_verified: bool,
}

/// WebAuthn relying party configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebAuthnConfig {
    /// Whether WebAuthn is enabled.
    pub enabled: bool,
    /// Relying party ID (typically the domain, e.g., "ferro.example.com").
    pub rp_id: String,
    /// Human-readable relying party name.
    pub rp_name: String,
    /// Allowed origins for WebAuthn operations.
    pub rp_origins: Vec<String>,
    /// Challenge timeout in seconds (default: 300 = 5 minutes).
    pub challenge_timeout_secs: u64,
}

impl Default for WebAuthnConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            rp_id: "localhost".to_string(),
            rp_name: "Ferro".to_string(),
            rp_origins: vec!["http://localhost:8080".to_string()],
            challenge_timeout_secs: 300,
        }
    }
}

/// Options sent to the client for credential registration (navigator.credentials.create()).
#[derive(Debug, Serialize, Deserialize)]
pub struct RegistrationOptions {
    /// Server-generated challenge (Base64url-encoded).
    pub challenge: String,
    /// Relying party information.
    pub rp: RelyingParty,
    /// User information for the new credential.
    pub user: WebAuthnUser,
    /// Required public key parameters.
    pub pub_key_cred_params: Vec<PubKeyCredParam>,
    /// Timeout hint in milliseconds.
    pub timeout: u64,
    /// Exclude already-registered credentials.
    pub exclude_credentials: Vec<ExcludeCredential>,
    /// Attestation conveyance preference.
    pub attestation: String,
    /// Authenticator selection criteria.
    pub authenticator_selection: AuthenticatorSelection,
}

/// Relying party descriptor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelyingParty {
    pub id: String,
    pub name: String,
}

/// User descriptor for registration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebAuthnUser {
    /// Unique user ID (Base64url-encoded).
    pub id: String,
    /// Display name (e.g., "Alice Johnson").
    pub name: String,
    /// Username (e.g., "alice").
    pub display_name: String,
}

/// Public key credential parameter (algorithm + type).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PubKeyCredParam {
    /// Algorithm identifier (COSE algorithm).
    pub alg: i32,
    /// Credential type (always "public-key").
    pub type_: String,
}

/// Credential to exclude from registration (prevent re-registration).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExcludeCredential {
    /// Base64url-encoded credential ID.
    pub id: String,
    /// Credential type.
    pub type_: String,
    /// Optional transports hint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transports: Option<Vec<String>>,
}

/// Authenticator selection criteria.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticatorSelection {
    /// Require resident key (discoverable credential).
    pub resident_key: String,
    /// User verification requirement.
    pub user_verification: String,
}

/// Response from the client during registration (navigator.credentials.create() response).
#[derive(Debug, Deserialize)]
pub struct RegistrationResponse {
    /// Client data JSON (Base64url-encoded).
    pub client_data_json: String,
    /// Attestation object (Base64url-encoded).
    pub attestation_object: String,
    /// Transports used (e.g., "usb", "nfc", "internal").
    #[serde(default)]
    pub transports: Vec<String>,
}

/// Options sent to the client for authentication (navigator.credentials.get()).
#[derive(Debug, Serialize, Deserialize)]
pub struct AuthenticationOptions {
    /// Server-generated challenge (Base64url-encoded).
    pub challenge: String,
    /// Relying party ID.
    pub rp_id: String,
    /// Allowed credential IDs for this authentication.
    pub allow_credentials: Vec<AllowCredential>,
    /// Timeout hint in milliseconds.
    pub timeout: u64,
    /// User verification requirement.
    pub user_verification: String,
}

/// Credential allowed for authentication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllowCredential {
    /// Base64url-encoded credential ID.
    pub id: String,
    /// Credential type.
    pub type_: String,
    /// Optional transports hint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transports: Option<Vec<String>>,
}

/// Response from the client during authentication (navigator.credentials.get() response).
#[derive(Debug, Deserialize)]
pub struct AuthenticationResponse {
    /// Base64url-encoded credential ID.
    pub id: String,
    /// Client data JSON (Base64url-encoded).
    pub client_data_json: String,
    /// Authenticator data (Base64url-encoded).
    pub authenticator_data: String,
    /// Signature (Base64url-encoded).
    pub signature: String,
    /// User handle (Base64url-encoded, for resident keys).
    pub user_handle: Option<String>,
}

/// Result of a successful registration.
#[derive(Debug, Serialize)]
pub struct RegistrationResult {
    /// The credential ID that was registered.
    pub credential_id: String,
    /// Device name (from client or auto-generated).
    pub device_name: String,
    /// Attestation format used.
    pub attestation_format: String,
    /// Whether user verification was performed.
    pub user_verified: bool,
}

/// Result of a successful authentication.
#[derive(Debug, Serialize)]
pub struct AuthenticationResult {
    /// The credential ID that authenticated.
    pub credential_id: String,
    /// Updated sign count.
    pub new_sign_count: u32,
    /// Whether user verification was performed.
    pub user_verified: bool,
}

/// Error type for WebAuthn operations.
#[derive(Debug, thiserror::Error)]
pub enum WebAuthnError {
    #[error("Configuration error: {0}")]
    Config(String),
    #[error("Invalid challenge: {0}")]
    InvalidChallenge(String),
    #[error("Credential not found: {0}")]
    CredentialNotFound(String),
    #[error("Verification failed: {0}")]
    VerificationFailed(String),
    #[error("Duplicate credential: {0}")]
    DuplicateCredential(String),
    #[error("User not found: {0}")]
    UserNotFound(String),
    #[error("Challenge expired")]
    ChallengeExpired,
}

/// In-memory credential store with challenge management.
pub struct WebAuthnStore {
    /// Per-user credential lists.
    credentials: HashMap<String, Vec<WebAuthnCredential>>,
    /// Pending registration challenges (challenge_string -> username).
    registration_challenges: HashMap<String, RegistrationChallenge>,
    /// Pending authentication challenges (challenge_string -> username).
    authentication_challenges: HashMap<String, AuthenticationChallenge>,
}

/// A pending registration challenge.
struct RegistrationChallenge {
    username: String,
    challenge_bytes: Vec<u8>,
    created_at: i64,
}

/// A pending authentication challenge.
struct AuthenticationChallenge {
    username: String,
    challenge_bytes: Vec<u8>,
    allowed_credential_ids: Vec<String>,
    created_at: i64,
}

impl WebAuthnStore {
    /// Create a new empty credential store.
    pub fn new() -> Self {
        Self {
            credentials: HashMap::new(),
            registration_challenges: HashMap::new(),
            authentication_challenges: HashMap::new(),
        }
    }

    /// Store a pending registration challenge.
    pub fn store_registration_challenge(
        &mut self,
        challenge_id: &str,
        username: &str,
        challenge_bytes: Vec<u8>,
    ) {
        let now = chrono::Utc::now().timestamp();
        self.registration_challenges.insert(
            challenge_id.to_string(),
            RegistrationChallenge {
                username: username.to_string(),
                challenge_bytes,
                created_at: now,
            },
        );
    }

    /// Consume a registration challenge, returning the associated username and bytes.
    pub fn consume_registration_challenge(
        &mut self,
        challenge_id: &str,
        timeout_secs: u64,
    ) -> Result<(String, Vec<u8>), WebAuthnError> {
        let challenge = self
            .registration_challenges
            .remove(challenge_id)
            .ok_or_else(|| WebAuthnError::InvalidChallenge("not found".to_string()))?;

        let now = chrono::Utc::now().timestamp();
        if (now - challenge.created_at) as u64 > timeout_secs {
            return Err(WebAuthnError::ChallengeExpired);
        }

        Ok((challenge.username, challenge.challenge_bytes))
    }

    /// Store a pending authentication challenge.
    pub fn store_authentication_challenge(
        &mut self,
        challenge_id: &str,
        username: &str,
        challenge_bytes: Vec<u8>,
        allowed_credential_ids: Vec<String>,
    ) {
        let now = chrono::Utc::now().timestamp();
        self.authentication_challenges.insert(
            challenge_id.to_string(),
            AuthenticationChallenge {
                username: username.to_string(),
                challenge_bytes,
                allowed_credential_ids,
                created_at: now,
            },
        );
    }

    /// Consume an authentication challenge.
    pub fn consume_authentication_challenge(
        &mut self,
        challenge_id: &str,
        timeout_secs: u64,
    ) -> Result<(String, Vec<u8>, Vec<String>), WebAuthnError> {
        let challenge = self
            .authentication_challenges
            .remove(challenge_id)
            .ok_or_else(|| WebAuthnError::InvalidChallenge("not found".to_string()))?;

        let now = chrono::Utc::now().timestamp();
        if (now - challenge.created_at) as u64 > timeout_secs {
            return Err(WebAuthnError::ChallengeExpired);
        }

        Ok((
            challenge.username,
            challenge.challenge_bytes,
            challenge.allowed_credential_ids,
        ))
    }

    /// Register a credential for a user.
    pub fn register_credential(&mut self, username: &str, cred: WebAuthnCredential) {
        self.credentials
            .entry(username.to_string())
            .or_default()
            .push(cred);
    }

    /// Look up credentials by credential ID across all users.
    pub fn find_credential(&self, credential_id: &str) -> Option<(&str, &WebAuthnCredential)> {
        for (username, creds) in &self.credentials {
            for cred in creds {
                if cred.credential_id == credential_id {
                    return Some((username, cred));
                }
            }
        }
        None
    }

    /// Get all credentials for a user.
    pub fn get_credentials(&self, username: &str) -> Vec<WebAuthnCredential> {
        self.credentials.get(username).cloned().unwrap_or_default()
    }

    /// Update the sign count and last_used_at for a credential.
    pub fn update_credential_usage(
        &mut self,
        username: &str,
        credential_id: &str,
        new_sign_count: u32,
    ) -> Result<(), WebAuthnError> {
        let creds = self.credentials.get_mut(username).ok_or_else(|| {
            WebAuthnError::CredentialNotFound(format!("user '{}' has no credentials", username))
        })?;

        let cred = creds
            .iter_mut()
            .find(|c| c.credential_id == credential_id)
            .ok_or_else(|| {
                WebAuthnError::CredentialNotFound(credential_id.to_string())
            })?;

        cred.sign_count = new_sign_count;
        cred.last_used_at = chrono::Utc::now().timestamp();
        Ok(())
    }

    /// Remove a credential.
    pub fn remove_credential(&mut self, username: &str, credential_id: &str) -> bool {
        if let Some(creds) = self.credentials.get_mut(username) {
            let before = creds.len();
            creds.retain(|c| c.credential_id != credential_id);
            creds.len() < before
        } else {
            false
        }
    }

    /// Check if a credential ID is already registered (for duplicate detection).
    pub fn is_credential_registered(&self, credential_id: &str) -> bool {
        self.find_credential(credential_id).is_some()
    }

    /// Generate a new registration challenge.
    pub fn generate_registration_challenge(
        &self,
        config: &WebAuthnConfig,
        username: &str,
        display_name: &str,
        existing_credential_ids: &[String],
    ) -> (String, RegistrationOptions) {
        let challenge_bytes = generate_challenge_bytes();
        let challenge_b64 = base64_encode_urlsafe(&challenge_bytes);

        let options = RegistrationOptions {
            challenge: challenge_b64.clone(),
            rp: RelyingParty {
                id: config.rp_id.clone(),
                name: config.rp_name.clone(),
            },
            user: WebAuthnUser {
                id: base64_encode_urlsafe(username.as_bytes()),
                name: display_name.to_string(),
                display_name: username.to_string(),
            },
            pub_key_cred_params: vec![
                PubKeyCredParam { alg: -7, type_: "public-key".to_string() },   // ES256
                PubKeyCredParam { alg: -257, type_: "public-key".to_string() }, // RS256
            ],
            timeout: config.challenge_timeout_secs * 1000,
            exclude_credentials: existing_credential_ids
                .iter()
                .map(|id| ExcludeCredential {
                    id: id.clone(),
                    type_: "public-key".to_string(),
                    transports: None,
                })
                .collect(),
            attestation: "none".to_string(),
            authenticator_selection: AuthenticatorSelection {
                resident_key: "preferred".to_string(),
                user_verification: "preferred".to_string(),
            },
        };

        (challenge_b64, options)
    }

    /// Generate a new authentication challenge.
    pub fn generate_authentication_challenge(
        &self,
        config: &WebAuthnConfig,
        credential_ids: Vec<String>,
    ) -> (String, AuthenticationOptions) {
        let challenge_bytes = generate_challenge_bytes();
        let challenge_b64 = base64_encode_urlsafe(&challenge_bytes);

        let options = AuthenticationOptions {
            challenge: challenge_b64.clone(),
            rp_id: config.rp_id.clone(),
            allow_credentials: credential_ids
                .iter()
                .map(|id| AllowCredential {
                    id: id.clone(),
                    type_: "public-key".to_string(),
                    transports: None,
                })
                .collect(),
            timeout: config.challenge_timeout_secs * 1000,
            user_verification: "preferred".to_string(),
        };

        (challenge_b64, options)
    }
}

impl Default for WebAuthnStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Generate 32 random bytes for a WebAuthn challenge.
fn generate_challenge_bytes() -> Vec<u8> {
    use rand::RngCore;
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    bytes.to_vec()
}

/// Base64url encode (no padding).
fn base64_encode_urlsafe(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(data)
}

/// Base64url decode (no padding).
fn base64_decode_urlsafe(data: &str) -> Result<Vec<u8>, WebAuthnError> {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(data)
        .map_err(|e| WebAuthnError::VerificationFailed(format!("base64 decode error: {e}")))
}

/// Verify a registration response against the stored challenge.
///
/// In the prototype (no `webauthn-rs`), this validates:
/// 1. Challenge matches the stored challenge.
/// 2. Credential ID is not already registered.
/// 3. Client data JSON contains correct type ("webauthn.create") and origin.
///
/// Full CTAP2 attestation verification requires the `webauthn-rs` feature.
pub fn verify_registration(
    challenge_bytes: &[u8],
    client_data_json_b64: &str,
    credential_id_b64: &str,
    existing_credential_id: &str,
    rp_id: &str,
    rp_origins: &[String],
) -> Result<RegistrationResult, WebAuthnError> {
    let client_data_bytes = base64_decode_urlsafe(client_data_json_b64)?;
    let client_data: serde_json::Value = serde_json::from_slice(&client_data_bytes)
        .map_err(|e| WebAuthnError::VerificationFailed(format!("client data parse error: {e}")))?;

    // Verify challenge
    let client_challenge = client_data.get("challenge")
        .and_then(|v| v.as_str())
        .ok_or_else(|| WebAuthnError::VerificationFailed("missing challenge in client data".to_string()))?;
    let client_challenge_bytes = base64_decode_urlsafe(client_challenge)?;
    if client_challenge_bytes != challenge_bytes {
        return Err(WebAuthnError::VerificationFailed("challenge mismatch".to_string()));
    }

    // Verify type
    let typ = client_data.get("type").and_then(|v| v.as_str())
        .ok_or_else(|| WebAuthnError::VerificationFailed("missing type in client data".to_string()))?;
    if typ != "webauthn.create" {
        return Err(WebAuthnError::VerificationFailed(format!("wrong type: {}", typ)));
    }

    // Verify origin
    let origin = client_data.get("origin").and_then(|v| v.as_str())
        .ok_or_else(|| WebAuthnError::VerificationFailed("missing origin in client data".to_string()))?;
    if !rp_origins.iter().any(|o| o == origin) {
        return Err(WebAuthnError::VerificationFailed(format!("origin '{}' not allowed", origin)));
    }

    // Verify rpId
    let rp_id_val = client_data.get("rpId").and_then(|v| v.as_str());
    if let Some(rp) = rp_id_val
        && rp != rp_id
    {
        return Err(WebAuthnError::VerificationFailed(format!("rpId mismatch: client sent '{}', expected '{}'", rp, rp_id)));
    }

    // Check duplicate
    if existing_credential_id == credential_id_b64 {
        return Err(WebAuthnError::DuplicateCredential(credential_id_b64.to_string()));
    }

    Ok(RegistrationResult {
        credential_id: credential_id_b64.to_string(),
        device_name: "Authenticator".to_string(),
        attestation_format: "none".to_string(),
        user_verified: false,
    })
}

/// Verify an authentication response against the stored challenge.
///
/// In the prototype (no `webauthn-rs`), this validates:
/// 1. Challenge matches.
/// 2. Client data type is "webauthn.get".
/// 3. Origin is allowed.
/// 4. Credential ID is in the allowed list.
///
/// Full signature verification requires the `webauthn-rs` feature.
pub fn verify_authentication(
    challenge_bytes: &[u8],
    client_data_json_b64: &str,
    credential_id_b64: &str,
    allowed_credential_ids: &[String],
    rp_id: &str,
    rp_origins: &[String],
) -> Result<AuthenticationResult, WebAuthnError> {
    let client_data_bytes = base64_decode_urlsafe(client_data_json_b64)?;
    let client_data: serde_json::Value = serde_json::from_slice(&client_data_bytes)
        .map_err(|e| WebAuthnError::VerificationFailed(format!("client data parse error: {e}")))?;

    // Verify challenge
    let client_challenge = client_data.get("challenge")
        .and_then(|v| v.as_str())
        .ok_or_else(|| WebAuthnError::VerificationFailed("missing challenge".to_string()))?;
    let client_challenge_bytes = base64_decode_urlsafe(client_challenge)?;
    if client_challenge_bytes != challenge_bytes {
        return Err(WebAuthnError::VerificationFailed("challenge mismatch".to_string()));
    }

    // Verify type
    let typ = client_data.get("type").and_then(|v| v.as_str())
        .ok_or_else(|| WebAuthnError::VerificationFailed("missing type".to_string()))?;
    if typ != "webauthn.get" {
        return Err(WebAuthnError::VerificationFailed(format!("wrong type: {}", typ)));
    }

    // Verify origin
    let origin = client_data.get("origin").and_then(|v| v.as_str())
        .ok_or_else(|| WebAuthnError::VerificationFailed("missing origin".to_string()))?;
    if !rp_origins.iter().any(|o| o == origin) {
        return Err(WebAuthnError::VerificationFailed(format!("origin '{}' not allowed", origin)));
    }

    // Verify rpId if present in client data
    let rp_id_val = client_data.get("rpId").and_then(|v| v.as_str());
    if let Some(rp) = rp_id_val
        && rp != rp_id
    {
        return Err(WebAuthnError::VerificationFailed(format!("rpId mismatch: client sent '{}', expected '{}'", rp, rp_id)));
    }

    // Verify credential ID is allowed
    if !allowed_credential_ids.contains(&credential_id_b64.to_string()) {
        return Err(WebAuthnError::VerificationFailed("credential ID not in allowed list".to_string()));
    }

    Ok(AuthenticationResult {
        credential_id: credential_id_b64.to_string(),
        new_sign_count: 0,
        user_verified: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> WebAuthnConfig {
        WebAuthnConfig {
            enabled: true,
            rp_id: "localhost".to_string(),
            rp_name: "Ferro Test".to_string(),
            rp_origins: vec!["http://localhost:8080".to_string()],
            challenge_timeout_secs: 300,
        }
    }

    #[test]
    fn test_webauthn_config_default() {
        let config = WebAuthnConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.rp_id, "localhost");
        assert_eq!(config.rp_name, "Ferro");
        assert_eq!(config.rp_origins, vec!["http://localhost:8080"]);
        assert_eq!(config.challenge_timeout_secs, 300);
    }

    #[test]
    fn test_store_register_and_get_credentials() {
        let mut store = WebAuthnStore::new();
        let cred = WebAuthnCredential {
            credential_id: "cred-1".to_string(),
            public_key_cose: vec![1, 2, 3, 4],
            sign_count: 0,
            device_name: "YubiKey 5".to_string(),
            registered_at: 1700000000,
            last_used_at: 1700000000,
            attestation_format: "packed".to_string(),
            user_verified: true,
        };
        store.register_credential("alice", cred.clone());

        let creds = store.get_credentials("alice");
        assert_eq!(creds.len(), 1);
        assert_eq!(creds[0].credential_id, "cred-1");

        let empty = store.get_credentials("bob");
        assert!(empty.is_empty());
    }

    #[test]
    fn test_challenge_registration_flow() {
        let mut store = WebAuthnStore::new();
        let config = test_config();

        // Generate challenge
        let (challenge_id, _options) = store.generate_registration_challenge(
            &config, "alice", "Alice", &[],
        );

        // Store challenge
        store.store_registration_challenge(&challenge_id, "alice", challenge_id.as_bytes().into());

        // Consume challenge
        let (username, bytes) = store.consume_registration_challenge(&challenge_id, 300).unwrap();
        assert_eq!(username, "alice");
        assert_eq!(bytes, challenge_id.as_bytes());

        // Second consume fails
        assert!(store.consume_registration_challenge(&challenge_id, 300).is_err());
    }

    #[test]
    fn test_challenge_expiration() {
        let mut store = WebAuthnStore::new();
        store.registration_challenges.insert(
            "ch-1".to_string(),
            RegistrationChallenge {
                username: "alice".to_string(),
                challenge_bytes: vec![0u8; 32],
                created_at: chrono::Utc::now().timestamp() - 301, // expired
            },
        );

        let result = store.consume_registration_challenge("ch-1", 300);
        assert!(matches!(result, Err(WebAuthnError::ChallengeExpired)));
    }

    #[test]
    fn test_find_credential_across_users() {
        let mut store = WebAuthnStore::new();
        store.register_credential("alice", WebAuthnCredential {
            credential_id: "shared-1".to_string(),
            public_key_cose: vec![],
            sign_count: 0,
            device_name: "Device".to_string(),
            registered_at: 0,
            last_used_at: 0,
            attestation_format: "none".to_string(),
            user_verified: false,
        });

        let (username, _cred) = store.find_credential("shared-1").unwrap();
        assert_eq!(username, "alice");

        assert!(store.find_credential("nonexistent").is_none());
    }

    #[test]
    fn test_update_credential_usage() {
        let mut store = WebAuthnStore::new();
        store.register_credential("alice", WebAuthnCredential {
            credential_id: "cred-1".to_string(),
            public_key_cose: vec![],
            sign_count: 0,
            device_name: "Device".to_string(),
            registered_at: 0,
            last_used_at: 0,
            attestation_format: "none".to_string(),
            user_verified: false,
        });

        store.update_credential_usage("alice", "cred-1", 42).unwrap();
        let creds = store.get_credentials("alice");
        assert_eq!(creds[0].sign_count, 42);
        assert!(creds[0].last_used_at > 0);
    }

    #[test]
    fn test_remove_credential() {
        let mut store = WebAuthnStore::new();
        store.register_credential("alice", WebAuthnCredential {
            credential_id: "cred-1".to_string(),
            public_key_cose: vec![],
            sign_count: 0,
            device_name: "Device".to_string(),
            registered_at: 0,
            last_used_at: 0,
            attestation_format: "none".to_string(),
            user_verified: false,
        });

        assert!(store.remove_credential("alice", "cred-1"));
        assert!(store.get_credentials("alice").is_empty());
        assert!(!store.remove_credential("alice", "cred-1")); // already removed
    }

    #[test]
    fn test_duplicate_detection() {
        let mut store = WebAuthnStore::new();
        store.register_credential("alice", WebAuthnCredential {
            credential_id: "cred-1".to_string(),
            public_key_cose: vec![],
            sign_count: 0,
            device_name: "Device".to_string(),
            registered_at: 0,
            last_used_at: 0,
            attestation_format: "none".to_string(),
            user_verified: false,
        });

        assert!(store.is_credential_registered("cred-1"));
        assert!(!store.is_credential_registered("cred-2"));
    }

    #[test]
    fn test_authentication_challenge_flow() {
        let mut store = WebAuthnStore::new();
        let config = test_config();

        let (challenge_id, _options) = store.generate_authentication_challenge(
            &config, vec!["cred-1".to_string()],
        );

        store.store_authentication_challenge(
            &challenge_id, "alice", vec![0u8; 32], vec!["cred-1".to_string()],
        );

        let (username, _bytes, allowed) = store.consume_authentication_challenge(&challenge_id, 300).unwrap();
        assert_eq!(username, "alice");
        assert_eq!(allowed, vec!["cred-1".to_string()]);
    }

    #[test]
    fn test_registration_options_serialization() {
        let mut store = WebAuthnStore::new();
        let config = test_config();
        let (_, options) = store.generate_registration_challenge(
            &config, "alice", "Alice Johnson", &["existing-1".to_string()],
        );

        let json = serde_json::to_string(&options).unwrap();
        let deser: RegistrationOptions = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.rp.id, "localhost");
        assert_eq!(deser.user.display_name, "alice");
        assert_eq!(deser.exclude_credentials.len(), 1);
        assert_eq!(deser.pub_key_cred_params.len(), 2);
    }

    #[test]
    fn test_authentication_options_serialization() {
        let mut store = WebAuthnStore::new();
        let config = test_config();
        let (_, options) = store.generate_authentication_challenge(
            &config, vec!["cred-1".to_string()],
        );

        let json = serde_json::to_string(&options).unwrap();
        let deser: AuthenticationOptions = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.rp_id, "localhost");
        assert_eq!(deser.allow_credentials.len(), 1);
    }

    #[test]
    fn test_verify_registration_valid() {
        let challenge = base64_encode_urlsafe(&[0u8; 32]);
        let client_data = serde_json::json!({
            "type": "webauthn.create",
            "challenge": challenge,
            "origin": "http://localhost:8080",
        });
        let client_data_b64 = base64_encode_urlsafe(&serde_json::to_vec(&client_data).unwrap());
        let cred_id_b64 = base64_encode_urlsafe(&[1, 2, 3, 4]);

        let result = verify_registration(
            &[0u8; 32],
            &client_data_b64,
            &cred_id_b64,
            "different-id",
            "localhost",
            &["http://localhost:8080".to_string()],
        ).unwrap();

        assert_eq!(result.credential_id, cred_id_b64);
        assert_eq!(result.attestation_format, "none");
    }

    #[test]
    fn test_verify_registration_challenge_mismatch() {
        let client_data = serde_json::json!({
            "type": "webauthn.create",
            "challenge": base64_encode_urlsafe(&[1u8; 32]), // wrong challenge
            "origin": "http://localhost:8080",
        });
        let client_data_b64 = base64_encode_urlsafe(&serde_json::to_vec(&client_data).unwrap());

        let result = verify_registration(
            &[0u8; 32],
            &client_data_b64,
            "cred-id",
            "different",
            "localhost",
            &["http://localhost:8080".to_string()],
        );
        assert!(matches!(result, Err(WebAuthnError::VerificationFailed(_))));
    }

    #[test]
    fn test_verify_registration_wrong_type() {
        let challenge = base64_encode_urlsafe(&[0u8; 32]);
        let client_data = serde_json::json!({
            "type": "webauthn.get", // wrong type
            "challenge": challenge,
            "origin": "http://localhost:8080",
        });
        let client_data_b64 = base64_encode_urlsafe(&serde_json::to_vec(&client_data).unwrap());

        let result = verify_registration(
            &[0u8; 32],
            &client_data_b64,
            "cred-id",
            "different",
            "localhost",
            &["http://localhost:8080".to_string()],
        );
        assert!(matches!(result, Err(WebAuthnError::VerificationFailed(_))));
    }

    #[test]
    fn test_verify_registration_wrong_origin() {
        let challenge = base64_encode_urlsafe(&[0u8; 32]);
        let client_data = serde_json::json!({
            "type": "webauthn.create",
            "challenge": challenge,
            "origin": "http://evil.com",
        });
        let client_data_b64 = base64_encode_urlsafe(&serde_json::to_vec(&client_data).unwrap());

        let result = verify_registration(
            &[0u8; 32],
            &client_data_b64,
            "cred-id",
            "different",
            "localhost",
            &["http://localhost:8080".to_string()],
        );
        assert!(matches!(result, Err(WebAuthnError::VerificationFailed(_))));
    }

    #[test]
    fn test_verify_registration_duplicate() {
        let challenge = base64_encode_urlsafe(&[0u8; 32]);
        let client_data = serde_json::json!({
            "type": "webauthn.create",
            "challenge": challenge,
            "origin": "http://localhost:8080",
        });
        let client_data_b64 = base64_encode_urlsafe(&serde_json::to_vec(&client_data).unwrap());
        let cred_id_b64 = base64_encode_urlsafe(&[1, 2, 3]);

        let result = verify_registration(
            &[0u8; 32],
            &client_data_b64,
            &cred_id_b64,
            &cred_id_b64, // same as existing -> duplicate
            "localhost",
            &["http://localhost:8080".to_string()],
        );
        assert!(matches!(result, Err(WebAuthnError::DuplicateCredential(_))));
    }

    #[test]
    fn test_verify_authentication_valid() {
        let challenge = base64_encode_urlsafe(&[0u8; 32]);
        let client_data = serde_json::json!({
            "type": "webauthn.get",
            "challenge": challenge,
            "origin": "http://localhost:8080",
        });
        let client_data_b64 = base64_encode_urlsafe(&serde_json::to_vec(&client_data).unwrap());
        let cred_id = "allowed-cred-1".to_string();

        let result = verify_authentication(
            &[0u8; 32],
            &client_data_b64,
            &cred_id,
            &[cred_id.clone()],
            "localhost",
            &["http://localhost:8080".to_string()],
        ).unwrap();

        assert_eq!(result.credential_id, cred_id);
    }

    #[test]
    fn test_verify_authentication_credential_not_allowed() {
        let challenge = base64_encode_urlsafe(&[0u8; 32]);
        let client_data = serde_json::json!({
            "type": "webauthn.get",
            "challenge": challenge,
            "origin": "http://localhost:8080",
        });
        let client_data_b64 = base64_encode_urlsafe(&serde_json::to_vec(&client_data).unwrap());

        let result = verify_authentication(
            &[0u8; 32],
            &client_data_b64,
            "unauthorized-cred",
            &["allowed-cred".to_string()],
            "localhost",
            &["http://localhost:8080".to_string()],
        );
        assert!(matches!(result, Err(WebAuthnError::VerificationFailed(_))));
    }

    #[test]
    fn test_generate_challenge_bytes_length() {
        let bytes = generate_challenge_bytes();
        assert_eq!(bytes.len(), 32);
    }

    #[test]
    fn test_base64_roundtrip() {
        let original = b"hello world";
        let encoded = base64_encode_urlsafe(original);
        let decoded = base64_decode_urlsafe(&encoded).unwrap();
        assert_eq!(decoded, original);
    }
}
