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
    #[arg(short, long, default_value = "http://localhost:9944")]
    pub rpc_url: String,

    /// Prometheus metrics endpoint URL
    #[arg(short = 'M', long, default_value = "http://localhost:9615/metrics")]
    pub metrics_url: String,

    /// Path to validator keys JSON file
    #[arg(short, long, conflicts_with = "keystore")]
    pub keys_file: Option<PathBuf>,

    /// Path to Substrate keystore directory
    #[arg(short = 'K', long, conflicts_with = "keys_file")]
    pub keystore: Option<PathBuf>,

    /// Monitoring interval in seconds
    #[arg(short, long, default_value_t = 60)]
    pub interval: u64,

    /// Run once and exit (don't loop)
    #[arg(long)]
    pub once: bool,
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
}

impl StatusMonitor {
    pub fn new(rpc_url: &str, metrics_url: &str, keys: Option<ValidatorKeys>) -> Self {
        Self {
            rpc: RpcClient::new(rpc_url),
            metrics: MetricsClient::new(metrics_url),
            keys,
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
                .map(|s| s.sidechain.epoch)
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
        info!(
            "Block: {} | Finalized: {} | Sync: {:.2}%",
            status.current_block,
            status.finalized_block,
            status.sync_percentage()
        );
        info!("Blocks produced: {}", status.blocks_produced);

        if let Some(ref sc) = status.sidechain_status {
            info!(
                "Sidechain: epoch {} slot {} | Mainchain: epoch {} slot {}",
                sc.sidechain.epoch, sc.sidechain.slot, sc.mainchain.epoch, sc.mainchain.slot
            );
        }

        if let Some(ref ks) = status.key_status {
            self.display_key_status(ks);
        }

        // Warnings
        if status.health.is_syncing {
            warn!("Node is still syncing");
        }
        if status.peer_count == 0 {
            error!("No peers connected!");
        }
        if status.current_block.saturating_sub(status.finalized_block) > 100 {
            warn!(
                "Large finality gap: {} blocks behind",
                status.current_block - status.finalized_block
            );
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

        match &ks.registration {
            Some(RegistrationStatus::Permissioned) => {
                info!("Registration: ✓ Permissioned candidate");
            }
            Some(RegistrationStatus::RegisteredValid) => {
                info!("Registration: ✓ Registered (valid)");
            }
            Some(RegistrationStatus::RegisteredInvalid(reason)) => {
                warn!("Registration: ⚠ Registered but INVALID: {}", reason);
            }
            Some(RegistrationStatus::NotRegistered) => {
                error!("Registration: ✗ Not registered");
            }
            None => {
                info!("Registration: ? Unable to check");
            }
        }

        if ks.sidechain_loaded == Some(false) {
            error!("Sidechain key not loaded in keystore!");
        }
        if ks.aura_loaded == Some(false) {
            error!("Aura key not loaded in keystore!");
        }
        if ks.grandpa_loaded == Some(false) {
            error!("Grandpa key not loaded in keystore!");
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
        info!("  Use --keystore <path> or --keys-file <path> to enable");
        None
    };

    let monitor = StatusMonitor::new(&args.rpc_url, &args.metrics_url, keys);

    // Try to get version on startup
    match monitor.get_version().await {
        Ok(version) => info!("Node version: {}", version),
        Err(e) => warn!("Could not fetch node version: {}", e),
    }

    if args.once {
        run_check(&monitor).await;
    } else {
        info!("Monitoring interval: {}s", args.interval);
        let mut interval = time::interval(Duration::from_secs(args.interval));

        loop {
            interval.tick().await;
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
