//! Configuration management for MVM

use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Configuration structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub rpc: RpcConfig,

    #[serde(default)]
    pub database: DatabaseConfig,

    #[serde(default)]
    pub validator: ValidatorConfig,

    #[serde(default)]
    pub sync: SyncConfig,

    #[serde(default)]
    pub view: ViewConfig,

    #[serde(default)]
    pub daemon: DaemonConfig,

    #[serde(default)]
    pub chain: ChainConfig,

    #[serde(default)]
    pub alerts: AlertConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcConfig {
    #[serde(default = "default_rpc_url")]
    pub url: String,

    #[serde(default = "default_metrics_url")]
    pub metrics_url: String,

    /// Optional node_exporter URL for system metrics (memory, FDs, CPU)
    /// If set, MVM will fetch process metrics from this endpoint
    /// Example: "http://localhost:9100/metrics"
    #[serde(default)]
    pub node_exporter_url: Option<String>,

    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,

    /// Maximum retry attempts for transient failures (0 = no retries)
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,

    /// Initial delay between retries in milliseconds
    #[serde(default = "default_retry_initial_delay")]
    pub retry_initial_delay_ms: u64,

    /// Maximum delay between retries in milliseconds
    #[serde(default = "default_retry_max_delay")]
    pub retry_max_delay_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    #[serde(default = "default_db_path")]
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ValidatorConfig {
    #[serde(default)]
    pub keystore_path: Option<String>,

    #[serde(default)]
    pub label: Option<String>,

    /// Display name for this node (defaults to hostname)
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    #[serde(default = "default_batch_size")]
    pub batch_size: u32,

    #[serde(default = "default_poll_interval")]
    pub poll_interval_secs: u64,

    #[serde(default)]
    pub finalized_only: bool,

    #[serde(default)]
    pub start_block: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewConfig {
    #[serde(default = "default_refresh_interval")]
    pub refresh_interval_ms: u64,

    /// Expected external IP address for filtering peer-reported addresses
    /// Only addresses matching this IP will be displayed
    #[serde(default)]
    pub expected_ip: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DaemonConfig {
    #[serde(default)]
    pub pid_file: Option<String>,

    #[serde(default)]
    pub log_file: Option<String>,

    #[serde(default)]
    pub enable_syslog: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainConfig {
    /// Network preset: "preview", "preprod", or "mainnet"
    /// Determines epoch durations and timing parameters
    #[serde(default = "default_network")]
    pub network: String,

    /// Optional: Override genesis timestamp (milliseconds since Unix epoch)
    /// If not set, uses the network preset default (when available)
    #[serde(default)]
    pub genesis_timestamp_ms: Option<u64>,
}

/// Alert configuration for block production monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertConfig {
    /// Enable block production alerts
    #[serde(default)]
    pub enabled: bool,

    /// Alert threshold: percentage of expected blocks below which to alert (0-100)
    /// Default: 80 (alert if producing less than 80% of expected blocks)
    #[serde(default = "default_alert_threshold")]
    pub threshold_percent: u8,

    /// Minimum blocks expected before alerting (to avoid false positives early in epoch)
    #[serde(default = "default_min_expected_blocks")]
    pub min_expected_blocks: u32,

    /// Optional webhook URL for sending alerts
    #[serde(default)]
    pub webhook_url: Option<String>,

    /// Cooldown between alerts in seconds (to avoid spam)
    #[serde(default = "default_alert_cooldown")]
    pub cooldown_secs: u64,
}

fn default_alert_threshold() -> u8 {
    80
}

fn default_min_expected_blocks() -> u32 {
    5
}

fn default_alert_cooldown() -> u64 {
    300 // 5 minutes
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            threshold_percent: default_alert_threshold(),
            min_expected_blocks: default_min_expected_blocks(),
            webhook_url: None,
            cooldown_secs: default_alert_cooldown(),
        }
    }
}

impl Default for ChainConfig {
    fn default() -> Self {
        Self {
            network: default_network(),
            genesis_timestamp_ms: None,
        }
    }
}

impl ChainConfig {
    /// Get the ChainTiming for this configuration
    pub fn timing(&self) -> crate::midnight::ChainTiming {
        let network = crate::midnight::Network::from_str(&self.network)
            .unwrap_or(crate::midnight::Network::Preview);
        let mut timing = crate::midnight::ChainTiming::for_network(network);

        // Override genesis if specified
        if let Some(genesis) = self.genesis_timestamp_ms {
            timing.genesis_timestamp_ms = Some(genesis);
        }

        timing
    }
}

fn default_network() -> String {
    "preview".to_string()
}

// Default values
fn default_rpc_url() -> String {
    "http://localhost:9944".to_string()
}

fn default_metrics_url() -> String {
    "http://localhost:9615/metrics".to_string()
}

fn default_timeout() -> u64 {
    30000
}

fn default_db_path() -> String {
    // Use /opt/midnight/mvm/data/mvm.db if it exists, otherwise local
    let opt_path = "/opt/midnight/mvm/data/mvm.db";
    if std::path::Path::new("/opt/midnight/mvm/data").exists() {
        opt_path.to_string()
    } else {
        "./mvm.db".to_string()
    }
}

fn default_batch_size() -> u32 {
    100
}

fn default_poll_interval() -> u64 {
    6
}

fn default_refresh_interval() -> u64 {
    6000 // Match Midnight block interval of 6 seconds
}

fn default_max_retries() -> u32 {
    3
}

fn default_retry_initial_delay() -> u64 {
    1000 // 1 second
}

fn default_retry_max_delay() -> u64 {
    30000 // 30 seconds
}

impl Default for RpcConfig {
    fn default() -> Self {
        Self {
            url: default_rpc_url(),
            metrics_url: default_metrics_url(),
            node_exporter_url: None,
            timeout_ms: default_timeout(),
            max_retries: default_max_retries(),
            retry_initial_delay_ms: default_retry_initial_delay(),
            retry_max_delay_ms: default_retry_max_delay(),
        }
    }
}

impl RpcConfig {
    /// Convert to RetryConfig for use with RpcClient
    pub fn retry_config(&self) -> crate::rpc::RetryConfig {
        crate::rpc::RetryConfig {
            max_retries: self.max_retries,
            initial_delay_ms: self.retry_initial_delay_ms,
            max_delay_ms: self.retry_max_delay_ms,
            backoff_multiplier: 2.0,
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            path: default_db_path(),
        }
    }
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            batch_size: default_batch_size(),
            poll_interval_secs: default_poll_interval(),
            finalized_only: false,
            start_block: 0,
        }
    }
}

