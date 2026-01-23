use anyhow::{Context, Result};
use reqwest::Client;
use std::collections::HashMap;

/// Type alias for parsed Prometheus metrics: metric_name -> Vec<(labels, value)>
type ParsedMetrics = HashMap<String, Vec<(HashMap<String, String>, f64)>>;

/// Client for fetching Prometheus metrics from Substrate node
pub struct MetricsClient {
    client: Client,
    endpoint: String,
}

/// Client for fetching metrics from node_exporter
pub struct NodeExporterClient {
    client: Client,
    endpoint: String,
}

/// Metrics from node_exporter (system-level metrics)
#[derive(Debug, Default, Clone)]
pub struct NodeExporterMetrics {
    /// System load average (1 minute)
    pub load1: f64,
    /// Total system memory in bytes
    pub memory_total_bytes: u64,
    /// Available system memory in bytes
    pub memory_available_bytes: u64,
    /// Root filesystem total size in bytes
    pub disk_total_bytes: u64,
    /// Root filesystem available space in bytes
    pub disk_available_bytes: u64,
}

impl NodeExporterClient {
    pub fn new(endpoint: &str) -> Self {
        Self {
            client: Client::new(),
            endpoint: endpoint.to_string(),
        }
    }

    /// Fetch and parse node_exporter metrics
    pub async fn fetch_metrics(&self) -> Result<NodeExporterMetrics> {
        let response = self
            .client
            .get(&self.endpoint)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
            .context("Failed to fetch node_exporter metrics")?;

        let body = response
            .text()
            .await
            .context("Failed to read node_exporter metrics body")?;

        Ok(parse_node_exporter_metrics(&body))
    }
}

/// Parse node_exporter Prometheus metrics (system-wide metrics)
fn parse_node_exporter_metrics(body: &str) -> NodeExporterMetrics {
    let mut metrics = NodeExporterMetrics::default();
    let parsed = parse_prometheus_text(body);

    // System load average (1 minute)
    if let Some(value) = find_metric(&parsed, "node_load1", None) {
        metrics.load1 = value;
    }

    // System memory
    if let Some(value) = find_metric(&parsed, "node_memory_MemTotal_bytes", None) {
        metrics.memory_total_bytes = value as u64;
    }
    if let Some(value) = find_metric(&parsed, "node_memory_MemAvailable_bytes", None) {
        metrics.memory_available_bytes = value as u64;
    }

    // Root filesystem (mountpoint="/")
    if let Some(value) = find_metric(
        &parsed,
        "node_filesystem_size_bytes",
        Some(&[("mountpoint", "/")]),
    ) {
        metrics.disk_total_bytes = value as u64;
    }
    if let Some(value) = find_metric(
        &parsed,
        "node_filesystem_avail_bytes",
        Some(&[("mountpoint", "/")]),
    ) {
        metrics.disk_available_bytes = value as u64;
    }

    metrics
}

/// Parsed Prometheus metrics relevant to block production
#[derive(Debug, Default, Clone)]
pub struct BlockProductionMetrics {
    /// Total blocks constructed/proposed by this validator
    pub blocks_produced: u64,
    /// Best block height from metrics
    pub best_block: u64,
    /// Finalized block height from metrics
    pub finalized_block: u64,
    /// Number of transactions in last produced block
    pub last_block_transactions: u64,
    /// Network bandwidth in bytes (received)
    pub bandwidth_in: u64,
    /// Network bandwidth in bytes (sent)
    pub bandwidth_out: u64,
    /// Ready transactions in pool
    pub txpool_ready: u64,
    /// Transaction validations scheduled
    pub txpool_validations_scheduled: u64,
    /// Transaction validations finished
    pub txpool_validations_finished: u64,
    /// Process start time (for uptime calculation)
    pub process_start_time: f64,
    /// Is this node a Grandpa voter
    pub grandpa_voter: bool,

    // Peer connection metrics (Prometheus-based, more accurate than RPC)
    /// Total inbound connections opened
    pub connections_in_opened: u64,
    /// Total inbound connections closed
    pub connections_in_closed: u64,
    /// Total outbound connections opened
    pub connections_out_opened: u64,
    /// Total outbound connections closed
    pub connections_out_closed: u64,
    /// Number of discovered peers
    pub peers_discovered: u64,
    /// Number of pending connections
    pub pending_connections: u64,
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

