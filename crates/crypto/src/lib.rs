//! Cryptographic primitives with pluggable provider backends.

#[cfg(feature = "ring")]
pub mod ring_provider;
pub mod traits;

pub use traits::{CryptoProvider, Result};

use thiserror::Error;

/// Errors that can occur during cryptographic operations.
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
            0x2c, 0xf2, 0x4d, 0xba, 0x5f, 0xb0, 0xa3, 0x0e, 0x26, 0xe8, 0x3b, 0x2a, 0xc5, 0xb9, 0xe2, 0x9e, 0x1b, 0x16,
            0x1e, 0x5c, 0x1f, 0xa7, 0x42, 0x5e, 0x73, 0x04, 0x33, 0x62, 0x93, 0x8b, 0x98, 0x24,
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
        assert!(ring_provider::RingProvider::constant_time_eq(b"same", b"same"));
        assert!(!ring_provider::RingProvider::constant_time_eq(b"same", b"different"));
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

    #[tokio::test]
    async fn test_sha256_empty() {
        let p = provider();
        let hash = p.sha256(b"").await.unwrap();
        let expected_hex = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        let expected: Vec<u8> = (0..64)
            .step_by(2)
            .map(|i| u8::from_str_radix(&expected_hex[i..i + 2], 16).unwrap())
            .collect();
        assert_eq!(hash, expected);
    }

    #[tokio::test]
    async fn test_sha256_large_input() {
        let p = provider();
        let data = vec![0u8; 1024 * 1024];
        let hash = p.sha256(&data).await.unwrap();
        assert_eq!(hash.len(), 32);
        let hash2 = p.sha256(&data).await.unwrap();
        assert_eq!(hash, hash2);
    }

    #[tokio::test]
    async fn test_hmac_sha256_known_vector() {
        let p = provider();
        let key = b"Jefe";
        let data = b"what do ya want for nothing?";
        let mac = p.hmac_sha256(key, data).await.unwrap();
        let expected_hex = "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843";
        let expected: Vec<u8> = (0..64)
            .step_by(2)
            .map(|i| u8::from_str_radix(&expected_hex[i..i + 2], 16).unwrap())
            .collect();
        assert_eq!(mac, expected);
    }

    #[tokio::test]
    async fn test_hmac_sha256_empty() {
        let p = provider();
        let mac = p.hmac_sha256(b"", b"").await.unwrap();
        assert_eq!(mac.len(), 32);
    }

    #[tokio::test]
    async fn test_random_bytes_different_lengths() {
        let p = provider();
        let a = p.random_bytes(1).await.unwrap();
        let b = p.random_bytes(128).await.unwrap();
        let c = p.random_bytes(1024).await.unwrap();
        assert_eq!(a.len(), 1);
        assert_eq!(b.len(), 128);
        assert_eq!(c.len(), 1024);
    }

    #[tokio::test]
    async fn test_constant_time_eq_empty() {
        assert!(ring_provider::RingProvider::constant_time_eq(b"", b""));
        assert!(!ring_provider::RingProvider::constant_time_eq(b"", b"a"));
    }

    #[tokio::test]
    async fn test_constant_time_eq_different_lengths() {
        assert!(!ring_provider::RingProvider::constant_time_eq(b"ab", b"a"));
        assert!(!ring_provider::RingProvider::constant_time_eq(b"a", b"ab"));
    }

    #[tokio::test]
    async fn test_password_hash_different_hashes() {
        let p = provider();
        let h1 = p.hash_password("password").await.unwrap();
        let h2 = p.hash_password("password").await.unwrap();
        assert_ne!(h1, h2);
        assert!(p.verify_password("password", &h1).await.unwrap());
        assert!(p.verify_password("password", &h2).await.unwrap());
    }

    #[tokio::test]
    async fn test_generate_token_no_padding() {
        let p = provider();
        let token = p.generate_token(64).await.unwrap();
        assert!(!token.contains('='));
        assert!(!token.contains('+'));
        assert!(!token.contains('/'));
        assert_eq!(token.len(), 86);
    }

    #[tokio::test]
    async fn test_sha512_empty() {
        let p = provider();
        let hash = p.sha512(b"").await.unwrap();
        assert_eq!(hash.len(), 64);
        let expected_hex = "cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e";
        let expected: Vec<u8> = (0..128)
            .step_by(2)
            .map(|i| u8::from_str_radix(&expected_hex[i..i + 2], 16).unwrap())
            .collect();
        assert_eq!(hash, expected);
    }
}
