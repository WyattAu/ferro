mod client;
mod commands;

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::Shell;
use ferro_common::format::format_size;
use sha2::Digest;

#[derive(Parser, Debug)]
#[command(
    name = "ferro",
    about = "Ferro Storage Orchestrator CLI",
    version,
    long_version = env!("CARGO_PKG_VERSION")
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(long, env = "FERRO_URL", default_value = "http://localhost:8080")]
    server_url: String,

    #[arg(long, env = "FERRO_TOKEN")]
    token: Option<String>,

    #[arg(long, default_value = "text")]
    output: String,

    #[arg(short, long)]
    verbose: bool,

    /// Generate shell completion script and exit
    #[arg(long = "generate-completions", value_enum)]
    generate_completions: Option<Shell>,

    /// Print man page to stdout and exit
    #[arg(long = "print-man-page")]
    print_man_page: bool,
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[command(subcommand)]
    Server(ServerCommands),

    #[command(subcommand)]
    File(FileCommands),

    #[command(subcommand)]
    User(UserCommands),

    #[command(subcommand)]
    Policy(PolicyCommands),

    #[command(subcommand)]
    Share(ShareCommands),

    #[command(subcommand)]
    Snapshot(SnapshotCommands),

    #[command(subcommand)]
    Backup(BackupCommands),

    #[command(subcommand)]
    Migrate(MigrateCommands),

    Info,
}

#[derive(Subcommand, Debug)]
enum ServerCommands {
    Health,
    Capabilities,
}

#[derive(Subcommand, Debug)]
enum FileCommands {
    List {
        #[arg(default_value = "/")]
        path: String,
        #[arg(short = 'd', long, default_value = "1")]
        depth: u8,
    },
    Upload {
        local_path: String,
        remote_path: String,
    },
    Download {
        remote_path: String,
        local_path: String,
    },
    Delete {
        path: String,
        #[arg(short = 'f', long)]
        force: bool,
    },
    Mkdir {
        path: String,
    },
    Info {
        path: String,
    },
    Hash {
        path: String,
    },
    Search {
        query: String,
        #[arg(short = 'n', long, default_value = "20")]
        limit: usize,
    },
}

#[derive(Subcommand, Debug)]
enum UserCommands {
    List,
    Whoami,
}

#[derive(Subcommand, Debug)]
enum PolicyCommands {
    List,
    Add { file: String },
    Remove { id: String },
}

#[derive(Subcommand, Debug)]
enum ShareCommands {
    List,
    Create {
        path: String,
        #[arg(long)]
        expires_hours: Option<u64>,
        #[arg(long)]
        password: Option<String>,
    },
    Delete {
        token: String,
    },
}

#[derive(Subcommand, Debug)]
enum SnapshotCommands {
    List,
    Create,
    Delete { id: String },
    Restore { id: String },
}

#[derive(Subcommand, Debug)]
enum BackupCommands {
    /// Create a full backup
    Create,
    /// List all backups
    List,
    /// Restore from a backup
    Restore { id: String },
    /// Delete a backup
    Delete { id: String },
}

