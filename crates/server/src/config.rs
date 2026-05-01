use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use crate::AppState;

/// GET /api/config — return server configuration and capabilities.
pub async fn get_server_config(State(state): State<AppState>) -> Response {
    let body = serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "auth_enabled": state.auth_enabled(),
        "search_enabled": state.search.is_some(),
        "wasm_enabled": state.wasm_runtime.is_some(),
        "wasm_workers_enabled": state.wasm_runtime.is_some(),
        "cedar_enabled": state.cedar.is_some(),
        "metadata_persistent": state.metadata_store.is_some(),
        "cas_enabled": state.cas_store.is_some(),
        "storage": "configured",
        "external_url": state.external_url,
        "wopi_configured": !state.wopi_office_url.is_empty(),
    });
    (StatusCode::OK, axum::Json(body)).into_response()
}

use clap::Parser;
use serde::Deserialize;

/// Configuration values loaded from a TOML file.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct FileConfigValues {
    pub host: Option<String>,
    pub port: Option<u16>,
    pub log_level: Option<String>,
    pub log_format: Option<String>,
    pub storage: Option<String>,
    pub data_dir: Option<String>,
    pub static_dir: Option<String>,
    pub max_body_size: Option<String>,
    pub admin_user: Option<String>,
    pub admin_password: Option<String>,
    pub external_url: Option<String>,
    pub wopi_token_secret: Option<String>,
    pub wopi_office_url: Option<String>,
    pub federation_secret: Option<String>,
    pub oidc_issuer: Option<String>,
    pub oidc_client_id: Option<String>,
    pub oidc_audience: Option<String>,
    pub oidc_jwks_uri: Option<String>,
    pub cedar_policy_file: Option<String>,
    pub search_index_path: Option<String>,
    pub metadata_db: Option<String>,
    pub cas_enabled: Option<bool>,
    pub wasm_enabled: Option<bool>,
    pub storage_quota: Option<String>,
    pub trash_ttl: Option<String>,
    pub graceful_shutdown_timeout: Option<u64>,
    pub cors_allowed_origins: Option<String>,
}

/// Configuration loaded from a TOML file with include support.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct FileConfig {
    /// Include other TOML files (merged in order, later files override earlier)
    #[serde(default)]
    pub include: Vec<String>,
    /// The actual configuration values
    #[serde(flatten)]
    pub values: FileConfigValues,
}

#[derive(Parser, Debug, Clone)]
#[command(name = "ferro-server", about = "Ferro Storage Orchestrator", version)]
pub struct ServerConfig {
    /// Path to configuration file (TOML format)
    #[arg(long, env = "FERRO_CONFIG")]
    pub config: Option<String>,

    #[arg(long, default_value = "0.0.0.0")]
    pub host: String,

    #[arg(short, long, default_value_t = 8080)]
    pub port: u16,

    #[arg(long, default_value = "info")]
    pub log_level: String,

    /// Log format: "text" (default) or "json"
    #[arg(long, env = "FERRO_LOG_FORMAT", default_value = "text")]
    pub log_format: String,

    /// Storage backend: "memory" (default) or "local:/path/to/dir"
    #[arg(long, default_value = "memory")]
    pub storage: String,

    /// OIDC issuer URL (enables authentication)
    #[arg(long, env = "FERRO_OIDC_ISSUER")]
    pub oidc_issuer: Option<String>,

    /// OIDC audience
    #[arg(long, env = "FERRO_OIDC_AUDIENCE", default_value = "ferro")]
    pub oidc_audience: String,

    /// OIDC client ID
    #[arg(long, env = "FERRO_OIDC_CLIENT_ID")]
    pub oidc_client_id: Option<String>,

    /// JWKS URI (overrides auto-discovery)
    #[arg(long, env = "FERRO_OIDC_JWKS_URI")]
    pub oidc_jwks_uri: Option<String>,

    /// Path to Cedar policy file
    #[arg(long, env = "FERRO_CEDAR_POLICY_FILE")]
    pub cedar_policy_file: Option<String>,

