//! Status command - display current validator node status

use crate::metrics::MetricsClient;
use crate::midnight::{get_key_status, KeyStatus, RegistrationStatus, ValidatorKeys};
use crate::rpc::{BlockHeader, RpcClient, SidechainStatus, SyncState, SystemHealth};
use anyhow::Result;
use clap::Args;
use std::path::PathBuf;
use std::time::Duration;
use tokio::time;
use tracing::{debug, error, info, warn};

/// Status command arguments
#[derive(Args, Debug)]
pub struct StatusArgs {
    /// Validator node RPC endpoint URL
    #[arg(short, long)]
    pub rpc_url: Option<String>,

    /// Prometheus metrics endpoint URL
    #[arg(short = 'M', long)]
    pub metrics_url: Option<String>,

    /// Path to validator keys JSON file
    #[arg(short, long, conflicts_with = "keystore")]
    pub keys_file: Option<PathBuf>,

    /// Path to Substrate keystore directory
    #[arg(short = 'K', long, conflicts_with = "keys_file")]
    pub keystore: Option<PathBuf>,

    /// Monitoring interval in seconds
    #[arg(short, long)]
    pub interval: Option<u64>,

    /// Run once and exit (don't loop)
    #[arg(long)]
    pub once: bool,

    /// Show explanations for each metric (educational mode)
    #[arg(short = 'E', long)]
    pub explain: bool,
}

/// Combined validator status for display
#[derive(Debug)]
pub struct ValidatorStatus {
    pub health: SystemHealth,
    pub sync_state: Option<SyncState>,
    pub current_block: u64,
    pub finalized_block: u64,
    pub sidechain_status: Option<SidechainStatus>,
    pub peer_count: usize,
    pub blocks_produced: u64,
    pub key_status: Option<KeyStatus>,
}

impl ValidatorStatus {
    pub fn sync_percentage(&self) -> f64 {
        if let Some(ref sync) = self.sync_state {
            if sync.highest_block == 0 {
                return 100.0;
            }
            (sync.current_block as f64 / sync.highest_block as f64) * 100.0
        } else if self.current_block > 0 {
            100.0
        } else {
            0.0
        }
    }

    pub fn is_healthy(&self) -> bool {
        !self.health.is_syncing && self.health.peers > 0
    }
}

/// Status monitor
pub struct StatusMonitor {
    rpc: RpcClient,
    metrics: MetricsClient,
    keys: Option<ValidatorKeys>,
    explain: bool,
}

impl StatusMonitor {
    pub fn new(rpc_url: &str, metrics_url: &str, keys: Option<ValidatorKeys>, timeout_ms: u64, explain: bool) -> Self {
        Self {
            rpc: RpcClient::with_timeout(rpc_url, timeout_ms),
            metrics: MetricsClient::new(metrics_url),
            keys,
            explain,
        }
    }

    pub async fn get_health(&self) -> Result<SystemHealth> {
        self.rpc.call("system_health", Vec::<()>::new()).await
    }

    pub async fn get_sync_state(&self) -> Result<Option<SyncState>> {
        match self.rpc.call::<_, SyncState>("system_syncState", Vec::<()>::new()).await {
            Ok(state) => Ok(Some(state)),
            Err(_) => Ok(None),
        }
    }

    pub async fn get_current_header(&self) -> Result<BlockHeader> {
        self.rpc.call("chain_getHeader", Vec::<()>::new()).await
    }

    pub async fn get_finalized_block(&self) -> Result<u64> {
        let hash: String = self.rpc.call("chain_getFinalizedHead", Vec::<()>::new()).await?;
        let header: BlockHeader = self.rpc.call("chain_getHeader", vec![&hash]).await?;
        Ok(header.block_number())
    }

    pub async fn get_sidechain_status(&self) -> Result<SidechainStatus> {
        self.rpc.call("sidechain_getStatus", Vec::<()>::new()).await
    }

    pub async fn get_version(&self) -> Result<String> {
        self.rpc.call("system_version", Vec::<()>::new()).await
    }

    pub async fn get_status(&self) -> Result<ValidatorStatus> {
        let health = self.get_health().await?;
        let sync_state = self.get_sync_state().await?;
        let header = self.get_current_header().await?;
        let finalized = self.get_finalized_block().await?;

        let sidechain_status = match self.get_sidechain_status().await {
            Ok(status) => Some(status),
            Err(e) => {
                debug!("Could not fetch sidechain status: {}", e);
                None
            }
        };

        let blocks_produced = match self.metrics.fetch_metrics().await {
            Ok(m) => m.blocks_produced,
            Err(e) => {
                debug!("Could not fetch metrics: {}", e);
                0
            }
        };

        let key_status = if let Some(ref keys) = self.keys {
            let current_epoch = sidechain_status
                .as_ref()
                .map(|s| s.mainchain.epoch)
                .unwrap_or(0);
            Some(get_key_status(&self.rpc, keys, current_epoch).await)
        } else {
            None
        };

        let peer_count = health.peers as usize;

        Ok(ValidatorStatus {
            health,
            sync_state,
            current_block: header.block_number(),
            finalized_block: finalized,
            sidechain_status,
            peer_count,
            blocks_produced,
            key_status,
        })
    }

