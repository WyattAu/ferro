#![cfg_attr(all(not(debug_assertions), feature = "tauri"), windows_subsystem = "windows")]

#[cfg(feature = "tauri")]
mod gui;

#[cfg(not(feature = "tauri"))]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use clap::Parser;
    use ferro_desktop::commands::DesktopState;
    use ferro_desktop::config::DesktopConfig;
    use tracing::info;

    #[derive(Parser, Debug)]
    #[command(name = "ferro-desktop", about = "Ferro Desktop Client")]
    struct Cli {
        #[arg(long, default_value = "http://localhost:8080")]
        server_url: String,

        #[arg(short, long)]
        username: Option<String>,

        #[arg(short = 'p', long)]
        password: Option<String>,

        #[arg(long)]
        mount_point: Option<String>,

        #[arg(long, default_value_t = false)]
        auto_mount: bool,

        /// Bearer token (OIDC access token). Falls back to FERRO_AUTH_TOKEN.
        #[arg(long)]
        auth_token: Option<String>,

        /// Local folder to sync (default: ~/Ferro).
        #[arg(long)]
        local_dir: Option<String>,

        /// Interactive browser login (RFC 8252 loopback, Keycloak) — saves
        /// tokens to the secret store and exits.
        #[arg(long, default_value_t = false)]
        login: bool,

        /// OIDC issuer for --login (defaults to the WyattAu Keycloak realm).
        #[arg(long)]
        issuer: Option<String>,

        /// OAuth client id for --login (default: ferro-desktop, public PKCE client).
        #[arg(long)]
        client_id: Option<String>,

        /// Run one sync cycle and exit (requires `sync` feature).
        #[arg(long, default_value_t = false)]
        sync_once: bool,

        /// Run sync periodically every N seconds (requires `sync` feature, 0 = off).
        #[arg(long, default_value_t = 0)]
        sync_interval: u64,

        #[arg(long, default_value = "info")]
        log_level: String,
    }

    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| cli.log_level.clone().into()),
        )
        .init();

    if cli.login {
        let issuer = cli
            .issuer
            .clone()
            .unwrap_or_else(|| ferro_desktop::login::DEFAULT_ISSUER.to_string());
        let client_id = cli
            .client_id
            .clone()
            .unwrap_or_else(|| ferro_desktop::login::DESKTOP_CLIENT_ID.to_string());
        let tokens = ferro_desktop::login::run_login(&issuer, &client_id)
            .await
            .map_err(anyhow::Error::msg)?;
        let saved = DesktopConfig {
            server_url: cli.server_url.clone(),
            auth_token: Some(tokens.access_token.clone()),
            refresh_token: tokens.refresh_token.clone(),
            username: tokens.username.clone().unwrap_or_default(),
            ..ferro_desktop::config::load_from_disk_or_default()
        };
        ferro_desktop::config::save_config_to_disk(&saved).map_err(anyhow::Error::msg)?;
        println!(
            "Signed in as {} — tokens stored in the OS secret store ({})",
            tokens.username.as_deref().unwrap_or("user"),
            ferro_desktop::secret_store::secrets_path().display()
        );
        return Ok(());
    }

    // Stored config (desktop.json + secrets overlay) is the base; CLI/env
    // args win per-field when explicitly provided.
    let mut config = ferro_desktop::config::load_config_from_disk().unwrap_or_default();
    if cli.server_url != "http://localhost:8080" {
        config.server_url = cli.server_url;
    }
    if let Some(u) = cli.username {
        config.username = u;
    }
    if let Some(p) = cli.password {
        config.password = p;
    }
    let env_token = std::env::var("FERRO_AUTH_TOKEN").ok().filter(|s| !s.is_empty());
    let env_password = std::env::var("FERRO_PASSWORD").ok().filter(|s| !s.is_empty());
    if let Some(t) = cli.auth_token.or(env_token) {
        config.auth_token = Some(t);
    }
    if let Some(p) = env_password {
        config.password = p;
    }
    config.mount_point = cli
        .mount_point
        .map(|p| p.into())
        .unwrap_or_else(|| config.mount_point.clone());
    config.auto_mount = cli.auto_mount;

    #[cfg(feature = "sync")]
    if cli.sync_once || cli.sync_interval > 0 {
        let issuer = cli
            .issuer
            .clone()
            .unwrap_or_else(|| ferro_desktop::login::DEFAULT_ISSUER.to_string());
        let client_id2 = cli
            .client_id
            .clone()
            .unwrap_or_else(|| ferro_desktop::login::DESKTOP_CLIENT_ID.to_string());
        return run_headless_sync(&config, cli.local_dir, cli.sync_interval, issuer, client_id2).await;
    }
    #[cfg(not(feature = "sync"))]
    if cli.sync_once || cli.sync_interval > 0 {
        anyhow::bail!("sync flags require building with `--features sync`");
    }

    let state = DesktopState::new(config.clone());

    info!("Ferro Desktop starting");
    info!("Server: {}", config.server_url);
    info!("Mount point: {}", config.mount_point.display());

    match ferro_desktop::rclone::RcloneManager::check_rclone_available() {
        Ok(version) => info!("rclone: {}", version),
        Err(e) => {
            tracing::warn!("{}", e);
            tracing::warn!("Ferro Desktop requires rclone for mount functionality");
            tracing::warn!("Install it from https://rclone.org/install/");
        }
    }

    let status = state.get_mount_status().await;
    info!("Mount status: {}", status.status);

    if cli.auto_mount {
        info!("Auto-mounting...");
        match state.mount_drive().await {
            Ok(_) => info!("Successfully mounted"),
            Err(e) => tracing::error!("Mount failed: {}", e),
        }
    }

    info!("Ferro Desktop ready (press Ctrl+C to exit)");
    tokio::signal::ctrl_c().await?;

    if state.get_mount_status().await.is_mounted {
        info!("Unmounting...");
        let _ = state.unmount_drive().await;
    }

    info!("Ferro Desktop exited");
    Ok(())
}