    /// Search index directory (defaults to {data-dir}/search-index, or /tmp/ferro-search if no data-dir)
    #[arg(long)]
    pub search_index_path: Option<String>,

    /// PostgreSQL metadata database URL (enables persistent metadata)
    #[arg(long, env = "FERRO_METADATA_DB")]
    pub metadata_db: Option<String>,

    /// Enable content-addressable deduplication
    #[arg(long, default_value_t = false)]
    pub cas_enabled: bool,

    /// Directory for persistent SQLite data (metadata, CAS, snapshots, audit).
    /// When set, all in-memory stores are replaced with SQLite-backed persistence.
    /// Example: `--data-dir /var/lib/ferro`
    #[arg(long, env = "FERRO_DATA_DIR")]
    pub data_dir: Option<String>,

    /// Maximum request body size in bytes (default: 1 GB).
    #[arg(long, env = "FERRO_MAX_BODY_SIZE", default_value = "1073741824")]
    pub max_body_size: u64,

    /// Enable WASM worker runtime.
    #[arg(long, env = "FERRO_WASM_ENABLED")]
    pub wasm_enabled: bool,

    /// Path to static web assets directory (serves index.html, JS, WASM)
    #[arg(long, env = "FERRO_STATIC_DIR")]
    pub static_dir: Option<String>,

    /// Secret used for signing WOPI access tokens (HMAC-SHA256).
    /// If not set, a default value is used (not safe for production).
    #[arg(
        long,
        env = "FERRO_WOPI_TOKEN_SECRET",
        default_value = "ferro-wopi-token-secret-change-me"
    )]
    pub wopi_token_secret: String,

    /// External base URL the server is accessible from (used for OIDC redirects).
    /// Default: http://localhost:8080
    #[arg(
        long,
        env = "FERRO_EXTERNAL_URL",
        default_value = "http://localhost:8080"
    )]
    pub external_url: String,

    /// WOPI office server URL (e.g., <https://collabora.example.com>).
    /// When set, the WOPI discovery endpoint returns this as urlsrc.
    /// When empty (default), WOPI integration is effectively disabled.
    #[arg(long, env = "FERRO_WOPI_OFFICE_URL", default_value = "")]
    pub wopi_office_url: String,

    /// Secret used for verifying HTTP Signatures on the federation inbox (HMAC-SHA256).
    /// When empty (default), federation is disabled and the inbox returns 503.
    #[arg(long, env = "FERRO_FEDERATION_SECRET", default_value = "")]
    pub federation_secret: String,

    /// Admin username for simple authentication (enables Basic Auth)
    #[arg(long, env = "FERRO_ADMIN_USER")]
    pub admin_user: Option<String>,

    /// Admin password for simple authentication (plain text, use env var in production)
    #[arg(long, env = "FERRO_ADMIN_PASSWORD")]
    pub admin_password: Option<String>,

    /// Storage quota (e.g., "10GB", "500MB", "1TB"). None means unlimited.
    #[arg(long, env = "FERRO_STORAGE_QUOTA")]
    pub storage_quota: Option<String>,

    /// Trash auto-purge TTL (e.g., "30d", "7d", "24h", "0" to disable). Default: "30d".
    #[arg(long, env = "FERRO_TRASH_TTL", default_value = "30d")]
    pub trash_ttl: String,

    /// Graceful shutdown timeout in seconds.
    #[arg(long, env = "FERRO_GRACEFUL_SHUTDOWN_TIMEOUT", default_value = "30")]
    pub graceful_shutdown_timeout: u64,

    /// Comma-separated list of allowed CORS origins (default "*" allows all).
    #[arg(long, env = "FERRO_CORS_ALLOWED_ORIGINS", default_value = "*")]
    pub cors_allowed_origins: String,

    /// API version prefix (default: "v1"). Routes are mounted at /api/{version}.
    #[arg(long, env = "FERRO_API_VERSION", default_value = "v1")]
    pub api_version: String,

    /// Comma-separated list of allowed CORS origins (alternative flag, same as --cors-allowed-origins).
    #[arg(long, env = "FERRO_CORS_ORIGINS", default_value = "*")]
    pub cors_origins: String,

    /// PostgreSQL database URL for distributed state (shares, favorites, preferences).
    /// Only used when the `pg` feature is enabled at compile time.
    #[cfg(feature = "pg")]
    #[arg(long, env = "FERRO_DATABASE_URL")]
    pub database_url: Option<String>,

    /// Redis URL for distributed locking and rate limiting.
    /// Only used when the `redis` feature is enabled at compile time.
    #[cfg(feature = "redis")]
    #[arg(long, env = "FERRO_REDIS_URL")]
    pub redis_url: Option<String>,

    /// Maximum number of file versions to retain per file (default: 10, 0 = disabled)
    #[arg(long, env = "FERRO_MAX_FILE_VERSIONS", default_value = "10")]
    pub max_file_versions: u64,

    /// Maximum thumbnail dimension in pixels (64-1024, default: 256)
    #[arg(long, env = "FERRO_THUMBNAIL_SIZE", default_value = "256")]
    pub thumbnail_size: u32,

    /// Enable multi-user mode with per-user home directories
    #[arg(long, env = "FERRO_MULTI_USER")]
    pub multi_user: bool,

    #[cfg(feature = "ldap")]
    /// LDAP server URL (enables LDAP authentication)
    #[arg(long, env = "FERRO_LDAP_URL")]
    pub ldap_url: Option<String>,

    #[cfg(feature = "ldap")]
    /// LDAP bind DN for service account
    #[arg(long, env = "FERRO_LDAP_BIND_DN")]
    pub ldap_bind_dn: Option<String>,

    #[cfg(feature = "ldap")]
    /// LDAP service account password
    #[arg(long, env = "FERRO_LDAP_BIND_PASSWORD")]
    pub ldap_bind_password: Option<String>,

    #[cfg(feature = "ldap")]
    /// LDAP user search base DN
    #[arg(long, env = "FERRO_LDAP_USER_SEARCH_BASE", default_value = "")]
    pub ldap_user_search_base: String,
}

