//! RPC Quorum Client
//!
//! Provides RPC client functionality with quorum-based consensus to prevent
//! single-point-of-failure or malicious provider attacks.
//!
//! When the `quorum` feature is enabled, this client makes actual HTTP JSON-RPC
//! calls to multiple providers and uses consensus to determine the correct response.

use alloc::vec::Vec;
use serde_json;
#[cfg(feature = "quorum")]
use std::collections::HashMap;

use crate::error::Result;

#[cfg(feature = "observability")]
use std::sync::{Arc, Mutex};

#[cfg(feature = "observability")]
use csv_observability::metrics::RpcMetrics;

/// RPC provider configuration
#[derive(Clone, Debug)]
pub struct RpcProvider {
    /// Provider URL
    pub url: String,
    /// Provider weight (for weighted quorum)
    pub weight: f64,
    /// Provider timeout in milliseconds
    pub timeout_ms: u64,
}

impl RpcProvider {
    /// Create a new RPC provider
    pub fn new(url: String) -> Self {
        Self {
            url,
            weight: 1.0,
            timeout_ms: 5000,
        }
    }

    /// Set the provider weight
    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = weight;
        self
    }

    /// Set the timeout
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }
}

/// Quorum configuration
#[derive(Clone, Debug)]
pub struct QuorumConfig {
    /// Minimum number of providers that must agree
    pub min_quorum: usize,
    /// Minimum percentage of providers that must agree (0.0 - 1.0)
    pub min_percentage: f64,
    /// Maximum number of providers to query
    pub max_providers: usize,
}

impl QuorumConfig {
    /// Create a new quorum configuration
    pub fn new(min_quorum: usize, min_percentage: f64) -> Self {
        Self {
            min_quorum,
            min_percentage,
            max_providers: 5,
        }
    }

    /// Set the maximum number of providers
    pub fn with_max_providers(mut self, max_providers: usize) -> Self {
        self.max_providers = max_providers;
        self
    }
}

impl Default for QuorumConfig {
    fn default() -> Self {
        Self::new(2, 0.51) // Default: 2 providers, 51% agreement
    }
}

/// RPC response from a provider
#[derive(Clone, Debug)]
pub struct RpcResponse {
    /// Provider that returned this response
    pub provider: String,
    /// Response data
    pub data: Vec<u8>,
    /// Whether the response was successful
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
    /// Response timestamp (milliseconds since epoch)
    pub timestamp_ms: u64,
    /// Response latency in milliseconds
    pub latency_ms: u64,
}

/// JSON-RPC request body
#[derive(serde::Serialize, Clone, Debug)]
#[allow(dead_code)]
struct RpcRequest {
    jsonrpc: String,
    method: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    params: Vec<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<u64>,
}

/// Internal JSON-RPC response body (from provider)
#[cfg(feature = "quorum")]
#[derive(serde::Deserialize, Clone, Debug)]
struct JsonRpcResponse {
    #[serde(rename = "jsonrpc")]
    #[allow(dead_code)]
    jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcErrorResponse>,
    #[allow(dead_code)]
    id: Option<u64>,
}

#[cfg(feature = "quorum")]
#[derive(serde::Deserialize, Clone, Debug)]
#[allow(dead_code)]
struct JsonRpcErrorResponse {
    #[allow(dead_code)]
    code: i64,
    message: String,
}

/// Quorum client for RPC queries.
///
/// When the `quorum` feature is enabled, this client makes actual HTTP
/// JSON-RPC calls to multiple providers in parallel and uses consensus
/// to determine the correct response.
#[cfg(feature = "quorum")]
#[derive(Clone, Debug)]
pub struct QuorumClient {
    /// RPC providers
    pub providers: Vec<RpcProvider>,
    config: QuorumConfig,
    http_client: reqwest::Client,
    #[cfg(feature = "observability")]
    metrics: Arc<Mutex<RpcMetrics>>,
}

