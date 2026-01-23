use anyhow::{Context, Result};
use reqwest::Client;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tracing::warn;

static REQUEST_ID: AtomicU64 = AtomicU64::new(1);

/// Configuration for RPC retry behavior
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts (0 = no retries)
    pub max_retries: u32,
    /// Initial delay between retries in milliseconds
    pub initial_delay_ms: u64,
    /// Maximum delay between retries in milliseconds
    pub max_delay_ms: u64,
    /// Multiplier for exponential backoff
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 1000,
            max_delay_ms: 30000,
            backoff_multiplier: 2.0,
        }
    }
}

#[derive(Debug, Serialize)]
struct JsonRpcRequest<P: Serialize> {
    jsonrpc: &'static str,
    id: u64,
    method: String,
    params: P,
}

#[derive(Debug, Deserialize)]
struct JsonRpcResponse<T> {
    #[allow(dead_code)]
    jsonrpc: String,
    #[allow(dead_code)]
    id: u64,
    result: Option<T>,
    error: Option<JsonRpcError>,
}

#[derive(Debug, Deserialize)]
struct JsonRpcError {
    code: i64,
    message: String,
}

#[derive(Clone)]
pub struct RpcClient {
    client: Client,
    endpoint: String,
    retry_config: RetryConfig,
}

impl RpcClient {
    /// Create a new RPC client with default timeout (30 seconds)
    #[allow(dead_code)]
    pub fn new(endpoint: &str) -> Self {
        Self::with_timeout(endpoint, 30000)
    }

    /// Create a new RPC client with custom timeout in milliseconds
    pub fn with_timeout(endpoint: &str, timeout_ms: u64) -> Self {
        Self::with_config(endpoint, timeout_ms, RetryConfig::default())
    }

    /// Create a new RPC client with custom timeout and retry configuration
    pub fn with_config(endpoint: &str, timeout_ms: u64, retry_config: RetryConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            client,
            endpoint: endpoint.to_string(),
            retry_config,
        }
    }

    #[allow(dead_code)]
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Set retry configuration
    #[allow(dead_code)]
    pub fn with_retry_config(mut self, config: RetryConfig) -> Self {
        self.retry_config = config;
        self
    }

    pub async fn call<P, R>(&self, method: &str, params: P) -> Result<R>
    where
        P: Serialize,
        R: DeserializeOwned,
    {
        let request = JsonRpcRequest {
            jsonrpc: "2.0",
            id: REQUEST_ID.fetch_add(1, Ordering::SeqCst),
            method: method.to_string(),
            params,
        };

        let response = self
            .client
            .post(&self.endpoint)
            .json(&request)
            .send()
            .await
            .context("Failed to send RPC request")?;

        let rpc_response: JsonRpcResponse<R> = response
            .json()
            .await
            .context("Failed to parse RPC response")?;

        if let Some(error) = rpc_response.error {
            anyhow::bail!("RPC error {}: {}", error.code, error.message);
        }

        rpc_response
            .result
            .context("RPC response missing result field")
    }

    /// Make an RPC call with automatic retry on transient failures
    pub async fn call_with_retry<P, R>(&self, method: &str, params: P) -> Result<R>
    where
        P: Serialize + Clone,
        R: DeserializeOwned,
    {
        let mut delay = self.retry_config.initial_delay_ms;
        let mut attempts = 0;

        loop {
            match self.call(method, params.clone()).await {
                Ok(result) => return Ok(result),
                Err(e) if Self::is_retryable(&e) && attempts < self.retry_config.max_retries => {
                    attempts += 1;
                    warn!(
                        "RPC call '{}' failed (attempt {}/{}), retrying in {}ms: {}",
                        method, attempts, self.retry_config.max_retries, delay, e
                    );
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                    delay = ((delay as f64) * self.retry_config.backoff_multiplier) as u64;
                    delay = delay.min(self.retry_config.max_delay_ms);
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Determine if an error is retryable (transient network issue)
    fn is_retryable(error: &anyhow::Error) -> bool {
        let err_str = error.to_string().to_lowercase();

        // Retry on transient network errors
        if err_str.contains("connection refused")
            || err_str.contains("connection reset")
            || err_str.contains("connection closed")
            || err_str.contains("timed out")
            || err_str.contains("timeout")
            || err_str.contains("temporarily unavailable")
            || err_str.contains("try again")
            || err_str.contains("503")
            || err_str.contains("502")
            || err_str.contains("504")
            || err_str.contains("service unavailable")
            || err_str.contains("bad gateway")
            || err_str.contains("gateway timeout")
        {
            return true;
        }

        // Don't retry on definitive errors
        if err_str.contains("method not found")
            || err_str.contains("invalid params")
            || err_str.contains("parse error")
            || err_str.contains("invalid request")
            || err_str.contains("400")
            || err_str.contains("401")
            || err_str.contains("403")
            || err_str.contains("404")
        {
            return false;
        }

        // Default: don't retry unknown errors
        false
    }
}
