//! Sync command - synchronize blocks to local database

use crate::db::{BlockRecord, Database, ValidatorRecord};
use crate::midnight::{extract_slot_from_digest, ValidatorSet};
use crate::rpc::{RpcClient, SignedBlock, SidechainStatus};
use anyhow::{Context, Result};
use clap::Args;
use std::path::PathBuf;
use std::time::Duration;
use tokio::time;
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
}

/// Run the sync command
pub async fn run(args: SyncArgs) -> Result<()> {
    info!("Starting block synchronization");
    info!("RPC endpoint: {}", args.rpc_url);
    info!("Database: {}", args.db_path.display());

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

        let synced = sync_block_range(&rpc, &db, current_block, batch_end, current_epoch).await?;

        if synced > 0 {
            info!(
                "Synced blocks {}-{} ({} blocks)",
                current_block, batch_end, synced
            );

            // Update sync status
            db.update_sync_status(batch_end, finalized, chain_tip, current_epoch, true)?;
        }

        current_block = batch_end + 1;
    }

    // Mark syncing complete
    db.update_sync_status(target, finalized, chain_tip, current_epoch, false)?;

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
        interval.tick().await;

        // Get current state
        let new_tip = get_chain_tip(&rpc).await?;
        let new_finalized = get_finalized_block(&rpc).await?;

        // Update finalized status
        if new_finalized > finalized {
            let marked = db.mark_finalized(new_finalized)?;
            if marked > 0 {
                debug!("Marked {} blocks as finalized", marked);
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
                let synced =
                    sync_block_range(&rpc, &db, last_synced + 1, target, current_epoch).await?;

                if synced > 0 {
                    info!(
                        "New block{}: {}-{} ({} synced)",
                        if synced > 1 { "s" } else { "" },
                        last_synced + 1,
                        target,
                        synced
                    );
                    last_synced = target;

                    db.update_sync_status(target, new_finalized, new_tip, current_epoch, false)?;
                }
            }
        }
    }
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
    epoch: u64,
) -> Result<u64> {
    let mut synced = 0;

    for block_num in from..=to {
        match sync_single_block(rpc, db, block_num, epoch).await {
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
    epoch: u64,
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

    // Create block record
    let record = BlockRecord {
        block_number,
        block_hash: hash,
        parent_hash: header.parent_hash.clone(),
        state_root: header.state_root.clone(),
        extrinsics_root: header.extrinsics_root.clone(),
        slot_number: slot,
        epoch,
        timestamp: chrono::Utc::now().timestamp(), // TODO: extract from extrinsics
        is_finalized: false,
        author_key: None, // TODO: calculate from slot % validator_count
        extrinsics_count: signed_block.block.extrinsics.len() as u32,
    };

    db.insert_block(&record)?;
    Ok(true)
}
