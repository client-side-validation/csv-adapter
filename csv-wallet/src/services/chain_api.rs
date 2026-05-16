//! Chain API service for wallet operations.
//!
//! Provides chain RPC operations using csv-sdk runtime.

use csv_sdk::runtime::{ChainRuntime, RuntimeConfig, RuntimeManager};
use csv_store::state::ChainId;

/// Errors that can occur during chain API operations.
#[derive(Debug, Clone)]
pub enum ChainApiError {
    /// Balance query failed - DO NOT silently return zero.
    BalanceUnavailable {
        chain: String,
        address: String,
        source: String,
    },
}

impl std::fmt::Display for ChainApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChainApiError::BalanceUnavailable {
                chain,
                address,
                source,
            } => {
                write!(
                    f,
                    "Balance unavailable for {} on {}: {}",
                    address, chain, source
                )
            }
        }
    }
}

impl std::error::Error for ChainApiError {}

/// Chain API using csv-sdk runtime.
pub struct ChainApi {
    runtime: ChainRuntime,
}

impl std::fmt::Debug for ChainApi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChainApi")
            .field("runtime", &"<ChainRuntime>")
            .finish()
    }
}

impl Clone for ChainApi {
    fn clone(&self) -> Self {
        // Create new runtime via RuntimeManager
        let runtime_config = RuntimeConfig::default();
        let runtime_manager = RuntimeManager::new(runtime_config);
        let runtime = runtime_manager.chain_runtime().clone();

        Self { runtime }
    }
}

impl Default for ChainApi {
    fn default() -> Self {
        Self::new(ChainConfig::for_chain(&ChainId::new("ethereum")))
    }
}

/// Chain configuration with real RPC endpoints.
#[derive(Debug, Clone)]
pub struct ChainConfig {
    pub rpc_url: String,
    pub api_url: String,
}

impl ChainConfig {
    /// Get chain config with appropriate RPC endpoints.
    pub fn for_chain(chain: &ChainId) -> Self {
        let rpc_url = match chain.as_str() {
            "bitcoin" => std::env::var("BTC_RPC_URL")
                .unwrap_or_else(|_| "https://mempool.space/signet/api".to_string()),
            "ethereum" => std::env::var("ETH_RPC_URL")
                .unwrap_or_else(|_| "https://ethereum-sepolia-rpc.publicnode.com".to_string()),
            "sui" => std::env::var("SUI_RPC_URL")
                .unwrap_or_else(|_| "https://fullnode.testnet.sui.io:443".to_string()),
            "aptos" => std::env::var("APTOS_RPC_URL")
                .unwrap_or_else(|_| "https://fullnode.testnet.aptoslabs.com/v1".to_string()),
            "solana" => std::env::var("SOL_RPC_URL")
                .unwrap_or_else(|_| "https://api.devnet.solana.com".to_string()),
            _ => "https://unknown-rpc.example.com".to_string(),
        };
        Self {
            rpc_url: rpc_url.clone(),
            api_url: rpc_url,
        }
    }
}

impl ChainApi {
    /// Create new chain API.
    pub fn new(_config: ChainConfig) -> Self {
        // Create runtime manager with default config
        let runtime_config = RuntimeConfig::default();
        let runtime_manager = RuntimeManager::new(runtime_config);
        let runtime = runtime_manager.chain_runtime().clone();

        Self { runtime }
    }

    /// Get balance for an address.
    pub async fn get_balance(
        &self,
        address: &str,
        chain: ChainId,
    ) -> Result<String, ChainApiError> {
        let chain_str = chain.as_str().to_string();
        // Use csv-sdk runtime to query balance
        match self.runtime.get_balance(chain, address).await {
            Ok(balance_info) => Ok(balance_info.total.to_string()),
            Err(e) => Err(ChainApiError::BalanceUnavailable {
                chain: chain_str,
                address: address.to_string(),
                source: e.to_string(),
            }),
        }
    }
}
