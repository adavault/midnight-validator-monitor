use crate::keys::{get_key_status, KeyStatus, ValidatorKeys};
use crate::metrics::MetricsClient;
use crate::rpc::RpcClient;
use crate::types::*;
use anyhow::Result;
use tracing::{debug, error, info, warn};

pub struct ValidatorMonitor {
    rpc: RpcClient,
    metrics: MetricsClient,
    keys: Option<ValidatorKeys>,
}

impl ValidatorMonitor {
    pub fn new(rpc_url: &str, metrics_url: &str, keys: Option<ValidatorKeys>) -> Self {
        Self {
            rpc: RpcClient::new(rpc_url),
            metrics: MetricsClient::new(metrics_url),
            keys,
        }
    }

    /// Fetch system health status
    pub async fn get_health(&self) -> Result<SystemHealth> {
        self.rpc.call("system_health", Vec::<()>::new()).await
    }

    /// Fetch sync state (returns None if not syncing)
    pub async fn get_sync_state(&self) -> Result<Option<SyncState>> {
        match self.rpc.call::<_, SyncState>("system_syncState", Vec::<()>::new()).await {
            Ok(state) => Ok(Some(state)),
            Err(_) => Ok(None),
        }
    }

    /// Fetch current block header
    pub async fn get_current_header(&self) -> Result<BlockHeader> {
        self.rpc.call("chain_getHeader", Vec::<()>::new()).await
    }

    /// Fetch finalized block hash and then its header
    pub async fn get_finalized_block(&self) -> Result<u64> {
        let hash: String = self.rpc.call("chain_getFinalizedHead", Vec::<()>::new()).await?;
        let header: BlockHeader = self.rpc.call("chain_getHeader", vec![&hash]).await?;
        Ok(header.block_number())
    }

    /// Fetch sidechain status (epoch/slot info)
    pub async fn get_sidechain_status(&self) -> Result<SidechainStatus> {
        self.rpc.call("sidechain_getStatus", Vec::<()>::new()).await
    }

    /// Fetch node version
    pub async fn get_version(&self) -> Result<String> {
        self.rpc.call("system_version", Vec::<()>::new()).await
    }

    /// Get the RPC client reference (for key status checks)
    pub fn rpc(&self) -> &RpcClient {
        &self.rpc
    }

    /// Collect complete validator status
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

        // Fetch block production metrics
        let blocks_produced = match self.metrics.fetch_metrics().await {
            Ok(m) => m.blocks_produced,
            Err(e) => {
                debug!("Could not fetch metrics: {}", e);
                0
            }
        };

        // Fetch key status if keys are configured
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

    /// Display status to console
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
                sc.sidechain.epoch, sc.sidechain.slot,
                sc.mainchain.epoch, sc.mainchain.slot
            );
        }

        // Display key status if available
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
        use crate::keys::RegistrationStatus;

        let sc_icon = key_status_icon(ks.sidechain_loaded);
        let aura_icon = key_status_icon(ks.aura_loaded);
        let gran_icon = key_status_icon(ks.grandpa_loaded);

        info!(
            "Keys: sidechain {} | aura {} | grandpa {}",
            sc_icon, aura_icon, gran_icon
        );

        // Display registration status
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

        // Warnings for key issues
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
