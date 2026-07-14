use clap::Parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut cli = ferro_server::cli::Cli::parse();

    // Early-exit: generate shell completions
    if let Some(shell) = cli.generate_completions {
        ferro_server::cli::generate_completions(shell);
        return Ok(());
    }

    // Early-exit: print man page
    if cli.print_man_page {
        ferro_server::cli::print_man_page();
        return Ok(());
    }

    // Early-exit: check for updates
    if cli.check_update {
        ferro_server::cli::check_for_updates().await;
        return Ok(());
    }

    // Load config file and apply to CLI
    let original_args: Vec<String> = std::env::args().collect();
    let file_config = ferro_server::cli::load_file_config(cli.config.as_deref())?;
    ferro_server::config::apply_file_config(&original_args, &mut cli, &file_config);

    // --validate-config: validate configuration and exit
    if cli.validate_config {
        ferro_server::cli::validate_config(&cli, &file_config)?;
    }

    // Validate config file schema version
    if let Some(version) = file_config.schema_version
        && version > 1
    {
        anyhow::bail!(
            "Unsupported config file schema_version: {}. Supported versions: 1. \
             Please update your ferro.toml or upgrade Ferro.",
            version
        );
    }

    // Initialize logging
    ferro_server::startup::init_logging(&cli);

    // Build state, spawn daemons, and run server
    let state = ferro_server::startup::build_state(&cli).await?;
    let shutdown_token = tokio_util::sync::CancellationToken::new();
    ferro_server::startup::spawn_daemons(&state, &cli, &shutdown_token);
    ferro_server::startup::run_server(state, &cli, shutdown_token).await
}
