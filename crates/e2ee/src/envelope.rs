use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, AeadCore};
use hkdf::Hkdf;
use sha2::{Digest, Sha256};

use crate::error::E2eeError;
use crate::key::E2eeKeyPair;

const ENVELOPE_KEY_INFO: &[u8] = b"ferro-e2ee-envelope";

#[derive(Debug, Clone)]
pub struct KeyEnvelope {
    pub recipient_key_id: [u8; 32],
    pub encrypted_file_key: Vec<u8>,
    pub sender_key_id: [u8; 32],
    pub signature: Vec<u8>,
}

fn derive_envelope_key(recipient_public_key: &[u8]) -> Result<[u8; 32], E2eeError> {
    let hk = Hkdf::<Sha256>::new(None, recipient_public_key);

    let mut key = [0u8; 32];
    hk.expand(ENVELOPE_KEY_INFO, &mut key)
        .map_err(|e| E2eeError::Encryption {
            message: e.to_string(),
        })?;
    Ok(key)
}

fn compute_sender_signature(sender: &E2eeKeyPair, recipient_public_key: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(sender.private_key_bytes());
    hasher.update(recipient_public_key);
    let result = hasher.finalize();
    let mut sig = [0u8; 32];
    sig.copy_from_slice(&result);
    sig
}

fn recipient_key_id_from_public_key(public_key: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(public_key);
    let result = hasher.finalize();
    let mut id = [0u8; 32];
    id.copy_from_slice(&result);
    id
}

pub fn create_envelope(
    sender: &E2eeKeyPair,
    recipient_public_key: &[u8],
    file_key: &[u8],
) -> Result<KeyEnvelope, E2eeError> {
    let envelope_key = derive_envelope_key(recipient_public_key)?;
    let cipher = Aes256Gcm::new_from_slice(&envelope_key).map_err(|e| E2eeError::Encryption {
        message: e.to_string(),
    })?;

    let nonce = Aes256Gcm::generate_nonce(rand::rngs::OsRng);
    let encrypted = cipher.encrypt(&nonce, file_key)?;

    let mut encrypted_file_key = Vec::with_capacity(12 + encrypted.len());
    encrypted_file_key.extend_from_slice(&nonce);
    encrypted_file_key.extend_from_slice(&encrypted);

    let recipient_key_id = recipient_key_id_from_public_key(recipient_public_key);
    let signature = compute_sender_signature(sender, recipient_public_key);

    Ok(KeyEnvelope {
        recipient_key_id,
        encrypted_file_key,
        sender_key_id: sender.key_id(),
        signature: signature.to_vec(),
    })
}

pub fn open_envelope(
    recipient: &E2eeKeyPair,
    envelope: &KeyEnvelope,
) -> Result<Vec<u8>, E2eeError> {
    let expected_key_id = recipient.key_id();
    if envelope.recipient_key_id != expected_key_id {
        return Err(E2eeError::Decryption {
            message: "Key ID mismatch: envelope not intended for this recipient".into(),
        });
    }

    let envelope_key = derive_envelope_key(recipient.public_key_bytes())?;
    let cipher = Aes256Gcm::new_from_slice(&envelope_key).map_err(|e| E2eeError::Decryption {
        message: e.to_string(),
    })?;

    let nonce = aes_gcm::Nonce::from_slice(&envelope.encrypted_file_key[..12]);
    let ciphertext_with_tag = &envelope.encrypted_file_key[12..];
    let file_key = cipher
        .decrypt(nonce, ciphertext_with_tag)
        .map_err(|e| E2eeError::Decryption {
            message: e.to_string(),
        })?;

    Ok(file_key)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_key() -> E2eeKeyPair {
        E2eeKeyPair::generate().unwrap()
    }

    #[test]
    fn test_envelope_roundtrip() {
        let sender = make_key();
        let recipient = make_key();
        let file_key = b"this-is-a-32-byte-enc-key-padded!";

        let envelope = create_envelope(&sender, recipient.public_key_bytes(), file_key).unwrap();
        let recovered = open_envelope(&recipient, &envelope).unwrap();
        assert_eq!(file_key.as_slice(), recovered.as_slice());
    }

    #[test]
    fn test_wrong_recipient_key_returns_error() {
        let sender = make_key();
        let recipient = make_key();
        let wrong_recipient = make_key();
        let file_key = b"this-is-a-32-byte-enc-key-padded!";

        let envelope = create_envelope(&sender, recipient.public_key_bytes(), file_key).unwrap();
        let result = open_envelope(&wrong_recipient, &envelope);
        assert!(result.is_err());
    }

    #[test]
    fn test_envelope_sender_key_id_matches() {
        let sender = make_key();
        let recipient = make_key();
        let file_key = b"this-is-a-32-byte-enc-key-padded!";

        let envelope = create_envelope(&sender, recipient.public_key_bytes(), file_key).unwrap();
        assert_eq!(envelope.sender_key_id, sender.key_id());
    }
}
