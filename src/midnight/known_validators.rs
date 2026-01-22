//! Known validators registry for friendly labels
//!
//! Loads validator labels from known_validators.toml (gitignored for privacy).
//! This allows users to add friendly names to validators they recognize.

use anyhow::Result;
use directories::ProjectDirs;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// A known validator entry
#[derive(Debug, Clone, Deserialize)]
pub struct KnownValidator {
    pub sidechain_key: String,
    pub label: String,
}

/// Known validators file structure
#[derive(Debug, Clone, Deserialize, Default)]
pub struct KnownValidatorsFile {
    #[serde(default)]
    pub validators: Vec<KnownValidator>,
}

/// Registry of known validators with labels
#[derive(Debug, Clone, Default)]
pub struct KnownValidators {
    /// Map from normalized sidechain key to label
    labels: HashMap<String, String>,
}

impl KnownValidators {
    /// Load known validators from file
    ///
    /// Searches in order:
    /// 1. ./known_validators.toml
    /// 2. ~/.config/mvm/known_validators.toml
    /// 3. /opt/midnight/mvm/config/known_validators.toml
    pub fn load() -> Self {
        match Self::try_load() {
            Ok(kv) => {
                if !kv.labels.is_empty() {
                    tracing::info!("Loaded {} known validator labels", kv.labels.len());
                }
                kv
            }
            Err(e) => {
                tracing::debug!("Could not load known validators: {}", e);
                Self::default()
            }
        }
    }

    fn try_load() -> Result<Self> {
        let paths = Self::file_paths();

        for path in &paths {
            if path.exists() {
                let contents = fs::read_to_string(path)?;
                let file: KnownValidatorsFile = toml::from_str(&contents)?;

                let mut labels = HashMap::new();
                for v in file.validators {
                    let key = normalize_key(&v.sidechain_key);
                    labels.insert(key, v.label);
                }

                tracing::debug!("Loaded known validators from: {}", path.display());
                return Ok(Self { labels });
            }
        }

        Ok(Self::default())
    }

    /// Get search paths for known_validators.toml
    pub fn file_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();

        // 1. Current directory
        paths.push(PathBuf::from("./known_validators.toml"));

        // 2. User config directory
        if let Some(proj_dirs) = ProjectDirs::from("com", "midnight", "mvm") {
            paths.push(proj_dirs.config_dir().join("known_validators.toml"));
        }

        // 3. Install location
        paths.push(PathBuf::from("/opt/midnight/mvm/config/known_validators.toml"));

        paths
    }

    /// Get label for a validator by sidechain key
    pub fn get_label(&self, sidechain_key: &str) -> Option<&str> {
        let key = normalize_key(sidechain_key);
        self.labels.get(&key).map(|s| s.as_str())
    }

    /// Check if we have any known validators loaded
    pub fn is_empty(&self) -> bool {
        self.labels.is_empty()
    }

    /// Get count of known validators
    pub fn len(&self) -> usize {
        self.labels.len()
    }
}

/// Normalize a hex key for consistent lookup
/// Removes 0x prefix and converts to lowercase
fn normalize_key(key: &str) -> String {
    key.trim()
        .to_lowercase()
        .strip_prefix("0x")
        .unwrap_or(key)
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_key() {
        assert_eq!(normalize_key("0x123ABC"), "123abc");
        assert_eq!(normalize_key("123ABC"), "123abc");
        assert_eq!(normalize_key("  0x123ABC  "), "123abc");
    }

    #[test]
    fn test_empty_registry() {
        let kv = KnownValidators::default();
        assert!(kv.is_empty());
        assert_eq!(kv.get_label("0x123"), None);
    }
}
