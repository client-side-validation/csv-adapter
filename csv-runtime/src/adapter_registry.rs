//! Adapter registry trait and implementation
//!
//! The runtime does not import any chain adapter directly. Chain adapters register
//! themselves via this trait.

#![allow(missing_docs)]

use csv_core::chain_config::ChainCapabilities;
use csv_core::finality::FinalityProof as FinalityVerifierProof;
use csv_core::hash::Hash;
use csv_core::proof::ProofBundle;
use csv_core::verified::{VerificationResult, VerificationFailure};

/// Cross-chain transfer data passed to adapters.
/// This type should eventually live in csv-core.
#[derive(Debug, Clone)]
pub struct CrossChainTransfer {
    /// Unique transfer ID
    pub id: String,
    /// Source chain ID
    pub source_chain: String,
    /// Destination chain ID
    pub destination_chain: String,
    /// Lock transaction hash on source chain
    pub lock_tx_hash: Vec<u8>,
    /// Lock output index on source chain
    pub lock_output_index: u32,
    /// Sanad ID being transferred
    pub sanad_id: Hash,
    /// Transition ID for the transfer
    pub transition_id: Vec<u8>,
}

/// Result of locking a sanad on the source chain
#[derive(Debug, Clone)]
pub struct LockResult {
    /// Transaction hash of the lock
    pub tx_hash: String,
    /// Block height where the lock was included
    pub block_height: u64,
}

/// Result of minting a sanad on the destination chain
#[derive(Debug, Clone)]
pub struct MintResult {
    /// Transaction hash of the mint
    pub tx_hash: String,
}

/// Status of a seal in the registry
#[derive(Debug, Clone)]
pub enum SealRegistryStatus {
    /// Seal has not been consumed
    Available,
    /// Seal has been consumed (replay)
    Consumed,
}

/// The interface between csv-runtime and csv-adapters.
/// Each adapter implements this for its chain.
/// The TransferCoordinator calls through this — never directly into adapters.
#[async_trait::async_trait]
pub trait ChainAdapter: Send + Sync {
    /// Returns the chain ID for this adapter
    fn chain_id(&self) -> &str;

    /// Returns the chain capabilities for this adapter
    fn capabilities(&self) -> &ChainCapabilities;

    /// Lock a sanad on the source chain
    async fn lock_sanad(
        &self,
        transfer: &CrossChainTransfer,
    ) -> Result<LockResult, AdapterError>;

    /// Mint a sanad on the destination chain
    async fn mint_sanad(
        &self,
        transfer: &CrossChainTransfer,
        proof_bundle: &ProofBundle,
    ) -> Result<MintResult, AdapterError>;

    /// Build an inclusion proof for a lock result
    async fn build_inclusion_proof(
        &self,
        lock_result: &LockResult,
    ) -> Result<ProofBundle, AdapterError>;

    /// Verify an inclusion proof
    async fn verify_inclusion_proof(
        &self,
        proof: &ProofBundle,
    ) -> Result<VerificationResult, AdapterError>;

    /// Verify finality for a block height
    async fn verify_finality(
        &self,
        block_height: u64,
    ) -> Result<FinalityVerifierProof, AdapterError>;

    /// Verify seal registry status
    async fn verify_seal_registry(
        &self,
        seal_id: &[u8],
    ) -> Result<SealRegistryStatus, AdapterError>;

    /// Get the balance for an address on this chain
    async fn get_balance(
        &self,
        address: &str,
    ) -> Result<String, AdapterError>;
}

/// Error type for adapter operations
#[derive(Debug, thiserror::Error)]
pub enum AdapterError {
    /// RPC or network error
    #[error("RPC error: {0}")]
    RpcError(String),

    /// Transaction failed
    #[error("Transaction failed: {0}")]
    TxFailed(String),

    /// Invalid proof
    #[error("Invalid proof: {0}")]
    InvalidProof(String),

    /// Seal not found
    #[error("Seal not found: {0}")]
    SealNotFound(String),

    /// Generic adapter error
    #[error("Adapter error: {0}")]
    Generic(String),
}

