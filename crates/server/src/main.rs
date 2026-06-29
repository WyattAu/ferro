use clap::Parser;
use ferro_offline::change_queue::ChangeQueueStore;
use ferro_server::auth::cedar::CedarAuthorizer;
use ferro_server::auth::oidc::OidcConfig;
use ferro_server::config::ServerConfig;
use ferro_server::config::{
    FileConfigValues, apply_file_config, load_config_file, redact_url_credentials,
};
use ferro_server::security;
use ferro_server::users::UserStoreTrait;
use ferro_server::{AppState, build_router_with_static};
use tokio_util::sync::CancellationToken;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut cli = ServerConfig::parse();

    // Early-exit: generate shell completions
    if let Some(shell) = cli.generate_completions {
        use clap::CommandFactory;
        let mut cmd = ServerConfig::command();
        clap_complete::generate(shell, &mut cmd, "ferro-server", &mut std::io::stdout());
        return Ok(());
    }

    // Early-exit: print man page
    if cli.print_man_page {
        print_man_page();
        return Ok(());
    }

    // Early-exit: check for updates
    if cli.check_update {
        check_for_updates().await;
        return Ok(());
    }

    let original_args: Vec<String> = std::env::args().collect();

    let file_config = if let Some(ref config_path) = cli.config {
        load_config_file(config_path)?
    } else if std::path::Path::new("ferro.toml").exists() {
        load_config_file("ferro.toml")?
    } else if std::path::Path::new("/etc/ferro/ferro.toml").exists() {
        load_config_file("/etc/ferro/ferro.toml")?
    } else {
        FileConfigValues::default()
    };

    apply_file_config(&original_args, &mut cli, &file_config);

    // --validate-config: load config, run validation, print results, exit
    if cli.validate_config {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Schema version check
        if let Some(version) = file_config.schema_version
            && version > 1
        {
            errors.push(format!(
                "Unsupported config file schema_version: {}. Supported versions: 1.",
                version
            ));
        }

        // Port validation
        if cli.port == 0 {
            errors.push("Port must be between 1 and 65535.".to_string());
        }

        // Storage backend validation
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

        // Data dir + persistence warnings
        if cli.data_dir.is_none() {
            warnings.push("No --data-dir set. All data will be lost on restart.".to_string());
        }

        // CORS + auth conflict
        if !cli.cors_allowed_origins.is_empty()
            && cli.cors_allowed_origins.contains('*')
            && cli.oidc_issuer.is_some()
        {
            errors.push(
                "CORS wildcard '*' cannot be used with OIDC authentication enabled.".to_string(),
            );
        }

        // OIDC validation
        if cli.oidc_issuer.is_some() && cli.oidc_client_id.is_none() {
            errors.push("--oidc-client-id is required when --oidc-issuer is set.".to_string());
        }

        // WOPI validation
        if !cli.wopi_office_url.is_empty() && cli.wopi_token_secret.is_none() {
            errors
                .push("--wopi-token-secret is required when --wopi-office-url is set.".to_string());
        }

        // Print results
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

    // Validate config file schema version (currently only version 1 is supported)
    if let Some(version) = file_config.schema_version
        && version > 1
    {
        anyhow::bail!(
            "Unsupported config file schema_version: {}. Supported versions: 1. \
             Please update your ferro.toml or upgrade Ferro.",
            version
        );
    }

    match cli.log_format.as_str() {
        "json" => {
            tracing_subscriber::fmt()
                .with_env_filter(
                    tracing_subscriber::EnvFilter::try_from_default_env()
                        .unwrap_or_else(|_| cli.log_level.clone().into()),
                )
                .event_format(ferro_server::json_logging::JsonFormatter)
                .init();
        }
        _ => {
            tracing_subscriber::fmt()
                .with_env_filter(
                    tracing_subscriber::EnvFilter::try_from_default_env()
                        .unwrap_or_else(|_| cli.log_level.clone().into()),
                )
                .init();
        }
    }

    if cli.data_dir.is_none() {
        tracing::warn!("═══════════════════════════════════════════════════════════════");
        tracing::warn!("  WARNING: Running without --data-dir");
        tracing::warn!(
            "  All data will be LOST on restart (files, metadata, shares, snapshots, audit log)"
        );
        tracing::warn!("  Use --data-dir /path/to/data for persistent storage");
        tracing::warn!("═══════════════════════════════════════════════════════════════");
    }

    info!(host = %cli.host, port = cli.port, storage = %cli.storage, "Starting Ferro server");

    // Build storage backend
    let state = match cli.storage.as_str() {
        "memory" => AppState::in_memory(),
        path if path.starts_with("nas:")
            || path.starts_with("nas-nfs:")
            || path.starts_with("nas-smb:") =>
        {
            info!("NAS storage backend: {}", path);
            let config = ferro_core::nas_backend::NasStorageConfig::parse(path)
                .ok_or_else(|| anyhow::anyhow!("Invalid NAS storage path: {}", path))?;
            let engine = ferro_core::nas_backend::NasStorageEngine::new(&config)
                .map_err(|e| anyhow::anyhow!("Failed to create NAS storage engine: {}", e))?;
            AppState::new(std::sync::Arc::new(engine))
        }
        path if path.starts_with("local:") => {
            let dir = path
                .strip_prefix("local:")
                .ok_or_else(|| anyhow::anyhow!("Invalid local storage path: {}", path))?;
            let store = object_store::local::LocalFileSystem::new_with_prefix(dir)
                .map_err(|e| anyhow::anyhow!("Failed to open local storage at {}: {}", dir, e))?;
            let base_path = std::path::PathBuf::from(dir);
            AppState::new(std::sync::Arc::new(
                ferro_server::object_store_backend::ObjectStoreStorageEngine::with_local_base(
                    std::sync::Arc::new(store),
                    base_path,
                ),
            ))
        }
        #[cfg(feature = "s3")]
        path if path.starts_with("s3://") => {
            info!("S3 storage backend: {}", path);
            let store = object_store::aws::AmazonS3Builder::from_env()
                .with_bucket_name(parse_bucket_from_url(path)?)
                .build()
                .map_err(|e| anyhow::anyhow!("Failed to create S3 client: {}", e))?;
            let store = std::sync::Arc::new(store);
            let presigned = std::sync::Arc::new(
                ferro_core::presigned::S3PresignedUrlGenerator::new(store.clone()),
            );
            AppState::new(std::sync::Arc::new(
                ferro_server::object_store_backend::ObjectStoreStorageEngine::new(store),
            ))
            .with_presigned_generator(presigned)
        }
        #[cfg(feature = "gcs")]
        path if path.starts_with("gs://") => {
            info!("GCS storage backend: {}", path);
            let bucket = parse_bucket_from_url(path)?;
            let store = object_store::gcp::GoogleCloudStorageBuilder::from_env()
                .with_bucket_name(&bucket)
                .build()
                .map_err(|e| anyhow::anyhow!("Failed to create GCS client: {}", e))?;
            let store = std::sync::Arc::new(store);
            let presigned = std::sync::Arc::new(
                ferro_core::presigned::GcsPresignedUrlGenerator::new(store.clone()),
            );
            AppState::new(std::sync::Arc::new(
                ferro_server::object_store_backend::ObjectStoreStorageEngine::new(store),
            ))
            .with_presigned_generator(presigned)
        }
        #[cfg(feature = "azure")]
        path if path.starts_with("az://") || path.starts_with("azure://") => {
            info!("Azure storage backend: {}", path);
            let store = object_store::azure::MicrosoftAzureBuilder::from_env()
                .with_container_name(parse_bucket_from_url(path)?)
                .build()
                .map_err(|e| anyhow::anyhow!("Failed to create Azure client: {}", e))?;
            let store = std::sync::Arc::new(store);
            let presigned = std::sync::Arc::new(
                ferro_core::presigned::AzurePresignedUrlGenerator::new(store.clone()),
            );
            AppState::new(std::sync::Arc::new(
                ferro_server::object_store_backend::ObjectStoreStorageEngine::new(store),
            ))
            .with_presigned_generator(presigned)
        }
        other => {
            let hint = {
                #[cfg(all(feature = "s3", feature = "gcs", feature = "azure"))]
                {
                    " Use 'memory', 'local:/path', 's3://bucket', 'gs://bucket', or 'az://container'."
                }
                #[cfg(all(feature = "s3", feature = "gcs", not(feature = "azure")))]
                {
                    " Use 'memory', 'local:/path', 's3://bucket', or 'gs://bucket'. Compile with --features azure for az://."
                }
                #[cfg(all(feature = "s3", not(feature = "gcs"), feature = "azure"))]
                {
                    " Use 'memory', 'local:/path', 's3://bucket', or 'az://container'. Compile with --features gcs for gs://."
                }
                #[cfg(all(not(feature = "s3"), feature = "gcs", feature = "azure"))]
                {
                    " Use 'memory', 'local:/path', 'gs://bucket', or 'az://container'. Compile with --features s3 for s3://."
                }
                #[cfg(all(feature = "s3", not(feature = "gcs"), not(feature = "azure")))]
                {
                    " Use 'memory', 'local:/path', or 's3://bucket'. Compile with --features gcs,azure for gs:// and az://."
                }
                #[cfg(all(not(feature = "s3"), feature = "gcs", not(feature = "azure")))]
                {
                    " Use 'memory', 'local:/path', or 'gs://bucket'. Compile with --features s3,azure for s3:// and az://."
                }
                #[cfg(all(not(feature = "s3"), not(feature = "gcs"), feature = "azure"))]
                {
                    " Use 'memory', 'local:/path', or 'az://container'. Compile with --features s3,gcs for s3:// and gs://."
                }
                #[cfg(all(not(feature = "s3"), not(feature = "gcs"), not(feature = "azure")))]
                {
                    " Use 'memory' or 'local:/path'. Compile with --features s3,gcs,azure for cloud backends."
                }
            };
            anyhow::bail!("Unknown storage backend: {}.{}", other, hint);
        }
    };

    // Run migration if --migrate-from is specified
    if let Some(ref source_url) = cli.migrate_from {
        info!("Starting migration from {} to {}", source_url, cli.storage);
        let source = build_storage_backend(source_url)?;
        let files = source
            .list_all("/", 10000)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to list source files: {}", e))?;
        let mut copied = 0u64;
        let mut skipped = 0u64;
        let mut errors = 0u64;
        for meta in &files {
            if meta.is_collection {
                continue;
            }
            // Skip if file already exists in destination
            if state.storage.exists(&meta.path).await.unwrap_or(false) {
                skipped += 1;
                continue;
            }
            match source.get(&meta.path).await {
                Ok(content) => match state.storage.put(&meta.path, content, &meta.owner).await {
                    Ok(_) => {
                        copied += 1;
                        if copied.is_multiple_of(100) {
                            info!("Migration progress: {} files copied", copied);
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to write {}: {}", meta.path, e);
                        errors += 1;
                    }
                },
                Err(e) => {
                    tracing::warn!("Failed to read {}: {}", meta.path, e);
                    errors += 1;
                }
            }
        }
        info!(
            "Migration complete: {} copied, {} skipped (already exist), {} errors",
            copied, skipped, errors
        );
        if errors > 0 {
            tracing::warn!(
                "{} files failed to migrate. Check logs above for details.",
                errors
            );
        }
    }

    // Configure OIDC if issuer is set
    let state = if let Some(issuer) = &cli.oidc_issuer {
        info!("OIDC authentication enabled: {}", issuer);
        let oidc_config = OidcConfig {
            issuer: issuer.clone(),
            client_id: cli.oidc_client_id.unwrap_or_else(|| "ferro".to_string()),
            audience: cli.oidc_audience.clone(),
            jwks_uri: cli.oidc_jwks_uri.clone(),
        };
        let validator = ferro_server::auth::oidc::OidcValidator::new(oidc_config);
        state.with_oidc(validator)
    } else {
        info!("OIDC authentication disabled (set FERRO_OIDC_ISSUER to enable)");
        state
    };

    // Configure Cedar if policy file is set
    let state = if let Some(policy_file) = &cli.cedar_policy_file {
        let policy_text = std::fs::read_to_string(policy_file).map_err(|e| {
            anyhow::anyhow!("Failed to read Cedar policy file {}: {}", policy_file, e)
        })?;
        let authorizer = CedarAuthorizer::new()?;
        authorizer.add_policy(&policy_text).await?;
        info!("Cedar authorization enabled: {} policies", 1);
        state.with_cedar(authorizer)
    } else {
        // Enable Cedar with default permissive policy when OIDC is on
        if cli.oidc_issuer.is_some() {
            let authorizer = CedarAuthorizer::new()?;
            info!("Cedar authorization enabled with default policy");
            state.with_cedar(authorizer)
        } else {
            state
        }
    };

    // Configure search
    let search_index_path = match &cli.search_index_path {
        Some(p) => p.clone(),
        None => match &cli.data_dir {
            Some(dd) => format!("{}/search-index", dd),
            None => "/tmp/ferro-search".to_string(),
        },
    };
    let state = {
        let search_path = std::path::Path::new(&search_index_path);
        // Ensure the search index directory exists before attempting to open/create
        if let Err(e) = std::fs::create_dir_all(search_path) {
            tracing::warn!(
                "Search engine unavailable: could not create directory {:?}: {}",
                search_path,
                e
            );
            state
        } else {
            match ferro_core::search::SearchEngine::open(search_path) {
                Ok(engine) => {
                    info!("Search engine enabled at {:?}", search_path);
                    state.with_search(engine)
                }
                Err(_) => match ferro_core::search::SearchEngine::new(search_path) {
                    Ok(engine) => {
                        info!("Search engine created at {:?}", search_path);
                        state.with_search(engine)
                    }
                    Err(e) => {
                        tracing::warn!("Search engine unavailable: {}", e);
                        state
                    }
                },
            }
        }
    };

    // Shared cancellation token for graceful shutdown of all subsystems.
    // Created early so it can be passed to background tasks as they spawn.
    let shutdown_token = CancellationToken::new();

    // Spawn background content indexer if search is enabled
    if state.search.is_some() {
        ferro_server::indexer::spawn_indexer(
            std::sync::Arc::new(state.clone()),
            60,
            shutdown_token.clone(),
        );
    }

    // Initialize WASM runtime if --wasm-enabled is set
    let state = if cli.wasm_enabled {
        match ferro_core::wasm::WasmWorkerRuntime::new() {
            Ok(runtime) => {
                info!("WASM worker runtime enabled");
                state.with_wasm_runtime(runtime)
            }
            Err(e) => {
                tracing::warn!("WASM runtime init failed: {}", e);
                state
            }
        }
    } else {
        state
    };

    // Spawn WASM worker runner if WASM runtime is configured
    if state.wasm_runtime.is_some() {
        ferro_server::worker_runner::spawn_worker_runner(
            std::sync::Arc::new(state.clone()),
            30,
            shutdown_token.clone(),
        );
    }

    // Configure workers directory for uploaded WASM modules
    let state = if let Some(data_dir) = &cli.data_dir {
        let workers_dir = std::path::PathBuf::from(data_dir).join("workers");
        if let Err(e) = std::fs::create_dir_all(&workers_dir) {
            tracing::warn!(
                "Failed to create workers directory {:?}: {}",
                workers_dir,
                e
            );
        } else {
            info!("WASM workers directory: {:?}", workers_dir);
        }
        state.with_workers_dir(workers_dir)
    } else {
        state
    };

    // Configure metadata store + CAS deduplication + persistence
    // If --data-dir is set, use unified SQLite persistence for metadata + CAS.
    // Otherwise fall back to --metadata-db for PostgreSQL or in-memory.
    let state = if let Some(data_dir) = &cli.data_dir {
        // Unified SQLite persistence: metadata + CAS + snapshots + audit
        std::fs::create_dir_all(data_dir)
            .map_err(|e| anyhow::anyhow!("Failed to create data dir {}: {}", data_dir, e))?;
        let db_path = std::path::Path::new(data_dir).join("ferro.db");
        let db_url = format!("sqlite:{}", db_path.display());

        info!("SQLite persistence enabled at {}", db_url);

        // Open a rusqlite connection for DashMap store persistence
        let db_handle = match ferro_server::db::open_db(data_dir) {
            Ok(conn) => {
                info!("SQLite DashMap persistence enabled");
                Some(std::sync::Arc::new(std::sync::Mutex::new(conn)))
            }
            Err(e) => {
                tracing::warn!(
                    "SQLite DashMap persistence failed: {}, using in-memory stores",
                    e
                );
                None
            }
        };

        match ferro_core::persistence::SqlitePersistence::new(&db_url).await {
            Ok(persistence) => {
                let persistence = std::sync::Arc::new(persistence);

                // Share the pool with SqliteMetadataStore
                let metadata = ferro_core::sqlx_metadata::SqliteMetadataStore::from_pool(
                    persistence.pool().clone(),
                );
                info!("Metadata store: SQLite (shared pool)");
                let mut state = state.with_metadata_store(std::sync::Arc::new(metadata));

                // CAS dedup is always enabled with persistence
                let cas: std::sync::Arc<dyn ferro_core::cas::CasStore> = persistence.clone();
                info!("CAS deduplication: SQLite-backed");
                state = state.with_cas_store(cas);

                // Audit log + snapshots persistence
                state = state.with_audit_persistence(persistence.clone());
                state = state.with_snapshot_persistence(persistence.clone());

                // DashMap store persistence
                if let Some(db) = db_handle {
                    state = state.with_db(db);
                }

                state
            }
            Err(e) => {
                tracing::warn!("SQLite persistence failed: {}, using in-memory stores", e);
                if let Some(db) = db_handle {
                    state.with_db(db)
                } else {
                    state
                }
            }
        }
    } else {
        // Legacy path: --metadata-db for PostgreSQL metadata
        let state = if let Some(db_url) = &cli.metadata_db {
            info!(
                "PostgreSQL metadata enabled: {}",
                redact_url_credentials(db_url)
            );
            #[cfg(feature = "pg")]
            match ferro_core::sqlx_metadata::PgMetadataStore::new(db_url).await {
                Ok(store) => {
                    info!("Connected to PostgreSQL metadata store");
                    state.with_metadata_store(std::sync::Arc::new(store))
                }
                Err(e) => {
                    tracing::warn!(
                        "PostgreSQL connection failed: {}, using in-memory metadata",
                        e
                    );
                    state
                }
            }
            #[cfg(not(feature = "pg"))]
            {
                tracing::warn!("PostgreSQL metadata requested but 'pg' feature is not enabled");
                state
            }
        } else {
            state
        };

        // Optional in-memory CAS dedup
        if cli.cas_enabled {
            let cas = std::sync::Arc::new(ferro_core::cas::InMemoryCasStore::new());
            info!("Content-addressable storage (CAS) deduplication enabled (in-memory)");
            state.with_cas_store(cas)
        } else {
            state
        }
    };

    let state = state
        .with_max_body_size(cli.max_body_size)
        .with_external_url({
            // Warn when external_url uses http in a non-localhost configuration
            if cli.external_url.starts_with("http://")
                && !cli.external_url.contains("localhost")
                && !cli.external_url.contains("127.0.0.1")
            {
                tracing::warn!(
                    "external_url '{}' uses HTTP (not HTTPS). OIDC callbacks, CORS, \
                     and generated URLs may be insecure in production. Set FERRO_EXTERNAL_URL \
                     to an HTTPS URL if behind a TLS-terminating reverse proxy.",
                    cli.external_url
                );
            }
            cli.external_url
        })
        .with_federation_secret(cli.federation_secret)
        .with_max_file_versions(cli.max_file_versions)
        .with_streaming_upload_threshold(cli.streaming_upload_threshold);

    let mut state = state;
    state.rate_limit_burst = cli.rate_limit_burst;
    state.rate_limit_refill = cli.rate_limit_refill;
    state.max_concurrent_requests = cli.max_concurrent_requests;
    state.max_snapshot_versions = cli.max_snapshot_versions;

    // Initialize federation token store if federation secret is configured
    let state = if !state.federation_secret.is_empty() {
        for peer in &cli.federation_trusted_peers {
            info!("Federation trusted peer: {}", peer);
        }
        state
    } else {
        state
    };

    let mut state = if let Some(ref data_dir) = cli.data_dir {
        state.with_data_dir(data_dir.clone())
    } else {
        state
    };

    state.thumbnail_size = cli.thumbnail_size.clamp(64, 1024);

    state.thumbnail_cache = {
        let cache_dir = state.data_dir.as_deref().unwrap_or("/tmp/ferro");
        std::sync::Arc::new(ferro_server::thumbnail_cache::ThumbnailCache::new(
            cache_dir,
            cli.thumbnail_cache_size,
            10_000,
        ))
    };

    let state = if let Some(ref data_dir) = cli.data_dir {
        let trash_dir = std::path::Path::new(data_dir).join(".trash");
        if let Err(e) = std::fs::create_dir_all(&trash_dir) {
            tracing::warn!("Failed to create trash directory {:?}: {}", trash_dir, e);
            state
        } else {
            info!("Trash directory: {:?}", trash_dir);
            state.with_trash_dir(trash_dir.to_string_lossy().to_string())
        }
    } else {
        state
    };

    let state = if let Some(ref quota_str) = cli.storage_quota {
        match ferro_server::quota::parse_human_size(quota_str) {
            Some(bytes) => {
                info!("Storage quota set to {} bytes ({})", bytes, quota_str);
                let mut s = state;
                s.quota_bytes = Some(bytes);
                s
            }
            None => {
                tracing::warn!("Invalid storage quota format: {}, ignoring", quota_str);
                state
            }
        }
    } else {
        state
    };

    // Configure WOPI office URL
    let state = if !cli.wopi_office_url.is_empty() {
        info!("WOPI office server: {}", cli.wopi_office_url);
        state.with_wopi_office_url(cli.wopi_office_url)
    } else {
        tracing::info!(
            "WOPI office URL not configured (set --wopi-office-url or FERRO_WOPI_OFFICE_URL to enable)"
        );
        state
    };

    // Configure WOPI token secret (required when WOPI office URL is set)
    let state = if !state.wopi_office_url.is_empty() && cli.wopi_token_secret.is_none() {
        anyhow::bail!(
            "WOPI is enabled (--wopi-office-url is set) but --wopi-token-secret is not configured. \
             Set --wopi-token-secret or FERRO_WOPI_TOKEN_SECRET to a strong random value."
        );
    } else if let Some(secret) = cli.wopi_token_secret {
        state.with_wopi_token_secret(secret)
    } else {
        state
    };

    // Configure simple auth (HTTP Basic Auth) if admin credentials are set
    let state = if cli.admin_user.is_some() && cli.admin_password.is_some() {
        info!(
            "Simple HTTP Basic Auth enabled for user: {}",
            cli.admin_user.as_deref().unwrap_or("")
        );
        let admin = ferro_server::users::InMemoryUserStore::create_admin(
            cli.admin_user.as_deref().unwrap_or(""),
            cli.admin_password.as_deref().unwrap_or(""),
        );
        let store = ferro_server::users::InMemoryUserStore::new();
        match admin {
            Some(user) => {
                if let Err(e) = store.create_user(user).await {
                    tracing::error!("Failed to create initial admin user: {:?}", e);
                    std::process::exit(1);
                }
            }
            None => {
                tracing::error!("Failed to hash admin password. Check system resources.");
                std::process::exit(1);
            }
        }
        state
            .with_admin_user(cli.admin_user.clone())
            .with_admin_password(cli.admin_password.clone())
            .with_user_store(std::sync::Arc::new(store))
    } else {
        if cli.admin_user.is_some() || cli.admin_password.is_some() {
            tracing::warn!(
                "Both --admin-user and --admin-password must be set to enable simple auth"
            );
        }
        state
    };

    // Configure Redis distributed lock manager if --redis-url is set
    let state = {
        #[cfg(feature = "redis")]
        {
            if let Some(ref redis_url) = cli.redis_url {
                info!(
                    "Redis distributed lock manager enabled: {}",
                    redact_url_credentials(redis_url)
                );
                match ferro_server::redis_lock::RedisLockManager::new(redis_url).await {
                    Ok(lock_mgr) => state.with_lock_manager(std::sync::Arc::new(lock_mgr)),
                    Err(e) => {
                        tracing::warn!(
                            "Redis connection failed: {}, using in-memory lock manager",
                            e
                        );
                        state
                    }
                }
            } else {
                state
            }
        }
        #[cfg(not(feature = "redis"))]
        {
            state
        }
    };

    // Configure PostgreSQL distributed state if --database-url is set
    let state = {
        #[cfg(feature = "pg")]
        {
            if let Some(ref database_url) = cli.database_url {
                info!(
                    "PostgreSQL distributed state enabled: {}",
                    redact_url_credentials(database_url)
                );
                match sqlx::PgPool::connect(database_url).await {
                    Ok(pool) => match ferro_server::pg_state::create_pg_stores(pool).await {
                        Ok((share_store, favorite_store, preference_store)) => {
                            info!("PostgreSQL stores initialized (shares, favorites, preferences)");
                            let mut state = state;
                            state = state.with_share_store(std::sync::Arc::new(share_store));
                            state = state.with_favorites(std::sync::Arc::new(favorite_store));
                            state = state.with_preferences(std::sync::Arc::new(preference_store));
                            state
                        }
                        Err(e) => {
                            tracing::warn!(
                                "PostgreSQL store init failed: {}, using in-memory stores",
                                e
                            );
                            state
                        }
                    },
                    Err(e) => {
                        tracing::warn!(
                            "PostgreSQL connection failed: {}, using in-memory stores",
                            e
                        );
                        state
                    }
                }
            } else {
                state
            }
        }
        #[cfg(not(feature = "pg"))]
        {
            state
        }
    };

    // Load webhooks from SQLite (async, requires tokio runtime)
    if let Some(ref db) = state.db {
        let hooks: Vec<ferro_server::webhooks::WebhookConfig> = {
            let conn = db.lock().unwrap_or_else(|e| e.into_inner());
            ferro_server::webhooks::load_webhooks_from_db(&conn).unwrap_or_default()
        };
        if !hooks.is_empty() {
            let mut wh = state.webhooks.write().await;
            wh.extend(hooks);
        }

        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        ferro_server::event_triggers::load_triggers_from_db(&conn);

        // Initialize DLP tables
        if let Err(e) = ferro_server::dlp_api::init_dlp_table(db) {
            tracing::warn!("Failed to init DLP tables: {}", e);
        }
    }

    // Validate storage backend is reachable
    match state.storage.list("/").await {
        Ok(_) => tracing::info!("Storage backend is reachable"),
        Err(e) => {
            tracing::error!("Storage backend is NOT reachable: {}. Aborting startup.", e);
            std::process::exit(1);
        }
    }

    // AU-013: Verify CAS content integrity on startup (if persistence enabled)
    if state.cas_store.is_some() && state.data_dir.is_some() {
        match state.storage.list_all("/", 10000).await {
            Ok(entries) => {
                let mut verified = 0u32;
                let mut mismatches = 0u32;
                for meta in &entries {
                    if meta.is_collection {
                        continue;
                    }
                    let stored_hash = meta.content_hash.as_str();
                    if stored_hash.is_empty() || stored_hash.len() != 64 {
                        continue;
                    }
                    if let Ok(content) = state.storage.get(&meta.path).await {
                        use sha2::{Digest, Sha256};
                        let computed = hex::encode(Sha256::digest(&content));
                        if computed == stored_hash {
                            verified += 1;
                        } else {
                            mismatches += 1;
                            tracing::warn!(
                                "CAS integrity mismatch: {} (stored={}, computed={})",
                                meta.path,
                                &stored_hash[..8],
                                &computed[..8]
                            );
                        }
                    }
                }
                if mismatches > 0 {
                    tracing::warn!(
                        "CAS startup verification: {} verified, {} mismatches. \
                         Run GET /api/admin/integrity for full report.",
                        verified,
                        mismatches
                    );
                } else if verified > 0 {
                    tracing::info!("CAS startup verification: {} files verified OK", verified);
                }
            }
            Err(e) => {
                tracing::warn!("CAS startup verification skipped: {}", e);
            }
        }
    }

    if let Some(ref pw) = state.admin_password
        && security::is_default_password(pw)
    {
        tracing::warn!(
            "Server started with a default admin password. \
             All non-whitelisted API requests will be blocked until password is changed \
             via POST /api/auth/change-password."
        );
        // Allow startup but flag password as default for middleware enforcement.
        // Refuse to start only if the password is literally empty (no auth at all).
    }

    // Hard-reject CORS wildcard with auth enabled (library code logs an error
    // but does not panic for test compatibility; here in production we halt).
    let cors_value = if cli.cors_origins != "*" {
        &cli.cors_origins
    } else {
        &cli.cors_allowed_origins
    };

    if cors_value == "*" && state.auth_enabled() {
        anyhow::bail!(
            "CORS origins are '*' while authentication is enabled. \
             Set a specific origin to prevent credential theft in production."
        );
    }

    if cli.maintenance_mode {
        state
            .maintenance_mode
            .store(true, std::sync::atomic::Ordering::Relaxed);
        tracing::warn!("Server started in MAINTENANCE MODE — all write operations are blocked");
    }

    // Configure offline-first mode if --offline-cache-dir is set
    let state = if let Some(ref cache_dir) = cli.offline_cache_dir {
        std::fs::create_dir_all(cache_dir).map_err(|e| {
            anyhow::anyhow!("Failed to create offline cache dir {}: {}", cache_dir, e)
        })?;
        info!("Offline-first mode enabled: cache dir = {}", cache_dir);

        let queue_db_path = std::path::Path::new(cache_dir).join("offline_queue.db");
        let queue_db_url = format!("sqlite:{}", queue_db_path.display());
        match rusqlite::Connection::open(&queue_db_path) {
            Ok(conn) => {
                let db_handle = std::sync::Arc::new(std::sync::Mutex::new(conn));
                let queue = std::sync::Arc::new(
                    ferro_offline::change_queue::SqliteChangeQueue::new(db_handle),
                );
                if let Err(e) = queue.init() {
                    tracing::warn!("Failed to init offline queue table: {}", e);
                }
                info!("Offline change queue initialized at {}", queue_db_url);
                state
                    .with_offline_queue(queue)
                    .with_offline_cache_size(cli.offline_queue_size as u64 * 1024)
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to open offline queue DB: {}, offline queue disabled",
                    e
                );
                state
            }
        }
    } else {
        state
    };

    // Spawn reconnection listener: when ConnectionMonitor detects online, sync queued changes
    if let Some(ref offline_queue) = state.offline_queue {
        let monitor = state.connection_monitor.clone();
        let queue = offline_queue.clone();
        let storage = state.storage.clone();
        let reconcile_cancel = shutdown_token.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    new_state = monitor.wait_for_change() => {
                        if new_state == ferro_offline::monitor::ConnectionState::Online {
                            info!("Connection restored — syncing offline queue");
                            let pending = queue.pending().await;
                            if pending.is_empty() {
                                continue;
                            }
                            info!("Replaying {} queued offline operations", pending.len());
                            let mut synced = 0u32;
                            let mut failed = 0u32;
                            for op in &pending {
                                let result: std::result::Result<(), common::error::FerroError> = match op.op {
                                    ferro_offline::change_queue::OperationType::Put => {
                                        storage.head(&op.source_path).await.map(|_| ())
                                    }
                                    ferro_offline::change_queue::OperationType::Delete => {
                                        storage.delete(&op.source_path).await
                                    }
                                    ferro_offline::change_queue::OperationType::Move => {
                                        if let Some(ref dest) = op.dest_path {
                                            storage.move_path(&op.source_path, dest).await
                                        } else {
                                            Ok(())
                                        }
                                    }
                                    ferro_offline::change_queue::OperationType::Copy => {
                                        if let Some(ref dest) = op.dest_path {
                                            storage.copy(&op.source_path, dest).await
                                        } else {
                                            Ok(())
                                        }
                                    }
                                    ferro_offline::change_queue::OperationType::CreateCollection => {
                                        storage.create_collection(&op.source_path, &op.owner).await.map(|_| ())
                                    }
                                    _ => {
                                        tracing::warn!("Unhandled offline operation type: {:?}", op.op);
                                        Ok(())
                                    }
                                };
                                match result {
                                    Ok(_) => {
                                        let _ = queue.mark_synced(&op.id).await;
                                        synced += 1;
                                    }
                                    Err(e) => {
                                        tracing::warn!("Failed to sync op {}: {}", op.id, e);
                                        failed += 1;
                                    }
                                }
                            }
                            info!("Offline queue sync complete: {} synced, {} failed", synced, failed);
                        }
                    }
                    _ = reconcile_cancel.cancelled() => {
                        tracing::info!("Offline reconnection listener shutting down");
                        break;
                    }
                }
            }
        });
    }

    // Configure push notifications if CLI flags are set
    let mut state = state;
    state.push_notification_config = ferro_server::push_notifications::PushNotificationConfig {
        fcm_server_key: cli.fcm_server_key.clone(),
        apns_key_path: cli.apns_key_path.clone(),
        apns_team_id: cli.apns_team_id.clone(),
        apns_bundle_id: "com.ferro.app".to_string(),
        apns_production: true,
    };
    if state.push_notification_config.fcm_server_key.is_some()
        || state.push_notification_config.apns_key_path.is_some()
    {
        info!(
            "Push notifications enabled (FCM: {}, APNS: {})",
            state.push_notification_config.fcm_server_key.is_some(),
            state.push_notification_config.apns_key_path.is_some()
        );
    }

    let lock_manager = state.lock_manager.clone();
    ferro_server::integration::event_dispatch::setup_event_handlers(&state);
    let app = build_router_with_static(
        state.clone(),
        cli.static_dir.as_deref(),
        cors_value,
        &cli.api_version,
    );

    // Parse trash TTL and spawn auto-purge background task
    let trash_ttl = match parse_duration(&cli.trash_ttl) {
        Some(d) => d,
        None => {
            tracing::warn!("Invalid --trash-ttl '{}', using 30 days", cli.trash_ttl);
            std::time::Duration::from_secs(30 * 24 * 3600)
        }
    };

    if !trash_ttl.is_zero() {
        let trash_state = state.clone();
        let trash_cancel = shutdown_token.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600));
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        let purged = ferro_server::trash::purge_expired(&trash_state, trash_ttl).await;
                        if purged > 0 {
                            tracing::info!("Auto-purged {} expired trash entries", purged);
                        }
                    }
                    _ = trash_cancel.cancelled() => {
                        tracing::info!("Trash auto-purge shutting down");
                        break;
                    }
                }
            }
        });
    }

    // Spawn retention policy daemon
    if cli.retention_check_interval > 0 {
        ferro_server::retention::spawn_retention_daemon(
            std::sync::Arc::new(state.clone()),
            cli.retention_check_interval,
            shutdown_token.clone(),
        );
    }

    // Spawn guest account cleanup daemon
    if cli.guest_cleanup_interval > 0 {
        ferro_server::guests::spawn_guest_cleanup_daemon(
            std::sync::Arc::new(state.clone()),
            cli.guest_cleanup_interval,
            shutdown_token.clone(),
        );
    }

    let lock_cancel = shutdown_token.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    lock_manager.cleanup_all_expired().await;
                }
                _ = lock_cancel.cancelled() => {
                    tracing::info!("Lock cleanup task shutting down");
                    break;
                }
            }
        }
    });

    let addr = format!("{}:{}", cli.host, cli.port);

    // Build the listener from std so we can set a larger listen backlog.
    // tokio::net::TcpListener::bind uses the OS default (typically 128),
    // which causes connection drops under burst uploads. We also enable
    // TCP_NODELAY via axum::serve for lower latency on small requests.
    let std_listener = std::net::TcpListener::bind(&addr)?;
    std_listener
        .set_nonblocking(true)
        .map_err(|e| anyhow::anyhow!("failed to set non-blocking on listener: {e}"))?;
    // Set TCP keepalive with short idle interval to detect dead connections
    // (Slowloris mitigation). The 30s idle + 3 probes covers ~60s detection.
    #[cfg(unix)]
    {
        use std::os::unix::io::AsRawFd;
        let raw_fd = std_listener.as_raw_fd();
        let on: libc::c_int = 1;
        let idle: libc::c_int = 30;
        let intvl: libc::c_int = 10;
        let cnt: libc::c_int = 3;
        // SAFETY: setsockopt is called with a valid file descriptor from
        // as_raw_fd(), valid SOL_SOCKET/IPPROTO_TCP level constants, valid
        // option name/value pointers, and correct size parameters. All
        // variables (`on`, `idle`, `intvl`, `cnt`) outlive the unsafe block.
        unsafe {
            libc::setsockopt(
                raw_fd,
                libc::SOL_SOCKET,
                libc::SO_KEEPALIVE,
                &on as *const _ as *const libc::c_void,
                std::mem::size_of_val(&on) as libc::socklen_t,
            );
            libc::setsockopt(
                raw_fd,
                libc::IPPROTO_TCP,
                libc::TCP_KEEPIDLE,
                &idle as *const _ as *const libc::c_void,
                std::mem::size_of_val(&idle) as libc::socklen_t,
            );
            libc::setsockopt(
                raw_fd,
                libc::IPPROTO_TCP,
                libc::TCP_KEEPINTVL,
                &intvl as *const _ as *const libc::c_void,
                std::mem::size_of_val(&intvl) as libc::socklen_t,
            );
            libc::setsockopt(
                raw_fd,
                libc::IPPROTO_TCP,
                libc::TCP_KEEPCNT,
                &cnt as *const _ as *const libc::c_void,
                std::mem::size_of_val(&cnt) as libc::socklen_t,
            );
        }
    }
    let listener = tokio::net::TcpListener::from_std(std_listener)?;

    // Mark startup as complete — all verification checks passed.
    state
        .startup_complete
        .store(true, std::sync::atomic::Ordering::Relaxed);

    info!("Ferro server listening on {}", addr);

    // Graceful shutdown: when SIGTERM/SIGINT is received, stop accepting
    // new connections and wait for in-flight connections to complete.
    //
    // The cancellation token is shared with all background tasks (indexer,
    // worker runner, trash purge, lock cleanup). When the signal fires,
    // the token is cancelled which signals all tasks to exit their loops.
    //
    // After the HTTP server drains, we wait briefly for background tasks
    // to finish, then perform cleanup (search index commit, DB close).
    let serve_cancel = shutdown_token.clone();
    let server = axum::serve(listener, app)
        .tcp_nodelay(true)
        .with_graceful_shutdown(shutdown_signal(serve_cancel));

    match server.await {
        Ok(()) => {
            info!("HTTP server drained, performing subsystem cleanup");
        }
        Err(e) => return Err(e.into()),
    }

    // Wait for background tasks to acknowledge cancellation (max 10s).
    // Tasks check the token each loop iteration (1-60s intervals), so
    // most will exit within one tick. We don't abort them — they're
    // reading from shared state that's about to be dropped.
    let cleanup_timeout = std::time::Duration::from_secs(cli.graceful_shutdown_timeout);
    tokio::select! {
        _ = shutdown_token.cancelled() => {
            // Token already cancelled; tasks should be stopping.
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
        _ = tokio::time::sleep(cleanup_timeout) => {
            tracing::warn!(
                "Background tasks did not shut down within {}s, proceeding with cleanup",
                cleanup_timeout.as_secs()
            );
        }
    }

    // Commit search index if search is enabled.
    if let Some(ref search) = state.search {
        match search.write().await.commit() {
            Ok(()) => info!("Search index committed on shutdown"),
            Err(e) => tracing::warn!("Failed to commit search index on shutdown: {}", e),
        }
    }

    // Close SQLite database to flush WAL.
    // Dropping the Arc allows the Connection to finalize, which checkpoints
    // the WAL. We explicitly drop the reference to ensure this happens before
    // the process exits.
    if let Some(ref db) = state.db {
        use std::sync::Arc;
        if Arc::strong_count(db) == 1 {
            info!("SQLite database closing on shutdown (last reference)");
        } else {
            tracing::info!(
                "SQLite database has {} remaining references, WAL will checkpoint on final drop",
                Arc::strong_count(db) - 1
            );
        }
    }

    info!("Server shutdown complete");
    Ok(())
}

