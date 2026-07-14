use super::credential::{AuthenticationResult, RegistrationResult};
use super::crypto::{
    alg_to_name, base64_decode_urlsafe, base64_encode_urlsafe, cbor_bytes, cbor_map_entries, parse_cose_key,
    verify_cose_signature,
};
use super::error::WebAuthnError;

/// Parsed authenticator data structure (CTAP2 §6.1).
struct AuthenticatorData {
    rp_id_hash: Vec<u8>,
    flags: u8,
    sign_count: u32,
    credential_public_key_cose: Option<Vec<u8>>,
    credential_id: Option<Vec<u8>>,
}

/// Authenticator data flag bits.
const FLAG_UP: u8 = 0x01;
const FLAG_UV: u8 = 0x04;
const FLAG_AT: u8 = 0x40;
#[allow(dead_code)]
const FLAG_ED: u8 = 0x80;

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
    let sign_count = u32::from_be_bytes(
        auth_data[33..37]
            .try_into()
            .map_err(|_| WebAuthnError::VerificationFailed("sign count bytes invalid".to_string()))?,
    );

    let mut offset = 37;
    let mut credential_id = None;
    let mut credential_public_key_cose = None;

    if flags & FLAG_AT != 0 {
        if auth_data.len() < offset + 18 {
            return Err(WebAuthnError::VerificationFailed(
                "Attested credential data truncated (AAGUID + length)".to_string(),
            ));
        }
        offset += 16;

        let cred_id_len = u16::from_be_bytes([auth_data[offset], auth_data[offset + 1]]) as usize;
        offset += 2;

        if auth_data.len() < offset + cred_id_len {
            return Err(WebAuthnError::VerificationFailed(
                "Attested credential data truncated (credential ID)".to_string(),
            ));
        }
        credential_id = Some(auth_data[offset..offset + cred_id_len].to_vec());
        offset += cred_id_len;

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

/// Verify a registration response with full CTAP2/COSE verification.
pub fn verify_registration(
    challenge_bytes: &[u8],
    client_data_json_b64: &str,
    attestation_object_b64: &str,
    existing_credential_id: &str,
    rp_id: &str,
    rp_origins: &[String],
) -> Result<RegistrationResult, WebAuthnError> {
    let client_data_bytes = base64_decode_urlsafe(client_data_json_b64)?;
    let client_data: serde_json::Value = serde_json::from_slice(&client_data_bytes)
        .map_err(|e| WebAuthnError::VerificationFailed(format!("client data parse error: {e}")))?;

    let client_challenge = client_data
        .get("challenge")
        .and_then(|v| v.as_str())
        .ok_or_else(|| WebAuthnError::VerificationFailed("missing challenge in client data".to_string()))?;
    let client_challenge_bytes = base64_decode_urlsafe(client_challenge)?;
    if client_challenge_bytes != challenge_bytes {
        return Err(WebAuthnError::VerificationFailed("challenge mismatch".to_string()));
    }

    let typ = client_data
        .get("type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| WebAuthnError::VerificationFailed("missing type in client data".to_string()))?;
    if typ != "webauthn.create" {
        return Err(WebAuthnError::VerificationFailed(format!("wrong type: {typ}")));
    }

    let origin = client_data
        .get("origin")
        .and_then(|v| v.as_str())
        .ok_or_else(|| WebAuthnError::VerificationFailed("missing origin in client data".to_string()))?;
    if !rp_origins.iter().any(|o| o == origin) {
        return Err(WebAuthnError::VerificationFailed(format!(
            "origin '{origin}' not allowed"
        )));
    }

    let rp_id_val = client_data.get("rpId").and_then(|v| v.as_str());
    if let Some(rp) = rp_id_val
        && rp != rp_id
    {
        return Err(WebAuthnError::VerificationFailed(format!(
            "rpId mismatch: client sent '{rp}', expected '{rp_id}'"
        )));
    }

    let attestation_bytes = base64_decode_urlsafe(attestation_object_b64)?;
    let attestation_val: ciborium::Value = ciborium::de::from_reader(&attestation_bytes[..])
        .map_err(|e| WebAuthnError::AttestationError(format!("attestation object CBOR parse error: {e}")))?;

    let attestation_entries = cbor_map_entries(&attestation_val)
        .ok_or_else(|| WebAuthnError::AttestationError("attestation object is not a CBOR map".to_string()))?;

    let mut fmt: Option<String> = None;
    let mut auth_data_bytes: Option<Vec<u8>> = None;

    for (key, val) in &attestation_entries {
        match *key {
            1 => {
                if let ciborium::Value::Text(s) = val {
                    fmt = Some(s.clone());
                }
            }
            2 => {
                if let Some(b) = cbor_bytes(val) {
                    auth_data_bytes = Some(b);
                }
            }
            _ => {}
        }
    }

    let _fmt = fmt.unwrap_or_else(|| "none".to_string());
    let auth_data_bytes = auth_data_bytes
        .ok_or_else(|| WebAuthnError::AttestationError("missing authData in attestation object".to_string()))?;

    let auth_data = parse_authenticator_data(&auth_data_bytes)?;

    use sha2::Digest;
    let computed_rp_id_hash = sha2::Sha256::digest(rp_id.as_bytes()).to_vec();
    if auth_data.rp_id_hash != computed_rp_id_hash {
        return Err(WebAuthnError::VerificationFailed(format!(
            "rpId hash mismatch: computed {:x?}, got {:x?}",
            computed_rp_id_hash, auth_data.rp_id_hash
        )));
    }

    if auth_data.flags & FLAG_UP == 0 {
        return Err(WebAuthnError::VerificationFailed(
            "User Present flag not set".to_string(),
        ));
    }

    if auth_data.flags & FLAG_AT == 0 {
        return Err(WebAuthnError::VerificationFailed(
            "Attested Credential Data flag not set during registration".to_string(),
        ));
    }

    let credential_id = auth_data
        .credential_id
        .ok_or_else(|| WebAuthnError::VerificationFailed("no credential ID in attested data".to_string()))?;
    let credential_id_b64 = base64_encode_urlsafe(&credential_id);

    if existing_credential_id == credential_id_b64 {
        return Err(WebAuthnError::DuplicateCredential(credential_id_b64));
    }

    let public_key_cose = auth_data
        .credential_public_key_cose
        .ok_or_else(|| WebAuthnError::VerificationFailed("no public key in attested data".to_string()))?;

    let (alg, _cose_key) = parse_cose_key(&public_key_cose)?;

    let user_verified = auth_data.flags & FLAG_UV != 0;

    Ok(RegistrationResult {
        credential_id: credential_id_b64,
        device_name: format!("WebAuthn ({})", alg_to_name(alg)),
        attestation_format: _fmt,
        user_verified,
    })
}

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
pub fn verify_authentication(params: &AuthenticationParams) -> Result<AuthenticationResult, WebAuthnError> {
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

    if !allowed_credential_ids.contains(&credential_id_b64.clone()) {
        return Err(WebAuthnError::VerificationFailed(
            "credential ID not in allowed list".to_string(),
        ));
    }

    let client_data_bytes = base64_decode_urlsafe(client_data_json_b64)?;
    let client_data: serde_json::Value = serde_json::from_slice(&client_data_bytes)
        .map_err(|e| WebAuthnError::VerificationFailed(format!("client data parse error: {e}")))?;

    let client_challenge = client_data
        .get("challenge")
        .and_then(|v| v.as_str())
        .ok_or_else(|| WebAuthnError::VerificationFailed("missing challenge".to_string()))?;
    let client_challenge_bytes = base64_decode_urlsafe(client_challenge)?;
    if client_challenge_bytes != challenge_bytes[..] {
        return Err(WebAuthnError::VerificationFailed("challenge mismatch".to_string()));
    }

    let typ = client_data
        .get("type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| WebAuthnError::VerificationFailed("missing type".to_string()))?;
    if typ != "webauthn.get" {
        return Err(WebAuthnError::VerificationFailed(format!("wrong type: {typ}")));
    }

    let origin = client_data
        .get("origin")
        .and_then(|v| v.as_str())
        .ok_or_else(|| WebAuthnError::VerificationFailed("missing origin".to_string()))?;
    if !rp_origins.iter().any(|o| o == origin) {
        return Err(WebAuthnError::VerificationFailed(format!(
            "origin '{origin}' not allowed"
        )));
    }

    let rp_id_val = client_data.get("rpId").and_then(|v| v.as_str());
    if let Some(rp) = rp_id_val
        && rp != rp_id
    {
        return Err(WebAuthnError::VerificationFailed(format!(
            "rpId mismatch: client sent '{rp}', expected '{rp_id}'"
        )));
    }

    let authenticator_data = base64_decode_urlsafe(authenticator_data_b64)?;
    let auth_data = parse_authenticator_data(&authenticator_data)?;

    use sha2::Digest;
    let computed_rp_id_hash = sha2::Sha256::digest(rp_id.as_bytes()).to_vec();
    if auth_data.rp_id_hash != computed_rp_id_hash {
        return Err(WebAuthnError::VerificationFailed(format!(
            "rpId hash mismatch: computed {:x?}, got {:x?}",
            computed_rp_id_hash, auth_data.rp_id_hash
        )));
    }

    if auth_data.flags & FLAG_UP == 0 {
        return Err(WebAuthnError::VerificationFailed(
            "User Present flag not set".to_string(),
        ));
    }

    if current_sign_count != 0
        && auth_data.sign_count <= current_sign_count
        && auth_data.sign_count < current_sign_count
    {
        return Err(WebAuthnError::VerificationFailed(format!(
            "Sign count decreased: {} < {} (possible cloned authenticator)",
            auth_data.sign_count, current_sign_count
        )));
    }

    let client_data_hash = sha2::Sha256::digest(&client_data_bytes).to_vec();

    let mut signed_data = Vec::with_capacity(authenticator_data.len() + 32);
    signed_data.extend_from_slice(&authenticator_data);
    signed_data.extend_from_slice(&client_data_hash);

    let signature = base64_decode_urlsafe(signature_b64)?;

    let (alg, cose_key) = parse_cose_key(public_key_cose)?;
    verify_cose_signature(alg, &cose_key, &signed_data, &signature)?;

    let user_verified = auth_data.flags & FLAG_UV != 0;
    let new_sign_count = auth_data.sign_count;

    Ok(AuthenticationResult {
        credential_id: credential_id_b64.clone(),
        new_sign_count,
        user_verified,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::webauthn::crypto::{base64_encode_urlsafe, generate_challenge_bytes};
    use ring::signature::KeyPair as _;

    use super::super::crypto::tests::build_cose_ec2_key;

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
            auth_data.extend_from_slice(&[0u8; 16]);
            auth_data.extend_from_slice(&(credential_id.len() as u16).to_be_bytes());
            auth_data.extend_from_slice(credential_id);
            auth_data.extend_from_slice(cose_key);
        }

        auth_data
    }

    #[test]
    fn test_parse_authenticator_data_minimal() {
        let mut auth_data = vec![0u8; 37];
        auth_data[..32].copy_from_slice(&[0xAA; 32]);
        auth_data[32] = FLAG_UP;
        auth_data[33..37].copy_from_slice(&1u32.to_be_bytes());

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

        let total_len = 37 + 16 + 2 + credential_id.len() + public_key_cose.len();
        let mut auth_data = vec![0u8; total_len];
        auth_data[32] = FLAG_UP | FLAG_AT;
        auth_data[33..37].copy_from_slice(&5u32.to_be_bytes());

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
        let auth_data = vec![0u8; 10];
        let result = parse_authenticator_data(&auth_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_registration_valid_es256() {
        use ring::signature::EcdsaKeyPair;

        let rng = ring::rand::SystemRandom::new();
        let pkcs8 = EcdsaKeyPair::generate_pkcs8(&ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING, &rng).unwrap();
        let key_pair =
            EcdsaKeyPair::from_pkcs8(&ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING, pkcs8.as_ref(), &rng).unwrap();

        let pub_bytes = key_pair.public_key().as_ref().to_vec();
        let x = pub_bytes[1..33].to_vec();
        let y = pub_bytes[33..65].to_vec();
        let cose_key = build_cose_ec2_key(&x, &y);

        let credential_id = vec![0x01, 0x02, 0x03, 0x04];
        let auth_data = build_auth_data_with_credential("localhost", FLAG_UP | FLAG_AT, 0, &credential_id, &cose_key);
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
            "challenge": base64_encode_urlsafe(&[1u8; 32]),
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
            "type": "webauthn.get",
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
            &cred_id_b64,
            "localhost",
            &["http://localhost:8080".to_string()],
        );
        assert!(matches!(result, Err(WebAuthnError::DuplicateCredential(_))));
    }

    #[test]
    fn test_verify_registration_wrong_rp_id_hash() {
        let credential_id = vec![0x01];
        let cose_key = build_cose_ec2_key(&[0xAA; 32], &[0xBB; 32]);
        let auth_data = build_auth_data_with_credential("evil.com", FLAG_UP | FLAG_AT, 0, &credential_id, &cose_key);
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
        let auth_data = build_auth_data_with_credential("localhost", FLAG_AT, 0, &credential_id, &cose_key);
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
    fn test_verify_authentication_valid_es256() {
        use ring::signature::EcdsaKeyPair;

        let rng = ring::rand::SystemRandom::new();
        let pkcs8 = EcdsaKeyPair::generate_pkcs8(&ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING, &rng).unwrap();
        let key_pair =
            EcdsaKeyPair::from_pkcs8(&ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING, pkcs8.as_ref(), &rng).unwrap();

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
        let pkcs8 = EcdsaKeyPair::generate_pkcs8(&ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING, &rng).unwrap();
        let key_pair =
            EcdsaKeyPair::from_pkcs8(&ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING, pkcs8.as_ref(), &rng).unwrap();

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
        wrong_sig[0] = 0xFF;

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
        assert!(matches!(result, Err(WebAuthnError::SignatureVerificationFailed)));
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
            "type": "webauthn.create",
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
        let pkcs8 = EcdsaKeyPair::generate_pkcs8(&ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING, &rng).unwrap();
        let key_pair =
            EcdsaKeyPair::from_pkcs8(&ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING, pkcs8.as_ref(), &rng).unwrap();

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
        auth_data.extend_from_slice(&5u32.to_be_bytes());

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
            current_sign_count: 10,
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
        let rp_id_hash = sha2::Sha256::digest(b"evil.com").to_vec();
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
        let pkcs8 = EcdsaKeyPair::generate_pkcs8(&ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING, &rng).unwrap();
        let key_pair =
            EcdsaKeyPair::from_pkcs8(&ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING, pkcs8.as_ref(), &rng).unwrap();

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
        auth_data.push(0);
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
        let pkcs8 = EcdsaKeyPair::generate_pkcs8(&ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING, &rng).unwrap();
        let key_pair =
            EcdsaKeyPair::from_pkcs8(&ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING, pkcs8.as_ref(), &rng).unwrap();

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
            "rpId": "evil.com",
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
            "challenge": base64_encode_urlsafe(&[99u8; 32]),
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