    // Network bandwidth - substrate_sub_libp2p_network_bytes_total
    if let Some(value) = find_metric(
        &parsed,
        "substrate_sub_libp2p_network_bytes_total",
        Some(&[("direction", "in")]),
    ) {
        metrics.bandwidth_in = value as u64;
    }
    if let Some(value) = find_metric(
        &parsed,
        "substrate_sub_libp2p_network_bytes_total",
        Some(&[("direction", "out")]),
    ) {
        metrics.bandwidth_out = value as u64;
    }

    // Transaction pool metrics
    if let Some(value) = find_metric(&parsed, "substrate_ready_transactions_number", None) {
        metrics.txpool_ready = value as u64;
    }
    if let Some(value) = find_metric(&parsed, "substrate_sub_txpool_validations_scheduled", None) {
        metrics.txpool_validations_scheduled = value as u64;
    }
    if let Some(value) = find_metric(&parsed, "substrate_sub_txpool_validations_finished", None) {
        metrics.txpool_validations_finished = value as u64;
    }

    // Process uptime
    if let Some(value) = find_metric(&parsed, "substrate_process_start_time_seconds", None) {
        metrics.process_start_time = value;
    }

    // Grandpa voter status - infer from prevotes cast (if > 0, we're a voter)
    if let Some(value) = find_metric(&parsed, "substrate_finality_grandpa_prevotes_total", None) {
        metrics.grandpa_voter = value > 0.0;
    }

    // Peer connection metrics (more accurate than RPC-based counting)
    if let Some(value) = find_metric(
        &parsed,
        "substrate_sub_libp2p_connections_opened_total",
        Some(&[("direction", "in")]),
    ) {
        metrics.connections_in_opened = value as u64;
    }
    // Use sum_metric for closed connections since they have multiple reason labels
    metrics.connections_in_closed = sum_metric(
        &parsed,
        "substrate_sub_libp2p_connections_closed_total",
        Some(&[("direction", "in")]),
    ) as u64;

    if let Some(value) = find_metric(
        &parsed,
        "substrate_sub_libp2p_connections_opened_total",
        Some(&[("direction", "out")]),
    ) {
        metrics.connections_out_opened = value as u64;
    }
    // Use sum_metric for closed connections since they have multiple reason labels
    metrics.connections_out_closed = sum_metric(
        &parsed,
        "substrate_sub_libp2p_connections_closed_total",
        Some(&[("direction", "out")]),
    ) as u64;
    if let Some(value) = find_metric(&parsed, "substrate_sub_libp2p_peerset_num_discovered", None) {
        metrics.peers_discovered = value as u64;
    }
    if let Some(value) = find_metric(&parsed, "substrate_sub_libp2p_pending_connections", None) {
        metrics.pending_connections = value as u64;
    }

    metrics
}

/// Simple Prometheus text format parser
/// Returns a map of metric_name -> Vec<(labels, value)>
fn parse_prometheus_text(body: &str) -> ParsedMetrics {
    let mut result: ParsedMetrics = HashMap::new();

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

            result.entry(name).or_default().push((labels, value));
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
    parsed: &ParsedMetrics,
    name: &str,
    label_filters: Option<&[(&str, &str)]>,
) -> Option<f64> {
    let entries = parsed.get(name)?;

    for (labels, value) in entries {
        let matches = match label_filters {
            Some(filters) => filters
                .iter()
                .all(|(k, v)| labels.get(*k).map(|lv| lv == *v).unwrap_or(false)),
            None => true,
        };

        if matches {
            return Some(*value);
        }
    }

    None
}

/// Sum all metric values matching the label filters
/// Used for metrics with multiple label dimensions (e.g., connections_closed has both direction and reason)
fn sum_metric(parsed: &ParsedMetrics, name: &str, label_filters: Option<&[(&str, &str)]>) -> f64 {
    let entries = match parsed.get(name) {
        Some(e) => e,
        None => return 0.0,
    };

    entries
        .iter()
        .filter(|(labels, _)| match label_filters {
            Some(filters) => filters
                .iter()
                .all(|(k, v)| labels.get(*k).map(|lv| lv == *v).unwrap_or(false)),
            None => true,
        })
        .map(|(_, value)| *value)
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_peers_discovered() {
        let body = r#"
# HELP substrate_sub_libp2p_peerset_num_discovered Number of nodes stored in the peerset manager
# TYPE substrate_sub_libp2p_peerset_num_discovered gauge
substrate_sub_libp2p_peerset_num_discovered{chain="testnet-02"} 109
substrate_sub_libp2p_pending_connections{chain="testnet-02"} 5
"#;
        let metrics = parse_metrics(body);
        assert_eq!(metrics.peers_discovered, 109);
        assert_eq!(metrics.pending_connections, 5);
    }
}
