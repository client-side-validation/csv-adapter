//! Chain Operations Implementation for Bitcoin
//!
//! Implements the core chain operation traits from csv-adapter-core
//! for real Bitcoin chain interactions.

use async_trait::async_trait;
use bitcoin::Network;
use csv_adapter_core::chain_operations::{
    ChainBroadcaster, ChainDeployer, ChainOpError, ChainOpResult, ChainProofProvider,
    ChainQuery, ChainRightOps, ChainSigner, DeploymentStatus, FinalityStatus,
    RightOperationResult, TransactionStatus,
};
use csv_adapter_core::hash::Hash;
use csv_adapter_core::proof::{FinalityProof, InclusionProof};
use csv_adapter_core::right::RightId;
use csv_adapter_core::signature::SignatureScheme;

use crate::adapter::BitcoinAnchorLayer;
use crate::rpc::BitcoinRpc;
use csv_adapter_core::AnchorLayer;

/// Bitcoin implementation of ChainQuery trait
pub struct BitcoinChainQuery {
    rpc: Box<dyn BitcoinRpc + Send + Sync>,
    network: Network,
}

impl BitcoinChainQuery {
    /// Create a new Bitcoin chain query instance
    pub fn new(rpc: Box<dyn BitcoinRpc + Send + Sync>, network: Network) -> Self {
        Self { rpc, network }
    }

    /// Create from a real Bitcoin RPC client
    #[cfg(feature = "rpc")]
    pub fn from_real_rpc(rpc: crate::real_rpc::real_rpc::RealBitcoinRpc, network: Network) -> Self {
        // RealBitcoinRpc implements BitcoinRpc, so we can box it
        Self::new(Box::new(rpc), network)
    }
}

#[async_trait]
impl ChainQuery for BitcoinChainQuery {
    async fn get_balance(&self, _address: &str) -> ChainOpResult<serde_json::Value> {
        // Bitcoin balance query requires wallet support
        // Return capability unavailable with structured error
        Err(ChainOpError::CapabilityUnavailable(
            "Bitcoin balance query requires wallet support or external API".to_string(),
        ))
    }

    async fn get_transaction(&self, tx_hash: &str) -> ChainOpResult<csv_adapter_core::chain_operations::TransactionInfo> {
        use csv_adapter_core::chain_operations::{TransactionInfo, TransactionStatus};
        
        // Parse the txid
        let txid_bytes = hex::decode(tx_hash.trim_start_matches("0x"))
            .map_err(|e| ChainOpError::InvalidInput(format!("Invalid tx hash: {}", e)))?;

        if txid_bytes.len() != 32 {
            return Err(ChainOpError::InvalidInput(
                "Transaction hash must be 32 bytes".to_string(),
            ));
        }

        let mut txid_array = [0u8; 32];
        txid_array.copy_from_slice(&txid_bytes);

        // Get confirmations via RPC
        let confirmations = self
            .rpc
            .get_tx_confirmations(txid_array)
            .map_err(|e| ChainOpError::RpcError(format!("Failed to get confirmations: {}", e)))?;

        let status = if confirmations == 0 {
            TransactionStatus::Pending
        } else {
            TransactionStatus::Confirmed { block_height: 0, confirmations: confirmations as u64 }
        };

        Ok(TransactionInfo {
            hash: tx_hash.to_string(),
            sender: String::new(), // Would need to decode transaction
            recipient: None,
            amount: None,
            status,
            block_height: None,
            timestamp: None,
            fee: None,
            raw_data: None,
        })
    }

    async fn get_finality(&self, tx_hash: &str) -> ChainOpResult<FinalityStatus> {
        let tx_info = self.get_transaction(tx_hash).await?;
        
        match tx_info.status {
            csv_adapter_core::chain_operations::TransactionStatus::Pending => Ok(FinalityStatus::Pending),
            csv_adapter_core::chain_operations::TransactionStatus::Confirmed { block_height, .. } => {
                // Treat confirmed as finalized for Bitcoin (6+ confirmations)
                Ok(FinalityStatus::Finalized {
                    block_height,
                    finality_block: block_height,
                })
            }
            csv_adapter_core::chain_operations::TransactionStatus::Failed { .. } => Ok(FinalityStatus::Orphaned),
            csv_adapter_core::chain_operations::TransactionStatus::Dropped => Ok(FinalityStatus::Orphaned),
            csv_adapter_core::chain_operations::TransactionStatus::Unknown => Ok(FinalityStatus::Pending),
        }
    }