/// Result of a quorum decision.
#[cfg(feature = "quorum")]
#[derive(Clone, Debug)]
pub enum QuorumDecision {
    /// A decisive consensus on a single response
    Consensus {
        /// The consensus data bytes
        data: Vec<u8>,
        /// Total weight supporting this consensus
        weight: f64,
        /// Providers that contributed to this consensus
        supporting_providers: Vec<String>,
    },
    /// No single response reached quorum; includes grouping info for diagnostics
    NoConsensus {
        /// Groups of conflicting responses with their weights and providers
        groups: Vec<(Vec<u8>, f64, usize, Vec<String>)>,
        /// Total weight across all providers
        total_weight: f64,
    },
}

#[cfg(feature = "quorum")]
impl QuorumClient {
    /// Compute a weighted quorum decision from provider responses.
    fn compute_quorum_decision(&self, responses: &[RpcResponse]) -> QuorumDecision {
        // Map provider URL to configured weight
        let mut weight_map: HashMap<String, f64> = HashMap::new();
        let mut total_weight = 0.0f64;
        for p in &self.providers {
            weight_map.insert(p.url.clone(), p.weight);
            total_weight += p.weight;
        }

        // Group responses by data bytes and sum weights
        let mut groups: HashMap<Vec<u8>, (f64, Vec<String>)> = HashMap::new();
        for r in responses.iter().filter(|r| r.success) {
            let w = *weight_map.get(&r.provider).unwrap_or(&1.0);
            let entry = groups.entry(r.data.clone()).or_insert((0.0f64, Vec::new()));
            entry.0 += w;
            entry.1.push(r.provider.clone());
        }

        // Build vector of groups
        let mut group_vec: Vec<(Vec<u8>, f64, usize, Vec<String>)> = groups
            .into_iter()
            .map(|(data, (weight, providers))| (data, weight, providers.len(), providers))
            .collect();

        // Sort by descending weight
        group_vec.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        if group_vec.is_empty() {
            return QuorumDecision::NoConsensus { groups: vec![], total_weight };
        }

        let best = &group_vec[0];
        // Check quorum thresholds
        let meets_weight = best.1 >= (self.config.min_percentage * total_weight);
        let meets_count = best.2 >= self.config.min_quorum;

        if meets_weight && meets_count {
            QuorumDecision::Consensus {
                data: best.0.clone(),
                weight: best.1,
                supporting_providers: best.3.clone(),
            }
        } else {
            QuorumDecision::NoConsensus { groups: group_vec, total_weight }
        }
    }
}

