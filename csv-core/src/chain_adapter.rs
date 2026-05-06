//! Chain adapter trait for dynamic chain support.

use async_trait::async_trait;
use std::collections::HashMap;
use thiserror::Error;

use crate::Chain;

// Re-export from chain_config for convenience
pub use crate::chain_config::{AccountModel, ChainCapabilities, ChainConfig};

/// Chain-specific error types
#[derive(Debug, Error)]
pub enum ChainError {
    /// Chain is not supported
    #[error("Unsupported chain: {0}")]
    UnsupportedChain(String),
    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
    /// RPC connection failed
    #[error("RPC error: {0}")]
    RpcError(String),
    /// Transaction failed
    #[error("Transaction error: {0}")]
    TransactionError(String),
    /// Wallet operation failed
    #[error("Wallet error: {0}")]
    WalletError(String),
    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),
    /// Network error
    #[error("Network error: {0}")]
    NetworkError(String),
    /// Invalid input
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    /// Feature or capability not available/enabled
    #[error("Feature not available: {0}")]
    FeatureNotEnabled(String),

    /// Capability not available
    #[error("Capability unavailable: {0}")]
    CapabilityUnavailable(String),
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
            return Err(ChainError::InvalidConfig(format!(
                "Chain ID mismatch: expected {}, got {}",
                self.chain_id(),
                config.chain_id
            )));
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

/// Helper trait for object-safe chain adapter operations
pub trait ChainAdapterExt: ChainAdapter {
    /// Box the adapter for storage in registry
    fn boxed(self) -> Box<dyn ChainAdapter>
    where
        Self: Sized + 'static,
    {
        Box::new(self)
    }
}

impl<T: ChainAdapter + 'static> ChainAdapterExt for T {}

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
///
/// Security note: This trait intentionally does not expose raw private key material.
/// All signing operations happen internally. Use `key_id()` for key reference only.
#[async_trait]
pub trait Wallet: Send + Sync {
    /// Get wallet address
    fn address(&self) -> &str;

    /// Get key identifier (not the actual private key)
    ///
    /// This returns a reference/key ID that can be used with the keystore
    /// to retrieve the actual key for signing operations. Never returns
    /// raw private key material.
    fn key_id(&self) -> &str;

    /// Sign transaction data using the wallet's internal key
    ///
    /// The signing happens internally - private key is never exposed.
    /// This is the secure way to sign transactions.
    async fn sign_transaction(&self, data: &[u8]) -> ChainResult<Vec<u8>>;

    /// Verify signature
    fn verify_signature(&self, data: &[u8], signature: &[u8]) -> bool;

    /// Generate new address
    fn generate_address(&self) -> ChainResult<String>;

    /// Import from private key
    ///
    /// # Security
    /// The private key is consumed and stored securely. It is not retained
    /// in memory after import.
    fn import_from_private_key(&self, private_key: &str) -> ChainResult<()>;
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
    pub fn get_adapter(&self, chain_id: &str) -> Option<&dyn ChainAdapter> {
        self.adapters.get(chain_id).map(|b| b.as_ref())
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
        self.capabilities
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
        let registry = ChainRegistry::new();
        assert_eq!(registry.supported_chains().len(), 0);

        // Test registration would go here
        // registry.register_adapter(Box::new(MockAdapter::new()));

        assert_eq!(registry.supported_chains().len(), 0);
    }
}
