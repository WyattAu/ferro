use crate::config::FerroConfig;
use crate::error::{ConfigError, ConfigWarning, WarningSeverity};

impl FerroConfig {
    pub fn validate(&self) -> Result<Vec<ConfigWarning>, ConfigError> {
        let mut warnings = Vec::new();

        validate_server(&self.server, &mut warnings)?;
        validate_storage(&self.storage, &mut warnings)?;
        validate_auth(&self.auth, &mut warnings)?;
        validate_security(&self.security, &mut warnings)?;
        validate_advanced(&self.advanced, &mut warnings);

        Ok(warnings)
    }
}

fn validate_server(
    server: &crate::config::ServerConfig,
    warnings: &mut Vec<ConfigWarning>,
) -> Result<(), ConfigError> {
    if server.port == 0 {
        return Err(ConfigError::InvalidValue {
            field: "server.port".to_string(),
            message: "port must be between 1 and 65535".to_string(),
        });
    }
    let max_request_bytes: u64 = 10 * 1024 * 1024 * 1024;
    if server.max_request_size as u64 > max_request_bytes {
        return Err(ConfigError::InvalidValue {
            field: "server.max_request_size".to_string(),
            message: "max_request_size must be less than 10GB".to_string(),
        });
    }
    if server.workers == 0 {
        warnings.push(ConfigWarning {
            field: "server.workers".to_string(),
            message: "zero workers configured".to_string(),
            severity: WarningSeverity::Warning,
        });
    }
    Ok(())
}

fn validate_storage(
    storage: &crate::config::StorageConfig,
    _warnings: &mut Vec<ConfigWarning>,
) -> Result<(), ConfigError> {
    if storage.chunk_size < 4096 {
        return Err(ConfigError::InvalidValue {
            field: "storage.chunk_size".to_string(),
            message: "chunk_size must be at least 4096 bytes (4KB)".to_string(),
        });
    }
    if !storage.chunk_size.is_power_of_two() {
        return Err(ConfigError::InvalidValue {
            field: "storage.chunk_size".to_string(),
            message: "chunk_size must be a power of 2".to_string(),
        });
    }
    Ok(())
}

fn validate_auth(
    auth: &crate::config::AuthConfig,
    _warnings: &mut Vec<ConfigWarning>,
) -> Result<(), ConfigError> {
    if auth.jwt_secret.is_empty() {
        return Err(ConfigError::MissingField(
            "auth.jwt_secret".to_string(),
        ));
    }
    if auth.jwt_secret.len() < 16 {
        return Err(ConfigError::InvalidValue {
            field: "auth.jwt_secret".to_string(),
            message: "jwt_secret must be at least 16 characters".to_string(),
        });
    }
    if auth.max_login_attempts == 0 {
        return Err(ConfigError::InvalidValue {
            field: "auth.max_login_attempts".to_string(),
            message: "max_login_attempts must be greater than 0".to_string(),
        });
    }
    Ok(())
}

fn validate_security(
    security: &crate::config::SecurityConfig,
    warnings: &mut Vec<ConfigWarning>,
) -> Result<(), ConfigError> {
    if security.cors_allowed_origins == ["*"] {
        warnings.push(ConfigWarning {
            field: "security.cors_allowed_origins".to_string(),
            message: "wildcard CORS origin is not recommended for production".to_string(),
            severity: WarningSeverity::Warning,
        });
    }
    Ok(())
}

fn validate_advanced(advanced: &crate::config::AdvancedConfig, warnings: &mut Vec<ConfigWarning>) {
    if advanced.e2ee_enabled && advanced.plugin_directory.is_none() {
        warnings.push(ConfigWarning {
            field: "advanced.plugin_directory".to_string(),
            message: "e2ee is enabled but no plugin_directory is set".to_string(),
            severity: WarningSeverity::Info,
        });
    }
}