#[cfg(feature = "quorum")]
impl QuorumClient {
    /// Create a new quorum client with the given providers and configuration.
    pub fn new(providers: Vec<RpcProvider>, config: QuorumConfig) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            providers,
            config,
            http_client,
            #[cfg(feature = "observability")]
            metrics: Arc::new(Mutex::new(RpcMetrics::new())),
        }
    }

    /// Create a quorum client with default configuration.
    pub fn with_defaults(providers: Vec<RpcProvider>) -> Self {
        Self::new(providers, QuorumConfig::default())
    }

    /// Get a reference to the metrics collector.
    #[cfg(feature = "observability")]
    pub fn metrics(&self) -> Arc<Mutex<RpcMetrics>> {
        self.metrics.clone()
    }

    /// Query all providers in parallel and return responses.
    ///
    /// Each provider is queried concurrently using tokio::spawn. Providers
    /// that fail to respond within their timeout are marked as failed.
    pub async fn query_all(&self, method: &str, params: &[serde_json::Value]) -> Vec<RpcResponse> {
        let mut handles = Vec::new();

        for provider in &self.providers {
            let client = self.http_client.clone();
            let url = provider.url.clone();
            let timeout = provider.timeout_ms;
            let method = method.to_string();
            let params = params.to_vec();
            #[cfg(feature = "observability")]
            let metrics = self.metrics.clone();

            let handle = tokio::spawn(async move {
                let start = std::time::Instant::now();

                let request_body = serde_json::json!({
                    "jsonrpc": "2.0",
                    "method": method,
                    "params": params,
                    "id": 1
                });

                let result = client
                    .post(&url)
                    .header("Content-Type", "application/json")
                    .json(&request_body)
                    .timeout(std::time::Duration::from_millis(timeout))
                    .send()
                    .await;

                let latency_ms = start.elapsed().as_millis() as u64;

                match result {
                    Ok(response) => {
                        let status = response.status();
                        if !status.is_success() {
                            #[cfg(feature = "observability")]
                            {
                                if let Ok(mut m) = metrics.lock() {
                                    m.record_failure(&url);
                                }
                            }
                            return RpcResponse {
                                provider: url,
                                data: Vec::new(),
                                success: false,
                                error: Some(format!("HTTP {}: {}", status.as_u16(), status)),
                                timestamp_ms: std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_millis()
                                    as u64,
                                latency_ms,
                            };
                        }

                        let text = match response.text().await {
                            Ok(t) => t,
                            Err(e) => {
                                #[cfg(feature = "observability")]
                                {
                                    if let Ok(mut m) = metrics.lock() {
                                        m.record_failure(&url);
                                    }
                                }
                                return RpcResponse {
                                    provider: url.clone(),
                                    data: Vec::new(),
                                    success: false,
                                    error: Some(format!("Failed to read response body: {}", e)),
                                    timestamp_ms: std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap_or_default()
                                        .as_millis()
                                        as u64,
                                    latency_ms,
                                };
                            }
                        };

                        // Parse JSON-RPC response
                        let rpc_response: std::result::Result<JsonRpcResponse, serde_json::Error> =
                            serde_json::from_str(&text);

                        match rpc_response {
                            Ok(inner) => {
                                if let Some(ref err) = inner.error {
                                    #[cfg(feature = "observability")]
                                    {
                                        if let Ok(mut m) = metrics.lock() {
                                            m.record_failure(&url);
                                        }
                                    }
                                    RpcResponse {
                                        provider: url,
                                        data: Vec::new(),
                                        success: false,
                                        error: Some(err.message.clone()),
                                        timestamp_ms: std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .unwrap_or_default()
                                            .as_millis()
                                            as u64,
                                        latency_ms,
                                    }
                                } else if let Some(result) = inner.result {
                                    let data = match serde_json::to_vec(&result) {
                                        Ok(d) => d,
                                        Err(e) => {
                                            #[cfg(feature = "observability")]
                                            {
                                                if let Ok(mut m) = metrics.lock() {
                                                    m.record_failure(&url);
                                                }
                                            }
                                            return RpcResponse {
                                                provider: url,
                                                data: Vec::new(),
                                                success: false,
                                                error: Some(format!(
                                                    "Failed to serialize result: {}",
                                                    e
                                                )),
                                                timestamp_ms: std::time::SystemTime::now()
                                                    .duration_since(std::time::UNIX_EPOCH)
                                                    .unwrap_or_default()
                                                    .as_millis()
                                                    as u64,
                                                latency_ms,
                                            };
                                        }
                                    };
                                    #[cfg(feature = "observability")]
                                    {
                                        if let Ok(mut m) = metrics.lock() {
                                            m.record_success(&url, latency_ms);
                                        }
                                    }
                                    RpcResponse {
                                        provider: url,
                                        data,
                                        success: true,
                                        error: None,
                                        timestamp_ms: std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .unwrap_or_default()
                                            .as_millis()
                                            as u64,
                                        latency_ms,
                                    }
                                } else {
                                    #[cfg(feature = "observability")]
                                    {
                                        if let Ok(mut m) = metrics.lock() {
                                            m.record_failure(&url);
                                        }
                                    }
                                    RpcResponse {
                                        provider: url,
                                        data: Vec::new(),
                                        success: false,
                                        error: Some("No result in RPC response".to_string()),
                                        timestamp_ms: std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .unwrap_or_default()
                                            .as_millis()
                                            as u64,
                                        latency_ms,
                                    }
                                }
                            }
                            Err(e) => {
                                #[cfg(feature = "observability")]
                                {
                                    if let Ok(mut m) = metrics.lock() {
                                        m.record_failure(&url);
                                    }
                                }
                                RpcResponse {
                                    provider: url,
                                    data: Vec::new(),
                                    success: false,
                                    error: Some(format!("Failed to parse RPC response: {}", e)),
                                    timestamp_ms: std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap_or_default()
                                        .as_millis()
                                        as u64,
                                    latency_ms,
                                }
                            }
                        }
                    }
                    Err(e) => {
                        #[cfg(feature = "observability")]
                        {
                            if let Ok(mut m) = metrics.lock() {
                                if e.is_timeout() {
                                    m.record_timeout(&url);
                                } else {
                                    m.record_failure(&url);
                                }
                            }
                        }
                        RpcResponse {
                            provider: url,
                            data: Vec::new(),
                            success: false,
                            error: Some(format!("Request failed: {}", e)),
                            timestamp_ms: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_millis() as u64,
                            latency_ms,
                        }
                    }
                }
            });

            handles.push(handle);
        }

        // Collect all results
        let mut responses = Vec::with_capacity(handles.len());
        for handle in handles {
            match handle.await {
                Ok(response) => responses.push(response),
                Err(e) => {
                    responses.push(RpcResponse {
                        provider: String::new(),
                        data: Vec::new(),
                        success: false,
                        error: Some(format!("Task join error: {}", e)),
                        timestamp_ms: 0,
                        latency_ms: 0,
                    });
                }
            }
        }

        responses
    }

    /// Query providers and return the quorum result.
    ///
    /// Queries all providers in parallel, then checks if a quorum of responses
    /// agree on the result. Returns the consensus data if quorum is reached,
    /// or an error otherwise.
    pub async fn query_quorum(
        &self,
        method: &str,
        params: &[serde_json::Value],
    ) -> Result<Vec<u8>> {
        let responses = self.query_all(method, params).await;

        // Compute weighted quorum decision
        let decision = self.compute_quorum_decision(&responses);
        match decision {
            QuorumDecision::Consensus { data, .. } => Ok(data),
            QuorumDecision::NoConsensus { groups, total_weight } => {
                // Build diagnostic message
                let mut msg = format!("Quorum not reached. total_weight={:.2}", total_weight);
                for (i, (_d, w, cnt, providers)) in groups.iter().enumerate() {
                    let provs = providers.join(",");
                    msg.push_str(&format!("; group{} weight={:.2} count={} providers=[{}]", i, w, cnt, provs));
                    if i >= 5 {
                        break;
                    }
                }
                Err(crate::error::ProtocolError::RpcQuorumFailed(msg))
            }
        }
    }

    /// Verify that all successful responses agree on the data.
    ///
    /// Returns the consensus data if all responses match, None otherwise.
    #[allow(dead_code)]
    fn verify_consensus(&self, responses: &[&RpcResponse]) -> Option<Vec<u8>> {
        if responses.is_empty() {
            return None;
        }

        // Get the first response as reference
        let reference_data = &responses[0].data;

        // Check that all other responses match
        for response in responses.iter().skip(1) {
            if response.data != *reference_data {
                log::warn!(
                    "Quorum disagreement: provider {} returned different data than reference",
                    response.provider
                );
                #[cfg(feature = "observability")]
                {
                    if let Ok(mut m) = self.metrics.lock() {
                        m.record_disagreement();
                    }
                }
                return None;
            }
        }

        Some(reference_data.clone())
    }

    /// Query a specific method with quorum and parse JSON response.
    pub async fn query_json<T: for<'de> serde::Deserialize<'de>>(
        &self,
        method: &str,
        params: &[serde_json::Value],
    ) -> Result<T> {
        let data = self.query_quorum(method, params).await?;
        serde_json::from_slice(&data).map_err(|e| {
            crate::error::ProtocolError::InvalidData(format!("Failed to parse RPC response: {}", e))
        })
    }

    /// Get block number with quorum.
    pub async fn get_block_number(&self) -> Result<String> {
        let response: serde_json::Value = self.query_json("eth_blockNumber", &[]).await?;
        Ok(response.as_str().unwrap_or("0x0").to_string())
    }

    /// Get block by hash with quorum.
    pub async fn get_block_by_hash(&self, block_hash: &str) -> Result<serde_json::Value> {
        self.query_json(
            "eth_getBlockByHash",
            &[serde_json::json!(block_hash), serde_json::json!(false)],
        )
        .await
    }

    /// Get transaction receipt with quorum.
    pub async fn get_transaction_receipt(&self, tx_hash: &str) -> Result<serde_json::Value> {
        self.query_json("eth_getTransactionReceipt", &[serde_json::json!(tx_hash)])
            .await
    }

    /// Get the number of providers.
    pub fn provider_count(&self) -> usize {
        self.providers.len()
    }

    /// Add a provider.
    pub fn add_provider(&mut self, provider: RpcProvider) {
        self.providers.push(provider);
    }

    /// Remove a provider by URL.
    pub fn remove_provider(&mut self, url: &str) {
        self.providers.retain(|p| p.url != url);
    }
}

