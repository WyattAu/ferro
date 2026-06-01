use std::env;
use std::sync::Mutex;

use super::*;

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn with_env_lock() -> std::sync::MutexGuard<'static, ()> {
    ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

fn cleanup_env() {
    let vars = [
        "FERRO_SERVER_HOST",
        "FERRO_SERVER_PORT",
        "FERRO_SERVER_WORKERS",
        "FERRO_SERVER_MAX_REQUEST_SIZE",
        "FERRO_SERVER_WEBSOCKET_ENABLED",
        "FERRO_SERVER_WEBSOCKET_MAX_CONNECTIONS",
        "FERRO_SERVER_GRACEFUL_SHUTDOWN_TIMEOUT_SECS",
        "FERRO_STORAGE_BACKEND",
        "FERRO_STORAGE_LOCAL_ROOT",
        "FERRO_STORAGE_S3_BUCKET",
        "FERRO_STORAGE_S3_REGION",
        "FERRO_STORAGE_S3_ENDPOINT",
        "FERRO_STORAGE_TEMP_DIR",
        "FERRO_STORAGE_MAX_FILE_SIZE",
        "FERRO_STORAGE_CHUNK_SIZE",
        "FERRO_STORAGE_VERSIONING_ENABLED",
        "FERRO_AUTH_JWT_SECRET",
        "FERRO_AUTH_JWT_EXPIRY_HOURS",
        "FERRO_AUTH_REFRESH_TOKEN_DAYS",
        "FERRO_AUTH_MAX_LOGIN_ATTEMPTS",
        "FERRO_AUTH_LOCKOUT_DURATION_MINUTES",
        "FERRO_AUTH_TOTP_ENABLED",
        "FERRO_AUTH_SAML_ENABLED",
        "FERRO_AUTH_OIDC_ENABLED",
        "FERRO_AUTH_API_KEYS_ENABLED",
        "FERRO_SECURITY_CORS_ALLOWED_ORIGINS",
        "FERRO_SECURITY_CORS_ALLOWED_METHODS",
        "FERRO_SECURITY_RATE_LIMIT_RPM",
        "FERRO_SECURITY_RATE_LIMIT_BURST",
        "FERRO_SECURITY_RANSOMWARE_DETECTION_ENABLED",
        "FERRO_LOG_LEVEL",
        "FERRO_LOG_FORMAT",
        "FERRO_LOG_FILE_OUTPUT",
        "FERRO_NETWORK_PUBLIC_URL",
        "FERRO_NETWORK_TRUSTED_PROXIES",
        "FERRO_NETWORK_WS_HEARTBEAT_SECS",
        "FERRO_ADVANCED_E2EE_ENABLED",
        "FERRO_ADVANCED_PLUGIN_DIRECTORY",
        "FERRO_ADVANCED_TELEMETRY_ENABLED",
    ];
    for var in vars {
        unsafe { env::remove_var(var) };
    }
}

#[test]
fn test_default_config() {
    let config = FerroConfig::default();
    assert_eq!(config.server.host, "0.0.0.0");
    assert_eq!(config.server.port, 8080);
    assert!(config.server.websocket_enabled);
    assert_eq!(config.server.graceful_shutdown_timeout.0, 30);
    assert_eq!(config.storage.backend, "local");
    assert_eq!(config.storage.local_root, "./data");
    assert!(config.storage.s3_bucket.is_none());
    assert_eq!(config.storage.temp_dir, "/tmp/ferro");
    assert!(config.storage.versioning_enabled);
    assert_eq!(config.storage.max_versions_per_file, 10);
    assert!(!config.auth.totp_enabled);
    assert!(!config.auth.saml_enabled);
    assert!(config.auth.api_keys_enabled);
    assert!(config.security.content_type_validation);
    assert!(config.security.audit_log_enabled);
    assert!(config.security.ransomware_detection_enabled);
    assert_eq!(config.security.rate_limit_requests_per_minute, 60);
    assert_eq!(config.logging.level, "info");
    assert_eq!(config.logging.format, "json");
    assert!(config.logging.file_output.is_none());
    assert!(config.logging.exclude_crates.is_empty());
    assert!(config.network.public_url.is_none());
    assert!(config.network.trusted_proxies.is_empty());
    assert_eq!(config.network.websocket_heartbeat_interval_secs, 30);
    assert!(!config.advanced.e2ee_enabled);
    assert!(config.advanced.plugin_directory.is_none());
    assert!(!config.advanced.telemetry_enabled);
    assert_eq!(config.advanced.startup_timeout_secs, 60);
}

