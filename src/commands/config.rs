//! Configuration management command

use anyhow::Result;
use clap::{Args, Subcommand};

#[derive(Args, Debug)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub command: ConfigCommands,
}

#[derive(Subcommand, Debug)]
pub enum ConfigCommands {
    /// Show current configuration (after applying all overrides)
    Show,

    /// Validate configuration file
    Validate,

    /// Print example configuration file
    Example,

    /// Show configuration file search paths
    Paths,
}

pub async fn run(args: ConfigArgs) -> Result<()> {
    match args.command {
        ConfigCommands::Show => run_show().await?,
        ConfigCommands::Validate => run_validate().await?,
        ConfigCommands::Example => run_example().await?,
        ConfigCommands::Paths => run_paths().await?,
    }

    Ok(())
}

async fn run_show() -> Result<()> {
    let config = crate::config::Config::load()?;
    config.validate()?;

    println!("Current Configuration:");
    println!("=====================\n");

    let toml_str = toml::to_string_pretty(&config)?;
    println!("{}", toml_str);

    println!("\nConfiguration loaded successfully.");
    println!("Priority: CLI flags > Environment variables > Config file > Defaults");

    Ok(())
}

async fn run_validate() -> Result<()> {
    println!("Validating configuration...\n");

    let paths = crate::config::Config::config_file_paths();
    let mut found = false;

    for path in &paths {
        if path.exists() {
            found = true;
            println!("Found config file: {}", path.display());

            match crate::config::Config::load() {
                Ok(config) => match config.validate() {
                    Ok(_) => {
                        println!("✓ Configuration is valid");
                    }
                    Err(e) => {
                        println!("✗ Configuration validation failed: {}", e);
                        return Err(e);
                    }
                },
                Err(e) => {
                    println!("✗ Failed to load configuration: {}", e);
                    return Err(e);
                }
            }
            break;
        }
    }

    if !found {
        println!("{}", crate::config::Config::config_not_found_help());
        println!();
        println!("Using defaults...");
        let config = crate::config::Config::default();
        config.validate()?;
        println!("✓ Default configuration is valid");
    }

    Ok(())
}

async fn run_example() -> Result<()> {
    println!(
        r#"# Midnight Validator Monitor (MVM) Configuration File
#
# Location priority (first found is used):
#   1. ./mvm.toml (current directory)
#   2. ~/.config/mvm/config.toml (user config)
#   3. /opt/midnight/mvm/config/config.toml (install location)
#
# Override priority: CLI flags > Environment variables > Config file > Defaults
#
# Environment variables: MVM_RPC_URL, MVM_METRICS_URL, MVM_DB_PATH,
#   MVM_KEYSTORE_PATH, MVM_VALIDATOR_LABEL, MVM_BATCH_SIZE, MVM_POLL_INTERVAL,
#   MVM_PID_FILE, MVM_EXPECTED_IP, MVM_NETWORK

[rpc]
# Midnight node JSON-RPC endpoint
url = "http://localhost:9944"
# Prometheus metrics endpoint (for bandwidth, uptime stats)
metrics_url = "http://localhost:9615/metrics"
# Request timeout in milliseconds
timeout_ms = 30000
# Retry settings for transient failures
max_retries = 3
retry_initial_delay_ms = 1000
retry_max_delay_ms = 30000

[database]
# SQLite database path for block and validator data
path = "/opt/midnight/mvm/data/mvm.db"

[validator]
# Path to Substrate keystore directory (optional - for key verification)
# keystore_path = "/opt/midnight/data/chains/testnet02/keystore"
# Display label for your validator (optional)
# label = "my-validator"
# Node display name (defaults to hostname)
# name = "validator-01"

[sync]
# Blocks to fetch per batch during sync
batch_size = 100
# Seconds between polling for new blocks
poll_interval_secs = 6
# Only sync finalized blocks (safer but slightly delayed)
finalized_only = false
# Block number to start sync from (0 = continue from last synced)
start_block = 0

[view]
# TUI refresh interval in milliseconds
refresh_interval_ms = 6000
# Filter external IPs to only show addresses matching this prefix
# Useful when node reports multiple addresses from peer discovery
# expected_ip = "203.0.113.1"

[daemon]
# PID file for daemon mode (optional)
# pid_file = "/opt/midnight/mvm/data/mvm-sync.pid"
# Log file for daemon mode (optional)
# log_file = "/opt/midnight/mvm/data/mvm-sync.log"
# Enable syslog output
enable_syslog = false

[chain]
# Network preset: "preview", "preprod", or "mainnet"
# Determines epoch durations for timing calculations:
#   preview: 2h sidechain epochs, 24h mainchain epochs
#   preprod/mainnet: TBD sidechain, 5d mainchain epochs
network = "preview"
# Override genesis timestamp (milliseconds since Unix epoch)
# Normally auto-calculated from current slot; only set if you know the exact value
# genesis_timestamp_ms = 1700000000000
"#
    );

    Ok(())
}

async fn run_paths() -> Result<()> {
    println!("Configuration File Search Paths:");
    println!("================================\n");

    let paths = crate::config::Config::config_file_paths();

    for (i, path) in paths.iter().enumerate() {
        let exists = if path.exists() { "✓ EXISTS" } else { "  " };
        println!("{}. {} {}", i + 1, path.display(), exists);
    }

    println!("\nConfiguration files are searched in order from top to bottom.");
    println!("The first file found will be used.");

    Ok(())
}