#[derive(Subcommand, Debug)]
enum MigrateCommands {
    /// Migrate from a Nextcloud instance to Ferro
    #[command(name = "nextcloud")]
    Nextcloud {
        /// Nextcloud instance URL (e.g. https://nextcloud.example.com)
        #[arg(long, env = "NC_SOURCE_URL")]
        source_url: String,

        /// Nextcloud admin username
        #[arg(long, env = "NC_SOURCE_USER")]
        source_user: String,

        /// Nextcloud admin password
        #[arg(long, env = "NC_SOURCE_PASS")]
        source_pass: String,

        /// Path to Nextcloud SQLite database file (for metadata migration)
        #[arg(long)]
        source_db: Option<String>,

        /// Ferro target server URL
        #[arg(long, env = "FERRO_URL", default_value = "http://localhost:8080")]
        target_url: String,

        /// Ferro admin API token
        #[arg(long, env = "FERRO_TOKEN")]
        target_token: String,

        /// Skip file migration
        #[arg(long)]
        skip_files: bool,

        /// Skip user migration
        #[arg(long)]
        skip_users: bool,

        /// Skip share migration
        #[arg(long)]
        skip_shares: bool,

        /// Skip tag migration
        #[arg(long)]
        skip_tags: bool,

        /// Skip favorite migration
        #[arg(long)]
        skip_favorites: bool,

        /// Maximum file size to migrate in bytes (0 = unlimited)
        #[arg(long, default_value = "0")]
        max_file_size: u64,

        /// Number of files per batch
        #[arg(long, default_value = "50")]
        batch_size: usize,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Early-exit: generate shell completions
    if let Some(shell) = cli.generate_completions {
        let mut cmd = Cli::command();
        clap_complete::generate(shell, &mut cmd, "ferro", &mut std::io::stdout());
        return Ok(());
    }

    // Early-exit: print man page
    if cli.print_man_page {
        print_cli_man_page();
        return Ok(());
    }

    let log_level = if cli.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| log_level.into()),
        )
        .init();

    let ferro_client = client::FerroClient::new(&cli.server_url, cli.token.as_deref())?;

    let Some(command) = cli.command else {
        eprintln!("No subcommand provided. Use --help for usage information.");
        std::process::exit(1);
    };

    match command {
        Commands::Server(cmd) => handle_server(cmd, &ferro_client).await,
        Commands::File(cmd) => handle_file(cmd, &ferro_client).await,
        Commands::User(cmd) => handle_user(cmd, &ferro_client).await,
        Commands::Policy(cmd) => handle_policy(cmd, &ferro_client).await,
        Commands::Share(cmd) => handle_share(cmd, &ferro_client).await,
        Commands::Snapshot(cmd) => handle_snapshot(cmd, &ferro_client).await,
        Commands::Backup(ref cmd) => cmd_backup(&ferro_client, cmd).await,
        Commands::Migrate(cmd) => cmd_migrate(cmd).await,
        Commands::Info => cmd_info(&ferro_client).await,
    }
}

async fn handle_server(cmd: ServerCommands, client: &client::FerroClient) -> anyhow::Result<()> {
    match cmd {
        ServerCommands::Health => {
            let healthy = client.health_check().await?;
            println!(
                "{}",
                if healthy {
                    "Server is healthy"
                } else {
                    "Server is unhealthy"
                }
            );
        }
        ServerCommands::Capabilities => {
            let caps = client.get_capabilities().await?;
            println!("WebDAV: {}", caps.webdav);
            println!("Authentication: {}", caps.auth);
        }
    }
    Ok(())
}

async fn handle_file(cmd: FileCommands, client: &client::FerroClient) -> anyhow::Result<()> {
    match cmd {
        FileCommands::List { path, depth } => {
            let entries = client.list_files(&path, depth).await?;
            for entry in &entries {
                let icon = if entry.is_collection { "d" } else { "-" };
                let size = if entry.is_collection {
                    String::new()
                } else {
                    format!(" ({})", format_size(entry.size))
                };
                println!("{} {}{}", icon, entry.path, size);
            }
            println!("\n{} items", entries.len());
        }
        FileCommands::Upload {
            local_path,
            remote_path,
        } => {
            let content = tokio::fs::read(&local_path).await?;
            let hash = sha2::Sha256::digest(&content);
            println!(
                "Uploading {} ({} bytes, hash: {})",
                local_path,
                content.len(),
                hex::encode(hash)
            );
            client.put_file(&remote_path, &content).await?;
            println!("Uploaded to {}", remote_path);
        }
        FileCommands::Download {
            remote_path,
            local_path,
        } => {
            println!("Downloading {} -> {}", remote_path, local_path);
            let content = client.get_file(&remote_path).await?;
            tokio::fs::write(&local_path, &content).await?;
            println!("Downloaded {} bytes", content.len());
        }
        FileCommands::Delete { path, force } => {
            if !force {
                println!("Are you sure you want to delete {}? (y/N)", path);
                let mut confirm = String::new();
                std::io::stdin().read_line(&mut confirm)?;
                if confirm.trim().to_lowercase() != "y" {
                    println!("Cancelled");
                    return Ok(());
                }
            }
            client.delete_file(&path).await?;
            println!("Deleted {}", path);
        }
        FileCommands::Info { path } => {
            let meta = client.head_file(&path).await?;
            println!("Path:         {}", meta.path);
            println!("Size:         {} bytes", meta.size);
            println!("Content Hash: {}", meta.content_hash.as_str());
            println!("MIME Type:    {}", meta.mime_type);
            println!("Collection:   {}", meta.is_collection);
            println!("ETag:         {}", meta.etag);
            println!("Modified:     {}", meta.modified_at);
            println!("Created:      {}", meta.created_at);
        }
        FileCommands::Hash { path } => {
            let content = tokio::fs::read(&path).await?;
            let hash = sha2::Sha256::digest(&content);
            println!("SHA-256({}): {}", path, hex::encode(hash));
        }
        FileCommands::Mkdir { path } => {
            client.create_directory(&path).await?;
            println!("Created directory: {}", path);
        }
        FileCommands::Search { query, limit } => {
            let results = client.search(&query, limit).await?;
            if results.is_empty() {
                println!("No results found for: {}", query);
            } else {
                println!("Found {} result(s) for: {}", results.len(), query);
                println!();
                for result in &results {
                    let path = result.get("path").and_then(|v| v.as_str()).unwrap_or("?");
                    let score = result.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let snippet = result.get("snippet").and_then(|v| v.as_str()).unwrap_or("");
                    println!("  {} (score: {:.2})", path, score);
                    if !snippet.is_empty() {
                        println!("    {}", snippet);
                    }
                }
            }
        }
    }
    Ok(())
}

