//! Chain API service for querying balances from different blockchains.
//!
//! This module provides a unified interface to query on-chain balances
//! across Bitcoin, Ethereum, Sui, and Aptos using wasm-compatible HTTP requests.

use csv_adapter_core::Chain;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::services::network::NetworkType;

/// Configuration for a chain API endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainConfig {
    /// API base URL for the chain.
    pub api_url: String,
    /// Whether this is a testnet connection.
    pub is_testnet: bool,
}

impl ChainConfig {
    /// Create a new chain configuration.
    pub fn new(api_url: impl Into<String>, is_testnet: bool) -> Self {
        Self {
            api_url: api_url.into(),
            is_testnet,
        }
    }

    /// Get the default configuration for a chain and network type.
    pub fn for_chain(chain: Chain, network: NetworkType) -> Self {
        let is_testnet = network.is_testnet();
        let api_url = match chain {
            Chain::Bitcoin => {
                if is_testnet {
                    "https://mempool.space/testnet/api".to_string()
                } else {
                    "https://mempool.space/api".to_string()
                }
            }
            Chain::Ethereum => {
                if is_testnet {
                    "https://rpc.sepolia.org".to_string()
                } else {
                    "https://rpc.ankr.com/eth".to_string()
                }
            }
            Chain::Sui => {
                if is_testnet {
                    "https://fullnode.testnet.sui.io:443".to_string()
                } else {
                    "https://fullnode.mainnet.sui.io:443".to_string()
                }
            }
            Chain::Aptos => {
                if is_testnet {
                    "https://fullnode.testnet.aptoslabs.com/v1".to_string()
                } else {
                    "https://fullnode.mainnet.aptoslabs.com/v1".to_string()
                }
            }
        };
        Self::new(api_url, is_testnet)
    }
}

/// Error type for chain API operations.
#[derive(Debug, thiserror::Error)]
pub enum ChainApiError {
    /// HTTP request failed.
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    /// JSON parsing failed.
    #[error("JSON parsing failed: {0}")]
    JsonError(#[from] serde_json::Error),

    /// Invalid address format.
    #[error("Invalid address for chain: {0}")]
    InvalidAddress(String),

    /// API returned an error.
    #[error("Chain API error: {0}")]
    ApiError(String),
}

/// Service for querying chain balances via external APIs.
pub struct ChainApi {
    /// HTTP client for requests.
    client: Client,
    /// Chain configurations.
    configs: std::collections::HashMap<Chain, ChainConfig>,
}

impl ChainApi {
    /// Create a new ChainApi with default configurations.
    pub fn new() -> Result<Self, ChainApiError> {
        let client = Client::builder()
            .build()
            .map_err(|e| ChainApiError::HttpError(e))?;

        let mut configs = std::collections::HashMap::new();
        configs.insert(
            Chain::Bitcoin,
            ChainConfig::for_chain(Chain::Bitcoin, NetworkType::Testnet),
        );
        configs.insert(
            Chain::Ethereum,
            ChainConfig::for_chain(Chain::Ethereum, NetworkType::Testnet),
        );
        configs.insert(
            Chain::Sui,
            ChainConfig::for_chain(Chain::Sui, NetworkType::Testnet),
        );
        configs.insert(
            Chain::Aptos,
            ChainConfig::for_chain(Chain::Aptos, NetworkType::Testnet),
        );

        Ok(Self { client, configs })
    }

    /// Create a new ChainApi with a custom HTTP client.
    pub fn with_client(client: Client) -> Self {
        let mut configs = std::collections::HashMap::new();
        configs.insert(
            Chain::Bitcoin,
            ChainConfig::for_chain(Chain::Bitcoin, NetworkType::Testnet),
        );
        configs.insert(
            Chain::Ethereum,
            ChainConfig::for_chain(Chain::Ethereum, NetworkType::Testnet),
        );
        configs.insert(
            Chain::Sui,
            ChainConfig::for_chain(Chain::Sui, NetworkType::Testnet),
        );
        configs.insert(
            Chain::Aptos,
            ChainConfig::for_chain(Chain::Aptos, NetworkType::Testnet),
        );

        Self { client, configs }
    }

    /// Update the configuration for a chain.
    pub fn set_config(&mut self, chain: Chain, config: ChainConfig) {
        self.configs.insert(chain, config);
    }

    /// Get the configuration for a chain.
    pub fn get_config(&self, chain: Chain) -> Option<&ChainConfig> {
        self.configs.get(&chain)
    }

    /// Get balance for an address on a specific chain.
    ///
    /// Returns the balance as a float (BTC, ETH, SUI, or APT depending on chain).
    pub async fn get_balance(&self, chain: Chain, address: &str) -> Result<f64, ChainApiError> {
        match chain {
            Chain::Bitcoin => self.get_bitcoin_balance(address).await,
            Chain::Ethereum => self.get_ethereum_balance(address).await,
            Chain::Sui => self.get_sui_balance(address).await,
            Chain::Aptos => self.get_aptos_balance(address).await,
        }
    }

    /// Query Bitcoin balance via mempool.space API.
    async fn get_bitcoin_balance(&self, address: &str) -> Result<f64, ChainApiError> {
        let config = self
            .configs
            .get(&Chain::Bitcoin)
            .ok_or_else(|| ChainApiError::ApiError("Bitcoin config not found".to_string()))?;

        let url = format!("{}/address/{}", config.api_url, address);
        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(ChainApiError::ApiError(format!(
                "Bitcoin API error: {}",
                response.status()
            )));
        }

