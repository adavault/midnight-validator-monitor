//! Application state management for TUI

use crate::db::{BlockRecord, Database, ValidatorRecord, ValidatorEpochRecord, ValidatorEpochHistoryRecord};
use crate::metrics::{MetricsClient, NodeExporterClient};
use crate::midnight::{ChainTiming, ValidatorSet};
use crate::rpc::{RpcClient, SidechainStatus};
use crate::tui::Theme;
use anyhow::Result;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// View modes for the TUI
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ViewMode {
    Dashboard,
    Blocks,
    Validators,
    Performance,
    Peers,
    Help,
    /// Drill-down view for validator epoch history
    ValidatorEpochDetail,
}

/// Popup content for modal overlays
#[derive(Debug, Clone)]
pub enum PopupContent {
    /// Block detail popup showing full block information
    BlockDetail { block: BlockRecord },
    /// Peer detail popup showing peer connection details
    PeerDetail { peer: PeerInfo },
    /// Validator detail popup showing epoch history (from Performance view)
    ValidatorDetail {
        validator: ValidatorRecord,
        epoch_history: Vec<ValidatorEpochHistoryRecord>,
        scroll_index: usize,
    },
    /// Validator identity card (from Validators view)
    ValidatorIdentity {
        validator: ValidatorRecord,
        aura_key: Option<String>,
        current_epoch_seats: u32,
        committee_size: u32,
        blocks_this_epoch: u64,
        stake_display: Option<String>,
    },
}

/// Entry in the view stack for drill-down navigation
#[derive(Debug, Clone)]
pub struct ViewStackEntry {
    /// The view mode to return to
    pub view: ViewMode,
    /// The selection index in that view
    pub selection: usize,
    /// Optional context (e.g., sidechain_key for validator detail)
    /// Currently reserved for future use
    #[allow(dead_code)]
    pub context: Option<String>,
}

/// Application state
pub struct App {
    /// Current view mode
    pub view_mode: ViewMode,
    /// Should quit the application
    pub should_quit: bool,
    /// Filter to show only our validators
    pub show_ours_only: bool,
    /// Per-view selection indices (preserved when switching views)
    pub view_selections: HashMap<ViewMode, usize>,
    /// Popup overlay (Block/Peer detail)
    pub popup: Option<PopupContent>,
    /// View stack for drill-down navigation
    pub view_stack: Vec<ViewStackEntry>,
    /// Context for drill-down views (e.g., sidechain_key for validator detail)
    pub drill_down_context: Option<String>,
    /// Epoch history data for validator drill-down view
    pub validator_epoch_history: Vec<ValidatorEpochHistoryRecord>,
    /// Validator info for drill-down header
    pub drill_down_validator: Option<ValidatorRecord>,
    /// Application state data
    pub state: AppState,
    /// Last update timestamp
    pub last_update: Instant,
    /// Color theme
    pub theme: Theme,
    /// Expected IP for filtering external addresses (from config)
    pub expected_ip: Option<String>,
    /// Chain timing parameters (network-specific)
    pub chain_timing: ChainTiming,
}

/// Epoch progress information
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct EpochProgress {
    /// Current slot within the epoch
    pub current_slot_in_epoch: u64,
    /// Total slots in an epoch (typically 7200 for Midnight)
    pub epoch_length_slots: u64,
    /// Sidechain epoch progress percentage (0-100) - 2 hour cycle
    pub progress_percent: f64,
    /// Mainchain epoch progress percentage (0-100) - 24 hour cycle
    pub mainchain_progress_percent: f64,
    /// Our blocks produced this epoch
    pub our_blocks_this_epoch: u64,
    /// Expected blocks for our validators this epoch
    pub expected_blocks: f64,
    /// Committee size (for block prediction)
    pub committee_size: u64,
    /// Number of seats our validators have in the committee
    pub our_committee_seats: u64,
}

/// Node sync progress information
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct SyncProgress {
    /// Current block the node has synced to
    pub current_block: u64,
    /// Highest known block in the network
    pub highest_block: u64,
    /// Block sync started from
    pub starting_block: u64,
    /// Sync percentage (0-100)
    pub sync_percent: f64,
    /// Whether the node is fully synced
    pub is_synced: bool,
    /// Blocks remaining to sync
    pub blocks_remaining: u64,
}

/// Application state data
pub struct AppState {
    // Network status
    pub chain_tip: u64,
    pub finalized_block: u64,
    pub mainchain_epoch: u64,
    pub sidechain_epoch: u64,
    pub sidechain_slot: u64,
    pub sync_state_syncing: bool,
    pub peer_count: u64,
    pub peers_inbound: u64,
    pub peers_outbound: u64,
    pub node_health: bool,

    // Node sync progress
    pub sync_progress: SyncProgress,

    // Node identity
    pub node_name: String,
    pub chain_name: String,
    pub node_version: String,

    // Database stats
    pub total_blocks: u64,
    pub total_validators: u64,
    pub our_validators_count: u64,

    // Recent blocks
    pub recent_blocks: Vec<BlockRecord>,

    // Validators
    pub validators: Vec<ValidatorRecord>,
    pub our_validators: Vec<ValidatorRecord>,

    // Epoch progress (enhanced dashboard)
    pub epoch_progress: EpochProgress,

