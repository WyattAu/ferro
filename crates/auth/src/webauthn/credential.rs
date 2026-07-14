use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::crypto::{base64_encode_urlsafe, generate_challenge_bytes};
use super::error::WebAuthnError;

/// A registered `WebAuthn` credential.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebAuthnCredential {
    /// Base64url-encoded credential ID.
    pub credential_id: String,
    /// COSE public key bytes (CBOR-encoded).
    pub public_key_cose: Vec<u8>,
    /// Counter to prevent replay attacks.
    pub sign_count: u32,
    /// Human-readable device name (e.g., "`YubiKey` 5 NFC").
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

/// `WebAuthn` relying party configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebAuthnConfig {
    /// Whether `WebAuthn` is enabled.
    pub enabled: bool,
    /// Relying party ID (typically the domain, e.g., "ferro.example.com").
    pub rp_id: String,
    /// Human-readable relying party name.
    pub rp_name: String,
    /// Allowed origins for `WebAuthn` operations.
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

/// Options sent to the client for credential registration (`navigator.credentials.create()`).
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

/// Response from the client during registration (`navigator.credentials.create()` response).
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

/// Options sent to the client for authentication (`navigator.credentials.get()`).
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

/// Response from the client during authentication (`navigator.credentials.get()` response).
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

/// In-memory credential store with challenge management.
pub struct WebAuthnStore {
    /// Per-user credential lists.
    credentials: HashMap<String, Vec<WebAuthnCredential>>,
    /// Pending registration challenges (`challenge_string` -> username).
    registration_challenges: HashMap<String, RegistrationChallenge>,
    /// Pending authentication challenges (`challenge_string` -> username).
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
    #[must_use]
    pub fn new() -> Self {
        Self {
            credentials: HashMap::new(),
            registration_challenges: HashMap::new(),
            authentication_challenges: HashMap::new(),
        }
    }

    /// Store a pending registration challenge.
    pub fn store_registration_challenge(&mut self, challenge_id: &str, username: &str, challenge_bytes: Vec<u8>) {
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
        self.credentials.entry(username.to_string()).or_default().push(cred);
    }

    /// Look up credentials by credential ID across all users.
    #[must_use]
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
    #[must_use]
    pub fn get_credentials(&self, username: &str) -> Vec<WebAuthnCredential> {
        self.credentials.get(username).cloned().unwrap_or_default()
    }

    /// Update the sign count and `last_used_at` for a credential.
    pub fn update_credential_usage(
        &mut self,
        username: &str,
        credential_id: &str,
        new_sign_count: u32,
    ) -> Result<(), WebAuthnError> {
        let creds = self
            .credentials
            .get_mut(username)
            .ok_or_else(|| WebAuthnError::CredentialNotFound(format!("user '{username}' has no credentials")))?;

        let cred = creds
            .iter_mut()
            .find(|c| c.credential_id == credential_id)
            .ok_or_else(|| WebAuthnError::CredentialNotFound(credential_id.to_string()))?;

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
    #[must_use]
    pub fn is_credential_registered(&self, credential_id: &str) -> bool {
        self.find_credential(credential_id).is_some()
    }

    /// Generate a new registration challenge.
    #[must_use]
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
                PubKeyCredParam {
                    alg: -7,
                    type_: "public-key".to_string(),
                },
                PubKeyCredParam {
                    alg: -257,
                    type_: "public-key".to_string(),
                },
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
    #[must_use]
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

#[cfg(test)]
impl WebAuthnStore {
    pub(crate) fn store_registration_challenge_at(
        &mut self,
        challenge_id: &str,
        username: &str,
        challenge_bytes: Vec<u8>,
        created_at: i64,
    ) {
        self.registration_challenges.insert(
            challenge_id.to_string(),
            RegistrationChallenge {
                username: username.to_string(),
                challenge_bytes,
                created_at,
            },
        );
    }

    pub(crate) fn store_authentication_challenge_at(
        &mut self,
        challenge_id: &str,
        username: &str,
        challenge_bytes: Vec<u8>,
        allowed_credential_ids: Vec<String>,
        created_at: i64,
    ) {
        self.authentication_challenges.insert(
            challenge_id.to_string(),
            AuthenticationChallenge {
                username: username.to_string(),
                challenge_bytes,
                allowed_credential_ids,
                created_at,
            },
        );
    }
}