        // mempool.space returns: { chain_stats: { funded_txo_sum, spent_txo_sum }, ... }
        let json: serde_json::Value = response.json().await?;
        let stats = json
            .get("chain_stats")
            .ok_or_else(|| ChainApiError::ApiError("Missing chain_stats in response".to_string()))?;

        let funded = stats
            .get("funded_txo_sum")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let spent = stats
            .get("spent_txo_sum")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        // Balance in satoshis, convert to BTC
        let balance_sats = funded - spent;
        Ok(balance_sats / 100_000_000.0)
    }

    /// Query Ethereum balance via JSON-RPC eth_getBalance.
    async fn get_ethereum_balance(&self, address: &str) -> Result<f64, ChainApiError> {
        let config = self
            .configs
            .get(&Chain::Ethereum)
            .ok_or_else(|| ChainApiError::ApiError("Ethereum config not found".to_string()))?;

        // Ensure address has 0x prefix
        let addr = if address.starts_with("0x") {
            address.to_string()
        } else {
            format!("0x{}", address)
        };

        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_getBalance",
            "params": [addr, "latest"],
            "id": 1
        });

        let response = self
            .client
            .post(&config.api_url)
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(ChainApiError::ApiError(format!(
                "Ethereum API error: {}",
                response.status()
            )));
        }

        let json: serde_json::Value = response.json().await?;
        let balance_hex = json
            .get("result")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ChainApiError::ApiError("Missing result in response".to_string()))?;

        // Parse hex balance (in wei) to f64 (in ETH).
        // Use u128 to avoid overflow for balances > 18.4 ETH.
        let balance_wei = u128::from_str_radix(balance_hex.trim_start_matches("0x"), 16)
            .map_err(|e| ChainApiError::ApiError(format!("Invalid hex balance: {}", e)))?;

        Ok(balance_wei as f64 / 1e18)
    }

    /// Query Sui balance via JSON-RPC suix_getBalance.
    async fn get_sui_balance(&self, address: &str) -> Result<f64, ChainApiError> {
        let config = self
            .configs
            .get(&Chain::Sui)
            .ok_or_else(|| ChainApiError::ApiError("Sui config not found".to_string()))?;

        // Sui uses coin type 0x2::sui::SUI::SUI for native SUI
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "suix_getBalance",
            "params": {
                "owner": address,
                "coin_type": "0x2::sui::SUI::SUI"
            }
        });

        let response = self
            .client
            .post(&config.api_url)
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(ChainApiError::ApiError(format!(
                "Sui API error: {}",
                response.status()
            )));
        }

        let json: serde_json::Value = response.json().await?;
        let total_balance = json
            .get("result")
            .and_then(|v| v.get("totalBalance"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| ChainApiError::ApiError("Missing totalBalance in response".to_string()))?;

        // Parse balance (in MIST) to SUI (1 SUI = 10^9 MIST)
        let balance_mist: f64 = total_balance
            .parse()
            .map_err(|e| ChainApiError::ApiError(format!("Invalid balance: {}", e)))?;

        Ok(balance_mist / 1e9)
    }

    /// Query Aptos balance via REST API.
    async fn get_aptos_balance(&self, address: &str) -> Result<f64, ChainApiError> {
        let config = self
            .configs
            .get(&Chain::Aptos)
            .ok_or_else(|| ChainApiError::ApiError("Aptos config not found".to_string()))?;

        // Get account resource for CoinStore
        let resource_type = "0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>";
        let url = format!(
            "{}/accounts/{}/resource/{}",
            config.api_url,
            address,
            resource_type
        );

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            // Account may not exist (zero balance)
            if response.status() == reqwest::StatusCode::NOT_FOUND {
                return Ok(0.0);
            }
            return Err(ChainApiError::ApiError(format!(
                "Aptos API error: {}",
                response.status()
            )));
        }

        let json: serde_json::Value = response.json().await?;
        let balance_str = json
            .get("data")
            .and_then(|v| v.get("coin"))
            .and_then(|v| v.get("value"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| ChainApiError::ApiError("Missing balance in response".to_string()))?;

        let balance_octas: f64 = balance_str
            .parse()
            .map_err(|e| ChainApiError::ApiError(format!("Invalid balance: {}", e)))?;

        // 1 APT = 10^8 octas
        Ok(balance_octas / 1e8)
    }
}

impl Default for ChainApi {
    fn default() -> Self {
        Self::with_client(Client::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chain_config_for_chain() {
        let btc_testnet = ChainConfig::for_chain(Chain::Bitcoin, NetworkType::Testnet);
        assert!(btc_testnet.is_testnet);
        assert!(btc_testnet.api_url.contains("testnet"));

        let btc_mainnet = ChainConfig::for_chain(Chain::Bitcoin, NetworkType::Mainnet);
        assert!(!btc_mainnet.is_testnet);
        assert!(!btc_mainnet.api_url.contains("testnet"));

        let eth_testnet = ChainConfig::for_chain(Chain::Ethereum, NetworkType::Testnet);
        assert!(eth_testnet.api_url.contains("sepolia"));
    }

    #[test]
    fn test_chain_config_new() {
        let config = ChainConfig::new("https://example.com", true);
        assert_eq!(config.api_url, "https://example.com");
        assert!(config.is_testnet);
    }

    #[test]
    fn test_chain_api_default() {
        let api = ChainApi::default();
        assert!(api.get_config(Chain::Bitcoin).is_some());
        assert!(api.get_config(Chain::Ethereum).is_some());
        assert!(api.get_config(Chain::Sui).is_some());
        assert!(api.get_config(Chain::Aptos).is_some());
    }
}
