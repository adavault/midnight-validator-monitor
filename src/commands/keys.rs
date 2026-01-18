//! Keys command - verify and manage session keys

use crate::db::{Database, ValidatorRecord};
use crate::midnight::{get_key_status, ValidatorKeys};
use crate::rpc::RpcClient;
use anyhow::Result;
use clap::{Args, Subcommand};
use std::path::PathBuf;
use tracing::{debug, error, info, warn};

/// Keys command arguments
#[derive(Args, Debug)]
pub struct KeysArgs {
    /// Path to Substrate keystore directory
    #[arg(short = 'K', long)]
    pub keystore: Option<PathBuf>,

    /// Validator node RPC endpoint URL (for verification)
    #[arg(short, long)]
    pub rpc_url: Option<String>,

    /// SQLite database path (for marking validators and showing stats)
    #[arg(short, long)]
    pub db_path: Option<PathBuf>,

    #[command(subcommand)]
    pub command: KeysCommands,
}

#[derive(Subcommand, Debug)]
pub enum KeysCommands {
    /// Display keys from keystore
    Show,

    /// Verify keys are loaded in node and registered
    Verify,
}

/// Run the keys command
pub async fn run(args: KeysArgs) -> Result<()> {
    // Load configuration
    let config = crate::config::Config::load()?;

    info!("Config database.path = {}", config.database.path);

    // Get keystore path from args or config
    let keystore_path = match args.keystore.or_else(|| config.validator.keystore_path.map(PathBuf::from)) {
        Some(path) => path,
        None => {
            error!("No keystore path provided");
            error!("Use --keystore flag or set validator.keystore_path in config");
            anyhow::bail!("Keystore path required");
        }
    };

    // Get RPC URL and database path from args or config
    let rpc_url = args.rpc_url.unwrap_or(config.rpc.url);
    let db_path = args.db_path.unwrap_or_else(|| std::path::PathBuf::from(&config.database.path));

    info!("db_path resolved to: {}", db_path.display());

    // Load keys from keystore
    let keys = match ValidatorKeys::from_keystore(&keystore_path) {
        Ok(k) => k,
        Err(e) => {
            error!("Failed to load keys from keystore: {}", e);
            error!("Path: {}", keystore_path.display());
            return Err(e);
        }
    };

    match args.command {
        KeysCommands::Show => run_show(&keys),
        KeysCommands::Verify => run_verify(&keys, &rpc_url, &db_path, config.rpc.timeout_ms).await,
    }
}

fn run_show(keys: &ValidatorKeys) -> Result<()> {
    info!("Validator Keys");
    info!("─────────────────────────────────────────────────────────────────────────────");
    info!("");
    info!("Sidechain (sdch):");
    info!("  {}", keys.sidechain_pub_key);
    info!("");
    info!("Aura (aura):");
    info!("  {}", keys.aura_pub_key);
    info!("");
    info!("Grandpa (gran):");
    info!("  {}", keys.grandpa_pub_key);
    info!("");
    info!("─────────────────────────────────────────────────────────────────────────────");

    Ok(())
}

