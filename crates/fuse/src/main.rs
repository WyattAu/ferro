#[cfg(feature = "offline-cache")]
mod cache;

#[cfg(target_os = "linux")]
mod fs;

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("ferro-fuse is only supported on Linux");
    std::process::exit(1);
}

#[cfg(target_os = "linux")]
#[derive(Debug, clap::Parser)]
#[command(name = "ferro-fuse", version, about = "FUSE filesystem mount for Ferro")]
struct Cli {
    #[arg(long, env = "FERRO_URL", default_value = "http://localhost:8080")]
    server_url: String,

    #[arg(long, env = "FERRO_MOUNT")]
    mount: String,

    #[arg(long, env = "FERRO_TOKEN")]
    token: Option<String>,

    #[arg(long, default_value_t = false)]
    allow_root: bool,

    #[arg(long, default_value_t = true)]
    #[allow(dead_code)]
    foreground: bool,

    #[cfg(feature = "offline-cache")]
    #[arg(long, env = "FERRO_CACHE_DIR")]
    cache_dir: Option<String>,
}

#[cfg(target_os = "linux")]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use clap::Parser;
    use std::path::{Path, PathBuf};
    use tracing::info;

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let cli = Cli::parse();

    if !Path::new(&cli.mount).exists() {
        std::fs::create_dir_all(&cli.mount)?;
    }

    let mount_path = PathBuf::from(&cli.mount);
    // SAFETY: getuid() is always safe and has no preconditions
    let uid = unsafe { libc::getuid() };
    // SAFETY: getgid() is always safe and has no preconditions
    let gid = unsafe { libc::getgid() };

    #[cfg(feature = "offline-cache")]
    let cache_dir = Some(cli.cache_dir.clone().unwrap_or_else(|| {
        std::env::var("HOME")
            .map(|h| format!("{}/.cache/ferro-fuse", h))
            .unwrap_or_else(|_| "/tmp/ferro-fuse".to_string())
    }));
    #[cfg(not(feature = "offline-cache"))]
    let cache_dir: Option<String> = None;

    let fs_impl = fs::FerroFs::new(&cli.server_url, cli.token.as_deref(), uid, gid, cache_dir.as_deref())?;

    info!("Mounting Ferro at {} (server: {})", cli.mount, cli.server_url);

    let mut options = fuse3::MountOptions::default();
    options.allow_root(cli.allow_root);
    options.uid(uid);
    options.gid(gid);

    let session = fuse3::raw::Session::new(options);
    let _guard = session
        .mount(fs_impl, &mount_path)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to mount at {}: {}", cli.mount, e))?;

    info!("FUSE filesystem mounted successfully");
    tokio::signal::ctrl_c().await?;
    info!("Unmounting...");
    Ok(())
}