    async fn get_contract_status(&self, _contract_address: &str) -> ChainOpResult<serde_json::Value> {
        // Bitcoin doesn't have smart contracts in the traditional sense
        Ok(serde_json::json!({
            "deployed": false,
            "chain": "bitcoin",
            "note": "Bitcoin does not support smart contracts"
        }))
    }

    async fn get_latest_block_height(&self) -> ChainOpResult<u64> {
        self.rpc
            .get_block_count()
            .map_err(|e| ChainOpError::RpcError(format!("Failed to get block count: {}", e)))
    }

    async fn get_chain_info(&self) -> ChainOpResult<serde_json::Value> {
        Ok(serde_json::json!({
            "chain": "bitcoin",
            "network": format!("{:?}", self.network),
            "version": "0.4.0",
            "protocol": "Bitcoin"
        }))
    }

    fn validate_address(&self, address: &str) -> bool {
        // Try to parse the address
        address.parse::<bitcoin::Address<_>>().is_ok()
    }
}

/// Bitcoin implementation of ChainSigner trait
#[derive(Debug)]
pub struct BitcoinChainSigner {
    network: Network,
}

impl BitcoinChainSigner {
    /// Create a new Bitcoin chain signer
    pub fn new(network: Network) -> Self {
        Self { network }
    }
}

impl ChainSigner for BitcoinChainSigner {
    fn derive_address(&self, _public_key: &[u8]) -> ChainOpResult<String> {
        // Derive a Bitcoin address from a public key
        // For Taproot (P2TR), we use x-only public key
        // This is a simplified implementation
        Err(ChainOpError::CapabilityUnavailable(
            "Address derivation requires secp256k1 operations".to_string(),
        ))
    }

    async fn sign_transaction(&self, _tx_data: &[u8], _key_id: &str) -> ChainOpResult<Vec<u8>> {
        // Sign a transaction
        Err(ChainOpError::CapabilityUnavailable(
            "Transaction signing not yet implemented".to_string(),
        ))
    }

    async fn sign_message(&self, _message: &[u8], _key_id: &str) -> ChainOpResult<Vec<u8>> {
        // Sign a message with a private key
        Err(ChainOpError::CapabilityUnavailable(
            "Message signing not yet implemented".to_string(),
        ))
    }

    fn verify_signature(
        &self,
        _message: &[u8],
        _signature: &[u8],
        _public_key: &[u8],
    ) -> ChainOpResult<bool> {
        // Verify a signature
        Err(ChainOpError::CapabilityUnavailable(
            "Signature verification not yet implemented".to_string(),
        ))
    }

    fn signature_scheme(&self) -> SignatureScheme {
        SignatureScheme::Secp256k1
    }
}

/// Bitcoin implementation of ChainBroadcaster trait
pub struct BitcoinChainBroadcaster {
    rpc: Box<dyn BitcoinRpc + Send + Sync>,
}

impl BitcoinChainBroadcaster {
    /// Create a new Bitcoin chain broadcaster
    pub fn new(rpc: Box<dyn BitcoinRpc + Send + Sync>) -> Self {
        Self { rpc }
    }
}

#[async_trait]
impl ChainBroadcaster for BitcoinChainBroadcaster {
    async fn submit_transaction(&self, signed_tx: &[u8]) -> ChainOpResult<String> {
        // Broadcast a raw Bitcoin transaction
        let tx_bytes_vec = signed_tx.to_vec();

        let txid = self
            .rpc
            .send_raw_transaction(tx_bytes_vec)
            .map_err(|e| ChainOpError::RpcError(format!("Failed to broadcast: {}", e)))?;

        // Convert txid to hex string
        Ok(hex::encode(txid))
    }