/// Load and parse a TOML configuration file, resolving includes recursively.
pub fn load_config_file(path: &str) -> anyhow::Result<FileConfigValues> {
    let mut chain = Vec::new();
    load_config_file_inner(path, &mut chain)
}

fn load_config_file_inner(
    path: &str,
    chain: &mut Vec<std::path::PathBuf>,
) -> anyhow::Result<FileConfigValues> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("Failed to read config file {}: {}", path, e))?;

    let canonical = std::path::Path::new(path)
        .canonicalize()
        .map_err(|e| anyhow::anyhow!("Failed to resolve config file path {}: {}", path, e))?;

    if chain.contains(&canonical) {
        return Err(anyhow::anyhow!(
            "Config file include cycle detected: {}",
            path
        ));
    }

    chain.push(canonical);

    let config: FileConfig = toml::from_str(&content)
        .map_err(|e| anyhow::anyhow!("Failed to parse config file {}: {}", path, e))?;

    let mut merged = config.values;

    for include_path in &config.include {
        let resolved = if std::path::Path::new(include_path).is_absolute() {
            include_path.clone()
        } else {
            let base_dir = std::path::Path::new(path)
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| ".".to_string());
            format!("{}/{}", base_dir, include_path)
        };

        let included = load_config_file_inner(&resolved, chain)?;
        merged = merge_configs(merged, included);
    }

    chain.pop();

    Ok(merged)
}

