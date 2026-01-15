use serde::Deserialize;

/// Response from system_health RPC call
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemHealth {
    pub peers: u32,
    pub is_syncing: bool,
    pub should_have_peers: bool,
}

/// Response from system_syncState RPC call
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncState {
    pub current_block: u64,
    pub highest_block: u64,
    pub starting_block: u64,
}

/// Response from chain_getHeader RPC call
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockHeader {
    pub parent_hash: String,
    pub number: String,
    pub state_root: String,
    pub extrinsics_root: String,
    pub digest: Option<Digest>,
}

impl BlockHeader {
    pub fn block_number(&self) -> u64 {
        parse_hex_number(&self.number).unwrap_or(0)
    }
}

/// Block digest containing consensus logs
#[derive(Debug, Clone, Deserialize)]
pub struct Digest {
    pub logs: Vec<String>,
}

/// Response from chain_getBlock RPC call
#[derive(Debug, Clone, Deserialize)]
pub struct SignedBlock {
    pub block: Block,
    pub justifications: Option<serde_json::Value>,
}

/// Block structure
#[derive(Debug, Clone, Deserialize)]
pub struct Block {
    pub header: BlockHeader,
    pub extrinsics: Vec<String>,
}

/// Epoch/slot info for a chain
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainStatus {
    pub epoch: u64,
    pub slot: u64,
    pub next_epoch_timestamp: Option<u64>,
}

/// Response from sidechain_getStatus RPC call
#[derive(Debug, Clone, Deserialize)]
pub struct SidechainStatus {
    pub sidechain: ChainStatus,
    pub mainchain: ChainStatus,
}

/// Parse a hex string (with or without 0x prefix) to u64
pub fn parse_hex_number(s: &str) -> Option<u64> {
    let hex_str = s.trim_start_matches("0x");
    u64::from_str_radix(hex_str, 16).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hex_number() {
        assert_eq!(parse_hex_number("0x332534"), Some(3351860));
        assert_eq!(parse_hex_number("0x0"), Some(0));
        assert_eq!(parse_hex_number("332534"), Some(3351860));
        assert_eq!(parse_hex_number("0x1"), Some(1));
    }
}
