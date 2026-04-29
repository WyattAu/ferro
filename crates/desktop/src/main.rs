#![cfg_attr(
    all(not(debug_assertions), feature = "tauri"),
    windows_subsystem = "windows"
)]

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

        #[arg(long, default_value = "info")]
        log_level: String,
    }

    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| cli.log_level.clone().into()),
        )
        .init();

    let config = DesktopConfig {
        server_url: cli.server_url,
        username: cli.username.unwrap_or_default(),
        password: cli.password.unwrap_or_default(),
        mount_point: cli
            .mount_point
            .map(|p| p.into())
            .unwrap_or_else(DesktopConfig::default_mount_point),
        auto_mount: cli.auto_mount,
        ..Default::default()
    };

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
    gui::run();
}
