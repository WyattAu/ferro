use async_trait::async_trait;

/// Result type for cryptographic operations.
pub type Result<T> = std::result::Result<T, crate::CryptoError>;

/// Abstraction over cryptographic provider implementations.
#[async_trait]
pub trait CryptoProvider: Send + Sync {
    /// Compute the SHA-256 digest of the input.
    async fn sha256(&self, data: &[u8]) -> Result<Vec<u8>>;
    /// Compute the SHA-512 digest of the input.
    async fn sha512(&self, data: &[u8]) -> Result<Vec<u8>>;
    /// Compute an HMAC-SHA256 message authentication code.
    async fn hmac_sha256(&self, key: &[u8], data: &[u8]) -> Result<Vec<u8>>;
    /// Generate cryptographically secure random bytes.
    async fn random_bytes(&self, len: usize) -> Result<Vec<u8>>;
    /// Hash a password using a secure algorithm (bcrypt).
    async fn hash_password(&self, password: &str) -> Result<String>;
    /// Verify a password against a previously computed hash.
    async fn verify_password(&self, password: &str, hash: &str) -> Result<bool>;
    /// Generate a URL-safe token of the given byte length.
    async fn generate_token(&self, len: usize) -> Result<String>;
    /// Compare two byte slices in constant time to prevent timing attacks.
    fn constant_time_eq(a: &[u8], b: &[u8]) -> bool;
    /// Return the name of the crypto provider (e.g. "ring", "ring-fips").
    fn provider_name(&self) -> &'static str;
    /// Return whether the provider uses FIPS-approved algorithms.
    fn is_fips_approved(&self) -> bool;
}
