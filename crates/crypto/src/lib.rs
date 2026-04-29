pub mod ring_provider;
pub mod traits;

pub use traits::{CryptoProvider, Result};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("Internal crypto error: {0}")]
    Internal(String),
    #[error("Random generation failed: {0}")]
    RandomGeneration(String),
    #[error("Password hashing failed: {0}")]
    PasswordHash(String),
    #[error("HMAC error: {0}")]
    Hmac(String),
    #[error("Invalid key: {0}")]
    InvalidKey(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn provider() -> ring_provider::RingProvider {
        ring_provider::RingProvider::new()
    }

    #[tokio::test]
    async fn test_sha256() {
        let p = provider();
        let hash = p.sha256(b"hello").await.unwrap();
        assert_eq!(hash.len(), 32);
        let expected: Vec<u8> = vec![
            0x2c, 0xf2, 0x4d, 0xba, 0x5f, 0xb0, 0xa3, 0x0e, 0x26, 0xe8, 0x3b, 0x2a, 0xc5, 0xb9,
            0xe2, 0x9e, 0x1b, 0x16, 0x1e, 0x5c, 0x1f, 0xa7, 0x42, 0x5e, 0x73, 0x04, 0x33, 0x62,
            0x93, 0x8b, 0x98, 0x24,
        ];
        assert_eq!(hash, expected);
    }

    #[tokio::test]
    async fn test_sha512() {
        let p = provider();
        let hash = p.sha512(b"hello").await.unwrap();
        assert_eq!(hash.len(), 64);
    }

    #[tokio::test]
    async fn test_hmac_sha256() {
        let p = provider();
        let mac = p.hmac_sha256(b"key", b"data").await.unwrap();
        assert_eq!(mac.len(), 32);
    }

    #[tokio::test]
    async fn test_random_bytes() {
        let p = provider();
        let a = p.random_bytes(32).await.unwrap();
        let b = p.random_bytes(32).await.unwrap();
        assert_ne!(a, b);
        assert_eq!(a.len(), 32);
    }

    #[tokio::test]
    async fn test_constant_time_eq() {
        assert!(ring_provider::RingProvider::constant_time_eq(
            b"same", b"same"
        ));
        assert!(!ring_provider::RingProvider::constant_time_eq(
            b"same",
            b"different"
        ));
    }

    #[tokio::test]
    async fn test_generate_token() {
        let p = provider();
        let t1 = p.generate_token(32).await.unwrap();
        let t2 = p.generate_token(32).await.unwrap();
        assert_ne!(t1, t2);
        assert!(!t1.contains('='));
    }

    #[tokio::test]
    async fn test_password_hash_verify() {
        let p = provider();
        let hash = p.hash_password("password123").await.unwrap();
        assert!(p.verify_password("password123", &hash).await.unwrap());
        assert!(!p.verify_password("wrong", &hash).await.unwrap());
    }

    #[test]
    fn test_provider_name() {
        let p = provider();
        assert!(!p.provider_name().is_empty());
    }
}