#[test]
fn test_load_with_defaults() {
    let config = ConfigLoader::load_with_defaults();
    assert_eq!(config.server.port, 8080);
    assert_eq!(config.storage.backend, "local");
    assert_eq!(config.logging.level, "info");
}

#[test]
fn test_load_from_toml_string() {
    let toml = r#"
[server]
port = 9090
host = "127.0.0.1"

[storage]
backend = "s3"
s3_bucket = "my-bucket"
s3_region = "us-east-1"

[auth]
jwt_secret = "my-super-secret-key-1234"

[logging]
level = "debug"

[network]
public_url = "https://example.com"
"#;
    let config = ConfigLoader::load_from_str(toml).unwrap();
    assert_eq!(config.server.port, 9090);
    assert_eq!(config.server.host, "127.0.0.1");
    assert_eq!(config.storage.backend, "s3");
    assert_eq!(config.storage.s3_bucket.as_deref(), Some("my-bucket"));
    assert_eq!(config.storage.s3_region.as_deref(), Some("us-east-1"));
    assert_eq!(config.auth.jwt_secret, "my-super-secret-key-1234");
    assert_eq!(config.logging.level, "debug");
    assert_eq!(
        config.network.public_url.as_deref(),
        Some("https://example.com")
    );
}

#[test]
fn test_load_from_empty_toml() {
    let config = ConfigLoader::load_from_str("").unwrap();
    assert_eq!(config.server.port, 8080);
    assert_eq!(config.storage.backend, "local");
    assert_eq!(config.logging.level, "info");
}

#[test]
fn test_load_from_malformed_toml() {
    let result = ConfigLoader::load_from_str("this is not [ valid toml");
    assert!(result.is_err());
}

#[test]
fn test_env_override_port() {
    let _guard = with_env_lock();
    cleanup_env();
    unsafe { env::set_var("FERRO_SERVER_PORT", "3000") };
    let config = ConfigLoader::load_from_env().unwrap();
    assert_eq!(config.server.port, 3000);
}

#[test]
fn test_env_override_string_fields() {
    let _guard = with_env_lock();
    cleanup_env();
    unsafe { env::set_var("FERRO_STORAGE_BACKEND", "s3") };
    unsafe { env::set_var("FERRO_LOG_LEVEL", "trace") };
    unsafe { env::set_var("FERRO_NETWORK_PUBLIC_URL", "https://app.ferro.io") };
    let config = ConfigLoader::load_from_env().unwrap();
    assert_eq!(config.storage.backend, "s3");
    assert_eq!(config.logging.level, "trace");
    assert_eq!(
        config.network.public_url.as_deref(),
        Some("https://app.ferro.io")
    );
}

#[test]
fn test_env_override_comma_separated_list() {
    let _guard = with_env_lock();
    cleanup_env();
    unsafe {
        env::set_var(
            "FERRO_SECURITY_CORS_ALLOWED_ORIGINS",
            "https://a.com,https://b.com",
        )
    };
    let config = ConfigLoader::load_from_env().unwrap();
    assert_eq!(
        config.security.cors_allowed_origins,
        vec!["https://a.com".to_string(), "https://b.com".to_string()]
    );
}

