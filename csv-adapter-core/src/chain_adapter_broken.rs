//! Chain adapter trait for dynamic chain support.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

use crate::Chain;

/// Chain-specific capabilities and features
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainCapabilities {
    /// Whether this chain supports NFTs
    pub supports_nfts: bool,
    /// Whether this chain supports smart contracts
    pub supports_smart_contracts: bool,
    /// Account model used by this chain
    pub account_model: AccountModel,
    /// Number of blocks needed for finality
    pub confirmation_blocks: u64,
    /// Maximum batch size for operations
    pub max_batch_size: usize,
    /// Supported networks for this chain
    pub supported_networks: Vec<String>,
    /// Whether chain supports cross-chain transfers
    pub supports_cross_chain: bool,
    /// Chain-specific features
    pub custom_features: HashMap<String, serde_json::Value>,
}

/// Account model types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccountModel {
    /// UTXO-based model (Bitcoin-like)
    UTXO,
    /// Account-based model (Ethereum-like)
    Account,
    /// Object-based model (Sui-like)
    Object,
    /// Hybrid model (mixed approaches)
    Hybrid,
}

/// Chain-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainConfig {
    /// Unique identifier for this chain
    pub chain_id: String,
    /// Human-readable name for this chain
    pub chain_name: String,
    /// Default network to use
    pub default_network: String,
    /// List of RPC endpoints
    pub rpc_endpoints: Vec<String>,
    /// CSV program ID for this chain
    pub program_id: Option<String>,
    /// Block explorer URLs
    pub block_explorer_urls: Vec<String>,
    /// Chain capabilities
    pub capabilities: ChainCapabilities,
    /// Chain-specific settings
    pub custom_settings: HashMap<String, serde_json::Value>,
}

/// Chain-specific error types
#[derive(Debug, Error)]
pub enum ChainError {
    /// Chain is not supported
    UnsupportedChain(String),
    /// Invalid configuration
    InvalidConfig(String),
    /// RPC connection failed
    RpcError(String),
    /// Transaction failed
    TransactionError(String),
    /// Wallet operation failed
    WalletError(String),
    /// Serialization/deserialization error
    SerializationError(String),
    /// Network error
    NetworkError(String),
    /// Invalid input
    InvalidInput(String),
}

/// Result type for chain operations
pub type ChainResult<T> = Result<T, ChainError>;

/// Standard interface for all chain adapters
#[async_trait]
pub trait ChainAdapter: Send + Sync {
    /// Get unique identifier for this chain
    fn chain_id(&self) -> &'static str;
    
    /// Get human-readable name for this chain
    fn chain_name(&self) -> &'static str;
    
    /// Get chain capabilities
    fn capabilities(&self) -> ChainCapabilities;
    
    /// Validate chain configuration
    fn validate_config(&self, config: &ChainConfig) -> ChainResult<()> {
        if config.chain_id != self.chain_id() {
            return Err(ChainError::InvalidConfig(
                format!("Chain ID mismatch: expected {}, got {}", self.chain_id(), config.chain_id)
            ));
        }
        Ok(())
    }
    
    /// Create RPC client for this chain
    async fn create_client(&self, config: &ChainConfig) -> ChainResult<Box<dyn RpcClient>>;
    
    /// Create wallet for this chain
    async fn create_wallet(&self, config: &ChainConfig) -> ChainResult<Box<dyn Wallet>>;
    
    /// Get chain-specific CSV program ID
    fn csv_program_id(&self) -> Option<&'static str>;
    
    /// Convert chain to core Chain enum
    fn to_core_chain(&self) -> Chain;
    
    /// Get default network for this chain
    fn default_network(&self) -> &'static str;
}

/// Standard interface for chain RPC clients
#[async_trait]
pub trait RpcClient: Send + Sync {
    /// Send transaction to blockchain
    async fn send_transaction(&self, tx: &[u8]) -> ChainResult<String>;
    
    /// Get transaction by hash/signature
    async fn get_transaction(&self, hash: &str) -> ChainResult<serde_json::Value>;
    
    /// Get latest block height
    async fn get_latest_block(&self) -> ChainResult<u64>;
    
    /// Get account balance
    async fn get_balance(&self, address: &str) -> ChainResult<u64>;
    
    /// Check transaction confirmation
    async fn is_transaction_confirmed(&self, hash: &str) -> ChainResult<bool>;
    
    /// Get chain-specific metadata
    async fn get_chain_info(&self) -> ChainResult<serde_json::Value>;
}

/// Standard interface for chain wallets
#[async_trait]
pub trait Wallet: Send + Sync {
    /// Get wallet address
    fn address(&self) -> &str;
    
    /// Get private key (encrypted)
    fn private_key(&self) -> &str;
    
    /// Sign transaction data
    async fn sign_transaction(&self, data: &[u8]) -> ChainResult<Vec<u8>>;
    
    /// Verify signature
    fn verify_signature(&self, data: &[u8], signature: &[u8]) -> bool;
    
    /// Generate new address
    fn generate_address(&self) -> ChainResult<String>;
    
    /// Import from private key
    fn from_private_key(&self, private_key: &str) -> ChainResult<()>;
}

/// Factory for creating chain adapters
pub struct ChainAdapterFactory;

