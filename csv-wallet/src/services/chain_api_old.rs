//! Chain API service for querying balances from different blockchains.
//!
//! This module provides a unified interface to query on-chain balances
//! across Bitcoin, Ethereum, Sui, and Aptos using wasm-compatible HTTP requests.

use csv_adapter_core::Chain;
use csv_adapter_core::agent_types::{HasErrorSuggestion, FixAction, error_codes};
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
                // Use signet for testnet (consistent with csv-cli)
                if is_testnet {
                    "https://mempool.space/signet/api".to_string()
                } else {
                    "https://mempool.space/api".to_string()
                }
            }
            Chain::Ethereum => {
                if is_testnet {
                    "https://ethereum-sepolia-rpc.publicnode.com".to_string()
                } else {
                    "https://ethereum-rpc.publicnode.com".to_string()
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
            Chain::Solana => {
                if is_testnet {
                    "https://api.devnet.solana.com".to_string()
                } else {
                    "https://api.mainnet-beta.solana.com".to_string()
                }
            }
            _ => {
                // Default to testnet for unknown chains
                "https://rpc.sepolia.org".to_string()
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

impl HasErrorSuggestion for ChainApiError {
    fn error_code(&self) -> &'static str {
        match self {
            ChainApiError::HttpError(_) => error_codes::WALLET_CHAIN_API_HTTP,
            ChainApiError::JsonError(_) => error_codes::WALLET_CHAIN_API_JSON,
            ChainApiError::InvalidAddress(_) => error_codes::WALLET_CHAIN_API_INVALID_ADDRESS,
            ChainApiError::ApiError(_) => error_codes::WALLET_CHAIN_API_ERROR,
        }
    }

    fn description(&self) -> String {
        self.to_string()
    }

    fn suggested_fix(&self) -> String {
        match self {
            ChainApiError::HttpError(_) => {
                "HTTP request to chain API failed. Check: \
                 1) Your internet connection, 2) The API endpoint is accessible, \
                 3) You're not being blocked by CORS (browser) or firewalls. \
                 Try a different RPC provider if the issue persists.".to_string()
            }
            ChainApiError::JsonError(_) => {
                "Failed to parse API response. The API format may have changed \
                 or the response is malformed. Try: 1) A different API endpoint, \
                 2) Updating to the latest SDK version.".to_string()
            }
            ChainApiError::InvalidAddress(chain) => {
                format!(
                    "Invalid address format for {}. Ensure the address matches \
                     the chain's format (e.g., 0x... for Ethereum, bc1... for Bitcoin).",
                    chain
                )
            }
            ChainApiError::ApiError(msg) => {
                format!(
                    "Chain API returned an error: {}. \
                     Check the API documentation for this specific error. \
                     The endpoint may be temporarily unavailable.",
                    msg
                )
            }
        }
    }

    fn docs_url(&self) -> String {
        error_codes::docs_url(self.error_code())
    }

    fn fix_action(&self) -> Option<FixAction> {
        match self {
            ChainApiError::HttpError(_) | ChainApiError::ApiError(_) => {
                Some(FixAction::Retry {
                    parameter_changes: std::collections::HashMap::from([
                        ("rpc_endpoint".to_string(), "try_alternative".to_string()),
                    ]),
                })
            }
            ChainApiError::JsonError(_) => {
                Some(FixAction::CheckState {
                    url: "https://docs.csv.dev/rpc-providers".to_string(),
                    what: "Verify API endpoint is compatible".to_string(),
                })
            }
            ChainApiError::InvalidAddress(_) => {
                Some(FixAction::CheckState {
                    url: "https://docs.csv.dev/addresses".to_string(),
                    what: "Verify address format for target chain".to_string(),
                })
            }
        }
    }
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
            .map_err(ChainApiError::HttpError)?;

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
        configs.insert(
            Chain::Solana,
            ChainConfig::for_chain(Chain::Solana, NetworkType::Testnet),
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
        configs.insert(
            Chain::Solana,
            ChainConfig::for_chain(Chain::Solana, NetworkType::Testnet),
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
            Chain::Solana => self.get_solana_balance(address).await,
            _ => Err(ChainApiError::ApiError("Unsupported chain".to_string())),
        }
    }

    /// Query Bitcoin balance via mempool.space API.
    async fn get_bitcoin_balance(&self, address: &str) -> Result<f64, ChainApiError> {
        let config = self
            .configs
            .get(&Chain::Bitcoin)
            .ok_or_else(|| ChainApiError::ApiError("Bitcoin config not found".to_string()))?;

        let url = format!("{}/address/{}", config.api_url, address);

        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(&format!("Fetching Bitcoin balance from: {}", url).into());

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            #[cfg(target_arch = "wasm32")]
            web_sys::console::error_1(&format!("Bitcoin API error {} for address {}", status, address).into());
            return Err(ChainApiError::ApiError(format!(
                "Bitcoin API error: {} (Address format may not be supported by mempool.space)",
                status
            )));
        }

        // mempool.space returns: { chain_stats: { funded_txo_sum, spent_txo_sum }, ... }
        let json: serde_json::Value = response.json().await?;

        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(&format!("Bitcoin API response: {:?}", json).into());

        // Try multiple ways to get the balance
        // Method 1: chain_stats.funded_txo_sum - chain_stats.spent_txo_sum
        if let Some(stats) = json.get("chain_stats") {
            let funded = stats
                .get("funded_txo_sum")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let spent = stats
                .get("spent_txo_sum")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);

            let balance_sats = funded - spent;
            let balance_btc = balance_sats / 100_000_000.0;

            #[cfg(target_arch = "wasm32")]
            web_sys::console::log_1(&format!("Bitcoin balance: {} satoshis = {} BTC for {}", balance_sats, balance_btc, address).into());

            return Ok(balance_btc);
        }

        // Method 2: Try balance field directly
        if let Some(balance) = json.get("balance").and_then(|v| v.as_f64()) {
            return Ok(balance / 100_000_000.0);
        }

        Err(ChainApiError::ApiError(
            "Could not parse balance from response. Address may not exist or API format changed.".to_string()
        ))
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

        let response = self.client.post(&config.api_url).json(&body).send().await?;

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

        // Sui uses coin type 0x2::sui::SUI for native SUI
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "suix_getBalance",
            "params": [address, "0x2::sui::SUI"]
        });

        let response = self.client.post(&config.api_url).json(&body).send().await?;

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
            .ok_or_else(|| {
                ChainApiError::ApiError("Missing totalBalance in response".to_string())
            })?;

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

        // Use the balance endpoint (same as csv-cli)
        let url = format!(
            "{}/accounts/{}/balance/0x1::aptos_coin::AptosCoin",
            config.api_url.trim_end_matches('/'),
            address
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

        let body = response.text().await?;

        // API may return a plain number string or JSON
        let balance_octas: u64 = if body.starts_with('{') {
            // Try parsing as JSON
            if let Ok(info) = serde_json::from_str::<serde_json::Value>(&body) {
                info.get("amount")
                    .and_then(|b| b.as_str())
                    .or_else(|| info.as_str())
                    .unwrap_or("0")
                    .parse()
                    .unwrap_or(0)
            } else {
                0
            }
        } else {
            // Plain number string
            body.trim().parse().unwrap_or(0)
        };

        // 1 APT = 10^8 octas
        Ok(balance_octas as f64 / 1e8)
    }

    /// Query Solana balance via JSON-RPC.
    async fn get_solana_balance(&self, address: &str) -> Result<f64, ChainApiError> {
        let config = self
            .configs
            .get(&Chain::Solana)
            .ok_or_else(|| ChainApiError::ApiError("Solana config not found".to_string()))?;

        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getBalance",
            "params": [address]
        });

        let response = self.client.post(&config.api_url).json(&body).send().await?;

        if !response.status().is_success() {
            return Err(ChainApiError::ApiError(format!(
                "Solana API error: {}",
                response.status()
            )));
        }

        let json: serde_json::Value = response.json().await?;
        let balance_lamports = json
            .get("result")
            .and_then(|v| v.get("value"))
            .and_then(|v| v.as_u64())
            .ok_or_else(|| ChainApiError::ApiError("Missing balance in response".to_string()))?;

        // 1 SOL = 10^9 lamports
        Ok(balance_lamports as f64 / 1e9)
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
        assert!(eth_testnet.api_url.contains("sepolia") || eth_testnet.api_url.contains("publicnode"));
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
        assert!(api.get_config(Chain::Solana).is_some());
    }
}