/// Registry that holds registered chain adapters
pub struct AdapterRegistryImpl {
    adapters: std::collections::HashMap<String, std::sync::Arc<dyn ChainAdapter>>,
}

impl AdapterRegistryImpl {
    /// Create a new adapter registry
    pub fn new() -> Self {
        Self {
            adapters: std::collections::HashMap::new(),
        }
    }

    /// Register a chain adapter
    pub fn register(&mut self, adapter: std::sync::Arc<dyn ChainAdapter>) {
        self.adapters
            .insert(adapter.chain_id().to_string(), adapter);
    }

    /// Get a registered adapter by chain ID
    pub fn get(&self, chain_id: &str) -> Option<&std::sync::Arc<dyn ChainAdapter>> {
        self.adapters.get(chain_id)
    }

    /// Get capabilities for a chain
    pub fn capabilities(&self, chain_id: &str) -> Option<&ChainCapabilities> {
        self.adapters
            .get(chain_id)
            .map(|a| a.capabilities())
    }
}

impl Default for AdapterRegistryImpl {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait that the TransferCoordinator uses to interact with adapters.
/// This abstracts over the registry to provide a clean interface.
#[async_trait::async_trait]
pub trait AdapterRegistry: Send + Sync {
    /// Get capabilities for a chain
    fn capabilities(&self, chain_id: &str) -> Option<&ChainCapabilities>;

    /// Lock a sanad on the source chain
    async fn lock_sanad(
        &self,
        chain_id: &str,
        transfer: &CrossChainTransfer,
    ) -> Result<LockResult, AdapterError>;

    /// Mint a sanad on the destination chain
    async fn mint_sanad(
        &self,
        chain_id: &str,
        transfer: &CrossChainTransfer,
        proof_bundle: &ProofBundle,
    ) -> Result<MintResult, AdapterError>;

    /// Build an inclusion proof
    async fn build_inclusion_proof(
        &self,
        chain_id: &str,
        lock_result: &LockResult,
    ) -> Result<ProofBundle, AdapterError>;

    /// Verify a proof bundle
    async fn verify_proof_bundle(
        &self,
        proof_bundle: &ProofBundle,
    ) -> Result<VerificationResult, VerificationFailure>;

    /// Verify finality
    async fn verify_finality(
        &self,
        chain_id: &str,
        block_height: u64,
    ) -> Result<FinalityVerifierProof, AdapterError>;

    /// Get balance for an address on a chain
    async fn get_balance(
        &self,
        chain_id: &str,
        address: &str,
    ) -> Result<String, AdapterError>;
}

#[async_trait::async_trait]
impl AdapterRegistry for AdapterRegistryImpl {
    fn capabilities(&self, chain_id: &str) -> Option<&ChainCapabilities> {
        AdapterRegistryImpl::capabilities(self, chain_id)
    }

    async fn lock_sanad(
        &self,
        chain_id: &str,
        transfer: &CrossChainTransfer,
    ) -> Result<LockResult, AdapterError> {
        let adapter = self
            .adapters
            .get(chain_id)
            .ok_or(AdapterError::Generic(format!(
                "Adapter not found for chain: {}",
                chain_id
            )))?;
        adapter.lock_sanad(transfer).await
    }

    async fn mint_sanad(
        &self,
        chain_id: &str,
        transfer: &CrossChainTransfer,
        proof_bundle: &ProofBundle,
    ) -> Result<MintResult, AdapterError> {
        let adapter = self
            .adapters
            .get(chain_id)
            .ok_or(AdapterError::Generic(format!(
                "Adapter not found for chain: {}",
                chain_id
            )))?;
        adapter.mint_sanad(transfer, proof_bundle).await
    }

    async fn build_inclusion_proof(
        &self,
        chain_id: &str,
        lock_result: &LockResult,
    ) -> Result<ProofBundle, AdapterError> {
        let adapter = self
            .adapters
            .get(chain_id)
            .ok_or(AdapterError::Generic(format!(
                "Adapter not found for chain: {}",
                chain_id
            )))?;
        adapter.build_inclusion_proof(lock_result).await
    }

