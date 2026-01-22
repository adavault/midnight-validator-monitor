use crate::rpc::RpcClient;
use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;

use super::keystore::{normalize_hex, CommitteeStatus, KeyStatus, ValidatorKeys};
use super::ValidatorSet;

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

impl std::fmt::Display for RegistrationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegistrationStatus::Permissioned => write!(f, "Permissioned candidate"),
            RegistrationStatus::RegisteredValid => write!(f, "Registered (valid)"),
            RegistrationStatus::RegisteredInvalid(reason) => {
                write!(f, "Registered but INVALID: {}", reason)
            }
            RegistrationStatus::NotRegistered => write!(f, "Not registered"),
        }
    }
}

/// Check if a key is loaded in the node's keystore via RPC
pub async fn check_key_loaded(rpc: &RpcClient, pubkey: &str, key_type: &str) -> Result<bool> {
    // author_hasKey params: [pubkey, key_type]
    // key_type: "gran" for grandpa, "aura" for aura, "crch" for sidechain
    let result: bool = rpc.call("author_hasKey", vec![pubkey, key_type]).await?;
    Ok(result)
}

/// Check validator registration status in the current epoch
pub async fn check_registration(
    rpc: &RpcClient,
    sidechain_pubkey: &str,
    epoch: u64,
) -> Result<RegistrationStatus> {
    #[derive(Debug, Deserialize)]
    struct AriadneParams {
        #[serde(rename = "permissionedCandidates")]
        permissioned_candidates: Vec<PermissionedCandidate>,
        #[serde(rename = "candidateRegistrations")]
        candidate_registrations: HashMap<String, Vec<CandidateRegistration>>,
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

    let params: AriadneParams = rpc
        .call("sidechain_getAriadneParameters", vec![epoch])
        .await?;
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

/// Get complete key status for a validator
pub async fn get_key_status(
    rpc: &RpcClient,
    keys: &ValidatorKeys,
    current_epoch: u64,
) -> KeyStatus {
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

    // Check committee status
    status.committee_status = get_committee_status(rpc, keys, current_epoch).await.ok();

    status
}

/// Get committee status for a validator
async fn get_committee_status(
    rpc: &RpcClient,
    keys: &ValidatorKeys,
    current_epoch: u64,
) -> Result<CommitteeStatus> {
    // Fetch the current committee
    let committee = ValidatorSet::fetch_committee_at_block(rpc, None).await?;
    let committee_size = committee.len() as u32;

    // Normalize the aura key for comparison
    let aura_key_normalized = normalize_hex(&keys.aura_pub_key);

    // Count seats in the committee
    let seat_count = committee.iter()
        .filter(|k| normalize_hex(k) == aura_key_normalized)
        .count() as u32;

    let in_committee = seat_count > 0;

    // Calculate selection probability
    let selection_probability = if committee_size > 0 {
        seat_count as f64 / committee_size as f64
    } else {
        0.0
    };

    // Fetch stake from AriadneParameters
    let stake_lovelace = get_validator_stake(rpc, &keys.sidechain_pub_key, current_epoch).await.ok().flatten();

    // Expected blocks per sidechain epoch (typically 1200 blocks on preview)
    // blocks_per_epoch * (seats / committee_size)
    let blocks_per_epoch = 1200.0; // preview network
    let expected_blocks_per_epoch = selection_probability * blocks_per_epoch;

    Ok(CommitteeStatus {
        in_committee,
        seat_count,
        committee_size,
        stake_lovelace,
        selection_probability,
        expected_blocks_per_epoch,
    })
}

/// Get validator stake from AriadneParameters
async fn get_validator_stake(
    rpc: &RpcClient,
    sidechain_pubkey: &str,
    epoch: u64,
) -> Result<Option<u64>> {
    #[derive(Debug, Deserialize)]
    struct AriadneParams {
        #[serde(rename = "candidateRegistrations")]
        candidate_registrations: HashMap<String, Vec<CandidateReg>>,
    }

    #[derive(Debug, Deserialize)]
    struct CandidateReg {
        #[serde(rename = "sidechainPubKey")]
        sidechain_pub_key: String,
        #[serde(default)]
        #[serde(alias = "stake", alias = "stakeAmount", alias = "stakeDelegation")]
        stake_delegation: Option<u64>,
    }

    let params: AriadneParams = rpc
        .call("sidechain_getAriadneParameters", vec![epoch])
        .await?;

    let normalized_key = normalize_hex(sidechain_pubkey);

    for registrations in params.candidate_registrations.values() {
        for reg in registrations {
            if normalize_hex(&reg.sidechain_pub_key) == normalized_key {
                return Ok(reg.stake_delegation);
            }
        }
    }

    Ok(None)
}