/// Stub implementation for when the `quorum` feature is not enabled.
#[cfg(not(feature = "quorum"))]
#[allow(dead_code, missing_docs)]
pub struct QuorumClient {
    providers: Vec<RpcProvider>,
    config: QuorumConfig,
    #[cfg(feature = "observability")]
    metrics: Arc<Mutex<RpcMetrics>>,
}

#[cfg(not(feature = "quorum"))]
#[allow(dead_code, missing_docs)]
impl QuorumClient {
    pub fn new(providers: Vec<RpcProvider>, config: QuorumConfig) -> Self {
        Self {
            providers,
            config,
            #[cfg(feature = "observability")]
            metrics: Arc::new(Mutex::new(RpcMetrics::new())),
        }
    }

    pub fn with_defaults(providers: Vec<RpcProvider>) -> Self {
        Self::new(providers, QuorumConfig::default())
    }

    /// Get a reference to the metrics collector.
    #[cfg(feature = "observability")]
    pub fn metrics(&self) -> Arc<Mutex<RpcMetrics>> {
        self.metrics.clone()
    }

    pub async fn query_all(
        &self,
        _method: &str,
        _params: &[serde_json::Value],
    ) -> Vec<RpcResponse> {
        let mut responses = Vec::new();
        for provider in &self.providers {
            responses.push(RpcResponse {
                provider: provider.url.clone(),
                data: Vec::new(),
                success: false,
                error: Some(
                    "Quorum feature not enabled. Enable the 'quorum' Cargo feature.".to_string(),
                ),
                timestamp_ms: 0,
                latency_ms: 0,
            });
        }
        responses
    }