fn merge_configs(base: FileConfigValues, override_: FileConfigValues) -> FileConfigValues {
    FileConfigValues {
        host: override_.host.or(base.host),
        port: override_.port.or(base.port),
        log_level: override_.log_level.or(base.log_level),
        log_format: override_.log_format.or(base.log_format),
        storage: override_.storage.or(base.storage),
        data_dir: override_.data_dir.or(base.data_dir),
        static_dir: override_.static_dir.or(base.static_dir),
        max_body_size: override_.max_body_size.or(base.max_body_size),
        admin_user: override_.admin_user.or(base.admin_user),
        admin_password: override_.admin_password.or(base.admin_password),
        external_url: override_.external_url.or(base.external_url),
        wopi_token_secret: override_.wopi_token_secret.or(base.wopi_token_secret),
        wopi_office_url: override_.wopi_office_url.or(base.wopi_office_url),
        federation_secret: override_.federation_secret.or(base.federation_secret),
        oidc_issuer: override_.oidc_issuer.or(base.oidc_issuer),
        oidc_client_id: override_.oidc_client_id.or(base.oidc_client_id),
        oidc_audience: override_.oidc_audience.or(base.oidc_audience),
        oidc_jwks_uri: override_.oidc_jwks_uri.or(base.oidc_jwks_uri),
        cedar_policy_file: override_.cedar_policy_file.or(base.cedar_policy_file),
        search_index_path: override_.search_index_path.or(base.search_index_path),
        metadata_db: override_.metadata_db.or(base.metadata_db),
        cas_enabled: override_.cas_enabled.or(base.cas_enabled),
        wasm_enabled: override_.wasm_enabled.or(base.wasm_enabled),
        storage_quota: override_.storage_quota.or(base.storage_quota),
        trash_ttl: override_.trash_ttl.or(base.trash_ttl),
        graceful_shutdown_timeout: override_
            .graceful_shutdown_timeout
            .or(base.graceful_shutdown_timeout),
        cors_allowed_origins: override_.cors_allowed_origins.or(base.cors_allowed_origins),
    }
}

/// Apply file-based configuration, without overriding CLI flags.
pub fn apply_file_config<I, T>(args: I, cli: &mut ServerConfig, file: &FileConfigValues)
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    use clap::CommandFactory;
    let matches = ServerConfig::command()
        .ignore_errors(true)
        .try_get_matches_from(args)
        .ok();
    let was_set = |name: &str| {
        matches
            .as_ref()
            .is_some_and(|m| m.value_source(name) == Some(clap::parser::ValueSource::CommandLine))
    };

    if !was_set("host")
        && let Some(ref host) = file.host
    {
        cli.host = host.clone();
    }
    if !was_set("port")
        && let Some(port) = file.port
    {
        cli.port = port;
    }
    if !was_set("log_level")
        && let Some(ref level) = file.log_level
    {
        cli.log_level = level.clone();
    }
    if !was_set("log_format")
        && let Some(ref format) = file.log_format
    {
        cli.log_format = format.clone();
    }
    if !was_set("storage")
        && let Some(ref storage) = file.storage
    {
        cli.storage = storage.clone();
    }
    if !was_set("data_dir") {
        cli.data_dir = file.data_dir.clone();
    }
    if !was_set("static_dir") {
        cli.static_dir = file.static_dir.clone();
    }
    if !was_set("admin_user") {
        cli.admin_user = file.admin_user.clone();
    }
    if !was_set("admin_password") {
        cli.admin_password = file.admin_password.clone();
    }
    if !was_set("external_url")
        && let Some(ref url) = file.external_url
    {
        cli.external_url = url.clone();
    }
    if !was_set("wopi_token_secret")
        && let Some(ref secret) = file.wopi_token_secret
    {
        cli.wopi_token_secret = secret.clone();
    }
    if !was_set("wopi_office_url")
        && let Some(ref url) = file.wopi_office_url
    {
        cli.wopi_office_url = url.clone();
    }
    if !was_set("federation_secret") {
        cli.federation_secret = file.federation_secret.clone().unwrap_or_default();
    }
    if !was_set("oidc_issuer") {
        cli.oidc_issuer = file.oidc_issuer.clone();
    }
    if !was_set("oidc_client_id") {
        cli.oidc_client_id = file.oidc_client_id.clone();
    }
    if !was_set("oidc_audience")
        && let Some(ref audience) = file.oidc_audience
    {
        cli.oidc_audience = audience.clone();
    }
    if !was_set("oidc_jwks_uri") {
        cli.oidc_jwks_uri = file.oidc_jwks_uri.clone();
    }
    if !was_set("cedar_policy_file") {
        cli.cedar_policy_file = file.cedar_policy_file.clone();
    }
    if !was_set("search_index_path") {
        cli.search_index_path = file.search_index_path.clone();
    }
    if !was_set("metadata_db") {
        cli.metadata_db = file.metadata_db.clone();
    }
    if !was_set("cas_enabled")
        && let Some(enabled) = file.cas_enabled
    {
        cli.cas_enabled = enabled;
    }
    if !was_set("wasm_enabled")
        && let Some(enabled) = file.wasm_enabled
    {
        cli.wasm_enabled = enabled;
    }
    if !was_set("max_body_size")
        && let Some(ref size_str) = file.max_body_size
        && let Ok(bytes) = parse_bytes(size_str)
    {
        cli.max_body_size = bytes;
    }
    if !was_set("storage_quota") {
        cli.storage_quota = file.storage_quota.clone();
    }
    if !was_set("trash_ttl")
        && let Some(ref ttl) = file.trash_ttl
    {
        cli.trash_ttl = ttl.clone();
    }
    if !was_set("graceful_shutdown_timeout")
        && let Some(timeout) = file.graceful_shutdown_timeout
    {
        cli.graceful_shutdown_timeout = timeout;
    }
    if !was_set("cors_allowed_origins")
        && let Some(ref origins) = file.cors_allowed_origins
    {
        cli.cors_allowed_origins = origins.clone();
    }
}

