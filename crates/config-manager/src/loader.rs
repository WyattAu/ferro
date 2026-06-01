use std::env;
use std::fs;
use std::str::FromStr;

use tracing::warn;

use crate::config::FerroConfig;
use crate::error::ConfigError;

pub struct ConfigLoader;

impl ConfigLoader {
    pub fn load_from_file(path: &str) -> Result<FerroConfig, ConfigError> {
        let content = fs::read_to_string(path)?;
        Self::load_from_str(&content)
    }

    pub fn load_from_str(toml: &str) -> Result<FerroConfig, ConfigError> {
        let config: FerroConfig = toml::from_str(toml)?;
        Ok(config)
    }

    pub fn load_from_env() -> Result<FerroConfig, ConfigError> {
        let mut config = FerroConfig::default();
        Self::apply_env_overrides(&mut config);
        Ok(config)
    }

    pub fn load_with_defaults() -> FerroConfig {
        FerroConfig::default()
    }

    pub fn load_merged(file_path: Option<&str>) -> Result<FerroConfig, ConfigError> {
        let mut config = FerroConfig::default();

        if let Some(path) = file_path {
            match Self::load_from_file(path) {
                Ok(file_config) => config = file_config,
                Err(ConfigError::FileRead(_)) => {
                    warn!(path, "config file not found, using defaults");
                }
                Err(e) => return Err(e),
            }
        }

        Self::apply_env_overrides(&mut config);
        Ok(config)
    }