    // Committee election status
    pub committee_elected: bool,
    pub committee_seats: usize,
    pub committee_size: usize,

    // Sidechain epoch timing (for block counting)
    /// Next sidechain epoch timestamp in ms (used to calculate epoch start)
    pub sidechain_next_epoch_ms: Option<u64>,

    // Validator epoch data (seats per validator in current epoch)
    /// Maps sidechain_key -> ValidatorEpochRecord for current sidechain epoch
    pub validator_epoch_data: HashMap<String, ValidatorEpochRecord>,
    /// Maps sidechain_key -> blocks produced this epoch
    pub validator_epoch_blocks: HashMap<String, u64>,

    // Block production sparkline (for dashboard)
    /// Block counts per sidechain epoch for our validators (last 24 epochs = 48h)
    /// Index 0 = oldest, index 23 = most recent (left to right in sparkline)
    pub our_blocks_sparkline: Vec<u64>,
    /// Total committee seats for our validators over the sparkline period
    pub sparkline_total_seats: u64,

    // Status
    pub last_error: Option<String>,
    pub update_duration: Duration,
    /// True until the first successful data fetch
    pub is_loading: bool,

    // Node metrics (from Prometheus endpoint)
    pub bandwidth_in: u64,
    pub bandwidth_out: u64,
    pub txpool_ready: u64,
    pub txpool_validations: u64,
    pub uptime_secs: u64,
    pub grandpa_voter: bool,

    // Network state (from system_unstable_networkState)
    pub local_peer_id: String,
    pub external_ips: Vec<String>,
    pub external_ip_fetched: bool,  // Flag to prevent re-fetching (IP order varies)
    pub connected_peers: Vec<PeerInfo>,

    // Prometheus-based peer metrics (supplemental info)
    pub peers_discovered: u64,
    pub pending_connections: u64,

    // System resource metrics (from node_exporter)
    pub system_load1: f64,
    pub system_memory_used_bytes: u64,
    pub system_memory_total_bytes: u64,
    pub system_disk_used_bytes: u64,
    pub system_disk_total_bytes: u64,
}

/// Information about a connected peer
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct PeerInfo {
    pub peer_id: String,
    pub best_hash: String,
    pub best_number: u64,
    pub address: Option<String>,  // IP:port if available
    pub is_outbound: bool,        // true = we dialed them, false = they dialed us
}

impl Default for AppState {
    fn default() -> Self {
        // Get hostname for default node name
        let node_name = hostname::get()
            .ok()
            .and_then(|h| h.into_string().ok())
            .unwrap_or_else(|| "unknown".to_string());

        Self {
            chain_tip: 0,
            finalized_block: 0,
            mainchain_epoch: 0,
            sidechain_epoch: 0,
            sidechain_slot: 0,
            sync_state_syncing: false,
            peer_count: 0,
            peers_inbound: 0,
            peers_outbound: 0,
            node_health: true,
            sync_progress: SyncProgress::default(),
            node_name,
            chain_name: String::new(),
            node_version: String::new(),
            total_blocks: 0,
            total_validators: 0,
            our_validators_count: 0,
            recent_blocks: Vec::new(),
            validators: Vec::new(),
            our_validators: Vec::new(),
            epoch_progress: EpochProgress::default(),
            committee_elected: false,
            committee_seats: 0,
            committee_size: 0,
            sidechain_next_epoch_ms: None,
            validator_epoch_data: HashMap::new(),
            validator_epoch_blocks: HashMap::new(),
            our_blocks_sparkline: Vec::new(),
            sparkline_total_seats: 0,
            last_error: None,
            update_duration: Duration::from_secs(0),
            is_loading: true,
            bandwidth_in: 0,
            bandwidth_out: 0,
            txpool_ready: 0,
            txpool_validations: 0,
            uptime_secs: 0,
            grandpa_voter: false,
            local_peer_id: String::new(),
            external_ips: Vec::new(),
            external_ip_fetched: false,
            connected_peers: Vec::new(),
            peers_discovered: 0,
            pending_connections: 0,
            system_load1: 0.0,
            system_memory_used_bytes: 0,
            system_memory_total_bytes: 0,
            system_disk_used_bytes: 0,
            system_disk_total_bytes: 0,
        }
    }
}

impl App {
    /// Create a new App instance
    pub fn new() -> Self {
        Self {
            view_mode: ViewMode::Dashboard,
            should_quit: false,
            show_ours_only: false,
            view_selections: HashMap::new(),
            popup: None,
            view_stack: Vec::new(),
            drill_down_context: None,
            validator_epoch_history: Vec::new(),
            drill_down_validator: None,
            state: AppState::default(),
            last_update: Instant::now(),
            theme: Theme::default(),
            expected_ip: None,
            chain_timing: ChainTiming::default(),
        }
    }

    /// Get the current selection index for the active view
    pub fn selected_index(&self) -> usize {
        *self.view_selections.get(&self.view_mode).unwrap_or(&0)
    }

    /// Set the selection index for the current view
    pub fn set_selected_index(&mut self, index: usize) {
        self.view_selections.insert(self.view_mode, index);
    }

    /// Set chain timing configuration
    pub fn with_chain_timing(mut self, timing: ChainTiming) -> Self {
        self.chain_timing = timing;
        self
    }

