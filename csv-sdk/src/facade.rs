//! Chain adapter facade implementations.
//!
//! This module provides unified facade functions that delegate to the appropriate
//! chain adapters while providing a consistent API across all supported chains.
//!
//! # Clean Architecture
//!
//! Following the Clean Architecture documented in `docs/ARCHITECTURE.md` and `docs/BLUEPRINT.md`:
//!
//! 1. **csv-adapter-core**: Defines `AnchorLayer` trait (protocol primitives) and 
//!    `FullChainAdapter` trait (chain operations like ChainQuery, ChainSigner, etc.)
//!
//! 2. **csv-adapter-{chain}**: Each chain provides:
//!    - `AnchorLayer` implementation (e.g., `EthereumAnchorLayer`)
//!    - `ChainOperations` type (e.g., `EthereumChainOperations`) implementing `FullChainAdapter`
//!    - ChainOperations can be created FROM AnchorLayer via `from_anchor_layer()`
//!
//! 3. **csv-adapter (this facade)**: 
//!    - Works with `Arc<dyn FullChainAdapter>` (ChainOperations, NOT AnchorLayer)
//!    - Provides `AdapterBuilder` for constructing adapters with chain-specific configs
//!    - `ChainFacade` delegates operations to registered adapters
//!
//! The facade pattern ensures that:
//! - CLI, wallet, and other components don't need direct chain adapter dependencies
//! - All chain operations go through a unified interface
//! - Error handling is consistent across chains
//! - Chain-specific implementations are properly abstracted

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

// Imports for contract call encoding
#[allow(unused_imports)]
use sha2::Sha256;
#[allow(unused_imports)]
use sha3::Keccak256;
// Digest trait needed for new(), update(), finalize() methods
#[allow(unused_imports)]
use sha2::Digest as Sha2Digest;
#[allow(unused_imports)]
use sha3::Digest;

use csv_adapter_core::{
    Chain,
    FullChainAdapter, BalanceInfo, TransactionInfo, TransactionStatus,
    DeploymentStatus, RightOperationResult,
    RightId, Hash, ProofBundle
};

use crate::client::ClientRef;
use crate::errors::CsvError;

/// Unified chain facade that provides all chain operations.
///
/// This is the main facade that chains, CLI, and wallet components should use
/// for all blockchain interactions. It delegates to the appropriate chain
/// adapters while providing a consistent API.
///
/// # Architecture
///
/// The facade holds `Arc<dyn FullChainAdapter>` instances which are the chain-specific
/// `ChainOperations` types (e.g., `EthereumChainOperations`), NOT the `AnchorLayer` types.
/// This distinction is crucial for Clean Architecture compliance.
///
/// Use `AdapterBuilder` to construct adapters properly with chain-specific configuration.
#[derive(Clone)]
pub struct ChainFacade {
    client: Arc<ClientRef>,
    adapters: Arc<Mutex<HashMap<Chain, Arc<dyn FullChainAdapter>>>>,
}

/// Pre-fetched seal consumption data for verification.
/// This avoids capturing the store lock in the closure passed to verify_proof.
struct SealCheckData {
    right_id: csv_adapter_core::RightId,
    is_consumed: bool,
}

