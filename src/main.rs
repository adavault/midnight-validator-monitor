mod alerts;
mod commands;
mod config;
mod daemon;
mod db;
mod metrics;
mod midnight;
mod rpc;
mod tui;

use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
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

    /// Interactive TUI for real-time monitoring
    View(commands::ViewArgs),

    /// Manage configuration
    Config(commands::ConfigArgs),

    /// Install MVM as a system service
    Install(commands::InstallArgs),

    /// Troubleshooting guides and documentation
    Guide(commands::GuideArgs),

    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Skip console logging for TUI and completions (completions must output clean shell script)
    let skip_logging = matches!(
        cli.command,
        Some(Commands::View(_)) | Some(Commands::Completions { .. })
    );

    // Initialize logging (skip for TUI and completions)
    if !skip_logging {
        let log_level = if cli.verbose {
            Level::DEBUG
        } else {
            Level::INFO
        };
        let subscriber = FmtSubscriber::builder().with_max_level(log_level).finish();
        tracing::subscriber::set_global_default(subscriber)?;

        info!(
            "Starting Midnight Validator Monitor v{} (schema v{})",
            env!("CARGO_PKG_VERSION"),
            crate::db::CURRENT_SCHEMA_VERSION
        );
    }

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
        Some(Commands::View(args)) => {
            commands::view::run(args).await?;
        }
        Some(Commands::Config(args)) => {
            commands::config::run(args).await?;
        }
        Some(Commands::Install(args)) => {
            commands::install::run(args).await?;
        }
        Some(Commands::Guide(args)) => {
            commands::guide::run(args).await?;
        }
        Some(Commands::Completions { shell }) => {
            let mut cmd = Cli::command();
            generate(shell, &mut cmd, "mvm", &mut std::io::stdout());
            return Ok(());
        }
        None => {
            // Default behavior: run status command with defaults
            // This maintains backward compatibility
            let args = commands::StatusArgs {
                rpc_url: None,
                metrics_url: None,
                keys_file: None,
                keystore: None,
                interval: None,
                once: false,
                explain: false,
            };
            commands::status::run(args).await?;
        }
    }

    Ok(())
}
