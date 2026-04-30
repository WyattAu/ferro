use async_trait::async_trait;

/// Result type for cryptographic operations.
pub type Result<T> = std::result::Result<T, crate::CryptoError>;

/// Abstraction over cryptographic provider implementations.
#[async_trait]
pub trait CryptoProvider: Send + Sync {
    async fn sha256(&self, data: &[u8]) -> Result<Vec<u8>>;
    async fn sha512(&self, data: &[u8]) -> Result<Vec<u8>>;
    async fn hmac_sha256(&self, key: &[u8], data: &[u8]) -> Result<Vec<u8>>;
    async fn random_bytes(&self, len: usize) -> Result<Vec<u8>>;
    async fn hash_password(&self, password: &str) -> Result<String>;
    async fn verify_password(&self, password: &str, hash: &str) -> Result<bool>;
    async fn generate_token(&self, len: usize) -> Result<String>;
    fn constant_time_eq(a: &[u8], b: &[u8]) -> bool;
    fn provider_name(&self) -> &'static str;
    fn is_fips_approved(&self) -> bool;
}