async fn handle_user(cmd: UserCommands, client: &client::FerroClient) -> anyhow::Result<()> {
    match cmd {
        UserCommands::List => {
            let resp = client.list_users().await?;
            println!(
                "{:<20} {:<30} {:<10} {:<10}",
                "USERNAME", "EMAIL", "ROLE", "STATUS"
            );
            println!("{}", "-".repeat(70));
            for user in &resp {
                println!(
                    "{:<20} {:<30} {:<10} {:<10}",
                    user.username,
                    user.email.as_deref().unwrap_or("-"),
                    user.role,
                    user.status
                );
            }
        }
        UserCommands::Whoami => {
            let info = client.whoami().await?;
            println!("Subject:   {}", info.subject);
            println!("Issuer:    {}", info.issuer);
            println!("Audience:  {}", info.audience);
            if let Some(email) = &info.email {
                println!("Email:     {}", email);
            }
            if let Some(name) = &info.name {
                println!("Name:      {}", name);
            }
        }
    }
    Ok(())
}

async fn handle_policy(cmd: PolicyCommands, client: &client::FerroClient) -> anyhow::Result<()> {
    match cmd {
        PolicyCommands::List => {
            let result = client.list_policies().await?;
            let configured = result
                .get("configured")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            println!(
                "Cedar authorization: {}",
                if configured { "enabled" } else { "disabled" }
            );
            let policies = result
                .get("policies")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            if policies.is_empty() {
                println!("No policies loaded.");
            } else {
                println!("Policies ({}):", policies.len());
                for (i, policy) in policies.iter().enumerate() {
                    if let Some(s) = policy.as_str() {
                        println!("  [{}] {}", i, s);
                    } else {
                        println!(
                            "  [{}] {}",
                            i,
                            serde_json::to_string_pretty(policy).unwrap_or_default()
                        );
                    }
                }
            }
        }
        PolicyCommands::Add { file } => {
            let content = tokio::fs::read_to_string(&file).await?;
            println!("Adding policy from {}...", file);
            let result = client.add_policy(&content).await?;
            if let Some(status) = result.get("status").and_then(|v| v.as_str()) {
                println!("Policy added: {}", status);
            } else {
                println!("Response: {}", serde_json::to_string_pretty(&result)?);
            }
        }
        PolicyCommands::Remove { id } => {
            println!("Removing policy {}...", id);
            let result = client.remove_policy(&id).await?;
            if let Some(error) = result.get("error").and_then(|v| v.as_str()) {
                println!("Error: {}", error);
            } else {
                println!("Response: {}", serde_json::to_string_pretty(&result)?);
            }
        }
    }
    Ok(())
}

async fn handle_share(cmd: ShareCommands, client: &client::FerroClient) -> anyhow::Result<()> {
    match cmd {
        ShareCommands::List => {
            let shares = client.list_shares().await?;
            if shares.is_empty() {
                println!("No active share links");
            } else {
                println!("Active share links ({}):", shares.len());
                println!();
                for share in &shares {
                    let token = share.get("token").and_then(|v| v.as_str()).unwrap_or("?");
                    let path = share.get("path").and_then(|v| v.as_str()).unwrap_or("?");
                    let downloads = share
                        .get("download_count")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);
                    println!("  {} -> {} ({} downloads)", token, path, downloads);
                }
            }
        }
        ShareCommands::Create {
            path,
            expires_hours,
            password,
        } => {
            println!("Creating share link for {}...", path);
            let share = client
                .create_share(&path, expires_hours, password.as_deref())
                .await?;
            let token = share.get("token").and_then(|v| v.as_str()).unwrap_or("?");
            let url = share.get("url").and_then(|v| v.as_str()).unwrap_or("");
            println!("Token: {}", token);
            println!("URL:   {}", url);
        }
        ShareCommands::Delete { token } => {
            client.delete_share(&token).await?;
            println!("Deleted share link: {}", token);
        }
    }
    Ok(())
}

