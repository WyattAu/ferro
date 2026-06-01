use rand::RngCore;
use sha2::{Digest, Sha256};
use zeroize::Zeroize;

#[derive(Zeroize)]
pub struct SessionToken {
    bytes: [u8; 32],
}

impl SessionToken {
    pub fn new() -> Self {
        let mut bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut bytes);
        Self { bytes }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.bytes)
    }
}

impl Default for SessionToken {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for SessionToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SessionToken([redacted])")
    }
}

impl Drop for SessionToken {
    fn drop(&mut self) {
        self.bytes.zeroize();
    }
}

pub fn hash_token(token: &SessionToken) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn verify_token_hash(token: &SessionToken, expected_hash: &str) -> bool {
    hash_token(token) == expected_hash
}