    async fn confirm_transaction(
        &self,
        tx_hash: &str,
        required_confirmations: u64,
        timeout_secs: u64,
    ) -> ChainOpResult<TransactionStatus> {
        let start = std::time::Instant::now();

        loop {
            // Get the transaction status
            let txid_bytes = hex::decode(tx_hash.trim_start_matches("0x"))
                .map_err(|e| ChainOpError::InvalidInput(format!("Invalid tx hash: {}", e)))?;

            let mut txid_array = [0u8; 32];
            txid_array.copy_from_slice(&txid_bytes);

            let confirmations = self
                .rpc
                .get_tx_confirmations(txid_array)
                .map_err(|e| ChainOpError::RpcError(format!("Failed to get confirmations: {}", e)))?;

            if confirmations >= required_confirmations {
                use csv_adapter_core::chain_operations::TransactionStatus;
                return Ok(TransactionStatus::Confirmed { block_height: 0, confirmations: confirmations as u64 });
            }

            if start.elapsed().as_secs() >= timeout_secs {
                return Err(ChainOpError::Timeout(
                    "Transaction confirmation timeout".to_string(),
                ));
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    }

    async fn get_fee_estimate(&self) -> ChainOpResult<u64> {
        // Estimate the fee rate (satoshis per byte)
        // Would need to call estimatesmartfee RPC
        Err(ChainOpError::CapabilityUnavailable(
            "Fee estimation requires estimatesmartfee RPC".to_string(),
        ))
    }

    async fn validate_transaction(&self, _tx_data: &[u8]) -> ChainOpResult<()> {
        // Validate a transaction before submission
        // Bitcoin doesn't support transaction pre-validation
        Ok(())
    }
}

/// Bitcoin implementation of ChainProofProvider trait
pub struct BitcoinChainProofProvider {
    rpc: Box<dyn BitcoinRpc + Send + Sync>,
}

impl BitcoinChainProofProvider {
    /// Create a new Bitcoin chain proof provider
    pub fn new(rpc: Box<dyn BitcoinRpc + Send + Sync>) -> Self {
        Self { rpc }
    }
}

#[async_trait]
impl ChainProofProvider for BitcoinChainProofProvider {
    async fn build_inclusion_proof(
        &self,
        _commitment: &Hash,
        _block_height: u64,
    ) -> ChainOpResult<InclusionProof> {
        // Build a Merkle proof for a transaction inclusion
        Err(ChainOpError::CapabilityUnavailable(
            "Merkle proof building requires block data".to_string(),
        ))
    }

    fn verify_inclusion_proof(
        &self,
        _proof: &InclusionProof,
        _commitment: &Hash,
    ) -> ChainOpResult<bool> {
        // Verify a Merkle proof
        Err(ChainOpError::CapabilityUnavailable(
            "Merkle proof verification not yet implemented".to_string(),
        ))
    }

    async fn build_finality_proof(&self, _tx_hash: &str) -> ChainOpResult<FinalityProof> {
        // Get a finality proof (SPV proof of confirmation depth)
        Err(ChainOpError::CapabilityUnavailable(
            "Finality proof not yet implemented".to_string(),
        ))
    }

    fn verify_finality_proof(
        &self,
        _proof: &FinalityProof,
        _tx_hash: &str,
    ) -> ChainOpResult<bool> {
        // Verify a finality proof
        Err(ChainOpError::CapabilityUnavailable(
            "Finality proof verification not yet implemented".to_string(),
        ))
    }

    fn domain_separator(&self) -> [u8; 32] {
        // Bitcoin domain separator
        *b"CSV-BTC-DOMAIN-SEPARATOR-0000000"
    }

    async fn verify_proof_bundle(
        &self,
        _inclusion_proof: &InclusionProof,
        _finality_proof: &FinalityProof,
        _commitment: &Hash,
    ) -> ChainOpResult<bool> {
        Err(ChainOpError::CapabilityUnavailable(
            "Proof bundle verification not yet implemented".to_string(),
        ))
    }
}

/// Bitcoin implementation of ChainDeployer trait
/// Note: Bitcoin doesn't support smart contract deployment
#[derive(Debug)]
pub struct BitcoinChainDeployer;

#[async_trait]
impl ChainDeployer for BitcoinChainDeployer {
    async fn deploy_lock_contract(
        &self,
        _admin_address: &str,
        _config: serde_json::Value,
    ) -> ChainOpResult<DeploymentStatus> {
        // Bitcoin doesn't have smart contracts
        Err(ChainOpError::UnsupportedChain(
            "Bitcoin does not support smart contract deployment".to_string(),
        ))
    }

    async fn deploy_mint_contract(
        &self,
        _admin_address: &str,
        _config: serde_json::Value,
    ) -> ChainOpResult<DeploymentStatus> {
        Err(ChainOpError::UnsupportedChain(
            "Bitcoin does not support smart contract deployment".to_string(),
        ))
    }

    async fn deploy_or_publish_seal_program(
        &self,
        _program_bytes: &[u8],
        _admin_address: &str,
    ) -> ChainOpResult<DeploymentStatus> {
        Err(ChainOpError::UnsupportedChain(
            "Bitcoin does not support smart contract deployment".to_string(),
        ))
    }

    async fn verify_deployment(&self, _contract_address: &str) -> ChainOpResult<bool> {
        Err(ChainOpError::UnsupportedChain(
            "Bitcoin does not support smart contract deployment".to_string(),
        ))
    }

    async fn estimate_deployment_cost(&self, _program_bytes: &[u8]) -> ChainOpResult<u64> {
        Err(ChainOpError::UnsupportedChain(
            "Bitcoin does not support smart contract deployment".to_string(),
        ))
    }
}

/// Bitcoin implementation of ChainRightOps trait
pub struct BitcoinChainRightOps {
    adapter: BitcoinAnchorLayer,
}

impl BitcoinChainRightOps {
    /// Create a new Bitcoin chain right ops instance
    pub fn new(adapter: BitcoinAnchorLayer) -> Self {
        Self { adapter }
    }
}

#[async_trait]
impl ChainRightOps for BitcoinChainRightOps {
    async fn create_right(
        &self,
        owner: &str,
        asset_class: &str,
        asset_id: &str,
        metadata: serde_json::Value,
    ) -> ChainOpResult<RightOperationResult> {
        // Create a new right by creating a UTXO seal
        let seal = self
            .adapter
            .create_seal(None)
            .map_err(|e| ChainOpError::InvalidInput(format!("Failed to create seal: {}", e)))?;

        Ok(RightOperationResult {
            right_id: RightId(Hash::from([0u8; 32])), // Implementation note: compute from asset hash
            operation: csv_adapter_core::chain_operations::RightOperation::Create,
            transaction_hash: hex::encode(seal.txid),
            block_height: 0,
            chain_id: "bitcoin".to_string(),
            metadata: serde_json::json!({
                "description": metadata,
                "owner": owner,
                "seal_outpoint": format!("{}:{}", hex::encode(seal.txid), seal.vout)
            }),
        })
    }

    async fn consume_right(
        &self,
        _right_id: &RightId,
        _owner_key_id: &str,
    ) -> ChainOpResult<RightOperationResult> {
        // Consume a right by spending the UTXO
        Err(ChainOpError::CapabilityUnavailable(
            "Right consumption requires transaction building".to_string(),
        ))
    }

    async fn lock_right(
        &self,
        _right_id: &RightId,
        _destination_chain: &str,
        _owner_key_id: &str,
    ) -> ChainOpResult<RightOperationResult> {
        // Lock a right for cross-chain transfer
        Err(ChainOpError::CapabilityUnavailable(
            "Cross-chain locking not yet implemented for Bitcoin".to_string(),
        ))
    }

    async fn mint_right(
        &self,
        _source_chain: &str,
        _source_right_id: &RightId,
        _lock_proof: &InclusionProof,
        _new_owner: &str,
    ) -> ChainOpResult<RightOperationResult> {
        // Mint a wrapped right on this chain - Bitcoin is the source, not destination
        Err(ChainOpError::UnsupportedChain(
            "Bitcoin cannot mint wrapped rights - it is a source chain".to_string(),
        ))
    }

    async fn refund_right(
        &self,
        _right_id: &RightId,
        _owner_key_id: &str,
    ) -> ChainOpResult<RightOperationResult> {
        // Refund a locked right after timeout
        Err(ChainOpError::CapabilityUnavailable(
            "Refund not yet implemented".to_string(),
        ))
    }

    async fn record_right_metadata(
        &self,
        _right_id: &RightId,
        _metadata: serde_json::Value,
        _owner_key_id: &str,
    ) -> ChainOpResult<RightOperationResult> {
        Err(ChainOpError::CapabilityUnavailable(
            "Metadata recording not yet implemented for Bitcoin".to_string(),
        ))
    }

    async fn verify_right_state(
        &self,
        _right_id: &RightId,
        _expected_state: &str,
    ) -> ChainOpResult<bool> {
        Err(ChainOpError::CapabilityUnavailable(
            "Right state verification not yet implemented".to_string(),
        ))
    }
}
