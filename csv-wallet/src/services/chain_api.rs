//! Chain API service for querying balances from different blockchains.
//!
//! This module provides a unified interface to query on-chain balances
//! across Bitcoin, Ethereum, Sui, Aptos, and Solana.
//!
//! # Architecture
//! - Uses csv-adapter facade for all chain operations
//! - No direct HTTP calls or RPC implementations
//! - Delegates to ChainQuery trait implementations in chain adapters
//!
//! This module is Production Guarantee Plan compliant - all operations
//! go through the csv-adapter facade rather than duplicate implementations.

use csv_adapter::prelude::CsvClient;
use csv_adapter::StoreBackend;
use csv_adapter_core::agent_types::{error_codes, FixAction, HasErrorSuggestion};
use csv_adapter_core::Chain;
use serde::{Deserialize, Serialize};

use crate::services::network::NetworkType;

/// Configuration for a chain API endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainConfig {
    /// API base URL for the chain (RPC endpoint).
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

    /// Adapter error.
    #[error("Chain adapter error: {0}")]
    AdapterError(String),
}

impl HasErrorSuggestion for ChainApiError {
    fn error_code(&self) -> &'static str {
        match self {
            ChainApiError::HttpError(_) => error_codes::WALLET_CHAIN_API_HTTP,
            ChainApiError::JsonError(_) => error_codes::WALLET_CHAIN_API_JSON,
            ChainApiError::InvalidAddress(_) => error_codes::WALLET_CHAIN_API_INVALID_ADDRESS,
            ChainApiError::ApiError(_) => error_codes::WALLET_CHAIN_API_ERROR,
            ChainApiError::AdapterError(_) => error_codes::WALLET_CHAIN_API_ERROR,
        }
    }

    fn description(&self) -> String {
        self.to_string()
    }

    fn suggested_fix(&self) -> String {
        match self {
            ChainApiError::HttpError(_) => "HTTP request to chain API failed. Check: \
                 1) Your internet connection, 2) The API endpoint is accessible, \
                 3) You're not being blocked by CORS (browser) or firewalls. \
                 Try a different RPC provider if the issue persists."
                .to_string(),
            ChainApiError::JsonError(_) => {
                "Failed to parse API response. The API format may have changed \
                 or the response is malformed. Try: 1) A different API endpoint, \
                 2) Updating to the latest SDK version."
                    .to_string()
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
            ChainApiError::AdapterError(msg) => {
                format!(
                    "Chain adapter error: {}. \
                     The adapter may not be properly configured for this chain.",
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
            ChainApiError::HttpError(_) | ChainApiError::ApiError(_) => Some(FixAction::Retry {
                parameter_changes: std::collections::HashMap::from([(
                    "rpc_endpoint".to_string(),
                    "try_alternative".to_string(),
                )]),
            }),
            ChainApiError::JsonError(_) => Some(FixAction::CheckState {
                url: "https://docs.csv.dev/rpc-providers".to_string(),
                what: "Verify API endpoint is compatible".to_string(),
            }),
            ChainApiError::InvalidAddress(_) => Some(FixAction::CheckState {
                url: "https://docs.csv.dev/addresses".to_string(),
                what: "Verify address format for target chain".to_string(),
            }),
            ChainApiError::AdapterError(_) => Some(FixAction::CheckState {
                url: "https://docs.csv.dev/adapters".to_string(),
                what: "Verify chain adapter configuration".to_string(),
            }),
        }
    }
}

/// Unified Chain API that uses the csv-adapter facade.
///
/// This implementation delegates all operations to the csv-adapter facade,
/// which routes to the appropriate chain adapter. No direct HTTP calls.
pub struct ChainApi {
    /// CSV adapter client for facade-based operations.
    csv_client: Option<CsvClient>,
    /// Chain configurations.
    configs: std::collections::HashMap<Chain, ChainConfig>,
}

impl ChainApi {
    /// Create a new ChainApi with default configurations.
    pub fn new() -> Result<Self, ChainApiError> {
        Ok(Self {
            csv_client: None,
            configs: default_configs(),
        })
    }

    /// Get balance for an address on a specific chain using the csv-adapter facade.
    ///
    /// This delegates to ChainQuery::get_balance via the facade instead of
    /// making direct HTTP calls, ensuring production guarantee compliance.
    ///
    /// Returns raw chain units (satoshis, lamports, MIST, octas, wei).
    /// Use `format_balance_display()` from hooks::use_balance for human-readable formatting.
    pub async fn get_balance(&self, chain: Chain, address: &str) -> Result<u64, ChainApiError> {
        // Build CSV client with the requested chain enabled
        let client = self.get_or_build_client(chain).await?;

        // Query balance through the facade (delegates to ChainQuery trait)
        // Note: get_balance expects the address as a string
        let balance_info = client
            .chain_facade()
            .get_balance(chain, address)
            .await
            .map_err(|e| ChainApiError::AdapterError(format!("Facade error: {}", e)))?;

        // Return raw chain units - display conversion happens at UI layer
        Ok(balance_info.total)
    }

    /// Update the configuration for a chain.
    pub fn set_config(&mut self, chain: Chain, config: ChainConfig) {
        self.configs.insert(chain, config);
    }

    /// Get the configuration for a chain.
    pub fn get_config(&self, chain: Chain) -> Option<&ChainConfig> {
        self.configs.get(&chain)
    }

    /// Get or build a CsvClient for the specified chain.
    async fn get_or_build_client(&self, chain: Chain) -> Result<CsvClient, ChainApiError> {
        // Build a new client with the requested chain enabled
        let _config = self
            .configs
            .get(&chain)
            .cloned()
            .unwrap_or_else(|| ChainConfig::for_chain(chain, NetworkType::Testnet));

        CsvClient::builder()
            .with_chain(chain)
            .with_store_backend(StoreBackend::InMemory)
            .build()
            .map_err(|e| {
                ChainApiError::AdapterError(format!("Failed to build CSV client: {}", e))
            })
    }

    /// Parse address string to bytes based on chain type.
    fn parse_address_to_bytes(&self, chain: Chain, address: &str) -> Result<Vec<u8>, ChainApiError> {
        let hex_str = address.trim_start_matches("0x");

        match chain {
            Chain::Bitcoin => {
                // Bitcoin addresses are base58 or bech32, not hex
                // For now, return empty - Bitcoin adapter handles address formats
                Ok(hex::decode(hex_str).unwrap_or_default())
            }
            Chain::Ethereum => {
                hex::decode(hex_str).map_err(|e| {
                    ChainApiError::InvalidAddress(format!("Invalid Ethereum address: {}", e))
                })
            }
            Chain::Sui | Chain::Aptos => {
                hex::decode(hex_str).map_err(|e| {
                    ChainApiError::InvalidAddress(format!("Invalid {} address: {}", chain, e))
                })
            }
            Chain::Solana => {
                // Solana addresses are base58
                bs58::decode(address)
                    .into_vec()
                    .map_err(|e| ChainApiError::InvalidAddress(format!("Invalid Solana address: {}", e)))
            }
            _ => Err(ChainApiError::InvalidAddress(format!("Unsupported chain: {}", chain))),
        }
    }

}

impl Default for ChainApi {
    fn default() -> Self {
        Self {
            csv_client: None,
            configs: default_configs(),
        }
    }
}

/// Default chain configurations.
fn default_configs() -> std::collections::HashMap<Chain, ChainConfig> {
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
    configs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chain_config_for_chain() {
        let btc_testnet = ChainConfig::for_chain(Chain::Bitcoin, NetworkType::Testnet);
        assert!(btc_testnet.is_testnet);
        assert!(btc_testnet.api_url.contains("testnet") || btc_testnet.api_url.contains("signet"));

        let btc_mainnet = ChainConfig::for_chain(Chain::Bitcoin, NetworkType::Mainnet);
        assert!(!btc_mainnet.is_testnet);
        assert!(!btc_mainnet.api_url.contains("testnet"));

        let eth_testnet = ChainConfig::for_chain(Chain::Ethereum, NetworkType::Testnet);
        assert!(
            eth_testnet.api_url.contains("sepolia") || eth_testnet.api_url.contains("publicnode")
        );
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
