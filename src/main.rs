mod commands;
mod db;
mod metrics;
mod midnight;
mod rpc;

use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

/// Midnight Validator Monitor - Monitor and manage Midnight blockchain validators
#[derive(Parser, Debug)]
#[command(name = "mvm")]
#[command(about = "A monitoring tool for Midnight blockchain validators")]
#[command(version)]
struct Cli {
    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Display current validator node status
    Status(commands::StatusArgs),

    /// Synchronize blocks to local database
    Sync(commands::SyncArgs),

    /// Query stored block data
    Query(commands::QueryArgs),

    /// Verify and manage session keys
    Keys(commands::KeysArgs),
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let log_level = if cli.verbose {
        Level::DEBUG
    } else {
        Level::INFO
    };
    let subscriber = FmtSubscriber::builder()
        .with_max_level(log_level)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting Midnight Validator Monitor");

    // Handle commands - default to status if no command given
    match cli.command {
        Some(Commands::Status(args)) => {
            commands::status::run(args).await?;
        }
        Some(Commands::Sync(args)) => {
            commands::sync::run(args).await?;
        }
        Some(Commands::Query(args)) => {
            commands::query::run(args).await?;
        }
        Some(Commands::Keys(args)) => {
            commands::keys::run(args).await?;
        }
        None => {
            // Default behavior: run status command with defaults
            // This maintains backward compatibility
            let args = commands::StatusArgs {
                rpc_url: "http://localhost:9944".to_string(),
                metrics_url: "http://localhost:9615/metrics".to_string(),
                keys_file: None,
                keystore: None,
                interval: 60,
                once: false,
            };
            commands::status::run(args).await?;
        }
    }

    Ok(())
}
