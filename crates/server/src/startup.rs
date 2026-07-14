use crate::AppState;
use ferro_server_config::ServerConfig as Cli;
use crate::users::UserStoreTrait;
use ferro_offline::change_queue::ChangeQueueStore;
use tokio_util::sync::CancellationToken;
use tracing::info;

pub fn init_logging(cli: &Cli) {
    #[cfg(feature = "otel")]
    {
        use opentelemetry::trace::TracerProvider;
        use opentelemetry_otlp::WithExportConfig;
        use tracing_opentelemetry::OpenTelemetryLayer;
        use tracing_subscriber::layer::SubscriberExt;
        use tracing_subscriber::util::SubscriberInitExt;

        let exporter = opentelemetry_otlp::SpanExporter::builder()
            .with_tonic()
            .with_endpoint(&cli.otlp_endpoint)
            .build()
            .expect("Failed to create OTLP exporter");

        let provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
            .with_batch_exporter(exporter)
            .with_resource(
                opentelemetry_sdk::Resource::builder()
                    .with_service_name(cli.otel_service_name.clone())
                    .build(),
            )
            .build();

        let tracer = provider.tracer("ferro-server");

        // Register as global provider to keep it alive for process lifetime.
        // This prevents the exporter from being dropped and shut down.
        opentelemetry::global::set_tracer_provider(provider);

        let otel_layer = OpenTelemetryLayer::new(tracer);

        let env_filter =
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| cli.log_level.clone().into());

        // Use a simpler approach without mixing formatters
        tracing_subscriber::registry()
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer())
            .with(otel_layer)
            .init();

        info!(
            "OpenTelemetry tracing enabled: endpoint={}, service={}",
            cli.otlp_endpoint, cli.otel_service_name
        );
    }

    #[cfg(not(feature = "otel"))]
    {
        match cli.log_format.as_str() {
            "json" => {
                tracing_subscriber::fmt()
                    .with_env_filter(
                        tracing_subscriber::EnvFilter::try_from_default_env()
                            .unwrap_or_else(|_| cli.log_level.clone().into()),
                    )
                    .event_format(crate::json_logging::JsonFormatter)
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
    }

    if cli.data_dir.is_none() {
        tracing::warn!("═══════════════════════════════════════════════════════════════");
        tracing::warn!("  WARNING: Running without --data-dir");
        tracing::warn!("  All data will be LOST on restart (files, metadata, shares, snapshots, audit log)");
        tracing::warn!("  Use --data-dir /path/to/data for persistent storage");
        tracing::warn!("═══════════════════════════════════════════════════════════════");
    }

    info!(host = %cli.host, port = cli.port, storage = %cli.storage, "Starting Ferro server");
}

pub async fn build_state(cli: &Cli) -> anyhow::Result<AppState> {
    // Build storage backend
    let state = match cli.storage.as_str() {
        "memory" => AppState::in_memory(),
        path if path.starts_with("nas:") || path.starts_with("nas-nfs:") || path.starts_with("nas-smb:") => {
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
                crate::object_store_backend::ObjectStoreStorageEngine::with_local_base(
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
            let presigned = std::sync::Arc::new(ferro_core::presigned::S3PresignedUrlGenerator::new(store.clone()));
            AppState::new(std::sync::Arc::new(
                crate::object_store_backend::ObjectStoreStorageEngine::new(store),
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
            let presigned = std::sync::Arc::new(ferro_core::presigned::GcsPresignedUrlGenerator::new(store.clone()));
            AppState::new(std::sync::Arc::new(
                crate::object_store_backend::ObjectStoreStorageEngine::new(store),
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
            let presigned = std::sync::Arc::new(ferro_core::presigned::AzurePresignedUrlGenerator::new(store.clone()));
            AppState::new(std::sync::Arc::new(
                crate::object_store_backend::ObjectStoreStorageEngine::new(store),
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
            tracing::warn!("{} files failed to migrate. Check logs above for details.", errors);
        }
    }

    // Configure OIDC if issuer is set
    let state = if let Some(issuer) = &cli.oidc_issuer {
        info!("OIDC authentication enabled: {}", issuer);
        let oidc_config = crate::auth::oidc::OidcConfig {
            issuer: issuer.clone(),
            client_id: cli.oidc_client_id.clone().unwrap_or_else(|| "ferro".to_string()),
            audience: cli.oidc_audience.clone(),
            jwks_uri: cli.oidc_jwks_uri.clone(),
        };
        let validator = crate::auth::oidc::OidcValidator::new(oidc_config);
        state.with_oidc(validator)
    } else {
        info!("OIDC authentication disabled (set FERRO_OIDC_ISSUER to enable)");
        state
    };

    // Configure Cedar if policy file is set
    let state = if let Some(policy_file) = &cli.cedar_policy_file {
        let policy_text = std::fs::read_to_string(policy_file)
            .map_err(|e| anyhow::anyhow!("Failed to read Cedar policy file {}: {}", policy_file, e))?;
        let authorizer = crate::auth::cedar::CedarAuthorizer::new()?;
        authorizer.add_policy(&policy_text).await?;
        info!("Cedar authorization enabled: {} policies", 1);
        state.with_cedar(authorizer)
    } else {
        if cli.oidc_issuer.is_some() {
            let authorizer = crate::auth::cedar::CedarAuthorizer::new()?;
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

    // Configure workers directory for uploaded WASM modules
    let state = if let Some(data_dir) = &cli.data_dir {
        let workers_dir = std::path::PathBuf::from(data_dir).join("workers");
        if let Err(e) = std::fs::create_dir_all(&workers_dir) {
            tracing::warn!("Failed to create workers directory {:?}: {}", workers_dir, e);
        } else {
            info!("WASM workers directory: {:?}", workers_dir);
        }
        state.with_workers_dir(workers_dir)
    } else {
        state
    };

    // Configure metadata store + CAS deduplication + persistence
    let state = if let Some(data_dir) = &cli.data_dir {
        std::fs::create_dir_all(data_dir)
            .map_err(|e| anyhow::anyhow!("Failed to create data dir {}: {}", data_dir, e))?;
        let db_path = std::path::Path::new(data_dir).join("ferro.db");
        let db_url = format!("sqlite:{}", db_path.display());

        info!("SQLite persistence enabled at {}", db_url);

        let db_handle = match crate::db::open_db(data_dir) {
            Ok(conn) => {
                info!("SQLite DashMap persistence enabled");
                Some(std::sync::Arc::new(std::sync::Mutex::new(conn)))
            }
            Err(e) => {
                tracing::warn!("SQLite DashMap persistence failed: {}, using in-memory stores", e);
                None
            }
        };

        match ferro_core::persistence::SqlitePersistence::new(&db_url).await {
            Ok(persistence) => {
                let persistence = std::sync::Arc::new(persistence);

                let metadata = ferro_core::sqlx_metadata::SqliteMetadataStore::from_pool(persistence.pool().clone());
                info!("Metadata store: SQLite (shared pool)");
                let mut state = state.with_metadata_store(std::sync::Arc::new(metadata));

                let cas: std::sync::Arc<dyn ferro_core::cas::CasStore> = persistence.clone();
                info!("CAS deduplication: SQLite-backed");

                state = state.with_cas_store(cas);

                state = state.with_audit_persistence(persistence.clone());
                state = state.with_snapshot_persistence(persistence.clone());

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
        let state = if let Some(db_url) = &cli.metadata_db {
            info!(
                "PostgreSQL metadata enabled: {}",
                ferro_server_config::redact_url_credentials(db_url)
            );
            #[cfg(feature = "pg")]
            match ferro_core::sqlx_metadata::PgMetadataStore::new(db_url).await {
                Ok(store) => {
                    info!("Connected to PostgreSQL metadata store");
                    state.with_metadata_store(std::sync::Arc::new(store))
                }
                Err(e) => {
                    tracing::warn!("PostgreSQL connection failed: {}, using in-memory metadata", e);
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
            cli.external_url.clone()
        })
        .with_federation_secret(cli.federation_secret.clone())
        .with_max_file_versions(cli.max_file_versions)
        .with_streaming_upload_threshold(cli.streaming_upload_threshold);

    let mut state = state;
    state.rate_limit_burst = cli.rate_limit_burst;
    state.rate_limit_refill = cli.rate_limit_refill;
    state.max_concurrent_requests = cli.max_concurrent_requests;
    state.max_snapshot_versions = cli.max_snapshot_versions;

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
        std::sync::Arc::new(crate::thumbnail_cache::ThumbnailCache::new(
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
        match crate::quota::parse_human_size(quota_str) {
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
        state.with_wopi_office_url(cli.wopi_office_url.clone())
    } else {
        tracing::info!("WOPI office URL not configured (set --wopi-office-url or FERRO_WOPI_OFFICE_URL to enable)");
        state
    };

    // Configure WOPI token secret
    let state = if !state.wopi_office_url.is_empty() && cli.wopi_token_secret.is_none() {
        anyhow::bail!(
            "WOPI is enabled (--wopi-office-url is set) but --wopi-token-secret is not configured. \
             Set --wopi-token-secret or FERRO_WOPI_TOKEN_SECRET to a strong random value."
        );
    } else if let Some(secret) = cli.wopi_token_secret.clone() {
        state.with_wopi_token_secret(secret)
    } else {
        state
    };

    // Configure simple auth (HTTP Basic Auth)
    let state = if cli.admin_user.is_some() && cli.admin_password.is_some() {
        info!(
            "Simple HTTP Basic Auth enabled for user: {}",
            cli.admin_user.as_deref().unwrap_or("")
        );
        let admin = crate::users::InMemoryUserStore::create_admin(
            cli.admin_user.as_deref().unwrap_or(""),
            cli.admin_password.as_deref().unwrap_or(""),
        );
        let store = crate::users::InMemoryUserStore::new();
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
            tracing::warn!("Both --admin-user and --admin-password must be set to enable simple auth");
        }
        state
    };

    // Configure Redis distributed lock manager
    let state = {
        #[cfg(feature = "redis")]
        {
            if let Some(ref redis_url) = cli.redis_url {
                info!(
                    "Redis distributed lock manager enabled: {}",
                    ferro_server_config::redact_url_credentials(redis_url)
                );
                match crate::redis_lock::RedisLockManager::new(redis_url).await {
                    Ok(lock_mgr) => state.with_lock_manager(std::sync::Arc::new(lock_mgr)),
                    Err(e) => {
                        tracing::warn!("Redis connection failed: {}, using in-memory lock manager", e);
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

    // Configure PostgreSQL distributed state
    let state = {
        #[cfg(feature = "pg")]
        {
            if let Some(ref database_url) = cli.database_url {
                info!(
                    "PostgreSQL distributed state enabled: {}",
                    ferro_server_config::redact_url_credentials(database_url)
                );
                match sqlx::PgPool::connect(database_url).await {
                    Ok(pool) => match crate::pg_state::create_pg_stores(pool).await {
                        Ok((share_store, favorite_store, preference_store)) => {
                            info!("PostgreSQL stores initialized (shares, favorites, preferences)");
                            let mut state = state;
                            state = state.with_share_store(std::sync::Arc::new(share_store));
                            state = state.with_favorites(std::sync::Arc::new(favorite_store));
                            state = state.with_preferences(std::sync::Arc::new(preference_store));
                            state
                        }
                        Err(e) => {
                            tracing::warn!("PostgreSQL store init failed: {}, using in-memory stores", e);
                            state
                        }
                    },
                    Err(e) => {
                        tracing::warn!("PostgreSQL connection failed: {}, using in-memory stores", e);
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

    // Load webhooks from SQLite
    if let Some(ref db) = state.db {
        let hooks: Vec<crate::webhooks::WebhookConfig> = {
            let conn = db.lock().unwrap_or_else(|e| e.into_inner());
            crate::webhooks::load_webhooks_from_db(&conn).unwrap_or_default()
        };
        if !hooks.is_empty() {
            let mut wh = state.webhooks.write().await;
            wh.extend(hooks);
        }

        {
            let conn = db.lock().unwrap_or_else(|e| e.into_inner());
            crate::event_triggers::load_triggers_from_db(&conn);
        }

        {
            let store = crate::dlp_api::DlpStore::new().with_db(db.clone());
            if let Err(e) = store.init_tables() {
                tracing::warn!("Failed to init DLP tables: {}", e);
            }
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

    // CAS content integrity verification on startup
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
        && crate::security::is_default_password(pw)
    {
        tracing::warn!(
            "Server started with a default admin password. \
             All non-whitelisted API requests will be blocked until password is changed \
             via POST /api/auth/change-password."
        );
    }

    // CORS wildcard + auth check
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
        state.maintenance_mode.store(true, std::sync::atomic::Ordering::Relaxed);
        tracing::warn!("Server started in MAINTENANCE MODE — all write operations are blocked");
    }

    // Configure offline-first mode
    let state = if let Some(ref cache_dir) = cli.offline_cache_dir {
        std::fs::create_dir_all(cache_dir)
            .map_err(|e| anyhow::anyhow!("Failed to create offline cache dir {}: {}", cache_dir, e))?;
        info!("Offline-first mode enabled: cache dir = {}", cache_dir);

        let queue_db_path = std::path::Path::new(cache_dir).join("offline_queue.db");
        let queue_db_url = format!("sqlite:{}", queue_db_path.display());
        match rusqlite::Connection::open(&queue_db_path) {
            Ok(conn) => {
                let db_handle = std::sync::Arc::new(std::sync::Mutex::new(conn));
                let queue = std::sync::Arc::new(ferro_offline::change_queue::SqliteChangeQueue::new(db_handle));
                if let Err(e) = queue.init() {
                    tracing::warn!("Failed to init offline queue table: {}", e);
                }
                info!("Offline change queue initialized at {}", queue_db_url);
                state
                    .with_offline_queue(queue)
                    .with_offline_cache_size(cli.offline_queue_size as u64 * 1024)
            }
            Err(e) => {
                tracing::warn!("Failed to open offline queue DB: {}, offline queue disabled", e);
                state
            }
        }
    } else {
        state
    };

    // Configure push notifications
    let mut state = state;
    state.push_notification_config = crate::push_notifications::PushNotificationConfig {
        fcm_server_key: cli.fcm_server_key.clone(),
        apns_key_path: cli.apns_key_path.clone(),
        apns_team_id: cli.apns_team_id.clone(),
        apns_bundle_id: "com.ferro.app".to_string(),
        apns_production: true,
    };
    if state.push_notification_config.fcm_server_key.is_some() || state.push_notification_config.apns_key_path.is_some()
    {
        info!(
            "Push notifications enabled (FCM: {}, APNS: {})",
            state.push_notification_config.fcm_server_key.is_some(),
            state.push_notification_config.apns_key_path.is_some()
        );
    }

    // Register deep health probes
    {
        let checker = &state.health_checker;

        // Register storage probe
        let storage_probe = ferro_health::StorageProbe::new(state.storage.clone()).with_timeout_ms(2000);
        let _ = checker.register(Box::new(storage_probe));
        info!("Health probe registered: storage");

        // Register SQLite probe if database is configured
        if let Some(ref db) = state.db {
            let sqlite_probe = ferro_health::SqliteProbe::new(db.clone()).with_timeout_ms(2000);
            let _ = checker.register(Box::new(sqlite_probe));
            info!("Health probe registered: database (SQLite)");
        }

        // Register Redis probe if Redis feature is enabled and configured
        #[cfg(feature = "redis")]
        {
            if let Some(ref redis_url) = cli.redis_url {
                let redis_probe = ferro_health::RedisProbe::new(redis_url).with_timeout_ms(2000);
                let _ = checker.register(Box::new(redis_probe));
                info!("Health probe registered: redis");
            }
        }
    }

    Ok(state)
}

pub fn spawn_daemons(state: &AppState, cli: &Cli, shutdown_token: &CancellationToken) {
    // Spawn background content indexer if search is enabled
    if state.search.is_some() {
        crate::indexer::spawn_indexer(std::sync::Arc::new(state.clone()), 60, shutdown_token.clone());
    }

    // Spawn WASM worker runner if WASM runtime is configured
    if state.wasm_runtime.is_some() {
        crate::worker_runner::spawn_worker_runner(std::sync::Arc::new(state.clone()), 30, shutdown_token.clone());
    }

    // Spawn reconnection listener for offline queue sync
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
                                        if let Some(dest) = &op.dest_path {
                                            storage.move_path(&op.source_path, dest).await
                                        } else {
                                            Ok(())
                                        }
                                    }
                                    ferro_offline::change_queue::OperationType::Copy => {
                                        if let Some(dest) = &op.dest_path {
                                            storage.copy(&op.source_path, dest).await
                                        } else {
                                            Ok(())
                                        }
                                    }
                                    ferro_offline::change_queue::OperationType::CreateCollection => {
                                        storage.create_collection(&op.source_path, &op.owner).await.map(|_| ())
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

    // Parse trash TTL and spawn auto-purge background task
    let trash_ttl = match crate::cli::parse_duration(&cli.trash_ttl) {
        Some(d) => d,
        None => {
            tracing::warn!("Invalid trash TTL, using 30 days");
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
                        let purged = crate::trash::purge_expired(&trash_state, trash_ttl).await;
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
        crate::retention::spawn_retention_daemon(
            std::sync::Arc::new(state.clone()),
            cli.retention_check_interval,
            shutdown_token.clone(),
        );
    }

    // Spawn guest account cleanup daemon
    if cli.guest_cleanup_interval > 0 {
        crate::guests::spawn_guest_cleanup_daemon(
            std::sync::Arc::new(state.clone()),
            cli.guest_cleanup_interval,
            shutdown_token.clone(),
        );
    }

    // Spawn lock cleanup daemon
    let lock_manager = state.lock_manager.clone();
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
}

pub async fn run_server(state: AppState, cli: &Cli, shutdown_token: CancellationToken) -> anyhow::Result<()> {
    crate::integration::event_dispatch::setup_event_handlers(&state);

    let cors_value = if cli.cors_origins != "*" {
        &cli.cors_origins
    } else {
        &cli.cors_allowed_origins
    };

    let app = crate::build_router_with_static(state.clone(), cli.static_dir.as_deref(), cors_value, &cli.api_version);

    let addr = format!("{}:{}", cli.host, cli.port);

    let std_listener = std::net::TcpListener::bind(&addr)?;
    std_listener
        .set_nonblocking(true)
        .map_err(|e| anyhow::anyhow!("failed to set non-blocking on listener: {e}"))?;

    #[cfg(unix)]
    {
        use std::os::unix::io::AsRawFd;
        let raw_fd = std_listener.as_raw_fd();
        let on: libc::c_int = 1;
        let idle: libc::c_int = 30;
        let intvl: libc::c_int = 10;
        let cnt: libc::c_int = 3;
        // SAFETY: `raw_fd` is a valid file descriptor from the accepted socket.
        // `on`, `idle`, `intvl`, `cnt` are valid `c_int` values passed by pointer
        // to the kernel; `size_of_val` correctly reports their sizes. `setsockopt`
        // returns 0 on success or -1 on error; we ignore the return value here
        // because TCP keepalive is a best-effort optimization, not a correctness
        // requirement.
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

    state.startup_complete.store(true, std::sync::atomic::Ordering::Relaxed);

    info!("Ferro server listening on {}", addr);

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

    let cleanup_timeout = std::time::Duration::from_secs(cli.graceful_shutdown_timeout);
    tokio::select! {
        _ = shutdown_token.cancelled() => {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
        _ = tokio::time::sleep(cleanup_timeout) => {
            tracing::warn!(
                "Background tasks did not shut down within {}s, proceeding with cleanup",
                cleanup_timeout.as_secs()
            );
        }
    }

    // Commit search index if search is enabled
    if let Some(ref search) = state.search {
        match search.write().await.commit() {
            Ok(()) => info!("Search index committed on shutdown"),
            Err(e) => tracing::warn!("Failed to commit search index on shutdown: {}", e),
        }
    }

    // Close SQLite database to flush WAL
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

fn build_storage_backend(url: &str) -> anyhow::Result<std::sync::Arc<dyn common::storage::StorageEngine>> {
    match url {
        "memory" => Ok(std::sync::Arc::new(crate::storage::InMemoryStorageEngine::new())),
        path if path.starts_with("nas:") || path.starts_with("nas-nfs:") || path.starts_with("nas-smb:") => {
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
                crate::object_store_backend::ObjectStoreStorageEngine::with_local_base(
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
                crate::object_store_backend::ObjectStoreStorageEngine::new(std::sync::Arc::new(store)),
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
                crate::object_store_backend::ObjectStoreStorageEngine::new(std::sync::Arc::new(store)),
            ))
        }
        #[cfg(feature = "azure")]
        path if path.starts_with("az://") || path.starts_with("azure://") => {
            let store = object_store::azure::MicrosoftAzureBuilder::from_env()
                .with_container_name(parse_bucket_from_url(path)?)
                .build()
                .map_err(|e| anyhow::anyhow!("Failed to create Azure client: {}", e))?;
            Ok(std::sync::Arc::new(
                crate::object_store_backend::ObjectStoreStorageEngine::new(std::sync::Arc::new(store)),
            ))
        }
        _ => anyhow::bail!("Unknown storage backend: {}", url),
    }
}

#[allow(dead_code)]
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
