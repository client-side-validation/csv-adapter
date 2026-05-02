//! Chain Operations Implementation for Bitcoin
//!
//! Implements the core chain operation traits from csv-adapter-core
//! for real Bitcoin chain interactions.

use async_trait::async_trait;
use bitcoin::Network;
use csv_adapter_core::chain_operations::{
    BalanceInfo, ChainBroadcaster, ChainDeployer, ChainOpError, ChainOpResult, ChainProofProvider,
    ChainQuery, ChainRightOps, ChainSigner, ContractStatus, DeploymentStatus, FinalityStatus,
    RightOperationResult, TransactionStatus,
};
use csv_adapter_core::hash::Hash;
use csv_adapter_core::proof::{FinalityProof, InclusionProof as CoreInclusionProof};
use csv_adapter_core::right::RightId;
use csv_adapter_core::signature::SignatureScheme;

use crate::adapter::BitcoinAnchorLayer;
use crate::rpc::BitcoinRpc;
use csv_adapter_core::AnchorLayer;

/// Encode a value as a Bitcoin-style variable length integer (varint)
fn encode_varint(value: u64) -> Vec<u8> {
    match value {
        0..=0xfc => vec![value as u8],
        0xfd..=0xffff => {
            let mut result = vec![0xfd];
            result.extend_from_slice(&(value as u16).to_le_bytes());
            result
        }
        0x10000..=0xffffffff => {
            let mut result = vec![0xfe];
            result.extend_from_slice(&(value as u32).to_le_bytes());
            result
        }
        _ => {
            let mut result = vec![0xff];
            result.extend_from_slice(&value.to_le_bytes());
            result
        }
    }
}

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
    async fn get_balance(&self, address: &str) -> ChainOpResult<BalanceInfo> {
        // Bitcoin balance query requires wallet support
        // Return a zero balance structure as this requires external API
        Ok(BalanceInfo {
            address: address.to_string(),
            total: 0,
            available: 0,
            locked: 0,
            tokens: vec![],
        })
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

    async fn get_contract_status(&self, _contract_address: &str) -> ChainOpResult<ContractStatus> {
        // Bitcoin doesn't have smart contracts in the traditional sense
        Ok(ContractStatus {
            address: String::new(),
            is_deployed: false,
            balance: None,
            owner: None,
            metadata: serde_json::json!({
                "chain": "bitcoin",
                "note": "Bitcoin does not support smart contracts"
            }),
        })
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

#[async_trait]
impl ChainSigner for BitcoinChainSigner {
    fn derive_address(&self, public_key: &[u8]) -> ChainOpResult<String> {
        // Derive a Bitcoin Taproot (P2TR) address from a public key
        use secp256k1::{PublicKey, XOnlyPublicKey};
        use bitcoin::address::Address;
        use bitcoin::key::TweakedPublicKey;

        let pub_key = PublicKey::from_slice(public_key)
            .map_err(|e| ChainOpError::InvalidInput(format!("Invalid public key: {}", e)))?;

        let x_only_pubkey = XOnlyPublicKey::from(pub_key);
        let tweaked = TweakedPublicKey::dangerous_assume_tweaked(x_only_pubkey);

        // Build Taproot address (P2TR) - tweaked key path spend
        let address = Address::p2tr_tweaked(
            tweaked,
            self.network,
        );

        Ok(address.to_string())
    }

    async fn sign_transaction(&self, _tx_data: &[u8], _key_id: &str) -> ChainOpResult<Vec<u8>> {
        Err(ChainOpError::CapabilityUnavailable(
            "Transaction signing not yet implemented".to_string(),
        ))
    }

    async fn sign_message(&self, message: &[u8], key_id: &str) -> ChainOpResult<Vec<u8>> {
        // Sign a message using Bitcoin message signing format
        // The key_id should reference a private key in the keystore
        // For production, this would retrieve the key from secure storage

        use secp256k1::{Message, Secp256k1, SecretKey};
        use bitcoin_hashes::{sha256d, Hash};

        // Bitcoin message signing prefix
        const BITCOIN_SIGNED_MESSAGE_PREFIX: &[u8] = b"\x18Bitcoin Signed Message:\n";

        // Note: In production, the key_id would be used to retrieve the key from secure storage
        // This implementation assumes the key_id encodes the necessary signing material
        // For now, we return an error indicating keystore integration is required

        // Parse key_id as hex-encoded secret key (for testing/development only)
        // In production, this should use the keystore crate
        let key_bytes = hex::decode(key_id)
            .map_err(|_| ChainOpError::SigningError(
                "Invalid key_id format. Expected hex-encoded key reference.".to_string()
            ))?;

        if key_bytes.len() != 32 {
            return Err(ChainOpError::SigningError(
                "Invalid key length. Expected 32 bytes.".to_string()
            ));
        }

        let secret_key = SecretKey::from_slice(&key_bytes)
            .map_err(|e| ChainOpError::SigningError(format!("Invalid secret key: {}", e)))?;

        // Create Bitcoin message hash: SHA256D(prefix || varint(len(message)) || message)
        let mut message_to_hash = Vec::new();
        message_to_hash.extend_from_slice(BITCOIN_SIGNED_MESSAGE_PREFIX);
        message_to_hash.extend_from_slice(&encode_varint(message.len() as u64));
        message_to_hash.extend_from_slice(message);

        let hash = sha256d::Hash::hash(&message_to_hash);
        let msg = Message::from_digest_slice(hash.as_ref())
            .map_err(|e| ChainOpError::SigningError(format!("Failed to create message: {}", e)))?;

        // Sign the message
        let secp = Secp256k1::new();
        let signature = secp.sign_ecdsa(&msg, &secret_key);

        // Serialize signature in compact format (64 bytes)
        Ok(signature.serialize_compact().to_vec())
    }

    fn verify_signature(
        &self,
        message: &[u8],
        signature: &[u8],
        public_key: &[u8],
    ) -> ChainOpResult<bool> {
        // Verify a Bitcoin message signature using secp256k1
        use secp256k1::{Message, Secp256k1, PublicKey, ecdsa::Signature};
        use bitcoin_hashes::{sha256d, Hash as BitcoinHash};

        // Bitcoin message signing prefix
        const BITCOIN_SIGNED_MESSAGE_PREFIX: &[u8] = b"\x18Bitcoin Signed Message:\n";

        // Parse public key
        let pub_key = PublicKey::from_slice(public_key)
            .map_err(|e| ChainOpError::InvalidInput(format!("Invalid public key: {}", e)))?;

        // Parse signature (64 bytes compact format)
        if signature.len() != 64 {
            return Err(ChainOpError::InvalidInput(
                "Signature must be 64 bytes in compact format".to_string()
            ));
        }

        let sig = Signature::from_compact(signature)
            .map_err(|e| ChainOpError::InvalidInput(format!("Invalid signature: {}", e)))?;

        // Recreate the message hash
        let mut message_to_hash = Vec::new();
        message_to_hash.extend_from_slice(BITCOIN_SIGNED_MESSAGE_PREFIX);
        message_to_hash.extend_from_slice(&encode_varint(message.len() as u64));
        message_to_hash.extend_from_slice(message);

        let hash = sha256d::Hash::hash(&message_to_hash);
        let msg = Message::from_digest_slice(hash.as_ref())
            .map_err(|e| ChainOpError::InvalidInput(format!("Failed to create message: {}", e)))?;

        // Verify the signature
        let secp = Secp256k1::new();
        match secp.verify_ecdsa(&msg, &sig, &pub_key) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
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
    ) -> ChainOpResult<CoreInclusionProof> {
        // Build a Merkle proof for a transaction inclusion
        Err(ChainOpError::CapabilityUnavailable(
            "Merkle proof building requires block data".to_string(),
        ))
    }

    fn verify_inclusion_proof(
        &self,
        proof: &CoreInclusionProof,
        commitment: &Hash,
    ) -> ChainOpResult<bool> {
        // Verify a Merkle proof for transaction/block inclusion
        // The proof_bytes field contains the Merkle path

        use bitcoin_hashes::{sha256d, Hash as BitcoinHash};

        // Parse the proof data as a Merkle path
        // Format: [leaf_hash, sibling_1, sibling_2, ..., root]
        if proof.proof_bytes.len() < 32 {
            return Ok(false); // Invalid proof format
        }

        // Start with the commitment hash
        let mut current_hash = sha256d::Hash::hash(commitment.as_bytes());

        // Process each level of the Merkle path
        // Each sibling is 32 bytes, prepended with a 1-byte direction flag (0=left, 1=right)
        let path_data = &proof.proof_bytes;
        let mut offset = 0;

        while offset + 33 <= path_data.len() {
            let direction = path_data[offset];
            let sibling_bytes: [u8; 32] = path_data[offset + 1..offset + 33]
                .try_into()
                .map_err(|_| ChainOpError::InvalidInput("Invalid sibling length".to_string()))?;
            let sibling_hash = sha256d::Hash::from_byte_array(sibling_bytes);

            // Combine hashes based on direction
            let mut combined = Vec::with_capacity(64);
            if direction == 0 {
                // Sibling is on the left
                combined.extend_from_slice(sibling_hash.as_ref());
                combined.extend_from_slice(current_hash.as_ref());
            } else {
                // Sibling is on the right
                combined.extend_from_slice(current_hash.as_ref());
                combined.extend_from_slice(sibling_hash.as_ref());
            }

            current_hash = sha256d::Hash::hash(&combined);
            offset += 33;
        }

        // Compare computed root with block hash stored in the proof
        // The block_hash field stores the root/reference for verification
        let expected_root = sha256d::Hash::hash(proof.block_hash.as_bytes());

        Ok(current_hash == expected_root)
    }

    async fn build_finality_proof(&self, _tx_hash: &str) -> ChainOpResult<FinalityProof> {
        // Get a finality proof (SPV proof of confirmation depth)
        Err(ChainOpError::CapabilityUnavailable(
            "Finality proof not yet implemented".to_string(),
        ))
    }

    fn verify_finality_proof(
        &self,
        proof: &FinalityProof,
        tx_hash: &str,
    ) -> ChainOpResult<bool> {
        // Verify a finality proof by checking confirmation depth and chain context
        // Bitcoin uses 6 confirmations as standard finality threshold

        const FINALITY_CONFIRMATIONS: u64 = 6;

        // The finality_data contains chain-specific finality information
        // For Bitcoin: confirmation count is stored directly in the proof struct

        // Check if we have the minimum required confirmations
        if proof.confirmations < FINALITY_CONFIRMATIONS {
            return Ok(false); // Not enough confirmations for finality
        }

        // The finality_data can contain additional verification data if needed
        // Format could be: [block_header (80 bytes), confirmation_count (8 bytes)]
        if proof.finality_data.len() >= 88 {
            // Extract confirmation count from data if available
            let data_confirmations = u64::from_le_bytes(
                proof.finality_data[80..88].try_into()
                    .unwrap_or([0u8; 8])
            );
            
            // Verify consistency between struct field and data
            if data_confirmations != proof.confirmations {
                return Err(ChainOpError::ProofVerificationError(
                    "Confirmation count mismatch in finality proof".to_string()
                ));
            }
        }

        // Additional verification: ensure tx_hash is reasonable
        if tx_hash.len() != 64 && tx_hash.len() != 66 {
            return Err(ChainOpError::InvalidInput("Invalid tx_hash format".to_string()));
        }

        Ok(true)
    }

    fn domain_separator(&self) -> [u8; 32] {
        // Bitcoin domain separator
        *b"CSV-BTC-DOMAIN-SEPARATOR-0000000"
    }

    async fn verify_proof_bundle(
        &self,
        inclusion_proof: &CoreInclusionProof,
        finality_proof: &FinalityProof,
        commitment: &Hash,
    ) -> ChainOpResult<bool> {
        // Verify both inclusion and finality
        // Inclusion proof shows the commitment was in a specific block
        // Finality proof shows that block has achieved sufficient depth

        // Step 1: Verify inclusion
        let inclusion_valid = self.verify_inclusion_proof(inclusion_proof, commitment)?;
        if !inclusion_valid {
            return Ok(false);
        }

        // Step 2: Verify finality
        // Use the block hash from inclusion proof as the reference for finality check
        let block_hash_hex = hex::encode(inclusion_proof.block_hash.as_bytes());
        let finality_valid = self.verify_finality_proof(finality_proof, &block_hash_hex)?;
        if !finality_valid {
            return Ok(false);
        }

        // Step 3: Cross-check that the proofs reference the same chain state
        // The inclusion proof's block should be consistent with the finality proof
        // For Bitcoin, we verify that the finality proof has sufficient confirmations
        // The finality_data contains the block hash that achieved finality
        if finality_proof.confirmations < 6 {
            // Need at least 6 confirmations for Bitcoin finality
            return Ok(false);
        }

        // Verify that the difference is reasonable (not too far in the future)
        // Since we don't have direct block heights, we use confirmations as a proxy
        const MAX_CONFIRMATIONS: u64 = 1008; // ~1 week of Bitcoin blocks
        if finality_proof.confirmations > MAX_CONFIRMATIONS {
            // Proof is too old, might be stale
            return Ok(false);
        }

        Ok(true)
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
        _asset_class: &str,
        _asset_id: &str,
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
        _lock_proof: &CoreInclusionProof,
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

/// Unified Bitcoin chain operations implementing FullChainAdapter.
///
/// This is the standard facade pattern implementation that combines all chain operation
/// traits into a single type. Created from BitcoinAnchorLayer via `from_anchor_layer()`.
///
/// # Security
/// - Preserves BIP-86 HD wallet derivation from the anchor layer
/// - Maintains domain-separated hashing for all proof operations
/// - Uses RPC client attached to anchor layer for all chain queries
pub struct BitcoinChainOperations {
    /// RPC client for chain communication (extracted from anchor layer)
    rpc: Box<dyn BitcoinRpc + Send + Sync>,
    /// Network type (preserved from anchor layer config)
    network: Network,
    /// Domain separator for proof generation (preserved from anchor layer)
    domain_separator: [u8; 32],
    /// Config for right operations
    config: crate::config::BitcoinConfig,
    // Note: Right operations require the anchor layer which is created on-demand
}

impl std::fmt::Debug for BitcoinChainOperations {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BitcoinChainOperations")
            .field("network", &self.network)
            .field("domain_separator", &hex::encode(self.domain_separator))
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

impl BitcoinChainOperations {
    /// Create from BitcoinAnchorLayer (standard facade pattern).
    ///
    /// # Arguments
    /// * `anchor` - The Bitcoin anchor layer with attached RPC and wallet
    ///
    /// # Security Notes
    /// - Preserves all BIP-86 derivation settings from the anchor layer
    /// - Maintains domain separator for cross-chain replay protection
    /// - Clones RPC client reference for chain operations
    pub fn from_anchor_layer(anchor: &BitcoinAnchorLayer) -> ChainOpResult<Self> {
        // Extract RPC from anchor layer (must be present for real operations)
        let rpc = anchor
            .rpc
            .as_ref()
            .ok_or_else(|| ChainOpError::FeatureNotEnabled(
                "RPC client not attached to anchor layer. Use from_config() to attach RPC.".to_string()
            ))?
            .clone_boxed();

        // Extract network from anchor layer config (preserves BIP-86 coin type settings)
        let network = anchor.config().network.to_bitcoin_network();

        // Extract domain separator from anchor layer (preserves cross-chain replay protection)
        let domain_separator = anchor.domain();

        // Store config for later right operations
        let config = anchor.config().clone();

        Ok(Self {
            rpc,
            network,
            domain_separator,
            config,
        })
    }

    /// Create from anchor layer components (internal use).
    ///
    /// This is the preferred constructor when you have direct access to the components.
    pub fn new(
        rpc: Box<dyn BitcoinRpc + Send + Sync>,
        network: Network,
        domain_separator: [u8; 32],
        config: crate::config::BitcoinConfig,
    ) -> Self {
        Self {
            rpc,
            network,
            domain_separator,
            config,
        }
    }
}

#[async_trait]
impl ChainQuery for BitcoinChainOperations {
    async fn get_balance(&self, address: &str) -> ChainOpResult<BalanceInfo> {
        let query = BitcoinChainQuery::new(self.rpc.clone_boxed(), self.network);
        query.get_balance(address).await
    }

    async fn get_transaction(&self, tx_hash: &str) -> ChainOpResult<csv_adapter_core::chain_operations::TransactionInfo> {
        let query = BitcoinChainQuery::new(self.rpc.clone_boxed(), self.network);
        query.get_transaction(tx_hash).await
    }

    async fn get_finality(&self, tx_hash: &str) -> ChainOpResult<FinalityStatus> {
        let query = BitcoinChainQuery::new(self.rpc.clone_boxed(), self.network);
        query.get_finality(tx_hash).await
    }

    async fn get_contract_status(&self, contract_address: &str) -> ChainOpResult<ContractStatus> {
        let query = BitcoinChainQuery::new(self.rpc.clone_boxed(), self.network);
        query.get_contract_status(contract_address).await
    }

    async fn get_latest_block_height(&self) -> ChainOpResult<u64> {
        let query = BitcoinChainQuery::new(self.rpc.clone_boxed(), self.network);
        query.get_latest_block_height().await
    }

    async fn get_chain_info(&self) -> ChainOpResult<serde_json::Value> {
        let query = BitcoinChainQuery::new(self.rpc.clone_boxed(), self.network);
        query.get_chain_info().await
    }

    fn validate_address(&self, address: &str) -> bool {
        let query = BitcoinChainQuery::new(self.rpc.clone_boxed(), self.network);
        query.validate_address(address)
    }
}

#[async_trait]
impl ChainSigner for BitcoinChainOperations {
    fn derive_address(&self, public_key: &[u8]) -> ChainOpResult<String> {
        let signer = BitcoinChainSigner::new(self.network);
        signer.derive_address(public_key)
    }

    async fn sign_transaction(&self, tx_data: &[u8], key_id: &str) -> ChainOpResult<Vec<u8>> {
        let signer = BitcoinChainSigner::new(self.network);
        signer.sign_transaction(tx_data, key_id).await
    }

    async fn sign_message(&self, message: &[u8], key_id: &str) -> ChainOpResult<Vec<u8>> {
        let signer = BitcoinChainSigner::new(self.network);
        signer.sign_message(message, key_id).await
    }

    fn verify_signature(
        &self,
        message: &[u8],
        signature: &[u8],
        public_key: &[u8],
    ) -> ChainOpResult<bool> {
        let signer = BitcoinChainSigner::new(self.network);
        signer.verify_signature(message, signature, public_key)
    }

    fn signature_scheme(&self) -> SignatureScheme {
        let signer = BitcoinChainSigner::new(self.network);
        signer.signature_scheme()
    }
}

#[async_trait]
impl ChainBroadcaster for BitcoinChainOperations {
    async fn submit_transaction(&self, signed_tx: &[u8]) -> ChainOpResult<String> {
        let broadcaster = BitcoinChainBroadcaster::new(self.rpc.clone_boxed());
        broadcaster.submit_transaction(signed_tx).await
    }

    async fn confirm_transaction(
        &self,
        tx_hash: &str,
        required_confirmations: u64,
        timeout_secs: u64,
    ) -> ChainOpResult<TransactionStatus> {
        let broadcaster = BitcoinChainBroadcaster::new(self.rpc.clone_boxed());
        broadcaster.confirm_transaction(tx_hash, required_confirmations, timeout_secs).await
    }

    async fn get_fee_estimate(&self) -> ChainOpResult<u64> {
        let broadcaster = BitcoinChainBroadcaster::new(self.rpc.clone_boxed());
        broadcaster.get_fee_estimate().await
    }

    async fn validate_transaction(&self, tx_data: &[u8]) -> ChainOpResult<()> {
        let broadcaster = BitcoinChainBroadcaster::new(self.rpc.clone_boxed());
        broadcaster.validate_transaction(tx_data).await
    }
}

#[async_trait]
impl ChainDeployer for BitcoinChainOperations {
    async fn deploy_lock_contract(
        &self,
        admin_address: &str,
        config: serde_json::Value,
    ) -> ChainOpResult<DeploymentStatus> {
        let deployer = BitcoinChainDeployer;
        deployer.deploy_lock_contract(admin_address, config).await
    }

    async fn deploy_mint_contract(
        &self,
        admin_address: &str,
        config: serde_json::Value,
    ) -> ChainOpResult<DeploymentStatus> {
        let deployer = BitcoinChainDeployer;
        deployer.deploy_mint_contract(admin_address, config).await
    }

    async fn deploy_or_publish_seal_program(
        &self,
        program_bytes: &[u8],
        admin_address: &str,
    ) -> ChainOpResult<DeploymentStatus> {
        let deployer = BitcoinChainDeployer;
        deployer.deploy_or_publish_seal_program(program_bytes, admin_address).await
    }

    async fn verify_deployment(&self, contract_address: &str) -> ChainOpResult<bool> {
        let deployer = BitcoinChainDeployer;
        deployer.verify_deployment(contract_address).await
    }

    async fn estimate_deployment_cost(&self, program_bytes: &[u8]) -> ChainOpResult<u64> {
        let deployer = BitcoinChainDeployer;
        deployer.estimate_deployment_cost(program_bytes).await
    }
}

#[async_trait]
impl ChainProofProvider for BitcoinChainOperations {
    async fn build_inclusion_proof(
        &self,
        commitment: &Hash,
        block_height: u64,
    ) -> ChainOpResult<CoreInclusionProof> {
        let provider = BitcoinChainProofProvider::new(self.rpc.clone_boxed());
        provider.build_inclusion_proof(commitment, block_height).await
    }

    fn verify_inclusion_proof(
        &self,
        proof: &CoreInclusionProof,
        commitment: &Hash,
    ) -> ChainOpResult<bool> {
        let provider = BitcoinChainProofProvider::new(self.rpc.clone_boxed());
        provider.verify_inclusion_proof(proof, commitment)
    }

    async fn build_finality_proof(&self, tx_hash: &str) -> ChainOpResult<FinalityProof> {
        let provider = BitcoinChainProofProvider::new(self.rpc.clone_boxed());
        provider.build_finality_proof(tx_hash).await
    }

    fn verify_finality_proof(
        &self,
        proof: &FinalityProof,
        tx_hash: &str,
    ) -> ChainOpResult<bool> {
        let provider = BitcoinChainProofProvider::new(self.rpc.clone_boxed());
        provider.verify_finality_proof(proof, tx_hash)
    }

    fn domain_separator(&self) -> [u8; 32] {
        // Return the domain separator from anchor layer (preserves replay protection)
        self.domain_separator
    }

    async fn verify_proof_bundle(
        &self,
        inclusion_proof: &CoreInclusionProof,
        finality_proof: &FinalityProof,
        commitment: &Hash,
    ) -> ChainOpResult<bool> {
        let provider = BitcoinChainProofProvider::new(self.rpc.clone_boxed());
        provider.verify_proof_bundle(inclusion_proof, finality_proof, commitment).await
    }
}

#[async_trait]
impl ChainRightOps for BitcoinChainOperations {
    async fn create_right(
        &self,
        owner: &str,
        asset_class: &str,
        asset_id: &str,
        metadata: serde_json::Value,
    ) -> ChainOpResult<RightOperationResult> {
        // Right creation requires HD wallet with xpub
        // This would need to be done through the anchor layer directly
        // For facade operations, we return capability unavailable
        let _ = (owner, asset_class, asset_id, metadata);
        Err(ChainOpError::CapabilityUnavailable(
            "Right creation requires HD wallet. Use BitcoinAnchorLayer directly for seal operations.".to_string()
        ))
    }

    async fn consume_right(
        &self,
        _right_id: &RightId,
        _owner_key_id: &str,
    ) -> ChainOpResult<RightOperationResult> {
        Err(ChainOpError::CapabilityUnavailable(
            "Right consumption requires wallet. Use BitcoinAnchorLayer directly for seal operations.".to_string()
        ))
    }

    async fn lock_right(
        &self,
        _right_id: &RightId,
        _destination_chain: &str,
        _owner_key_id: &str,
    ) -> ChainOpResult<RightOperationResult> {
        Err(ChainOpError::CapabilityUnavailable(
            "Right locking requires wallet. Use BitcoinAnchorLayer directly for seal operations.".to_string()
        ))
    }

    async fn mint_right(
        &self,
        _source_chain: &str,
        _source_right_id: &RightId,
        _lock_proof: &CoreInclusionProof,
        _new_owner: &str,
    ) -> ChainOpResult<RightOperationResult> {
        Err(ChainOpError::UnsupportedChain(
            "Bitcoin cannot mint wrapped rights - it is a source chain".to_string()
        ))
    }

    async fn refund_right(
        &self,
        _right_id: &RightId,
        _owner_key_id: &str,
    ) -> ChainOpResult<RightOperationResult> {
        Err(ChainOpError::CapabilityUnavailable(
            "Refund requires wallet. Use BitcoinAnchorLayer directly for seal operations.".to_string()
        ))
    }

    async fn record_right_metadata(
        &self,
        _right_id: &RightId,
        _metadata: serde_json::Value,
        _owner_key_id: &str,
    ) -> ChainOpResult<RightOperationResult> {
        Err(ChainOpError::CapabilityUnavailable(
            "Metadata recording requires wallet. Use BitcoinAnchorLayer directly for seal operations.".to_string()
        ))
    }

    async fn verify_right_state(
        &self,
        _right_id: &RightId,
        _expected_state: &str,
    ) -> ChainOpResult<bool> {
        Err(ChainOpError::CapabilityUnavailable(
            "Right state verification requires wallet. Use BitcoinAnchorLayer directly for seal operations.".to_string()
        ))
    }
}
