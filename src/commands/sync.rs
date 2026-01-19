//! Sync command - synchronize blocks to local database

use crate::db::{BlockRecord, Database, ValidatorRecord, ValidatorEpochRecord};
use crate::midnight::{extract_slot_from_digest, ChainTiming, ValidatorSet};
use crate::rpc::{RpcClient, SignedBlock, SidechainStatus};
use anyhow::{Context, Result};
use clap::Args;
use signal_hook::consts::signal::*;
use signal_hook_tokio::Signals;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::Duration;
use tokio::time;
use tokio::select;
use tokio_stream::StreamExt;
use tracing::{debug, info, warn};

/// Sync command arguments
#[derive(Args, Debug)]
pub struct SyncArgs {
    /// Validator node RPC endpoint URL
    #[arg(short, long)]
    pub rpc_url: Option<String>,

    /// SQLite database path
    #[arg(short, long)]
    pub db_path: Option<PathBuf>,

    /// Block number to start sync from (0 = from last synced or genesis)
    #[arg(short, long)]
    pub start_block: Option<u64>,

    /// Blocks to fetch per batch
    #[arg(short, long)]
    pub batch_size: Option<u32>,

    /// Only sync finalized blocks
    #[arg(long)]
    pub finalized_only: Option<bool>,

    /// Seconds between new block checks
    #[arg(long)]
    pub poll_interval: Option<u64>,

    /// Run in daemon mode (continuous sync)
    #[arg(long)]
    pub daemon: bool,

    /// PID file path for daemon mode
    #[arg(long)]
    pub pid_file: Option<PathBuf>,
}