    pub fn display_status(&self, status: &ValidatorStatus) {
        let health_icon = if status.is_healthy() { "✓" } else { "✗" };
        let sync_icon = if status.health.is_syncing { "⟳" } else { "✓" };

        info!("─────────────────────────────────────────");
        info!(
            "Health: {} | Syncing: {} | Peers: {}",
            health_icon, sync_icon, status.peer_count
        );
        if self.explain {
            info!("  → Health: Combined indicator - node is connected and not syncing");
            info!("  → Syncing: ✓ means synced to chain tip, ⟳ means still catching up");
            info!("  → Peers: Number of connected nodes. Want 10+, minimum 3-5 to function");
        }

        info!(
            "Block: {} | Finalized: {} | Sync: {:.2}%",
            status.current_block,
            status.finalized_block,
            status.sync_percentage()
        );
        if self.explain {
            info!("  → Block: Current best block (may not be finalized yet)");
            info!("  → Finalized: Highest block confirmed by GRANDPA consensus");
            info!("  → Sync: Percentage of known chain downloaded (100% = fully synced)");
        }

        info!("Blocks produced: {}", status.blocks_produced);
        if self.explain {
            info!("  → Blocks produced: Total blocks authored by your node since startup");
            info!("    This resets when node restarts. For historical data, use 'mvm view'");
        }

        if let Some(ref sc) = status.sidechain_status {
            info!(
                "Sidechain: epoch {} slot {} | Mainchain: epoch {} slot {}",
                sc.sidechain.epoch, sc.sidechain.slot, sc.mainchain.epoch, sc.mainchain.slot
            );
            if self.explain {
                info!("  → Sidechain epoch: 2-hour cycle (preview) that determines committee");
                info!("  → Mainchain epoch: 24-hour cycle (preview) used for registration");
                info!("  → Slot: 6-second block production window within each epoch");
            }
        }

        if let Some(ref ks) = status.key_status {
            self.display_key_status(ks);
        }

        // Warnings
        if status.health.is_syncing {
            warn!("Node is still syncing");
            if self.explain {
                info!("  → Your node must finish syncing before it can produce blocks");
            }
        }
        if status.peer_count == 0 {
            error!("No peers connected!");
            if self.explain {
                info!("  → Check internet connectivity and firewall (port 30333)");
            }
        }
        if status.current_block.saturating_sub(status.finalized_block) > 100 {
            warn!(
                "Large finality gap: {} blocks behind",
                status.current_block - status.finalized_block
            );
            if self.explain {
                info!("  → Normally finality is within 10-20 blocks. Large gap may indicate");
                info!("    network issues or your node is on a minority fork");
            }
        }
    }

