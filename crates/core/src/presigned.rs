use async_trait::async_trait;
use ferro_common::error::{FerroError, Result};
use url::Url;

// ── Trait ──────────────────────────────────────────────────────────────────

/// Trait for generating presigned upload/download URLs.
#[async_trait]
pub trait PresignedUrlGenerator: Send + Sync {
    async fn generate_put_url(&self, path: &str, expires_in_secs: u32) -> Result<Url>;
    async fn generate_get_url(&self, path: &str, expires_in_secs: u32) -> Result<Url>;
}

// ── Server-based generator ─────────────────────────────────────────────────

/// Presigned URL generator that uses a server base URL.
/// Generates direct server URLs for PUT/GET operations.
/// For cloud backends (S3, GCS, Azure), presigned URLs can be generated
/// by the cloud-specific `ObjectStore` implementations at a later date.
#[derive(Debug)]
pub struct ServerPresignedUrlGenerator {
    base_url: Url,
}

impl ServerPresignedUrlGenerator {
    /// Create a new server-based presigned URL generator.
    pub fn new(base_url: &str) -> Result<Self> {
        let base = Url::parse(base_url).map_err(|e| FerroError::Internal(format!("Invalid base URL: {e}")))?;
        Ok(Self { base_url: base })
    }
}

#[async_trait]
impl PresignedUrlGenerator for ServerPresignedUrlGenerator {
    async fn generate_put_url(&self, path: &str, _expires_in_secs: u32) -> Result<Url> {
        let clean_path = path.trim_start_matches('/');
        let full = format!("{}/{}", self.base_url.as_str().trim_end_matches('/'), clean_path);
        Url::parse(&full).map_err(|e| FerroError::Internal(format!("Invalid URL: {e}")))
    }

    async fn generate_get_url(&self, path: &str, _expires_in_secs: u32) -> Result<Url> {
        let clean_path = path.trim_start_matches('/');
        let full = format!("{}/{}", self.base_url.as_str().trim_end_matches('/'), clean_path);
        Url::parse(&full).map_err(|e| FerroError::Internal(format!("Invalid URL: {e}")))
    }
}

// ── No-op generator (testing) ──────────────────────────────────────────────

/// No-op presigned URL generator (returns localhost URLs, for testing).
#[derive(Debug)]
pub struct NoOpPresignedUrlGenerator;

#[async_trait]
impl PresignedUrlGenerator for NoOpPresignedUrlGenerator {
    async fn generate_put_url(&self, path: &str, _expires_in_secs: u32) -> Result<Url> {
        Url::parse(&format!("http://localhost:8080/storage{path}")).map_err(|e| FerroError::Internal(e.to_string()))
    }

    async fn generate_get_url(&self, path: &str, _expires_in_secs: u32) -> Result<Url> {
        Url::parse(&format!("http://localhost:8080/storage{path}")).map_err(|e| FerroError::Internal(e.to_string()))
    }
}

// ── Cloud generators ───────────────────────────────────────────────────────

/// S3-backed presigned URL generator.
#[derive(Debug)]
#[cfg(feature = "s3")]
pub struct S3PresignedUrlGenerator {
    store: std::sync::Arc<object_store::aws::AmazonS3>,
}

#[cfg(feature = "s3")]
impl S3PresignedUrlGenerator {
    /// Create a new S3 presigned URL generator.
    pub fn new(store: std::sync::Arc<object_store::aws::AmazonS3>) -> Self {
        Self { store }
    }
}

#[cfg(feature = "s3")]
#[async_trait]
impl PresignedUrlGenerator for S3PresignedUrlGenerator {
    async fn generate_put_url(&self, path: &str, expires_in_secs: u32) -> Result<Url> {
        use object_store::signer::Signer;
        let obj_path = object_store::path::Path::from(path.trim_start_matches('/'));
        self.store
            .signed_url(
                reqwest::Method::PUT,
                &obj_path,
                std::time::Duration::from_secs(expires_in_secs as u64),
            )
            .await
            .map_err(|e| FerroError::Internal(format!("S3 signing failed: {}", e)))
    }

    async fn generate_get_url(&self, path: &str, expires_in_secs: u32) -> Result<Url> {
        use object_store::signer::Signer;
        let obj_path = object_store::path::Path::from(path.trim_start_matches('/'));
        self.store
            .signed_url(
                reqwest::Method::GET,
                &obj_path,
                std::time::Duration::from_secs(expires_in_secs as u64),
            )
            .await
            .map_err(|e| FerroError::Internal(format!("S3 signing failed: {}", e)))
    }
}

/// GCS-backed presigned URL generator.
#[derive(Debug)]
#[cfg(feature = "gcs")]
pub struct GcsPresignedUrlGenerator {
    store: std::sync::Arc<object_store::gcp::GoogleCloudStorage>,
}

#[cfg(feature = "gcs")]
impl GcsPresignedUrlGenerator {
    /// Create a new GCS presigned URL generator.
    pub fn new(store: std::sync::Arc<object_store::gcp::GoogleCloudStorage>) -> Self {
        Self { store }
    }
}

