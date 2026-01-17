use crate::rpc::RpcClient;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use tracing;

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
    /// Registered validator candidates (from AriadneParameters)
    /// These are the validators that MAY be in the committee
    pub candidates: Vec<Validator>,
    /// Actual committee (from AuraApi_authorities)
    /// This is the ordered list of AURA keys that produce blocks
    /// Block author = committee[slot % committee.len()]
    pub committee: Vec<String>,
}

impl ValidatorSet {
    /// Legacy accessor for backward compatibility
    pub fn validators(&self) -> &[Validator] {
        &self.candidates
    }
}

impl ValidatorSet {
    /// Fetch validator set with committee for a given epoch at a specific block
    ///
    /// This is the CORRECT method that fetches both:
    /// 1. Candidates from AriadneParameters (for reference)
    /// 2. Committee from AuraApi_authorities (for block attribution)
    ///
    /// IMPORTANT: When syncing historical blocks, pass the block_hash to get
    /// the committee that was active at that point in time. The committee
    /// changes each epoch, so using the current committee for historical
    /// blocks will result in incorrect author attribution.
    pub async fn fetch_with_committee(
        rpc: &RpcClient,
        epoch: u64,
        block_hash: Option<&str>,
    ) -> Result<Self> {
        // Fetch validator candidates
        let candidates = Self::fetch_candidates(rpc, epoch).await?;

        // Fetch actual committee at the specified block (or current if None)
        let committee = Self::fetch_committee_at_block(rpc, block_hash).await?;

        Ok(ValidatorSet {
            epoch,
            candidates,
            committee,
        })
    }

    /// Fetch validator set with committee, falling back to current state if historical is pruned
    ///
    /// This method attempts to fetch the committee at a historical block hash, but if
    /// the node has pruned that state (non-archive node), it falls back to the current
    /// committee. This allows syncing historical blocks on pruned nodes, though author
    /// attribution may be inaccurate for blocks from different epochs.
    ///
    /// Returns (ValidatorSet, used_fallback) where used_fallback is true if we had to
    /// use the current committee instead of the historical one.
    pub async fn fetch_with_committee_or_fallback(
        rpc: &RpcClient,
        epoch: u64,
        block_hash: &str,
    ) -> Result<(Self, bool)> {
        // Fetch validator candidates
        let candidates = Self::fetch_candidates(rpc, epoch).await?;

        // Try to fetch committee at the historical block
        match Self::fetch_committee_at_block(rpc, Some(block_hash)).await {
            Ok(committee) => {
                // Historical state available
                Ok((
                    ValidatorSet {
                        epoch,
                        candidates,
                        committee,
                    },
                    false, // No fallback used
                ))
            }
            Err(e) => {
                // Check if this is a "state discarded" error anywhere in the error chain
                // We check the full error string which includes all context and causes
                let full_error = format!("{:?}", e);
                let is_pruned_state = full_error.contains("State already discarded")
                    || full_error.contains("UnknownBlock");

                if is_pruned_state {
                    // State was pruned, fall back to current committee
                    tracing::warn!(
                        "Historical state pruned for block {}, using current committee (attribution may be inaccurate)",
                        block_hash
                    );

                    let committee = Self::fetch_committee_at_block(rpc, None)
                        .await
                        .context("Failed to fetch current committee as fallback")?;

                    Ok((
                        ValidatorSet {
                            epoch,
                            candidates,
                            committee,
                        },
                        true, // Fallback was used
                    ))
                } else {
                    // Some other error, propagate it
                    Err(e)
                }
            }
        }
    }

    /// Fetch validator candidates from AriadneParameters
    async fn fetch_candidates(rpc: &RpcClient, epoch: u64) -> Result<Vec<Validator>> {
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

        Ok(validators)
    }

    /// Fetch committee from AURA runtime at a specific block
    ///
    /// Returns the actual committee that produces blocks (typically ~1200 AURA keys).
    ///
    /// If block_hash is provided, queries the state at that historical block.
    /// If None, queries the current state.
    async fn fetch_committee_at_block(
        rpc: &RpcClient,
        block_hash: Option<&str>,
    ) -> Result<Vec<String>> {
        use crate::midnight::decode_aura_authorities;
        use serde_json::Value;

        // Call AuraApi_authorities runtime method
        // state_call accepts: (method, data, [optional block_hash])
        let result: String = if let Some(hash) = block_hash {
            // Query at specific block hash
            rpc.call(
                "state_call",
                vec![
                    Value::String("AuraApi_authorities".to_string()),
                    Value::String("0x".to_string()),
                    Value::String(hash.to_string()),
                ],
            )
            .await
            .with_context(|| {
                format!(
                    "Failed to call AuraApi_authorities at block {}",
                    hash
                )
            })?
        } else {
            // Query current state
            rpc.call(
                "state_call",
                vec![
                    Value::String("AuraApi_authorities".to_string()),
                    Value::String("0x".to_string()),
                ],
            )
            .await
            .context("Failed to call AuraApi_authorities")?
        };

        // Decode SCALE-encoded response
        decode_aura_authorities(&result)
            .context("Failed to decode AURA authorities response")
    }

