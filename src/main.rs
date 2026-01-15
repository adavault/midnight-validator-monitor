mod keys;
mod metrics;
mod monitor;
mod rpc;
mod types;

use anyhow::Result;
use clap::Parser;
use keys::ValidatorKeys;
use monitor::ValidatorMonitor;
use std::path::PathBuf;
use std::time::Duration;
use tokio::time;
use tracing::{error, info, warn, Level};
use tracing_subscriber::FmtSubscriber;

/// Midnight Validator Monitor - Monitor validator status and performance
#[derive(Parser, Debug)]
#[command(name = "midnight-validator-monitor")]
#[command(about = "A monitoring tool for Midnight blockchain validators")]
struct Args {
    /// Validator node RPC endpoint URL
    #[arg(short, long, default_value = "http://localhost:9944")]
    rpc_url: String,

    /// Prometheus metrics endpoint URL
    #[arg(short = 'M', long, default_value = "http://localhost:9615/metrics")]
    metrics_url: String,

    /// Path to validator keys JSON file (for key status monitoring)
    #[arg(short, long, conflicts_with = "keystore")]
    keys_file: Option<PathBuf>,

    /// Path to Substrate keystore directory (auto-detect keys)
    #[arg(short = 'K', long, conflicts_with = "keys_file")]
    keystore: Option<PathBuf>,

    /// Monitoring interval in seconds
    #[arg(short, long, default_value_t = 60)]
    interval: u64,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Run once and exit (don't loop)
    #[arg(long)]
    once: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    let log_level = if args.verbose {
        Level::DEBUG
    } else {
        Level::INFO
    };
    let subscriber = FmtSubscriber::builder()
        .with_max_level(log_level)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting Midnight Validator Monitor");
    info!("RPC endpoint: {}", args.rpc_url);
    info!("Metrics endpoint: {}", args.metrics_url);

    // Load validator keys if specified
    let keys = if let Some(ref keys_path) = args.keys_file {
        match ValidatorKeys::from_file(keys_path) {
            Ok(k) => {
                info!("Loaded validator keys from {}", keys_path.display());
                Some(k)
            }
            Err(e) => {
                error!("Failed to load keys file: {}", e);
                None
            }
        }
    } else if let Some(ref keystore_path) = args.keystore {
        match ValidatorKeys::from_keystore(keystore_path) {
            Ok(k) => {
                info!("Loaded validator keys from keystore: {}", keystore_path.display());
                info!("  Sidechain: {}", k.sidechain_pub_key);
                info!("  Aura: {}", k.aura_pub_key);
                info!("  Grandpa: {}", k.grandpa_pub_key);
                Some(k)
            }
            Err(e) => {
                error!("Failed to load keys from keystore: {}", e);
                None
            }
        }
    } else {
        info!("No keys specified - key status monitoring disabled");
        info!("  Use --keystore <path> or --keys-file <path> to enable");
        None
    };

    let monitor = ValidatorMonitor::new(&args.rpc_url, &args.metrics_url, keys);

    // Try to get version on startup
    match monitor.get_version().await {
        Ok(version) => info!("Node version: {}", version),
        Err(e) => warn!("Could not fetch node version: {}", e),
    }

    if args.once {
        // Single run mode
        run_check(&monitor).await;
    } else {
        // Continuous monitoring loop
        info!("Monitoring interval: {}s", args.interval);
        let mut interval = time::interval(Duration::from_secs(args.interval));

        loop {
            interval.tick().await;
            run_check(&monitor).await;
        }
    }

    Ok(())
}

async fn run_check(monitor: &ValidatorMonitor) {
    match monitor.get_status().await {
        Ok(status) => {
            monitor.display_status(&status);
        }
        Err(e) => {
            error!("Failed to fetch validator status: {}", e);
        }
    }
}