async fn handle_snapshot(
    cmd: SnapshotCommands,
    client: &client::FerroClient,
) -> anyhow::Result<()> {
    match cmd {
        SnapshotCommands::List => {
            let snapshots = client.list_snapshots().await?;
            if snapshots.is_empty() {
                println!("No snapshots");
            } else {
                println!("Snapshots ({}):", snapshots.len());
                println!();
                for snap in &snapshots {
                    let id = snap.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                    let file_count = snap.get("file_count").and_then(|v| v.as_u64()).unwrap_or(0);
                    let created = snap
                        .get("created_at")
                        .and_then(|v| v.as_str())
                        .unwrap_or("?");
                    println!("  {} ({} files, created {})", id, file_count, created);
                }
            }
        }
        SnapshotCommands::Create => {
            println!("Creating snapshot...");
            let snap = client.create_snapshot().await?;
            let id = snap.get("id").and_then(|v| v.as_str()).unwrap_or("?");
            let file_count = snap.get("file_count").and_then(|v| v.as_u64()).unwrap_or(0);
            println!("Snapshot {} created ({} files)", id, file_count);
        }
        SnapshotCommands::Delete { id } => {
            client.delete_snapshot(&id).await?;
            println!("Deleted snapshot: {}", id);
        }
        SnapshotCommands::Restore { id } => {
            println!("Restoring snapshot {}...", id);
            client.restore_snapshot(&id).await?;
            println!("Snapshot {} restored", id);
        }
    }
    Ok(())
}

async fn cmd_migrate(cmd: MigrateCommands) -> anyhow::Result<()> {
    match cmd {
        MigrateCommands::Nextcloud {
            source_url,
            source_user,
            source_pass,
            source_db,
            target_url,
            target_token,
            skip_files,
            skip_users,
            skip_shares,
            skip_tags,
            skip_favorites,
            max_file_size,
            batch_size,
        } => {
            let config = ferro_migrate::MigrationConfig {
                source: ferro_migrate::NextcloudSource {
                    url: source_url,
                    username: source_user,
                    password: source_pass,
                    db_path: source_db,
                },
                target: ferro_migrate::FerroTargetConfig {
                    url: target_url,
                    admin_token: target_token,
                },
                options: ferro_migrate::MigrationOptions {
                    skip_files,
                    skip_users,
                    skip_shares,
                    skip_tags,
                    skip_favorites,
                    batch_size,
                    max_file_size,
                },
            };

            println!("Starting Nextcloud -> Ferro migration...");
            let report = ferro_migrate::run_migration(config).await?;

            println!();
            println!("Migration Report");
            println!("================");
            println!(
                "Users:      {} migrated, {} skipped",
                report.users_migrated, report.users_skipped
            );
            println!(
                "Files:      {} migrated, {} skipped, {} failed",
                report.files_migrated, report.files_skipped, report.files_failed
            );
            println!("Shares:     {} migrated", report.shares_migrated);
            println!("Tags:       {} migrated", report.tags_migrated);
            println!("Favorites:  {} migrated", report.favorites_migrated);
            println!(
                "Total data: {:.2} MB",
                report.total_bytes as f64 / 1_048_576.0
            );
            println!("Duration:   {:.1}s", report.duration_secs);

            if !report.errors.is_empty() {
                println!();
                println!("Errors ({}):", report.errors.len());
                for err in &report.errors {
                    println!("  - {}", err);
                }
            }
        }
    }
    Ok(())
}