#[test]
fn test_env_override_bool_fields() {
    let _guard = with_env_lock();
    cleanup_env();
    unsafe { env::set_var("FERRO_AUTH_TOTP_ENABLED", "true") };
    unsafe { env::set_var("FERRO_ADVANCED_TELEMETRY_ENABLED", "true") };
    let config = ConfigLoader::load_from_env().unwrap();
    assert!(config.auth.totp_enabled);
    assert!(config.advanced.telemetry_enabled);
}

#[test]
fn test_env_invalid_port_ignored() {
    let _guard = with_env_lock();
    cleanup_env();
    unsafe { env::set_var("FERRO_SERVER_PORT", "not_a_number") };
    let config = ConfigLoader::load_from_env().unwrap();
    assert_eq!(config.server.port, 8080);
}

#[test]
fn test_merged_with_file() {
    let _guard = with_env_lock();
    cleanup_env();
    let dir = std::env::temp_dir().join("ferro-test-cfg-file");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("ferro.toml");
    std::fs::write(&path, "[server]\nport = 4000\n").unwrap();

    let config = ConfigLoader::load_merged(Some(path.to_str().unwrap())).unwrap();
    assert_eq!(config.server.port, 4000);
    assert_eq!(config.storage.backend, "local");

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_merged_missing_file_uses_defaults() {
    let _guard = with_env_lock();
    cleanup_env();
    let config = ConfigLoader::load_merged(Some("/nonexistent/ferro.toml")).unwrap();
    assert_eq!(config.server.port, 8080);
}

#[test]
fn test_merged_none_path_uses_defaults() {
    let _guard = with_env_lock();
    cleanup_env();
    let config = ConfigLoader::load_merged(None).unwrap();
    assert_eq!(config.server.port, 8080);
}

#[test]
fn test_validate_valid_config() {
    let mut config = FerroConfig::default();
    config.auth.jwt_secret = "a-good-secret-that-is-long-enough".to_string();
    let result = config.validate();
    assert!(result.is_ok());
}

#[test]
fn test_validate_empty_jwt_secret() {
    let config = FerroConfig::default();
    let result = config.validate();
    assert!(matches!(result, Err(ConfigError::MissingField(_))));
}

#[test]
fn test_validate_short_jwt_secret() {
    let mut config = FerroConfig::default();
    config.auth.jwt_secret = "tooshort".to_string();
    let result = config.validate();
    assert!(matches!(result, Err(ConfigError::InvalidValue { .. })));
}

#[test]
fn test_validate_port_zero() {
    let mut config = FerroConfig::default();
    config.auth.jwt_secret = "a-good-secret-that-is-long-enough".to_string();
    config.server.port = 0;
    let result = config.validate();
    assert!(matches!(result, Err(ConfigError::InvalidValue { .. })));
}

#[test]
fn test_validate_max_request_size_too_large() {
    let mut config = FerroConfig::default();
    config.auth.jwt_secret = "a-good-secret-that-is-long-enough".to_string();
    config.server.max_request_size = 11 * 1024 * 1024 * 1024;
    let result = config.validate();
    assert!(matches!(result, Err(ConfigError::InvalidValue { .. })));
}

#[test]
fn test_validate_chunk_size_not_power_of_two() {
    let mut config = FerroConfig::default();
    config.auth.jwt_secret = "a-good-secret-that-is-long-enough".to_string();
    config.storage.chunk_size = 100;
    let result = config.validate();
    assert!(matches!(result, Err(ConfigError::InvalidValue { .. })));
}

#[test]
fn test_validate_chunk_size_too_small() {
    let mut config = FerroConfig::default();
    config.auth.jwt_secret = "a-good-secret-that-is-long-enough".to_string();
    config.storage.chunk_size = 2048;
    let result = config.validate();
    assert!(matches!(result, Err(ConfigError::InvalidValue { .. })));
}

#[test]
fn test_validate_max_login_attempts_zero() {
    let mut config = FerroConfig::default();
    config.auth.jwt_secret = "a-good-secret-that-is-long-enough".to_string();
    config.auth.max_login_attempts = 0;
    let result = config.validate();
    assert!(matches!(result, Err(ConfigError::InvalidValue { .. })));
}

#[test]
fn test_validate_wildcard_cors_warning() {
    let mut config = FerroConfig::default();
    config.auth.jwt_secret = "a-good-secret-that-is-long-enough".to_string();
    let warnings = config.validate().unwrap();
    let cors_warning = warnings
        .iter()
        .find(|w| w.field == "security.cors_allowed_origins");
    assert!(cors_warning.is_some());
    assert_eq!(cors_warning.unwrap().severity, WarningSeverity::Warning);
}

#[test]
fn test_validate_e2ee_no_plugin_warning() {
    let mut config = FerroConfig::default();
    config.auth.jwt_secret = "a-good-secret-that-is-long-enough".to_string();
    config.advanced.e2ee_enabled = true;
    config.advanced.plugin_directory = None;
    let warnings = config.validate().unwrap();
    let e2ee_warning = warnings
        .iter()
        .find(|w| w.field == "advanced.plugin_directory");
    assert!(e2ee_warning.is_some());
    assert_eq!(e2ee_warning.unwrap().severity, WarningSeverity::Info);
}

#[test]
fn test_validate_specific_cors_no_warning() {
    let mut config = FerroConfig::default();
    config.auth.jwt_secret = "a-good-secret-that-is-long-enough".to_string();
    config.security.cors_allowed_origins = vec!["https://example.com".to_string()];
    let warnings = config.validate().unwrap();
    let cors_warning = warnings
        .iter()
        .find(|w| w.field == "security.cors_allowed_origins");
    assert!(cors_warning.is_none());
}

#[test]
fn test_merged_file_then_env() {
    let _guard = with_env_lock();
    cleanup_env();
    let dir = std::env::temp_dir().join("ferro-test-cfg-merged");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("ferro.toml");
    std::fs::write(&path, "[server]\nport = 4000\n").unwrap();

    unsafe { env::set_var("FERRO_SERVER_PORT", "5555") };
    let config = ConfigLoader::load_merged(Some(path.to_str().unwrap())).unwrap();
    assert_eq!(config.server.port, 5555);

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_load_file_nonexistent() {
    let result = ConfigLoader::load_from_file("/nonexistent/ferro.toml");
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), ConfigError::FileRead(_)));
}

