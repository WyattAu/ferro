use clap::CommandFactory;

pub use crate::config::ServerConfig as Cli;

pub fn generate_completions(shell: clap_complete::Shell) {
    let mut cmd = Cli::command();
    clap_complete::generate(shell, &mut cmd, "ferro-server", &mut std::io::stdout());
}

pub fn load_file_config(config_path: Option<&str>) -> anyhow::Result<crate::config::FileConfigValues> {
    if let Some(path) = config_path {
        crate::config::load_config_file(path)
    } else if std::path::Path::new("ferro.toml").exists() {
        crate::config::load_config_file("ferro.toml")
    } else if std::path::Path::new("/etc/ferro/ferro.toml").exists() {
        crate::config::load_config_file("/etc/ferro/ferro.toml")
    } else {
        Ok(crate::config::FileConfigValues::default())
    }
}

pub fn validate_config(cli: &Cli, file_config: &crate::config::FileConfigValues) -> anyhow::Result<()> {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    if let Some(version) = file_config.schema_version
        && version > 1
    {
        errors.push(format!(
            "Unsupported config file schema_version: {}. Supported versions: 1.",
            version
        ));
    }

    if cli.port == 0 {
        errors.push("Port must be between 1 and 65535.".to_string());
    }

    if cli.storage != "memory"
        && !cli.storage.starts_with("local:")
        && !cli.storage.starts_with("nas:")
        && !cli.storage.starts_with("nas-nfs:")
        && !cli.storage.starts_with("nas-smb:")
        && !cli.storage.starts_with("s3://")
        && !cli.storage.starts_with("gs://")
        && !cli.storage.starts_with("az://")
    {
        errors.push(format!(
            "Invalid storage backend '{}'. Supported: memory, local:/path, nas:/mount, nas-nfs:/mount, nas-smb:/mount, s3://bucket, gs://bucket, az://container",
            cli.storage
        ));
    }

    if cli.data_dir.is_none() {
        warnings.push("No --data-dir set. All data will be lost on restart.".to_string());
    }

    if !cli.cors_allowed_origins.is_empty() && cli.cors_allowed_origins.contains('*') && cli.oidc_issuer.is_some() {
        errors.push("CORS wildcard '*' cannot be used with OIDC authentication enabled.".to_string());
    }

    if cli.oidc_issuer.is_some() && cli.oidc_client_id.is_none() {
        errors.push("--oidc-client-id is required when --oidc-issuer is set.".to_string());
    }

    if !cli.wopi_office_url.is_empty() && cli.wopi_token_secret.is_none() {
        errors.push("--wopi-token-secret is required when --wopi-office-url is set.".to_string());
    }

    if !errors.is_empty() {
        eprintln!("Configuration errors:");
        for e in &errors {
            eprintln!("  ERROR: {}", e);
        }
    }
    if !warnings.is_empty() {
        eprintln!("Configuration warnings:");
        for w in &warnings {
            eprintln!("  WARN:  {}", w);
        }
    }

    if errors.is_empty() {
        println!("Configuration is valid. {} warning(s).", warnings.len());
        std::process::exit(0);
    } else {
        eprintln!(
            "Configuration validation failed: {} error(s), {} warning(s).",
            errors.len(),
            warnings.len()
        );
        std::process::exit(1);
    }
}

pub fn parse_duration(s: &str) -> Option<std::time::Duration> {
    let s = s.trim().to_lowercase();
    if s == "0" || s == "off" || s == "never" {
        return Some(std::time::Duration::ZERO);
    }
    let (num, suffix) = s.split_at(s.len().saturating_sub(1));
    let num: u64 = num.parse().ok()?;
    match suffix {
        "s" => Some(std::time::Duration::from_secs(num)),
        "m" => Some(std::time::Duration::from_secs(num * 60)),
        "h" => Some(std::time::Duration::from_secs(num * 3600)),
        "d" => Some(std::time::Duration::from_secs(num * 86400)),
        _ => None,
    }
}