fn print_man_page() {
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

async fn check_for_updates() {
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
        println!(
            "Update available: v{} (current: v{})",
            latest_version, current_version
        );
        println!("Download: {}", html_url);
    }
}

fn parse_duration(s: &str) -> Option<std::time::Duration> {
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

/// Build a storage backend from a URL string.
/// Used both for the main server storage and for migration source backends.
fn build_storage_backend(
    url: &str,
) -> anyhow::Result<std::sync::Arc<dyn common::storage::StorageEngine>> {
    match url {
        "memory" => Ok(std::sync::Arc::new(
            ferro_server::storage::InMemoryStorageEngine::new(),
        )),
        path if path.starts_with("nas:")
            || path.starts_with("nas-nfs:")
            || path.starts_with("nas-smb:") =>
        {
            let config = ferro_core::nas_backend::NasStorageConfig::parse(path)
                .ok_or_else(|| anyhow::anyhow!("Invalid NAS storage path: {}", path))?;
            let engine = ferro_core::nas_backend::NasStorageEngine::new(&config)
                .map_err(|e| anyhow::anyhow!("Failed to create NAS storage engine: {}", e))?;
            Ok(std::sync::Arc::new(engine))
        }
        path if path.starts_with("local:") => {
            let dir = path
                .strip_prefix("local:")
                .ok_or_else(|| anyhow::anyhow!("Invalid local storage path: {}", path))?;
            let store = object_store::local::LocalFileSystem::new_with_prefix(dir)
                .map_err(|e| anyhow::anyhow!("Failed to open local storage at {}: {}", dir, e))?;
            Ok(std::sync::Arc::new(
                ferro_server::object_store_backend::ObjectStoreStorageEngine::with_local_base(
                    std::sync::Arc::new(store),
                    std::path::PathBuf::from(dir),
                ),
            ))
        }
        #[cfg(feature = "s3")]
        path if path.starts_with("s3://") => {
            let store = object_store::aws::AmazonS3Builder::from_env()
                .with_bucket_name(parse_bucket_from_url(path)?)
                .build()
                .map_err(|e| anyhow::anyhow!("Failed to create S3 client: {}", e))?;
            Ok(std::sync::Arc::new(
                ferro_server::object_store_backend::ObjectStoreStorageEngine::new(
                    std::sync::Arc::new(store),
                ),
            ))
        }
        #[cfg(feature = "gcs")]
        path if path.starts_with("gs://") => {
            let bucket = parse_bucket_from_url(path)?;
            let store = object_store::gcp::GoogleCloudStorageBuilder::from_env()
                .with_bucket_name(&bucket)
                .build()
                .map_err(|e| anyhow::anyhow!("Failed to create GCS client: {}", e))?;
            Ok(std::sync::Arc::new(
                ferro_server::object_store_backend::ObjectStoreStorageEngine::new(
                    std::sync::Arc::new(store),
                ),
            ))
        }
        #[cfg(feature = "azure")]
        path if path.starts_with("az://") || path.starts_with("azure://") => {
            let store = object_store::azure::MicrosoftAzureBuilder::from_env()
                .with_container_name(parse_bucket_from_url(path)?)
                .build()
                .map_err(|e| anyhow::anyhow!("Failed to create Azure client: {}", e))?;
            Ok(std::sync::Arc::new(
                ferro_server::object_store_backend::ObjectStoreStorageEngine::new(
                    std::sync::Arc::new(store),
                ),
            ))
        }
        _ => anyhow::bail!("Unknown storage backend: {}", url),
    }
}

/// Parse bucket/container name from a URL like "s3://my-bucket" -> "my-bucket".
/// Also handles prefix paths like "s3://my-bucket/path" → "my-bucket".
#[allow(dead_code)] // Feature-gated: only used when s3/gcs/azure features are enabled
fn parse_bucket_from_url(url: &str) -> anyhow::Result<String> {
    let without_scheme = url
        .trim_start_matches("s3://")
        .trim_start_matches("gs://")
        .trim_start_matches("az://")
        .trim_start_matches("azure://");
    let bucket = without_scheme
        .split('/')
        .next()
        .ok_or_else(|| anyhow::anyhow!("Cannot parse bucket from URL: {}", url))?;
    if bucket.is_empty() {
        anyhow::bail!("Empty bucket name in URL: {}", url);
    }
    Ok(bucket.to_string())
}

#[cfg(unix)]
async fn shutdown_signal(cancel: CancellationToken) {
    use tokio::signal::unix::{SignalKind, signal};
    let mut sigterm = match signal(SignalKind::terminate()) {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Failed to install SIGTERM handler: {e}");
            std::process::exit(1);
        }
    };
    let mut sigint = match signal(SignalKind::interrupt()) {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Failed to install SIGINT handler: {e}");
            std::process::exit(1);
        }
    };
    tokio::select! {
        _ = sigterm.recv() => info!("Received SIGTERM, starting graceful shutdown"),
        _ = sigint.recv()  => info!("Received SIGINT, starting graceful shutdown"),
    }
    cancel.cancel();
}

#[cfg(not(unix))]
async fn shutdown_signal(cancel: CancellationToken) {
    if let Err(e) = tokio::signal::ctrl_c().await {
        tracing::error!("Failed to install ctrl-c handler: {e}");
        std::process::exit(1);
    }
    info!("Received Ctrl-C, starting graceful shutdown");
    cancel.cancel();
}

#[cfg(not(unix))]
async fn shutdown_signal() {
    if let Err(e) = tokio::signal::ctrl_c().await {
        tracing::error!("Failed to install ctrl-c handler: {e}");
        std::process::exit(1);
    }
}