#[test]
fn test_seconds_to_duration() {
    let secs = crate::config::Seconds(30);
    assert_eq!(secs.to_duration(), std::time::Duration::from_secs(30));
}

#[test]
fn test_seconds_serde_round_trip() {
    let parsed: serde_json::Value = serde_json::json!({"seconds": 45});
    let secs: crate::config::Seconds = serde_json::from_value(parsed["seconds"].clone()).unwrap();
    assert_eq!(secs.0, 45);
}

#[test]
fn test_default_chunk_size_is_power_of_two() {
    let config = FerroConfig::default();
    assert!(config.storage.chunk_size.is_power_of_two());
    assert!(config.storage.chunk_size >= 4096);
}

#[test]
fn test_default_cors_methods() {
    let config = FerroConfig::default();
    assert!(
        config
            .security
            .cors_allowed_methods
            .contains(&"GET".to_string())
    );
    assert!(
        config
            .security
            .cors_allowed_methods
            .contains(&"POST".to_string())
    );
    assert!(
        config
            .security
            .cors_allowed_methods
            .contains(&"PUT".to_string())
    );
    assert!(
        config
            .security
            .cors_allowed_methods
            .contains(&"DELETE".to_string())
    );
    assert!(
        config
            .security
            .cors_allowed_methods
            .contains(&"OPTIONS".to_string())
    );
}
