//! Keys command - verify and manage session keys

use crate::midnight::{get_key_status, ValidatorKeys};
use crate::rpc::RpcClient;
use anyhow::Result;
use clap::{Args, Subcommand};
use std::path::PathBuf;
use tracing::{error, info, warn};

/// Keys command arguments
#[derive(Args, Debug)]
pub struct KeysArgs {
    /// Path to Substrate keystore directory
    #[arg(short = 'K', long)]
    pub keystore: PathBuf,

    /// Validator node RPC endpoint URL (for verification)
    #[arg(short, long, default_value = "http://localhost:9944")]
    pub rpc_url: String,

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
    // Load keys from keystore
    let keys = match ValidatorKeys::from_keystore(&args.keystore) {
        Ok(k) => k,
        Err(e) => {
            error!("Failed to load keys from keystore: {}", e);
            error!("Path: {}", args.keystore.display());
            return Err(e);
        }
    };

    match args.command {
        KeysCommands::Show => run_show(&keys),
        KeysCommands::Verify => run_verify(&keys, &args.rpc_url).await,
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

async fn run_verify(keys: &ValidatorKeys, rpc_url: &str) -> Result<()> {
    info!("Verifying validator keys...");
    info!("RPC endpoint: {}", rpc_url);
    info!("─────────────────────────────────────────────────────────────────────────────");

    let rpc = RpcClient::new(rpc_url);

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

    Ok(())
}