/// Run the sync command
pub async fn run(args: SyncArgs) -> Result<()> {
    // Load configuration
    let config = crate::config::Config::load()?;

    // Use args or fall back to config
    let rpc_url = args.rpc_url.unwrap_or_else(|| config.rpc.url.clone());
    let db_path = args.db_path.unwrap_or_else(|| std::path::PathBuf::from(&config.database.path));
    let batch_size = args.batch_size.unwrap_or(config.sync.batch_size);
    let poll_interval = args.poll_interval.unwrap_or(config.sync.poll_interval_secs);
    let finalized_only = args.finalized_only.unwrap_or(config.sync.finalized_only);
    let start_block = args.start_block.unwrap_or(config.sync.start_block);

    info!("Starting block synchronization");
    info!("RPC endpoint: {}", rpc_url);
    info!("Database: {}", db_path.display());

    // Create PID file if specified
    let _pid_file = if let Some(ref pid_path) = args.pid_file {
        Some(crate::daemon::PidFile::create(pid_path)?)
    } else {
        None
    };

    // Set up signal handling for graceful shutdown
    let signals = Signals::new([SIGTERM, SIGINT, SIGQUIT])
        .context("Failed to register signal handlers")?;
    let mut signals = signals.fuse();

    if args.daemon {
        info!("Running in daemon mode");
    }

    // Open database
    let db = Database::open(&db_path)?;
    info!("Database opened successfully");

    // Connect to RPC with configured timeout and retry settings
    let rpc = RpcClient::with_config(&rpc_url, config.rpc.timeout_ms, config.rpc.retry_config());

    // Get current chain state
    let chain_tip = get_chain_tip(&rpc).await
        .context(format!("Failed to connect to node at {}.

Tip: Make sure your Midnight node is running and RPC is enabled.
     Check the RPC URL is correct: {}
     Default port is 9944 for HTTP RPC.", rpc_url, rpc_url))?;
    let finalized = get_finalized_block(&rpc).await?;
    let sidechain_status = get_sidechain_status(&rpc).await.ok();
    let mainchain_epoch = sidechain_status
        .as_ref()
        .map(|s| s.mainchain.epoch)
        .unwrap_or(0);
    let sidechain_epoch = sidechain_status
        .as_ref()
        .map(|s| s.sidechain.epoch)
        .unwrap_or(0);

    // Get chain timing from config
    let mut chain_timing = config.chain.timing();

    // If genesis timestamp not configured, calculate from current slot and time
    if chain_timing.genesis_timestamp_ms.is_none() {
        if let Some(ref status) = sidechain_status {
            let current_slot = status.sidechain.slot;
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);

            // genesis_time = now - (current_slot * slot_duration)
            let calculated_genesis = now_ms.saturating_sub(current_slot * chain_timing.slot_duration_ms);
            chain_timing.genesis_timestamp_ms = Some(calculated_genesis);

            debug!(
                "Calculated genesis timestamp: {} (from slot {} at {})",
                calculated_genesis, current_slot, now_ms
            );
        }
    }

    info!("Chain tip: {}, finalized: {}", chain_tip, finalized);
    info!("Mainchain epoch: {}, Sidechain epoch: {}", mainchain_epoch, sidechain_epoch);

    // Determine start block
    let sync_status = db.get_sync_status()?;
    let start_from = if start_block > 0 {
        start_block
    } else if sync_status.last_synced_block > 0 {
        sync_status.last_synced_block + 1
    } else {
        // Start from block 1 to sync entire chain history
        1
    };

    info!("Starting sync from block {}", start_from);
    info!("Target block: {} ({})",
        if finalized_only { finalized } else { chain_tip },
        if finalized_only { "finalized" } else { "chain tip" }
    );

    // Detect if historical state is available for author attribution
    if let Some(safe_start) = detect_safe_start_block(&rpc, chain_tip).await {
        if start_from < safe_start {
            warn!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            warn!("Historical state is pruned before block {}", safe_start);
            warn!("Blocks {} to {} will be synced WITHOUT author attribution", start_from, safe_start - 1);
            warn!("Block data will be complete, but author_key will be NULL for these blocks.");
            warn!("For full attribution history, use an archive node (--state-pruning archive)");
            warn!("See docs/BLOCK_ATTRIBUTION.md for details");
            warn!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        } else {
            info!("Historical state available - full author attribution enabled");
        }
    }

    // Initial sync: catch up to chain tip
    let mut current_block = start_from;
    let target = if finalized_only {
        finalized
    } else {
        chain_tip
    };

    let total_blocks_to_sync = if target >= start_from {
        target - start_from + 1
    } else {
        0
    };

    while current_block <= target {
        let batch_end = std::cmp::min(current_block + batch_size as u64 - 1, target);

        let synced = sync_block_range(&rpc, &db, current_block, batch_end, &chain_timing).await?;

        if synced > 0 {
            let blocks_synced_so_far = batch_end - start_from + 1;
            let progress_pct = if total_blocks_to_sync > 0 {
                (blocks_synced_so_far as f64 / total_blocks_to_sync as f64) * 100.0
            } else {
                100.0
            };

            info!(
                "Synced blocks {}-{} ({} blocks) - Progress: {:.1}% ({}/{})",
                current_block, batch_end, synced,
                progress_pct, blocks_synced_so_far, total_blocks_to_sync
            );

            // Update sync status
            db.update_sync_status(batch_end, finalized, chain_tip, mainchain_epoch, true)?;
        }

        current_block = batch_end + 1;
    }

    // Mark syncing complete
    db.update_sync_status(target, finalized, chain_tip, mainchain_epoch, false)?;

    let total_blocks = db.count_blocks()?;
    info!(
        "Initial sync complete - Progress: 100.0% ({}/{})",
        total_blocks_to_sync, total_blocks_to_sync
    );
    info!(
        "{} blocks in database (block range: {}-{})",
        total_blocks, start_from, target
    );

    // Continuous sync: poll for new blocks
    info!(
        "Sync at 100.0% - Watching for new blocks (poll interval: {}s)",
        poll_interval
    );
    let mut interval = time::interval(Duration::from_secs(poll_interval));
    let mut last_synced = target;

    loop {
        select! {
            _ = interval.tick() => {
                // Get current state
                let new_tip = match get_chain_tip(&rpc).await {
                    Ok(tip) => tip,
                    Err(e) => {
                        warn!("Failed to get chain tip: {}", e);
                        continue;
                    }
                };

                let new_finalized = match get_finalized_block(&rpc).await {
                    Ok(fin) => fin,
                    Err(e) => {
                        warn!("Failed to get finalized block: {}", e);
                        continue;
                    }
                };

                // Get current epoch (may have changed since start)
                let current_mainchain_epoch = get_sidechain_status(&rpc)
                    .await
                    .ok()
                    .map(|s| s.mainchain.epoch)
                    .unwrap_or(mainchain_epoch);

                // Update finalized status
                if new_finalized > finalized {
                    match db.mark_finalized(new_finalized) {
                        Ok(marked) => {
                            if marked > 0 {
                                debug!("Marked {} blocks as finalized", marked);
                            }
                        }
                        Err(e) => {
                            warn!("Failed to mark blocks as finalized: {}", e);
                        }
                    }
                }

                // Sync new blocks
                if new_tip > last_synced {
                    let target = if finalized_only {
                        new_finalized
                    } else {
                        new_tip
                    };

                    if target > last_synced {
                        match sync_block_range(&rpc, &db, last_synced + 1, target, &chain_timing).await {
                            Ok(synced) => {
                                if synced > 0 {
                                    // Calculate how far behind we are
                                    let blocks_behind = new_tip.saturating_sub(target);
                                    let sync_pct = if blocks_behind == 0 {
                                        100.0
                                    } else {
                                        ((target - start_from) as f64 / (new_tip - start_from) as f64) * 100.0
                                    };

                                    info!(
                                        "New block{}: {}-{} ({} synced) - Sync: {:.1}% ({} behind)",
                                        if synced > 1 { "s" } else { "" },
                                        last_synced + 1,
                                        target,
                                        synced,
                                        sync_pct,
                                        blocks_behind
                                    );
                                    last_synced = target;

                                    if let Err(e) = db.update_sync_status(target, new_finalized, new_tip, current_mainchain_epoch, false) {
                                        warn!("Failed to update sync status: {}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                warn!("Failed to sync block range {}-{}: {}", last_synced + 1, target, e);
                            }
                        }
                    } else {
                        // Fully synced - only log occasionally
                        debug!("Sync at 100.0% - No new blocks");
                    }
                }
            }
            Some(signal) = signals.next() => {
                match signal {
                    SIGTERM | SIGINT | SIGQUIT => {
                        info!("Received signal {}, initiating graceful shutdown...", signal);
                        break;
                    }
                    _ => {
                        debug!("Received unexpected signal {}", signal);
                    }
                }
            }
        }
    }

    info!("Shutting down gracefully...");
    info!("Final sync status: {} blocks synced", last_synced);
    Ok(())
}

async fn get_chain_tip(rpc: &RpcClient) -> Result<u64> {
    let header: crate::rpc::BlockHeader = rpc.call_with_retry("chain_getHeader", Vec::<()>::new()).await?;
    Ok(header.block_number())
}

async fn get_finalized_block(rpc: &RpcClient) -> Result<u64> {
    let hash: String = rpc
        .call_with_retry("chain_getFinalizedHead", Vec::<()>::new())
        .await?;
    let header: crate::rpc::BlockHeader = rpc.call_with_retry("chain_getHeader", vec![&hash]).await?;
    Ok(header.block_number())
}

async fn get_sidechain_status(rpc: &RpcClient) -> Result<SidechainStatus> {
    rpc.call_with_retry("sidechain_getStatus", Vec::<()>::new()).await
}

async fn get_sidechain_status_at_block(rpc: &RpcClient, block_hash: &str) -> Result<SidechainStatus> {
    // Query sidechain status at a specific block hash
    // Substrate RPC methods typically accept an optional block hash as the last parameter
    rpc.call_with_retry("sidechain_getStatus", vec![block_hash]).await
}

async fn get_block_hash(rpc: &RpcClient, block_number: u64) -> Result<String> {
    rpc.call_with_retry("chain_getBlockHash", vec![block_number]).await
}

async fn get_block(rpc: &RpcClient, hash: &str) -> Result<SignedBlock> {
    rpc.call_with_retry("chain_getBlock", vec![hash]).await
}

/// Detect the oldest block that has state available for committee queries.
///
/// Non-archive nodes prune historical state. This function uses binary search
/// to find the approximate boundary between pruned and available state.
///
/// Returns the oldest block number where state is available, or None if all state
/// is available (archive node) or detection failed.
async fn detect_safe_start_block(rpc: &RpcClient, chain_tip: u64) -> Option<u64> {
    // Most pruned nodes keep ~256 blocks of state
    // We'll check a few sample points to detect if pruning is in effect

    // First, check if we're on an archive node by testing an old block
    let test_old_block = chain_tip.saturating_sub(1000);
    if test_old_block == 0 {
        return None; // Chain is too short to need pruning detection
    }

    // Get hash for the test block
    let hash = match get_block_hash(rpc, test_old_block).await {
        Ok(h) => h,
        Err(_) => return None,
    };

    // Try to query committee at this old block
    match ValidatorSet::fetch_committee_at_block(rpc, Some(&hash)).await {
        Ok(_) => {
            // State is available for old blocks - likely an archive node
            debug!("Historical state available at block {} - archive node detected", test_old_block);
            None
        }
        Err(e) => {
            let err_str = format!("{:?}", e);
            if err_str.contains("State already discarded") || err_str.contains("UnknownBlock") {
                // State is pruned - do binary search to find the boundary
                debug!("State pruned at block {}, searching for safe start...", test_old_block);

                // Binary search between test_old_block and chain_tip
                let mut low = test_old_block;
                let mut high = chain_tip;

                while high - low > 10 {
                    let mid = (low + high) / 2;
                    let mid_hash = match get_block_hash(rpc, mid).await {
                        Ok(h) => h,
                        Err(_) => break,
                    };

                    match ValidatorSet::fetch_committee_at_block(rpc, Some(&mid_hash)).await {
                        Ok(_) => {
                            // State available at mid, search lower
                            high = mid;
                        }
                        Err(_) => {
                            // State pruned at mid, search higher
                            low = mid;
                        }
                    }
                }

                // Return slightly above high to ensure we're in safe territory
                Some(high + 1)
            } else {
                // Different error - can't determine pruning status
                debug!("Could not determine pruning status: {}", e);
                None
            }
        }
    }
}

/// Committee cache entry with the block hash used to fetch it
struct CommitteeCache {
    /// The validator set (committee + candidates) for this epoch
    /// If used_fallback is true, this should NOT be used for attribution
    validator_set: ValidatorSet,
    /// Block hash used to fetch this committee (for reference/debugging)
    #[allow(dead_code)]
    fetched_at_block: String,
    /// True if we had to fall back to current committee due to pruned state
    /// When true, attribution is skipped because it would be inaccurate
    used_fallback: bool,
}

/// Capture validator epoch snapshot data
///
/// Stores validator state (committee seats, registration status) for a sidechain epoch.
/// This is called when we first encounter a new sidechain epoch during sync.
fn capture_validator_epoch_snapshot(
    db: &Database,
    sidechain_epoch: u64,
    validator_set: &ValidatorSet,
) -> Result<()> {
    let committee_size = validator_set.committee_size() as u32;
    let timestamp = chrono::Utc::now().timestamp();

    let mut captured_count = 0;

    for candidate in &validator_set.candidates {
        let seats = validator_set.count_seats(&candidate.aura_key) as u32;

        let record = ValidatorEpochRecord {
            sidechain_epoch,
            sidechain_key: candidate.sidechain_key.clone(),
            aura_key: candidate.aura_key.clone(),
            committee_seats: seats,
            committee_size,
            is_permissioned: candidate.is_permissioned,
            stake_lovelace: candidate.stake_lovelace,
            captured_at: timestamp,
        };

        if let Err(e) = db.store_validator_epoch(&record) {
            warn!(
                "Failed to store validator epoch data for {} in epoch {}: {}",
                candidate.sidechain_key, sidechain_epoch, e
            );
        } else {
            captured_count += 1;
        }
    }

    info!(
        "Captured validator epoch snapshot for epoch {} ({} validators, {} committee seats)",
        sidechain_epoch, captured_count, committee_size
    );

    Ok(())
}

async fn sync_block_range(
    rpc: &RpcClient,
    db: &Database,
    from: u64,
    to: u64,
    chain_timing: &ChainTiming,
) -> Result<u64> {
    let mut synced = 0;

    // Cache committees per epoch (multiple epochs may exist in a range)
    // We store (validator_set, block_hash_used) so we can fetch the committee
    // at the correct historical point for each epoch
    let mut committee_cache: HashMap<u64, CommitteeCache> = HashMap::new();

    // Track sidechain epochs where we've captured validator epoch snapshots
    // Initialize with epochs already in database to avoid duplicates
    let mut captured_sidechain_epochs: HashSet<u64> = HashSet::new();

    for block_num in from..=to {
        match sync_single_block(rpc, db, block_num, &mut committee_cache, &mut captured_sidechain_epochs, chain_timing).await {
            Ok(true) => synced += 1,
            Ok(false) => {
                debug!("Block {} already exists, skipping", block_num);
            }
            Err(e) => {
                warn!("Failed to sync block {}: {}", block_num, e);
                // Continue with next block
            }
        }
    }

    Ok(synced)
}

async fn sync_single_block(
    rpc: &RpcClient,
    db: &Database,
    block_number: u64,
    committee_cache: &mut HashMap<u64, CommitteeCache>,
    captured_sidechain_epochs: &mut HashSet<u64>,
    chain_timing: &ChainTiming,
) -> Result<bool> {
    // Check if already synced
    if db.get_block(block_number)?.is_some() {
        return Ok(false);
    }

    // Fetch block
    let hash = get_block_hash(rpc, block_number)
        .await
        .with_context(|| format!("Failed to get hash for block {}", block_number))?;

    let signed_block = get_block(rpc, &hash)
        .await
        .with_context(|| format!("Failed to get block {}", block_number))?;

    let header = &signed_block.block.header;

    // Determine the actual epochs for this block by querying at the block hash
    let (mainchain_epoch, sidechain_epoch) = match get_sidechain_status_at_block(rpc, &hash).await {
        Ok(status) => {
            debug!(
                "Block {} is from mainchain epoch {}, sidechain epoch {}",
                block_number, status.mainchain.epoch, status.sidechain.epoch
            );
            (status.mainchain.epoch, status.sidechain.epoch)
        }
        Err(e) => {
            warn!(
                "Failed to get epoch for block {} (hash {}): {}. Using epoch 0.",
                block_number, hash, e
            );
            (0, 0)
        }
    };

    // Fetch or retrieve cached committee for this epoch
    // IMPORTANT: When fetching the committee, we must query at a block hash from
    // that epoch to get the correct historical committee (committees change each epoch)
    //
    // If historical state is pruned, we skip attribution entirely rather than
    // attribute incorrectly using the current committee. See docs/BLOCK_ATTRIBUTION.md
    let validator_set = if let Some(cached) = committee_cache.get(&mainchain_epoch) {
        // Already cached - check if this was a fallback (pruned state)
        if cached.used_fallback {
            // State was pruned for this epoch - skip attribution
            None
        } else {
            // Historical state was available - use for accurate attribution
            Some(&cached.validator_set)
        }
    } else if mainchain_epoch > 0 {
        // Fetch and cache committee for this epoch AT THIS BLOCK HASH
        // This ensures we get the committee that was active when this block was produced
        match ValidatorSet::fetch_with_committee_or_fallback(rpc, mainchain_epoch, &hash).await {
            Ok((vs, used_fallback)) => {
                if used_fallback {
                    // Historical state is pruned - log warning and skip attribution
                    warn!(
                        "Historical state pruned for epoch {} - blocks will be stored without author attribution. \
                         See docs/BLOCK_ATTRIBUTION.md for details.",
                        mainchain_epoch
                    );
                    // Don't store committee snapshot - it would be inaccurate for this epoch
                } else {
                    debug!(
                        "Fetched validator set for epoch {} at block {} ({} candidates, {} committee seats)",
                        mainchain_epoch,
                        hash,
                        vs.candidate_count(),
                        vs.committee_size()
                    );
                    // Only store committee snapshot when we have accurate historical data
                    if let Err(e) = db.store_committee_snapshot(mainchain_epoch, &vs.committee) {
                        warn!(
                            "Failed to store committee snapshot for epoch {}: {}",
                            mainchain_epoch, e
                        );
                    } else {
                        debug!("Stored committee snapshot for epoch {}", mainchain_epoch);
                    }
                }

                committee_cache.insert(
                    mainchain_epoch,
                    CommitteeCache {
                        validator_set: vs,
                        fetched_at_block: hash.clone(),
                        used_fallback,
                    },
                );

                // Only return validator set for attribution if state wasn't pruned
                if used_fallback {
                    None
                } else {
                    committee_cache.get(&mainchain_epoch).map(|c| &c.validator_set)
                }
            }
            Err(e) => {
                warn!(
                    "Failed to fetch validator set for epoch {} at block {}: {}. Author attribution will be skipped.",
                    mainchain_epoch, hash, e
                );
                None
            }
        }
    } else {
        None
    };

    // Capture validator epoch snapshot for this sidechain epoch if:
    // 1. We have valid validator data (not pruned state)
    // 2. We haven't already captured this sidechain epoch
    // 3. The epoch is valid (> 0)
    if let Some(vset) = &validator_set {
        if sidechain_epoch > 0 && !captured_sidechain_epochs.contains(&sidechain_epoch) {
            // Check if already in database
            let already_captured = db.has_validator_epoch_snapshot(sidechain_epoch)
                .unwrap_or(false);

            if !already_captured {
                if let Err(e) = capture_validator_epoch_snapshot(db, sidechain_epoch, vset) {
                    warn!(
                        "Failed to capture validator epoch snapshot for epoch {}: {}",
                        sidechain_epoch, e
                    );
                }
            }
            captured_sidechain_epochs.insert(sidechain_epoch);
        }
    }

    // Extract slot from digest
    let slot = header
        .digest
        .as_ref()
        .and_then(|d| extract_slot_from_digest(&d.logs))
        .unwrap_or(0);

    // Calculate block author from slot and validator set
    let author_key = if let Some(vset) = validator_set {
        if slot > 0 {
            if let Some(validator) = vset.get_author(slot) {
                // Determine registration status
                let registration_status = if validator.is_permissioned {
                    Some("permissioned".to_string())
                } else {
                    Some("registered".to_string())
                };

                // Upsert validator record and increment block count
                let validator_record = ValidatorRecord {
                    sidechain_key: validator.sidechain_key.clone(),
                    aura_key: Some(validator.aura_key.clone()),
                    grandpa_key: Some(validator.grandpa_key.clone()),
                    label: None,
                    is_ours: false, // Will be set by keys command
                    registration_status,
                    first_seen_epoch: Some(mainchain_epoch),
                    total_blocks: 0, // Will be incremented by database
                };

                // Upsert validator (insert or update)
                if let Err(e) = db.upsert_validator(&validator_record) {
                    warn!(
                        "Failed to upsert validator {}: {}",
                        validator.sidechain_key, e
                    );
                }

                // Increment block count
                if let Err(e) = db.increment_block_count(&validator.sidechain_key) {
                    warn!(
                        "Failed to increment block count for validator {}: {}",
                        validator.sidechain_key, e
                    );
                }

                debug!(
                    "Block {} authored by validator {} (slot {} % {} committee seats)",
                    block_number,
                    validator.sidechain_key,
                    slot,
                    vset.committee_size()
                );

                Some(validator.sidechain_key.clone())
            } else {
                warn!(
                    "Failed to get author for block {} (slot {}): validator set is empty or invalid",
                    block_number, slot
                );
                None
            }
        } else {
            debug!("Block {} has no slot number, cannot determine author", block_number);
            None
        }
    } else {
        None
    };

    // Calculate timestamp from slot number (if genesis is known)
    // This gives accurate historical timestamps rather than using sync time
    let timestamp = if slot > 0 {
        chain_timing
            .slot_to_timestamp_ms(slot)
            .map(|ms| (ms / 1000) as i64)
            .unwrap_or_else(|| chrono::Utc::now().timestamp())
    } else {
        chrono::Utc::now().timestamp()
    };

    // Create block record
    let record = BlockRecord {
        block_number,
        block_hash: hash,
        parent_hash: header.parent_hash.clone(),
        state_root: header.state_root.clone(),
        extrinsics_root: header.extrinsics_root.clone(),
        slot_number: slot,
        epoch: mainchain_epoch,
        sidechain_epoch,
        timestamp,
        is_finalized: false,
        author_key,
        extrinsics_count: signed_block.block.extrinsics.len() as u32,
    };

    db.insert_block(&record)?;
    Ok(true)
}