#[cfg(feature = "tauri")]
fn main() {
    // Disable WebKitGTK DMA-BUF renderer on Linux.
    // Some GPU drivers (especially in VMs or older hardware) fail to create
    // GBM buffers, causing the webview to render a blank window despite
    // DOM content being present. Setting this before GTK init forces
    // software compositing as a fallback.
    #[cfg(target_os = "linux")]
    if std::env::var("WEBKIT_DISABLE_DMABUF_RENDERER").is_err() {
        // SAFETY: Called at startup before any threads are spawned; no concurrent access to env.
        unsafe { std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1") };
    }

    // Force X11 backend on Linux/Wayland when user hasn't overridden.
    // Tauri v2 + native Wayland causes KWin to not map the window surface —
    // the webview renders internally (screenshots work) but never appears on
    // screen. Forcing GDK_BACKEND=x11 makes the window visible under KWin.
    #[cfg(target_os = "linux")]
    if std::env::var("GDK_BACKEND").is_err() {
        // SAFETY: Called at startup before any threads are spawned.
        unsafe { std::env::set_var("GDK_BACKEND", "x11") };
    }

    let args = gui::CliArgs::parse();

    // Initialize tracing before GUI setup.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| match args.debug {
                0 => "info".into(),
                1 => "ferro_desktop=debug".into(),
                _ => "debug".into(),
            }),
        )
        .with_writer(std::io::stderr)
        .init();

    tracing::info!("Ferro Desktop starting (Tauri mode)");
    if let Some(ref url) = args.server_url {
        tracing::info!("CLI server URL: {}", url);
    }
    if let Some(ref token) = args.auth_token {
        tracing::info!("CLI auth token: provided ({} chars)", token.len());
    }
    if args.debug > 0 {
        tracing::info!("Debug mode: level {}", args.debug);
    }

    if let Err(e) = gui::run(args) {
        tracing::error!("Fatal error: {e}");
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

#[cfg(all(feature = "sync", not(feature = "tauri")))]
async fn run_headless_sync(
    config: &ferro_desktop::config::DesktopConfig,
    local_dir: Option<String>,
    interval_secs: u64,
    issuer: String,
    client_id2: String,
) -> anyhow::Result<()> {
    use ferro_desktop::sync::engine::{SyncConfig, SyncEngine};
    use tracing::info;

    if config.auth_token.is_none() && (config.username.is_empty() || config.password.is_empty()) {
        anyhow::bail!("sync requires --auth-token (or FERRO_AUTH_TOKEN) or --username/--password");
    }

    let local_path = match local_dir {
        Some(d) => std::path::PathBuf::from(d),
        None => dirs::home_dir()
            .map(|h| h.join("Ferro"))
            .unwrap_or_else(|| std::path::PathBuf::from(".")),
    };

    let engine = SyncEngine::new(SyncConfig {
        local_path,
        remote_path: "/".to_string(),
        server_url: config.server_url.clone(),
        username: config.username.clone(),
        password: config.password.clone(),
        bearer_token: config.auth_token.clone(),
        refresh_token: config.refresh_token.clone(),
        oidc_issuer: Some(issuer.to_string()),
        oidc_client_id: Some(client_id2.to_string()),
        ..Default::default()
    })?;

    let run_once = || async {
        match engine.sync().await {
            Ok(summary) => info!(
                uploaded = summary.uploaded,
                downloaded = summary.downloaded,
                conflicts = summary.conflicts,
                errors = summary.errors,
                bytes = summary.bytes_transferred,
                "sync cycle complete"
            ),
            Err(e) => tracing::error!("sync cycle failed: {e}"),
        }
    };

    run_once().await;
    if interval_secs == 0 {
        return Ok(());
    }

    info!("sync interval: every {interval_secs}s (Ctrl+C to stop)");
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(interval_secs.max(10)));
    loop {
        tokio::select! {
            _ = interval.tick() => run_once().await,
            _ = tokio::signal::ctrl_c() => {
                info!("sync stopped");
                return Ok(());
            }
        }
    }
}