pub fn print_man_page() {
    let version = env!("CARGO_PKG_VERSION");
    print!(
        r#".TH FERRO-SERVER 1 "June 2026" "Ferro {version}" "User Commands"
.SH NAME
ferro-server \- Ferro Storage Orchestrator server
.SH SYNOPSIS
.B ferro-server
[\fIOPTIONS\fR]
.SH DESCRIPTION
.B ferro-server
starts the Ferro storage orchestrator, providing WebDAV, CalDAV, CardDAV,
and REST API access to configured storage backends.
.SH OPTIONS
.TP
.BI \-\-config " " \fIFILE\fR
Path to TOML configuration file. Auto-detected at ./ferro.toml or /etc/ferro/ferro.toml.
.TP
.BI \-\-host " " \fIADDR\fR
Listen address (default: 0.0.0.0).
.TP
.BI \-p ", " \-\-port " " \fIPORT\fR
Listen port (default: 8080).
.TP
.BI \-\-log-level " " \fILEVEL\fR
Log level: trace, debug, info, warn, error (default: info).
.TP
.BI \-\-log-format " " \fIFORMAT\fR
Log format: text or json (default: text).
.TP
.BI \-\-storage " " \fIBACKEND\fR
Storage backend: memory (default), local:/path, s3://bucket, gs://bucket, az://container.
.TP
.BI \-\-data-dir " " \fIDIR\fR
Directory for persistent SQLite data (metadata, CAS, snapshots, audit).
.TP
.BI \-\-oidc-issuer " " \fIURL\fR
OIDC issuer URL (enables authentication).
.TP
.BI \-\-oidc-client-id " " \fIID\fR
OIDC client ID (required when --oidc-issuer is set).
.TP
.BI \-\-admin-user " " \fIUSER\fR
Admin username for HTTP Basic Auth.
.TP
.BI \-\-admin-password " " \fIPASS\fR
Admin password for HTTP Basic Auth.
.TP
.BI \-\-wasm-enabled
Enable WASM worker runtime.
.TP
.BI \-\-cas-enabled
Enable content-addressable deduplication.
.TP
.BI \-\-static-dir " " \fIDIR\fR
Path to static web assets directory.
.TP
.BI \-\-validate-config
Validate configuration file and exit.
.TP
.BI \-\-generate-completions " " \fISHELL\fR
Generate shell completion script (bash, zsh, fish, powershell) and exit.
.TP
.BI \-\-print-man-page
Print this man page to stdout and exit.
.TP
.BI \-\-check-update
Check for new versions and exit.
.TP
.BI \-h ", " \-\-help
Print help information.
.TP
.BI \-V ", " \-\-version
Print version information.
.SH FILES
.TP
.I /etc/ferro/ferro.toml
System-wide configuration file (auto-detected).
.TP
.I ./ferro.toml
Per-project configuration file (auto-detected).
.SH ENVIRONMENT
.TP
.B FERRO_CONFIG
Path to configuration file (alternative to --config).
.TP
.B FERRO_DATA_DIR
Data directory (alternative to --data-dir).
.TP
.B FERRO_OIDC_ISSUER
OIDC issuer URL (alternative to --oidc-issuer).
.SH EXAMPLES
Start with default settings:
.RS
.B ferro-server
.RE
.PP
Start with persistent storage and authentication:
.RS
.B ferro-server --data-dir /var/lib/ferro --oidc-issuer https://auth.example.com
.RE
.PP
Generate bash completions:
.RS
.B ferro-server --generate-completions bash > /etc/bash_completion.d/ferro-server
.RE
.PP
Install man page:
.RS
.B ferro-server --print-man-page > /usr/share/man/man1/ferro-server.1
.RE
.SH AUTHOR
Ferro Contributors
.SH LICENSE
See the Ferro project repository for license details.
"#,
        version = version
    );
}

pub async fn check_for_updates() {
    let current_version = env!("CARGO_PKG_VERSION");
    println!("Current version: {}", current_version);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent(format!("ferro-server/{}", current_version))
        .build();

    let client = match client {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to create HTTP client: {}", e);
            return;
        }
    };

    let url = "https://api.github.com/repos/WyattAu/ferro/releases/latest";
    let resp = match client.get(url).send().await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to check for updates: {}", e);
            return;
        }
    };

    if !resp.status().is_success() {
        eprintln!("GitHub API returned status: {}", resp.status());
        return;
    }

    let val: serde_json::Value = match resp.json().await {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Failed to parse response: {}", e);
            return;
        }
    };

    let latest_version = val
        .get("tag_name")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .trim_start_matches('v');

    let html_url = val
        .get("html_url")
        .and_then(|v| v.as_str())
        .unwrap_or("https://github.com/WyattAu/ferro/releases/latest");

    if latest_version == current_version {
        println!("You are running the latest version (v{}).", current_version);
    } else {
        println!("Update available: v{} (current: v{})", latest_version, current_version);
        println!("Download: {}", html_url);
    }
}
