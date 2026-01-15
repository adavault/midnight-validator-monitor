use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

/// Key type identifiers (hex-encoded ASCII)
pub const KEY_TYPE_AURA: &str = "61757261"; // "aura"
pub const KEY_TYPE_GRANDPA: &str = "6772616e"; // "gran"
pub const KEY_TYPE_SIDECHAIN: &str = "63726368"; // "crch"

/// Validator public keys
#[derive(Debug, Clone, Deserialize)]
pub struct ValidatorKeys {
    #[serde(rename = "sidechain_pub_key")]
    pub sidechain_pub_key: String,
    #[serde(rename = "aura_pub_key")]
    pub aura_pub_key: String,
    #[serde(rename = "grandpa_pub_key")]
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
    ///
    /// Keystore files are named: `<key_type_hex><public_key_hex>`
    /// Key types:
    /// - "aura" (61757261)
    /// - "crch" (63726368) - sidechain
    /// - "gran" (6772616e) - grandpa
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
                KEY_TYPE_AURA => aura_pub_key = Some(pubkey),
                KEY_TYPE_SIDECHAIN => sidechain_pub_key = Some(pubkey),
                KEY_TYPE_GRANDPA => grandpa_pub_key = Some(pubkey),
                _ => {}
            }
        }

        Ok(Self {
            sidechain_pub_key: sidechain_pub_key
                .context("Sidechain key (crch) not found in keystore")?,
            aura_pub_key: aura_pub_key.context("Aura key not found in keystore")?,
            grandpa_pub_key: grandpa_pub_key.context("Grandpa key (gran) not found in keystore")?,
        })
    }

    /// Get a short form of the sidechain key for display
    pub fn sidechain_short(&self) -> String {
        truncate_key(&self.sidechain_pub_key, 10)
    }

    /// Get a short form of the aura key for display
    pub fn aura_short(&self) -> String {
        truncate_key(&self.aura_pub_key, 10)
    }

    /// Get a short form of the grandpa key for display
    pub fn grandpa_short(&self) -> String {
        truncate_key(&self.grandpa_pub_key, 10)
    }
}

/// Status of validator keys
#[derive(Debug, Default, Clone)]
pub struct KeyStatus {
    /// Sidechain key loaded in node keystore
    pub sidechain_loaded: Option<bool>,
    /// Aura key loaded in node keystore
    pub aura_loaded: Option<bool>,
    /// Grandpa key loaded in node keystore
    pub grandpa_loaded: Option<bool>,
    /// Validator registration status
    pub registration: Option<super::RegistrationStatus>,
}

impl KeyStatus {
    pub fn all_keys_loaded(&self) -> bool {
        self.sidechain_loaded == Some(true)
            && self.aura_loaded == Some(true)
            && self.grandpa_loaded == Some(true)
    }
}

/// Truncate a hex key for display
fn truncate_key(key: &str, chars: usize) -> String {
    if key.len() <= chars + 3 {
        key.to_string()
    } else {
        format!("{}...", &key[..chars])
    }
}

/// Normalize hex string for comparison (lowercase, with 0x prefix)
pub fn normalize_hex(s: &str) -> String {
    let s = s.to_lowercase();
    if s.starts_with("0x") {
        s
    } else {
        format!("0x{}", s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_hex() {
        assert_eq!(normalize_hex("0xABCD"), "0xabcd");
        assert_eq!(normalize_hex("ABCD"), "0xabcd");
        assert_eq!(normalize_hex("0x1234"), "0x1234");
    }

    #[test]
    fn test_truncate_key() {
        assert_eq!(truncate_key("0x1234567890abcdef", 10), "0x12345678...");
        assert_eq!(truncate_key("0x1234", 10), "0x1234");
    }
}
