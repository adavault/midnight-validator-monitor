//! Sync command - synchronize blocks to local database

use crate::db::{BlockRecord, Database, ValidatorRecord};
use crate::midnight::{extract_slot_from_digest, ValidatorSet};
use crate::rpc::{RpcClient, SignedBlock, SidechainStatus};
use anyhow::{Context, Result};
use clap::Args;
use signal_hook::consts::signal::*;
use signal_hook_tokio::Signals;
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
    #[arg(short, long, default_value = "http://localhost:9944")]
    pub rpc_url: String,

    /// SQLite database path
    #[arg(short, long, default_value = "./mvm.db")]
    pub db_path: PathBuf,

    /// Block number to start sync from (0 = from last synced or genesis)
    #[arg(short, long, default_value_t = 0)]
    pub start_block: u64,

    /// Blocks to fetch per batch
    #[arg(short, long, default_value_t = 100)]
    pub batch_size: u32,

    /// Only sync finalized blocks
    #[arg(long)]
    pub finalized_only: bool,

    /// Seconds between new block checks
    #[arg(long, default_value_t = 6)]
    pub poll_interval: u64,

    /// Run in daemon mode (continuous sync)
    #[arg(long)]
    pub daemon: bool,

    /// PID file path for daemon mode
    #[arg(long)]
    pub pid_file: Option<PathBuf>,
}

/// Run the sync command
pub async fn run(args: SyncArgs) -> Result<()> {
    info!("Starting block synchronization");
    info!("RPC endpoint: {}", args.rpc_url);
    info!("Database: {}", args.db_path.display());

    // Create PID file if specified
    let _pid_file = if let Some(ref pid_path) = args.pid_file {
        Some(crate::daemon::PidFile::create(pid_path)?)
    } else {
        None
    };

    // Set up signal handling for graceful shutdown
    let signals = Signals::new(&[SIGTERM, SIGINT, SIGQUIT])
        .context("Failed to register signal handlers")?;
    let mut signals = signals.fuse();

    if args.daemon {
        info!("Running in daemon mode");
    }

    // Open database
    let db = Database::open(&args.db_path)?;
    info!("Database opened successfully");

    // Connect to RPC
    let rpc = RpcClient::new(&args.rpc_url);

    // Get current chain state
    let chain_tip = get_chain_tip(&rpc).await?;
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

    info!("Chain tip: {}, finalized: {}", chain_tip, finalized);
    info!("Mainchain epoch: {}, Sidechain epoch: {}", mainchain_epoch, sidechain_epoch);

    // Determine start block
    let sync_status = db.get_sync_status()?;
    let start_from = if args.start_block > 0 {
        args.start_block
    } else if sync_status.last_synced_block > 0 {
        sync_status.last_synced_block + 1
    } else {
        // Start from a recent block rather than genesis (too many blocks)
        chain_tip.saturating_sub(1000)
    };

    info!("Starting sync from block {}", start_from);

    // Initial sync: catch up to chain tip
    let mut current_block = start_from;
    let target = if args.finalized_only {
        finalized
    } else {
        chain_tip
    };

    while current_block <= target {
        let batch_end = std::cmp::min(current_block + args.batch_size as u64 - 1, target);

        let synced = sync_block_range(&rpc, &db, current_block, batch_end, mainchain_epoch).await?;

        if synced > 0 {
            info!(
                "Synced blocks {}-{} ({} blocks)",
                current_block, batch_end, synced
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
        "Initial sync complete. {} blocks in database",
        total_blocks
    );

    // Continuous sync: poll for new blocks
    info!(
        "Watching for new blocks (poll interval: {}s)",
        args.poll_interval
    );
    let mut interval = time::interval(Duration::from_secs(args.poll_interval));
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
                    let target = if args.finalized_only {
                        new_finalized
                    } else {
                        new_tip
                    };

                    if target > last_synced {
                        match sync_block_range(&rpc, &db, last_synced + 1, target, current_mainchain_epoch).await {
                            Ok(synced) => {
                                if synced > 0 {
                                    info!(
                                        "New block{}: {}-{} ({} synced)",
                                        if synced > 1 { "s" } else { "" },
                                        last_synced + 1,
                                        target,
                                        synced
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
    let header: crate::rpc::BlockHeader = rpc.call("chain_getHeader", Vec::<()>::new()).await?;
    Ok(header.block_number())
}

async fn get_finalized_block(rpc: &RpcClient) -> Result<u64> {
    let hash: String = rpc
        .call("chain_getFinalizedHead", Vec::<()>::new())
        .await?;
    let header: crate::rpc::BlockHeader = rpc.call("chain_getHeader", vec![&hash]).await?;
    Ok(header.block_number())
}

async fn get_sidechain_status(rpc: &RpcClient) -> Result<SidechainStatus> {
    rpc.call("sidechain_getStatus", Vec::<()>::new()).await
}

async fn get_block_hash(rpc: &RpcClient, block_number: u64) -> Result<String> {
    rpc.call("chain_getBlockHash", vec![block_number]).await
}

async fn get_block(rpc: &RpcClient, hash: &str) -> Result<SignedBlock> {
    rpc.call("chain_getBlock", vec![hash]).await
}

async fn sync_block_range(
    rpc: &RpcClient,
    db: &Database,
    from: u64,
    to: u64,
    mainchain_epoch: u64,
) -> Result<u64> {
    let mut synced = 0;

    // Fetch validator set for this epoch (cached for the batch)
    let validator_set = match ValidatorSet::fetch(rpc, mainchain_epoch).await {
        Ok(vs) => {
            debug!(
                "Fetched validator set for epoch {} ({} validators)",
                mainchain_epoch,
                vs.validators.len()
            );
            Some(vs)
        }
        Err(e) => {
            warn!(
                "Failed to fetch validator set for epoch {}: {}. Author attribution will be skipped.",
                mainchain_epoch, e
            );
            None
        }
    };

    for block_num in from..=to {
        match sync_single_block(rpc, db, block_num, mainchain_epoch, validator_set.as_ref()).await {
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
    mainchain_epoch: u64,
    validator_set: Option<&ValidatorSet>,
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
                    "Block {} authored by validator {} (slot {} % {} validators)",
                    block_number,
                    validator.sidechain_key,
                    slot,
                    vset.validators.len()
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

    // Create block record
    let record = BlockRecord {
        block_number,
        block_hash: hash,
        parent_hash: header.parent_hash.clone(),
        state_root: header.state_root.clone(),
        extrinsics_root: header.extrinsics_root.clone(),
        slot_number: slot,
        epoch: mainchain_epoch,
        timestamp: chrono::Utc::now().timestamp(), // TODO: extract from extrinsics
        is_finalized: false,
        author_key,
        extrinsics_count: signed_block.block.extrinsics.len() as u32,
    };

    db.insert_block(&record)?;
    Ok(true)
}