    pub async fn query_quorum(
        &self,
        _method: &str,
        _params: &[serde_json::Value],
    ) -> Result<Vec<u8>> {
        Err(crate::error::ProtocolError::RpcQuorumFailed(
            "Quorum feature not enabled. Enable the 'quorum' Cargo feature.".to_string(),
        ))
    }

    fn verify_consensus(&self, _responses: &[&RpcResponse]) -> Option<Vec<u8>> {
        None
    }

    pub async fn query_json<T: for<'de> serde::Deserialize<'de>>(
        &self,
        _method: &str,
        _params: &[serde_json::Value],
    ) -> Result<T> {
        Err(crate::error::ProtocolError::RpcQuorumFailed(
            "Quorum feature not enabled. Enable the 'quorum' Cargo feature.".to_string(),
        ))
    }

    pub async fn get_block_number(&self) -> Result<String> {
        Err(crate::error::ProtocolError::RpcQuorumFailed(
            "Quorum feature not enabled. Enable the 'quorum' Cargo feature.".to_string(),
        ))
    }

    pub async fn get_block_by_hash(&self, _block_hash: &str) -> Result<serde_json::Value> {
        Err(crate::error::ProtocolError::RpcQuorumFailed(
            "Quorum feature not enabled. Enable the 'quorum' Cargo feature.".to_string(),
        ))
    }

