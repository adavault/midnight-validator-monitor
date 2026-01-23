//! Alert system for monitoring block production
//!
//! Tracks block production performance and generates alerts when
//! validators are underperforming their expected block production.
//!
//! Note: This module is kept for future integration with the sync command.
//! See BACKLOG.md "Pending Integration" section.

#![allow(dead_code)]

use crate::config::AlertConfig;
use anyhow::Result;
use std::collections::HashMap;
use std::time::Instant;
use tracing::{error, info, warn};

/// Alert state for a validator
#[derive(Debug, Clone)]
pub struct ValidatorAlertState {
    /// Sidechain key of the validator
    pub sidechain_key: String,
    /// Blocks produced in current tracking period
    pub blocks_produced: u64,
    /// Expected blocks based on committee seats
    pub expected_blocks: f64,
    /// Committee seats for this validator
    pub committee_seats: u32,
    /// Last alert time (for cooldown)
    pub last_alert: Option<Instant>,
}

/// Block production alert manager
pub struct AlertManager {
    /// Configuration
    config: AlertConfig,
    /// Per-validator alert state
    validator_states: HashMap<String, ValidatorAlertState>,
    /// Current sidechain epoch being tracked
    current_epoch: u64,
}

impl AlertManager {
    /// Create a new alert manager
    pub fn new(config: AlertConfig) -> Self {
        Self {
            config,
            validator_states: HashMap::new(),
            current_epoch: 0,
        }
    }

    /// Check if alerts are enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Reset tracking for a new epoch
    pub fn reset_epoch(&mut self, epoch: u64) {
        if epoch != self.current_epoch {
            info!("Alert tracking: resetting for sidechain epoch {}", epoch);
            self.validator_states.clear();
            self.current_epoch = epoch;
        }
    }

    /// Update validator state with new block production data
    pub fn update_validator(
        &mut self,
        sidechain_key: &str,
        blocks_produced: u64,
        expected_blocks: f64,
        committee_seats: u32,
    ) {
        let state = self
            .validator_states
            .entry(sidechain_key.to_string())
            .or_insert_with(|| ValidatorAlertState {
                sidechain_key: sidechain_key.to_string(),
                blocks_produced: 0,
                expected_blocks: 0.0,
                committee_seats: 0,
                last_alert: None,
            });

        state.blocks_produced = blocks_produced;
        state.expected_blocks = expected_blocks;
        state.committee_seats = committee_seats;
    }

    /// Check all validators and generate alerts if needed
    pub async fn check_alerts(&mut self) -> Vec<BlockProductionAlert> {
        if !self.config.enabled {
            return Vec::new();
        }

        let mut alerts = Vec::new();
        let threshold = self.config.threshold_percent as f64 / 100.0;
        let min_expected = self.config.min_expected_blocks as f64;
        let cooldown = std::time::Duration::from_secs(self.config.cooldown_secs);

        for state in self.validator_states.values_mut() {
            // Skip if not enough expected blocks (avoid early epoch false positives)
            if state.expected_blocks < min_expected {
                continue;
            }

            // Skip if in committee but has 0 seats (shouldn't happen)
            if state.committee_seats == 0 {
                continue;
            }

            // Calculate production ratio
            let ratio = state.blocks_produced as f64 / state.expected_blocks;

            // Check if below threshold
            if ratio < threshold {
                // Check cooldown
                let should_alert = state
                    .last_alert
                    .map(|t| t.elapsed() >= cooldown)
                    .unwrap_or(true);

                if should_alert {
                    let alert = BlockProductionAlert {
                        sidechain_key: state.sidechain_key.clone(),
                        epoch: self.current_epoch,
                        blocks_produced: state.blocks_produced,
                        expected_blocks: state.expected_blocks,
                        committee_seats: state.committee_seats,
                        production_ratio: ratio,
                        threshold,
                    };

                    // Log the alert
                    warn!(
                        "ALERT: Validator {} is underperforming: {} blocks produced vs {:.1} expected ({:.1}% of {:.0}% threshold)",
                        truncate_key(&state.sidechain_key),
                        state.blocks_produced,
                        state.expected_blocks,
                        ratio * 100.0,
                        threshold * 100.0
                    );

                    // Send webhook if configured
                    if let Some(ref url) = self.config.webhook_url {
                        if let Err(e) = send_webhook_alert(url, &alert).await {
                            error!("Failed to send webhook alert: {}", e);
                        }
                    }

                    state.last_alert = Some(Instant::now());
                    alerts.push(alert);
                }
            }
        }

        alerts
    }

    /// Get current status for all tracked validators
    pub fn get_status(&self) -> Vec<&ValidatorAlertState> {
        self.validator_states.values().collect()
    }
}

/// Block production alert
#[derive(Debug, Clone)]
pub struct BlockProductionAlert {
    pub sidechain_key: String,
    pub epoch: u64,
    pub blocks_produced: u64,
    pub expected_blocks: f64,
    pub committee_seats: u32,
    pub production_ratio: f64,
    pub threshold: f64,
}

/// Send an alert via webhook
async fn send_webhook_alert(url: &str, alert: &BlockProductionAlert) -> Result<()> {
    let client = reqwest::Client::new();

    let payload = serde_json::json!({
        "alert_type": "block_production_low",
        "validator": truncate_key(&alert.sidechain_key),
        "epoch": alert.epoch,
        "blocks_produced": alert.blocks_produced,
        "expected_blocks": alert.expected_blocks,
        "committee_seats": alert.committee_seats,
        "production_ratio": alert.production_ratio,
        "threshold": alert.threshold,
        "message": format!(
            "Validator {} is underperforming: {} blocks vs {:.1} expected ({:.1}%)",
            truncate_key(&alert.sidechain_key),
            alert.blocks_produced,
            alert.expected_blocks,
            alert.production_ratio * 100.0
        )
    });

    let response = client.post(url).json(&payload).send().await?;

    if !response.status().is_success() {
        anyhow::bail!("Webhook returned status: {}", response.status());
    }

    info!("Sent block production alert to webhook");
    Ok(())
}

/// Truncate a key for display
fn truncate_key(key: &str) -> String {
    if key.len() > 16 {
        format!("{}...{}", &key[..8], &key[key.len() - 4..])
    } else {
        key.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alert_threshold() {
        let config = AlertConfig {
            enabled: true,
            threshold_percent: 80,
            min_expected_blocks: 5,
            webhook_url: None,
            cooldown_secs: 0,
        };

        let mut manager = AlertManager::new(config);
        manager.current_epoch = 100;

        // Validator producing well above threshold (90%)
        manager.update_validator("0xaaa", 9, 10.0, 10);

        // Validator producing below threshold (50%)
        manager.update_validator("0xbbb", 5, 10.0, 10);

        // Validator with too few expected blocks (should not alert)
        manager.update_validator("0xccc", 1, 3.0, 3);
    }
}
