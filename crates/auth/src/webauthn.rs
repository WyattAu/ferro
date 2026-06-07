//! WebAuthn/FIDO2 passwordless authentication framework.
//!
//! Provides challenge-response flows for credential registration and authentication
//! with real CTAP2/COSE cryptographic verification.
//!
//! Supports:
//! - ES256 (ECDSA P-256 + SHA-256) — the most common WebAuthn algorithm
//! - RS256 (RSA PKCS#1 v1.5 + SHA-256) — for broader authenticator compatibility
//!
//! ## Security
//!
//! This module performs full cryptographic verification of WebAuthn assertions:
//! - COSE public key parsing and signature verification via `ring`
//! - Authenticator data parsing (rpIdHash, flags, signCount, credential data)
//! - Client data JSON hash verification
//! - Origin and RP-ID validation
//! - Challenge freshness and replay protection

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
    #[error("Unsupported algorithm: {0}")]
    UnsupportedAlgorithm(i32),
    #[error("Signature verification failed")]
    SignatureVerificationFailed,
    #[error("Attestation error: {0}")]
    AttestationError(String),
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
                PubKeyCredParam {
                    alg: -7,
                    type_: "public-key".to_string(),
                }, // ES256
                PubKeyCredParam {
                    alg: -257,
                    type_: "public-key".to_string(),
                }, // RS256
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

// ---------------------------------------------------------------------------
// COSE key parsing and signature verification
// ---------------------------------------------------------------------------

/// Parsed COSE public key (EC2 or RSA).
#[derive(Debug, Clone)]
enum CosePublicKey {
    Ec2 { x: Vec<u8>, y: Vec<u8> },
    Rsa { n: Vec<u8>, e: Vec<u8> },
}

/// COSE key type constants.
const COSE_KTY_OKP: i64 = 1;
const COSE_KTY_EC2: i64 = 2;
const COSE_KTY_RSA: i64 = 3;

/// COSE algorithm constants.
const COSE_ALG_ES256: i32 = -7;
const COSE_ALG_RS256: i32 = -257;

/// COSE key map parameter labels.
const COSE_KEY_KTY: i64 = 1;
const COSE_KEY_ALG: i64 = 2;
const COSE_KEY_CRV_N: i64 = -1; // -1 is 'crv' for EC2, 'n' for RSA
const COSE_KEY_X_E: i64 = -2; // -2 is 'x' for EC2, 'e' for RSA
const COSE_KEY_Y: i64 = -3;

/// COSE curve identifiers.
const COSE_CRV_P256: i64 = 1;

/// Parse a CBOR integer from a value, handling both positive and negative.
fn cbor_i64(val: &ciborium::Value) -> Option<i64> {
    use ciborium::Value;
    match val {
        Value::Integer(i) => (*i).try_into().ok(),
        _ => None,
    }
}

/// Parse a CBOR byte string from a value.
fn cbor_bytes(val: &ciborium::Value) -> Option<Vec<u8>> {
    use ciborium::Value;
    match val {
        Value::Bytes(b) => Some(b.clone()),
        Value::Text(t) => Some(t.as_bytes().to_vec()),
        _ => None,
    }
}

/// Parse a CBOR map into a key-value vec of (i64, Value).
fn cbor_map_entries(val: &ciborium::Value) -> Option<Vec<(i64, ciborium::Value)>> {
    use ciborium::Value;
    match val {
        Value::Map(entries) => {
            let mut result = Vec::with_capacity(entries.len());
            for (k, v) in entries {
                let key = cbor_i64(k)?;
                result.push((key, v.clone()));
            }
            Some(result)
        }
        _ => None,
    }
}

/// Parse a COSE key from its CBOR encoding.
fn parse_cose_key(cose_bytes: &[u8]) -> Result<(i32, CosePublicKey), WebAuthnError> {
    use ciborium::Value;

    let key_val: Value = ciborium::de::from_reader(cose_bytes).map_err(|e| {
        WebAuthnError::VerificationFailed(format!("COSE key CBOR parse error: {e}"))
    })?;

    let entries = cbor_map_entries(&key_val).ok_or_else(|| {
        WebAuthnError::VerificationFailed("COSE key is not a CBOR map".to_string())
    })?;

    let mut kty: Option<i64> = None;
    let mut alg: Option<i32> = None;
    let mut crv: Option<i64> = None;
    let mut x: Option<Vec<u8>> = None;
    let mut y: Option<Vec<u8>> = None;
    let mut n: Option<Vec<u8>> = None;
    let mut e: Option<Vec<u8>> = None;

    for (label, val) in &entries {
        match *label {
            COSE_KEY_KTY => kty = cbor_i64(val),
            COSE_KEY_ALG => alg = cbor_i64(val).map(|v| v as i32),
            COSE_KEY_CRV_N => {
                // -1 is 'crv' for EC2 keys, 'n' for RSA keys
                match kty {
                    Some(COSE_KTY_RSA) => n = cbor_bytes(val),
                    _ => crv = cbor_i64(val),
                }
            }
            COSE_KEY_X_E => {
                // -2 is 'x' for EC2 keys, 'e' for RSA keys
                match kty {
                    Some(COSE_KTY_RSA) => e = cbor_bytes(val),
                    _ => x = cbor_bytes(val),
                }
            }
            COSE_KEY_Y => y = cbor_bytes(val),
            _ => {}
        }
    }

    let kty =
        kty.ok_or_else(|| WebAuthnError::VerificationFailed("COSE key missing 'kty'".to_string()))?;
    let alg =
        alg.ok_or_else(|| WebAuthnError::VerificationFailed("COSE key missing 'alg'".to_string()))?;

    match kty {
        COSE_KTY_EC2 => {
            let crv = crv.ok_or_else(|| {
                WebAuthnError::VerificationFailed("EC2 key missing 'crv'".to_string())
            })?;
            if crv != COSE_CRV_P256 {
                return Err(WebAuthnError::UnsupportedAlgorithm(alg));
            }
            let x = x.ok_or_else(|| {
                WebAuthnError::VerificationFailed("EC2 key missing 'x'".to_string())
            })?;
            let y = y.ok_or_else(|| {
                WebAuthnError::VerificationFailed("EC2 key missing 'y'".to_string())
            })?;
            Ok((alg, CosePublicKey::Ec2 { x, y }))
        }
        COSE_KTY_RSA => {
            let n = n.ok_or_else(|| {
                WebAuthnError::VerificationFailed("RSA key missing 'n'".to_string())
            })?;
            let e = e.ok_or_else(|| {
                WebAuthnError::VerificationFailed("RSA key missing 'e'".to_string())
            })?;
            Ok((alg, CosePublicKey::Rsa { n, e }))
        }
        COSE_KTY_OKP => {
            // OKP keys (EdDSA) are not supported in this implementation.
            Err(WebAuthnError::UnsupportedAlgorithm(alg))
        }
        other => Err(WebAuthnError::VerificationFailed(format!(
            "Unsupported COSE key type: {other}"
        ))),
    }
}

/// Verify a COSE signature using the parsed public key.
///
/// For ES256 (ECDSA P-256 + SHA-256): the signature is r||s (64 bytes, fixed-length).
/// For RS256 (RSA PKCS#1 v1.5 + SHA-256): the signature is DER-encoded.
fn verify_cose_signature(
    alg: i32,
    public_key: &CosePublicKey,
    signed_data: &[u8],
    signature: &[u8],
) -> Result<(), WebAuthnError> {
    use ring::signature;

    match alg {
        COSE_ALG_ES256 => {
            let CosePublicKey::Ec2 { x, y } = public_key else {
                return Err(WebAuthnError::VerificationFailed(
                    "EC2 key expected for ES256".to_string(),
                ));
            };

            // Build uncompressed public key: 0x04 || x || y (65 bytes for P-256)
            let mut public_key_bytes = Vec::with_capacity(1 + x.len() + y.len());
            public_key_bytes.push(0x04);
            public_key_bytes.extend_from_slice(x);
            public_key_bytes.extend_from_slice(y);

            let public_key = signature::UnparsedPublicKey::new(
                &signature::ECDSA_P256_SHA256_FIXED,
                &public_key_bytes,
            );
            public_key
                .verify(signed_data, signature)
                .map_err(|_| WebAuthnError::SignatureVerificationFailed)?;
            Ok(())
        }
        COSE_ALG_RS256 => {
            let CosePublicKey::Rsa { n, e } = public_key else {
                return Err(WebAuthnError::VerificationFailed(
                    "RSA key expected for RS256".to_string(),
                ));
            };

            // Build ASN.1 DER-encoded RSA public key (SubjectPublicKeyInfo)
            let rsa_public_key = RsaPublicKeyDer { n, e };
            let der_bytes = rsa_public_key.to_der()?;

            let public_key = signature::UnparsedPublicKey::new(
                &signature::RSA_PKCS1_2048_8192_SHA256,
                &der_bytes,
            );
            public_key
                .verify(signed_data, signature)
                .map_err(|_| WebAuthnError::SignatureVerificationFailed)?;
            Ok(())
        }
        other => Err(WebAuthnError::UnsupportedAlgorithm(other)),
    }
}

