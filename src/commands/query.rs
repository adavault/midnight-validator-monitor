//! Query command - query stored block data

use crate::db::Database;
use anyhow::Result;
use clap::{Args, Subcommand};
use std::path::PathBuf;
use tracing::info;

/// Query command arguments
#[derive(Args, Debug)]
pub struct QueryArgs {
    /// SQLite database path
    #[arg(short, long, default_value = "./mvm.db")]
    pub db_path: PathBuf,

    #[command(subcommand)]
    pub command: QueryCommands,
}

#[derive(Subcommand, Debug)]
pub enum QueryCommands {
    /// Show database statistics
    Stats,

    /// List blocks in a range
    Blocks {
        /// Start block number
        #[arg(short, long)]
        from: Option<u64>,

        /// End block number
        #[arg(short, long)]
        to: Option<u64>,

        /// Number of blocks to show (default: 10)
        #[arg(short, long, default_value_t = 10)]
        limit: u64,
    },

    /// Find gaps in synced blocks
    Gaps,
}

/// Run the query command
pub async fn run(args: QueryArgs) -> Result<()> {
    let db = Database::open(&args.db_path)?;

    match args.command {
        QueryCommands::Stats => run_stats(&db)?,
        QueryCommands::Blocks { from, to, limit } => run_blocks(&db, from, to, limit)?,
        QueryCommands::Gaps => run_gaps(&db)?,
    }

    Ok(())
}

fn run_stats(db: &Database) -> Result<()> {
    let total_blocks = db.count_blocks()?;
    let finalized_blocks = db.count_finalized_blocks()?;
    let sync_status = db.get_sync_status()?;

    info!("Database Statistics");
    info!("─────────────────────────────────────────");
    info!("Total blocks:     {}", total_blocks);
    info!("Finalized blocks: {}", finalized_blocks);
    info!("Unfinalized:      {}", total_blocks - finalized_blocks);

    if let Some(min) = db.get_block(sync_status.last_synced_block)? {
        info!("─────────────────────────────────────────");
        info!("Latest synced:    Block #{}", min.block_number);
        info!("  Slot:           {}", min.slot_number);
        info!("  Epoch:          {}", min.epoch);
    }

    if let Some(max_block) = db.get_max_block_number()? {
        if let Some(min_block) = db.get_blocks_in_range(max_block.saturating_sub(1000), max_block, Some(1))?
            .first()
        {
            info!("─────────────────────────────────────────");
            info!("Block range:      {} - {}", min_block.block_number, max_block);
        }
    }

    // Check for gaps
    let gaps = db.find_gaps()?;
    if gaps.is_empty() {
        info!("Gaps:             None (continuous)");
    } else {
        info!("Gaps:             {} gap(s) detected", gaps.len());
    }

    Ok(())
}

fn run_blocks(db: &Database, from: Option<u64>, to: Option<u64>, limit: u64) -> Result<()> {
    let (start, end) = match (from, to) {
        (Some(f), Some(t)) => (f, t),
        (Some(f), None) => (f, f + limit - 1),
        (None, Some(t)) => (t.saturating_sub(limit - 1), t),
        (None, None) => {
            // Show most recent blocks
            let max = db.get_max_block_number()?.unwrap_or(0);
            (max.saturating_sub(limit - 1), max)
        }
    };

    let blocks = db.get_blocks_in_range(start, end, Some(limit as u32 + 1))?;

    if blocks.is_empty() {
        info!("No blocks found in range {} - {}", start, end);
        return Ok(());
    }

    info!("Blocks {} - {} ({} found)", start, end, blocks.len());
    info!("─────────────────────────────────────────────────────────────────────────────");
    info!(
        "{:>10} {:>12} {:>8} {:>12} {:>6} {:>10}",
        "Block", "Slot", "Epoch", "Extrinsics", "Final", "Hash"
    );
    info!("─────────────────────────────────────────────────────────────────────────────");

    for block in blocks.iter().take(limit as usize) {
        let finalized = if block.is_finalized { "✓" } else { "" };
        let hash_short = if block.block_hash.len() > 12 {
            &block.block_hash[..12]
        } else {
            &block.block_hash
        };
        info!(
            "{:>10} {:>12} {:>8} {:>12} {:>6} {}...",
            block.block_number,
            block.slot_number,
            block.epoch,
            block.extrinsics_count,
            finalized,
            hash_short
        );
    }

    if blocks.len() > limit as usize {
        info!("... and {} more", blocks.len() - limit as usize);
    }

    Ok(())
}

fn run_gaps(db: &Database) -> Result<()> {
    let gaps = db.find_gaps()?;

    if gaps.is_empty() {
        info!("No gaps found - block data is continuous");
        return Ok(());
    }

    info!("Found {} gap(s) in block data:", gaps.len());
    info!("─────────────────────────────────────────");
    info!("{:>12} {:>12} {:>12}", "From", "To", "Missing");
    info!("─────────────────────────────────────────");

    let mut total_missing = 0u64;
    for (start, end) in &gaps {
        let missing = end - start - 1;
        total_missing += missing;
        info!("{:>12} {:>12} {:>12}", start, end, missing);
    }

    info!("─────────────────────────────────────────");
    info!("Total missing blocks: {}", total_missing);
    info!("");
    info!("To resync gaps, run: mvm sync --start-block <from>");

    Ok(())
}
