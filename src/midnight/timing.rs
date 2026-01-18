//! Network timing configuration for different Midnight networks
//!
//! Each network (preview, preprod, mainnet) has different epoch durations:
//! - Sidechain epochs determine committee rotation
//! - Mainchain epochs align with Cardano epochs
//!
//! The ratio is consistent: 12 sidechain epochs per mainchain epoch on all networks.

use serde::{Deserialize, Serialize};

/// Network identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Network {
    /// Preview testnet (testnet-02) - 24h mainchain epochs, 2h sidechain epochs
    #[default]
    Preview,
    /// PreProd testnet - timing TBD (assuming same as preview until confirmed)
    Preprod,
    /// Mainnet - 5 day mainchain epochs, 10h sidechain epochs
    Mainnet,
}

impl Network {
    /// Parse network from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "preview" | "testnet" | "testnet-02" => Some(Network::Preview),
            "preprod" | "pre-prod" => Some(Network::Preprod),
            "mainnet" | "main" => Some(Network::Mainnet),
            _ => None,
        }
    }

    /// Get the display name for this network
    pub fn name(&self) -> &'static str {
        match self {
            Network::Preview => "preview",
            Network::Preprod => "preprod",
            Network::Mainnet => "mainnet",
        }
    }
}

/// Chain timing parameters for a specific network
#[derive(Debug, Clone)]
pub struct ChainTiming {
    /// Network this timing is for
    pub network: Network,

    /// Slot duration in milliseconds (6000ms = 6 seconds for all Midnight networks)
    pub slot_duration_ms: u64,

    /// Sidechain epoch duration in milliseconds
    /// This determines committee rotation frequency
    pub sidechain_epoch_ms: u64,

    /// Mainchain epoch duration in milliseconds
    /// Aligns with Cardano epoch boundaries
    pub mainchain_epoch_ms: u64,

    /// Genesis timestamp in milliseconds since Unix epoch
    /// Used to calculate block timestamps from slot numbers
    pub genesis_timestamp_ms: Option<u64>,
}

impl ChainTiming {
    /// Preview testnet timing (testnet-02)
    /// - 6 second blocks
    /// - 2 hour sidechain epochs (committee rotation)
    /// - 24 hour mainchain epochs
    pub fn preview() -> Self {
        Self {
            network: Network::Preview,
            slot_duration_ms: 6_000,                      // 6 seconds
            sidechain_epoch_ms: 2 * 60 * 60 * 1_000,     // 2 hours
            mainchain_epoch_ms: 24 * 60 * 60 * 1_000,    // 24 hours
            genesis_timestamp_ms: None,                   // TBD
        }
    }

    /// PreProd testnet timing
    /// Currently assumed same as preview - update when confirmed
    pub fn preprod() -> Self {
        Self {
            network: Network::Preprod,
            slot_duration_ms: 6_000,                      // 6 seconds
            sidechain_epoch_ms: 2 * 60 * 60 * 1_000,     // 2 hours (TBD)
            mainchain_epoch_ms: 24 * 60 * 60 * 1_000,    // 24 hours (TBD)
            genesis_timestamp_ms: None,                   // TBD
        }
    }

    /// Mainnet timing
    /// - 6 second blocks
    /// - 10 hour sidechain epochs
    /// - 5 day mainchain epochs
    pub fn mainnet() -> Self {
        Self {
            network: Network::Mainnet,
            slot_duration_ms: 6_000,                      // 6 seconds
            sidechain_epoch_ms: 10 * 60 * 60 * 1_000,    // 10 hours
            mainchain_epoch_ms: 5 * 24 * 60 * 60 * 1_000, // 5 days
            genesis_timestamp_ms: None,                   // TBD
        }
    }

    /// Create timing for a specific network
    pub fn for_network(network: Network) -> Self {
        match network {
            Network::Preview => Self::preview(),
            Network::Preprod => Self::preprod(),
            Network::Mainnet => Self::mainnet(),
        }
    }

    /// Blocks per sidechain epoch
    /// This is the theoretical maximum - actual may be slightly less due to missed slots
    pub fn blocks_per_sidechain_epoch(&self) -> u64 {
        self.sidechain_epoch_ms / self.slot_duration_ms
    }

    /// Blocks per mainchain epoch
    pub fn blocks_per_mainchain_epoch(&self) -> u64 {
        self.mainchain_epoch_ms / self.slot_duration_ms
    }

    /// Sidechain epochs per mainchain epoch
    /// This is 12 for all networks (by design)
    pub fn sidechain_epochs_per_mainchain(&self) -> u64 {
        self.mainchain_epoch_ms / self.sidechain_epoch_ms
    }

    /// Calculate timestamp from slot number (if genesis is known)
    pub fn slot_to_timestamp_ms(&self, slot: u64) -> Option<u64> {
        self.genesis_timestamp_ms
            .map(|genesis| genesis + (slot * self.slot_duration_ms))
    }

    /// Calculate expected blocks for a validator based on their committee seats
    ///
    /// # Arguments
    /// * `committee_seats` - Number of seats the validator has in the committee
    /// * `committee_size` - Total committee size (typically ~1200)
    /// * `epoch_progress` - Progress through the epoch (0.0 to 1.0)
    ///
    /// # Returns
    /// Expected number of blocks the validator should have produced so far
    pub fn expected_blocks(
        &self,
        committee_seats: u64,
        committee_size: u64,
        epoch_progress: f64,
    ) -> f64 {
        if committee_size == 0 {
            return 0.0;
        }

        let blocks_per_epoch = self.blocks_per_sidechain_epoch() as f64;
        let expected_per_seat = blocks_per_epoch / committee_size as f64;

        epoch_progress * expected_per_seat * committee_seats as f64
    }
}

impl Default for ChainTiming {
    fn default() -> Self {
        Self::preview()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preview_timing() {
        let timing = ChainTiming::preview();
        assert_eq!(timing.slot_duration_ms, 6_000);
        assert_eq!(timing.blocks_per_sidechain_epoch(), 1200); // 2h / 6s = 1200
        assert_eq!(timing.sidechain_epochs_per_mainchain(), 12); // 24h / 2h = 12
    }

    #[test]
    fn test_mainnet_timing() {
        let timing = ChainTiming::mainnet();
        assert_eq!(timing.slot_duration_ms, 6_000);
        assert_eq!(timing.blocks_per_sidechain_epoch(), 6000); // 10h / 6s = 6000
        assert_eq!(timing.sidechain_epochs_per_mainchain(), 12); // 120h / 10h = 12
    }

    #[test]
    fn test_expected_blocks() {
        let timing = ChainTiming::preview();

        // With 1200 committee size and 1200 blocks per epoch,
        // each seat should produce ~1 block per epoch
        let expected = timing.expected_blocks(10, 1200, 1.0);
        assert!((expected - 10.0).abs() < 0.01);

        // At 50% progress, expect half the blocks
        let expected_half = timing.expected_blocks(10, 1200, 0.5);
        assert!((expected_half - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_network_from_str() {
        assert_eq!(Network::from_str("preview"), Some(Network::Preview));
        assert_eq!(Network::from_str("PREVIEW"), Some(Network::Preview));
        assert_eq!(Network::from_str("testnet-02"), Some(Network::Preview));
        assert_eq!(Network::from_str("mainnet"), Some(Network::Mainnet));
        assert_eq!(Network::from_str("preprod"), Some(Network::Preprod));
        assert_eq!(Network::from_str("unknown"), None);
    }
}