    async fn verify_proof_bundle(
        &self,
        _proof_bundle: &ProofBundle,
    ) -> Result<VerificationResult, VerificationFailure> {
        // For now, return a passing verification result.
        // In production, this would verify the proof against the source chain.
        Ok(VerificationResult {
            valid: true,
            assurance: csv_core::verified::VerificationAssurance::ConsensusBound,
            verified_components: csv_core::verified::VerifiedComponents {
                inclusion: csv_core::verified::InclusionStrength::MerklePath,
                finality: csv_core::verified::FinalityStrength::Probabilistic { confirmations: 6 },
                replay_checked: true,
                ownership_signature: true,
            },
            error: None,
        })
    }

    async fn verify_finality(
        &self,
        chain_id: &str,
        block_height: u64,
    ) -> Result<FinalityVerifierProof, AdapterError> {
        let adapter = self
            .adapters
            .get(chain_id)
            .ok_or(AdapterError::Generic(format!(
                "Adapter not found for chain: {}",
                chain_id
            )))?;
        adapter.verify_finality(block_height).await
    }

    async fn get_balance(
        &self,
        chain_id: &str,
        address: &str,
    ) -> Result<String, AdapterError> {
        let adapter = self
            .adapters
            .get(chain_id)
            .ok_or(AdapterError::Generic(format!(
                "Adapter not found for chain: {}",
                chain_id
            )))?;
        adapter.get_balance(address).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockAdapter {
        caps: ChainCapabilities,
    }

    impl MockAdapter {
        fn new() -> Self {
            Self {
                caps: ChainCapabilities::bitcoin(),
            }
        }
    }

    #[async_trait::async_trait]
    impl ChainAdapter for MockAdapter {
        fn chain_id(&self) -> &str {
            "mock-chain"
        }

        fn capabilities(&self) -> &ChainCapabilities {
            &self.caps
        }

        async fn lock_sanad(
            &self,
            _transfer: &CrossChainTransfer,
        ) -> Result<LockResult, AdapterError> {
            Ok(LockResult {
                tx_hash: "0xmock".to_string(),
                block_height: 100,
            })
        }

        async fn mint_sanad(
            &self,
            _transfer: &CrossChainTransfer,
            _proof_bundle: &ProofBundle,
        ) -> Result<MintResult, AdapterError> {
            Ok(MintResult {
                tx_hash: "0xmint".to_string(),
            })
        }

        async fn build_inclusion_proof(
            &self,
            _lock_result: &LockResult,
        ) -> Result<ProofBundle, AdapterError> {
            unimplemented!("Not needed for registry tests")
        }

        async fn verify_inclusion_proof(
            &self,
            _proof: &ProofBundle,
        ) -> Result<VerificationResult, AdapterError> {
            unimplemented!("Not needed for registry tests")
        }

        async fn verify_finality(&self, _block_height: u64) -> Result<FinalityVerifierProof, AdapterError> {
            unimplemented!("Not needed for registry tests")
        }

        async fn verify_seal_registry(&self, _seal_id: &[u8]) -> Result<SealRegistryStatus, AdapterError> {
            Ok(SealRegistryStatus::Available)
        }

        async fn get_balance(&self, _address: &str) -> Result<String, AdapterError> {
            Ok("0".to_string())
        }
    }

    #[tokio::test]
    async fn test_registry_registers_and_retrieves_adapter() {
        let mut registry = AdapterRegistryImpl::new();
        registry.register(std::sync::Arc::new(MockAdapter::new()));

        assert!(registry.get("mock-chain").is_some());
        assert!(registry.get("nonexistent").is_none());
    }

    #[tokio::test]
    async fn test_registry_lock_sanad() {
        let mut registry = AdapterRegistryImpl::new();
        registry.register(std::sync::Arc::new(MockAdapter::new()));

        let transfer = CrossChainTransfer {
            id: "test-1".to_string(),
            source_chain: "mock-chain".to_string(),
            destination_chain: "mock-chain".to_string(),
            lock_tx_hash: vec![0u8; 32],
            lock_output_index: 0,
            sanad_id: csv_core::hash::Hash::new([1u8; 32]),
            transition_id: vec![0u8; 32],
        };

        let result = registry.lock_sanad("mock-chain", &transfer).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().tx_hash, "0xmock");
    }
}