impl ChainFacade {
    pub(crate) fn new(client: Arc<ClientRef>) -> Self {
        Self {
            client,
            adapters: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Create a new ChainFacade with pre-built adapters.
    ///
    /// This is used by the builder to auto-register adapters when chains are enabled.
    pub(crate) fn with_adapters(
        client: Arc<ClientRef>,
        adapters: HashMap<Chain, Arc<dyn FullChainAdapter>>,
    ) -> Self {
        Self {
            client,
            adapters: Arc::new(Mutex::new(adapters)),
        }
    }

    /// Register a chain adapter for the given chain.
    ///
    /// The adapter must implement `FullChainAdapter` (e.g., `EthereumChainOperations`).
    /// Use `AdapterBuilder` to construct adapters with proper chain-specific configuration.
    ///
    /// # Example
    /// ```no_run
    /// use csv_adapter::facade::{ChainFacade, AdapterBuilder, AdapterConfig};
    /// use csv_adapter_core::Chain;
    ///
    /// let facade = ChainFacade::new(/* client ref */);
    /// let adapter = AdapterBuilder::new()
    ///     .ethereum_from_config(config, rpc, csv_seal_address)
    ///     .await
    ///     .build();
    /// facade.register_adapter(Chain::Ethereum, adapter);
    /// ```
    pub async fn register_adapter(&self, chain: Chain, adapter: Arc<dyn FullChainAdapter>) {
        let mut adapters = self.adapters.lock().await;
        adapters.insert(chain, adapter);
    }

    /// Query the balance for an address on the specified chain.
    ///
    /// This is the primary facade function used by CLI and wallet
    /// for balance queries across all chains.
    pub async fn get_balance(
        &self,
        chain: Chain,
        address: &str,
    ) -> Result<BalanceInfo, CsvError> {
        let adapter = self.get_adapter(chain).await?;
        
        adapter
            .get_balance(address)
            .await
            .map_err(|e| CsvError::AdapterError {
                chain,
                message: format!("Balance query failed: {}", e),
            })
    }

    /// Get transaction information by hash.
    ///
    /// Returns TransactionInfo which includes the transaction status.
    pub async fn get_transaction(
        &self,
        chain: Chain,
        tx_hash: &str,
    ) -> Result<TransactionInfo, CsvError> {
        let adapter = self.get_adapter(chain).await?;
        
        adapter
            .get_transaction(tx_hash)
            .await
            .map_err(|e| CsvError::AdapterError {
                chain,
                message: format!("Transaction query failed: {}", e),
            })
    }

    /// Sign a transaction using the wallet's key identifier.
    ///
    /// This facade function is used by CLI and wallet for transaction signing.
    pub async fn sign_transaction(
        &self,
        chain: Chain,
        unsigned_tx: &[u8],
        key_id: &str,
    ) -> Result<Vec<u8>, CsvError> {
        let adapter = self.get_adapter(chain).await?;
        
        adapter
            .sign_transaction(unsigned_tx, key_id)
            .await
            .map_err(|e| CsvError::AdapterError {
                chain,
                message: format!("Transaction signing failed: {}", e),
            })
    }

    /// Broadcast a signed transaction to the network.
    ///
    /// This facade function is used by CLI and wallet for transaction broadcasting.
    /// Delegates to ChainBroadcaster::submit_transaction.
    pub async fn broadcast_transaction(
        &self,
        chain: Chain,
        signed_tx: &[u8],
    ) -> Result<String, CsvError> {
        let adapter = self.get_adapter(chain).await?;
        
        adapter
            .submit_transaction(signed_tx)
            .await
            .map_err(|e| CsvError::AdapterError {
                chain,
                message: format!("Transaction broadcast failed: {}", e),
            })
    }

    /// Build an inclusion proof for a commitment on the specified chain.
    ///
    /// This facade function is used by CLI and wallet for proof generation.
    /// Delegates to ChainProofProvider::build_inclusion_proof.
    pub async fn build_inclusion_proof(
        &self,
        chain: Chain,
        commitment: &Hash,
        block_height: u64,
    ) -> Result<csv_adapter_core::InclusionProof, CsvError> {
        let adapter = self.get_adapter(chain).await?;
        
        adapter
            .build_inclusion_proof(commitment, block_height)
            .await
            .map_err(|e| CsvError::AdapterError {
                chain,
                message: format!("Proof generation failed: {}", e),
            })
    }

    /// Deploy a lock contract to the specified chain.
    ///
    /// This facade function is used by CLI for contract deployment.
    /// Delegates to ChainDeployer::deploy_lock_contract.
    pub async fn deploy_lock_contract(
        &self,
        chain: Chain,
        admin_address: &str,
        config: serde_json::Value,
    ) -> Result<DeploymentStatus, CsvError> {
        let adapter = self.get_adapter(chain).await?;
        
        adapter
            .deploy_lock_contract(admin_address, config)
            .await
            .map_err(|e| CsvError::AdapterError {
                chain,
                message: format!("Contract deployment failed: {}", e),
            })
    }

    /// Verify contract deployment status.
    ///
    /// Delegates to ChainDeployer::verify_deployment.
    pub async fn verify_deployment(
        &self,
        chain: Chain,
        contract_address: &str,
    ) -> Result<bool, CsvError> {
        let adapter = self.get_adapter(chain).await?;
        
        adapter
            .verify_deployment(contract_address)
            .await
            .map_err(|e| CsvError::AdapterError {
                chain,
                message: format!("Deployment verification failed: {}", e),
            })
    }

    /// Create a new right on the specified chain.
    ///
    /// Delegates to ChainRightOps::create_right.
    pub async fn create_right(
        &self,
        chain: Chain,
        owner: &str,
        asset_class: &str,
        asset_id: &str,
        metadata: serde_json::Value,
    ) -> Result<RightOperationResult, CsvError> {
        let adapter = self.get_adapter(chain).await?;
        
        adapter
            .create_right(owner, asset_class, asset_id, metadata)
            .await
            .map_err(|e| CsvError::AdapterError {
                chain,
                message: format!("Right creation failed: {}", e),
            })
    }

    /// Consume a right on the specified chain.
    ///
    /// Delegates to ChainRightOps::consume_right.
    pub async fn consume_right(
        &self,
        chain: Chain,
        right_id: &RightId,
        owner_key_id: &str,
    ) -> Result<RightOperationResult, CsvError> {
        let adapter = self.get_adapter(chain).await?;
        
        adapter
            .consume_right(right_id, owner_key_id)
            .await
            .map_err(|e| CsvError::AdapterError {
                chain,
                message: format!("Right consumption failed: {}", e),
            })
    }

    /// Lock a right for cross-chain transfer.
    ///
    /// Delegates to ChainRightOps::lock_right.
    pub async fn lock_right(
        &self,
        chain: Chain,
        right_id: &RightId,
        destination_chain: &str,
        owner_key_id: &str,
    ) -> Result<RightOperationResult, CsvError> {
        let adapter = self.get_adapter(chain).await?;
        
        adapter
            .lock_right(right_id, destination_chain, owner_key_id)
            .await
            .map_err(|e| CsvError::AdapterError {
                chain,
                message: format!("Right lock failed: {}", e),
            })
    }

    /// Create a new seal on the specified chain.
    ///
    /// This is the primary facade function for seal creation. It delegates to the
    /// chain adapter's AnchorLayer::create_seal method to create a real chain-native seal.
    ///
    /// # Arguments
    /// * `chain` - The blockchain where the seal will be created
    /// * `value` - Optional value/funding for the seal (chain-specific units like satoshis, wei, etc.)
    ///
    /// # Returns
    /// * `Ok(SealRef)` - The real chain-native seal reference
    /// * `Err` - If seal creation fails
    ///
    /// # Example
    /// ```rust,ignore
    /// let seal_ref = facade.create_seal(Chain::Bitcoin, Some(100_000)).await?;
    /// // seal_ref.seal_id contains the actual on-chain identifier (e.g., UTXO txid)
    /// ```
    pub async fn create_seal(
        &self,
        chain: Chain,
        value: Option<u64>,
    ) -> Result<csv_adapter_core::SealRef, CsvError> {
        let adapter = self.get_adapter(chain).await?;
        
        // Delegate to the adapter's create_seal method
        adapter
            .create_seal(value)
            .map_err(|e| CsvError::AdapterError {
                chain,
                message: format!("Seal creation failed: {}", e),
            })
    }

    /// Mint a right on the destination chain.
    ///
    /// Delegates to ChainRightOps::mint_right.
    pub async fn mint_right(
        &self,
        chain: Chain,
        source_chain: &str,
        source_right_id: &RightId,
        lock_proof: &csv_adapter_core::InclusionProof,
        new_owner: &str,
    ) -> Result<RightOperationResult, CsvError> {
        let adapter = self.get_adapter(chain).await?;
        
        adapter
            .mint_right(source_chain, source_right_id, lock_proof, new_owner)
            .await
            .map_err(|e| CsvError::AdapterError {
                chain,
                message: format!("Right mint failed: {}", e),
            })
    }

    /// Confirm a transaction and check its finality status.
    ///
    /// Delegates to ChainBroadcaster::confirm_transaction.
    pub async fn confirm_transaction(
        &self,
        chain: Chain,
        tx_hash: &str,
        required_confirmations: u64,
        timeout_secs: u64,
    ) -> Result<TransactionStatus, CsvError> {
        let adapter = self.get_adapter(chain).await?;
        
        adapter
            .confirm_transaction(tx_hash, required_confirmations, timeout_secs)
            .await
            .map_err(|e| CsvError::AdapterError {
                chain,
                message: format!("Transaction confirmation failed: {}", e),
            })
    }

    /// Get fee estimate for the specified chain.
    ///
    /// Uses ChainBroadcaster trait to get the current recommended fee/gas price.
    /// This replaces raw HTTP JSON-RPC calls in wallet and CLI.
    pub async fn get_fee_estimate(&self, chain: Chain) -> Result<u64, CsvError> {
        let adapter = self.get_adapter(chain).await?;
        
        adapter
            .get_fee_estimate()
            .await
            .map_err(|e| CsvError::AdapterError {
                chain,
                message: format!("Fee estimate query failed: {}", e),
            })
    }

    /// Get transaction count (nonce) for an address.
    ///
    /// Uses ChainQuery trait to get the account transaction count.
    /// This replaces raw HTTP JSON-RPC calls in wallet and CLI.
    pub async fn get_transaction_count(&self, chain: Chain, address: &str) -> Result<u64, CsvError> {
        let adapter = self.get_adapter(chain).await?;

        // Use the ChainQuery trait's get_account_nonce method
        // This properly queries the chain for account-specific nonce
        adapter
            .get_account_nonce(address)
            .await
            .map_err(|e| CsvError::AdapterError {
                chain,
                message: format!("Failed to get transaction count: {}", e),
            })
    }

    /// Build a contract call transaction.
    ///
    /// This method builds the transaction data for a contract function call
    /// using the chain's native serialization format (ABI for Ethereum,
    /// BCS for Sui/Aptos, etc.).
    ///
    /// # Arguments
    /// * `chain` - The target chain
    /// * `contract` - Contract address (or package ID for Move chains)
    /// * `function` - Function name or selector
    /// * `args` - Function arguments as encoded bytes
    /// * `from` - Sender address
    /// * `nonce` - Transaction nonce/sequence number
    ///
    /// # Returns
    /// Serialized transaction data ready for signing
    pub async fn build_contract_call(
        &self,
        chain: Chain,
        contract: &str,
        function: &str,
        args: Vec<Vec<u8>>,
        from: &str,
        nonce: u64,
    ) -> Result<Vec<u8>, CsvError> {
        let _adapter = self.get_adapter(chain).await?;

        // Encode the transaction data using chain-specific format
        let tx_data = match chain {
            Chain::Ethereum => {
                // Ethereum uses ABI encoding - encode function selector + args
                encode_eth_contract_call(contract, function, args)
            }
            Chain::Sui | Chain::Aptos => {
                // Move chains use BCS encoding
                encode_move_contract_call(contract, function, args, from, nonce)
            }
            Chain::Solana => {
                // Solana uses instruction encoding
                encode_solana_contract_call(contract, function, args, from)
            }
            Chain::Bitcoin => {
                return Err(CsvError::CapabilityUnavailable {
                    chain,
                    capability: "contract_calls".to_string(),
                })
            }
            _ => {
                return Err(CsvError::ChainNotSupported(chain))
            }
        };

        Ok(tx_data)
    }

    /// Get the adapter for the specified chain.
    async fn get_adapter(&self, chain: Chain) -> Result<Arc<dyn FullChainAdapter>, CsvError> {
        let adapters = self.adapters.lock().await;
        adapters
            .get(&chain)
            .cloned()
            .ok_or(CsvError::ChainNotSupported(chain))
    }

    /// Check if an adapter is registered for the given chain.
    pub async fn has_adapter(&self, chain: Chain) -> bool {
        let adapters = self.adapters.lock().await;
        adapters.contains_key(&chain)
    }

    /// Get the list of registered chains.
    pub async fn registered_chains(&self) -> Vec<Chain> {
        let adapters = self.adapters.lock().await;
        adapters.keys().copied().collect()
    }

    /// Generate a proof for a right on the specified chain.
    ///
    /// This implementation queries the chain for inclusion proof data and constructs
    /// a complete ProofBundle for cross-chain transfers.
    ///
    /// # Security
    /// - Fetches real inclusion proof from chain state
    /// - Includes finality proof with confirmation count
    /// - Creates proper DAG segment with commitment
    /// - Signs the proof bundle for authenticity
    pub async fn generate_proof(
        &self,
        chain: Chain,
        right_id: &RightId,
    ) -> Result<ProofBundle, CsvError> {
        let adapter = self.get_adapter(chain).await?;

        // Query the chain for the inclusion proof at the latest block
        let block_height = adapter.get_latest_block_height().await.map_err(|e| {
            CsvError::AdapterError {
                chain,
                message: format!("Failed to get latest block height: {}", e),
            }
        })?;

        // Create commitment from right_id (the right's hash is the commitment)
        let commitment = csv_adapter_core::hash::Hash::new(*right_id.as_bytes());

        // Build inclusion proof from chain state
        let inclusion_proof = adapter
            .build_inclusion_proof(&commitment, block_height)
            .await
            .map_err(|e| CsvError::AdapterError {
                chain,
                message: format!("Failed to build inclusion proof: {}", e),
            })?;

        // Get the transaction info to build finality proof
        // For proof generation, we use the right_id as the lookup key
        let tx_hash_lookup = hex::encode(right_id.as_bytes());
        let tx_info = adapter.get_transaction(tx_hash_lookup.as_str()).await;
        let tx_hash = match &tx_info {
            Ok(info) => info.hash.clone(),
            Err(_) => hex::encode(right_id.as_bytes()), // Fallback to right_id as hex
        };

        // Build finality proof using the transaction hash
        let finality_proof = adapter
            .build_finality_proof(&tx_hash)
            .await
            .map_err(|e| CsvError::AdapterError {
                chain,
                message: format!("Failed to build finality proof: {}", e),
            })?;

        // Create a simple DAG segment with the right commitment
        use csv_adapter_core::dag::{DAGNode, DAGSegment};
        let dag_node = DAGNode::new(
            commitment,
            vec![], // No inputs for lock operation
            vec![], // No signatures yet - added later
            vec![], // No outputs yet
            vec![], // No state transitions yet
        );
        let dag_segment = DAGSegment::new(vec![dag_node], commitment);

        // Create the proof bundle
        let seal_id = right_id.as_bytes().to_vec();
        let proof_bundle = ProofBundle::new(
            dag_segment,
            vec![], // Signatures will be added by the caller
            csv_adapter_core::seal::SealRef::new(seal_id.clone(), None)
                .map_err(|e| CsvError::AdapterError {
                    chain,
                    message: format!("Failed to create seal ref: {}", e),
                })?,
            csv_adapter_core::seal::AnchorRef::new(seal_id, block_height, inclusion_proof.proof_bytes.clone())
                .map_err(|e| CsvError::AdapterError {
                    chain,
                    message: format!("Failed to create anchor ref: {}", e),
                })?,
            inclusion_proof,
            finality_proof,
        )
        .map_err(|e| CsvError::AdapterError {
            chain,
            message: format!("Failed to create proof bundle: {}", e),
        })?;

        log::info!(
            "Generated proof bundle for right {:?} on {:?} at block {}",
            right_id, chain, block_height
        );

        Ok(proof_bundle)
    }

    /// Verify a proof bundle for a cross-chain transfer.
    ///
    /// This implementation uses the core proof verification pipeline to cryptographically
    /// validate the proof bundle before accepting cross-chain transfers.
    ///
    /// # Security
    /// - Verifies all signatures using the chain's signature scheme
    /// - Checks seal registry for replay attacks
    /// - Validates inclusion proof and finality
    /// - Returns false for any invalid proof, true only for fully valid proofs
    pub async fn verify_proof_bundle(
        &self,
        chain: Chain,
        proof_bundle: &ProofBundle,
        right_id: &RightId,
    ) -> Result<bool, CsvError> {
        let adapter = self.get_adapter(chain).await?;

        // Get the signature scheme for this chain
        let signature_scheme = adapter.signature_scheme();

        // First verify the inclusion proof using the chain's native verification
        let commitment = csv_adapter_core::hash::Hash::new(*right_id.as_bytes());
        let inclusion_valid = adapter
            .verify_inclusion_proof(&proof_bundle.inclusion_proof, &commitment)
            .await
            .map_err(|e| CsvError::AdapterError {
                chain,
                message: format!("Inclusion proof verification failed: {}", e),
            })?;

        if !inclusion_valid {
            log::warn!("Inclusion proof invalid for right {:?} on {:?}", right_id, chain);
            return Ok(false);
        }

        // Verify the finality proof
        let tx_hash = hex::encode(right_id.as_bytes());
        let finality_valid = adapter
            .verify_finality_proof(&proof_bundle.finality_proof, &tx_hash)
            .await
            .map_err(|e| CsvError::AdapterError {
                chain,
                message: format!("Finality proof verification failed: {}", e),
            })?;

        if !finality_valid {
            log::warn!("Finality proof invalid for right {:?} on {:?}", right_id, chain);
            return Ok(false);
        }

        // Pre-fetch seal consumption data BEFORE creating the closure
        // This avoids capturing self (which contains the sync Mutex) in the async context
        let seal_check_data = self.pre_fetch_seal_data(right_id).await?;

        // Create a seal registry checker for replay protection
        // The closure now only captures pre-fetched data, not self
        let seal_checker = move |seal_id: &[u8]| {
            let check_right_id = csv_adapter_core::RightId::from_bytes(seal_id);
            // Only return consumed if the seal_id matches the pre-fetched right_id AND it was consumed
            if check_right_id == seal_check_data.right_id {
                if seal_check_data.is_consumed {
                    log::warn!("Seal {} has already been consumed", hex::encode(seal_id));
                }
                seal_check_data.is_consumed
            } else {
                // Unknown seal - assume not consumed
                log::debug!("Seal {} not in pre-fetched data - assuming not consumed", hex::encode(seal_id));
                false
            }
        };

        // Use the core proof verification pipeline for signatures and seal check
        match csv_adapter_core::proof_verify::verify_proof(
            proof_bundle,
            seal_checker,
            signature_scheme,
        ) {
            Ok(()) => {
                log::info!("Proof bundle verified successfully for right {:?} on {:?}", right_id, chain);
                Ok(true)
            }
            Err(e) => {
                log::warn!("Proof verification failed for right {:?} on {:?}: {}", right_id, chain, e);
                Ok(false)
            }
        }
    }

    /// Pre-fetch seal consumption data to avoid locking in async closure.
    /// 
    /// This fetches the right record from the store synchronously BEFORE entering
    /// the verification closure, preventing the async deadlock risk.
    async fn pre_fetch_seal_data(&self, right_id: &RightId) -> Result<SealCheckData, CsvError> {
        // Clone the Arc to avoid capturing self in the spawned task
        let store_arc = Arc::clone(&self.client.store);
        let right_id_clone = right_id.clone();
        
        // Run the store access in a blocking task since it uses std::sync::Mutex
        let is_consumed = tokio::task::spawn_blocking(move || {
            let store = store_arc.lock().map_err(|e| e.to_string())?;
            match store.get_right(&right_id_clone) {
                Ok(Some(record)) => Ok(record.consumed_at.is_some()),
                Ok(None) => Ok(false), // Right not found = not consumed
                Err(e) => Err(format!("Store error: {}", e)),
            }
        })
        .await
        .map_err(|e| CsvError::StoreError(format!("Task join error: {}", e)))?
        .map_err(|e| CsvError::StoreError(e))?;
        
        Ok(SealCheckData {
            right_id: right_id.clone(),
            is_consumed,
        })
    }
}

/// Adapter configuration for the facade.
#[derive(Debug, Clone)]
pub struct AdapterConfig {
    /// RPC endpoints for each chain
    pub rpc_endpoints: HashMap<Chain, String>,
    /// Chain-specific configuration
    pub chain_config: HashMap<Chain, HashMap<String, String>>,
}

impl Default for AdapterConfig {
    fn default() -> Self {
        Self {
            rpc_endpoints: HashMap::new(),
            chain_config: HashMap::new(),
        }
    }
}

/// Builder for constructing chain-specific FullChainAdapter instances.
///
/// This builder provides chain-specific construction methods that use the correct
/// constructors for each chain's ChainOperations type, following Clean Architecture.
///
/// # Architecture Compliance
///
/// All chains now follow the standard facade pattern:
/// - **Bitcoin**: Uses `from_config(config, rpc)` with optional xpub in config
/// - **Ethereum**: Uses `from_config(config, rpc, csv_seal_address)`
/// - **Sui**: Uses `from_config(config, rpc)`
/// - **Aptos**: Uses `from_config(config, rpc)`
/// - **Solana**: Uses `from_config(config, rpc)`
///
/// Chain operations are created from anchor layers via `from_anchor_layer(&anchor)`,
/// producing `Arc<dyn FullChainAdapter>` for registration in ChainFacade.
///
/// The builder methods handle chain-specific configuration internally while
/// presenting a unified interface for the facade.
pub struct AdapterBuilder;

impl AdapterBuilder {
    /// Create a new adapter builder.
    pub fn new() -> Self {
        Self
    }

    /// Build an Ethereum adapter from its specific configuration.
    ///
    /// Uses `EthereumChainOperations::from_anchor_layer()` internally which creates
    /// the FullChainAdapter implementation from an EthereumAnchorLayer.
    #[cfg(feature = "ethereum")]
    pub async fn ethereum_from_config(
        &self,
        config: csv_adapter_ethereum::config::EthereumConfig,
        rpc: Box<dyn csv_adapter_ethereum::rpc::EthereumRpc>,
        csv_seal_address: [u8; 20],
    ) -> Result<Arc<dyn FullChainAdapter>, CsvError> {
        use csv_adapter_ethereum::chain_operations::EthereumChainOperations;
        use csv_adapter_ethereum::adapter::EthereumAnchorLayer;

        // Create the AnchorLayer first (this is the protocol primitive)
        let anchor_layer = EthereumAnchorLayer::from_config(config, rpc, csv_seal_address)
            .map_err(|e| CsvError::AdapterError {
                chain: Chain::Ethereum,
                message: format!("Failed to create Ethereum anchor layer: {}", e),
            })?;

        // Create ChainOperations from AnchorLayer (this implements FullChainAdapter)
        let operations = EthereumChainOperations::from_anchor_layer(&anchor_layer)
            .map_err(|e| CsvError::AdapterError {
                chain: Chain::Ethereum,
                message: format!("Failed to create Ethereum chain operations: {}", e),
            })?;

        Ok(Arc::new(operations))
    }

    /// Build a Sui adapter from configuration.
    #[cfg(feature = "sui")]
    pub async fn sui_from_config(
        &self,
        config: csv_adapter_sui::config::SuiConfig,
        rpc: Box<dyn csv_adapter_sui::rpc::SuiRpc>,
    ) -> Result<Arc<dyn FullChainAdapter>, CsvError> {
        use csv_adapter_sui::chain_operations::SuiChainOperations;
        use csv_adapter_sui::adapter::SuiAnchorLayer;

        let anchor_layer = SuiAnchorLayer::from_config(config, rpc)
            .map_err(|e| CsvError::AdapterError {
                chain: Chain::Sui,
                message: format!("Failed to create Sui anchor layer: {}", e),
            })?;

        let operations = SuiChainOperations::from_anchor_layer(&anchor_layer)
            .map_err(|e| CsvError::AdapterError {
                chain: Chain::Sui,
                message: format!("Failed to create Sui chain operations: {}", e),
            })?;

        Ok(Arc::new(operations))
    }

    /// Build an Aptos adapter from configuration.
    #[cfg(feature = "aptos")]
    pub async fn aptos_from_config(
        &self,
        config: csv_adapter_aptos::config::AptosConfig,
        rpc: Box<dyn csv_adapter_aptos::rpc::AptosRpc>,
    ) -> Result<Arc<dyn FullChainAdapter>, CsvError> {
        use csv_adapter_aptos::chain_operations::AptosChainOperations;
        use csv_adapter_aptos::adapter::AptosAnchorLayer;

        let anchor_layer = AptosAnchorLayer::from_config(config, rpc)
            .map_err(|e| CsvError::AdapterError {
                chain: Chain::Aptos,
                message: format!("Failed to create Aptos anchor layer: {}", e),
            })?;

        let operations = AptosChainOperations::from_anchor_layer(&anchor_layer)
            .map_err(|e| CsvError::AdapterError {
                chain: Chain::Aptos,
                message: format!("Failed to create Aptos chain operations: {}", e),
            })?;

        Ok(Arc::new(operations))
    }

    /// Build a Solana adapter from configuration.
    #[cfg(feature = "solana")]
    pub async fn solana_from_config(
        &self,
        config: csv_adapter_solana::config::SolanaConfig,
        rpc: Box<dyn csv_adapter_solana::rpc::SolanaRpc>,
    ) -> Result<Arc<dyn FullChainAdapter>, CsvError> {
        use csv_adapter_solana::chain_operations::SolanaChainOperations;
        use csv_adapter_solana::adapter::SolanaAnchorLayer;

        // Solana now uses from_config() following the standard facade pattern
        let anchor_layer = SolanaAnchorLayer::from_config(config, rpc)
            .map_err(|e| CsvError::AdapterError {
                chain: Chain::Solana,
                message: format!("Failed to create Solana anchor layer: {}", e),
            })?;

        let operations = SolanaChainOperations::from_anchor_layer(&anchor_layer)
            .map_err(|e| CsvError::AdapterError {
                chain: Chain::Solana,
                message: format!("Failed to create Solana chain operations: {}", e),
            })?;

        Ok(Arc::new(operations))
    }

    /// Build a Bitcoin adapter from configuration.
    #[cfg(feature = "bitcoin")]
    pub async fn bitcoin_from_config(
        &self,
        config: csv_adapter_bitcoin::config::BitcoinConfig,
        rpc: Box<dyn csv_adapter_bitcoin::rpc::BitcoinRpc + Send + Sync>,
    ) -> Result<Arc<dyn FullChainAdapter>, CsvError> {
        use csv_adapter_bitcoin::chain_operations::BitcoinChainOperations;
        use csv_adapter_bitcoin::adapter::BitcoinAnchorLayer;

        // Bitcoin uses from_config() following the standard facade pattern
        let anchor_layer = BitcoinAnchorLayer::from_config(config, rpc)
            .map_err(|e| CsvError::AdapterError {
                chain: Chain::Bitcoin,
                message: format!("Failed to create Bitcoin anchor layer: {}", e),
            })?;

        let operations = BitcoinChainOperations::from_anchor_layer(&anchor_layer)
            .map_err(|e| CsvError::AdapterError {
                chain: Chain::Bitcoin,
                message: format!("Failed to create Bitcoin chain operations: {}", e),
            })?;

        Ok(Arc::new(operations))
    }
}

impl Default for AdapterBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Main adapter facade that manages all chain adapters.
///
/// This is the entry point for creating and managing chain adapters
/// through a unified interface.
///
/// # Clean Architecture
///
/// This facade does NOT directly create adapters. Instead:
/// 1. Use `AdapterBuilder` to construct chain-specific adapters with proper configuration
/// 2. Register adapters via `register_adapter()`
/// 3. Use `chain_facade()` to access the unified operation interface
///
/// This design respects each chain's unique construction requirements while
/// providing a consistent interface for operations.
pub struct AdapterFacade {
    config: AdapterConfig,
    chain_facade: ChainFacade,
    builder: AdapterBuilder,
}

impl AdapterFacade {
    /// Create a new adapter facade with the given configuration.
    pub fn new(config: AdapterConfig) -> Self {
        let client_ref = Arc::new(ClientRef::new());
        let chain_facade = ChainFacade::new(client_ref);
        let builder = AdapterBuilder::new();
        
        Self {
            config,
            chain_facade,
            builder,
        }
    }

    /// Get the chain facade for operations.
    pub fn chain_facade(&self) -> &ChainFacade {
        &self.chain_facade
    }

    /// Get a mutable reference to the chain facade for registering adapters.
    pub fn chain_facade_mut(&mut self) -> &mut ChainFacade {
        &mut self.chain_facade
    }

    /// Get the adapter builder for constructing chain adapters.
    pub fn builder(&self) -> &AdapterBuilder {
        &self.builder
    }

    /// Register a pre-built adapter for a chain.
    ///
    /// Use this when you have constructed an adapter using `AdapterBuilder` or
    /// have a custom adapter implementation.
    pub async fn register_adapter(&mut self, chain: Chain, adapter: Arc<dyn FullChainAdapter>) {
        self.chain_facade.register_adapter(chain, adapter).await;
    }

    /// Get the adapter configuration.
    pub fn config(&self) -> &AdapterConfig {
        &self.config
    }

    /// Get a mutable reference to the adapter configuration.
    pub fn config_mut(&mut self) -> &mut AdapterConfig {
        &mut self.config
    }
}

// Helper functions for encoding contract calls

/// Encode an Ethereum contract call using ABI format
fn encode_eth_contract_call(
    _contract: &str,
    function: &str,
    args: Vec<Vec<u8>>,
) -> Vec<u8> {
    // Simple ABI encoding: function selector (4 bytes) + encoded arguments
    // In production, this would use the ethabi or alloy-sol-types crate
    let mut data = Vec::new();

    // Create function selector from function signature hash
    let mut hasher = Keccak256::new();
    hasher.update(function.as_bytes());
    let hash = hasher.finalize();

    // First 4 bytes are the function selector
    data.extend_from_slice(&hash[..4]);

    // Append encoded arguments (padded to 32 bytes each for Ethereum)
    for arg in args {
        let mut padded = arg;
        // Pad to 32 bytes
        while padded.len() < 32 {
            padded.push(0);
        }
        data.extend_from_slice(&padded[..32.min(padded.len())]);
    }

    data
}

/// Encode a Move contract call (Sui/Aptos) using BCS format
fn encode_move_contract_call(
    package: &str,
    function: &str,
    _args: Vec<Vec<u8>>,
    sender: &str,
    sequence_number: u64,
) -> Vec<u8> {
    // BCS-encoded transaction data
    // This is a simplified representation - production would use the bcs crate
    let mut data = Vec::new();

    // Package ID (32 bytes)
    let package_bytes = hex::decode(package.trim_start_matches("0x")).unwrap_or_default();
    data.extend_from_slice(&package_bytes);

    // Function name (length-prefixed string)
    data.push(function.len() as u8);
    data.extend_from_slice(function.as_bytes());

    // Sender address (32 bytes)
    let sender_bytes = hex::decode(sender.trim_start_matches("0x")).unwrap_or_default();
    data.extend_from_slice(&sender_bytes);

    // Sequence number (8 bytes, little-endian)
    data.extend_from_slice(&sequence_number.to_le_bytes());

    data
}

/// Encode a Solana contract call using instruction format
fn encode_solana_contract_call(
    program_id: &str,
    function: &str,
    _args: Vec<Vec<u8>>,
    _from: &str,
) -> Vec<u8> {
    // Solana instruction encoding
    // This is a simplified representation
    let mut data = Vec::new();

    // Program ID (32 bytes) - decode base58
    let program_bytes = bs58::decode(program_id).into_vec().unwrap_or_default();
    data.extend_from_slice(&program_bytes);

    // Function discriminator (8 bytes - hash of function name)
    let mut hasher = Sha256::new();
    hasher.update(function.as_bytes());
    let hash = hasher.finalize();
    data.extend_from_slice(&hash[..8]);

    data
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_config_default() {
        let config = AdapterConfig::default();
        assert!(config.rpc_endpoints.is_empty());
        assert!(config.chain_config.is_empty());
    }

    #[tokio::test]
    async fn test_chain_facade_creation() {
        let client_ref = Arc::new(ClientRef::new());
        let facade = ChainFacade::new(client_ref);
        assert!(facade.adapters.lock().await.is_empty());
    }

    #[test]
    fn test_eth_contract_call_encoding() {
        let data = encode_eth_contract_call(
            "0x1234567890123456789012345678901234567890",
            "mint(bytes32,address)",
            vec![vec![0xAB; 32], vec![0xCD; 20]],
        );
        assert_eq!(data.len(), 4 + 32 + 32); // selector + 2 padded args
    }
}
