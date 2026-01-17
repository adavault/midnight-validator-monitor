//! Application state management for TUI

use crate::db::{BlockRecord, Database, ValidatorRecord};
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
}

/// Epoch progress information
#[derive(Debug, Clone, Default)]
pub struct EpochProgress {
    /// Current slot within the epoch
    pub current_slot_in_epoch: u64,
    /// Total slots in an epoch (typically 7200 for Midnight)
    pub epoch_length_slots: u64,
    /// Progress percentage (0-100)
    pub progress_percent: f64,
    /// Our blocks produced this epoch
    pub our_blocks_this_epoch: u64,
    /// Expected blocks for our validators this epoch
    pub expected_blocks: f64,
    /// Committee size (for block prediction)
    pub committee_size: u64,
    /// Number of seats our validators have in the committee
    pub our_committee_seats: u64,
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

    // Status
    pub last_error: Option<String>,
    pub update_duration: Duration,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            chain_tip: 0,
            finalized_block: 0,
            mainchain_epoch: 0,
            sidechain_epoch: 0,
            sidechain_slot: 0,
            sync_state_syncing: false,
            peer_count: 0,
            node_health: true,
            total_blocks: 0,
            total_validators: 0,
            our_validators_count: 0,
            recent_blocks: Vec::new(),
            validators: Vec::new(),
            our_validators: Vec::new(),
            epoch_progress: EpochProgress::default(),
            last_error: None,
            update_duration: Duration::from_secs(0),
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
        }
    }

    /// Update application state from RPC and database
    pub async fn update(&mut self, rpc: &RpcClient, db: &Database) -> Result<()> {
        let start = Instant::now();

        // Fetch RPC data
        match self.fetch_rpc_data(rpc).await {
            Ok(_) => {
                self.state.last_error = None;
            }
            Err(e) => {
                self.state.last_error = Some(format!("RPC error: {}", e));
            }
        }

        // Fetch database data
        match self.fetch_db_data(db) {
            Ok(_) => {
                if self.state.last_error.is_none() {
                    self.state.last_error = None;
                }
            }
            Err(e) => {
                self.state.last_error = Some(format!("DB error: {}", e));
            }
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

        // Get sidechain status and calculate epoch progress
        if let Ok(status) = rpc.call::<_, SidechainStatus>("sidechain_getStatus", Vec::<()>::new()).await {
            self.state.mainchain_epoch = status.mainchain.epoch;
            self.state.sidechain_epoch = status.sidechain.epoch;
            self.state.sidechain_slot = status.sidechain.slot;

            // Calculate epoch progress
            // Midnight epochs are approximately 7200 slots (2 hours at 1 slot/second)
            const EPOCH_LENGTH_SLOTS: u64 = 7200;
            let epoch_start_slot = status.sidechain.epoch * EPOCH_LENGTH_SLOTS;
            let current_slot_in_epoch = status.sidechain.slot.saturating_sub(epoch_start_slot);
            let progress = (current_slot_in_epoch as f64 / EPOCH_LENGTH_SLOTS as f64) * 100.0;

            self.state.epoch_progress.epoch_length_slots = EPOCH_LENGTH_SLOTS;
            self.state.epoch_progress.current_slot_in_epoch = current_slot_in_epoch.min(EPOCH_LENGTH_SLOTS);
            self.state.epoch_progress.progress_percent = progress.min(100.0);
        }

        // Get sync state
        if let Ok(sync_state) = rpc.call::<_, serde_json::Value>("system_syncState", Vec::<()>::new()).await {
            self.state.sync_state_syncing = sync_state.get("currentBlock")
                .and_then(|v| v.as_u64())
                .map(|current| current < self.state.chain_tip)
                .unwrap_or(false);
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

        // Calculate our blocks in current epoch
        let current_epoch = self.state.sidechain_epoch;
        if current_epoch > 0 {
            let mut our_blocks_this_epoch: u64 = 0;
            for v in &self.state.our_validators {
                if let Some(aura_key) = v.aura_key.as_ref() {
                    // Count blocks produced by this validator in the current epoch
                    // Note: For now we count from recent_blocks; a more accurate count
                    // would require a database query for blocks in the epoch range
                    let count = self.state.recent_blocks.iter()
                        .filter(|b| b.epoch == current_epoch &&
                                b.author_key.as_ref() == Some(aura_key))
                        .count() as u64;
                    our_blocks_this_epoch += count;
                }
            }
            self.state.epoch_progress.our_blocks_this_epoch = our_blocks_this_epoch;

            // Calculate expected blocks (if we have committee info)
            // Expected = (epoch_progress_percent / 100) * (epoch_length / committee_size) * our_seats
            // For now, use a simple estimate based on total validators
            if self.state.total_validators > 0 && self.state.our_validators_count > 0 {
                let epoch_progress_ratio = self.state.epoch_progress.progress_percent / 100.0;
                let expected_per_seat = self.state.epoch_progress.epoch_length_slots as f64 / 1200.0; // Approximate committee size
                self.state.epoch_progress.expected_blocks = epoch_progress_ratio * expected_per_seat * self.state.our_validators_count as f64;
            }
        }

        Ok(())
    }

    /// Switch to next view
    pub fn next_view(&mut self) {
        self.view_mode = match self.view_mode {
            ViewMode::Dashboard => ViewMode::Blocks,
            ViewMode::Blocks => ViewMode::Validators,
            ViewMode::Validators => ViewMode::Performance,
            ViewMode::Performance => ViewMode::Help,
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
            ViewMode::Help => ViewMode::Performance,
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
