//! Application state management for TUI

use crate::db::{BlockRecord, Database, ValidatorRecord};
use crate::metrics::MetricsClient;
use crate::midnight::ValidatorSet;
use crate::rpc::{RpcClient, SidechainStatus};
use crate::tui::Theme;
use anyhow::Result;
use std::time::{Duration, Instant};

/// View modes for the TUI
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ViewMode {
    Dashboard,
    Blocks,
    Validators,
    Performance,
    Peers,
    Help,
}

/// Application state
pub struct App {
    /// Current view mode
    pub view_mode: ViewMode,
    /// Should quit the application
    pub should_quit: bool,
    /// Filter to show only our validators
    pub show_ours_only: bool,
    /// Selected index for scrollable lists
    pub selected_index: usize,
    /// Application state data
    pub state: AppState,
    /// Last update timestamp
    pub last_update: Instant,
    /// Color theme
    pub theme: Theme,
    /// Expected IP for filtering external addresses (from config)
    pub expected_ip: Option<String>,
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
}

/// Information about a connected peer
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct PeerInfo {
    pub peer_id: String,
    pub best_hash: String,
    pub best_number: u64,
    pub address: Option<String>,  // IP:port if available
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
            selected_index: 0,
            state: AppState::default(),
            last_update: Instant::now(),
            theme: Theme::default(),
            expected_ip: None,
        }
    }

    /// Update application state from RPC and database
    pub async fn update(&mut self, rpc: &RpcClient, metrics: &MetricsClient, db: &Database) -> Result<()> {
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
            // Sidechain epochs are 2 hours (7200 seconds) and determine committee elections
            if let Some(next_epoch_ms) = status.sidechain.next_epoch_timestamp {
                const SIDECHAIN_EPOCH_DURATION_MS: u64 = 2 * 60 * 60 * 1000; // 2 hours in ms

                // Store for block counting in fetch_db_data
                self.state.sidechain_next_epoch_ms = Some(next_epoch_ms);

                let time_remaining_ms = next_epoch_ms.saturating_sub(now_ms);
                let time_elapsed_ms = SIDECHAIN_EPOCH_DURATION_MS.saturating_sub(time_remaining_ms);
                let progress = (time_elapsed_ms as f64 / SIDECHAIN_EPOCH_DURATION_MS as f64) * 100.0;

                self.state.epoch_progress.epoch_length_slots = SIDECHAIN_EPOCH_DURATION_MS / 1000;
                self.state.epoch_progress.current_slot_in_epoch = time_elapsed_ms / 1000;
                self.state.epoch_progress.progress_percent = progress.clamp(0.0, 100.0);
            }

            // Calculate MAINCHAIN epoch progress using nextEpochTimestamp
            // Mainchain epochs are 24 hours (86400 seconds)
            if let Some(next_epoch_ms) = status.mainchain.next_epoch_timestamp {
                const MAINCHAIN_EPOCH_DURATION_MS: u64 = 24 * 60 * 60 * 1000; // 24 hours in ms

                let time_remaining_ms = next_epoch_ms.saturating_sub(now_ms);
                let time_elapsed_ms = MAINCHAIN_EPOCH_DURATION_MS.saturating_sub(time_remaining_ms);
                let progress = (time_elapsed_ms as f64 / MAINCHAIN_EPOCH_DURATION_MS as f64) * 100.0;

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
                            // Parse multiaddr format like /ip4/82.69.29.148/tcp/30333
                            if addr.starts_with("/ip4/") {
                                let parts: Vec<&str> = addr.split('/').collect();
                                // parts: ["", "ip4", "82.69.29.148", "tcp", "30333"]
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
            // Also get network state to extract peer addresses
            let peer_addresses: std::collections::HashMap<String, String> =
                if let Ok(net_state) = rpc.call::<_, serde_json::Value>("system_unstable_networkState", Vec::<()>::new()).await {
                    net_state.get("connectedPeers")
                        .and_then(|v| v.as_object())
                        .map(|peers_obj| {
                            peers_obj.iter()
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
                        })
                        .unwrap_or_default()
                } else {
                    std::collections::HashMap::new()
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
                    Some(PeerInfo {
                        peer_id,
                        best_hash,
                        best_number,
                        address,
                    })
                })
                .collect();

            // Sort by best_number descending (most synced peers first)
            self.state.connected_peers.sort_by(|a, b| b.best_number.cmp(&a.best_number));
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

        // Get recent blocks
        let max_block = db.get_max_block_number()?.unwrap_or(0);
        if max_block > 0 {
            let start = max_block.saturating_sub(20);
            self.state.recent_blocks = db.get_blocks_in_range(start, max_block, Some(20))?;
            self.state.recent_blocks.reverse(); // Most recent first
        }

        // Get validators
        self.state.validators = db.get_all_validators()?;
        self.state.our_validators = db.get_our_validators()?;

        // Calculate our blocks in current SIDECHAIN epoch using timestamp-based query
        // Sidechain epochs are 2 hours and determine committee election
        // Blocks are timestamped, so we query by time range
        const SIDECHAIN_EPOCH_DURATION_SECS: i64 = 2 * 60 * 60; // 2 hours in seconds

        if let Some(next_epoch_ms) = self.state.sidechain_next_epoch_ms {
            // Calculate sidechain epoch start: next_epoch - 2 hours
            let epoch_start_secs = (next_epoch_ms / 1000) as i64 - SIDECHAIN_EPOCH_DURATION_SECS;

            let mut our_blocks_this_epoch: u64 = 0;
            for v in &self.state.our_validators {
                // Query database for blocks by this validator since epoch start
                match db.count_blocks_by_author_since(&v.sidechain_key, epoch_start_secs) {
                    Ok(count) => our_blocks_this_epoch += count,
                    Err(e) => {
                        tracing::warn!("Failed to count epoch blocks for {}: {}", v.sidechain_key, e);
                    }
                }
            }
            self.state.epoch_progress.our_blocks_this_epoch = our_blocks_this_epoch;

            // Calculate expected blocks - only if we're elected to the committee
            // If not in committee, expected is 0
            if self.state.committee_elected && self.state.committee_seats > 0 && self.state.committee_size > 0 {
                // Use committee seats for expected calculation
                // ~1200 blocks per 2h sidechain epoch (1 block per 6 seconds)
                // Each committee seat produces ~1 block per epoch (round-robin)
                let epoch_progress_ratio = self.state.epoch_progress.progress_percent / 100.0;
                const BLOCKS_PER_SIDECHAIN_EPOCH: f64 = 1200.0;
                let expected_per_seat = BLOCKS_PER_SIDECHAIN_EPOCH / self.state.committee_size as f64;
                self.state.epoch_progress.expected_blocks =
                    epoch_progress_ratio * expected_per_seat * self.state.committee_seats as f64;
            } else {
                // Not in committee - no blocks expected
                self.state.epoch_progress.expected_blocks = 0.0;
            }
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
        }
    }

    /// Switch to next view
    pub fn next_view(&mut self) {
        self.view_mode = match self.view_mode {
            ViewMode::Dashboard => ViewMode::Blocks,
            ViewMode::Blocks => ViewMode::Validators,
            ViewMode::Validators => ViewMode::Performance,
            ViewMode::Performance => ViewMode::Peers,
            ViewMode::Peers => ViewMode::Help,
            ViewMode::Help => ViewMode::Dashboard,
        };
        self.selected_index = 0;
    }

    /// Switch to previous view
    pub fn previous_view(&mut self) {
        self.view_mode = match self.view_mode {
            ViewMode::Dashboard => ViewMode::Help,
            ViewMode::Blocks => ViewMode::Dashboard,
            ViewMode::Validators => ViewMode::Blocks,
            ViewMode::Performance => ViewMode::Validators,
            ViewMode::Peers => ViewMode::Performance,
            ViewMode::Help => ViewMode::Peers,
        };
        self.selected_index = 0;
    }

    /// Switch to specific view
    pub fn set_view(&mut self, view: ViewMode) {
        self.view_mode = view;
        self.selected_index = 0;
    }

    /// Scroll down in current view
    pub fn scroll_down(&mut self) {
        // Help screen has 42 items (count of ListItems in render_help)
        const HELP_ITEM_COUNT: usize = 42;

        let max_index = match self.view_mode {
            ViewMode::Blocks => self.state.recent_blocks.len().saturating_sub(1),
            ViewMode::Validators => {
                if self.show_ours_only {
                    self.state.our_validators.len().saturating_sub(1)
                } else {
                    self.state.validators.len().saturating_sub(1)
                }
            }
            ViewMode::Performance => self.state.validators.len().saturating_sub(1),
            ViewMode::Peers => self.state.connected_peers.len().saturating_sub(1),
            ViewMode::Help => HELP_ITEM_COUNT.saturating_sub(1),
            _ => 0,
        };

        if self.selected_index < max_index {
            self.selected_index += 1;
        }
    }

    /// Scroll up in current view
    pub fn scroll_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    /// Toggle "ours only" filter
    pub fn toggle_ours_filter(&mut self) {
        self.show_ours_only = !self.show_ours_only;
        self.selected_index = 0;
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