    /// Update application state from RPC and database
    pub async fn update(&mut self, rpc: &RpcClient, metrics: &MetricsClient, node_exporter: Option<&NodeExporterClient>, db: &Database) -> Result<()> {
        let start = Instant::now();

        // Fetch RPC data
        let rpc_ok = match self.fetch_rpc_data(rpc).await {
            Ok(_) => {
                self.state.last_error = None;
                true
            }
            Err(e) => {
                self.state.last_error = Some(format!("RPC error: {}", e));
                false
            }
        };

        // Fetch metrics data (non-critical, don't fail on error)
        self.fetch_metrics_data(metrics).await;

        // Fetch node_exporter metrics if configured (non-critical)
        if let Some(ne) = node_exporter {
            self.fetch_node_exporter_data(ne).await;
        }

        // Fetch database data
        let db_ok = match self.fetch_db_data(db) {
            Ok(_) => {
                if self.state.last_error.is_none() {
                    self.state.last_error = None;
                }
                true
            }
            Err(e) => {
                self.state.last_error = Some(format!("DB error: {}", e));
                false
            }
        };

        // Clear loading state on first successful fetch
        if rpc_ok && db_ok {
            self.state.is_loading = false;
        }

        self.state.update_duration = start.elapsed();
        self.last_update = Instant::now();

        Ok(())
    }

    async fn fetch_rpc_data(&mut self, rpc: &RpcClient) -> Result<()> {
        // Get chain tip
        let header: crate::rpc::BlockHeader = rpc.call("chain_getHeader", Vec::<()>::new()).await?;
        self.state.chain_tip = header.block_number();

        // Get finalized block
        let finalized_hash: String = rpc.call("chain_getFinalizedHead", Vec::<()>::new()).await?;
        let finalized_header: crate::rpc::BlockHeader = rpc.call("chain_getHeader", vec![&finalized_hash]).await?;
        self.state.finalized_block = finalized_header.block_number();

        // Get chain name (network identifier)
        if self.state.chain_name.is_empty() {
            if let Ok(chain) = rpc.call::<_, String>("system_chain", Vec::<()>::new()).await {
                self.state.chain_name = chain;
            }
        }

        // Get node version
        if self.state.node_version.is_empty() {
            if let Ok(version) = rpc.call::<_, String>("system_version", Vec::<()>::new()).await {
                self.state.node_version = version;
            }
        }

        // Get sidechain status and calculate epoch progress
        if let Ok(status) = rpc.call::<_, SidechainStatus>("sidechain_getStatus", Vec::<()>::new()).await {
            self.state.mainchain_epoch = status.mainchain.epoch;
            self.state.sidechain_epoch = status.sidechain.epoch;
            self.state.sidechain_slot = status.sidechain.slot;

            // Get current time for progress calculations
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);

            // Calculate SIDECHAIN epoch progress using nextEpochTimestamp
            // Sidechain epoch duration varies by network (2h preview, 10h mainnet)
            if let Some(next_epoch_ms) = status.sidechain.next_epoch_timestamp {
                let sidechain_epoch_ms = self.chain_timing.sidechain_epoch_ms;

                // Store for block counting in fetch_db_data
                self.state.sidechain_next_epoch_ms = Some(next_epoch_ms);

                let time_remaining_ms = next_epoch_ms.saturating_sub(now_ms);
                let time_elapsed_ms = sidechain_epoch_ms.saturating_sub(time_remaining_ms);
                let progress = (time_elapsed_ms as f64 / sidechain_epoch_ms as f64) * 100.0;

                self.state.epoch_progress.epoch_length_slots = sidechain_epoch_ms / 1000;
                self.state.epoch_progress.current_slot_in_epoch = time_elapsed_ms / 1000;
                self.state.epoch_progress.progress_percent = progress.clamp(0.0, 100.0);
            }

            // Calculate MAINCHAIN epoch progress using nextEpochTimestamp
            // Mainchain epoch duration varies by network (24h preview, 5d mainnet)
            if let Some(next_epoch_ms) = status.mainchain.next_epoch_timestamp {
                let mainchain_epoch_ms = self.chain_timing.mainchain_epoch_ms;

                let time_remaining_ms = next_epoch_ms.saturating_sub(now_ms);
                let time_elapsed_ms = mainchain_epoch_ms.saturating_sub(time_remaining_ms);
                let progress = (time_elapsed_ms as f64 / mainchain_epoch_ms as f64) * 100.0;

                self.state.epoch_progress.mainchain_progress_percent = progress.clamp(0.0, 100.0);
            }
        }

        // Get sync state with detailed progress
        if let Ok(sync_state) = rpc.call::<_, serde_json::Value>("system_syncState", Vec::<()>::new()).await {
            let current_block = sync_state.get("currentBlock")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let highest_block = sync_state.get("highestBlock")
                .and_then(|v| v.as_u64())
                .unwrap_or(self.state.chain_tip);
            let starting_block = sync_state.get("startingBlock")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);

            // Calculate sync progress
            let total_to_sync = highest_block.saturating_sub(starting_block);
            let synced = current_block.saturating_sub(starting_block);
            let sync_percent = if total_to_sync > 0 {
                (synced as f64 / total_to_sync as f64) * 100.0
            } else {
                100.0
            };