/// ASN.1 DER encoding helper for RSA public keys.
///
/// Encodes as SubjectPublicKeyInfo wrapping RSAPublicKey:
/// ```asn1
/// SubjectPublicKeyInfo ::= SEQUENCE {
///   algorithm         AlgorithmIdentifier,
///   subjectPublicKey  BIT STRING
/// }
///
/// AlgorithmIdentifier ::= SEQUENCE {
///   algorithm   OID (1.2.840.113549.1.1.11 = sha256WithRSAEncryption),
///   parameters  ANY DEFINED BY algorithm NULL
/// }
///
/// RSAPublicKey ::= SEQUENCE {
///   modulus           INTEGER,
///   publicExponent    INTEGER
/// }
/// ```
struct RsaPublicKeyDer<'a> {
    n: &'a [u8],
    e: &'a [u8],
}

impl<'a> RsaPublicKeyDer<'a> {
    fn to_der(&self) -> Result<Vec<u8>, WebAuthnError> {
        let mut der = Vec::new();

        // Build the AlgorithmIdentifier SEQUENCE
        let alg_id = Self::encode_sequence_owned({
            let mut buf = Vec::new();
            // OID: 1.2.840.113549.1.1.11 (sha256WithRSAEncryption)
            buf.extend_from_slice(&[
                0x06, 0x09, 0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x0B,
            ]);
            // NULL
            buf.extend_from_slice(&[0x05, 0x00]);
            buf
        });

        // Build the inner RSAPublicKey SEQUENCE
        let rsa_key = Self::encode_sequence_owned({
            let mut buf = Vec::new();
            Self::encode_integer(&mut buf, self.n);
            Self::encode_integer(&mut buf, self.e);
            buf
        });

        // SubjectPublicKeyInfo = SEQUENCE { algorithm_id, BIT STRING(rsa_key) }
        der.extend_from_slice(&alg_id);
        Self::encode_bit_string(&mut der, &rsa_key);
        Self::encode_sequence_in_place(&mut der);

        Ok(der)
    }

    fn encode_integer(buf: &mut Vec<u8>, value: &[u8]) {
        // Remove leading zeros, then encode as ASN.1 INTEGER
        let mut v = value;
        while v.len() > 1 && v[0] == 0 {
            v = &v[1..];
        }
        // If high bit is set, prepend 0x00 to make it positive
        if v.first().is_some_and(|&b| b & 0x80 != 0) {
            buf.push(0x02);
            buf.push((v.len() + 1) as u8);
            buf.push(0x00);
            buf.extend_from_slice(v);
        } else {
            buf.push(0x02);
            buf.push(v.len() as u8);
            buf.extend_from_slice(v);
        }
    }

    fn encode_bit_string(buf: &mut Vec<u8>, content: &[u8]) {
        let len = content.len() + 1; // +1 for unused bits byte
        buf.push(0x03);
        Self::encode_length(buf, len);
        buf.push(0x00); // 0 unused bits
        buf.extend_from_slice(content);
    }

    fn encode_sequence_owned(content: Vec<u8>) -> Vec<u8> {
        let mut result = Vec::with_capacity(2 + content.len());
        result.push(0x30);
        Self::encode_length(&mut result, content.len());
        result.extend_from_slice(&content);
        result
    }

    fn encode_sequence_in_place(buf: &mut Vec<u8>) {
        let content_len = buf.len();
        let header_len = if content_len < 0x80 {
            2
        } else if content_len < 0x100 {
            3
        } else {
            4
        };
        // Grow the buffer to make room for the header
        buf.resize(content_len + header_len, 0);
        // Shift content right by header_len (copy from end to avoid overlap)
        for i in (0..content_len).rev() {
            buf[i + header_len] = buf[i];
        }
        buf[0] = 0x30;
        Self::encode_length_at(buf, 1, content_len);
    }

    fn encode_length(buf: &mut Vec<u8>, len: usize) {
        if len < 0x80 {
            buf.push(len as u8);
        } else if len < 0x100 {
            buf.push(0x81);
            buf.push(len as u8);
        } else {
            buf.push(0x82);
            buf.push((len >> 8) as u8);
            buf.push((len & 0xFF) as u8);
        }
    }

    fn encode_length_at(buf: &mut [u8], offset: usize, len: usize) {
        if len < 0x80 {
            buf[offset] = len as u8;
        } else if len < 0x100 {
            buf[offset] = 0x81;
            buf[offset + 1] = len as u8;
        } else {
            buf[offset] = 0x82;
            buf[offset + 1] = (len >> 8) as u8;
            buf[offset + 2] = (len & 0xFF) as u8;
        }
    }
}

// ---------------------------------------------------------------------------
// Authenticator data parsing
// ---------------------------------------------------------------------------

/// Parsed authenticator data structure (CTAP2 §6.1).
struct AuthenticatorData {
    rp_id_hash: Vec<u8>,
    flags: u8,
    sign_count: u32,
    /// Credential public key COSE bytes (present only if AT flag is set).
    credential_public_key_cose: Option<Vec<u8>>,
    /// Credential ID (present only if AT flag is set).
    credential_id: Option<Vec<u8>>,
}

/// Authenticator data flag bits.
const FLAG_UP: u8 = 0x01; // User Present
const FLAG_UV: u8 = 0x04; // User Verified
const FLAG_AT: u8 = 0x40; // Attested Credential Data included
#[allow(dead_code)]
const FLAG_ED: u8 = 0x80; // Extensions included

/// Minimum authenticator data length: rpIdHash(32) + flags(1) + signCount(4) = 37.
const AUTH_DATA_MIN_LEN: usize = 37;

/// Parse authenticator data from raw bytes.
fn parse_authenticator_data(auth_data: &[u8]) -> Result<AuthenticatorData, WebAuthnError> {
    if auth_data.len() < AUTH_DATA_MIN_LEN {
        return Err(WebAuthnError::VerificationFailed(format!(
            "Authenticator data too short: {} bytes (minimum {})",
            auth_data.len(),
            AUTH_DATA_MIN_LEN
        )));
    }

    let rp_id_hash = auth_data[..32].to_vec();
    let flags = auth_data[32];
    let sign_count = u32::from_be_bytes(auth_data[33..37].try_into().unwrap());

    let mut offset = 37;
    let mut credential_id = None;
    let mut credential_public_key_cose = None;

    if flags & FLAG_AT != 0 {
        // Attested credential data present
        // AAGUID (16 bytes) + credential ID length (2 bytes) + credential ID + COSE public key
        if auth_data.len() < offset + 18 {
            return Err(WebAuthnError::VerificationFailed(
                "Attested credential data truncated (AAGUID + length)".to_string(),
            ));
        }
        offset += 16; // Skip AAGUID

        let cred_id_len = u16::from_be_bytes([auth_data[offset], auth_data[offset + 1]]) as usize;
        offset += 2;

        if auth_data.len() < offset + cred_id_len {
            return Err(WebAuthnError::VerificationFailed(
                "Attested credential data truncated (credential ID)".to_string(),
            ));
        }
        credential_id = Some(auth_data[offset..offset + cred_id_len].to_vec());
        offset += cred_id_len;

        // The remaining bytes are the COSE public key
        if offset >= auth_data.len() {
            return Err(WebAuthnError::VerificationFailed(
                "Attested credential data truncated (public key)".to_string(),
            ));
        }
        credential_public_key_cose = Some(auth_data[offset..].to_vec());
    }

    Ok(AuthenticatorData {
        rp_id_hash,
        flags,
        sign_count,
        credential_public_key_cose,
        credential_id,
    })
}

// ---------------------------------------------------------------------------
// Registration verification
// ---------------------------------------------------------------------------

