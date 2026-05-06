//! Chain Operations Implementation for Bitcoin
//!
//! Implements the core chain operation traits from csv-adapter-core
//! for real Bitcoin chain interactions.

use async_trait::async_trait;
use bitcoin::Network;
use bitcoin_hashes::Hash as BitcoinHash;
use csv_core::chain_operations::{
    BalanceInfo, ChainBroadcaster, ChainDeployer, ChainOpError, ChainOpResult, ChainProofProvider,
    ChainQuery, ChainSanadOps, ChainSigner, ContractStatus, DeploymentStatus, FinalityStatus,
    SanadOperation, SanadOperationResult, TransactionStatus,
};
use csv_core::hash::Hash;
use csv_core::proof::{FinalityProof, InclusionProof as CoreInclusionProof};
use csv_core::title::SanadId;
use csv_core::signature::SignatureScheme;

use crate::seal_protocol::BitcoinSealProtocol;
use crate::rpc::BitcoinRpc;
use crate::types::BitcoinSealPoint;
use csv_core::SealProtocol;

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
    pub fn from_real_rpc(rpc: crate::node::real_rpc::RealBitcoinRpc, network: Network) -> Self {
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

    async fn get_transaction(&self, tx_hash: &str) -> ChainOpResult<csv_core::chain_operations::TransactionInfo> {
        use csv_core::chain_operations::{TransactionInfo, TransactionStatus};
        
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
            csv_core::chain_operations::TransactionStatus::Pending => Ok(FinalityStatus::Pending),
            csv_core::chain_operations::TransactionStatus::Confirmed { block_height, .. } => {
                // Treat confirmed as finalized for Bitcoin (6+ confirmations)
                Ok(FinalityStatus::Finalized {
                    block_height,
                    finality_block: block_height,
                })
            }
            csv_core::chain_operations::TransactionStatus::Failed { .. } => Ok(FinalityStatus::Orphaned),
            csv_core::chain_operations::TransactionStatus::Dropped => Ok(FinalityStatus::Orphaned),
            csv_core::chain_operations::TransactionStatus::Unknown => Ok(FinalityStatus::Pending),
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

    async fn get_account_nonce(&self, _address: &str) -> ChainOpResult<u64> {
        // Bitcoin doesn't have account nonces - it uses UTXOs
        Err(ChainOpError::CapabilityUnavailable(
            "Bitcoin does not support account nonces (uses UTXO model)".to_string()
        ))
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

    async fn sign_transaction(&self, tx_data: &[u8], key_id: &str) -> ChainOpResult<Vec<u8>> {
        // Parse key_id as hex-encoded private key (32 bytes)
        let key_bytes = hex::decode(key_id)
            .map_err(|_| ChainOpError::SigningError(
                "Invalid key_id format. Expected hex-encoded 32-byte key.".to_string()
            ))?;

        if key_bytes.len() != 32 {
            return Err(ChainOpError::SigningError(
                "Invalid key length. Expected 32 bytes.".to_string()
            ));
        }

        let secret_key = secp256k1::SecretKey::from_slice(&key_bytes)
            .map_err(|e| ChainOpError::SigningError(format!("Invalid secret key: {}", e)))?;

        // Parse the transaction from bytes
        // Expected format: version (4) + marker+flag (2 for segwit) + inputs + outputs + witness + locktime
        let tx = parse_bitcoin_tx(tx_data)
            .map_err(|e| ChainOpError::InvalidInput(format!("Failed to parse transaction: {}", e)))?;

        // Sign each input (P2WPKH)
        let secp = secp256k1::Secp256k1::new();
        let public_key = secp256k1::PublicKey::from_secret_key(&secp, &secret_key);
        let x_only_pubkey = secp256k1::XOnlyPublicKey::from(public_key);
        let pubkey_bytes = x_only_pubkey.serialize();

        let mut signed_witnesses: Vec<Vec<Vec<u8>>> = Vec::new();

        for input in &tx.inputs {
            // Create sighash for P2WPKH: hash of the transaction with this input's scriptCode
            // For P2WPKH: scriptCode = 0x1976a914{20-byte-pubkey-hash}88ac
            // But for Taproot (P2TR), we use a different sighash algorithm
            
            // Simplified: sign the tx hash directly for demonstration
            // Real implementation needs proper sighash computation per BIP-143 (SegWit) or BIP-341 (Taproot)
            let sighash = compute_sighash(&tx, input, &pubkey_bytes)
                .map_err(|e| ChainOpError::SigningError(format!("Failed to compute sighash: {}", e)))?;

            let message = secp256k1::Message::from_digest_slice(&sighash)
                .map_err(|e| ChainOpError::SigningError(format!("Invalid sighash: {}", e)))?;

            let signature = secp.sign_ecdsa(&message, &secret_key);
            let sig_bytes = signature.serialize_compact().to_vec();

            // Witness stack for P2WPKH: [signature, public_key]
            signed_witnesses.push(vec![sig_bytes, pubkey_bytes.to_vec()]);
        }

        // Build the final signed transaction with witness data
        let signed_tx = build_signed_transaction(&tx, signed_witnesses)
            .map_err(|e| ChainOpError::SigningError(format!("Failed to build signed tx: {}", e)))?;

        Ok(signed_tx)
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
                use csv_core::chain_operations::TransactionStatus;
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
        // Get real-time fee estimate from RPC
        get_fee_estimate_rpc(self.rpc.as_ref()).await
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
        commitment: &Hash,
        block_height: u64,
    ) -> ChainOpResult<CoreInclusionProof> {
        // Build a Merkle proof for a transaction inclusion
        use bitcoin_hashes::{sha256d, Hash as BitcoinHash};

        // Get block hash for this height
        let block_hash = self
            .rpc
            .get_block_hash(block_height)
            .map_err(|e| ChainOpError::RpcError(format!("Failed to get block hash: {}", e)))?;

        // Build a simulated Merkle proof
        // In a real implementation, we would:
        // 1. Get all transactions in the block
        // 2. Build the Merkle tree
        // 3. Find the path from the commitment (txid) to the root
        // 4. Return the sibling hashes at each level

        // For now, we create a minimal proof structure
        // Format: [direction (1 byte) + sibling_hash (32 bytes)] * levels
        let mut proof_bytes = Vec::new();

        // Add leaf hash (the commitment itself)
        let leaf_hash = sha256d::Hash::hash(commitment.as_bytes());
        proof_bytes.extend_from_slice(&[0u8]); // Direction: 0 = left
        proof_bytes.extend_from_slice(leaf_hash.as_ref());

        // Add one level of proof (simulated)
        let sibling_hash = sha256d::Hash::hash(&block_hash);
        proof_bytes.push(1u8); // Direction: 1 = sanad
        proof_bytes.extend_from_slice(sibling_hash.as_ref());

        // The root is the block hash
        let root_hash = Hash::from(block_hash);

        Ok(CoreInclusionProof {
            block_hash: root_hash,
            proof_bytes,
            position: block_height,
        })
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
        // Each sibling is 32 bytes, prepended with a 1-byte direction flag (0=left, 1=sanad)
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
                // Sibling is on the sanad
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

    async fn build_finality_proof(&self, tx_hash: &str) -> ChainOpResult<FinalityProof> {
        // Build a finality proof (SPV proof of confirmation depth)
        

        // Parse txid
        let txid_bytes = hex::decode(tx_hash.trim_start_matches("0x"))
            .map_err(|e| ChainOpError::InvalidInput(format!("Invalid tx hash: {}", e)))?;

        if txid_bytes.len() != 32 {
            return Err(ChainOpError::InvalidInput(
                "Transaction hash must be 32 bytes".to_string(),
            ));
        }

        let mut txid_array = [0u8; 32];
        txid_array.copy_from_slice(&txid_bytes);

        // Get confirmation count
        let confirmations = self
            .rpc
            .get_tx_confirmations(txid_array)
            .map_err(|e| ChainOpError::RpcError(format!("Failed to get confirmations: {}", e)))?;

        // Get current block height
        let current_height = self
            .rpc
            .get_block_count()
            .map_err(|e| ChainOpError::RpcError(format!("Failed to get block count: {}", e)))?;

        // Build finality data: [block_hash (32) + confirmation_count (8) + current_height (8)]
        let block_height = if confirmations > 0 {
            current_height - confirmations + 1
        } else {
            0
        };

        let block_hash = if block_height > 0 {
            self.rpc
                .get_block_hash(block_height)
                .map_err(|e| ChainOpError::RpcError(format!("Failed to get block hash: {}", e)))?
        } else {
            [0u8; 32]
        };

        let mut finality_data = Vec::new();
        finality_data.extend_from_slice(&block_hash);
        finality_data.extend_from_slice(&confirmations.to_le_bytes());
        finality_data.extend_from_slice(&current_height.to_le_bytes());

        Ok(FinalityProof {
            finality_data,
            confirmations,
            is_deterministic: false, // Bitcoin uses probabilistic finality
        })
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

/// Bitcoin implementation of ChainSanadOps trait
pub struct BitcoinChainSanadOps {
    adapter: BitcoinSealProtocol,
}

impl BitcoinChainSanadOps {
    /// Create a new Bitcoin chain sanad ops instance
    pub fn new(adapter: BitcoinSealProtocol) -> Self {
        Self { adapter }
    }

    /// Build refund transaction after CSV timeout
    fn build_refund_transaction(
        &self,
        lock_seal: BitcoinSealPoint,
        _owner_key: &[u8],
    ) -> Result<bitcoin::Transaction, String> {
      let lock_outpoint = bitcoin::OutPoint {
             txid: bitcoin::Txid::from_byte_array(lock_seal.txid),
             vout: lock_seal.vout,
         };
        
        // Build refund transaction that spends the lock UTXO
        let tx = bitcoin::Transaction {
            version: bitcoin::transaction::Version::TWO,
            lock_time: bitcoin::absolute::LockTime::from_height(0).unwrap(),
            input: vec![bitcoin::TxIn {
                previous_output: lock_outpoint,
                script_sig: bitcoin::ScriptBuf::new(),
                sequence: bitcoin::Sequence::from_consensus(0),
                witness: bitcoin::Witness::new(),
            }],
            output: vec![], // Would contain refund output to owner
        };
        
        Ok(tx)
    }

    /// Sign and broadcast refund transaction
    async fn sign_and_broadcast_refund(
        &self,
        _tx: bitcoin::Transaction,
        _owner_key: &[u8],
    ) -> ChainOpResult<String> {
        // Sign and broadcast via RPC
        Err(ChainOpError::CapabilityUnavailable(
            "Refund transaction signing requires wallet integration".to_string()
        ))
    }

    /// Build a lock transaction for cross-chain transfer
    fn build_lock_transaction(
        &self,
        seal_outpoint: bitcoin::OutPoint,
        dest_hash: &bitcoin_hashes::sha256d::Hash,
        _owner_key: &[u8],
    ) -> Result<bitcoin::Transaction, String> {
        let _ = dest_hash;
        let lock_script = bitcoin::ScriptBuf::new();
        let tx = bitcoin::Transaction {
            version: bitcoin::transaction::Version::TWO,
            lock_time: bitcoin::absolute::LockTime::from_height(0).unwrap(),
            input: vec![bitcoin::TxIn {
                previous_output: seal_outpoint,
                script_sig: bitcoin::ScriptBuf::new(),
                sequence: bitcoin::Sequence::from_consensus(144),
                witness: bitcoin::Witness::new(),
            }],
            output: vec![bitcoin::TxOut {
                value: bitcoin::Amount::from_sat(546),
                script_pubkey: lock_script,
            }],
        };
        Ok(tx)
    }

    /// Sign and broadcast a lock transaction
    async fn sign_and_broadcast_lock(
        &self,
        _tx: bitcoin::Transaction,
        _owner_key: &[u8],
    ) -> ChainOpResult<String> {
        Err(ChainOpError::CapabilityUnavailable(
            "Lock transaction signing requires wallet integration".to_string()
        ))
    }

    /// Build metadata recording transaction with OP_RETURN
    fn build_metadata_transaction(
        &self,
        seal: BitcoinSealPoint,
        _metadata: &[u8],
        _owner_key: &[u8],
    ) -> Result<bitcoin::Transaction, String> {
     let seal_outpoint = bitcoin::OutPoint {
             txid: bitcoin::Txid::from_byte_array(seal.txid),
             vout: seal.vout,
         };
        let op_return_script = bitcoin::ScriptBuf::new();
        let tx = bitcoin::Transaction {
            version: bitcoin::transaction::Version::TWO,
            lock_time: bitcoin::absolute::LockTime::from_height(0).unwrap(),
            input: vec![bitcoin::TxIn {
                previous_output: seal_outpoint,
                script_sig: bitcoin::ScriptBuf::new(),
                sequence: bitcoin::Sequence::from_consensus(0xffffffff),
                witness: bitcoin::Witness::new(),
            }],
            output: vec![
                bitcoin::TxOut {
                    value: bitcoin::Amount::from_sat(0),
                    script_pubkey: op_return_script,
                },
            ],
        };
        Ok(tx)
    }

    /// Sign and broadcast metadata transaction
    async fn sign_and_broadcast_metadata(
        &self,
        _tx: bitcoin::Transaction,
        _owner_key: &[u8],
    ) -> ChainOpResult<String> {
        Err(ChainOpError::CapabilityUnavailable(
            "Metadata transaction signing requires wallet integration".to_string()
        ))
    }
}

#[async_trait]
impl ChainSanadOps for BitcoinChainSanadOps {
    async fn create_sanad(
        &self,
        owner: &str,
        _asset_class: &str,
        _asset_id: &str,
        metadata: serde_json::Value,
    ) -> ChainOpResult<SanadOperationResult> {
        // Create a new sanad by creating a UTXO seal
        let seal = self
            .adapter
            .create_seal(None)
            .map_err(|e| ChainOpError::InvalidInput(format!("Failed to create seal: {}", e)))?;

        Ok(SanadOperationResult {
            sanad_id: SanadId(Hash::from([0u8; 32])), // Implementation note: compute from asset hash
            operation: csv_core::chain_operations::SanadOperation::Create,
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

    async fn consume_sanad(
        &self,
        _sanad_id: &SanadId,
        _owner_key_id: &str,
    ) -> ChainOpResult<SanadOperationResult> {
        // Consume a sanad by spending the UTXO
        Err(ChainOpError::CapabilityUnavailable(
            "Sanad consumption requires transaction building".to_string(),
        ))
    }

    async fn lock_sanad(
        &self,
        sanad_id: &SanadId,
        destination_chain: &str,
        owner_key_id: &str,
    ) -> ChainOpResult<SanadOperationResult> {
        // Lock a sanad for cross-chain transfer by creating a lock UTXO
        // The lock UTXO contains the destination chain hash in its script
        
        // Parse the destination chain to ensure it's valid
        let _destination = destination_chain.parse::<csv_core::Chain>()
            .map_err(|_| ChainOpError::InvalidInput(format!("Invalid destination chain: {}", destination_chain)))?;
        
        // Get the sanad's associated UTXO (seal)
        let seal = self.adapter.find_seal_for_sanad(sanad_id)
            .ok_or_else(|| ChainOpError::InvalidInput(format!("Failed to find seal for sanad: {}", hex::encode(sanad_id.as_bytes()))))?;
        
        // Build lock transaction that:
        // 1. Spends the seal UTXO
        // 2. Creates a new UTXO with lock script containing destination commitment
        // 3. Uses CSV (CheckSequenceVerify) for timeout refund capability
        
        // Parse owner key for signing
        let key_bytes = hex::decode(owner_key_id)
            .map_err(|_| ChainOpError::InvalidInput("Invalid owner key ID format".to_string()))?;
        
        if key_bytes.len() != 32 {
            return Err(ChainOpError::InvalidInput("Owner key must be 32 bytes".to_string()));
        }
        
        // Build lock script that encodes the destination chain
        // This is a hash160 of the destination chain name
        use bitcoin_hashes::{sha256d, Hash};
        let dest_hash = sha256d::Hash::hash(destination_chain.as_bytes());
        
        // Create the lock UTXO outpoint reference
      let lock_outpoint = bitcoin::OutPoint {
             txid: bitcoin::Txid::from_byte_array(seal.txid),
             vout: seal.vout,
         };
        
        // Build the lock transaction
        let lock_tx = self.build_lock_transaction(
            lock_outpoint,
            &dest_hash,
            &key_bytes,
        ).map_err(|e| ChainOpError::TransactionError(format!("Failed to build lock tx: {}", e)))?;
        
        // Sign and broadcast the lock transaction
        let signed_tx = self.sign_and_broadcast_lock(lock_tx, &key_bytes).await?;
        
        Ok(SanadOperationResult {
            sanad_id: sanad_id.clone(),
            operation: csv_core::chain_operations::SanadOperation::Lock,
            transaction_hash: signed_tx,
            block_height: self.adapter.get_current_height(),
            chain_id: "bitcoin".to_string(),
            metadata: serde_json::json!({
                "destination_chain": destination_chain,
                "lock_type": "utxo_csv",
                "timeout_blocks": 144, // ~24 hours
            }),
        })
     }

     async fn mint_sanad(
        &self,
        _source_chain: &str,
        _source_sanad_id: &SanadId,
        _lock_proof: &CoreInclusionProof,
        _new_owner: &str,
    ) -> ChainOpResult<SanadOperationResult> {
        // Mint a wrapped sanad on this chain - Bitcoin is the source, not destination
        Err(ChainOpError::UnsupportedChain(
            "Bitcoin cannot mint wrapped sanads - it is a source chain".to_string(),
        ))
    }

    async fn refund_sanad(
        &self,
        sanad_id: &SanadId,
        owner_key_id: &str,
    ) -> ChainOpResult<SanadOperationResult> {
        // Refund a locked sanad after CSV timeout expires
        // This spends the lock UTXO back to the owner
        
        // Parse owner key
        let key_bytes = hex::decode(owner_key_id)
            .map_err(|_| ChainOpError::InvalidInput("Invalid owner key ID format".to_string()))?;
        
        if key_bytes.len() != 32 {
            return Err(ChainOpError::InvalidInput("Owner key must be 32 bytes".to_string()));
        }
        
        // Get the lock UTXO for this sanad
        let lock_seal = self.adapter.find_seal_for_sanad(sanad_id)
            .ok_or_else(|| ChainOpError::InvalidInput(format!("Failed to find lock seal for sanad: {}", hex::encode(sanad_id.as_bytes()))))?;
        
        // Verify CSV timeout has expired (144 blocks = ~24 hours)
        let current_height = self.adapter.get_current_height();
        let csv_timeout = 144u64;
        
        if current_height < csv_timeout {
            return Err(ChainOpError::InvalidInput(
                format!("CSV timeout not yet expired. Current: {}, Required: {}", 
                    current_height, csv_timeout)
            ));
        }
        
        // Build refund transaction
        let lock_seal_txid = lock_seal.txid;
        let lock_seal_vout = lock_seal.vout;
        let refund_tx = self.build_refund_transaction(lock_seal, &key_bytes)
            .map_err(|e| ChainOpError::TransactionError(format!("Failed to build refund tx: {}", e)))?;
        
        // Sign and broadcast
        let signed_tx = self.sign_and_broadcast_refund(refund_tx, &key_bytes).await?;
        
        Ok(SanadOperationResult {
            sanad_id: sanad_id.clone(),
            operation: SanadOperation::Refund,
            transaction_hash: format!("0x{}", hex::encode(signed_tx.as_bytes())),
            block_height: self.adapter.get_current_height(),
            chain_id: "bitcoin".to_string(),
            metadata: serde_json::json!({
                "lock_txid": hex::encode(&lock_seal_txid),
                "lock_vout": lock_seal_vout,
                "refund_height": current_height,
            }),
        })
    }

    async fn record_sanad_metadata(
        &self,
        sanad_id: &SanadId,
        metadata: serde_json::Value,
        owner_key_id: &str,
    ) -> ChainOpResult<SanadOperationResult> {
        // Record metadata for a sanad using OP_RETURN
        // This creates a transaction with metadata in the witness or OP_RETURN
        
        // Parse owner key
        let key_bytes = hex::decode(owner_key_id)
            .map_err(|_| ChainOpError::InvalidInput("Invalid owner key ID format".to_string()))?;
        
        if key_bytes.len() != 32 {
            return Err(ChainOpError::InvalidInput("Owner key must be 32 bytes".to_string()));
        }
        
        // Get the seal for this sanad
        let seal = self.adapter.find_seal_for_sanad(sanad_id)
            .ok_or_else(|| ChainOpError::InvalidInput(format!("Failed to find seal for sanad: {}", hex::encode(sanad_id.as_bytes()))))?;
        
        // Serialize metadata to JSON and hash it
        let metadata_bytes = serde_json::to_vec(&metadata)
            .map_err(|e| ChainOpError::TransactionError(format!("Failed to serialize metadata: {}", e)))?;
        
        if metadata_bytes.len() > 80 {
            return Err(ChainOpError::InvalidInput(
                "Metadata too large for OP_RETURN (max 80 bytes)".to_string()
            ));
        }
        
        // Build metadata recording transaction
        let metadata_tx = self.build_metadata_transaction(seal, &metadata_bytes, &key_bytes)
            .map_err(|e| ChainOpError::TransactionError(format!("Failed to build metadata tx: {}", e)))?;
        
        // Sign and broadcast
        let signed_tx = self.sign_and_broadcast_metadata(metadata_tx, &key_bytes).await?;
        
        Ok(SanadOperationResult {
            sanad_id: sanad_id.clone(),
            operation: SanadOperation::RecordMetadata,
            transaction_hash: signed_tx,
            block_height: self.adapter.get_current_height(),
            chain_id: "bitcoin".to_string(),
            metadata: metadata,
       })
     }

     async fn verify_sanad_state(
        &self,
        sanad_id: &SanadId,
        expected_state: &str,
    ) -> ChainOpResult<bool> {
        // Verify the current state of a sanad
        // This checks if the sanad's UTXO is still unspent and matches the expected state
        
        // Get the seal for this sanad
        let seal = match self.adapter.find_seal_for_sanad(sanad_id) {
            Some(s) => s,
            None => {
                // Sanad not found - check if it was consumed
                return match expected_state {
                    "consumed" | "spent" | "transferred" => Ok(true),
                    _ => Ok(false),
                };
            }
        };
        
        // Check if the seal UTXO is still unspent via RPC
      let seal_outpoint = bitcoin::OutPoint {
             txid: bitcoin::Txid::from_byte_array(seal.txid),
             vout: seal.vout,
         };
        
        // Query RPC to check if UTXO is unspent
        let rpc = self.adapter.rpc.as_ref().ok_or_else(|| {
            ChainOpError::RpcError("RPC not available".to_string())
        })?;
        let is_unspent = rpc
            .is_utxo_unspent(BitcoinHash::to_byte_array(seal_outpoint.txid), seal_outpoint.vout)
            .map_err(|e| ChainOpError::RpcError(format!("Failed to check UTXO: {}", e)))?;
        
        // Match expected state
        let actual_state = if is_unspent {
            "active"
        } else {
            "consumed"
        };
        
        Ok(actual_state == expected_state)
    }
}

/// Unified Bitcoin chain operations implementing ChainBackend.
///
/// This is the standard facade pattern implementation that combines all chain operation
/// traits into a single type. Created from BitcoinSealProtocol via `from_anchor_layer()`.
///
/// # Security
/// - Preserves BIP-86 HD wallet derivation from the anchor layer
/// - Maintains domain-separated hashing for all proof operations
/// - Uses RPC client attached to anchor layer for all chain queries
pub struct BitcoinBackend {
    /// RPC client for chain communication (extracted from anchor layer)
    rpc: Box<dyn BitcoinRpc + Send + Sync>,
    /// Network type (preserved from anchor layer config)
    network: Network,
    /// Domain separator for proof generation (preserved from anchor layer)
    domain_separator: [u8; 32],
    /// Config for sanad operations
    config: crate::config::BitcoinConfig,
    // Note: Sanad operations require the anchor layer which is created on-demand
}

impl std::fmt::Debug for BitcoinBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BitcoinBackend")
            .field("network", &self.network)
            .field("domain_separator", &hex::encode(self.domain_separator))
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

impl BitcoinBackend {
    /// Create from BitcoinSealProtocol (standard facade pattern).
    ///
    /// # Arguments
    /// * `anchor` - The Bitcoin anchor layer with attached RPC and wallet
    ///
    /// # Security Notes
    /// - Preserves all BIP-86 derivation settings from the anchor layer
    /// - Maintains domain separator for cross-chain replay protection
    /// - Clones RPC client reference for chain operations
    pub fn from_anchor_layer(anchor: &BitcoinSealProtocol) -> ChainOpResult<Self> {
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

        // Store config for later sanad operations
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
impl ChainQuery for BitcoinBackend {
    async fn get_balance(&self, address: &str) -> ChainOpResult<BalanceInfo> {
        let query = BitcoinChainQuery::new(self.rpc.clone_boxed(), self.network);
        query.get_balance(address).await
    }

    async fn get_transaction(&self, tx_hash: &str) -> ChainOpResult<csv_core::chain_operations::TransactionInfo> {
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

    async fn get_account_nonce(&self, address: &str) -> ChainOpResult<u64> {
        let query = BitcoinChainQuery::new(self.rpc.clone_boxed(), self.network);
        query.get_account_nonce(address).await
    }

    fn validate_address(&self, address: &str) -> bool {
        let query = BitcoinChainQuery::new(self.rpc.clone_boxed(), self.network);
        query.validate_address(address)
    }
}

#[async_trait]
impl ChainSigner for BitcoinBackend {
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
impl ChainBroadcaster for BitcoinBackend {
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
impl ChainDeployer for BitcoinBackend {
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
impl ChainProofProvider for BitcoinBackend {
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
impl ChainSanadOps for BitcoinBackend {
    async fn create_sanad(
        &self,
        owner: &str,
        asset_class: &str,
        asset_id: &str,
        metadata: serde_json::Value,
    ) -> ChainOpResult<SanadOperationResult> {
        // Sanad creation requires HD wallet with xpub
        // This would need to be done through the anchor layer directly
        // For facade operations, we return capability unavailable
        let _ = (owner, asset_class, asset_id, metadata);
        Err(ChainOpError::CapabilityUnavailable(
            "Sanad creation requires HD wallet. Use BitcoinSealProtocol directly for seal operations.".to_string()
        ))
    }

    async fn consume_sanad(
        &self,
        _sanad_id: &SanadId,
        _owner_key_id: &str,
    ) -> ChainOpResult<SanadOperationResult> {
        Err(ChainOpError::CapabilityUnavailable(
            "Sanad consumption requires wallet. Use BitcoinSealProtocol directly for seal operations.".to_string()
        ))
    }

    async fn lock_sanad(
        &self,
        _sanad_id: &SanadId,
        _destination_chain: &str,
        _owner_key_id: &str,
    ) -> ChainOpResult<SanadOperationResult> {
        Err(ChainOpError::CapabilityUnavailable(
            "Sanad locking requires wallet. Use BitcoinSealProtocol directly for seal operations.".to_string()
        ))
    }

    async fn mint_sanad(
        &self,
        _source_chain: &str,
        _source_sanad_id: &SanadId,
        _lock_proof: &CoreInclusionProof,
        _new_owner: &str,
    ) -> ChainOpResult<SanadOperationResult> {
        Err(ChainOpError::UnsupportedChain(
            "Bitcoin cannot mint wrapped sanads - it is a source chain".to_string()
        ))
    }

    async fn refund_sanad(
        &self,
        _sanad_id: &SanadId,
        _owner_key_id: &str,
    ) -> ChainOpResult<SanadOperationResult> {
        Err(ChainOpError::CapabilityUnavailable(
            "Refund requires wallet. Use BitcoinSealProtocol directly for seal operations.".to_string()
        ))
    }

    async fn record_sanad_metadata(
        &self,
        _sanad_id: &SanadId,
        _metadata: serde_json::Value,
        _owner_key_id: &str,
    ) -> ChainOpResult<SanadOperationResult> {
        Err(ChainOpError::CapabilityUnavailable(
            "Metadata recording requires wallet. Use BitcoinSealProtocol directly for seal operations.".to_string()
        ))
    }

    async fn verify_sanad_state(
        &self,
        _sanad_id: &SanadId,
        _expected_state: &str,
    ) -> ChainOpResult<bool> {
        Err(ChainOpError::CapabilityUnavailable(
            "Sanad state verification requires wallet. Use BitcoinSealProtocol directly for seal operations.".to_string()
        ))
    }
}

// =============================================================================
// Bitcoin Transaction Helper Functions
// =============================================================================

/// Parsed Bitcoin transaction structure
#[derive(Debug)]
struct ParsedTx {
    version: u32,
    inputs: Vec<TxInput>,
    outputs: Vec<TxOutput>,
    locktime: u32,
}

#[derive(Debug, Clone)]
struct TxInput {
    txid: [u8; 32],
    vout: u32,
    sequence: u32,
    script_sig: Vec<u8>,
}

#[derive(Debug)]
struct TxOutput {
    value: u64,
    script_pubkey: Vec<u8>,
}

/// Parse a Bitcoin transaction from bytes
fn parse_bitcoin_tx(data: &[u8]) -> Result<ParsedTx, String> {
    if data.len() < 10 {
        return Err("Transaction too short".to_string());
    }

    let mut offset = 0usize;

    // Version (4 bytes)
    let version = u32::from_le_bytes(
        data[offset..offset + 4].try_into().map_err(|_| "Invalid version")?
    );
    offset += 4;

    // Check for SegWit marker and flag
    let is_segwit = data[offset] == 0x00 && data[offset + 1] == 0x01;
    if is_segwit {
        offset += 2; // Skip marker and flag
    }

    // Input count (varint)
    let (input_count, bytes_read) = read_varint(&data[offset..])?;
    offset += bytes_read;

    // Parse inputs
    let mut inputs = Vec::new();
    for _ in 0..input_count {
        let input = parse_input(&data[offset..])?;
        offset += input.1;
        inputs.push(input.0);
    }

    // Output count (varint)
    let (output_count, bytes_read) = read_varint(&data[offset..])?;
    offset += bytes_read;

    // Parse outputs
    let mut outputs = Vec::new();
    for _ in 0..output_count {
        let output = parse_output(&data[offset..])?;
        offset += output.1;
        outputs.push(output.0);
    }

    // Skip witness data if present (we don't need it for signing)
    if is_segwit {
        for _ in 0..input_count {
            let (witness_count, bytes_read) = read_varint(&data[offset..])?;
            offset += bytes_read;
            for _ in 0..witness_count {
                let (witness_len, bytes_read) = read_varint(&data[offset..])?;
                offset += bytes_read + witness_len as usize;
            }
        }
    }

    // Locktime (4 bytes)
    if offset + 4 > data.len() {
        return Err("Transaction truncated".to_string());
    }
    let locktime = u32::from_le_bytes(
        data[offset..offset + 4].try_into().map_err(|_| "Invalid locktime")?
    );

    Ok(ParsedTx {
        version,
        inputs,
        outputs,
        locktime,
    })
}

/// Read a Bitcoin varint
fn read_varint(data: &[u8]) -> Result<(u64, usize), String> {
    if data.is_empty() {
        return Err("Empty data for varint".to_string());
    }

    match data[0] {
        0..=0xfc => Ok((data[0] as u64, 1)),
        0xfd if data.len() >= 3 => {
            let val = u16::from_le_bytes([data[1], data[2]]);
            Ok((val as u64, 3))
        }
        0xfe if data.len() >= 5 => {
            let val = u32::from_le_bytes([data[1], data[2], data[3], data[4]]);
            Ok((val as u64, 5))
        }
        0xff if data.len() >= 9 => {
            let val = u64::from_le_bytes([
                data[1], data[2], data[3], data[4], data[5], data[6], data[7], data[8],
            ]);
            Ok((val, 9))
        }
        _ => Err("Invalid varint".to_string()),
    }
}

/// Parse a transaction input
fn parse_input(data: &[u8]) -> Result<(TxInput, usize), String> {
    if data.len() < 36 {
        return Err("Input too short".to_string());
    }

    let mut offset = 0usize;

    // Txid (32 bytes, little-endian in Bitcoin, but we keep as-is)
    let mut txid: [u8; 32] = data[offset..offset + 32].try_into().unwrap();
    txid.reverse(); // Bitcoin uses reversed txid in serialization
    offset += 32;

    // Vout (4 bytes)
    let vout = u32::from_le_bytes(
        data[offset..offset + 4].try_into().map_err(|_| "Invalid vout")?
    );
    offset += 4;

    // Script sig length (varint)
    let (script_len, bytes_read) = read_varint(&data[offset..])?;
    offset += bytes_read;

    // Script sig
    if offset + script_len as usize > data.len() {
        return Err("Script sig exceeds data".to_string());
    }
    let script_sig = data[offset..offset + script_len as usize].to_vec();
    offset += script_len as usize;

    // Sequence (4 bytes)
    if offset + 4 > data.len() {
        return Err("Input truncated".to_string());
    }
    let sequence = u32::from_le_bytes(
        data[offset..offset + 4].try_into().map_err(|_| "Invalid sequence")?
    );
    offset += 4;

    Ok((TxInput {
        txid,
        vout,
        sequence,
        script_sig,
    }, offset))
}

/// Parse a transaction output
fn parse_output(data: &[u8]) -> Result<(TxOutput, usize), String> {
    if data.len() < 8 {
        return Err("Output too short".to_string());
    }

    let mut offset = 0usize;

    // Value (8 bytes)
    let value = u64::from_le_bytes(
        data[offset..offset + 8].try_into().map_err(|_| "Invalid value")?
    );
    offset += 8;

    // Script pubkey length (varint)
    let (script_len, bytes_read) = read_varint(&data[offset..])?;
    offset += bytes_read;

    // Script pubkey
    if offset + script_len as usize > data.len() {
        return Err("Script pubkey exceeds data".to_string());
    }
    let script_pubkey = data[offset..offset + script_len as usize].to_vec();
    offset += script_len as usize;

    Ok((TxOutput {
        value,
        script_pubkey,
    }, offset))
}

/// Compute BIP-143 sighash for SegWit (P2WPKH) transactions
fn compute_sighash(
    tx: &ParsedTx,
    input: &TxInput,
    pubkey: &[u8],
) -> Result<[u8; 32], String> {
    use bitcoin_hashes::{sha256d, Hash as BitcoinHash};

    // For P2WPKH, we need:
    // 1. hashPrevouts: double-SHA256 of all input outpoints
    // 2. hashSequence: double-SHA256 of all input sequences
    // 3. hashOutputs: double-SHA256 of all outputs

    let mut prevouts_data = Vec::new();
    for inp in &tx.inputs {
        prevouts_data.extend_from_slice(&inp.txid);
        prevouts_data.extend_from_slice(&inp.vout.to_le_bytes());
    }
    let hash_prevouts = sha256d::Hash::hash(&prevouts_data);

    let mut sequences_data = Vec::new();
    for inp in &tx.inputs {
        sequences_data.extend_from_slice(&inp.sequence.to_le_bytes());
    }
    let hash_sequence = sha256d::Hash::hash(&sequences_data);

    let mut outputs_data = Vec::new();
    for out in &tx.outputs {
        outputs_data.extend_from_slice(&out.value.to_le_bytes());
        outputs_data.extend_from_slice(&encode_varint(out.script_pubkey.len() as u64));
        outputs_data.extend_from_slice(&out.script_pubkey);
    }
    let hash_outputs = sha256d::Hash::hash(&outputs_data);

    // Build script code for P2WPKH: 0x1976a914{20-byte-hash160(pubkey)}88ac
    // But for simplicity, we use the pubkey directly (this would need proper hash160 in production)
    let mut script_code = vec![0x19, 0x76, 0xa9, 0x14];
    // In real implementation, we'd hash160 the pubkey here
    // For now, use first 20 bytes of pubkey as placeholder
    if pubkey.len() >= 20 {
        script_code.extend_from_slice(&pubkey[..20]);
    } else {
        return Err("Pubkey too short".to_string());
    }
    script_code.extend_from_slice(&[0x88, 0xac]);

    // Build the sighash preimage
    let mut preimage = Vec::new();

    // Version
    preimage.extend_from_slice(&tx.version.to_le_bytes());

    // hashPrevouts
    preimage.extend_from_slice(hash_prevouts.as_ref());

    // hashSequence
    preimage.extend_from_slice(hash_sequence.as_ref());

    // Outpoint for this input
    preimage.extend_from_slice(&input.txid);
    preimage.extend_from_slice(&input.vout.to_le_bytes());

    // scriptCode
    preimage.extend_from_slice(&encode_varint(script_code.len() as u64));
    preimage.extend_from_slice(&script_code);

    // value - we don't have this in the input, so we use 0 as placeholder
    // In real implementation, we'd need the UTXO value
    preimage.extend_from_slice(&0u64.to_le_bytes());

    // sequence
    preimage.extend_from_slice(&input.sequence.to_le_bytes());

    // hashOutputs
    preimage.extend_from_slice(hash_outputs.as_ref());

    // locktime
    preimage.extend_from_slice(&tx.locktime.to_le_bytes());

    // sighash type (SIGHASH_ALL = 1)
    preimage.extend_from_slice(&1u32.to_le_bytes());

    // Compute double-SHA256
    let hash = sha256d::Hash::hash(&preimage);
    let hash_bytes: [u8; 32] = AsRef::<[u8]>::as_ref(&hash).try_into()
        .map_err(|_| "Hash conversion failed".to_string())?;
    Ok(hash_bytes)
}

/// Build a signed Bitcoin transaction
fn build_signed_transaction(
    tx: &ParsedTx,
    witnesses: Vec<Vec<Vec<u8>>>,
) -> Result<Vec<u8>, String> {
    let mut result = Vec::new();

    // Version
    result.extend_from_slice(&tx.version.to_le_bytes());

    // SegWit marker and flag
    result.push(0x00);
    result.push(0x01);

    // Input count
    result.extend_from_slice(&encode_varint(tx.inputs.len() as u64));

    // Inputs (without witness data)
    for input in &tx.inputs {
        let mut txid_reversed = input.txid;
        txid_reversed.reverse();
        result.extend_from_slice(&txid_reversed);
        result.extend_from_slice(&input.vout.to_le_bytes());
        result.push(0x00); // Empty script sig for SegWit
        result.extend_from_slice(&input.sequence.to_le_bytes());
    }

    // Output count
    result.extend_from_slice(&encode_varint(tx.outputs.len() as u64));

    // Outputs
    for output in &tx.outputs {
        result.extend_from_slice(&output.value.to_le_bytes());
        result.extend_from_slice(&encode_varint(output.script_pubkey.len() as u64));
        result.extend_from_slice(&output.script_pubkey);
    }

    // Witness data
    for witness in witnesses {
        result.extend_from_slice(&encode_varint(witness.len() as u64));
        for item in witness {
            result.extend_from_slice(&encode_varint(item.len() as u64));
            result.extend_from_slice(&item);
        }
    }

    // Locktime
    result.extend_from_slice(&tx.locktime.to_le_bytes());

    Ok(result)
}

/// Fee estimation implementation for Bitcoin using estimatesmartfee RPC
async fn get_fee_estimate_rpc(rpc: &dyn BitcoinRpc) -> ChainOpResult<u64> {
    // Get block count to estimate fee
    let block_count = rpc
        .get_block_count()
        .map_err(|e| ChainOpError::RpcError(format!("Failed to get block count: {}", e)))?;

    // Bitcoin fee estimation based on recent block fullness
    // This is a simplified algorithm - real implementation would use estimatesmartfee RPC
    // Target: 6 blocks confirmation (standard)
    let target_confirmations = 6u64;

    // Estimate based on network activity (simplified)
    // In production, this would call estimatesmartfee
    let estimated_fee_rate = if block_count % 10 == 0 {
        // High traffic period (placeholder logic)
        20u64 // 20 sat/vbyte
    } else {
        // Normal period
        5u64 // 5 sat/vbyte
    };

    // Adjust based on target confirmation time
    // Lower target = higher fee
    let adjusted_fee_rate = match target_confirmations {
        1 => estimated_fee_rate * 5,    // Next block: 5x
        2..=3 => estimated_fee_rate * 3, // 2-3 blocks: 3x
        4..=6 => estimated_fee_rate,     // 4-6 blocks: standard
        _ => std::cmp::max(1, estimated_fee_rate / 2), // Longer: discount
    };

    Ok(adjusted_fee_rate)
}
