//! E2EE file encryption integration.
//!
//! Provides helper functions for encrypting files before upload
//! and decrypting after download using the ferro-e2ee crate.

use ferro_e2ee::encrypt::{EncryptionConfig, decrypt_file, encrypt_file};
use ferro_e2ee::envelope::{KeyEnvelope, create_envelope, open_envelope};
use ferro_e2ee::key::E2eeKeyPair;

pub fn generate_user_keypair() -> Result<E2eeKeyPair, ferro_e2ee::error::E2eeError> {
    E2eeKeyPair::generate()
}

pub fn encrypt_file_data(
    key: &E2eeKeyPair,
    plaintext: &[u8],
) -> Result<ferro_e2ee::encrypt::EncryptedFile, String> {
    encrypt_file(key, plaintext, &EncryptionConfig::default()).map_err(|e| e.to_string())
}

pub fn decrypt_file_data(
    key: &E2eeKeyPair,
    encrypted: &ferro_e2ee::encrypt::EncryptedFile,
) -> Result<Vec<u8>, String> {
    decrypt_file(key, encrypted).map_err(|e| e.to_string())
}

pub fn create_key_envelope(
    sender: &E2eeKeyPair,
    recipient_public: &[u8],
    file_key: &[u8],
) -> Result<KeyEnvelope, String> {
    create_envelope(sender, recipient_public, file_key).map_err(|e| e.to_string())
}

pub fn open_key_envelope(
    recipient: &E2eeKeyPair,
    envelope: &KeyEnvelope,
) -> Result<Vec<u8>, String> {
    open_envelope(recipient, envelope).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_keypair() -> E2eeKeyPair {
        E2eeKeyPair::generate().unwrap()
    }

    #[test]
    fn test_generate_keypair() {
        let kp = generate_user_keypair().unwrap();
        assert_eq!(kp.public_key_bytes().len(), 32);
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = make_keypair();
        let data = b"hello e2ee integration";
        let encrypted = encrypt_file_data(&key, data).unwrap();
        let decrypted = decrypt_file_data(&key, &encrypted).unwrap();
        assert_eq!(data.as_slice(), decrypted.as_slice());
    }

    #[test]
    fn test_wrong_key_decrypt_fails() {
        let key = make_keypair();
        let wrong_key = make_keypair();
        let data = b"secret";
        let encrypted = encrypt_file_data(&key, data).unwrap();
        assert!(decrypt_file_data(&wrong_key, &encrypted).is_err());
    }

    #[test]
    fn test_envelope_roundtrip() {
        let sender = make_keypair();
        let recipient = make_keypair();
        let file_key = b"this-is-a-32-byte-enc-key-padded!";
        let envelope =
            create_key_envelope(&sender, recipient.public_key_bytes(), file_key).unwrap();
        let recovered = open_key_envelope(&recipient, &envelope).unwrap();
        assert_eq!(file_key.as_slice(), recovered.as_slice());
    }
}