fn parse_bytes(s: &str) -> anyhow::Result<u64> {
    let s = s.trim();
    let (num_str, multiplier) = if let Some(s) = s.strip_suffix("GB") {
        (s.trim_end(), 1_073_741_824u64)
    } else if let Some(s) = s.strip_suffix("MB") {
        (s.trim_end(), 1_048_576)
    } else if let Some(s) = s.strip_suffix("KB") {
        (s.trim_end(), 1024)
    } else if let Some(s) = s.strip_suffix("B") {
        (s.trim_end(), 1)
    } else {
        (s, 1)
    };
    let num: u64 = num_str
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid byte size: {}", s))?;
    Ok(num * multiplier)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AppState;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    async fn body_json(response: axum::response::Response) -> serde_json::Value {
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn test_config_auth_disabled_without_oidc() {
        let app = crate::build_router(AppState::in_memory());

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/config")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert_eq!(json["auth_enabled"], false);
    }

    #[tokio::test]
    async fn test_config_has_required_fields() {
        let app = crate::build_router(AppState::in_memory());

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/config")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let json = body_json(response).await;
        assert!(json.get("version").is_some());
        assert!(json.get("auth_enabled").is_some());
        assert!(json.get("search_enabled").is_some());
        assert!(json.get("wasm_workers_enabled").is_some());
        assert!(json.get("cedar_enabled").is_some());
        assert!(json.get("metadata_persistent").is_some());
        assert!(json.get("cas_enabled").is_some());
        assert!(json.get("storage").is_some());
    }

    #[tokio::test]
    async fn test_config_metadata_persistent_false() {
        let app = crate::build_router(AppState::in_memory());

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/config")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let json = body_json(response).await;
        assert_eq!(json["metadata_persistent"], false);
    }

    #[tokio::test]
    async fn test_config_cas_enabled_false() {
        let app = crate::build_router(AppState::in_memory());

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/config")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let json = body_json(response).await;
        assert_eq!(json["cas_enabled"], false);
    }

    #[test]
    fn test_load_config_file_valid_toml() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("ferro.toml");
        std::fs::write(
            &config_path,
            r#"
            host = "127.0.0.1"
            port = 9090
            log_level = "debug"
            storage = "local:/data/files"
            wasm_enabled = true
        "#,
        )
        .unwrap();

        let config = load_config_file(config_path.to_str().unwrap()).unwrap();
        assert_eq!(config.host.as_deref(), Some("127.0.0.1"));
        assert_eq!(config.port, Some(9090));
        assert_eq!(config.log_level.as_deref(), Some("debug"));
        assert_eq!(config.storage.as_deref(), Some("local:/data/files"));
        assert_eq!(config.wasm_enabled, Some(true));
        assert!(config.admin_user.is_none());
        assert!(config.data_dir.is_none());
    }

    #[test]
    fn test_load_config_file_nonexistent() {
        let result = load_config_file("/nonexistent/path/ferro.toml");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Failed to read config file")
        );
    }

    #[test]
    fn test_load_config_file_invalid_toml() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("bad.toml");
        std::fs::write(&config_path, "this is not [ valid toml").unwrap();

        let result = load_config_file(config_path.to_str().unwrap());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Failed to parse config file")
        );
    }

    #[test]
    fn test_apply_file_config_overrides_defaults() {
        let file = FileConfigValues {
            host: Some("192.168.1.1".into()),
            port: Some(3000),
            log_level: Some("debug".into()),
            storage: Some("local:/tmp/files".into()),
            data_dir: Some("/var/lib/ferro".into()),
            admin_user: Some("admin".into()),
            admin_password: Some("secret".into()),
            external_url: Some("https://ferro.example.com".into()),
            wasm_enabled: Some(true),
            cas_enabled: Some(true),
            max_body_size: Some("2GB".into()),
            ..Default::default()
        };

        let args = ["ferro-server"];
        let mut cli = ServerConfig::parse_from(args.iter().copied());
        apply_file_config(args.iter().copied(), &mut cli, &file);

        assert_eq!(cli.host, "192.168.1.1");
        assert_eq!(cli.port, 3000);
        assert_eq!(cli.log_level, "debug");
        assert_eq!(cli.storage, "local:/tmp/files");
        assert_eq!(cli.data_dir.as_deref(), Some("/var/lib/ferro"));
        assert_eq!(cli.admin_user.as_deref(), Some("admin"));
        assert_eq!(cli.admin_password.as_deref(), Some("secret"));
        assert_eq!(cli.external_url, "https://ferro.example.com");
        assert!(cli.wasm_enabled);
        assert!(cli.cas_enabled);
        assert_eq!(cli.max_body_size, 2_147_483_648);
    }

    #[test]
    fn test_apply_file_config_does_not_override_cli_flags() {
        let file = FileConfigValues {
            host: Some("192.168.1.1".into()),
            port: Some(3000),
            log_level: Some("debug".into()),
            storage: Some("local:/tmp/files".into()),
            wasm_enabled: Some(true),
            ..Default::default()
        };

        let args = [
            "ferro-server",
            "--host",
            "10.0.0.1",
            "--port",
            "4000",
            "--log-level",
            "trace",
            "--storage",
            "memory",
            "--wasm-enabled",
        ];
        let mut cli = ServerConfig::parse_from(args.iter().copied());
        apply_file_config(args.iter().copied(), &mut cli, &file);

        assert_eq!(cli.host, "10.0.0.1");
        assert_eq!(cli.port, 4000);
        assert_eq!(cli.log_level, "trace");
        assert_eq!(cli.storage, "memory");
        assert!(cli.wasm_enabled);
    }

    #[test]
    fn test_parse_bytes() {
        assert_eq!(parse_bytes("1073741824").unwrap(), 1073741824);
        assert_eq!(parse_bytes("1GB").unwrap(), 1073741824);
        assert_eq!(parse_bytes("512MB").unwrap(), 536870912);
        assert_eq!(parse_bytes("1024KB").unwrap(), 1048576);
        assert_eq!(parse_bytes("1024B").unwrap(), 1024);
        assert!(parse_bytes("invalid").is_err());
    }

    #[test]
    fn test_merge_configs_override() {
        let base = FileConfigValues {
            host: Some("0.0.0.0".into()),
            port: Some(8080),
            log_level: Some("info".into()),
            admin_user: Some("base_admin".into()),
            ..Default::default()
        };
        let override_ = FileConfigValues {
            host: Some("192.168.1.1".into()),
            port: Some(3000),
            admin_password: Some("secret".into()),
            ..Default::default()
        };
        let merged = merge_configs(base, override_);
        assert_eq!(merged.host.as_deref(), Some("192.168.1.1"));
        assert_eq!(merged.port, Some(3000));
        assert_eq!(merged.log_level.as_deref(), Some("info"));
        assert_eq!(merged.admin_user.as_deref(), Some("base_admin"));
        assert_eq!(merged.admin_password.as_deref(), Some("secret"));
    }

    #[test]
    fn test_merge_configs_base_only() {
        let base = FileConfigValues {
            host: Some("10.0.0.1".into()),
            port: Some(9090),
            wasm_enabled: Some(true),
            ..Default::default()
        };
        let override_ = FileConfigValues::default();
        let merged = merge_configs(base, override_);
        assert_eq!(merged.host.as_deref(), Some("10.0.0.1"));
        assert_eq!(merged.port, Some(9090));
        assert_eq!(merged.wasm_enabled, Some(true));
        assert!(merged.admin_user.is_none());
    }

    #[test]
    fn test_load_config_file_not_found() {
        let result = load_config_file("/nonexistent/path/ferro.toml");
        assert!(result.is_err());
    }

    #[test]
    fn test_load_config_file_with_include() {
        let dir = tempfile::tempdir().unwrap();
        let base_path = dir.path().join("base.toml");
        let override_path = dir.path().join("override.toml");

        std::fs::write(
            &base_path,
            r#"
            host = "0.0.0.0"
            port = 8080
            log_level = "info"
            admin_user = "base_user"
            include = ["override.toml"]
            "#,
        )
        .unwrap();

        std::fs::write(
            &override_path,
            r#"
            host = "192.168.1.1"
            port = 3000
            admin_password = "secret"
            "#,
        )
        .unwrap();

        let config = load_config_file(base_path.to_str().unwrap()).unwrap();
        assert_eq!(config.host.as_deref(), Some("192.168.1.1"));
        assert_eq!(config.port, Some(3000));
        assert_eq!(config.log_level.as_deref(), Some("info"));
        assert_eq!(config.admin_user.as_deref(), Some("base_user"));
        assert_eq!(config.admin_password.as_deref(), Some("secret"));
    }

    #[test]
    fn test_load_config_file_cycle_detection() {
        let dir = tempfile::tempdir().unwrap();
        let a_path = dir.path().join("a.toml");
        let b_path = dir.path().join("b.toml");

        std::fs::write(
            &a_path,
            r#"
            host = "0.0.0.0"
            include = ["b.toml"]
            "#,
        )
        .unwrap();

        std::fs::write(
            &b_path,
            r#"
            port = 3000
            include = ["a.toml"]
            "#,
        )
        .unwrap();

        let result = load_config_file(a_path.to_str().unwrap());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cycle"));
    }

    #[test]
    fn test_load_config_file_nested_include() {
        let dir = tempfile::tempdir().unwrap();
        let base_path = dir.path().join("base.toml");
        let mid_path = dir.path().join("mid.toml");
        let override_path = dir.path().join("override.toml");

        std::fs::write(
            &base_path,
            r#"
            host = "0.0.0.0"
            log_level = "debug"
            include = ["mid.toml"]
            "#,
        )
        .unwrap();

        std::fs::write(
            &mid_path,
            r#"
            port = 3000
            include = ["override.toml"]
            "#,
        )
        .unwrap();

        std::fs::write(
            &override_path,
            r#"
            host = "10.0.0.1"
            admin_user = "admin"
            "#,
        )
        .unwrap();

        let config = load_config_file(base_path.to_str().unwrap()).unwrap();
        assert_eq!(config.host.as_deref(), Some("10.0.0.1"));
        assert_eq!(config.port, Some(3000));
        assert_eq!(config.log_level.as_deref(), Some("debug"));
        assert_eq!(config.admin_user.as_deref(), Some("admin"));
    }
}