    fn display_key_status(&self, ks: &KeyStatus) {
        let sc_icon = key_status_icon(ks.sidechain_loaded);
        let aura_icon = key_status_icon(ks.aura_loaded);
        let gran_icon = key_status_icon(ks.grandpa_loaded);

        info!(
            "Keys: sidechain {} | aura {} | grandpa {}",
            sc_icon, aura_icon, gran_icon
        );
        if self.explain {
            info!("  → Sidechain (crch): Your validator's identity key");
            info!("  → Aura: Block production authorization key");
            info!("  → Grandpa: Finality voting key");
            info!("  → ✓ = loaded in node keystore, ✗ = missing, ? = unable to verify");
        }

        // Show note if keys can't be verified
        if ks.sidechain_loaded.is_none() && ks.aura_loaded.is_none() && ks.grandpa_loaded.is_none() {
            info!("Note: Key verification requires node started with --rpc-methods=unsafe");
        }

        match &ks.registration {
            Some(RegistrationStatus::Permissioned) => {
                info!("Registration: ✓ Permissioned candidate");
                if self.explain {
                    info!("  → Permissioned = IOG/Midnight team validator, no stake required");
                }
            }
            Some(RegistrationStatus::RegisteredValid) => {
                info!("Registration: ✓ Registered (valid)");
                if self.explain {
                    info!("  → Your registration is active and eligible for committee selection");
                }
            }
            Some(RegistrationStatus::RegisteredInvalid(reason)) => {
                warn!("Registration: ⚠ Registered but INVALID: {}", reason);
                if self.explain {
                    info!("  → Registration exists but not valid. May be pending or have issues.");
                    info!("  → Common causes: insufficient stake, keys mismatch, processing delay");
                }
            }
            Some(RegistrationStatus::NotRegistered) => {
                error!("Registration: ✗ Not registered");
                if self.explain {
                    info!("  → No registration found. Submit registration transaction to participate.");
                }
            }
            None => {
                info!("Registration: ? Unable to check");
            }
        }

        // Display committee status
        if let Some(ref committee) = ks.committee_status {
            if committee.in_committee {
                info!(
                    "Committee: ✓ Elected ({} seats / {} total = {:.2}% selection)",
                    committee.seat_count,
                    committee.committee_size,
                    committee.selection_probability * 100.0
                );
                if self.explain {
                    info!("  → You ARE in this epoch's committee and CAN produce blocks");
                    info!("  → Seats = how many times you appear in the rotation schedule");
                    info!("  → More seats = more block production opportunities");
                }
                info!(
                    "Expected blocks: ~{:.1} per sidechain epoch",
                    committee.expected_blocks_per_epoch
                );
                if self.explain {
                    info!("  → Based on your seat count and epoch duration (1200 blocks on preview)");
                }
            } else {
                warn!(
                    "Committee: ✗ NOT elected (committee size: {})",
                    committee.committee_size
                );
                warn!("Your validator is registered but was not selected for this epoch's committee.");
                warn!("Committee selection is stake-weighted random. Keep your node running and staked.");
                if self.explain {
                    info!("  → Being registered does NOT guarantee committee selection");
                    info!("  → Selection is stake-weighted random each sidechain epoch (2h preview)");
                    info!("  → Higher stake = higher probability, but not guaranteed");
                    info!("  → This is NORMAL - wait for next epoch or increase stake");
                }
            }

            // Display stake if available
            if let Some(stake) = committee.stake_lovelace {
                let ada = stake as f64 / 1_000_000.0;
                if ada >= 1_000_000.0 {
                    info!("Stake: {:.2}M tADA", ada / 1_000_000.0);
                } else if ada >= 1_000.0 {
                    info!("Stake: {:.2}K tADA", ada / 1_000.0);
                } else {
                    info!("Stake: {:.2} tADA", ada);
                }
                if self.explain {
                    info!("  → Your delegated stake affects committee selection probability");
                }
            }
        }

        if ks.sidechain_loaded == Some(false) {
            error!("Sidechain key not loaded in keystore!");
            if self.explain {
                info!("  → Copy your sidechain key file to the node's keystore directory");
            }
        }
        if ks.aura_loaded == Some(false) {
            error!("Aura key not loaded in keystore!");
            if self.explain {
                info!("  → Without AURA key, your node cannot produce blocks");
            }
        }
        if ks.grandpa_loaded == Some(false) {
            error!("Grandpa key not loaded in keystore!");
            if self.explain {
                info!("  → Without GRANDPA key, your node cannot vote on finality");
            }
        }
    }
}

fn key_status_icon(status: Option<bool>) -> &'static str {
    match status {
        Some(true) => "✓",
        Some(false) => "✗",
        None => "?",
    }
}

/// Run the status command
pub async fn run(args: StatusArgs) -> Result<()> {
    // Load configuration
    let config = crate::config::Config::load()?;

    // Use args or fall back to config
    let rpc_url = args.rpc_url.unwrap_or(config.rpc.url);
    let metrics_url = args.metrics_url.unwrap_or(config.rpc.metrics_url);
    let interval = args.interval.unwrap_or(60);

    info!("RPC endpoint: {}", rpc_url);
    info!("Metrics endpoint: {}", metrics_url);

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
    } else {
        // Try keystore from args or config
        let keystore_path = args.keystore.or_else(|| config.validator.keystore_path.map(PathBuf::from));

        if let Some(ref keystore_path) = keystore_path {
            match ValidatorKeys::from_keystore(keystore_path) {
                Ok(k) => {
                    info!(
                        "Loaded validator keys from keystore: {}",
                        keystore_path.display()
                    );
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
            info!("  Use --keystore <path> or set validator.keystore_path in config");
            None
        }
    };

    let monitor = StatusMonitor::new(&rpc_url, &metrics_url, keys, config.rpc.timeout_ms, args.explain);

    // Try to get version on startup
    match monitor.get_version().await {
        Ok(version) => info!("Node version: {}", version),
        Err(e) => {
            warn!("Could not connect to node at {}", rpc_url);
            warn!("Error: {}", e);
            warn!("");
            warn!("Tip: Make sure your Midnight node is running and RPC is enabled.");
            warn!("     Check the RPC URL is correct (default port is 9944).");
        }
    }

    if args.once {
        run_check(&monitor).await;
    } else {
        info!("Monitoring interval: {}s", interval);
        let mut interval_timer = time::interval(Duration::from_secs(interval));

        loop {
            interval_timer.tick().await;
            run_check(&monitor).await;
        }
    }

    Ok(())
}

async fn run_check(monitor: &StatusMonitor) {
    match monitor.get_status().await {
        Ok(status) => {
            monitor.display_status(&status);
        }
        Err(e) => {
            error!("Failed to fetch validator status: {}", e);
        }
    }
}