async fn run_verify(keys: &ValidatorKeys, rpc_url: &str, db_path: &PathBuf, timeout_ms: u64) -> Result<()> {
    info!("Verifying validator keys...");
    info!("RPC endpoint: {}", rpc_url);
    info!("─────────────────────────────────────────────────────────────────────────────");

    let rpc = RpcClient::with_timeout(rpc_url, timeout_ms);

    // Get current epoch from sidechain status
    let current_epoch = match rpc
        .call::<_, crate::rpc::SidechainStatus>("sidechain_getStatus", Vec::<()>::new())
        .await
    {
        Ok(status) => {
            info!("Current mainchain epoch: {}", status.mainchain.epoch);
            status.mainchain.epoch
        }
        Err(e) => {
            warn!("Could not get sidechain status: {}", e);
            0
        }
    };

    // Get key status
    let key_status = get_key_status(&rpc, keys, current_epoch).await;

    // Try to open database for marking validator and showing stats
    info!("Opening database: {}", db_path.display());
    let db = match Database::open(db_path) {
        Ok(db) => {
            info!("Database opened successfully");
            Some(db)
        }
        Err(e) => {
            warn!("Could not open database at {}: {}", db_path.display(), e);
            warn!("Block production statistics will not be available");
            None
        }
    };

    info!("");
    info!("Key Status:");
    info!("─────────────────────────────────────────────────────────────────────────────");

    // Sidechain key
    let sc_status = match key_status.sidechain_loaded {
        Some(true) => ("✓", "Loaded in keystore"),
        Some(false) => ("✗", "NOT LOADED"),
        None => ("?", "Could not verify"),
    };
    info!(
        "  Sidechain: {} {}",
        sc_status.0, sc_status.1
    );
    info!("    Key: {}...{}",
        &keys.sidechain_pub_key[..10],
        &keys.sidechain_pub_key[keys.sidechain_pub_key.len()-8..]
    );

    // Aura key
    let aura_status = match key_status.aura_loaded {
        Some(true) => ("✓", "Loaded in keystore"),
        Some(false) => ("✗", "NOT LOADED"),
        None => ("?", "Could not verify"),
    };
    info!(
        "  Aura:      {} {}",
        aura_status.0, aura_status.1
    );
    info!("    Key: {}...{}",
        &keys.aura_pub_key[..10],
        &keys.aura_pub_key[keys.aura_pub_key.len()-8..]
    );

    // Grandpa key
    let gran_status = match key_status.grandpa_loaded {
        Some(true) => ("✓", "Loaded in keystore"),
        Some(false) => ("✗", "NOT LOADED"),
        None => ("?", "Could not verify"),
    };
    info!(
        "  Grandpa:   {} {}",
        gran_status.0, gran_status.1
    );
    info!("    Key: {}...{}",
        &keys.grandpa_pub_key[..10],
        &keys.grandpa_pub_key[keys.grandpa_pub_key.len()-8..]
    );

    // Show note if keys can't be verified
    if key_status.sidechain_loaded.is_none()
        && key_status.aura_loaded.is_none()
        && key_status.grandpa_loaded.is_none() {
        info!("");
        warn!("  Note: Key verification requires node started with --rpc-methods=unsafe");
        warn!("        Keys shown above are from your keystore file only");
    }

    info!("");
    info!("Registration Status:");
    info!("─────────────────────────────────────────────────────────────────────────────");

    match &key_status.registration {
        Some(crate::midnight::RegistrationStatus::Permissioned) => {
            info!("  ✓ Permissioned candidate");
            info!("    Your key is in the permissioned candidates list");
        }
        Some(crate::midnight::RegistrationStatus::RegisteredValid) => {
            info!("  ✓ Registered (valid)");
            info!("    Your validator is registered and eligible to produce blocks");
        }
        Some(crate::midnight::RegistrationStatus::RegisteredInvalid(reason)) => {
            warn!("  ⚠ Registered but INVALID");
            warn!("    Reason: {}", reason);
        }
        Some(crate::midnight::RegistrationStatus::NotRegistered) => {
            error!("  ✗ Not registered");
            error!("    Your sidechain key is not in the candidates or registered list");
        }
        None => {
            info!("  ? Unable to check registration status");
        }
    }

    info!("");

    // Summary
    let all_loaded = key_status.sidechain_loaded == Some(true)
        && key_status.aura_loaded == Some(true)
        && key_status.grandpa_loaded == Some(true);

    let is_registered = matches!(
        key_status.registration,
        Some(crate::midnight::RegistrationStatus::Permissioned)
            | Some(crate::midnight::RegistrationStatus::RegisteredValid)
    );

    if all_loaded && is_registered {
        info!("Summary: ✓ All keys loaded and registered");
    } else if all_loaded {
        warn!("Summary: Keys loaded but registration issue detected");
    } else {
        error!("Summary: One or more keys not loaded!");
    }

    // Mark validator as ours in database and show block production stats
    if let Some(db) = db {
        info!("");
        info!("Block Production Statistics:");
        info!("─────────────────────────────────────────────────────────────────────────────");

        // Check if validator exists in database
        match db.get_validator(&keys.sidechain_pub_key)? {
            Some(mut validator) => {
                // Mark as ours if not already marked
                if !validator.is_ours {
                    validator.is_ours = true;
                    db.upsert_validator(&validator)?;
                    debug!("Marked validator {} as ours", keys.sidechain_pub_key);
                }

                // Show block production stats
                info!("  Total blocks produced: {}", validator.total_blocks);
                if let Some(epoch) = validator.first_seen_epoch {
                    info!("  First seen in epoch:   {}", epoch);
                }

                // Calculate share of total blocks
                let total_blocks = db.count_blocks()?;
                if total_blocks > 0 {
                    let share = (validator.total_blocks as f64 / total_blocks as f64) * 100.0;
                    info!("  Share of synced blocks: {:.2}%", share);
                }

                // Get all validators to show rank
                let all_validators = db.get_all_validators()?;
                if !all_validators.is_empty() {
                    let rank = all_validators.iter()
                        .position(|v| v.sidechain_key == keys.sidechain_pub_key)
                        .map(|i| i + 1)
                        .unwrap_or(0);
                    if rank > 0 {
                        info!("  Performance rank:       #{} of {} validators", rank, all_validators.len());
                    }
                }

                // Show recent blocks
                if validator.total_blocks > 0 {
                    let max_block = db.get_max_block_number()?.unwrap_or(0);
                    if max_block > 0 {
                        let start = max_block.saturating_sub(1000);
                        let blocks = db.get_blocks_in_range(start, max_block, Some(1000))?;

                        let recent_blocks: Vec<_> = blocks
                            .iter()
                            .filter(|b| {
                                b.author_key.as_ref()
                                    .map(|k| k == &keys.sidechain_pub_key)
                                    .unwrap_or(false)
                            })
                            .take(3)
                            .collect();

                        if !recent_blocks.is_empty() {
                            info!("");
                            info!("  Recent blocks (last 1000):");
                            for block in recent_blocks {
                                info!(
                                    "    Block #{} (slot {}, epoch {})",
                                    block.block_number, block.slot_number, block.epoch
                                );
                            }
                        }
                    }
                }
            }
            None => {
                // Validator not in database yet - create record if registered
                let registration_status = match &key_status.registration {
                    Some(crate::midnight::RegistrationStatus::Permissioned) => {
                        Some("permissioned".to_string())
                    }
                    Some(crate::midnight::RegistrationStatus::RegisteredValid) => {
                        Some("registered".to_string())
                    }
                    _ => None,
                };

                if registration_status.is_some() {
                    let validator_record = ValidatorRecord {
                        sidechain_key: keys.sidechain_pub_key.clone(),
                        aura_key: Some(keys.aura_pub_key.clone()),
                        grandpa_key: Some(keys.grandpa_pub_key.clone()),
                        label: None,
                        is_ours: true,
                        registration_status,
                        first_seen_epoch: Some(current_epoch),
                        total_blocks: 0,
                    };
                    db.upsert_validator(&validator_record)?;
                    info!("  ✓ Validator record created and marked as ours");
                    info!("  No blocks produced yet in synced range");
                    info!("  Block stats will appear once your validator produces blocks");
                } else {
                    info!("  Validator not found in database");
                    info!("  Register your validator first, then run this command again");
                }
            }
        }
    } else {
        debug!("Database not available, skipping validator marking and stats");
    }

    Ok(())
}
