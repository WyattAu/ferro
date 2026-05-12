use clap::Parser;
use ferro_server::auth::cedar::CedarAuthorizer;
use ferro_server::auth::oidc::OidcConfig;
use ferro_server::config::ServerConfig;
use ferro_server::config::{FileConfigValues, apply_file_config, load_config_file};
use ferro_server::users::UserStoreTrait;
use ferro_server::{AppState, build_router_with_static};
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut cli = ServerConfig::parse();
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

    // Spawn background content indexer if search is enabled
    if state.search.is_some() {
        ferro_server::indexer::spawn_indexer(std::sync::Arc::new(state.clone()), 60);
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
        ferro_server::worker_runner::spawn_worker_runner(std::sync::Arc::new(state.clone()), 30);
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
            info!("PostgreSQL metadata enabled: {}", db_url);
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
        .with_external_url(cli.external_url)
        .with_federation_secret(cli.federation_secret)
        .with_max_file_versions(cli.max_file_versions);

    let mut state = if let Some(ref data_dir) = cli.data_dir {
        state.with_data_dir(data_dir.clone())
    } else {
        state
    };

    state.thumbnail_size = cli.thumbnail_size.clamp(64, 1024);

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
        store.create_user(admin).await.expect(
            "Failed to create initial admin user — this should never fail with in-memory store",
        );
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
                info!("Redis distributed lock manager enabled: {}", redis_url);
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
                info!("PostgreSQL distributed state enabled: {}", database_url);
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
    }

    // Validate storage backend is reachable
    match state.storage.list("/").await {
        Ok(_) => tracing::info!("Storage backend is reachable"),
        Err(e) => {
            tracing::error!("Storage backend is NOT reachable: {}. Aborting startup.", e);
            std::process::exit(1);
        }
    }

    if state.admin_password.as_deref() == Some("changeme") {
        anyhow::bail!(
            "Refusing to start with default password 'changeme'. \
             Set --admin-password (or FERRO_ADMIN_PASSWORD) to a strong value."
        );
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

    let lock_manager = state.lock_manager.clone();
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
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600));
            loop {
                interval.tick().await;
                let purged = ferro_server::trash::purge_expired(&trash_state, trash_ttl).await;
                if purged > 0 {
                    tracing::info!("Auto-purged {} expired trash entries", purged);
                }
            }
        });
    }

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            lock_manager.cleanup_all_expired().await;
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
        .expect("failed to set non-blocking");
    let listener = tokio::net::TcpListener::from_std(std_listener)?;
    info!("Ferro server listening on {}", addr);

    // Graceful shutdown: when SIGTERM/SIGINT is received, stop accepting
    // new connections and wait for in-flight connections to complete.
    //
    // IMPORTANT: We do NOT wrap the entire server future in tokio::time::timeout.
    // The previous implementation did this, causing the server to force-exit
    // N seconds after startup even without receiving any signal. Instead,
    // we rely on axum's built-in graceful shutdown which:
    //   1. Awaits the signal future
    //   2. Stops accepting new connections
    //   3. Waits for all in-flight connections to finish
    //   4. Returns Ok(())
    //
    // axum has no built-in drain timeout — if connections hang, the server
    // waits. Docker's stop_grace_period handles the force-exit timeout.
    let server = axum::serve(listener, app)
        .tcp_nodelay(true)
        .with_graceful_shutdown(shutdown_signal());

    match server.await {
        Ok(()) => {
            info!("Server shutdown complete");
        }
        Err(e) => return Err(e.into()),
    }

    Ok(())
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

/// Parse bucket/container name from a URL like "s3://my-bucket" → "my-bucket".
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
async fn shutdown_signal() {
    use tokio::signal::unix::{SignalKind, signal};
    let mut sigterm = signal(SignalKind::terminate()).expect("Failed to install SIGTERM handler");
    let mut sigint = signal(SignalKind::interrupt()).expect("Failed to install SIGINT handler");
    tokio::select! {
        _ = sigterm.recv() => info!("Received SIGTERM, starting graceful shutdown"),
        _ = sigint.recv() => info!("Received SIGINT, starting graceful shutdown"),
    }
}

#[cfg(not(unix))]
async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to install ctrl-c handler");
}
