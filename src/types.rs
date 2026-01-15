use crate::keys::KeyStatus;
use serde::Deserialize;

/// Response from system_health RPC call
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemHealth {
    pub peers: u32,
    pub is_syncing: bool,
    pub should_have_peers: bool,
}

/// Response from system_syncState RPC call
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncState {
    pub current_block: u64,
    pub highest_block: u64,
    pub starting_block: u64,
}

/// Response from chain_getHeader RPC call
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockHeader {
    pub parent_hash: String,
    pub number: String,
    pub state_root: String,
    pub extrinsics_root: String,
}

impl BlockHeader {
    pub fn block_number(&self) -> u64 {
        // Number comes as hex string like "0x1a2b"
        let hex_str = self.number.trim_start_matches("0x");
        u64::from_str_radix(hex_str, 16).unwrap_or(0)
    }
}

/// Epoch/slot info for a chain
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainStatus {
    pub epoch: u64,
    pub slot: u64,
    pub next_epoch_timestamp: Option<u64>,
}

/// Response from sidechain_getStatus RPC call
#[derive(Debug, Deserialize)]
pub struct SidechainStatus {
    pub sidechain: ChainStatus,
    pub mainchain: ChainStatus,
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
    /// Total blocks produced by this validator
    pub blocks_produced: u64,
    /// Validator key status (if keys configured)
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
            100.0 // Not syncing means we're synced
        } else {
            0.0
        }
    }

    pub fn is_healthy(&self) -> bool {
        !self.health.is_syncing && self.health.peers > 0
    }
}