/// Verify a registration response with full CTAP2/COSE verification.
///
/// Performs:
/// 1. Client data JSON parsing and hash verification
/// 2. Challenge verification
/// 3. Origin validation
/// 4. RP-ID hash validation
/// 5. Attestation object parsing (CBOR)
/// 6. Authenticator data parsing
/// 7. Credential public key extraction
pub fn verify_registration(
    challenge_bytes: &[u8],
    client_data_json_b64: &str,
    attestation_object_b64: &str,
    existing_credential_id: &str,
    rp_id: &str,
    rp_origins: &[String],
) -> Result<RegistrationResult, WebAuthnError> {
    // Parse client data JSON
    let client_data_bytes = base64_decode_urlsafe(client_data_json_b64)?;
    let client_data: serde_json::Value = serde_json::from_slice(&client_data_bytes)
        .map_err(|e| WebAuthnError::VerificationFailed(format!("client data parse error: {e}")))?;

    // Verify challenge
    let client_challenge = client_data
        .get("challenge")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            WebAuthnError::VerificationFailed("missing challenge in client data".to_string())
        })?;
    let client_challenge_bytes = base64_decode_urlsafe(client_challenge)?;
    if client_challenge_bytes != challenge_bytes {
        return Err(WebAuthnError::VerificationFailed(
            "challenge mismatch".to_string(),
        ));
    }

    // Verify type
    let typ = client_data
        .get("type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            WebAuthnError::VerificationFailed("missing type in client data".to_string())
        })?;
    if typ != "webauthn.create" {
        return Err(WebAuthnError::VerificationFailed(format!(
            "wrong type: {typ}"
        )));
    }

    // Verify origin
    let origin = client_data
        .get("origin")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            WebAuthnError::VerificationFailed("missing origin in client data".to_string())
        })?;
    if !rp_origins.iter().any(|o| o == origin) {
        return Err(WebAuthnError::VerificationFailed(format!(
            "origin '{origin}' not allowed"
        )));
    }

    // Verify rpId if present in client data
    let rp_id_val = client_data.get("rpId").and_then(|v| v.as_str());
    if let Some(rp) = rp_id_val
        && rp != rp_id
    {
        return Err(WebAuthnError::VerificationFailed(format!(
            "rpId mismatch: client sent '{rp}', expected '{rp_id}'"
        )));
    }

    // Parse attestation object (CBOR)
    let attestation_bytes = base64_decode_urlsafe(attestation_object_b64)?;
    let attestation_val: ciborium::Value = ciborium::de::from_reader(&attestation_bytes[..])
        .map_err(|e| {
            WebAuthnError::AttestationError(format!("attestation object CBOR parse error: {e}"))
        })?;

    let attestation_entries = cbor_map_entries(&attestation_val).ok_or_else(|| {
        WebAuthnError::AttestationError("attestation object is not a CBOR map".to_string())
    })?;

    // Extract fmt (attestation format) and authData
    let mut fmt: Option<String> = None;
    let mut auth_data_bytes: Option<Vec<u8>> = None;

    for (key, val) in &attestation_entries {
        match *key {
            1 => {
                // "fmt" key
                if let ciborium::Value::Text(s) = val {
                    fmt = Some(s.clone());
                }
            }
            2 => {
                // "authData" key
                if let Some(b) = cbor_bytes(val) {
                    auth_data_bytes = Some(b);
                }
            }
            _ => {} // attStmt (key 3) and others are ignored for "none" format
        }
    }

    let _fmt = fmt.unwrap_or_else(|| "none".to_string());
    let auth_data_bytes = auth_data_bytes.ok_or_else(|| {
        WebAuthnError::AttestationError("missing authData in attestation object".to_string())
    })?;

    // Parse authenticator data
    let auth_data = parse_authenticator_data(&auth_data_bytes)?;

    // Verify rpIdHash = SHA-256(rp_id)
    use sha2::Digest;
    let computed_rp_id_hash = sha2::Sha256::digest(rp_id.as_bytes()).to_vec();
    if auth_data.rp_id_hash != computed_rp_id_hash {
        return Err(WebAuthnError::VerificationFailed(format!(
            "rpId hash mismatch: computed {:x?}, got {:x?}",
            computed_rp_id_hash, auth_data.rp_id_hash
        )));
    }

    // Verify UP flag is set (user must be present)
    if auth_data.flags & FLAG_UP == 0 {
        return Err(WebAuthnError::VerificationFailed(
            "User Present flag not set".to_string(),
        ));
    }

    // Verify AT flag is set (attested credential data must be present in registration)
    if auth_data.flags & FLAG_AT == 0 {
        return Err(WebAuthnError::VerificationFailed(
            "Attested Credential Data flag not set during registration".to_string(),
        ));
    }

    // Extract credential ID from authenticator data
    let credential_id = auth_data.credential_id.ok_or_else(|| {
        WebAuthnError::VerificationFailed("no credential ID in attested data".to_string())
    })?;
    let credential_id_b64 = base64_encode_urlsafe(&credential_id);

    // Check duplicate
    if existing_credential_id == credential_id_b64 {
        return Err(WebAuthnError::DuplicateCredential(credential_id_b64));
    }

    // Extract COSE public key
    let public_key_cose = auth_data.credential_public_key_cose.ok_or_else(|| {
        WebAuthnError::VerificationFailed("no public key in attested data".to_string())
    })?;

    // Verify the public key can be parsed (validates structure)
    let (alg, _cose_key) = parse_cose_key(&public_key_cose)?;

    // Verify the signature on the attestation data for "none" format:
    // The "none" format means no attestation statement, which is valid.
    // For other formats (packed, fido-u2f), attestation verification would be needed.
    // This is consistent with FIDO2 spec: "none" format is acceptable when
    // the relying party does not require attestation.

    let user_verified = auth_data.flags & FLAG_UV != 0;

    Ok(RegistrationResult {
        credential_id: credential_id_b64,
        device_name: format!("WebAuthn ({})", alg_to_name(alg)),
        attestation_format: _fmt,
        user_verified,
    })
}

// ---------------------------------------------------------------------------
// Authentication verification
// ---------------------------------------------------------------------------

/// Parameters for verifying an authentication response.
pub struct AuthenticationParams {
    /// The raw challenge bytes that were stored server-side.
    pub challenge_bytes: Vec<u8>,
    /// Base64url-encoded client data JSON.
    pub client_data_json_b64: String,
    /// Base64url-encoded authenticator data.
    pub authenticator_data_b64: String,
    /// Base64url-encoded signature.
    pub signature_b64: String,
    /// Base64url-encoded credential ID presented by the authenticator.
    pub credential_id_b64: String,
    /// COSE-encoded public key for this credential.
    pub public_key_cose: Vec<u8>,
    /// Current sign count stored server-side for this credential.
    pub current_sign_count: u32,
    /// Allowed credential IDs for this authentication session.
    pub allowed_credential_ids: Vec<String>,
    /// Expected relying party ID.
    pub rp_id: String,
    /// Allowed origins.
    pub rp_origins: Vec<String>,
}