#[cfg(feature = "gcs")]
#[async_trait]
impl PresignedUrlGenerator for GcsPresignedUrlGenerator {
    async fn generate_put_url(&self, path: &str, expires_in_secs: u32) -> Result<Url> {
        use object_store::signer::Signer;
        let obj_path = object_store::path::Path::from(path.trim_start_matches('/'));
        self.store
            .signed_url(
                reqwest::Method::PUT,
                &obj_path,
                std::time::Duration::from_secs(expires_in_secs as u64),
            )
            .await
            .map_err(|e| FerroError::Internal(format!("GCS signing failed: {}", e)))
    }

    async fn generate_get_url(&self, path: &str, expires_in_secs: u32) -> Result<Url> {
        use object_store::signer::Signer;
        let obj_path = object_store::path::Path::from(path.trim_start_matches('/'));
        self.store
            .signed_url(
                reqwest::Method::GET,
                &obj_path,
                std::time::Duration::from_secs(expires_in_secs as u64),
            )
            .await
            .map_err(|e| FerroError::Internal(format!("GCS signing failed: {}", e)))
    }
}

/// Azure Blob Storage-backed presigned URL generator.
#[derive(Debug)]
#[cfg(feature = "azure")]
pub struct AzurePresignedUrlGenerator {
    store: std::sync::Arc<object_store::azure::MicrosoftAzure>,
}

#[cfg(feature = "azure")]
impl AzurePresignedUrlGenerator {
    /// Create a new Azure presigned URL generator.
    pub fn new(store: std::sync::Arc<object_store::azure::MicrosoftAzure>) -> Self {
        Self { store }
    }
}

#[cfg(feature = "azure")]
#[async_trait]
impl PresignedUrlGenerator for AzurePresignedUrlGenerator {
    async fn generate_put_url(&self, path: &str, expires_in_secs: u32) -> Result<Url> {
        use object_store::signer::Signer;
        let obj_path = object_store::path::Path::from(path.trim_start_matches('/'));
        self.store
            .signed_url(
                reqwest::Method::PUT,
                &obj_path,
                std::time::Duration::from_secs(expires_in_secs as u64),
            )
            .await
            .map_err(|e| FerroError::Internal(format!("Azure signing failed: {}", e)))
    }

    async fn generate_get_url(&self, path: &str, expires_in_secs: u32) -> Result<Url> {
        use object_store::signer::Signer;
        let obj_path = object_store::path::Path::from(path.trim_start_matches('/'));
        self.store
            .signed_url(
                reqwest::Method::GET,
                &obj_path,
                std::time::Duration::from_secs(expires_in_secs as u64),
            )
            .await
            .map_err(|e| FerroError::Internal(format!("Azure signing failed: {}", e)))
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_noop_put_url() {
        let generator = NoOpPresignedUrlGenerator;
        let url = generator.generate_put_url("/test.txt", 3600).await.unwrap();
        assert_eq!(url.as_str(), "http://localhost:8080/storage/test.txt");
    }

    #[tokio::test]
    async fn test_noop_get_url() {
        let generator = NoOpPresignedUrlGenerator;
        let url = generator.generate_get_url("/test.txt", 3600).await.unwrap();
        assert_eq!(url.as_str(), "http://localhost:8080/storage/test.txt");
    }

    #[tokio::test]
    async fn test_server_presigned_url() {
        let generator = ServerPresignedUrlGenerator::new("http://example.com/files").unwrap();
        let url = generator.generate_get_url("/docs/report.pdf", 3600).await.unwrap();
        assert_eq!(url.as_str(), "http://example.com/files/docs/report.pdf");
    }

    #[tokio::test]
    async fn test_server_presigned_url_trailing_slash() {
        let generator = ServerPresignedUrlGenerator::new("http://example.com/files/").unwrap();
        let url = generator.generate_get_url("/docs/report.pdf", 3600).await.unwrap();
        assert_eq!(url.as_str(), "http://example.com/files/docs/report.pdf");
    }

    #[tokio::test]
    async fn test_server_presigned_url_nested() {
        let generator = ServerPresignedUrlGenerator::new("http://example.com/").unwrap();
        let url = generator.generate_put_url("/a/b/c/file.txt", 7200).await.unwrap();
        assert_eq!(url.as_str(), "http://example.com/a/b/c/file.txt");
    }

    #[cfg(feature = "s3")]
    #[test]
    fn test_s3_generator_new() {
        let builder = object_store::aws::AmazonS3Builder::new()
            .with_bucket_name("test-bucket")
            .with_region("us-east-1")
            .with_access_key_id("fake")
            .with_secret_access_key("fake");
        let store = builder.build().unwrap();
        let _generator = S3PresignedUrlGenerator::new(std::sync::Arc::new(store));
    }

    #[cfg(feature = "gcs")]
    #[test]
    #[ignore = "GCS requires valid service account credentials to construct builder"]
    fn test_gcs_generator_new() {
        let builder = object_store::gcp::GoogleCloudStorageBuilder::new()
            .with_bucket_name("test-bucket")
            .with_service_account_key("fake");
        let store = builder.build().unwrap();
        let _generator = GcsPresignedUrlGenerator::new(std::sync::Arc::new(store));
    }

    #[cfg(feature = "azure")]
    #[test]
    fn test_azure_generator_new() {
        let builder = object_store::azure::MicrosoftAzureBuilder::new()
            .with_container_name("test-container")
            .with_account("fake")
            .with_access_key("fake");
        let store = builder.build().unwrap();
        let _generator = AzurePresignedUrlGenerator::new(std::sync::Arc::new(store));
    }
}