impl Default for ViewConfig {
    fn default() -> Self {
        Self {
            refresh_interval_ms: default_refresh_interval(),
            expected_ip: None,
        }
    }
}

impl Config {
    /// Load configuration from file, environment, and defaults
    /// Priority: Environment variables > Config file > Defaults
    pub fn load() -> Result<Self> {
        let mut config = Config::default();

        // Try to load from config file (multiple locations)
        if let Some((file_config, config_path)) = Self::load_from_file()? {
            tracing::info!("Loaded configuration from: {}", config_path.display());
            config = file_config;
        } else {
            tracing::info!("Using default configuration (no config file found)");
        }

        // Override with environment variables
        config.apply_env_overrides();

        Ok(config)
    }

    /// Load configuration from file (searches multiple locations)
    fn load_from_file() -> Result<Option<(Self, PathBuf)>> {
        let paths = Self::config_file_paths();

        for path in &paths {
            if path.exists() {
                let contents = fs::read_to_string(path)
                    .with_context(|| format!("Failed to read config file: {}", path.display()))?;

                let config: Config = toml::from_str(&contents)
                    .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

                return Ok(Some((config, path.clone())));
            }
        }

        Ok(None)
    }

    /// Get list of config file paths to search (in order of priority)
    pub fn config_file_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();

        // 1. Current directory
        paths.push(PathBuf::from("./mvm.toml"));

        // 2. User config directory (~/.config/mvm/config.toml)
        if let Some(proj_dirs) = ProjectDirs::from("com", "midnight", "mvm") {
            paths.push(proj_dirs.config_dir().join("config.toml"));
        }

        // 3. Install location (/opt/midnight/mvm/config/config.toml)
        paths.push(PathBuf::from("/opt/midnight/mvm/config/config.toml"));

        paths
    }

    /// Apply environment variable overrides
    fn apply_env_overrides(&mut self) {
        // RPC
        if let Ok(url) = std::env::var("MVM_RPC_URL") {
            self.rpc.url = url;
        }
        if let Ok(metrics_url) = std::env::var("MVM_METRICS_URL") {
            self.rpc.metrics_url = metrics_url;
        }
        if let Ok(node_exporter_url) = std::env::var("MVM_NODE_EXPORTER_URL") {
            self.rpc.node_exporter_url = Some(node_exporter_url);
        }

        // Database
        if let Ok(db_path) = std::env::var("MVM_DB_PATH") {
            self.database.path = db_path;
        }

        // Validator
        if let Ok(keystore) = std::env::var("MVM_KEYSTORE_PATH") {
            self.validator.keystore_path = Some(keystore);
        }
        if let Ok(label) = std::env::var("MVM_VALIDATOR_LABEL") {
            self.validator.label = Some(label);
        }

        // Sync
        if let Ok(batch_size) = std::env::var("MVM_BATCH_SIZE") {
            if let Ok(size) = batch_size.parse() {
                self.sync.batch_size = size;
            }
        }
        if let Ok(poll_interval) = std::env::var("MVM_POLL_INTERVAL") {
            if let Ok(interval) = poll_interval.parse() {
                self.sync.poll_interval_secs = interval;
            }
        }

        // Daemon
        if let Ok(pid_file) = std::env::var("MVM_PID_FILE") {
            self.daemon.pid_file = Some(pid_file);
        }

        // View
        if let Ok(expected_ip) = std::env::var("MVM_EXPECTED_IP") {
            self.view.expected_ip = Some(expected_ip);
        }

        // Chain
        if let Ok(network) = std::env::var("MVM_NETWORK") {
            self.chain.network = network;
        }
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        // Validate RPC URL
        if !self.rpc.url.starts_with("http://") && !self.rpc.url.starts_with("https://") {
            anyhow::bail!("Invalid RPC URL: {}", self.rpc.url);
        }

        // Validate batch size
        if self.sync.batch_size == 0 {
            anyhow::bail!("Batch size must be greater than 0");
        }

        Ok(())
    }

    /// Get example configuration as TOML string (for programmatic access)
    #[allow(dead_code)]
    pub fn example_toml() -> String {
        toml::to_string_pretty(&Config::default())
            .unwrap_or_else(|_| "# Error generating example".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.rpc.url, "http://localhost:9944");
        assert_eq!(config.sync.batch_size, 100);
    }

    #[test]
    fn test_config_validation() {
        let config = Config::default();
        assert!(config.validate().is_ok());
    }
}
