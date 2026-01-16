use crate::rpc::RpcClient;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;

/// Validator information from AriadneParameters
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionedCandidate {
    pub sidechain_public_key: String,
    pub aura_public_key: String,
    pub grandpa_public_key: String,
    pub is_valid: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CandidateRegistration {
    pub sidechain_pub_key: String,
    pub aura_pub_key: String,
    pub grandpa_pub_key: String,
    pub is_valid: bool,
    #[serde(skip)]
    pub mainchain_pub_key: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AriadneParameters {
    pub permissioned_candidates: Vec<PermissionedCandidate>,
    pub candidate_registrations: HashMap<String, Vec<CandidateRegistration>>,
}

/// A validator in the active set
#[derive(Debug, Clone)]
pub struct Validator {
    pub sidechain_key: String,
    pub aura_key: String,
    pub grandpa_key: String,
    pub is_permissioned: bool,
}

/// Ordered validator set for a specific epoch
#[derive(Debug, Clone)]
pub struct ValidatorSet {
    pub epoch: u64,
    pub validators: Vec<Validator>,
}

impl ValidatorSet {
    /// Fetch validator set for a given epoch
    pub async fn fetch(rpc: &RpcClient, epoch: u64) -> Result<Self> {
        let params: AriadneParameters = rpc
            .call("sidechain_getAriadneParameters", vec![epoch])
            .await
            .with_context(|| format!("Failed to fetch validator set for epoch {}", epoch))?;

        let mut validators = Vec::new();

        // Add permissioned candidates
        for candidate in params.permissioned_candidates {
            if candidate.is_valid {
                validators.push(Validator {
                    sidechain_key: normalize_hex(&candidate.sidechain_public_key),
                    aura_key: normalize_hex(&candidate.aura_public_key),
                    grandpa_key: normalize_hex(&candidate.grandpa_public_key),
                    is_permissioned: true,
                });
            }
        }

        // Add registered candidates
        for registrations in params.candidate_registrations.values() {
            for reg in registrations {
                if reg.is_valid {
                    validators.push(Validator {
                        sidechain_key: normalize_hex(&reg.sidechain_pub_key),
                        aura_key: normalize_hex(&reg.aura_pub_key),
                        grandpa_key: normalize_hex(&reg.grandpa_pub_key),
                        is_permissioned: false,
                    });
                }
            }
        }

        // Sort validators by AURA public key (deterministic ordering)
        // This matches how AURA consensus orders the authority set
        validators.sort_by(|a, b| a.aura_key.cmp(&b.aura_key));

        Ok(ValidatorSet { epoch, validators })
    }

    /// Get the block author for a given slot number
    pub fn get_author(&self, slot_number: u64) -> Option<&Validator> {
        if self.validators.is_empty() {
            return None;
        }

        let author_index = (slot_number as usize) % self.validators.len();
        self.validators.get(author_index)
    }

    /// Get validator count
    pub fn count(&self) -> usize {
        self.validators.len()
    }

    /// Find validator by sidechain key
    pub fn find_by_sidechain_key(&self, key: &str) -> Option<&Validator> {
        let normalized = normalize_hex(key);
        self.validators
            .iter()
            .find(|v| v.sidechain_key == normalized)
    }

    /// Find validator by aura key
    pub fn find_by_aura_key(&self, key: &str) -> Option<&Validator> {
        let normalized = normalize_hex(key);
        self.validators.iter().find(|v| v.aura_key == normalized)
    }
}

/// Normalize hex string (lowercase, with 0x prefix)
fn normalize_hex(hex: &str) -> String {
    let hex = hex.trim().to_lowercase();
    if hex.starts_with("0x") {
        hex
    } else {
        format!("0x{}", hex)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_hex() {
        assert_eq!(
            normalize_hex("0xABCD1234"),
            "0xabcd1234"
        );
        assert_eq!(
            normalize_hex("ABCD1234"),
            "0xabcd1234"
        );
        assert_eq!(
            normalize_hex("  0xABCD1234  "),
            "0xabcd1234"
        );
    }

    #[test]
    fn test_author_calculation() {
        let mut validator_set = ValidatorSet {
            epoch: 1000,
            validators: vec![
                Validator {
                    sidechain_key: "0xaaa".to_string(),
                    aura_key: "0x111".to_string(),
                    grandpa_key: "0x111".to_string(),
                    is_permissioned: true,
                },
                Validator {
                    sidechain_key: "0xbbb".to_string(),
                    aura_key: "0x222".to_string(),
                    grandpa_key: "0x222".to_string(),
                    is_permissioned: true,
                },
                Validator {
                    sidechain_key: "0xccc".to_string(),
                    aura_key: "0x333".to_string(),
                    grandpa_key: "0x333".to_string(),
                    is_permissioned: false,
                },
            ],
        };

        // Sort by aura key
        validator_set.validators.sort_by(|a, b| a.aura_key.cmp(&b.aura_key));

        // Test author calculation
        assert_eq!(validator_set.get_author(0).unwrap().aura_key, "0x111");
        assert_eq!(validator_set.get_author(1).unwrap().aura_key, "0x222");
        assert_eq!(validator_set.get_author(2).unwrap().aura_key, "0x333");
        assert_eq!(validator_set.get_author(3).unwrap().aura_key, "0x111"); // Wraps around
        assert_eq!(validator_set.get_author(100).unwrap().aura_key, "0x222"); // 100 % 3 = 1
    }
}