impl ChainAdapterFactory {
    /// Create adapter from configuration
    pub fn create_adapter(config: &ChainConfig) -> ChainResult<Box<dyn ChainAdapter>> {
        match config.chain_id.as_str() {
            "bitcoin" => {
                #[cfg(feature = "bitcoin")]
                return Ok(Box::new(crate::adapters::BitcoinAdapter::new(config)));
                #[cfg(not(feature = "bitcoin"))]
                return Err(ChainError::UnsupportedChain("Bitcoin adapter not compiled".to_string()));
            }
            "ethereum" => {
                #[cfg(feature = "ethereum")]
                return Ok(Box::new(crate::adapters::EthereumAdapter::new(config)));
                #[cfg(not(feature = "ethereum"))]
                return Err(ChainError::UnsupportedChain("Ethereum adapter not compiled".to_string()));
            }
            "solana" => {
                #[cfg(feature = "solana")]
                return Ok(Box::new(crate::adapters::SolanaAdapter::new(config)));
                #[cfg(not(feature = "solana"))]
                return Err(ChainError::UnsupportedChain("Solana adapter not compiled".to_string()));
            }
            "sui" => {
                #[cfg(feature = "sui")]
                return Ok(Box::new(crate::adapters::SuiAdapter::new(config)));
                #[cfg(not(feature = "sui"))]
                return Err(ChainError::UnsupportedChain("Sui adapter not compiled".to_string()));
            }
            "aptos" => {
                #[cfg(feature = "aptos")]
                return Ok(Box::new(crate::adapters::AptosAdapter::new(config)));
                #[cfg(not(feature = "aptos"))]
                return Err(ChainError::UnsupportedChain("Aptos adapter not compiled".to_string()));
            }
            // Future chains will be loaded dynamically
            _ => Err(ChainError::UnsupportedChain(config.chain_id.clone())),
        }
    }
    
    /// Get supported chain IDs
    pub fn supported_chains() -> Vec<&'static str> {
        let mut chains = vec![];
        
        #[cfg(feature = "bitcoin")]
        chains.push("bitcoin");
        
        #[cfg(feature = "ethereum")]
        chains.push("ethereum");
        
        #[cfg(feature = "solana")]
        chains.push("solana");
        
        #[cfg(feature = "sui")]
        chains.push("sui");
        
        #[cfg(feature = "aptos")]
        chains.push("aptos");
        
        chains
    }
}

/// Registry for managing chain adapters
pub struct ChainRegistry {
    adapters: HashMap<String, Box<dyn ChainAdapter>>,
    capabilities: HashMap<String, ChainCapabilities>,
}

impl ChainRegistry {
    /// Create new empty registry
    pub fn new() -> Self {
        Self {
            adapters: HashMap::new(),
            capabilities: HashMap::new(),
        }
    }
    
    /// Register a new chain adapter
    pub fn register_adapter(&mut self, adapter: Box<dyn ChainAdapter>) {
        let chain_id = adapter.chain_id();
        let capabilities = adapter.capabilities();
        
        self.adapters.insert(chain_id.to_string(), adapter);
        self.capabilities.insert(chain_id.to_string(), capabilities);
    }
    
    /// Get adapter by chain ID
    pub fn get_adapter(&self, chain_id: &str) -> Option<&dyn ChainAdapter>> {
        self.adapters.get(chain_id)
    }
    
    /// Get all supported chain IDs
    pub fn supported_chains(&self) -> Vec<&str> {
        self.adapters.keys().map(|k| k.as_str()).collect()
    }
    
    /// Get capabilities for a chain
    pub fn get_capabilities(&self, chain_id: &str) -> Option<&ChainCapabilities> {
        self.capabilities.get(chain_id)
    }
    
    /// Find chains by capability
    pub fn find_chains_with_capability<F>(&self, capability_check: F) -> Vec<&str>
    where
        F: Fn(&ChainCapabilities) -> bool,
    {
        self.adapters
            .iter()
            .filter(|(_, cap)| capability_check(cap))
            .map(|(id, _)| id.as_str())
            .collect()
    }
    
    /// Find chains that support NFTs
    pub fn nft_supported_chains(&self) -> Vec<&str> {
        self.find_chains_with_capability(|cap| cap.supports_nfts)
    }
    
    /// Find chains that support smart contracts
    pub fn smart_contract_chains(&self) -> Vec<&str> {
        self.find_chains_with_capability(|cap| cap.supports_smart_contracts)
    }
    
    /// Find chains that support cross-chain transfers
    pub fn cross_chain_supported_chains(&self) -> Vec<&str> {
        self.find_chains_with_capability(|cap| cap.supports_cross_chain)
    }
}

impl Default for ChainRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_chain_capabilities() {
        let caps = ChainCapabilities {
            supports_nfts: true,
            supports_smart_contracts: true,
            account_model: AccountModel::Account,
            confirmation_blocks: 12,
            max_batch_size: 100,
            supported_networks: vec!["mainnet".to_string(), "testnet".to_string()],
            supports_cross_chain: true,
            custom_features: HashMap::new(),
        };
        
        assert!(caps.supports_nfts);
        assert!(caps.supports_smart_contracts);
        assert_eq!(caps.confirmation_blocks, 12);
    }
    
    #[test]
    fn test_chain_registry() {
        let mut registry = ChainRegistry::new();
        assert_eq!(registry.supported_chains().len(), 0);
        
        // Test registration would go here
        // registry.register_adapter(Box::new(MockAdapter::new()));
        
        assert_eq!(registry.supported_chains().len(), 0);
    }
}