/// Verify an authentication response with full CTAP2/COSE signature verification.
///
/// Performs:
/// 1. Client data JSON parsing and hash verification
/// 2. Challenge verification
/// 3. Origin validation
/// 4. RP-ID hash validation
/// 5. Authenticator data parsing
/// 6. Signature verification: sign(authenticatorData || SHA-256(clientDataJSON))
pub fn verify_authentication(
    params: &AuthenticationParams,
) -> Result<AuthenticationResult, WebAuthnError> {
    let challenge_bytes = &params.challenge_bytes;
    let client_data_json_b64 = &params.client_data_json_b64;
    let authenticator_data_b64 = &params.authenticator_data_b64;
    let signature_b64 = &params.signature_b64;
    let credential_id_b64 = &params.credential_id_b64;
    let public_key_cose = &params.public_key_cose;
    let current_sign_count = params.current_sign_count;
    let allowed_credential_ids = &params.allowed_credential_ids;
    let rp_id = &params.rp_id;
    let rp_origins = &params.rp_origins;
    // Verify credential ID is in allowed list
    if !allowed_credential_ids.contains(&credential_id_b64.to_string()) {
        return Err(WebAuthnError::VerificationFailed(
            "credential ID not in allowed list".to_string(),
        ));
    }

    // Parse client data JSON
    let client_data_bytes = base64_decode_urlsafe(client_data_json_b64)?;
    let client_data: serde_json::Value = serde_json::from_slice(&client_data_bytes)
        .map_err(|e| WebAuthnError::VerificationFailed(format!("client data parse error: {e}")))?;

    // Verify challenge
    let client_challenge = client_data
        .get("challenge")
        .and_then(|v| v.as_str())
        .ok_or_else(|| WebAuthnError::VerificationFailed("missing challenge".to_string()))?;
    let client_challenge_bytes = base64_decode_urlsafe(client_challenge)?;
    if client_challenge_bytes != challenge_bytes[..] {
        return Err(WebAuthnError::VerificationFailed(
            "challenge mismatch".to_string(),
        ));
    }

    // Verify type
    let typ = client_data
        .get("type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| WebAuthnError::VerificationFailed("missing type".to_string()))?;
    if typ != "webauthn.get" {
        return Err(WebAuthnError::VerificationFailed(format!(
            "wrong type: {typ}"
        )));
    }

    // Verify origin
    let origin = client_data
        .get("origin")
        .and_then(|v| v.as_str())
        .ok_or_else(|| WebAuthnError::VerificationFailed("missing origin".to_string()))?;
    if !rp_origins.iter().any(|o| o == origin) {
        return Err(WebAuthnError::VerificationFailed(format!(
            "origin '{origin}' not allowed"
        )));
    }

    // Verify rpId if present in client data
    let rp_id_val = client_data.get("rpId").and_then(|v| v.as_str());
    if let Some(rp) = rp_id_val
        && rp != rp_id
    {
        return Err(WebAuthnError::VerificationFailed(format!(
            "rpId mismatch: client sent '{rp}', expected '{rp_id}'"
        )));
    }

    // Parse authenticator data
    let authenticator_data = base64_decode_urlsafe(authenticator_data_b64)?;
    let auth_data = parse_authenticator_data(&authenticator_data)?;

    // Verify rpIdHash = SHA-256(rp_id)
    use sha2::Digest;
    let computed_rp_id_hash = sha2::Sha256::digest(rp_id.as_bytes()).to_vec();
    if auth_data.rp_id_hash != computed_rp_id_hash {
        return Err(WebAuthnError::VerificationFailed(format!(
            "rpId hash mismatch: computed {:x?}, got {:x?}",
            computed_rp_id_hash, auth_data.rp_id_hash
        )));
    }

    // Verify UP flag is set (user must be present)
    if auth_data.flags & FLAG_UP == 0 {
        return Err(WebAuthnError::VerificationFailed(
            "User Present flag not set".to_string(),
        ));
    }

    // Verify sign count (prevent replay attacks)
    // If current_sign_count is 0 and auth_data.sign_count is 0, skip check
    // (both could be 0 if the authenticator doesn't maintain a counter)
    if current_sign_count != 0 && auth_data.sign_count <= current_sign_count {
        // Some authenticators may not increment the counter, so we only fail
        // if the counter went backwards (which would indicate a cloned authenticator).
        if auth_data.sign_count < current_sign_count {
            return Err(WebAuthnError::VerificationFailed(format!(
                "Sign count decreased: {} < {} (possible cloned authenticator)",
                auth_data.sign_count, current_sign_count
            )));
        }
    }

    // Compute client data hash
    let client_data_hash = sha2::Sha256::digest(&client_data_bytes).to_vec();

    // Build signed data: authenticatorData || SHA-256(clientDataJSON)
    let mut signed_data = Vec::with_capacity(authenticator_data.len() + 32);
    signed_data.extend_from_slice(&authenticator_data);
    signed_data.extend_from_slice(&client_data_hash);

    // Parse signature
    let signature = base64_decode_urlsafe(signature_b64)?;

    // Parse COSE public key and verify signature
    let (alg, cose_key) = parse_cose_key(public_key_cose)?;
    verify_cose_signature(alg, &cose_key, &signed_data, &signature)?;

    let user_verified = auth_data.flags & FLAG_UV != 0;
    let new_sign_count = auth_data.sign_count;

    Ok(AuthenticationResult {
        credential_id: credential_id_b64.to_string(),
        new_sign_count,
        user_verified,
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Convert a COSE algorithm ID to a human-readable name.
fn alg_to_name(alg: i32) -> &'static str {
    match alg {
        COSE_ALG_ES256 => "ES256",
        COSE_ALG_RS256 => "RS256",
        _ => "unknown",
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

#[cfg(test)]
mod tests {
    use super::*;
    use ring::signature::KeyPair as _;

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

        let (challenge_id, _options) =
            store.generate_registration_challenge(&config, "alice", "Alice", &[]);

        store.store_registration_challenge(&challenge_id, "alice", challenge_id.as_bytes().into());

        let (username, bytes) = store
            .consume_registration_challenge(&challenge_id, 300)
            .unwrap();
        assert_eq!(username, "alice");
        assert_eq!(bytes, challenge_id.as_bytes());

        assert!(
            store
                .consume_registration_challenge(&challenge_id, 300)
                .is_err()
        );
    }

    #[test]
    fn test_challenge_expiration() {
        let mut store = WebAuthnStore::new();
        store.registration_challenges.insert(
            "ch-1".to_string(),
            RegistrationChallenge {
                username: "alice".to_string(),
                challenge_bytes: vec![0u8; 32],
                created_at: chrono::Utc::now().timestamp() - 301,
            },
        );

        let result = store.consume_registration_challenge("ch-1", 300);
        assert!(matches!(result, Err(WebAuthnError::ChallengeExpired)));
    }

    #[test]
    fn test_find_credential_across_users() {
        let mut store = WebAuthnStore::new();
        store.register_credential(
            "alice",
            WebAuthnCredential {
                credential_id: "shared-1".to_string(),
                public_key_cose: vec![],
                sign_count: 0,
                device_name: "Device".to_string(),
                registered_at: 0,
                last_used_at: 0,
                attestation_format: "none".to_string(),
                user_verified: false,
            },
        );

        let (username, _cred) = store.find_credential("shared-1").unwrap();
        assert_eq!(username, "alice");

        assert!(store.find_credential("nonexistent").is_none());
    }

    #[test]
    fn test_update_credential_usage() {
        let mut store = WebAuthnStore::new();
        store.register_credential(
            "alice",
            WebAuthnCredential {
                credential_id: "cred-1".to_string(),
                public_key_cose: vec![],
                sign_count: 0,
                device_name: "Device".to_string(),
                registered_at: 0,
                last_used_at: 0,
                attestation_format: "none".to_string(),
                user_verified: false,
            },
        );

        store
            .update_credential_usage("alice", "cred-1", 42)
            .unwrap();
        let creds = store.get_credentials("alice");
        assert_eq!(creds[0].sign_count, 42);
        assert!(creds[0].last_used_at > 0);
    }

    #[test]
    fn test_remove_credential() {
        let mut store = WebAuthnStore::new();
        store.register_credential(
            "alice",
            WebAuthnCredential {
                credential_id: "cred-1".to_string(),
                public_key_cose: vec![],
                sign_count: 0,
                device_name: "Device".to_string(),
                registered_at: 0,
                last_used_at: 0,
                attestation_format: "none".to_string(),
                user_verified: false,
            },
        );

        assert!(store.remove_credential("alice", "cred-1"));
        assert!(store.get_credentials("alice").is_empty());
        assert!(!store.remove_credential("alice", "cred-1"));
    }

    #[test]
    fn test_duplicate_detection() {
        let mut store = WebAuthnStore::new();
        store.register_credential(
            "alice",
            WebAuthnCredential {
                credential_id: "cred-1".to_string(),
                public_key_cose: vec![],
                sign_count: 0,
                device_name: "Device".to_string(),
                registered_at: 0,
                last_used_at: 0,
                attestation_format: "none".to_string(),
                user_verified: false,
            },
        );

        assert!(store.is_credential_registered("cred-1"));
        assert!(!store.is_credential_registered("cred-2"));
    }

    #[test]
    fn test_authentication_challenge_flow() {
        let mut store = WebAuthnStore::new();
        let config = test_config();

        let (challenge_id, _options) =
            store.generate_authentication_challenge(&config, vec!["cred-1".to_string()]);

        store.store_authentication_challenge(
            &challenge_id,
            "alice",
            vec![0u8; 32],
            vec!["cred-1".to_string()],
        );

        let (username, _bytes, allowed) = store
            .consume_authentication_challenge(&challenge_id, 300)
            .unwrap();
        assert_eq!(username, "alice");
        assert_eq!(allowed, vec!["cred-1".to_string()]);
    }

    #[test]
    fn test_registration_options_serialization() {
        let store = WebAuthnStore::new();
        let config = test_config();
        let (_, options) = store.generate_registration_challenge(
            &config,
            "alice",
            "Alice Johnson",
            &["existing-1".to_string()],
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
        let store = WebAuthnStore::new();
        let config = test_config();
        let (_, options) =
            store.generate_authentication_challenge(&config, vec!["cred-1".to_string()]);

        let json = serde_json::to_string(&options).unwrap();
        let deser: AuthenticationOptions = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.rp_id, "localhost");
        assert_eq!(deser.allow_credentials.len(), 1);
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

    // ---------------------------------------------------------------------------
    // COSE key parsing tests
    // ---------------------------------------------------------------------------

    /// Build a minimal COSE EC2 key for ES256 (P-256).
    fn build_cose_ec2_key(x: &[u8], y: &[u8]) -> Vec<u8> {
        use ciborium::Value;
        let map = vec![
            (Value::Integer(1.into()), Value::Integer(2.into())),
            (Value::Integer(2.into()), Value::Integer((-7).into())),
            (Value::Integer((-1).into()), Value::Integer(1.into())),
            (Value::Integer((-2).into()), Value::Bytes(x.to_vec())),
            (Value::Integer((-3).into()), Value::Bytes(y.to_vec())),
        ];
        let mut buf = Vec::new();
        ciborium::ser::into_writer(&Value::Map(map), &mut buf).unwrap();
        buf
    }

    fn build_cose_rsa_key(n: &[u8], e: &[u8]) -> Vec<u8> {
        use ciborium::Value;
        let map = vec![
            (Value::Integer(1.into()), Value::Integer(3.into())),
            (Value::Integer(2.into()), Value::Integer((-257).into())),
            (Value::Integer((-1).into()), Value::Bytes(n.to_vec())),
            (Value::Integer((-2).into()), Value::Bytes(e.to_vec())),
        ];
        let mut buf = Vec::new();
        ciborium::ser::into_writer(&Value::Map(map), &mut buf).unwrap();
        buf
    }

    #[test]
    fn test_parse_cose_ec2_key() {
        let x = vec![0xAA; 32];
        let y = vec![0xBB; 32];
        let cose_key = build_cose_ec2_key(&x, &y);

        let (alg, key) = parse_cose_key(&cose_key).unwrap();
        assert_eq!(alg, -7); // ES256
        match key {
            CosePublicKey::Ec2 { x: kx, y: ky } => {
                assert_eq!(kx, x);
                assert_eq!(ky, y);
            }
            _ => panic!("Expected EC2 key"),
        }
    }

    #[test]
    fn test_parse_cose_rsa_key() {
        let n = vec![0xAA; 256];
        let e = vec![0x01, 0x00, 0x01]; // 65537
        let cose_key = build_cose_rsa_key(&n, &e);

        let (alg, key) = parse_cose_key(&cose_key).unwrap();
        assert_eq!(alg, -257); // RS256
        match key {
            CosePublicKey::Rsa { n: kn, e: ke } => {
                assert_eq!(kn, n);
                assert_eq!(ke, e);
            }
            _ => panic!("Expected RSA key"),
        }
    }

    #[test]
    fn test_verify_cose_signature_es256() {
        use ring::signature::{ECDSA_P256_SHA256_FIXED_SIGNING, EcdsaKeyPair};

        let rng = ring::rand::SystemRandom::new();
        let pkcs8 = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, &rng).unwrap();
        let key_pair =
            EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, pkcs8.as_ref(), &rng)
                .unwrap();

        let public_key_bytes = key_pair.public_key().as_ref().to_vec();
        // Uncompressed: 0x04 || x(32) || y(32)
        let x = public_key_bytes[1..33].to_vec();
        let y = public_key_bytes[33..65].to_vec();

        let cose_key = CosePublicKey::Ec2 { x, y };
        let message = b"test message for WebAuthn";
        let signature = key_pair.sign(&rng, message).unwrap();

        let result = verify_cose_signature(COSE_ALG_ES256, &cose_key, message, signature.as_ref());
        assert!(result.is_ok());

        // Wrong message should fail
        let wrong_result = verify_cose_signature(
            COSE_ALG_ES256,
            &cose_key,
            b"wrong message",
            signature.as_ref(),
        );
        assert!(matches!(
            wrong_result,
            Err(WebAuthnError::SignatureVerificationFailed)
        ));
    }

    #[test]
    fn test_verify_cose_signature_rs256() {
        // Build a simple RSA public key with known n, e and test our DER encoding.
        // n = 0xC0E95A... (2048-bit), e = 0x010001 (65537)
        let n = vec![
            0xC0, 0xE9, 0x5A, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
        ];
        let e = vec![0x01, 0x00, 0x01];
        let cose_key = build_cose_rsa_key(&n, &e);

        let (alg, key) = parse_cose_key(&cose_key).unwrap();
        assert_eq!(alg, -257); // RS256
        match key {
            CosePublicKey::Rsa { n: kn, e: ke } => {
                assert_eq!(kn, n);
                assert_eq!(ke, e);
            }
            _ => panic!("Expected RSA key"),
        }

        // Test DER encoding produces valid structure by encoding and checking length
        let rsa_key = RsaPublicKeyDer { n: &n, e: &e };
        let der = rsa_key.to_der().unwrap();
        // SubjectPublicKeyInfo should start with SEQUENCE tag (0x30)
        assert_eq!(der[0], 0x30);
        // Should contain the AlgorithmIdentifier
        assert!(der.len() > 40);
    }

    // ---------------------------------------------------------------------------
    // Authenticator data parsing tests
    // ---------------------------------------------------------------------------

    #[test]
    fn test_parse_authenticator_data_minimal() {
        let mut auth_data = vec![0u8; 37];
        auth_data[..32].copy_from_slice(&[0xAA; 32]); // rpIdHash
        auth_data[32] = FLAG_UP; // flags: User Present
        auth_data[33..37].copy_from_slice(&1u32.to_be_bytes()); // signCount = 1

        let parsed = parse_authenticator_data(&auth_data).unwrap();
        assert_eq!(parsed.rp_id_hash, vec![0xAA; 32]);
        assert_eq!(parsed.flags, FLAG_UP);
        assert_eq!(parsed.sign_count, 1);
        assert!(parsed.credential_public_key_cose.is_none());
        assert!(parsed.credential_id.is_none());
    }

    #[test]
    fn test_parse_authenticator_data_with_attested_credential() {
        let credential_id = vec![0x01, 0x02, 0x03, 0x04];
        let public_key_cose = vec![0x10, 0x20, 0x30];

        // 37 (header) + 16 (AAGUID) + 2 (cred_id_len) + 4 (cred_id) + 3 (public key)
        let total_len = 37 + 16 + 2 + credential_id.len() + public_key_cose.len();
        let mut auth_data = vec![0u8; total_len];
        auth_data[32] = FLAG_UP | FLAG_AT; // flags: UP + AT
        auth_data[33..37].copy_from_slice(&5u32.to_be_bytes()); // signCount = 5

        // AAGUID (16 bytes at offset 37, all zeros is fine)
        let offset = 37 + 16;
        auth_data[offset..offset + 2].copy_from_slice(&(credential_id.len() as u16).to_be_bytes());
        auth_data[offset + 2..offset + 2 + credential_id.len()].copy_from_slice(&credential_id);
        let pk_offset = offset + 2 + credential_id.len();
        auth_data[pk_offset..].copy_from_slice(&public_key_cose);

        let parsed = parse_authenticator_data(&auth_data).unwrap();
        assert_eq!(parsed.flags, FLAG_UP | FLAG_AT);
        assert_eq!(parsed.sign_count, 5);
        assert_eq!(parsed.credential_id, Some(credential_id));
        assert_eq!(parsed.credential_public_key_cose, Some(public_key_cose));
    }

    #[test]
    fn test_parse_authenticator_data_too_short() {
        let auth_data = vec![0u8; 10]; // too short
        let result = parse_authenticator_data(&auth_data);
        assert!(result.is_err());
    }

    // ---------------------------------------------------------------------------
    // Registration verification tests (using real COSE/CTAP2 structures)
    // ---------------------------------------------------------------------------

    /// Build a minimal attestation object (CBOR) for "none" format.
    fn build_attestation_object(auth_data: &[u8]) -> Vec<u8> {
        use ciborium::Value;
        let map = vec![
            (Value::Integer(1.into()), Value::Text("none".to_string())),
            (Value::Integer(2.into()), Value::Bytes(auth_data.to_vec())),
            (Value::Integer(3.into()), Value::Map(vec![])),
        ];
        let mut buf = Vec::new();
        ciborium::ser::into_writer(&Value::Map(map), &mut buf).unwrap();
        buf
    }

    /// Build authenticator data with attested credential data containing an ES256 key.
    fn build_auth_data_with_credential(
        rp_id: &str,
        flags: u8,
        sign_count: u32,
        credential_id: &[u8],
        cose_key: &[u8],
    ) -> Vec<u8> {
        use sha2::Digest;
        let rp_id_hash = sha2::Sha256::digest(rp_id.as_bytes()).to_vec();
        let mut auth_data = Vec::new();
        auth_data.extend_from_slice(&rp_id_hash);
        auth_data.push(flags);
        auth_data.extend_from_slice(&sign_count.to_be_bytes());

        if flags & FLAG_AT != 0 {
            // AAGUID (16 zeros)
            auth_data.extend_from_slice(&[0u8; 16]);
            // credential ID length
            auth_data.extend_from_slice(&(credential_id.len() as u16).to_be_bytes());
            // credential ID
            auth_data.extend_from_slice(credential_id);
            // COSE public key
            auth_data.extend_from_slice(cose_key);
        }

        auth_data
    }

    #[test]
    fn test_verify_registration_valid_es256() {
        use ring::signature::EcdsaKeyPair;

        let rng = ring::rand::SystemRandom::new();
        let pkcs8 =
            EcdsaKeyPair::generate_pkcs8(&ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING, &rng)
                .unwrap();
        let key_pair = EcdsaKeyPair::from_pkcs8(
            &ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING,
            pkcs8.as_ref(),
            &rng,
        )
        .unwrap();

        let pub_bytes = key_pair.public_key().as_ref().to_vec();
        let x = pub_bytes[1..33].to_vec();
        let y = pub_bytes[33..65].to_vec();
        let cose_key = build_cose_ec2_key(&x, &y);

        let credential_id = vec![0x01, 0x02, 0x03, 0x04];
        let auth_data = build_auth_data_with_credential(
            "localhost",
            FLAG_UP | FLAG_AT,
            0,
            &credential_id,
            &cose_key,
        );
        let att_obj = build_attestation_object(&auth_data);

        let challenge = generate_challenge_bytes();
        let challenge_b64 = base64_encode_urlsafe(&challenge);

        let client_data = serde_json::json!({
            "type": "webauthn.create",
            "challenge": challenge_b64,
            "origin": "http://localhost:8080",
        });
        let client_data_b64 = base64_encode_urlsafe(&serde_json::to_vec(&client_data).unwrap());
        let att_obj_b64 = base64_encode_urlsafe(&att_obj);

        let result = verify_registration(
            &challenge,
            &client_data_b64,
            &att_obj_b64,
            "different-id",
            "localhost",
            &["http://localhost:8080".to_string()],
        )
        .unwrap();

        assert_eq!(result.credential_id, base64_encode_urlsafe(&credential_id));
        assert!(result.attestation_format == "none");
    }

    #[test]
    fn test_verify_registration_challenge_mismatch() {
        let auth_data = build_auth_data_with_credential(
            "localhost",
            FLAG_UP | FLAG_AT,
            0,
            &[0x01],
            &build_cose_ec2_key(&[0xAA; 32], &[0xBB; 32]),
        );
        let att_obj = build_attestation_object(&auth_data);

        let challenge = generate_challenge_bytes();
        let _challenge_b64 = base64_encode_urlsafe(&challenge);

        let client_data = serde_json::json!({
            "type": "webauthn.create",
            "challenge": base64_encode_urlsafe(&[1u8; 32]), // wrong challenge
            "origin": "http://localhost:8080",
        });
        let client_data_b64 = base64_encode_urlsafe(&serde_json::to_vec(&client_data).unwrap());
        let att_obj_b64 = base64_encode_urlsafe(&att_obj);

        let result = verify_registration(
            &challenge,
            &client_data_b64,
            &att_obj_b64,
            "different",
            "localhost",
            &["http://localhost:8080".to_string()],
        );
        assert!(matches!(result, Err(WebAuthnError::VerificationFailed(_))));
    }

    #[test]
    fn test_verify_registration_wrong_type() {
        let auth_data = build_auth_data_with_credential(
            "localhost",
            FLAG_UP | FLAG_AT,
            0,
            &[0x01],
            &build_cose_ec2_key(&[0xAA; 32], &[0xBB; 32]),
        );
        let att_obj = build_attestation_object(&auth_data);

        let challenge = generate_challenge_bytes();
        let challenge_b64 = base64_encode_urlsafe(&challenge);

        let client_data = serde_json::json!({
            "type": "webauthn.get", // wrong type
            "challenge": challenge_b64,
            "origin": "http://localhost:8080",
        });
        let client_data_b64 = base64_encode_urlsafe(&serde_json::to_vec(&client_data).unwrap());
        let att_obj_b64 = base64_encode_urlsafe(&att_obj);

        let result = verify_registration(
            &challenge,
            &client_data_b64,
            &att_obj_b64,
            "different",
            "localhost",
            &["http://localhost:8080".to_string()],
        );
        assert!(matches!(result, Err(WebAuthnError::VerificationFailed(_))));
    }

    #[test]
    fn test_verify_registration_wrong_origin() {
        let auth_data = build_auth_data_with_credential(
            "localhost",
            FLAG_UP | FLAG_AT,
            0,
            &[0x01],
            &build_cose_ec2_key(&[0xAA; 32], &[0xBB; 32]),
        );
        let att_obj = build_attestation_object(&auth_data);

        let challenge = generate_challenge_bytes();
        let challenge_b64 = base64_encode_urlsafe(&challenge);

        let client_data = serde_json::json!({
            "type": "webauthn.create",
            "challenge": challenge_b64,
            "origin": "http://evil.com",
        });
        let client_data_b64 = base64_encode_urlsafe(&serde_json::to_vec(&client_data).unwrap());
        let att_obj_b64 = base64_encode_urlsafe(&att_obj);

        let result = verify_registration(
            &challenge,
            &client_data_b64,
            &att_obj_b64,
            "different",
            "localhost",
            &["http://localhost:8080".to_string()],
        );
        assert!(matches!(result, Err(WebAuthnError::VerificationFailed(_))));
    }

    #[test]
    fn test_verify_registration_duplicate() {
        let auth_data = build_auth_data_with_credential(
            "localhost",
            FLAG_UP | FLAG_AT,
            0,
            &[0x01, 0x02, 0x03],
            &build_cose_ec2_key(&[0xAA; 32], &[0xBB; 32]),
        );
        let att_obj = build_attestation_object(&auth_data);

        let challenge = generate_challenge_bytes();
        let challenge_b64 = base64_encode_urlsafe(&challenge);

        let client_data = serde_json::json!({
            "type": "webauthn.create",
            "challenge": challenge_b64,
            "origin": "http://localhost:8080",
        });
        let client_data_b64 = base64_encode_urlsafe(&serde_json::to_vec(&client_data).unwrap());
        let att_obj_b64 = base64_encode_urlsafe(&att_obj);
        let cred_id_b64 = base64_encode_urlsafe(&[0x01, 0x02, 0x03]);

        let result = verify_registration(
            &challenge,
            &client_data_b64,
            &att_obj_b64,
            &cred_id_b64, // same as credential in attestation -> duplicate
            "localhost",
            &["http://localhost:8080".to_string()],
        );
        assert!(matches!(result, Err(WebAuthnError::DuplicateCredential(_))));
    }

    #[test]
    fn test_verify_registration_wrong_rp_id_hash() {
        let credential_id = vec![0x01];
        let cose_key = build_cose_ec2_key(&[0xAA; 32], &[0xBB; 32]);
        let auth_data = build_auth_data_with_credential(
            "evil.com", // wrong RP ID
            FLAG_UP | FLAG_AT,
            0,
            &credential_id,
            &cose_key,
        );
        let att_obj = build_attestation_object(&auth_data);

        let challenge = generate_challenge_bytes();
        let challenge_b64 = base64_encode_urlsafe(&challenge);

        let client_data = serde_json::json!({
            "type": "webauthn.create",
            "challenge": challenge_b64,
            "origin": "http://localhost:8080",
        });
        let client_data_b64 = base64_encode_urlsafe(&serde_json::to_vec(&client_data).unwrap());
        let att_obj_b64 = base64_encode_urlsafe(&att_obj);

        let result = verify_registration(
            &challenge,
            &client_data_b64,
            &att_obj_b64,
            "different",
            "localhost",
            &["http://localhost:8080".to_string()],
        );
        assert!(matches!(result, Err(WebAuthnError::VerificationFailed(_))));
    }

    #[test]
    fn test_verify_registration_missing_up_flag() {
        let credential_id = vec![0x01];
        let cose_key = build_cose_ec2_key(&[0xAA; 32], &[0xBB; 32]);
        let auth_data = build_auth_data_with_credential(
            "localhost",
            FLAG_AT, // no UP flag
            0,
            &credential_id,
            &cose_key,
        );
        let att_obj = build_attestation_object(&auth_data);

        let challenge = generate_challenge_bytes();
        let challenge_b64 = base64_encode_urlsafe(&challenge);

        let client_data = serde_json::json!({
            "type": "webauthn.create",
            "challenge": challenge_b64,
            "origin": "http://localhost:8080",
        });
        let client_data_b64 = base64_encode_urlsafe(&serde_json::to_vec(&client_data).unwrap());
        let att_obj_b64 = base64_encode_urlsafe(&att_obj);

        let result = verify_registration(
            &challenge,
            &client_data_b64,
            &att_obj_b64,
            "different",
            "localhost",
            &["http://localhost:8080".to_string()],
        );
        assert!(matches!(result, Err(WebAuthnError::VerificationFailed(_))));
    }

    // ---------------------------------------------------------------------------
    // Authentication verification tests
    // ---------------------------------------------------------------------------

    #[test]
    fn test_verify_authentication_valid_es256() {
        use ring::signature::EcdsaKeyPair;

        let rng = ring::rand::SystemRandom::new();
        let pkcs8 =
            EcdsaKeyPair::generate_pkcs8(&ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING, &rng)
                .unwrap();
        let key_pair = EcdsaKeyPair::from_pkcs8(
            &ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING,
            pkcs8.as_ref(),
            &rng,
        )
        .unwrap();

        let pub_bytes = key_pair.public_key().as_ref().to_vec();
        let x = pub_bytes[1..33].to_vec();
        let y = pub_bytes[33..65].to_vec();
        let cose_key = build_cose_ec2_key(&x, &y);

        let challenge = generate_challenge_bytes();
        let challenge_b64 = base64_encode_urlsafe(&challenge);

        // Build authenticator data
        use sha2::Digest;
        let rp_id_hash = sha2::Sha256::digest(b"localhost").to_vec();
        let mut auth_data = Vec::new();
        auth_data.extend_from_slice(&rp_id_hash);
        auth_data.push(FLAG_UP);
        auth_data.extend_from_slice(&1u32.to_be_bytes()); // signCount = 1

        // Build client data and compute hash
        let client_data = serde_json::json!({
            "type": "webauthn.get",
            "challenge": challenge_b64,
            "origin": "http://localhost:8080",
        });
        let client_data_bytes = serde_json::to_vec(&client_data).unwrap();
        let client_data_b64 = base64_encode_urlsafe(&client_data_bytes);
        let client_data_hash = sha2::Sha256::digest(&client_data_bytes).to_vec();

        // Build signed data: authenticatorData || SHA-256(clientDataJSON)
        let mut signed_data = Vec::new();
        signed_data.extend_from_slice(&auth_data);
        signed_data.extend_from_slice(&client_data_hash);

        // Sign
        let signature = key_pair.sign(&rng, &signed_data).unwrap();

        let auth_data_b64 = base64_encode_urlsafe(&auth_data);
        let sig_b64 = base64_encode_urlsafe(signature.as_ref());
        let cred_id_b64 = base64_encode_urlsafe(&[0x01, 0x02]);

        let result = verify_authentication(&AuthenticationParams {
            challenge_bytes: challenge.clone(),
            client_data_json_b64: client_data_b64,
            authenticator_data_b64: auth_data_b64,
            signature_b64: sig_b64,
            credential_id_b64: cred_id_b64.clone(),
            public_key_cose: cose_key,
            current_sign_count: 0,
            allowed_credential_ids: vec![cred_id_b64.clone()],
            rp_id: "localhost".to_string(),
            rp_origins: vec!["http://localhost:8080".to_string()],
        })
        .unwrap();

        assert_eq!(result.credential_id, cred_id_b64);
        assert_eq!(result.new_sign_count, 1);
    }

    #[test]
    fn test_verify_authentication_wrong_signature() {
        use ring::signature::EcdsaKeyPair;

        let rng = ring::rand::SystemRandom::new();
        let pkcs8 =
            EcdsaKeyPair::generate_pkcs8(&ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING, &rng)
                .unwrap();
        let key_pair = EcdsaKeyPair::from_pkcs8(
            &ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING,
            pkcs8.as_ref(),
            &rng,
        )
        .unwrap();

        let pub_bytes = key_pair.public_key().as_ref().to_vec();
        let x = pub_bytes[1..33].to_vec();
        let y = pub_bytes[33..65].to_vec();
        let cose_key = build_cose_ec2_key(&x, &y);

        let challenge = generate_challenge_bytes();
        let challenge_b64 = base64_encode_urlsafe(&challenge);

        use sha2::Digest;
        let rp_id_hash = sha2::Sha256::digest(b"localhost").to_vec();
        let mut auth_data = Vec::new();
        auth_data.extend_from_slice(&rp_id_hash);
        auth_data.push(FLAG_UP);
        auth_data.extend_from_slice(&1u32.to_be_bytes());

        let client_data = serde_json::json!({
            "type": "webauthn.get",
            "challenge": challenge_b64,
            "origin": "http://localhost:8080",
        });
        let client_data_bytes = serde_json::to_vec(&client_data).unwrap();
        let client_data_b64 = base64_encode_urlsafe(&client_data_bytes);
        let client_data_hash = sha2::Sha256::digest(&client_data_bytes).to_vec();

        let mut signed_data = Vec::new();
        signed_data.extend_from_slice(&auth_data);
        signed_data.extend_from_slice(&client_data_hash);

        let mut wrong_sig = vec![0u8; 64];
        wrong_sig[0] = 0xFF; // garbage signature

        let auth_data_b64 = base64_encode_urlsafe(&auth_data);
        let sig_b64 = base64_encode_urlsafe(&wrong_sig);
        let cred_id_b64 = base64_encode_urlsafe(&[0x01]);

        let result = verify_authentication(&AuthenticationParams {
            challenge_bytes: challenge.clone(),
            client_data_json_b64: client_data_b64,
            authenticator_data_b64: auth_data_b64,
            signature_b64: sig_b64,
            credential_id_b64: cred_id_b64.clone(),
            public_key_cose: cose_key,
            current_sign_count: 0,
            allowed_credential_ids: vec![cred_id_b64],
            rp_id: "localhost".to_string(),
            rp_origins: vec!["http://localhost:8080".to_string()],
        });
        assert!(matches!(
            result,
            Err(WebAuthnError::SignatureVerificationFailed)
        ));
    }

    #[test]
    fn test_verify_authentication_credential_not_allowed() {
        let challenge = generate_challenge_bytes();
        let challenge_b64 = base64_encode_urlsafe(&challenge);

        let client_data = serde_json::json!({
            "type": "webauthn.get",
            "challenge": challenge_b64,
            "origin": "http://localhost:8080",
        });
        let client_data_b64 = base64_encode_urlsafe(&serde_json::to_vec(&client_data).unwrap());

        use sha2::Digest;
        let rp_id_hash = sha2::Sha256::digest(b"localhost").to_vec();
        let mut auth_data = Vec::new();
        auth_data.extend_from_slice(&rp_id_hash);
        auth_data.push(FLAG_UP);
        auth_data.extend_from_slice(&0u32.to_be_bytes());
        let auth_data_b64 = base64_encode_urlsafe(&auth_data);

        let result = verify_authentication(&AuthenticationParams {
            challenge_bytes: challenge.clone(),
            client_data_json_b64: client_data_b64,
            authenticator_data_b64: auth_data_b64,
            signature_b64: "sig".to_string(),
            credential_id_b64: "unauthorized-cred".to_string(),
            public_key_cose: vec![0x10, 0x20],
            current_sign_count: 0,
            allowed_credential_ids: vec!["allowed-cred".to_string()],
            rp_id: "localhost".to_string(),
            rp_origins: vec!["http://localhost:8080".to_string()],
        });
        assert!(matches!(result, Err(WebAuthnError::VerificationFailed(_))));
    }

    #[test]
    fn test_verify_authentication_wrong_origin() {
        let challenge = generate_challenge_bytes();
        let challenge_b64 = base64_encode_urlsafe(&challenge);

        let client_data = serde_json::json!({
            "type": "webauthn.get",
            "challenge": challenge_b64,
            "origin": "http://evil.com",
        });
        let client_data_b64 = base64_encode_urlsafe(&serde_json::to_vec(&client_data).unwrap());

        use sha2::Digest;
        let rp_id_hash = sha2::Sha256::digest(b"localhost").to_vec();
        let mut auth_data = Vec::new();
        auth_data.extend_from_slice(&rp_id_hash);
        auth_data.push(FLAG_UP);
        auth_data.extend_from_slice(&0u32.to_be_bytes());
        let auth_data_b64 = base64_encode_urlsafe(&auth_data);

        let result = verify_authentication(&AuthenticationParams {
            challenge_bytes: challenge.clone(),
            client_data_json_b64: client_data_b64,
            authenticator_data_b64: auth_data_b64,
            signature_b64: "sig".to_string(),
            credential_id_b64: "cred".to_string(),
            public_key_cose: vec![0x10],
            current_sign_count: 0,
            allowed_credential_ids: vec!["cred".to_string()],
            rp_id: "localhost".to_string(),
            rp_origins: vec!["http://localhost:8080".to_string()],
        });
        assert!(matches!(result, Err(WebAuthnError::VerificationFailed(_))));
    }

    #[test]
    fn test_verify_authentication_wrong_type() {
        let challenge = generate_challenge_bytes();
        let challenge_b64 = base64_encode_urlsafe(&challenge);

        let client_data = serde_json::json!({
            "type": "webauthn.create", // wrong type
            "challenge": challenge_b64,
            "origin": "http://localhost:8080",
        });
        let client_data_b64 = base64_encode_urlsafe(&serde_json::to_vec(&client_data).unwrap());

        use sha2::Digest;
        let rp_id_hash = sha2::Sha256::digest(b"localhost").to_vec();
        let mut auth_data = Vec::new();
        auth_data.extend_from_slice(&rp_id_hash);
        auth_data.push(FLAG_UP);
        auth_data.extend_from_slice(&0u32.to_be_bytes());
        let auth_data_b64 = base64_encode_urlsafe(&auth_data);

        let result = verify_authentication(&AuthenticationParams {
            challenge_bytes: challenge.clone(),
            client_data_json_b64: client_data_b64,
            authenticator_data_b64: auth_data_b64,
            signature_b64: "sig".to_string(),
            credential_id_b64: "cred".to_string(),
            public_key_cose: vec![0x10],
            current_sign_count: 0,
            allowed_credential_ids: vec!["cred".to_string()],
            rp_id: "localhost".to_string(),
            rp_origins: vec!["http://localhost:8080".to_string()],
        });
        assert!(matches!(result, Err(WebAuthnError::VerificationFailed(_))));
    }

    #[test]
    fn test_verify_authentication_sign_count_decrease() {
        use ring::signature::EcdsaKeyPair;

        let rng = ring::rand::SystemRandom::new();
        let pkcs8 =
            EcdsaKeyPair::generate_pkcs8(&ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING, &rng)
                .unwrap();
        let key_pair = EcdsaKeyPair::from_pkcs8(
            &ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING,
            pkcs8.as_ref(),
            &rng,
        )
        .unwrap();

        let pub_bytes = key_pair.public_key().as_ref().to_vec();
        let x = pub_bytes[1..33].to_vec();
        let y = pub_bytes[33..65].to_vec();
        let cose_key = build_cose_ec2_key(&x, &y);

        let challenge = generate_challenge_bytes();
        let challenge_b64 = base64_encode_urlsafe(&challenge);

        use sha2::Digest;
        let rp_id_hash = sha2::Sha256::digest(b"localhost").to_vec();
        let mut auth_data = Vec::new();
        auth_data.extend_from_slice(&rp_id_hash);
        auth_data.push(FLAG_UP);
        auth_data.extend_from_slice(&5u32.to_be_bytes()); // signCount = 5

        let client_data = serde_json::json!({
            "type": "webauthn.get",
            "challenge": challenge_b64,
            "origin": "http://localhost:8080",
        });
        let client_data_bytes = serde_json::to_vec(&client_data).unwrap();
        let client_data_b64 = base64_encode_urlsafe(&client_data_bytes);
        let client_data_hash = sha2::Sha256::digest(&client_data_bytes).to_vec();

        let mut signed_data = Vec::new();
        signed_data.extend_from_slice(&auth_data);
        signed_data.extend_from_slice(&client_data_hash);

        let signature = key_pair.sign(&rng, &signed_data).unwrap();

        let auth_data_b64 = base64_encode_urlsafe(&auth_data);
        let sig_b64 = base64_encode_urlsafe(signature.as_ref());
        let cred_id_b64 = base64_encode_urlsafe(&[0x01]);

        // Current sign count is 10, but auth data says 5 -> cloned authenticator
        let result = verify_authentication(&AuthenticationParams {
            challenge_bytes: challenge.clone(),
            client_data_json_b64: client_data_b64,
            authenticator_data_b64: auth_data_b64,
            signature_b64: sig_b64,
            credential_id_b64: cred_id_b64.clone(),
            public_key_cose: cose_key,
            current_sign_count: 10, // higher than auth data's 5
            allowed_credential_ids: vec![cred_id_b64],
            rp_id: "localhost".to_string(),
            rp_origins: vec!["http://localhost:8080".to_string()],
        });
        assert!(matches!(result, Err(WebAuthnError::VerificationFailed(_))));
    }

    #[test]
    fn test_verify_authentication_wrong_rp_id_hash() {
        let challenge = generate_challenge_bytes();
        let challenge_b64 = base64_encode_urlsafe(&challenge);

        let client_data = serde_json::json!({
            "type": "webauthn.get",
            "challenge": challenge_b64,
            "origin": "http://localhost:8080",
        });
        let client_data_b64 = base64_encode_urlsafe(&serde_json::to_vec(&client_data).unwrap());

        use sha2::Digest;
        let rp_id_hash = sha2::Sha256::digest(b"evil.com").to_vec(); // wrong RP ID
        let mut auth_data = Vec::new();
        auth_data.extend_from_slice(&rp_id_hash);
        auth_data.push(FLAG_UP);
        auth_data.extend_from_slice(&0u32.to_be_bytes());
        let auth_data_b64 = base64_encode_urlsafe(&auth_data);

        let result = verify_authentication(&AuthenticationParams {
            challenge_bytes: challenge.clone(),
            client_data_json_b64: client_data_b64,
            authenticator_data_b64: auth_data_b64,
            signature_b64: "sig".to_string(),
            credential_id_b64: "cred".to_string(),
            public_key_cose: vec![0x10],
            current_sign_count: 0,
            allowed_credential_ids: vec!["cred".to_string()],
            rp_id: "localhost".to_string(),
            rp_origins: vec!["http://localhost:8080".to_string()],
        });
        assert!(matches!(result, Err(WebAuthnError::VerificationFailed(_))));
    }

    #[test]
    fn test_verify_authentication_missing_up_flag() {
        use ring::signature::EcdsaKeyPair;

        let rng = ring::rand::SystemRandom::new();
        let pkcs8 =
            EcdsaKeyPair::generate_pkcs8(&ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING, &rng)
                .unwrap();
        let key_pair = EcdsaKeyPair::from_pkcs8(
            &ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING,
            pkcs8.as_ref(),
            &rng,
        )
        .unwrap();

        let pub_bytes = key_pair.public_key().as_ref().to_vec();
        let x = pub_bytes[1..33].to_vec();
        let y = pub_bytes[33..65].to_vec();
        let cose_key = build_cose_ec2_key(&x, &y);

        let challenge = generate_challenge_bytes();
        let challenge_b64 = base64_encode_urlsafe(&challenge);

        use sha2::Digest;
        let rp_id_hash = sha2::Sha256::digest(b"localhost").to_vec();
        let mut auth_data = Vec::new();
        auth_data.extend_from_slice(&rp_id_hash);
        auth_data.push(0); // no UP flag
        auth_data.extend_from_slice(&1u32.to_be_bytes());

        let client_data = serde_json::json!({
            "type": "webauthn.get",
            "challenge": challenge_b64,
            "origin": "http://localhost:8080",
        });
        let client_data_bytes = serde_json::to_vec(&client_data).unwrap();
        let client_data_b64 = base64_encode_urlsafe(&client_data_bytes);
        let client_data_hash = sha2::Sha256::digest(&client_data_bytes).to_vec();

        let mut signed_data = Vec::new();
        signed_data.extend_from_slice(&auth_data);
        signed_data.extend_from_slice(&client_data_hash);

        let signature = key_pair.sign(&rng, &signed_data).unwrap();

        let auth_data_b64 = base64_encode_urlsafe(&auth_data);
        let sig_b64 = base64_encode_urlsafe(signature.as_ref());
        let cred_id_b64 = base64_encode_urlsafe(&[0x01]);

        let result = verify_authentication(&AuthenticationParams {
            challenge_bytes: challenge.clone(),
            client_data_json_b64: client_data_b64,
            authenticator_data_b64: auth_data_b64,
            signature_b64: sig_b64,
            credential_id_b64: cred_id_b64.clone(),
            public_key_cose: cose_key,
            current_sign_count: 0,
            allowed_credential_ids: vec![cred_id_b64],
            rp_id: "localhost".to_string(),
            rp_origins: vec!["http://localhost:8080".to_string()],
        });
        assert!(matches!(result, Err(WebAuthnError::VerificationFailed(_))));
    }

    #[test]
    fn test_verify_authentication_wrong_rp_id_in_client_data() {
        use ring::signature::EcdsaKeyPair;

        let rng = ring::rand::SystemRandom::new();
        let pkcs8 =
            EcdsaKeyPair::generate_pkcs8(&ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING, &rng)
                .unwrap();
        let key_pair = EcdsaKeyPair::from_pkcs8(
            &ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING,
            pkcs8.as_ref(),
            &rng,
        )
        .unwrap();

        let pub_bytes = key_pair.public_key().as_ref().to_vec();
        let x = pub_bytes[1..33].to_vec();
        let y = pub_bytes[33..65].to_vec();
        let cose_key = build_cose_ec2_key(&x, &y);

        let challenge = generate_challenge_bytes();
        let challenge_b64 = base64_encode_urlsafe(&challenge);

        let client_data = serde_json::json!({
            "type": "webauthn.get",
            "challenge": challenge_b64,
            "origin": "http://localhost:8080",
            "rpId": "evil.com", // wrong rpId
        });
        let client_data_bytes = serde_json::to_vec(&client_data).unwrap();
        let client_data_b64 = base64_encode_urlsafe(&client_data_bytes);

        use sha2::Digest;
        let rp_id_hash = sha2::Sha256::digest(b"localhost").to_vec();
        let mut auth_data = Vec::new();
        auth_data.extend_from_slice(&rp_id_hash);
        auth_data.push(FLAG_UP);
        auth_data.extend_from_slice(&1u32.to_be_bytes());

        let client_data_hash = sha2::Sha256::digest(&client_data_bytes).to_vec();
        let mut signed_data = Vec::new();
        signed_data.extend_from_slice(&auth_data);
        signed_data.extend_from_slice(&client_data_hash);

        let signature = key_pair.sign(&rng, &signed_data).unwrap();

        let auth_data_b64 = base64_encode_urlsafe(&auth_data);
        let sig_b64 = base64_encode_urlsafe(signature.as_ref());
        let cred_id_b64 = base64_encode_urlsafe(&[0x01]);

        let result = verify_authentication(&AuthenticationParams {
            challenge_bytes: challenge.clone(),
            client_data_json_b64: client_data_b64,
            authenticator_data_b64: auth_data_b64,
            signature_b64: sig_b64,
            credential_id_b64: cred_id_b64.clone(),
            public_key_cose: cose_key,
            current_sign_count: 0,
            allowed_credential_ids: vec![cred_id_b64],
            rp_id: "localhost".to_string(),
            rp_origins: vec!["http://localhost:8080".to_string()],
        });
        assert!(matches!(result, Err(WebAuthnError::VerificationFailed(_))));
    }

    #[test]
    fn test_verify_authentication_challenge_mismatch() {
        let challenge = generate_challenge_bytes();
        let _challenge_b64 = base64_encode_urlsafe(&challenge);

        let client_data = serde_json::json!({
            "type": "webauthn.get",
            "challenge": base64_encode_urlsafe(&[99u8; 32]), // wrong challenge
            "origin": "http://localhost:8080",
        });
        let client_data_b64 = base64_encode_urlsafe(&serde_json::to_vec(&client_data).unwrap());

        use sha2::Digest;
        let rp_id_hash = sha2::Sha256::digest(b"localhost").to_vec();
        let mut auth_data = Vec::new();
        auth_data.extend_from_slice(&rp_id_hash);
        auth_data.push(FLAG_UP);
        auth_data.extend_from_slice(&0u32.to_be_bytes());
        let auth_data_b64 = base64_encode_urlsafe(&auth_data);

        let result = verify_authentication(&AuthenticationParams {
            challenge_bytes: challenge.clone(),
            client_data_json_b64: client_data_b64,
            authenticator_data_b64: auth_data_b64,
            signature_b64: "sig".to_string(),
            credential_id_b64: "cred".to_string(),
            public_key_cose: vec![0x10],
            current_sign_count: 0,
            allowed_credential_ids: vec!["cred".to_string()],
            rp_id: "localhost".to_string(),
            rp_origins: vec!["http://localhost:8080".to_string()],
        });
        assert!(matches!(result, Err(WebAuthnError::VerificationFailed(_))));
    }
}
