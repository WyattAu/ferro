pub mod db;
pub mod error;
pub mod ferro_target;
pub mod graph_api;
pub mod mapper;
pub mod nextcloud;
pub mod ocis;
pub mod progress;
pub mod webdav;

use serde::{Deserialize, Serialize};

use error::{MigrationError, Result as MigrateResult};
use ferro_target::FerroTarget;
use mapper::{map_share, map_user, nc_path_to_ferro};
use nextcloud::NextcloudClient;
use ocis::OcisClient;
use progress::ProgressTracker;
use webdav::{PipelineConfig, WebDavPipeline, WebDavSource};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationConfig {
    pub source: MigrationSource,
    pub target: FerroTargetConfig,
    pub options: MigrationOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MigrationSource {
    Nextcloud(NextcloudSource),
    Ocis(OcisSource),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NextcloudSource {
    pub url: String,
    pub username: String,
    pub password: String,
    pub db_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcisSource {
    pub url: String,
    pub username: String,
    /// Password for Basic Auth or OIDC ROPC grant.
    #[serde(default)]
    pub password: String,
    /// Pre-obtained Bearer token (personal access token from oCIS UI).
    #[serde(default)]
    pub token: Option<String>,
    /// OIDC client ID for automatic token acquisition via ROPC grant.
    #[serde(default)]
    pub oidc_client_id: Option<String>,
    #[serde(default = "default_ocis_webdav_base")]
    pub webdav_base: String,
}

fn default_ocis_webdav_base() -> String {
    "/dav/files".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FerroTargetConfig {
    pub url: String,
    pub admin_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationOptions {
    #[serde(default)]
    pub skip_files: bool,
    #[serde(default)]
    pub skip_users: bool,
    #[serde(default)]
    pub skip_shares: bool,
    #[serde(default)]
    pub skip_tags: bool,
    #[serde(default)]
    pub skip_favorites: bool,
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
    #[serde(default)]
    pub max_file_size: u64,
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,
    #[serde(default)]
    pub use_graph_api: bool,
    #[serde(default = "default_true")]
    pub show_progress: bool,
}

fn default_true() -> bool {
    true
}

fn default_batch_size() -> usize {
    50
}

fn default_concurrency() -> usize {
    8
}

impl Default for MigrationOptions {
    fn default() -> Self {
        Self {
            skip_files: false,
            skip_users: false,
            skip_shares: false,
            skip_tags: false,
            skip_favorites: false,
            batch_size: 50,
            max_file_size: 0,
            concurrency: 8,
            use_graph_api: false,
            show_progress: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationReport {
    pub users_migrated: usize,
    pub users_skipped: usize,
    pub files_migrated: usize,
    pub files_skipped: usize,
    pub files_failed: usize,
    pub shares_migrated: usize,
    pub tags_migrated: usize,
    pub favorites_migrated: usize,
    pub total_bytes: u64,
    pub duration_secs: f64,
    pub errors: Vec<String>,
}

pub async fn run_migration(config: MigrationConfig) -> MigrateResult<MigrationReport> {
    let start = std::time::Instant::now();
    let mut report = MigrationReport {
        users_migrated: 0,
        users_skipped: 0,
        files_migrated: 0,
        files_skipped: 0,
        files_failed: 0,
        shares_migrated: 0,
        tags_migrated: 0,
        favorites_migrated: 0,
        total_bytes: 0,
        duration_secs: 0.0,
        errors: Vec::new(),
    };

    let ferro = FerroTarget::new(&config.target.url, &config.target.admin_token)?;

    tracing::info!("Validating Ferro target connection...");
    ferro
        .validate()
        .await
        .map_err(|e| MigrationError::connection(format!("Cannot connect to Ferro target: {}", e)))?;

    let progress = ProgressTracker::new_visible(config.options.show_progress);

    match config.source {
        MigrationSource::Nextcloud(source) => {
            run_nextcloud_migration(&source, &ferro, &config.options, &progress, &mut report).await?;
        }
        MigrationSource::Ocis(source) => {
            run_ocis_migration(&source, &ferro, &config.options, &progress, &mut report).await?;
        }
    }

    progress.finish();
    report.duration_secs = start.elapsed().as_secs_f64();

    tracing::info!("Migration completed in {:.1}s", report.duration_secs);

    Ok(report)
}

async fn run_nextcloud_migration(
    source: &NextcloudSource,
    ferro: &FerroTarget,
    options: &MigrationOptions,
    progress: &ProgressTracker,
    report: &mut MigrationReport,
) -> MigrateResult<()> {
    let nc = NextcloudClient::new(&source.url, &source.username, &source.password)?;
    let webdav_source = WebDavSource::Nextcloud(nc);

    tracing::info!("Validating Nextcloud connection...");
    webdav_source
        .validate(&source.username)
        .await
        .map_err(|e| MigrationError::connection(format!("Cannot connect to Nextcloud: {}", e)))?;

    let db = match &source.db_path {
        Some(path) => Some(db::NextcloudDb::open(path)?),
        None => {
            tracing::warn!("No database path provided; metadata migration will be skipped");
            None
        }
    };

    if !options.skip_users {
        if let Some(ref db) = db {
            tracing::info!("Migrating users...");
            match db.read_users() {
                Ok(nc_users) => {
                    progress.set_user_total(nc_users.len() as u64);
                    for nc_user in &nc_users {
                        let ferro_user = map_user(nc_user);
                        match ferro.create_user(&ferro_user).await {
                            Ok(()) => {
                                report.users_migrated += 1;
                            }
                            Err(e) => {
                                tracing::warn!("Skipping user '{}': {}", nc_user.uid, e);
                                report.users_skipped += 1;
                                report.errors.push(format!("user {}: {}", nc_user.uid, e));
                            }
                        }
                        progress.inc_user();
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to read users from DB: {}", e);
                    report.errors.push(format!("read users: {}", e));
                }
            }
        }
    } else {
        tracing::info!("Skipping user migration");
    }

    if !options.skip_files {
        tracing::info!("Migrating files...");
        let pipeline = WebDavPipeline::new(&webdav_source, ferro, options.max_file_size, options.batch_size);
        match pipeline.copy_all_files(&source.username, progress).await {
            Ok(stats) => {
                report.files_migrated = stats.migrated;
                report.files_skipped = stats.skipped;
                report.files_failed = stats.failed;
                report.total_bytes = stats.total_bytes;
            }
            Err(e) => {
                tracing::error!("File migration failed: {}", e);
                report.errors.push(format!("file migration: {}", e));
            }
        }
    } else {
        tracing::info!("Skipping file migration");
    }

    if !options.skip_shares {
        if let Some(ref db) = db {
            tracing::info!("Migrating shares...");
            match db.read_shares() {
                Ok(shares) => {
                    progress.set_share_total(shares.len() as u64);
                    for share in &shares {
                        let file_path = nc_path_to_ferro(&share.file_target, &share.uid_owner);
                        let ferro_share = map_share(share, &file_path);
                        let share_type_str = match ferro_share.share_type {
                            mapper::FerroShareType::User => "user",
                            mapper::FerroShareType::Group => "group",
                            mapper::FerroShareType::Link => "link",
                            mapper::FerroShareType::Remote => "remote",
                        };
                        match ferro
                            .create_share(
                                &ferro_share.path,
                                share_type_str,
                                ferro_share.shared_with.as_deref(),
                                ferro_share.permissions.read,
                                ferro_share.permissions.write,
                            )
                            .await
                        {
                            Ok(()) => report.shares_migrated += 1,
                            Err(e) => {
                                tracing::warn!("Share migration failed: {}", e);
                                report.errors.push(format!("share: {}", e));
                            }
                        }
                        progress.inc_share();
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to read shares: {}", e);
                    report.errors.push(format!("read shares: {}", e));
                }
            }
        }
    } else {
        tracing::info!("Skipping share migration");
    }

    if !options.skip_tags {
        if let Some(ref db) = db {
            tracing::info!("Migrating tags...");
            match db.read_system_tags() {
                Ok(tags) => match db.read_tag_mappings() {
                    Ok(mappings) => {
                        let mapped_mappings: Vec<(i64, String, i64)> = mappings
                            .into_iter()
                            .map(|m| (m.object_id, m.object_type, m.systemtag_id))
                            .collect();
                        let ferro_tags = mapper::map_tags(&tags, &mapped_mappings);
                        progress.set_tag_total(ferro_tags.len() as u64);
                        for tag in &ferro_tags {
                            if let Err(e) = ferro.apply_tags("/", std::slice::from_ref(&tag.name)).await {
                                tracing::warn!("Tag migration failed: {}", e);
                            } else {
                                report.tags_migrated += 1;
                            }
                            progress.inc_tag();
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to read tag mappings: {}", e);
                        report.errors.push(format!("read tag mappings: {}", e));
                    }
                },
                Err(e) => {
                    tracing::error!("Failed to read tags: {}", e);
                    report.errors.push(format!("read tags: {}", e));
                }
            }
        }
    } else {
        tracing::info!("Skipping tag migration");
    }

    if !options.skip_favorites {
        if let Some(ref db) = db {
            tracing::info!("Migrating favorites...");
            match db.read_filecache() {
                Ok(files) => {
                    let favorites: Vec<_> = files.iter().filter(|f| f.favorite).collect();
                    progress.set_favorite_total(favorites.len() as u64);
                    for file in &favorites {
                        let path = nc_path_to_ferro(&file.path, &source.username);
                        if let Err(e) = ferro.set_favorite(&path, true).await {
                            tracing::warn!("Favorite migration failed for {}: {}", path, e);
                        } else {
                            report.favorites_migrated += 1;
                        }
                        progress.inc_favorite();
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to read filecache for favorites: {}", e);
                    report.errors.push(format!("read favorites: {}", e));
                }
            }
        }
    } else {
        tracing::info!("Skipping favorite migration");
    }

    Ok(())
}

async fn run_ocis_migration(
    source: &OcisSource,
    ferro: &FerroTarget,
    options: &MigrationOptions,
    progress: &ProgressTracker,
    report: &mut MigrationReport,
) -> MigrateResult<()> {
    // Determine auth method: token > OIDC > basic
    // When token + oidc_client_id are both provided, use token but enable refresh
    let ocis = if let Some(ref token) = source.token {
        if let Some(ref client_id) = source.oidc_client_id {
            tracing::info!("Using Bearer token with OIDC refresh (client_id={})...", client_id);
            let mut client = OcisClient::with_token(&source.url, &source.username, token)?;
            // Set up OIDC refresh credentials
            client
                .set_oidc_refresh(
                    &source.url,
                    source.username.clone(),
                    source.password.clone(),
                    client_id.clone(),
                )
                .await;
            client
        } else {
            tracing::info!("Using Bearer token authentication for oCIS");
            OcisClient::with_token(&source.url, &source.username, token)?
        }
    } else if let Some(ref client_id) = source.oidc_client_id {
        tracing::info!("Acquiring OIDC token via ROPC grant (client_id={})...", client_id);
        OcisClient::with_oidc(&source.url, &source.username, &source.password, client_id).await?
    } else if !source.password.is_empty() {
        tracing::info!("Using Basic authentication for oCIS");
        OcisClient::new(&source.url, &source.username, &source.password)?
    } else {
        return Err(MigrationError::authentication(
            "No auth method specified for oCIS. \
             Provide --source-token (PAT), --oidc-client-id + password, or --source-pass (basic auth).",
        ));
    };
    let ocis = ocis.with_webdav_base(&source.webdav_base);
    let ocis_token = ocis.token().to_string();
    let ocis_for_shares = ocis.clone();
    let webdav_source = WebDavSource::Ocis(ocis);

    tracing::info!("Validating oCIS connection...");
    webdav_source
        .validate(&source.username)
        .await
        .map_err(|e| MigrationError::connection(format!("Cannot connect to oCIS: {}", e)))?;

    if !options.skip_users {
        tracing::info!("Migrating users from oCIS via Graph API...");
        let graph = crate::graph_api::GraphApiClient::new(&source.url, &ocis_token);
        match graph.list_users().await {
            Ok(users) => {
                progress.set_user_total(users.len() as u64);
                for graph_user in &users {
                    let username = graph_user
                        .userPrincipalName
                        .as_deref()
                        .or(graph_user.displayName.as_deref())
                        .unwrap_or("unknown");
                    let email = graph_user.mail.as_deref().unwrap_or("");
                    let display_name = graph_user.displayName.as_deref().unwrap_or(username);
                    let ferro_user = mapper::FerroUser {
                        username: username.to_string(),
                        email: if email.is_empty() { None } else { Some(email.to_string()) },
                        display_name: if display_name.is_empty() { None } else { Some(display_name.to_string()) },
                        role: "user".to_string(),
                    };
                    match ferro.create_user(&ferro_user).await {
                        Ok(()) => {
                            report.users_migrated += 1;
                        }
                        Err(e) => {
                            tracing::warn!("Skipping user '{}': {}", username, e);
                            report.users_skipped += 1;
                            report.errors.push(format!("user {}: {}", username, e));
                        }
                    }
                    progress.inc_user();
                }
            }
            Err(e) => {
                tracing::error!("Failed to list oCIS users via Graph API: {}", e);
                report.errors.push(format!("list oCIS users: {}", e));
            }
        }
    } else {
        tracing::info!("Skipping user migration");
    }

    if !options.skip_files {
        tracing::info!("Migrating files from oCIS...");
        let pipeline = WebDavPipeline::new(&webdav_source, ferro, options.max_file_size, options.batch_size);
        let config = PipelineConfig {
            transfer_workers: options.concurrency,
            ..PipelineConfig::default()
        };
        let pipeline = pipeline.with_config(config);
        let file_result = if options.use_graph_api {
            tracing::info!("Using Graph API for file discovery + WebDAV for download");
            pipeline.copy_all_files_graph(&source.username, progress).await
        } else {
            pipeline.copy_all_files(&source.username, progress).await
        };
        match file_result {
            Ok(stats) => {
                report.files_migrated = stats.migrated;
                report.files_skipped = stats.skipped;
                report.files_failed = stats.failed;
                report.total_bytes = stats.total_bytes;
            }
            Err(e) => {
                tracing::error!("File migration failed: {}", e);
                report.errors.push(format!("file migration: {}", e));
            }
        }
    } else {
        tracing::info!("Skipping file migration");
    }

    if !options.skip_shares {
        tracing::info!("Migrating shares from oCIS via OCS API...");
        match ocis_for_shares.list_shares().await {
            Ok(shares) => {
                progress.set_share_total(shares.len() as u64);
                for ocs_share in &shares {
                    let path = if let Some(ref p) = ocs_share.path {
                        format!("/remote.php/dav/files/{}{}", source.username, p)
                    } else {
                        tracing::warn!("Skipping share without path: {:?}", ocs_share.id);
                        progress.inc_share();
                        continue;
                    };
                    let share_type_str = match ocs_share.share_type {
                        0 => "user",
                        1 => "group",
                        3 => "remote",
                        _ => "link",
                    };
                    let shared_with = ocs_share.share_with.as_deref();
                    let read = ocs_share.permissions & 1 != 0;
                    let write = ocs_share.permissions & 2 != 0;
                    match ferro.create_share(&path, share_type_str, shared_with, read, write).await {
                        Ok(()) => report.shares_migrated += 1,
                        Err(e) => {
                            tracing::warn!("Share migration failed: {}", e);
                            report.errors.push(format!("share: {}", e));
                        }
                    }
                    progress.inc_share();
                }
            }
            Err(e) => {
                tracing::error!("Failed to list oCIS shares: {}", e);
                report.errors.push(format!("list oCIS shares: {}", e));
            }
        }
    } else {
        tracing::info!("Skipping share migration");
    }

    if !options.skip_tags {
        tracing::info!("oCIS tag migration requires extended PROPFIND (not yet implemented)");
        tracing::info!("Skipping tag migration");
    } else {
        tracing::info!("Skipping tag migration");
    }

    if !options.skip_favorites {
        tracing::info!("oCIS favorites migration requires extended PROPFIND (not yet implemented)");
        tracing::info!("Skipping favorite migration");
    } else {
        tracing::info!("Skipping favorite migration");
    }

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationReport {
    pub total_source_files: usize,
    pub verified: usize,
    pub missing_on_target: usize,
    pub hash_mismatch: usize,
    pub errors: Vec<String>,
    pub duration_secs: f64,
}

pub async fn verify_migration(config: MigrationConfig) -> MigrateResult<VerificationReport> {
    let start = std::time::Instant::now();
    let mut report = VerificationReport {
        total_source_files: 0,
        verified: 0,
        missing_on_target: 0,
        hash_mismatch: 0,
        errors: Vec::new(),
        duration_secs: 0.0,
    };

    let ferro = FerroTarget::new(&config.target.url, &config.target.admin_token)?;

    tracing::info!("Validating Ferro target connection...");
    ferro
        .validate()
        .await
        .map_err(|e| MigrationError::connection(format!("Cannot connect to Ferro target: {}", e)))?;

    let webdav_source = match &config.source {
        MigrationSource::Nextcloud(source) => {
            let nc = NextcloudClient::new(&source.url, &source.username, &source.password)?;
            tracing::info!("Validating Nextcloud connection...");
            let ws = WebDavSource::Nextcloud(nc);
            ws.validate(&source.username)
                .await
                .map_err(|e| MigrationError::connection(format!("Cannot connect to Nextcloud: {}", e)))?;
            ws
        }
        MigrationSource::Ocis(source) => {
            let ocis = if let Some(ref token) = source.token {
                OcisClient::with_token(&source.url, &source.username, token)?
            } else if !source.password.is_empty() {
                OcisClient::new(&source.url, &source.username, &source.password)?
            } else {
                return Err(MigrationError::authentication(
                    "No auth method specified for oCIS verification.",
                ));
            };
            let ocis = ocis.with_webdav_base(&source.webdav_base);
            let ws = WebDavSource::Ocis(ocis);
            tracing::info!("Validating oCIS connection...");
            ws.validate(&source.username)
                .await
                .map_err(|e| MigrationError::connection(format!("Cannot connect to oCIS: {}", e)))?;
            ws
        }
    };

    let user = match &config.source {
        MigrationSource::Nextcloud(s) => s.username.clone(),
        MigrationSource::Ocis(s) => s.username.clone(),
    };

    tracing::info!("Listing source files for verification...");
    let source_files = webdav_source.list_directory_recursive(&user, "/").await?;
    let source_files: Vec<_> = source_files.into_iter().filter(|e| !e.is_collection).collect();
    report.total_source_files = source_files.len();
    tracing::info!("Found {} source files to verify", source_files.len());

    for entry in &source_files {
        let ferro_path = webdav::dav_path_to_ferro(&entry.path);

        // Download from source to get content hash
        match webdav_source.download_file(&user, &entry.path).await {
            Ok(content) => {
                let content_hash = {
                    use sha2::Digest;
                    let mut hasher = sha2::Sha256::new();
                    hasher.update(&content);
                    format!("{:x}", hasher.finalize())
                };

                if ferro.file_exists_with_hash(&ferro_path, &content_hash).await {
                    report.verified += 1;
                } else {
                    // Check if file exists but hash differs
                    let url = format!("{}/api/v1/files{}", config.target.url, ferro_path);
                    match ferro.http.get(&url).send().await {
                        Ok(resp) if resp.status().is_success() => {
                            report.hash_mismatch += 1;
                            tracing::warn!("Hash mismatch: {}", ferro_path);
                        }
                        _ => {
                            report.missing_on_target += 1;
                            tracing::warn!("Missing on target: {}", ferro_path);
                        }
                    }
                }
            }
            Err(e) => {
                report.errors.push(format!("{}: {}", entry.path, e));
            }
        }
    }

    report.duration_secs = start.elapsed().as_secs_f64();
    Ok(report)
}
