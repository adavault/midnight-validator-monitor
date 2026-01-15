use anyhow::{Context, Result};
use reqwest::Client;
use std::collections::HashMap;

/// Client for fetching Prometheus metrics
pub struct MetricsClient {
    client: Client,
    endpoint: String,
}

/// Parsed Prometheus metrics relevant to block production
#[derive(Debug, Default)]
pub struct BlockProductionMetrics {
    /// Total blocks constructed/proposed by this validator
    pub blocks_produced: u64,
    /// Best block height from metrics
    pub best_block: u64,
    /// Finalized block height from metrics
    pub finalized_block: u64,
    /// Number of transactions in last produced block
    pub last_block_transactions: u64,
}

impl MetricsClient {
    pub fn new(endpoint: &str) -> Self {
        Self {
            client: Client::new(),
            endpoint: endpoint.to_string(),
        }
    }

    /// Fetch and parse Prometheus metrics
    pub async fn fetch_metrics(&self) -> Result<BlockProductionMetrics> {
        let response = self
            .client
            .get(&self.endpoint)
            .send()
            .await
            .context("Failed to fetch metrics")?;

        let body = response
            .text()
            .await
            .context("Failed to read metrics body")?;

        Ok(parse_metrics(&body))
    }
}

/// Parse Prometheus text format into our metrics struct
fn parse_metrics(body: &str) -> BlockProductionMetrics {
    let mut metrics = BlockProductionMetrics::default();
    let parsed = parse_prometheus_text(body);

    // Blocks produced by this validator
    // substrate_proposer_block_constructed_count{chain="testnet-02"} 1
    if let Some(value) = find_metric(&parsed, "substrate_proposer_block_constructed_count", None) {
        metrics.blocks_produced = value as u64;
    }

    // Block heights from substrate_block_height gauge
    if let Some(value) = find_metric(
        &parsed,
        "substrate_block_height",
        Some(&[("status", "best")]),
    ) {
        metrics.best_block = value as u64;
    }

    if let Some(value) = find_metric(
        &parsed,
        "substrate_block_height",
        Some(&[("status", "finalized")]),
    ) {
        metrics.finalized_block = value as u64;
    }

    // Transactions in last produced block
    if let Some(value) = find_metric(&parsed, "substrate_proposer_number_of_transactions", None) {
        metrics.last_block_transactions = value as u64;
    }

    metrics
}

/// Simple Prometheus text format parser
/// Returns a map of metric_name -> Vec<(labels, value)>
fn parse_prometheus_text(body: &str) -> HashMap<String, Vec<(HashMap<String, String>, f64)>> {
    let mut result: HashMap<String, Vec<(HashMap<String, String>, f64)>> = HashMap::new();

    for line in body.lines() {
        let line = line.trim();

        // Skip comments and empty lines
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Parse: metric_name{label="value",...} value
        // or: metric_name value
        if let Some((name_labels, value_str)) = line.rsplit_once(' ') {
            let value: f64 = match value_str.parse() {
                Ok(v) => v,
                Err(_) => continue,
            };

            let (name, labels) = if let Some(brace_start) = name_labels.find('{') {
                let name = &name_labels[..brace_start];
                let labels_str = &name_labels[brace_start + 1..name_labels.len() - 1];
                let labels = parse_labels(labels_str);
                (name.to_string(), labels)
            } else {
                (name_labels.to_string(), HashMap::new())
            };

            result
                .entry(name)
                .or_default()
                .push((labels, value));
        }
    }

    result
}

/// Parse label string like: label1="value1",label2="value2"
fn parse_labels(labels_str: &str) -> HashMap<String, String> {
    let mut labels = HashMap::new();

    // Simple parser - handles basic cases
    let mut remaining = labels_str;
    while !remaining.is_empty() {
        // Find key=
        if let Some(eq_pos) = remaining.find('=') {
            let key = remaining[..eq_pos].trim();
            remaining = &remaining[eq_pos + 1..];

            // Find quoted value
            if remaining.starts_with('"') {
                remaining = &remaining[1..];
                if let Some(end_quote) = remaining.find('"') {
                    let value = &remaining[..end_quote];
                    labels.insert(key.to_string(), value.to_string());
                    remaining = &remaining[end_quote + 1..];

                    // Skip comma if present
                    remaining = remaining.trim_start_matches(',');
                } else {
                    break;
                }
            } else {
                break;
            }
        } else {
            break;
        }
    }

    labels
}

/// Find a metric value by name and optional label filters
fn find_metric(
    parsed: &HashMap<String, Vec<(HashMap<String, String>, f64)>>,
    name: &str,
    label_filters: Option<&[(&str, &str)]>,
) -> Option<f64> {
    let entries = parsed.get(name)?;

    for (labels, value) in entries {
        let matches = match label_filters {
            Some(filters) => filters.iter().all(|(k, v)| {
                labels.get(*k).map(|lv| lv == *v).unwrap_or(false)
            }),
            None => true,
        };

        if matches {
            return Some(*value);
        }
    }

    None
}