    /// Legacy fetch method (DEPRECATED - uses incorrect candidate list for block attribution)
    ///
    /// This method is kept for backward compatibility but should NOT be used for
    /// block author calculation as it returns candidates (185) not the actual
    /// committee (1200).
    #[deprecated(
        since = "0.4.0",
        note = "Use fetch_with_committee() instead for correct block attribution"
    )]
    pub async fn fetch(rpc: &RpcClient, epoch: u64) -> Result<Self> {
        let candidates = Self::fetch_candidates(rpc, epoch).await?;
        Ok(ValidatorSet {
            epoch,
            candidates,
            committee: Vec::new(), // Empty - will cause get_author() to fail
        })
    }

    /// Get the block author for a given slot number (FIXED VERSION)
    ///
    /// Uses the actual committee (not candidates) for correct attribution.
    /// Formula: author = committee[slot % committee.len()]
    ///
    /// Returns the AURA key from the committee and tries to find the corresponding
    /// candidate/validator for additional information.
    pub fn get_author(&self, slot_number: u64) -> Option<&Validator> {
        if self.committee.is_empty() {
            tracing::warn!(
                "Committee is empty - cannot determine block author. \
                 Use fetch_with_committee() instead of fetch()"
            );
            return None;
        }

        // CORRECT: Use committee size, not candidate count
        let committee_index = (slot_number as usize) % self.committee.len();
        let aura_key = &self.committee[committee_index];

        // Try to find the validator in our candidate list
        // Note: May return None if validator is in committee but not in candidates
        self.find_by_aura_key(aura_key)
    }

    /// Get the AURA key that should produce the block for a given slot
    ///
    /// This always works even if the validator is not in our candidate list.
    pub fn get_author_aura_key(&self, slot_number: u64) -> Option<&str> {
        if self.committee.is_empty() {
            return None;
        }

        let committee_index = (slot_number as usize) % self.committee.len();
        self.committee.get(committee_index).map(|s| s.as_str())
    }

    /// Get candidate count (registered validators)
    pub fn candidate_count(&self) -> usize {
        self.candidates.len()
    }

    /// Get committee size (actual block producers)
    pub fn committee_size(&self) -> usize {
        self.committee.len()
    }

    /// Legacy count method (returns candidate count for backward compatibility)
    pub fn count(&self) -> usize {
        self.candidate_count()
    }

    /// Find validator by sidechain key
    pub fn find_by_sidechain_key(&self, key: &str) -> Option<&Validator> {
        let normalized = normalize_hex(key);
        self.candidates
            .iter()
            .find(|v| v.sidechain_key == normalized)
    }

    /// Find validator by aura key
    pub fn find_by_aura_key(&self, key: &str) -> Option<&Validator> {
        let normalized = normalize_hex(key);
        self.candidates
            .iter()
            .find(|v| v.aura_key == normalized)
    }

    /// Check if an AURA key is in the committee
    pub fn is_in_committee(&self, aura_key: &str) -> bool {
        let normalized = normalize_hex(aura_key);
        self.committee.contains(&normalized)
    }

    /// Count how many seats a validator has in the committee
    pub fn count_seats(&self, aura_key: &str) -> usize {
        let normalized = normalize_hex(aura_key);
        self.committee.iter().filter(|k| *k == &normalized).count()
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
        let mut candidates = vec![
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
        ];

        // Sort by aura key
        candidates.sort_by(|a, b| a.aura_key.cmp(&b.aura_key));

        // Committee with some validators appearing multiple times
        let committee = vec![
            "0x111".to_string(),
            "0x222".to_string(),
            "0x333".to_string(),
            "0x111".to_string(), // Validator 1 appears twice
        ];

        let validator_set = ValidatorSet {
            epoch: 1000,
            candidates,
            committee,
        };

        // Test author calculation (uses committee, not candidates)
        assert_eq!(validator_set.get_author(0).unwrap().aura_key, "0x111");
        assert_eq!(validator_set.get_author(1).unwrap().aura_key, "0x222");
        assert_eq!(validator_set.get_author(2).unwrap().aura_key, "0x333");
        assert_eq!(validator_set.get_author(3).unwrap().aura_key, "0x111"); // 4th slot wraps to validator 1
        assert_eq!(validator_set.get_author(4).unwrap().aura_key, "0x111"); // 5th slot wraps around (4 % 4 = 0)
        assert_eq!(validator_set.get_author(100).unwrap().aura_key, "0x111"); // 100 % 4 = 0

        // Test AURA key retrieval
        assert_eq!(validator_set.get_author_aura_key(0).unwrap(), "0x111");
        assert_eq!(validator_set.get_author_aura_key(1).unwrap(), "0x222");

        // Test seat counting
        assert_eq!(validator_set.count_seats("0x111"), 2);
        assert_eq!(validator_set.count_seats("0x222"), 1);
        assert_eq!(validator_set.count_seats("0x333"), 1);
        assert_eq!(validator_set.count_seats("0x999"), 0);

        // Test committee membership
        assert!(validator_set.is_in_committee("0x111"));
        assert!(validator_set.is_in_committee("0x222"));
        assert!(!validator_set.is_in_committee("0x999"));

        // Test counts
        assert_eq!(validator_set.candidate_count(), 3);
        assert_eq!(validator_set.committee_size(), 4);
    }
}