    fn apply_env_overrides(config: &mut FerroConfig) {
        if let Ok(v) = env::var("FERRO_SERVER_HOST") {
            config.server.host = v;
        }
        if let Ok(v) = env::var("FERRO_SERVER_PORT") {
            if let Ok(port) = u16::from_str(&v) {
                config.server.port = port;
            } else {
                warn!("FERRO_SERVER_PORT is not a valid u16, ignoring");
            }
        }
        if let Ok(v) = env::var("FERRO_SERVER_WORKERS") && let Ok(w) = usize::from_str(&v) {
            config.server.workers = w;
        }
        if let Ok(v) = env::var("FERRO_SERVER_MAX_REQUEST_SIZE") && let Ok(s) = usize::from_str(&v) {
            config.server.max_request_size = s;
        }
        if let Ok(v) = env::var("FERRO_SERVER_WEBSOCKET_ENABLED") && let Ok(b) = bool::from_str(&v) {
            config.server.websocket_enabled = b;
        }
        if let Ok(v) = env::var("FERRO_SERVER_WEBSOCKET_MAX_CONNECTIONS") && let Ok(n) = usize::from_str(&v) {
            config.server.websocket_max_connections = n;
        }
        if let Ok(v) = env::var("FERRO_SERVER_GRACEFUL_SHUTDOWN_TIMEOUT_SECS")
            && let Ok(s) = u64::from_str(&v)
        {
            config.server.graceful_shutdown_timeout.0 = s;
        }

        if let Ok(v) = env::var("FERRO_STORAGE_BACKEND") {
            config.storage.backend = v;
        }
        if let Ok(v) = env::var("FERRO_STORAGE_LOCAL_ROOT") {
            config.storage.local_root = v;
        }
        if let Ok(v) = env::var("FERRO_STORAGE_S3_BUCKET") {
            config.storage.s3_bucket = Some(v);
        }
        if let Ok(v) = env::var("FERRO_STORAGE_S3_REGION") {
            config.storage.s3_region = Some(v);
        }
        if let Ok(v) = env::var("FERRO_STORAGE_S3_ENDPOINT") {
            config.storage.s3_endpoint = Some(v);
        }
        if let Ok(v) = env::var("FERRO_STORAGE_TEMP_DIR") {
            config.storage.temp_dir = v;
        }
        if let Ok(v) = env::var("FERRO_STORAGE_MAX_FILE_SIZE") && let Ok(s) = u64::from_str(&v) {
            config.storage.max_file_size = s;
        }
        if let Ok(v) = env::var("FERRO_STORAGE_CHUNK_SIZE") && let Ok(s) = usize::from_str(&v) {
            config.storage.chunk_size = s;
        }
        if let Ok(v) = env::var("FERRO_STORAGE_VERSIONING_ENABLED") && let Ok(b) = bool::from_str(&v) {
            config.storage.versioning_enabled = b;
        }

        if let Ok(v) = env::var("FERRO_AUTH_JWT_SECRET") {
            config.auth.jwt_secret = v;
        }
        if let Ok(v) = env::var("FERRO_AUTH_JWT_EXPIRY_HOURS") && let Ok(h) = u64::from_str(&v) {
            config.auth.jwt_expiry_hours = h;
        }
        if let Ok(v) = env::var("FERRO_AUTH_REFRESH_TOKEN_DAYS") && let Ok(d) = u64::from_str(&v) {
            config.auth.refresh_token_days = d;
        }
        if let Ok(v) = env::var("FERRO_AUTH_MAX_LOGIN_ATTEMPTS") && let Ok(a) = u32::from_str(&v) {
            config.auth.max_login_attempts = a;
        }
        if let Ok(v) = env::var("FERRO_AUTH_TOTP_ENABLED") && let Ok(b) = bool::from_str(&v) {
            config.auth.totp_enabled = b;
        }
        if let Ok(v) = env::var("FERRO_AUTH_API_KEYS_ENABLED") && let Ok(b) = bool::from_str(&v) {
            config.auth.api_keys_enabled = b;
        }

        if let Ok(v) = env::var("FERRO_SECURITY_CORS_ALLOWED_ORIGINS") {
            config.security.cors_allowed_origins = v.split(',').map(String::from).collect();
        }
        if let Ok(v) = env::var("FERRO_SECURITY_CORS_ALLOWED_METHODS") {
            config.security.cors_allowed_methods = v.split(',').map(String::from).collect();
        }
        if let Ok(v) = env::var("FERRO_SECURITY_RATE_LIMIT_RPM") && let Ok(n) = u32::from_str(&v) {
            config.security.rate_limit_requests_per_minute = n;
        }
        if let Ok(v) = env::var("FERRO_SECURITY_RATE_LIMIT_BURST") && let Ok(n) = u32::from_str(&v) {
            config.security.rate_limit_burst = n;
        }
        if let Ok(v) = env::var("FERRO_SECURITY_RANSOMWARE_DETECTION_ENABLED")
            && let Ok(b) = bool::from_str(&v)
        {
            config.security.ransomware_detection_enabled = b;
        }

        if let Ok(v) = env::var("FERRO_LOG_LEVEL") {
            config.logging.level = v;
        }
        if let Ok(v) = env::var("FERRO_LOG_FORMAT") {
            config.logging.format = v;
        }
        if let Ok(v) = env::var("FERRO_LOG_FILE_OUTPUT") {
            config.logging.file_output = Some(v);
        }

        if let Ok(v) = env::var("FERRO_NETWORK_PUBLIC_URL") {
            config.network.public_url = Some(v);
        }
        if let Ok(v) = env::var("FERRO_NETWORK_TRUSTED_PROXIES") {
            config.network.trusted_proxies = v.split(',').map(String::from).collect();
        }
        if let Ok(v) = env::var("FERRO_NETWORK_WS_HEARTBEAT_SECS") && let Ok(s) = u64::from_str(&v) {
            config.network.websocket_heartbeat_interval_secs = s;
        }

        if let Ok(v) = env::var("FERRO_ADVANCED_E2EE_ENABLED") && let Ok(b) = bool::from_str(&v) {
            config.advanced.e2ee_enabled = b;
        }
        if let Ok(v) = env::var("FERRO_ADVANCED_PLUGIN_DIRECTORY") {
            config.advanced.plugin_directory = Some(v);
        }
        if let Ok(v) = env::var("FERRO_ADVANCED_TELEMETRY_ENABLED") && let Ok(b) = bool::from_str(&v) {
            config.advanced.telemetry_enabled = b;
        }
    }
}
