use anyhow::{Context, Result};
use reqwest::Client;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

static REQUEST_ID: AtomicU64 = AtomicU64::new(1);

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
}

impl RpcClient {
    /// Create a new RPC client with default timeout (30 seconds)
    pub fn new(endpoint: &str) -> Self {
        Self::with_timeout(endpoint, 30000)
    }

    /// Create a new RPC client with custom timeout in milliseconds
    pub fn with_timeout(endpoint: &str, timeout_ms: u64) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            client,
            endpoint: endpoint.to_string(),
        }
    }

    pub fn endpoint(&self) -> &str {
        &self.endpoint
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
}
