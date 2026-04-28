use async_trait::async_trait;
use ring::{digest, hmac, rand};
use ring::rand::SecureRandom;

use super::traits::{CryptoProvider, Result};
use crate::CryptoError;

pub struct RingProvider {
    _private: (),
}

impl RingProvider {
    pub fn new() -> Self {
        Self { _private: () }
    }
}

impl Default for RingProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CryptoProvider for RingProvider {
    async fn sha256(&self, data: &[u8]) -> Result<Vec<u8>> {
        let data = data.to_vec();
        tokio::task::spawn_blocking(move || {
            let mut ctx = digest::Context::new(&digest::SHA256);
            ctx.update(&data);
            Ok(ctx.finish().as_ref().to_vec())
        })
        .await
        .map_err(|e| CryptoError::Internal(format!("SHA-256 task error: {e}")))?
    }

    async fn sha512(&self, data: &[u8]) -> Result<Vec<u8>> {
        let data = data.to_vec();
        tokio::task::spawn_blocking(move || {
            let mut ctx = digest::Context::new(&digest::SHA512);
            ctx.update(&data);
            Ok(ctx.finish().as_ref().to_vec())
        })
        .await
        .map_err(|e| CryptoError::Internal(format!("SHA-512 task error: {e}")))?
    }

    async fn hmac_sha256(&self, key: &[u8], data: &[u8]) -> Result<Vec<u8>> {
        let key = key.to_vec();
        let data = data.to_vec();
        tokio::task::spawn_blocking(move || {
            let key = hmac::Key::new(hmac::HMAC_SHA256, &key);
            let tag = hmac::sign(&key, &data);
            Ok(tag.as_ref().to_vec())
        })
        .await
        .map_err(|e| CryptoError::Internal(format!("HMAC task error: {e}")))?
    }

    async fn random_bytes(&self, len: usize) -> Result<Vec<u8>> {
        tokio::task::spawn_blocking(move || {
            let rng = rand::SystemRandom::new();
            let mut buf = vec![0u8; len];
            rng.fill(&mut buf)
                .map_err(|e| CryptoError::RandomGeneration(e.to_string()))?;
            Ok(buf)
        })
        .await
        .map_err(|e| CryptoError::Internal(format!("Random task error: {e}")))?
    }

    async fn hash_password(&self, password: &str) -> Result<String> {
        let password = password.to_string();
        tokio::task::spawn_blocking(move || {
            bcrypt::hash(&password, bcrypt::DEFAULT_COST)
                .map_err(|e| CryptoError::PasswordHash(e.to_string()))
        })
        .await
        .map_err(|e| CryptoError::Internal(format!("Hash task error: {e}")))?
    }

    async fn verify_password(&self, password: &str, hash: &str) -> Result<bool> {
        let password = password.to_string();
        let hash = hash.to_string();
        tokio::task::spawn_blocking(move || Ok(bcrypt::verify(&password, &hash).unwrap_or(false)))
            .await
            .map_err(|e| CryptoError::Internal(format!("Verify task error: {e}")))?
    }

    async fn generate_token(&self, len: usize) -> Result<String> {
        let bytes = self.random_bytes(len).await?;
        use base64::Engine;
        let engine = base64::engine::general_purpose::URL_SAFE_NO_PAD;
        Ok(engine.encode(&bytes))
    }

    fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
        if a.len() != b.len() {
            return false;
        }
        let mut result: u8 = 0;
        for (x, y) in a.iter().zip(b.iter()) {
            result |= x ^ y;
        }
        result == 0
    }

    fn provider_name(&self) -> &'static str {
        #[cfg(feature = "fips")]
        {
            "ring-fips"
        }
        #[cfg(not(feature = "fips"))]
        {
            "ring"
        }
    }

    fn is_fips_approved(&self) -> bool {
        cfg!(feature = "fips")
    }
}
