use rand::RngCore;
use sha2::{Digest, Sha256};
use zeroize::Zeroize;

use crate::error::E2eeError;

pub struct E2eeKeyPair {
    public_key: Vec<u8>,
    private_key: Vec<u8>,
}

impl Drop for E2eeKeyPair {
    fn drop(&mut self) {
        self.public_key.zeroize();
        self.private_key.zeroize();
    }
}

impl E2eeKeyPair {
    pub fn generate() -> Result<Self, E2eeError> {
        let mut secret_bytes = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut secret_bytes);

        let private_key = secret_bytes.to_vec();
        let public_key =
            x25519_dalek::x25519(secret_bytes, x25519_dalek::X25519_BASEPOINT_BYTES).to_vec();

        Ok(Self {
            public_key,
            private_key,
        })
    }

    pub fn public_key_bytes(&self) -> &[u8] {
        &self.public_key
    }

    pub fn key_id(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(&self.public_key);
        let result = hasher.finalize();
        let mut id = [0u8; 32];
        id.copy_from_slice(&result);
        id
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let pub_len = self.public_key.len() as u32;
        let priv_len = self.private_key.len() as u32;
        let mut out = Vec::with_capacity(8 + self.public_key.len() + self.private_key.len());
        out.extend_from_slice(&pub_len.to_le_bytes());
        out.extend_from_slice(&priv_len.to_le_bytes());
        out.extend_from_slice(&self.public_key);
        out.extend_from_slice(&self.private_key);
        out
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, E2eeError> {
        if data.len() < 8 {
            return Err(E2eeError::InvalidKey {
                reason: "Data too short".into(),
            });
        }

        let pub_len = u32::from_le_bytes(data[0..4].try_into().unwrap()) as usize;
        let priv_len = u32::from_le_bytes(data[4..8].try_into().unwrap()) as usize;

        if data.len() != 8 + pub_len + priv_len {
            return Err(E2eeError::InvalidKey {
                reason: format!(
                    "Length mismatch: expected {}, got {}",
                    8 + pub_len + priv_len,
                    data.len()
                ),
            });
        }

        let public_key = data[8..8 + pub_len].to_vec();
        let private_key = data[8 + pub_len..8 + pub_len + priv_len].to_vec();

        Ok(Self {
            public_key,
            private_key,
        })
    }

    pub(crate) fn private_key_bytes(&self) -> &[u8] {
        &self.private_key
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_roundtrip() {
        let kp = E2eeKeyPair::generate().unwrap();
        assert_eq!(kp.public_key_bytes().len(), 32);
        assert_eq!(kp.private_key_bytes().len(), 32);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let kp = E2eeKeyPair::generate().unwrap();
        let bytes = kp.to_bytes();
        let kp2 = E2eeKeyPair::from_bytes(&bytes).unwrap();
        assert_eq!(kp.public_key_bytes(), kp2.public_key_bytes());
        assert_eq!(kp.private_key_bytes(), kp2.private_key_bytes());
    }

    #[test]
    fn test_key_id_consistency() {
        let kp = E2eeKeyPair::generate().unwrap();
        let id1 = kp.key_id();
        let id2 = kp.key_id();
        assert_eq!(id1, id2);
        assert_eq!(id1.len(), 32);
    }

    #[test]
    fn test_from_bytes_invalid() {
        let result = E2eeKeyPair::from_bytes(&[0, 1, 2]);
        assert!(result.is_err());
    }

    #[test]
    fn test_different_keys_have_different_ids() {
        let kp1 = E2eeKeyPair::generate().unwrap();
        let kp2 = E2eeKeyPair::generate().unwrap();
        assert_ne!(kp1.key_id(), kp2.key_id());
    }
}