    pub async fn get_transaction_receipt(&self, _tx_hash: &str) -> Result<serde_json::Value> {
        Err(crate::error::ProtocolError::RpcQuorumFailed(
            "Quorum feature not enabled. Enable the 'quorum' Cargo feature.".to_string(),
        ))
    }

    pub fn provider_count(&self) -> usize {
        self.providers.len()
    }

    pub fn add_provider(&mut self, provider: RpcProvider) {
        self.providers.push(provider);
    }

    pub fn remove_provider(&mut self, url: &str) {
        self.providers.retain(|p| p.url != url);
    }
}

/// Fault injection modes for Byzantine RPC testing.
///
/// Used in adversarial tests to simulate malicious or misbehaving RPC providers.
#[cfg(any(test, feature = "adversarial-tests"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaultyRpcMode {
    /// Returns a valid-looking but cryptographically invalid proof
    InvalidProof,
    /// Returns finality = true when block is not finalized
    WrongFinality,
    /// Returns partial/truncated state data
    PartialState,
    /// Returns Ok(vec![]) for all queries
    EmptyResult,
    /// Never responds (simulated via immediate error)
    Timeout,
    /// Returns a reorged chain view
    Reorg,
    /// Returns a receipt with wrong log data
    FakeReceipt,
}

#[cfg(any(test, feature = "adversarial-tests"))]
impl FaultyRpcMode {
    /// Returns a human-readable description of this fault mode.
    pub fn description(&self) -> &'static str {
        match self {
            Self::InvalidProof => "invalid proof",
            Self::WrongFinality => "wrong finality",
            Self::PartialState => "partial state",
            Self::EmptyResult => "empty result",
            Self::Timeout => "timeout",
            Self::Reorg => "reorg",
            Self::FakeReceipt => "fake receipt",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rpc_provider_creation() {
        let provider = RpcProvider::new("http://localhost:8545".to_string())
            .with_weight(2.0)
            .with_timeout(10000);

        assert_eq!(provider.url, "http://localhost:8545");
        assert_eq!(provider.weight, 2.0);
        assert_eq!(provider.timeout_ms, 10000);
    }

    #[test]
    fn test_quorum_config_default() {
        let config = QuorumConfig::default();
        assert_eq!(config.min_quorum, 2);
        assert_eq!(config.min_percentage, 0.51);
    }

    #[test]
    fn test_quorum_client_creation() {
        let providers = vec![
            RpcProvider::new("http://provider1.com".to_string()),
            RpcProvider::new("http://provider2.com".to_string()),
        ];

        let client = QuorumClient::with_defaults(providers);
        assert_eq!(client.provider_count(), 2);
    }

    #[test]
    fn test_faulty_rpc_mode_descriptions() {
        use super::FaultyRpcMode;

        assert_eq!(FaultyRpcMode::InvalidProof.description(), "invalid proof");
        assert_eq!(FaultyRpcMode::WrongFinality.description(), "wrong finality");
        assert_eq!(FaultyRpcMode::PartialState.description(), "partial state");
        assert_eq!(FaultyRpcMode::EmptyResult.description(), "empty result");
        assert_eq!(FaultyRpcMode::Timeout.description(), "timeout");
        assert_eq!(FaultyRpcMode::Reorg.description(), "reorg");
        assert_eq!(FaultyRpcMode::FakeReceipt.description(), "fake receipt");
    }

    #[test]
    fn test_faulty_rpc_mode_equality() {
        use super::FaultyRpcMode;

        assert_eq!(FaultyRpcMode::InvalidProof, FaultyRpcMode::InvalidProof);
        assert_ne!(FaultyRpcMode::InvalidProof, FaultyRpcMode::WrongFinality);
    }

    #[test]
    fn test_faulty_rpc_mode_clone() {
        use super::FaultyRpcMode;

        let mode = FaultyRpcMode::Reorg;
        let cloned = mode.clone();
        assert_eq!(mode, cloned);
    }
}
