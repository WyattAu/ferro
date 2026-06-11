pub mod db;
pub mod error;
pub mod ferro_target;
pub mod mapper;
pub mod nextcloud;
pub mod progress;
pub mod webdav;

use serde::{Deserialize, Serialize};

use error::{MigrationError, Result as MigrateResult};
use ferro_target::FerroTarget;
use mapper::{map_share, map_user, nc_path_to_ferro};
use nextcloud::NextcloudClient;
use progress::ProgressTracker;
use webdav::WebDavPipeline;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationConfig {
    pub source: NextcloudSource,
    pub target: FerroTargetConfig,
    pub options: MigrationOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NextcloudSource {
    pub url: String,
    pub username: String,
    pub password: String,
    pub db_path: Option<String>,
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
}

fn default_batch_size() -> usize {
    50
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

    let nc = NextcloudClient::new(
        &config.source.url,
        &config.source.username,
        &config.source.password,
    )?;

    tracing::info!("Validating Nextcloud connection...");
    nc.validate()
        .await
        .map_err(|e| MigrationError::connection(format!("Cannot connect to Nextcloud: {}", e)))?;

    let ferro = FerroTarget::new(&config.target.url, &config.target.admin_token)?;

    tracing::info!("Validating Ferro target connection...");
    ferro.validate().await.map_err(|e| {
        MigrationError::connection(format!("Cannot connect to Ferro target: {}", e))
    })?;

    let progress = ProgressTracker::new();

    let db = match &config.source.db_path {
        Some(path) => Some(db::NextcloudDb::open(path)?),
        None => {
            tracing::warn!("No database path provided; metadata migration will be skipped");
            None
        }
    };

    if !config.options.skip_users {
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

    if !config.options.skip_files {
        tracing::info!("Migrating files...");
        let pipeline = WebDavPipeline::new(
            &nc,
            &ferro,
            config.options.max_file_size,
            config.options.batch_size,
        );
        match pipeline
            .copy_all_files(&config.source.username, &progress)
            .await
        {
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

    if !config.options.skip_shares {
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

    if !config.options.skip_tags {
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

    if !config.options.skip_favorites {
        if let Some(ref db) = db {
            tracing::info!("Migrating favorites...");
            match db.read_filecache() {
                Ok(files) => {
                    let favorites: Vec<_> = files.iter().filter(|f| f.favorite).collect();
                    progress.set_favorite_total(favorites.len() as u64);
                    for file in &favorites {
                        let path = nc_path_to_ferro(&file.path, &config.source.username);
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

    progress.finish();
    report.duration_secs = start.elapsed().as_secs_f64();

    tracing::info!("Migration completed in {:.1}s", report.duration_secs);

    Ok(report)
}
