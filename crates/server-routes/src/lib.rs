use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use tower::limit::ConcurrencyLimitLayer;
use tower_http::compression::CompressionLayer;

/// Configuration for the middleware stack.
pub struct MiddlewareConfig {
    /// CORS allowed origins (comma-separated, or "*" for all).
    pub cors_allowed_origins: String,
    /// Whether authentication is enabled.
    pub auth_enabled: bool,
    /// Rate limit burst capacity.
    pub rate_limit_burst: u32,
    /// Rate limit refill rate per second.
    pub rate_limit_refill: u32,
    /// Maximum body size in bytes.
    pub max_body_size: u64,
    /// Maximum concurrent in-flight requests.
    pub max_concurrent_requests: usize,
    /// API version string (e.g., "v1").
    pub api_version: String,
}

impl Default for MiddlewareConfig {
    fn default() -> Self {
        Self {
            cors_allowed_origins: "*".to_string(),
            auth_enabled: false,
            rate_limit_burst: 10_000,
            rate_limit_refill: 166,
            max_body_size: 1024 * 1024 * 1024,
            max_concurrent_requests: 128,
            api_version: "v1".to_string(),
        }
    }
}

/// Validates that CORS origins are configured securely.
///
/// Returns `true` if the configuration is secure.
pub fn validate_cors_config(cors_allowed_origins: &str, auth_enabled: bool) -> bool {
    if cors_allowed_origins == "*" {
        tracing::warn!(
            "SECURITY WARNING: CORS is configured to allow all origins ('*'). \
             This is appropriate for development but should be restricted in production."
        );
    }
    if cors_allowed_origins == "*" && auth_enabled {
        tracing::error!(
            "CORS allowed origins is '*' while auth is enabled -- \
             set a specific origin in production to prevent credential theft"
        );
        return false;
    }
    true
}

/// Generates a versioned API path.
pub fn versioned_api_path(api_version: &str) -> String {
    format!("/api/{}", api_version)
}

/// Creates a compression middleware layer using gzip.
pub fn create_compression_layer() -> CompressionLayer {
    CompressionLayer::new()
}

/// Creates a concurrency limit middleware layer.
pub fn create_concurrency_limit_layer(max_concurrent: usize) -> ConcurrencyLimitLayer {
    ConcurrencyLimitLayer::new(max_concurrent)
}

/// Creates a body size limit middleware layer.
pub fn create_body_limit_layer(max_body_size: u64) -> axum::extract::DefaultBodyLimit {
    axum::extract::DefaultBodyLimit::max(max_body_size as usize)
}

// Error types for middleware
pub struct ApiError;

impl ApiError {
    pub const RATE_LIMITED: &'static str = "RATE_LIMITED";
    pub const MAINTENANCE_MODE: &'static str = "MAINTENANCE_MODE";

    pub fn too_many_requests(_code: String, message: String) -> Response {
        (StatusCode::TOO_MANY_REQUESTS, message).into_response()
    }

    pub fn service_unavailable(_code: String, message: String) -> Response {
        (StatusCode::SERVICE_UNAVAILABLE, message).into_response()
    }
}

/// Utility functions for route configuration.
pub mod utils {
    /// Validates that CORS origins are configured securely.
    ///
    /// Returns `true` if the configuration is secure.
    pub fn validate_cors_config(cors_allowed_origins: &str, auth_enabled: bool) -> bool {
        if cors_allowed_origins == "*" && auth_enabled {
            tracing::error!(
                "CORS allowed origins is '*' while auth is enabled -- \
                 set a specific origin in production to prevent credential theft"
            );
            return false;
        }
        true
    }

    /// Generates a versioned API path.
    pub fn versioned_api_path(api_version: &str) -> String {
        format!("/api/{}", api_version)
    }
}
