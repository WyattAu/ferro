use clap::Parser;
use ferro_server::{AppState, build_router_with_static};
use ferro_server::config::ServerConfig;
use ferro_server::config::{load_config_file, apply_file_config, FileConfig};
use ferro_server::auth::oidc::OidcConfig;
use ferro_server::auth::cedar::CedarAuthorizer;
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
        FileConfig::default()
    };

    apply_file_config(&original_args, &mut cli, &file_config);

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| cli.log_level.clone().into()),
        )
        .init();

    if cli.data_dir.is_none() {
        tracing::warn!("═══════════════════════════════════════════════════════════════");
        tracing::warn!("  WARNING: Running without --data-dir");
        tracing::warn!("  All data will be LOST on restart (files, metadata, shares, snapshots, audit log)");
        tracing::warn!("  Use --data-dir /path/to/data for persistent storage");
        tracing::warn!("═══════════════════════════════════════════════════════════════");
    }

    info!(host = %cli.host, port = cli.port, storage = %cli.storage, "Starting Ferro server");

    // Build storage backend
    let state = match cli.storage.as_str() {
        "memory" => AppState::in_memory(),
        path if path.starts_with("local:") => {
            let dir = path.strip_prefix("local:")
                .ok_or_else(|| anyhow::anyhow!("Invalid local storage path: {}", path))?;
            let store = object_store::local::LocalFileSystem::new_with_prefix(dir)
                .map_err(|e| anyhow::anyhow!("Failed to open local storage at {}: {}", dir, e))?;
            AppState::new(std::sync::Arc::new(ferro_server::object_store_backend::ObjectStoreStorageEngine::new(
                std::sync::Arc::new(store),
            )))
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
            AppState::new(std::sync::Arc::new(ferro_server::object_store_backend::ObjectStoreStorageEngine::new(
                store,
            ))).with_presigned_generator(presigned)
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
            AppState::new(std::sync::Arc::new(ferro_server::object_store_backend::ObjectStoreStorageEngine::new(
                store,
            ))).with_presigned_generator(presigned)
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
            AppState::new(std::sync::Arc::new(ferro_server::object_store_backend::ObjectStoreStorageEngine::new(
                store,
            ))).with_presigned_generator(presigned)
        }
        other => {
            let hint = {
                #[cfg(all(feature = "s3", feature = "gcs", feature = "azure"))]
                { " Use 'memory', 'local:/path', 's3://bucket', 'gs://bucket', or 'az://container'." }
                #[cfg(all(feature = "s3", feature = "gcs", not(feature = "azure")))]
                { " Use 'memory', 'local:/path', 's3://bucket', or 'gs://bucket'. Compile with --features azure for az://." }
                #[cfg(all(feature = "s3", not(feature = "gcs"), feature = "azure"))]
                { " Use 'memory', 'local:/path', 's3://bucket', or 'az://container'. Compile with --features gcs for gs://." }
                #[cfg(all(not(feature = "s3"), feature = "gcs", feature = "azure"))]
                { " Use 'memory', 'local:/path', 'gs://bucket', or 'az://container'. Compile with --features s3 for s3://." }
                #[cfg(all(feature = "s3", not(feature = "gcs"), not(feature = "azure")))]
                { " Use 'memory', 'local:/path', or 's3://bucket'. Compile with --features gcs,azure for gs:// and az://." }
                #[cfg(all(not(feature = "s3"), feature = "gcs", not(feature = "azure")))]
                { " Use 'memory', 'local:/path', or 'gs://bucket'. Compile with --features s3,azure for s3:// and az://." }
                #[cfg(all(not(feature = "s3"), not(feature = "gcs"), feature = "azure"))]
                { " Use 'memory', 'local:/path', or 'az://container'. Compile with --features s3,gcs for s3:// and gs://." }
                #[cfg(all(not(feature = "s3"), not(feature = "gcs"), not(feature = "azure")))]
                { " Use 'memory' or 'local:/path'. Compile with --features s3,gcs,azure for cloud backends." }
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
        let policy_text = std::fs::read_to_string(policy_file)
            .map_err(|e| anyhow::anyhow!("Failed to read Cedar policy file {}: {}", policy_file, e))?;
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
    let state = {
        let search_path = std::path::Path::new(&cli.search_index_path);
        match ferro_core::search::SearchEngine::open(search_path) {
            Ok(engine) => {
                info!("Search engine enabled at {:?}", search_path);
                state.with_search(engine)
            }
            Err(_) => {
                match ferro_core::search::SearchEngine::new(search_path) {
                    Ok(engine) => {
                        info!("Search engine created at {:?}", search_path);
                        state.with_search(engine)
                    }
                    Err(e) => {
                        tracing::warn!("Search engine unavailable: {}", e);
                        state
                    }
                }
            }
        }
    };

    // Spawn background content indexer if search is enabled
    if state.search.is_some() {
        ferro_server::indexer::spawn_indexer(
            std::sync::Arc::new(state.clone()),
            60,
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
        );
    }

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
    // If --data-dir is set, use unified SQLite persistence for metadata + CAS.
    // Otherwise fall back to --metadata-db for PostgreSQL or in-memory.
    let state = if let Some(data_dir) = &cli.data_dir {
        // Unified SQLite persistence: metadata + CAS + snapshots + audit
        std::fs::create_dir_all(data_dir)
            .map_err(|e| anyhow::anyhow!("Failed to create data dir {}: {}", data_dir, e))?;
        let db_path = std::path::Path::new(data_dir).join("ferro.db");
        let db_url = format!("sqlite:{}", db_path.display());

        info!("SQLite persistence enabled at {}", db_url);

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

                state
            }
            Err(e) => {
                tracing::warn!("SQLite persistence failed: {}, using in-memory stores", e);
                state
            }
        }
    } else {
        // Legacy path: --metadata-db for PostgreSQL metadata
        let state = if let Some(db_url) = &cli.metadata_db {
            info!("PostgreSQL metadata enabled: {}", db_url);
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

    let state = state.with_max_body_size(cli.max_body_size).with_external_url(cli.external_url);

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
        tracing::info!("WOPI office URL not configured (set --wopi-office-url or FERRO_WOPI_OFFICE_URL to enable)");
        state
    };

    // Configure WOPI token secret
    let state = if cli.wopi_token_secret == "ferro-wopi-token-secret-change-me" {
        tracing::warn!("Using default WOPI token secret. Set --wopi-token-secret or FERRO_WOPI_TOKEN_SECRET for production.");
        state
    } else {
        state.with_wopi_token_secret(cli.wopi_token_secret.clone())
    };

    // Configure simple auth (HTTP Basic Auth) if admin credentials are set
    let state = if cli.admin_user.is_some() && cli.admin_password.is_some() {
        info!("Simple HTTP Basic Auth enabled for user: {}", cli.admin_user.as_deref().unwrap_or(""));
        state
            .with_admin_user(cli.admin_user.clone())
            .with_admin_password(cli.admin_password.clone())
    } else {
        if cli.admin_user.is_some() || cli.admin_password.is_some() {
            tracing::warn!("Both --admin-user and --admin-password must be set to enable simple auth");
        }
        state
    };

    let app = build_router_with_static(state, cli.static_dir.as_deref());

    let addr = format!("{}:{}", cli.host, cli.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("Ferro server listening on {}", addr);

    let shutdown_signal = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install CTRL+C handler");
        info!("Received CTRL+C, starting graceful shutdown...");
    };

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal)
        .await?;

    info!("Server shutdown complete");
    Ok(())
}

/// Parse bucket/container name from a URL like "s3://my-bucket" → "my-bucket".
/// Also handles prefix paths like "s3://my-bucket/path" → "my-bucket".
#[allow(dead_code)]
fn parse_bucket_from_url(url: &str) -> anyhow::Result<String> {
    let without_scheme = url
        .trim_start_matches("s3://")
        .trim_start_matches("gs://")
        .trim_start_matches("az://")
        .trim_start_matches("azure://");
    let bucket = without_scheme.split('/').next()
        .ok_or_else(|| anyhow::anyhow!("Cannot parse bucket from URL: {}", url))?;
    if bucket.is_empty() {
        anyhow::bail!("Empty bucket name in URL: {}", url);
    }
    Ok(bucket.to_string())
}
