use crate::rpc::RpcClient;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

/// Validator public keys
#[derive(Debug, Clone, Deserialize)]
pub struct ValidatorKeys {
    pub sidechain_pub_key: String,
    pub aura_pub_key: String,
    pub grandpa_pub_key: String,
}

impl ValidatorKeys {
    /// Load keys from a JSON file
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read keys file: {}", path.display()))?;

        serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse keys file: {}", path.display()))
    }

    /// Load keys from a Substrate keystore directory
    /// Keystore files are named: <key_type_hex><public_key_hex>
    /// Key types: "aura" (61757261), "crch" (63726368), "gran" (6772616e)
    pub fn from_keystore(path: &Path) -> Result<Self> {
        let mut sidechain_pub_key = None;
        let mut aura_pub_key = None;
        let mut grandpa_pub_key = None;

        let entries = std::fs::read_dir(path)
            .with_context(|| format!("Failed to read keystore directory: {}", path.display()))?;

        for entry in entries {
            let entry = entry?;
            let filename = entry.file_name();
            let filename = filename.to_string_lossy();

            if filename.len() < 8 {
                continue;
            }

            let key_type = &filename[..8];
            let pubkey = format!("0x{}", &filename[8..]);

            match key_type {
                "61757261" => aura_pub_key = Some(pubkey),      // "aura"
                "63726368" => sidechain_pub_key = Some(pubkey), // "crch"
                "6772616e" => grandpa_pub_key = Some(pubkey),   // "gran"
                _ => {}
            }
        }

        Ok(Self {
            sidechain_pub_key: sidechain_pub_key
                .context("Sidechain key (crch) not found in keystore")?,
            aura_pub_key: aura_pub_key
                .context("Aura key not found in keystore")?,
            grandpa_pub_key: grandpa_pub_key
                .context("Grandpa key (gran) not found in keystore")?,
        })
    }
}

/// Registration status for a validator
#[derive(Debug, Clone, PartialEq)]
pub enum RegistrationStatus {
    /// Registered as permissioned candidate (static)
    Permissioned,
    /// Registered as candidate and valid
    RegisteredValid,
    /// Registered as candidate but invalid (with reason)
    RegisteredInvalid(String),
    /// Not registered at all
    NotRegistered,
}

/// Status of validator keys
#[derive(Debug, Default)]
pub struct KeyStatus {
    /// Sidechain key loaded in keystore
    pub sidechain_loaded: Option<bool>,
    /// Aura key loaded in keystore
    pub aura_loaded: Option<bool>,
    /// Grandpa key loaded in keystore
    pub grandpa_loaded: Option<bool>,
    /// Validator registration status
    pub registration: Option<RegistrationStatus>,
}

impl KeyStatus {
    pub fn all_keys_loaded(&self) -> bool {
        self.sidechain_loaded == Some(true)
            && self.aura_loaded == Some(true)
            && self.grandpa_loaded == Some(true)
    }
}

/// Check if a key is loaded in the node's keystore
pub async fn check_key_loaded(rpc: &RpcClient, pubkey: &str, key_type: &str) -> Result<bool> {
    // author_hasKey params: [pubkey, key_type]
    // key_type: "gran" for grandpa, "aura" for aura, "crch" for sidechain
    let result: bool = rpc.call("author_hasKey", vec![pubkey, key_type]).await?;
    Ok(result)
}

/// Check validator registration status in the current epoch
pub async fn check_registration(rpc: &RpcClient, sidechain_pubkey: &str, epoch: u64) -> Result<RegistrationStatus> {
    #[derive(Debug, Deserialize)]
    struct AriadneParams {
        #[serde(rename = "permissionedCandidates")]
        permissioned_candidates: Vec<PermissionedCandidate>,
        #[serde(rename = "candidateRegistrations")]
        candidate_registrations: std::collections::HashMap<String, Vec<CandidateRegistration>>,
    }

    #[derive(Debug, Deserialize)]
    struct PermissionedCandidate {
        #[serde(rename = "sidechainPublicKey")]
        sidechain_public_key: String,
    }

    #[derive(Debug, Deserialize)]
    struct CandidateRegistration {
        #[serde(rename = "sidechainPubKey")]
        sidechain_pub_key: String,
        #[serde(rename = "isValid")]
        is_valid: bool,
        #[serde(rename = "invalidReasons")]
        invalid_reasons: Option<serde_json::Value>,
    }

    let params: AriadneParams = rpc.call("sidechain_getAriadneParameters", vec![epoch]).await?;
    let normalized_key = normalize_hex(sidechain_pubkey);

    // First check permissioned candidates (static list)
    for candidate in &params.permissioned_candidates {
        if normalize_hex(&candidate.sidechain_public_key) == normalized_key {
            return Ok(RegistrationStatus::Permissioned);
        }
    }

    // Then check candidate registrations (dynamic)
    for registrations in params.candidate_registrations.values() {
        for reg in registrations {
            if normalize_hex(&reg.sidechain_pub_key) == normalized_key {
                if reg.is_valid {
                    return Ok(RegistrationStatus::RegisteredValid);
                } else {
                    let reason = reg
                        .invalid_reasons
                        .as_ref()
                        .map(|r| r.to_string())
                        .unwrap_or_else(|| "Unknown".to_string());
                    return Ok(RegistrationStatus::RegisteredInvalid(reason));
                }
            }
        }
    }

    Ok(RegistrationStatus::NotRegistered)
}

/// Normalize hex string for comparison (lowercase, with 0x prefix)
fn normalize_hex(s: &str) -> String {
    let s = s.to_lowercase();
    if s.starts_with("0x") {
        s
    } else {
        format!("0x{}", s)
    }
}

/// Get complete key status for a validator
pub async fn get_key_status(rpc: &RpcClient, keys: &ValidatorKeys, current_epoch: u64) -> KeyStatus {
    let mut status = KeyStatus::default();

    // Check if keys are loaded in keystore
    status.sidechain_loaded = check_key_loaded(rpc, &keys.sidechain_pub_key, "crch")
        .await
        .ok();

    status.aura_loaded = check_key_loaded(rpc, &keys.aura_pub_key, "aura")
        .await
        .ok();

    status.grandpa_loaded = check_key_loaded(rpc, &keys.grandpa_pub_key, "gran")
        .await
        .ok();

    // Check registration status
    status.registration = check_registration(rpc, &keys.sidechain_pub_key, current_epoch)
        .await
        .ok();

    status
}