            let blocks_remaining = highest_block.saturating_sub(current_block);
            let is_synced = blocks_remaining <= 1; // Allow 1 block tolerance

            self.state.sync_progress = SyncProgress {
                current_block,
                highest_block,
                starting_block,
                sync_percent: sync_percent.clamp(0.0, 100.0),
                is_synced,
                blocks_remaining,
            };

            self.state.sync_state_syncing = !is_synced;
        }

        // Get system health (includes peer count)
        if let Ok(health) = rpc.call::<_, serde_json::Value>("system_health", Vec::<()>::new()).await {
            self.state.peer_count = health.get("peers")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            self.state.node_health = !health.get("isSyncing")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
        }

        // Get network state (external IPs, peer ID, connected peers)
        // This requires --rpc-methods=unsafe on the node
        if let Ok(network_state) = rpc.call::<_, serde_json::Value>("system_unstable_networkState", Vec::<()>::new()).await {
            // Extract local peer ID (only once)
            if self.state.local_peer_id.is_empty() {
                if let Some(peer_id) = network_state.get("peerId").and_then(|v| v.as_str()) {
                    self.state.local_peer_id = peer_id.to_string();
                }
            }

            // Extract external addresses - collect all unique public IPs
            // Use flag to prevent re-fetching since array order varies between RPC calls
            if !self.state.external_ip_fetched {
                self.state.external_ip_fetched = true;  // Mark as attempted regardless of result

                if let Some(external) = network_state.get("externalAddresses").and_then(|v| v.as_array()) {
                    // Collect all public IPs (not just the first one)
                    let mut public_ips: Vec<String> = external.iter()
                        .filter_map(|addr| addr.as_str())
                        .filter_map(|addr| {
                            // Parse multiaddr format like /ip4/203.0.113.1/tcp/30333
                            if addr.starts_with("/ip4/") {
                                let parts: Vec<&str> = addr.split('/').collect();
                                // parts: ["", "ip4", "203.0.113.1", "tcp", "30333"]
                                if parts.len() >= 5 {
                                    let ip = parts[2];
                                    let port = parts[4];
                                    // Filter out private/internal IPs
                                    if !ip.starts_with("127.")
                                        && !ip.starts_with("10.")
                                        && !ip.starts_with("192.168.")
                                        && !ip.starts_with("172.16.")
                                        && !ip.starts_with("172.17.")
                                        && !ip.starts_with("172.18.")
                                        && !ip.starts_with("172.19.")
                                        && !ip.starts_with("172.2")
                                        && !ip.starts_with("172.30.")
                                        && !ip.starts_with("172.31.")
                                        && !ip.starts_with("0.")
                                    {
                                        return Some(format!("{}:{}", ip, port));
                                    }
                                }
                            }
                            None
                        })
                        .collect();

                    // Deduplicate and sort for consistent display
                    public_ips.sort();
                    public_ips.dedup();

                    // Filter by expected IP if configured
                    if let Some(ref expected) = self.expected_ip {
                        public_ips.retain(|addr| addr.starts_with(expected));
                    }

                    if !public_ips.is_empty() {
                        self.state.external_ips = public_ips;
                    }
                }
            }
        }

        // Get connected peers with sync info
        if let Ok(peers) = rpc.call::<_, Vec<serde_json::Value>>("system_peers", Vec::<()>::new()).await {
            // Also get network state to extract peer addresses and connection direction
            let (peer_addresses, peer_directions): (HashMap<String, String>, HashMap<String, bool>) =
                if let Ok(net_state) = rpc.call::<_, serde_json::Value>("system_unstable_networkState", Vec::<()>::new()).await {
                    let peers_obj = net_state.get("connectedPeers").and_then(|v| v.as_object());

                    let addresses = peers_obj.map(|obj| {
                        obj.iter()
                            .filter_map(|(peer_id, info)| {
                                // Extract first public IP from knownAddresses (try IPv4 first, then IPv6)
                                let addr = info.get("knownAddresses")
                                    .and_then(|v| v.as_array())
                                    .and_then(|addrs| {
                                        // First try to find a public IPv4
                                        let ipv4 = addrs.iter()
                                            .filter_map(|a| a.as_str())
                                            .find_map(|addr| {
                                                if addr.starts_with("/ip4/") {
                                                    let parts: Vec<&str> = addr.split('/').collect();
                                                    if parts.len() >= 5 {
                                                        let ip = parts[2];
                                                        let port = parts[4];
                                                        // Filter out private IPs
                                                        if !ip.starts_with("127.")
                                                            && !ip.starts_with("10.")
                                                            && !ip.starts_with("192.168.")
                                                            && !ip.starts_with("172.16.")
                                                            && !ip.starts_with("172.17.")
                                                            && !ip.starts_with("172.18.")
                                                            && !ip.starts_with("172.19.")
                                                            && !ip.starts_with("172.2")
                                                            && !ip.starts_with("172.30.")
                                                            && !ip.starts_with("172.31.")
                                                            && !ip.starts_with("0.")
                                                        {
                                                            return Some(format!("{}:{}", ip, port));
                                                        }
                                                    }
                                                }
                                                None
                                            });

                                        // If no IPv4, try IPv6
                                        ipv4.or_else(|| {
                                            addrs.iter()
                                                .filter_map(|a| a.as_str())
                                                .find_map(|addr| {
                                                    if addr.starts_with("/ip6/") {
                                                        let parts: Vec<&str> = addr.split('/').collect();
                                                        if parts.len() >= 5 {
                                                            let ip = parts[2];
                                                            let port = parts[4];
                                                            // Filter out localhost
                                                            if ip != "::1" && !ip.starts_with("fe80:") {
                                                                return Some(format!("[{}]:{}", ip, port));
                                                            }
                                                        }
                                                    }
                                                    None
                                                })
                                        })
                                    });
                                addr.map(|a| (peer_id.clone(), a))
                            })
                            .collect()
                    }).unwrap_or_default();

                    // Extract connection direction from endpoint field
                    // "dialing" = outbound (we connected to them)
                    // "listening" = inbound (they connected to us)
                    let directions = peers_obj.map(|obj| {
                        obj.iter()
                            .map(|(peer_id, info)| {
                                let is_outbound = info.get("endpoint")
                                    .and_then(|e| e.as_object())
                                    .map(|ep| ep.contains_key("dialing"))
                                    .unwrap_or(true); // Default to outbound if unknown
                                (peer_id.clone(), is_outbound)
                            })
                            .collect()
                    }).unwrap_or_default();

                    (addresses, directions)
                } else {
                    (HashMap::new(), HashMap::new())
                };

            self.state.connected_peers = peers.iter()
                .filter_map(|peer| {
                    let peer_id = peer.get("peerId")?.as_str()?.to_string();
                    let best_hash = peer.get("bestHash")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let best_number = peer.get("bestNumber")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);
                    let address = peer_addresses.get(&peer_id).cloned();
                    let is_outbound = peer_directions.get(&peer_id).copied().unwrap_or(true);
                    Some(PeerInfo {
                        peer_id,
                        best_hash,
                        best_number,
                        address,
                        is_outbound,
                    })
                })
                .collect();

            // Sort by best_number descending (most synced peers first)
            self.state.connected_peers.sort_by(|a, b| b.best_number.cmp(&a.best_number));

            // Count inbound/outbound
            self.state.peers_outbound = self.state.connected_peers.iter().filter(|p| p.is_outbound).count() as u64;
            self.state.peers_inbound = self.state.connected_peers.iter().filter(|p| !p.is_outbound).count() as u64;
        }

        // Check committee election status for our validators
        // Only check if we have validators marked as ours
        if !self.state.our_validators.is_empty() {
            if let Ok(committee) = ValidatorSet::fetch_committee_at_block(rpc, None).await {
                self.state.committee_size = committee.len();

                // Count how many seats our validators have in the committee
                let mut total_seats = 0;
                for validator in &self.state.our_validators {
                    if let Some(ref aura_key) = validator.aura_key {
                        // Normalize key for comparison
                        let normalized = if aura_key.starts_with("0x") {
                            aura_key.to_lowercase()
                        } else {
                            format!("0x{}", aura_key.to_lowercase())
                        };
                        // Count occurrences in committee
                        total_seats += committee.iter()
                            .filter(|k| k.to_lowercase() == normalized)
                            .count();
                    }
                }

                self.state.committee_seats = total_seats;
                self.state.committee_elected = total_seats > 0;
            }
        }

        Ok(())
    }

    fn fetch_db_data(&mut self, db: &Database) -> Result<()> {
        // Get database stats
        self.state.total_blocks = db.count_blocks()?;
        self.state.total_validators = db.count_validators()?;
        self.state.our_validators_count = db.count_our_validators()?;

        // Get recent blocks - fetch enough to fill most terminal heights
        let max_block = db.get_max_block_number()?.unwrap_or(0);
        if max_block > 0 {
            let blocks_to_fetch = 50; // Enough for tall terminals
            let start = max_block.saturating_sub(blocks_to_fetch - 1);
            self.state.recent_blocks = db.get_blocks_in_range(start, max_block, Some(blocks_to_fetch as u32))?;
            self.state.recent_blocks.reverse(); // Most recent first
        }

        // Get validators
        self.state.validators = db.get_all_validators()?;
        self.state.our_validators = db.get_our_validators()?;

        // Get validator epoch data for current sidechain epoch (seats info)
        if self.state.sidechain_epoch > 0 {
            match db.get_validators_for_epoch(self.state.sidechain_epoch) {
                Ok(epoch_records) => {
                    self.state.validator_epoch_data = epoch_records
                        .into_iter()
                        .map(|r| (r.sidechain_key.clone(), r))
                        .collect();
                }
                Err(e) => {
                    tracing::debug!("No epoch data for epoch {}: {}", self.state.sidechain_epoch, e);
                }
            }
        }

        // Calculate blocks in current SIDECHAIN epoch for all validators
        // Sidechain epoch duration varies by network (2h preview, 10h mainnet)
        // Blocks are timestamped, so we query by time range
        let sidechain_epoch_duration_secs = (self.chain_timing.sidechain_epoch_ms / 1000) as i64;

        if let Some(next_epoch_ms) = self.state.sidechain_next_epoch_ms {
            // Calculate sidechain epoch start: next_epoch - epoch duration
            let epoch_start_secs = (next_epoch_ms / 1000) as i64 - sidechain_epoch_duration_secs;

            // Calculate epoch blocks for ALL validators (for validators view)
            self.state.validator_epoch_blocks.clear();
            for v in &self.state.validators {
                match db.count_blocks_by_author_since(&v.sidechain_key, epoch_start_secs) {
                    Ok(count) => {
                        self.state.validator_epoch_blocks.insert(v.sidechain_key.clone(), count);
                    }
                    Err(e) => {
                        tracing::debug!("Failed to count epoch blocks for {}: {}", v.sidechain_key, e);
                    }
                }
            }

            // Sum up our validators' blocks for the dashboard
            let mut our_blocks_this_epoch: u64 = 0;
            for v in &self.state.our_validators {
                if let Some(&count) = self.state.validator_epoch_blocks.get(&v.sidechain_key) {
                    our_blocks_this_epoch += count;
                }
            }
            self.state.epoch_progress.our_blocks_this_epoch = our_blocks_this_epoch;

            // Calculate expected blocks - only if we're elected to the committee
            // If not in committee, expected is 0
            if self.state.committee_elected && self.state.committee_seats > 0 && self.state.committee_size > 0 {
                // Use network-aware timing for expected block calculation
                // blocks_per_epoch varies by network (1200 for preview, 6000 for mainnet)
                let epoch_progress_ratio = self.state.epoch_progress.progress_percent / 100.0;
                self.state.epoch_progress.expected_blocks = self.chain_timing.expected_blocks(
                    self.state.committee_seats as u64,
                    self.state.committee_size as u64,
                    epoch_progress_ratio,
                );
            } else {
                // Not in committee - no blocks expected
                self.state.epoch_progress.expected_blocks = 0.0;
            }
        }

        // Fetch sparkline data for our validators (block production over last 24 sidechain epochs = 48h)
        let num_buckets = 24; // 24 epochs = 48h on preview, 10 days on mainnet
        if !self.state.our_validators.is_empty() {
            let author_keys: Vec<String> = self.state.our_validators
                .iter()
                .map(|v| v.sidechain_key.clone())
                .collect();

            // Use sidechain epoch duration for buckets (2h for preview, 10h for mainnet)
            let bucket_secs = (self.chain_timing.sidechain_epoch_ms / 1000) as i64;

            match db.get_block_counts_bucketed(&author_keys, bucket_secs, num_buckets) {
                Ok(counts) => {
                    self.state.our_blocks_sparkline = counts;
                }
                Err(e) => {
                    tracing::debug!("Failed to fetch sparkline data: {}", e);
                    self.state.our_blocks_sparkline = vec![0; num_buckets];
                }
            }

            // Fetch total seats for the sparkline period
            match db.get_total_seats_for_epochs(&author_keys, self.state.sidechain_epoch, num_buckets) {
                Ok(seats) => {
                    self.state.sparkline_total_seats = seats;
                }
                Err(e) => {
                    tracing::debug!("Failed to fetch sparkline seats: {}", e);
                    self.state.sparkline_total_seats = 0;
                }
            }
        } else {
            self.state.our_blocks_sparkline = vec![0; num_buckets];
            self.state.sparkline_total_seats = 0;
        }

        Ok(())
    }

    async fn fetch_metrics_data(&mut self, metrics: &MetricsClient) {
        // Metrics are non-critical - don't fail the update if they're unavailable
        if let Ok(m) = metrics.fetch_metrics().await {
            self.state.bandwidth_in = m.bandwidth_in;
            self.state.bandwidth_out = m.bandwidth_out;
            self.state.txpool_ready = m.txpool_ready;
            self.state.txpool_validations = m.txpool_validations_finished;
            self.state.grandpa_voter = m.grandpa_voter;

            // Calculate uptime from process start time
            if m.process_start_time > 0.0 {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs_f64())
                    .unwrap_or(0.0);
                self.state.uptime_secs = (now - m.process_start_time) as u64;
            }

            // Prometheus provides additional peer network info (don't override RPC counts)
            self.state.peers_discovered = m.peers_discovered;
            self.state.pending_connections = m.pending_connections;
        }
    }

    async fn fetch_node_exporter_data(&mut self, node_exporter: &NodeExporterClient) {
        // Node exporter metrics are non-critical - don't fail the update if unavailable
        if let Ok(m) = node_exporter.fetch_metrics().await {
            self.state.system_load1 = m.load1;

            // Calculate memory used = total - available
            if m.memory_total_bytes > 0 {
                self.state.system_memory_total_bytes = m.memory_total_bytes;
                self.state.system_memory_used_bytes = m.memory_total_bytes
                    .saturating_sub(m.memory_available_bytes);
            }

            // Calculate disk used = total - available
            if m.disk_total_bytes > 0 {
                self.state.system_disk_total_bytes = m.disk_total_bytes;
                self.state.system_disk_used_bytes = m.disk_total_bytes
                    .saturating_sub(m.disk_available_bytes);
            }
        }
    }

    /// Switch to next view (skips drill-down views)
    pub fn next_view(&mut self) {
        // Close popup if open
        if self.popup.is_some() {
            self.popup = None;
            return;
        }
        // If in a drill-down view, pop back first
        if self.view_mode == ViewMode::ValidatorEpochDetail {
            self.pop_view();
            return;
        }
        self.view_mode = match self.view_mode {
            ViewMode::Dashboard => ViewMode::Blocks,
            ViewMode::Blocks => ViewMode::Validators,
            ViewMode::Validators => ViewMode::Performance,
            ViewMode::Performance => ViewMode::Peers,
            ViewMode::Peers => ViewMode::Help,
            ViewMode::Help => ViewMode::Dashboard,
            ViewMode::ValidatorEpochDetail => ViewMode::Performance, // Should not happen
        };
        // Selection is preserved in view_selections HashMap
    }

    /// Switch to previous view (skips drill-down views)
    pub fn previous_view(&mut self) {
        // Close popup if open
        if self.popup.is_some() {
            self.popup = None;
            return;
        }
        // If in a drill-down view, pop back first
        if self.view_mode == ViewMode::ValidatorEpochDetail {
            self.pop_view();
            return;
        }
        self.view_mode = match self.view_mode {
            ViewMode::Dashboard => ViewMode::Help,
            ViewMode::Blocks => ViewMode::Dashboard,
            ViewMode::Validators => ViewMode::Blocks,
            ViewMode::Performance => ViewMode::Validators,
            ViewMode::Peers => ViewMode::Performance,
            ViewMode::Help => ViewMode::Peers,
            ViewMode::ValidatorEpochDetail => ViewMode::Performance, // Should not happen
        };
        // Selection is preserved in view_selections HashMap
    }

    /// Switch to specific view (does not affect drill-down views)
    pub fn set_view(&mut self, view: ViewMode) {
        // Close popup if open
        if self.popup.is_some() {
            self.popup = None;
        }
        // Clear view stack when explicitly switching views
        self.view_stack.clear();
        self.drill_down_context = None;
        self.view_mode = view;
        // Selection is preserved in view_selections HashMap
    }

    /// Get max scroll index for current view
    fn max_scroll_index(&self) -> usize {
        // Help screen item count (count of ListItems in render_help)
        const HELP_ITEM_COUNT: usize = 73; // About + Keyboard Shortcuts + Reference sections

        match self.view_mode {
            ViewMode::Blocks => self.state.recent_blocks.len().saturating_sub(1),
            ViewMode::Validators => {
                if self.show_ours_only {
                    self.state.our_validators.len().saturating_sub(1)
                } else {
                    self.state.validators.len().saturating_sub(1)
                }
            }
            ViewMode::Performance => {
                if self.show_ours_only {
                    self.state.our_validators.len().saturating_sub(1)
                } else {
                    self.state.validators.len().saturating_sub(1)
                }
            }
            ViewMode::Peers => self.state.connected_peers.len().saturating_sub(1),
            ViewMode::Help => HELP_ITEM_COUNT.saturating_sub(1),
            ViewMode::ValidatorEpochDetail => self.validator_epoch_history.len().saturating_sub(1),
            _ => 0,
        }
    }

    /// Scroll down in current view
    pub fn scroll_down(&mut self) {
        let current = self.selected_index();
        let max_index = self.max_scroll_index();

        if current < max_index {
            self.set_selected_index(current + 1);
        }
    }

    /// Scroll up in current view
    pub fn scroll_up(&mut self) {
        let current = self.selected_index();
        if current > 0 {
            self.set_selected_index(current - 1);
        }
    }

    /// Scroll down by a page (10 items)
    pub fn scroll_page_down(&mut self) {
        const PAGE_SIZE: usize = 10;
        let current = self.selected_index();
        let max_index = self.max_scroll_index();

        self.set_selected_index((current + PAGE_SIZE).min(max_index));
    }

    /// Scroll up by a page (10 items)
    pub fn scroll_page_up(&mut self) {
        const PAGE_SIZE: usize = 10;
        let current = self.selected_index();
        self.set_selected_index(current.saturating_sub(PAGE_SIZE));
    }

    /// Toggle "ours only" filter
    pub fn toggle_ours_filter(&mut self) {
        self.show_ours_only = !self.show_ours_only;
        self.set_selected_index(0);
    }

    /// Get validators sorted for display (permissioned first, then by seats desc)
    /// This is the single source of truth for validator ordering
    pub fn get_sorted_validators(&self) -> Vec<ValidatorRecord> {
        let mut validators: Vec<_> = if self.show_ours_only {
            self.state.our_validators.clone()
        } else {
            self.state.validators.clone()
        };

        let epoch_data = &self.state.validator_epoch_data;
        validators.sort_by(|a, b| {
            let a_perm = a.registration_status.as_deref() == Some("permissioned");
            let b_perm = b.registration_status.as_deref() == Some("permissioned");
            match (a_perm, b_perm) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => {
                    let a_seats = epoch_data.get(&a.sidechain_key).map(|e| e.committee_seats).unwrap_or(0);
                    let b_seats = epoch_data.get(&b.sidechain_key).map(|e| e.committee_seats).unwrap_or(0);
                    b_seats.cmp(&a_seats).then_with(|| a.sidechain_key.cmp(&b.sidechain_key))
                }
            }
        });

        validators
    }

    // ========================================
    // Popup Management
    // ========================================

    /// Open block detail popup
    pub fn open_block_popup(&mut self) {
        let index = self.selected_index();
        if index < self.state.recent_blocks.len() {
            let block = self.state.recent_blocks[index].clone();
            self.popup = Some(PopupContent::BlockDetail { block });
        }
    }

    /// Open peer detail popup
    pub fn open_peer_popup(&mut self) {
        let index = self.selected_index();
        if index < self.state.connected_peers.len() {
            let peer = self.state.connected_peers[index].clone();
            self.popup = Some(PopupContent::PeerDetail { peer });
        }
    }

    /// Open validator identity popup (from Validators view)
    pub fn open_validator_identity_popup(&mut self) {
        // Use the same sorted list as render_validators
        let validators = self.get_sorted_validators();

        let index = self.selected_index();
        if index >= validators.len() {
            return;
        }

        let validator = validators[index].clone();
        let sidechain_key = &validator.sidechain_key;

        // Get epoch data for this validator
        let epoch_data = self.state.validator_epoch_data.get(sidechain_key);
        let current_epoch_seats = epoch_data.map(|d| d.committee_seats).unwrap_or(0);
        let committee_size = epoch_data.map(|d| d.committee_size).unwrap_or(0);
        let aura_key = epoch_data.map(|d| d.aura_key.clone());

        // Get blocks this epoch
        let blocks_this_epoch = self.state.validator_epoch_blocks
            .get(sidechain_key)
            .copied()
            .unwrap_or(0);

        // Format stake if available
        let stake_display = epoch_data.and_then(|d| d.stake_lovelace).map(|stake| {
            // Convert lovelace to ADA (1 ADA = 1,000,000 lovelace)
            let ada = stake as f64 / 1_000_000.0;
            if ada >= 1_000_000.0 {
                format!("{:.2}M tADA", ada / 1_000_000.0)
            } else if ada >= 1_000.0 {
                format!("{:.2}K tADA", ada / 1_000.0)
            } else {
                format!("{:.2} tADA", ada)
            }
        });

        self.popup = Some(PopupContent::ValidatorIdentity {
            validator,
            aura_key,
            current_epoch_seats,
            committee_size,
            blocks_this_epoch,
            stake_display,
        });
    }

    /// Close any open popup
    pub fn close_popup(&mut self) {
        self.popup = None;
    }

    /// Check if a popup is open
    pub fn has_popup(&self) -> bool {
        self.popup.is_some()
    }

    // ========================================
    // Validator Detail Popup
    // ========================================

    /// Open validator detail popup (from Performance view)
    pub fn open_validator_popup(&mut self, db: &Database) {
        // Use the same sorting as render_performance: by total_blocks descending
        let mut validators: Vec<_> = if self.show_ours_only {
            self.state.our_validators.clone()
        } else {
            self.state.validators.clone()
        };
        validators.sort_by(|a, b| b.total_blocks.cmp(&a.total_blocks));

        let index = self.selected_index();
        if index >= validators.len() {
            return;
        }

        let validator = validators[index].clone();
        let sidechain_key = validator.sidechain_key.clone();

        // Load epoch history from database
        let epoch_history = match db.get_validator_epoch_history(&sidechain_key, 50) {
            Ok(history) => history,
            Err(e) => {
                tracing::warn!("Failed to load validator epoch history: {}", e);
                Vec::new()
            }
        };

        // Open as popup
        self.popup = Some(PopupContent::ValidatorDetail {
            validator,
            epoch_history,
            scroll_index: 0,
        });
    }

    /// Scroll down within validator detail popup
    pub fn popup_scroll_down(&mut self) {
        if let Some(PopupContent::ValidatorDetail { epoch_history, scroll_index, .. }) = &mut self.popup {
            let max_index = epoch_history.len().saturating_sub(1);
            if *scroll_index < max_index {
                *scroll_index += 1;
            }
        }
    }

    /// Scroll up within validator detail popup
    pub fn popup_scroll_up(&mut self) {
        if let Some(PopupContent::ValidatorDetail { scroll_index, .. }) = &mut self.popup {
            if *scroll_index > 0 {
                *scroll_index -= 1;
            }
        }
    }

    /// Page down within validator detail popup
    pub fn popup_page_down(&mut self) {
        if let Some(PopupContent::ValidatorDetail { epoch_history, scroll_index, .. }) = &mut self.popup {
            let max_index = epoch_history.len().saturating_sub(1);
            *scroll_index = (*scroll_index + 10).min(max_index);
        }
    }

    /// Page up within validator detail popup
    pub fn popup_page_up(&mut self) {
        if let Some(PopupContent::ValidatorDetail { scroll_index, .. }) = &mut self.popup {
            *scroll_index = scroll_index.saturating_sub(10);
        }
    }

    // ========================================
    // View Stack Management (Legacy - kept for potential future use)
    // ========================================

    /// Pop back to the previous view in the stack
    #[allow(dead_code)]
    pub fn pop_view(&mut self) {
        if let Some(entry) = self.view_stack.pop() {
            self.view_mode = entry.view;
            self.set_selected_index(entry.selection);
            self.drill_down_context = None;
            self.drill_down_validator = None;
            self.validator_epoch_history.clear();
        }
    }

    /// Check if we can pop back (view stack is not empty)
    #[allow(dead_code)]
    pub fn can_pop(&self) -> bool {
        !self.view_stack.is_empty()
    }

    /// Toggle theme
    pub fn toggle_theme(&mut self) {
        self.theme = self.theme.toggle();
    }

    /// Quit the application
    pub fn quit(&mut self) {
        self.should_quit = true;
    }
}
