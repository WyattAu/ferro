use std::time::Duration;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seconds(pub u64);

impl Seconds {
    pub fn to_duration(&self) -> Duration {
        Duration::from_secs(self.0)
    }
}

impl<'de> Deserialize<'de> for Seconds {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let secs: u64 = u64::deserialize(deserializer)?;
        Ok(Self(secs))
    }
}

impl Serialize for Seconds {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct FerroConfig {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub storage: StorageConfig,
    #[serde(default)]
    pub auth: AuthConfig,
    #[serde(default)]
    pub security: SecurityConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
    #[serde(default)]
    pub network: NetworkConfig,
    #[serde(default)]
    pub advanced: AdvancedConfig,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_workers")]
    pub workers: usize,
    #[serde(default = "default_max_request_size")]
    pub max_request_size: usize,
    #[serde(default = "default_true")]
    pub websocket_enabled: bool,
    #[serde(default = "default_websocket_max_connections")]
    pub websocket_max_connections: usize,
    #[serde(default = "default_graceful_shutdown_timeout")]
    pub graceful_shutdown_timeout: Seconds,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StorageConfig {
    #[serde(default = "default_storage_backend")]
    pub backend: String,
    #[serde(default = "default_local_root")]
    pub local_root: String,
    #[serde(default)]
    pub s3_bucket: Option<String>,
    #[serde(default)]
    pub s3_region: Option<String>,
    #[serde(default)]
    pub s3_endpoint: Option<String>,
    #[serde(default = "default_temp_dir")]
    pub temp_dir: String,
    #[serde(default = "default_max_file_size")]
    pub max_file_size: u64,
    #[serde(default = "default_chunk_size")]
    pub chunk_size: usize,
    #[serde(default = "default_true")]
    pub versioning_enabled: bool,
    #[serde(default = "default_max_versions")]
    pub max_versions_per_file: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AuthConfig {
    #[serde(default)]
    pub jwt_secret: String,
    #[serde(default = "default_jwt_expiry")]
    pub jwt_expiry_hours: u64,
    #[serde(default = "default_refresh_days")]
    pub refresh_token_days: u64,
    #[serde(default = "default_max_login_attempts")]
    pub max_login_attempts: u32,
    #[serde(default = "default_lockout_duration")]
    pub lockout_duration_minutes: u64,
    #[serde(default)]
    pub totp_enabled: bool,
    #[serde(default)]
    pub saml_enabled: bool,
    #[serde(default)]
    pub oidc_enabled: bool,
    #[serde(default = "default_true")]
    pub api_keys_enabled: bool,
    #[serde(default = "default_max_api_keys")]
    pub max_api_keys_per_user: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SecurityConfig {
    #[serde(default = "default_cors_origins")]
    pub cors_allowed_origins: Vec<String>,
    #[serde(default = "default_cors_methods")]
    pub cors_allowed_methods: Vec<String>,
    #[serde(default = "default_rate_limit_rpm")]
    pub rate_limit_requests_per_minute: u32,
    #[serde(default = "default_rate_limit_burst")]
    pub rate_limit_burst: u32,
    #[serde(default)]
    pub max_upload_rate_mb_per_sec: u64,
    #[serde(default = "default_true")]
    pub content_type_validation: bool,
    #[serde(default = "default_true")]
    pub audit_log_enabled: bool,
    #[serde(default = "default_true")]
    pub ransomware_detection_enabled: bool,
    #[serde(default = "default_ransomware_threshold")]
    pub ransomware_threshold_ops_per_minute: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    #[serde(default = "default_log_format")]
    pub format: String,
    #[serde(default)]
    pub file_output: Option<String>,
    #[serde(default = "default_max_log_size")]
    pub max_log_size_mb: u64,
    #[serde(default = "default_log_rotation")]
    pub log_rotation_count: usize,
    #[serde(default)]
    pub exclude_crates: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NetworkConfig {
    #[serde(default)]
    pub public_url: Option<String>,
    #[serde(default)]
    pub trusted_proxies: Vec<String>,
    #[serde(default = "default_ws_heartbeat")]
    pub websocket_heartbeat_interval_secs: u64,
    #[serde(default = "default_ws_message_size")]
    pub max_websocket_message_size: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AdvancedConfig {
    #[serde(default)]
    pub e2ee_enabled: bool,
    #[serde(default)]
    pub plugin_directory: Option<String>,
    #[serde(default)]
    pub telemetry_enabled: bool,
    #[serde(default = "default_startup_timeout")]
    pub startup_timeout_secs: u64,
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    8080
}

fn default_workers() -> usize {
    num_cpus::get()
}

fn default_max_request_size() -> usize {
    100 * 1024 * 1024
}

const fn default_websocket_max_connections() -> usize {
    1000
}

const fn default_graceful_shutdown_timeout() -> Seconds {
    Seconds(30)
}

fn default_storage_backend() -> String {
    "local".to_string()
}

fn default_local_root() -> String {
    "./data".to_string()
}

fn default_temp_dir() -> String {
    "/tmp/ferro".to_string()
}

const fn default_max_file_size() -> u64 {
    10 * 1024 * 1024 * 1024
}

const fn default_chunk_size() -> usize {
    8 * 1024 * 1024
}

const fn default_max_versions() -> usize {
    10
}

const fn default_jwt_expiry() -> u64 {
    24
}

const fn default_refresh_days() -> u64 {
    7
}

const fn default_max_login_attempts() -> u32 {
    5
}

const fn default_lockout_duration() -> u64 {
    15
}

const fn default_max_api_keys() -> usize {
    25
}

fn default_cors_origins() -> Vec<String> {
    vec!["*".to_string()]
}

fn default_cors_methods() -> Vec<String> {
    vec![
        "GET".to_string(),
        "PUT".to_string(),
        "DELETE".to_string(),
        "POST".to_string(),
        "OPTIONS".to_string(),
    ]
}

const fn default_rate_limit_rpm() -> u32 {
    60
}

const fn default_rate_limit_burst() -> u32 {
    10
}

const fn default_ransomware_threshold() -> u32 {
    100
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_log_format() -> String {
    "json".to_string()
}

const fn default_max_log_size() -> u64 {
    100
}

const fn default_log_rotation() -> usize {
    5
}

const fn default_ws_heartbeat() -> u64 {
    30
}

const fn default_ws_message_size() -> usize {
    1024 * 1024
}

const fn default_startup_timeout() -> u64 {
    60
}

const fn default_true() -> bool {
    true
}


impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            workers: default_workers(),
            max_request_size: default_max_request_size(),
            websocket_enabled: true,
            websocket_max_connections: default_websocket_max_connections(),
            graceful_shutdown_timeout: default_graceful_shutdown_timeout(),
        }
    }
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            backend: default_storage_backend(),
            local_root: default_local_root(),
            s3_bucket: None,
            s3_region: None,
            s3_endpoint: None,
            temp_dir: default_temp_dir(),
            max_file_size: default_max_file_size(),
            chunk_size: default_chunk_size(),
            versioning_enabled: true,
            max_versions_per_file: default_max_versions(),
        }
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            jwt_secret: String::new(),
            jwt_expiry_hours: default_jwt_expiry(),
            refresh_token_days: default_refresh_days(),
            max_login_attempts: default_max_login_attempts(),
            lockout_duration_minutes: default_lockout_duration(),
            totp_enabled: false,
            saml_enabled: false,
            oidc_enabled: false,
            api_keys_enabled: true,
            max_api_keys_per_user: default_max_api_keys(),
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            cors_allowed_origins: default_cors_origins(),
            cors_allowed_methods: default_cors_methods(),
            rate_limit_requests_per_minute: default_rate_limit_rpm(),
            rate_limit_burst: default_rate_limit_burst(),
            max_upload_rate_mb_per_sec: 0,
            content_type_validation: true,
            audit_log_enabled: true,
            ransomware_detection_enabled: true,
            ransomware_threshold_ops_per_minute: default_ransomware_threshold(),
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            format: default_log_format(),
            file_output: None,
            max_log_size_mb: default_max_log_size(),
            log_rotation_count: default_log_rotation(),
            exclude_crates: Vec::new(),
        }
    }
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            public_url: None,
            trusted_proxies: Vec::new(),
            websocket_heartbeat_interval_secs: default_ws_heartbeat(),
            max_websocket_message_size: default_ws_message_size(),
        }
    }
}

impl Default for AdvancedConfig {
    fn default() -> Self {
        Self {
            e2ee_enabled: false,
            plugin_directory: None,
            telemetry_enabled: false,
            startup_timeout_secs: default_startup_timeout(),
        }
    }
}