async fn cmd_backup(client: &client::FerroClient, cmd: &BackupCommands) -> anyhow::Result<()> {
    match cmd {
        BackupCommands::Create => {
            println!("Creating backup...");
            let resp = client
                .post_json("/api/admin/backup", &serde_json::json!({}))
                .await?;
            let id = resp
                .get("backup_id")
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            let files = resp.get("file_count").and_then(|v| v.as_u64()).unwrap_or(0);
            let size = resp
                .get("total_bytes")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            println!(
                "Backup {} created ({} files, {:.2} MB)",
                id,
                files,
                size as f64 / 1_048_576.0
            );
        }
        BackupCommands::List => {
            let resp = client.get_json("/api/admin/backups").await?;
            let backups = resp.get("backups").and_then(|v| v.as_array());
            match backups {
                Some(arr) if !arr.is_empty() => {
                    println!("{:<30} {:>10} {:>12}", "ID", "FILES", "SIZE");
                    println!("{}", "-".repeat(70));
                    for b in arr {
                        let id = b.get("backup_id").and_then(|v| v.as_str()).unwrap_or("?");
                        let files = b.get("file_count").and_then(|v| v.as_u64()).unwrap_or(0);
                        let size = b.get("total_bytes").and_then(|v| v.as_u64()).unwrap_or(0);
                        println!(
                            "{:<30} {:>10} {:>8.1} MB",
                            id,
                            files,
                            size as f64 / 1_048_576.0
                        );
                    }
                }
                _ => println!("No backups found"),
            }
        }
        BackupCommands::Restore { id } => {
            println!("Restoring backup {}...", id);
            client
                .post_json("/api/admin/restore", &serde_json::json!({"backup_id": id}))
                .await?;
            println!("Backup {} restored", id);
        }
        BackupCommands::Delete { id } => {
            client.delete(&format!("/api/admin/backup/{}", id)).await?;
            println!("Deleted backup: {}", id);
        }
    }
    Ok(())
}

async fn cmd_info(client: &client::FerroClient) -> anyhow::Result<()> {
    println!("Ferro Storage Orchestrator");
    println!("========================");

    let healthy = client.health_check().await?;
    println!(
        "Server Status: {}",
        if healthy { "Connected" } else { "Disconnected" }
    );
    println!("Server URL:    {}", client.server_url());

    match client.get_capabilities().await {
        Ok(caps) => {
            println!("WebDAV:         {}", caps.webdav);
        }
        Err(_) => {
            println!("WebDAV:         Unknown (server not reachable)");
        }
    }

    Ok(())
}

fn print_cli_man_page() {
    let version = env!("CARGO_PKG_VERSION");
    print!(
        r#".TH FERRO 1 "June 2026" "Ferro {version}" "User Commands"
.SH NAME
ferro \- Ferro Storage Orchestrator CLI client
.SH SYNOPSIS
.B ferro
[\fIOPTIONS\fR] \fISUBCOMMAND\fR
.SH DESCRIPTION
.B ferro
is a command-line client for interacting with Ferro storage servers.
It supports file operations, user management, sharing, snapshots, and backups.
.SH OPTIONS
.TP
.BI \-\-server-url " " \fIURL\fR
Ferro server URL (default: http://localhost:8080). Also set via FERRO_URL.
.TP
.BI \-\-token " " \fITOKEN\fR
Authentication token. Also set via FERRO_TOKEN.
.TP
.BI \-\-output " " \fIFORMAT\fR
Output format: text or json (default: text).
.TP
.BI \-v ", " \-\-verbose
Enable debug logging.
.TP
.BI \-\-generate-completions " " \fISHELL\fR
Generate shell completion script (bash, zsh, fish, powershell) and exit.
.TP
.BI \-\-print-man-page
Print this man page to stdout and exit.
.TP
.BI \-h ", " \-\-help
Print help information.
.TP
.BI \-V ", " \-\-version
Print version information.
.SH SUBCOMMANDS
.TP
.B server health|capabilities
Query server status and capabilities.
.TP
.B file list|upload|download|delete|mkdir|info|hash|search
File operations on the remote server.
.TP
.B user list|whoami
User management operations.
.TP
.B policy list|add|remove
Cedar authorization policy management.
.TP
.B share list|create|delete
Share link management.
.TP
.B snapshot list|create|delete|restore
Snapshot management.
.TP
.B backup create|list|restore|delete
Backup management.
.TP
.B info
Display server connection info.
.SH EXAMPLES
List files at the root:
.RS
.B ferro file list /
.RE
.PP
Upload a file:
.RS
.B ferro file upload ./photo.jpg /photos/photo.jpg
.RE
.PP
Create a share link:
.RS
.B ferro share create /documents/report.pdf --expires-hours 24
.RE
.PP
Generate bash completions:
.RS
.B ferro --generate-completions bash > /etc/bash_completion.d/ferro
.RE
.PP
Install man page:
.RS
.B ferro --print-man-page > /usr/share/man/man1/ferro.1
.RE
.SH ENVIRONMENT
.TP
.B FERRO_URL
Server URL (alternative to --server-url).
.TP
.B FERRO_TOKEN
Authentication token (alternative to --token).
.SH AUTHOR
Ferro Contributors
.SH LICENSE
See the Ferro project repository for license details.
"#,
        version = version
    );
}
