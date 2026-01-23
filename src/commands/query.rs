//! Query command - query stored block data

use crate::db::Database;
use anyhow::{bail, Result};
use clap::{Args, Subcommand};
use std::path::PathBuf;
use tracing::info;

/// Query command arguments
#[derive(Args, Debug)]
pub struct QueryArgs {
    /// SQLite database path
    #[arg(short, long)]
    pub db_path: Option<PathBuf>,

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

    /// List all validators with statistics
    Validators {
        /// Only show our validators
        #[arg(long)]
        ours: bool,

        /// Number of validators to show (default: 20)
        #[arg(short, long, default_value_t = 20)]
        limit: u64,
    },

    /// Show details for a specific validator
    Validator {
        /// Validator sidechain public key
        key: String,
    },

    /// Show block production performance statistics
    Performance {
        /// Only show our validators
        #[arg(long)]
        ours: bool,

        /// Number of validators to show (default: 10)
        #[arg(short, long, default_value_t = 10)]
        limit: u64,
    },
}

/// Run the query command
pub async fn run(args: QueryArgs) -> Result<()> {
    // Load configuration
    let config = crate::config::Config::load()?;

    // Use args or fall back to config
    let db_path = args
        .db_path
        .unwrap_or_else(|| std::path::PathBuf::from(&config.database.path));

    let db = Database::open(&db_path)?;

    match args.command {
        QueryCommands::Stats => run_stats(&db)?,
        QueryCommands::Blocks { from, to, limit } => run_blocks(&db, from, to, limit)?,
        QueryCommands::Gaps => run_gaps(&db)?,
        QueryCommands::Validators { ours, limit } => run_validators(&db, ours, limit)?,
        QueryCommands::Validator { key } => run_validator(&db, &key)?,
        QueryCommands::Performance { ours, limit } => run_performance(&db, ours, limit)?,
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
        if let Some(min_block) = db
            .get_blocks_in_range(max_block.saturating_sub(1000), max_block, Some(1))?
            .first()
        {
            info!("─────────────────────────────────────────");
            info!(
                "Block range:      {} - {}",
                min_block.block_number, max_block
            );
        }
    }

    // Check for gaps
    let gaps = db.find_gaps()?;
    if gaps.is_empty() {
        info!("Gaps:             None (continuous)");
    } else {
        info!("Gaps:             {} gap(s) detected", gaps.len());
    }

    // Validator statistics
    let total_validators = db.count_validators()?;
    let our_validators = db.count_our_validators()?;

    if total_validators > 0 {
        info!("─────────────────────────────────────────");
        info!("Validators:       {}", total_validators);
        if our_validators > 0 {
            info!("  Ours:           {}", our_validators);
        }

        // Show blocks with author attribution
        let blocks_with_authors = db
            .get_blocks_in_range(0, u64::MAX, None)?
            .iter()
            .filter(|b| b.author_key.is_some())
            .count();

        if blocks_with_authors > 0 {
            let attribution_pct = (blocks_with_authors as f64 / total_blocks as f64) * 100.0;
            info!(
                "  Blocks with author: {} ({:.1}%)",
                blocks_with_authors, attribution_pct
            );
        }
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

fn run_validators(db: &Database, ours_only: bool, limit: u64) -> Result<()> {
    let validators = if ours_only {
        db.get_our_validators()?
    } else {
        db.get_all_validators()?
    };

    if validators.is_empty() {
        if ours_only {
            info!("No validators marked as ours.");
            info!("Run 'mvm keys verify' to mark your validators.");
        } else {
            info!("No validators found in database.");
            info!("Run 'mvm sync' to populate validator data.");
        }
        return Ok(());
    }

    let title = if ours_only {
        format!("Our Validators ({} total)", validators.len())
    } else {
        format!("All Validators ({} total)", validators.len())
    };

    info!("{}", title);
    info!("─────────────────────────────────────────────────────────────────────────────────────");
    info!("{:>68} {:>15} {:>12}", "Sidechain Key", "Status", "Blocks");
    info!("─────────────────────────────────────────────────────────────────────────────────────");

    for validator in validators.iter().take(limit as usize) {
        let status = validator
            .registration_status
            .as_deref()
            .unwrap_or("unknown");
        let ours_marker = if validator.is_ours { " *" } else { "" };

        info!(
            "{:>68} {:>15} {:>12}{}",
            validator.sidechain_key, status, validator.total_blocks, ours_marker
        );
    }

    if validators.len() > limit as usize {
        info!("");
        info!(
            "... and {} more (use --limit to show more)",
            validators.len() - limit as usize
        );
    }

    if !ours_only && validators.iter().any(|v| v.is_ours) {
        info!("");
        info!("* = Our validator");
    }

    Ok(())
}

fn run_validator(db: &Database, key: &str) -> Result<()> {
    // Normalize the key
    let normalized_key = if key.starts_with("0x") {
        key.to_lowercase()
    } else {
        format!("0x{}", key.to_lowercase())
    };

    let validator = db.get_validator(&normalized_key)?;

    match validator {
        Some(v) => {
            info!("Validator Details");
            info!("─────────────────────────────────────────────────────────────────");
            info!("Sidechain Key:  {}", v.sidechain_key);
            if let Some(aura) = &v.aura_key {
                info!("AURA Key:       {}", aura);
            }
            if let Some(grandpa) = &v.grandpa_key {
                info!("Grandpa Key:    {}", grandpa);
            }
            if let Some(label) = &v.label {
                info!("Label:          {}", label);
            }
            info!(
                "Status:         {}",
                v.registration_status
                    .as_ref()
                    .unwrap_or(&"unknown".to_string())
            );
            info!("Is Ours:        {}", if v.is_ours { "Yes" } else { "No" });
            if let Some(epoch) = v.first_seen_epoch {
                info!("First Seen:     Epoch {}", epoch);
            }
            info!("Blocks Produced: {}", v.total_blocks);

            // Get recent blocks by this validator
            info!("");
            info!("Recent blocks produced by this validator:");
            let total_blocks = db.count_blocks()?;
            if total_blocks > 0 {
                let max_block = db.get_max_block_number()?.unwrap_or(0);
                let start = max_block.saturating_sub(1000);
                let blocks = db.get_blocks_in_range(start, max_block, Some(1000))?;

                let validator_blocks: Vec<_> = blocks
                    .iter()
                    .filter(|b| {
                        b.author_key
                            .as_ref()
                            .map(|k| k == &v.sidechain_key)
                            .unwrap_or(false)
                    })
                    .take(5)
                    .collect();

                if validator_blocks.is_empty() {
                    info!("  No recent blocks found (checked last 1000 blocks)");
                } else {
                    for block in validator_blocks {
                        info!(
                            "  Block #{} (slot {}, epoch {})",
                            block.block_number, block.slot_number, block.epoch
                        );
                    }
                }
            }

            Ok(())
        }
        None => {
            bail!("Validator not found: {}", normalized_key);
        }
    }
}

fn run_performance(db: &Database, ours_only: bool, limit: u64) -> Result<()> {
    let validators = if ours_only {
        db.get_our_validators()?
    } else {
        db.get_all_validators()?
    };

    if validators.is_empty() {
        if ours_only {
            info!("No validators marked as ours.");
            info!("Run 'mvm keys verify' to mark your validators.");
        } else {
            info!("No validators found in database.");
            info!("Run 'mvm sync' to populate validator data.");
        }
        return Ok(());
    }

    // Filter validators with at least 1 block
    let active_validators: Vec<_> = validators.iter().filter(|v| v.total_blocks > 0).collect();

    if active_validators.is_empty() {
        info!("No validators have produced blocks yet.");
        return Ok(());
    }

    let total_blocks: u64 = active_validators.iter().map(|v| v.total_blocks).sum();
    let avg_blocks = if !active_validators.is_empty() {
        total_blocks as f64 / active_validators.len() as f64
    } else {
        0.0
    };

    let title = if ours_only {
        "Our Validator Performance"
    } else {
        "Validator Performance"
    };

    info!("{}", title);
    info!("─────────────────────────────────────────────────────────────────────────────────────");
    info!("Active validators: {}", active_validators.len());
    info!("Total blocks:      {}", total_blocks);
    info!("Average blocks:    {:.2}", avg_blocks);
    info!("─────────────────────────────────────────────────────────────────────────────────────");
    info!(
        "{:>4} {:>68} {:>12} {:>8}",
        "Rank", "Sidechain Key", "Blocks", "Share %"
    );
    info!("─────────────────────────────────────────────────────────────────────────────────────");

    for (i, validator) in active_validators.iter().take(limit as usize).enumerate() {
        let share = if total_blocks > 0 {
            (validator.total_blocks as f64 / total_blocks as f64) * 100.0
        } else {
            0.0
        };
        let ours_marker = if validator.is_ours { " *" } else { "" };

        info!(
            "{:>4} {:>68} {:>12} {:>7.2}%{}",
            i + 1,
            validator.sidechain_key,
            validator.total_blocks,
            share,
            ours_marker
        );
    }

    if active_validators.len() > limit as usize {
        info!("");
        info!(
            "... and {} more (use --limit to show more)",
            active_validators.len() - limit as usize
        );
    }

    if !ours_only && validators.iter().any(|v| v.is_ours) {
        info!("");
        info!("* = Our validator");
    }

    Ok(())
}
