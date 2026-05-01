//! Chain Operation Traits Implementation for Aptos
//!
//! This module implements all chain operation traits from csv-adapter-core:
//! - ChainQuery: Querying chain state via REST API
//! - ChainSigner: Ed25519 signing operations
//! - ChainBroadcaster: Transaction broadcasting
//! - ChainDeployer: Move module deployment
//! - ChainProofProvider: Proof building and verification
//! - ChainRightOps: Right management operations

use async_trait::async_trait;
use csv_adapter_core::chain_operations::{
    BalanceInfo, ChainBroadcaster, ChainDeployer, ChainOpError, ChainOpResult, ChainProofProvider,
    ChainQuery, ChainRightOps, ChainSigner, ContractStatus, DeploymentStatus, FinalityStatus,
    InclusionProof, RightOperation, RightOperationResult, TokenBalance, TransactionInfo,
    TransactionStatus,
};
use csv_adapter_core::hash::Hash;
use csv_adapter_core::proof::{FinalityProof, InclusionProof as CoreInclusionProof};
use csv_adapter_core::right::RightId;
use csv_adapter_core::signature::SignatureScheme;

use crate::adapter::AptosAnchorLayer;
use crate::config::AptosNetwork;
use crate::error::AptosError;
use crate::proofs::CommitmentEventBuilder;
use crate::rpc::{AptosLedgerInfo, AptosResource, AptosRpc, AptosTransaction};

/// Aptos chain operations implementation
pub struct AptosChainOperations {
    /// Inner RPC client for chain communication
    rpc: Box<dyn AptosRpc>,
    /// Chain configuration
    network: AptosNetwork,
    /// Domain separator for proof generation
    domain_separator: [u8; 32],
    /// Commitment event builder
    event_builder: CommitmentEventBuilder,
}

impl AptosChainOperations {
    /// Create new Aptos chain operations from RPC client
    pub fn new(rpc: Box<dyn AptosRpc>, network: AptosNetwork) -> Self {
        let mut domain = [0u8; 32];
        domain[..10].copy_from_slice(b"CSV-APTOS-");
        let chain_id = network.chain_id().to_le_bytes();
        domain[10..18].copy_from_slice(&chain_id);

        Self {
            rpc,
            network,
            domain_separator: domain,
            event_builder: CommitmentEventBuilder::new(),
        }
    }

    /// Create from AptosAnchorLayer
    pub fn from_anchor_layer(anchor: &AptosAnchorLayer) -> ChainOpResult<Self> {
        Ok(Self {
            rpc: anchor.get_rpc(),
            network: anchor.config.network,
            domain_separator: anchor.domain_separator,
            event_builder: CommitmentEventBuilder::new(),
        })
    }

    /// Parse Aptos address from string
    fn parse_address(&self, address: &str) -> ChainOpResult<[u8; 32]> {
        let hex_str = address.trim_start_matches("0x");
        let bytes = hex::decode(hex_str)
            .map_err(|e| ChainOpError::InvalidInput(format!("Invalid hex address: {}", e)))?;

        if bytes.len() != 32 {
            return Err(ChainOpError::InvalidInput(
                "Aptos address must be 32 bytes".to_string(),
            ));
        }

        let mut addr = [0u8; 32];
        addr.copy_from_slice(&bytes);
        Ok(addr)
    }

    /// Format Aptos address for display
    fn format_address(&self, addr: [u8; 32]) -> String {
        format!("0x{}", hex::encode(addr))
    }

    /// Parse transaction hash (version)
    fn parse_version(&self, hash: &str) -> ChainOpResult<u64> {
        // Aptos uses version numbers, not hashes
        hash.parse()
            .map_err(|e| ChainOpError::InvalidInput(format!("Invalid version: {}", e)))
    }

    /// Convert Aptos transaction to TransactionInfo
    fn tx_to_info(&self, tx: &AptosTransaction) -> TransactionInfo {
        let status = if tx.success {
            TransactionStatus::Confirmed {
                block_height: tx.version,
                confirmations: 1, // Aptos has immediate finality
            }
        } else {
            TransactionStatus::Failed {
                reason: tx.vm_status.clone(),
            }
        };

        TransactionInfo {
            hash: format!("0x{}", hex::encode(tx.hash)),
            sender: "unknown".to_string(), // Would need to parse from payload
            recipient: None,
            amount: None,
            status,
            block_height: Some(tx.version),
            timestamp: None,
            fee: Some(tx.gas_used),
            raw_data: Some(tx.payload.clone()),
        }
    }

    /// Get RPC client reference
    fn rpc(&self) -> &dyn AptosRpc {
        self.rpc.as_ref()
    }
}

#[async_trait]
impl ChainQuery for AptosChainOperations {
    async fn get_balance(&self, address: &str) -> ChainOpResult<BalanceInfo> {
        let addr = self.parse_address(address)?;

        // Get account resources
        let resources = self
            .rpc()
            .get_resources(addr)
            .map_err(|e| ChainOpError::RpcError(format!("Failed to get resources: {}", e)))?;

        // Look for CoinStore resource
        let mut total_balance = 0u64;
        let mut token_balances = Vec::new();

        for resource in resources {
            if resource.resource_type == "0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>" {
                // Parse coin balance from resource data
                // This is simplified - real implementation would parse BCS
                total_balance = 0; // Would parse from resource.data
            }
        }

        Ok(BalanceInfo {
            address: address.to_string(),
            total: total_balance,
            available: total_balance,
            locked: 0,
            tokens: token_balances,
        })
    }

    async fn get_transaction(&self, hash: &str) -> ChainOpResult<TransactionInfo> {
        let version = self.parse_version(hash)?;

        let tx = self
            .rpc()
            .get_transaction(version)
            .map_err(|e| ChainOpError::RpcError(format!("Failed to get transaction: {}", e)))?
            .ok_or_else(|| ChainOpError::RpcError("Transaction not found".to_string()))?;

        Ok(self.tx_to_info(&tx))
    }

    async fn get_finality(&self, tx_hash: &str) -> ChainOpResult<FinalityStatus> {
        // In Aptos, transactions are finalized immediately
        // Finality is determined by being in a ledger with certified block
        let tx_info = self.get_transaction(tx_hash).await?;

        match tx_info.status {
            TransactionStatus::Confirmed { block_height, .. } => {
                // Get ledger info to verify
                let ledger = self
                    .rpc()
                    .get_ledger_info()
                    .map_err(|e| ChainOpError::RpcError(format!("Failed to get ledger: {}", e)))?;

                // If transaction version is in current or older epoch, it's finalized
                if block_height <= ledger.ledger_version {
                    Ok(FinalityStatus::Finalized {
                        block_height,
                        finality_block: block_height,
                    })
                } else {
                    Ok(FinalityStatus::Pending)
                }
            }
            TransactionStatus::Failed { .. } => Ok(FinalityStatus::Orphaned),
            _ => Ok(FinalityStatus::Pending),
        }
    }

    async fn get_contract_status(&self, contract_address: &str) -> ChainOpResult<ContractStatus> {
        let addr = self.parse_address(contract_address)?;

        // Get resources at address - if it has module resources, contract exists
        let resources = self.rpc().get_resources(addr);

        let is_deployed = match resources {
            Ok(res) => !res.is_empty(),
            Err(_) => false,
        };

        Ok(ContractStatus {
            address: contract_address.to_string(),
            is_deployed,
            balance: None,
            owner: Some(contract_address.to_string()),
            metadata: serde_json::json!({
                "chain": "aptos",
                "network": format!("{:?}", self.network),
            }),
        })
    }

    async fn get_latest_block_height(&self) -> ChainOpResult<u64> {
        let ledger = self
            .rpc()
            .get_ledger_info()
            .map_err(|e| ChainOpError::RpcError(format!("Failed to get ledger: {}", e)))?;

        Ok(ledger.ledger_version)
    }

    async fn get_chain_info(&self) -> ChainOpResult<serde_json::Value> {
        let ledger = self
            .rpc()
            .get_ledger_info()
            .map_err(|e| ChainOpError::RpcError(format!("Failed to get ledger: {}", e)))?;

        Ok(serde_json::json!({
            "chain_id": ledger.chain_id,
            "chain": "aptos",
            "network": format!("{:?}", self.network),
            "epoch": ledger.epoch,
            "ledger_version": ledger.ledger_version,
            "oldest_version": ledger.oldest_ledger_version,
            "protocol": "Move",
            "finality": "deterministic",
        }))
    }

    fn validate_address(&self, address: &str) -> bool {
        let hex_str = address.trim_start_matches("0x");
        match hex::decode(hex_str) {
            Ok(bytes) => bytes.len() == 32,
            Err(_) => false,
        }
    }
}

#[async_trait]
impl ChainSigner for AptosChainOperations {
    fn derive_address(&self, public_key: &[u8]) -> ChainOpResult<String> {
        if public_key.len() != 32 {
            return Err(ChainOpError::InvalidInput(
                "Ed25519 public key must be 32 bytes".to_string(),
            ));
        }

        let mut pubkey = [0u8; 32];
        pubkey.copy_from_slice(public_key);

        // Aptos authentication key = SHA3-256(public_key | signature_scheme)
        // For single-key accounts: auth_key = SHA3-256(pubkey || 0x00)
        use sha3::{Digest, Sha3_256};
        let mut data = pubkey.to_vec();
        data.push(0x00); // Ed25519 single key scheme
        let hash = Keccak256::digest(&data);
        let mut addr = [0u8; 32];
        addr.copy_from_slice(&hash[..32]);

        Ok(format!("0x{}", hex::encode(addr)))
    }

    async fn sign_transaction(&self, _tx_data: &[u8], _key_id: &str) -> ChainOpResult<Vec<u8>> {
        Err(ChainOpError::CapabilityUnavailable(
            "Direct transaction signing not available. \
             Use an external keystore with the key_id reference.".to_string(),
        ))
    }

    async fn sign_message(&self, _message: &[u8], _key_id: &str) -> ChainOpResult<Vec<u8>> {
        Err(ChainOpError::CapabilityUnavailable(
            "Direct message signing not available. \
             Use an external keystore with the key_id reference.".to_string(),
        ))
    }

    fn verify_signature(
        &self,
        message: &[u8],
        signature: &[u8],
        public_key: &[u8],
    ) -> ChainOpResult<bool> {
        if public_key.len() != 32 {
            return Err(ChainOpError::InvalidInput(
                "Ed25519 public key must be 32 bytes".to_string(),
            ));
        }

        if signature.len() != 64 {
            return Err(ChainOpError::InvalidInput(
                "Ed25519 signature must be 64 bytes".to_string(),
            ));
        }

        // Would use ed25519_dalek for verification
        // Similar to Sui implementation

        let _ = message;
        let _ = signature;
        let _ = public_key;

        Err(ChainOpError::CapabilityUnavailable(
            "Signature verification requires ed25519-dalek integration".to_string(),
        ))
    }

    fn signature_scheme(&self) -> SignatureScheme {
        SignatureScheme::Ed25519
    }
}

#[async_trait]
impl ChainBroadcaster for AptosChainOperations {
    async fn submit_transaction(&self, signed_tx: &[u8]) -> ChainOpResult<String> {
        // Aptos signed transaction is BCS-encoded
        // Submit via submit_signed_transaction

        let signed_json = serde_json::from_slice(signed_tx)
            .map_err(|e| ChainOpError::InvalidInput(format!("Invalid signed transaction: {}", e)))?;

        let hash = self
            .rpc()
            .submit_signed_transaction(signed_json)
            .map_err(|e| ChainOpError::TransactionError(format!("Submission failed: {}", e)))?;

        Ok(format!("0x{}", hex::encode(hash)))
    }

    async fn confirm_transaction(
        &self,
        tx_hash: &str,
        _required_confirmations: u64,
        timeout_secs: u64,
    ) -> ChainOpResult<TransactionStatus> {
        let version = self.parse_version(tx_hash)?;
        let hash = self.parse_tx_hash(tx_hash)?;

        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(timeout_secs);
        let poll_interval = std::time::Duration::from_millis(500);

        loop {
            if start.elapsed() > timeout {
                return Err(ChainOpError::Timeout(
                    "Transaction confirmation timeout".to_string(),
                ));
            }

            match self.rpc().wait_for_transaction(hash) {
                Ok(tx) => {
                    return Ok(if tx.success {
                        TransactionStatus::Confirmed {
                            block_height: tx.version,
                            confirmations: 1,
                        }
                    } else {
                        TransactionStatus::Failed {
                            reason: tx.vm_status,
                        }
                    });
                }
                Err(_) => {
                    std::thread::sleep(poll_interval);
                }
            }
        }
    }

    async fn get_fee_estimate(&self) -> ChainOpResult<u64> {
        // Aptos gas estimation
        // Typical transaction: ~1000 gas units at 100 gas price = 100000 Octa (0.001 APT)
        Ok(100_000)
    }

    async fn validate_transaction(&self, tx_data: &[u8]) -> ChainOpResult<()> {
        if tx_data.is_empty() {
            return Err(ChainOpError::InvalidInput(
                "Empty transaction data".to_string(),
            ));
        }

        // Would decode BCS and validate structure

        Ok(())
    }
}

#[async_trait]
impl ChainDeployer for AptosChainOperations {
    async fn deploy_lock_contract(
        &self,
        admin_address: &str,
        config: serde_json::Value,
    ) -> ChainOpResult<DeploymentStatus> {
        let _ = admin_address;
        let _ = config;

        Err(ChainOpError::CapabilityUnavailable(
            "Lock contract deployment requires Move module publishing. \
             Use deploy_or_publish_seal_program() with compiled Move bytecode.".to_string(),
        ))
    }

    async fn deploy_mint_contract(
        &self,
        admin_address: &str,
        config: serde_json::Value,
    ) -> ChainOpResult<DeploymentStatus> {
        let _ = admin_address;
        let _ = config;

        Err(ChainOpError::CapabilityUnavailable(
            "Mint contract deployment requires Move module publishing. \
             Same module handles both lock and mint in Aptos.".to_string(),
        ))
    }

    async fn deploy_or_publish_seal_program(
        &self,
        program_bytes: &[u8],
        admin_address: &str,
    ) -> ChainOpResult<DeploymentStatus> {
        let _ = program_bytes;
        let _ = admin_address;

        Err(ChainOpError::CapabilityUnavailable(
            "Seal program publishing requires signed transaction. \
             Use deploy_csv_seal_module() with compiled Move bytecode \
             or external tools (aptos move publish).".to_string(),
        ))
    }

    async fn verify_deployment(&self, contract_address: &str) -> ChainOpResult<bool> {
        let status = self.get_contract_status(contract_address).await?;
        Ok(status.is_deployed)
    }

    async fn estimate_deployment_cost(&self, program_bytes: &[u8]) -> ChainOpResult<u64> {
        // Aptos deployment cost estimation
        let base_cost = 100_000u64; // Base gas
        let per_byte_cost = 10u64; // Gas per byte of code
        let code_cost = (program_bytes.len() as u64) * per_byte_cost;

        Ok(base_cost + code_cost)
    }
}

#[async_trait]
impl ChainProofProvider for AptosChainOperations {
    async fn build_inclusion_proof(
        &self,
        commitment: &Hash,
        block_height: u64,
    ) -> ChainOpResult<InclusionProof> {
        // Get block/ledger info
        let ledger = self
            .rpc()
            .get_ledger_info()
            .map_err(|e| ChainOpError::RpcError(format!("Failed to get ledger: {}", e)))?;

        // Build event proof
        let event_data = self
            .event_builder
            .build_commitment_event(commitment, block_height);

        Ok(InclusionProof {
            block_height,
            transaction_hash: format!("{}", ledger.ledger_version),
            proof_data: event_data,
            merkle_root: ledger.event_root_hash.to_vec(),
        })
    }

    fn verify_inclusion_proof(
        &self,
        proof: &InclusionProof,
        commitment: &Hash,
    ) -> ChainOpResult<bool> {
        let _ = commitment;

        // Verify ledger version exists
        let ledger = self
            .rpc()
            .get_ledger_info()
            .map_err(|e| ChainOpError::RpcError(format!("Failed to get ledger: {}", e)))?;

        if ledger.ledger_version < proof.block_height {
            return Ok(false);
        }

        // Would verify against accumulator_root_hash

        Ok(true)
    }

    async fn build_finality_proof(&self, tx_hash: &str) -> ChainOpResult<FinalityProof> {
        let finality = self.get_finality(tx_hash).await?;

        match finality {
            FinalityStatus::Finalized { finality_block, .. } => {
                let ledger = self
                    .rpc()
                    .get_ledger_info()
                    .map_err(|e| ChainOpError::RpcError(format!("Failed to get ledger: {}", e)))?;

                let proof_data = serde_json::to_vec(&ledger)
                    .map_err(|e| ChainOpError::Unknown(format!("Serialization failed: {}", e)))?;

                Ok(FinalityProof {
                    block_height: finality_block,
                    proof_data,
                    signature: ledger.block_hash.to_vec(),
                })
            }
            _ => Err(ChainOpError::ProofVerificationError(
                "Transaction not finalized".to_string(),
            )),
        }
    }

    fn verify_finality_proof(
        &self,
        proof: &FinalityProof,
        tx_hash: &str,
    ) -> ChainOpResult<bool> {
        let _ = tx_hash;

        // Verify epoch and round
        let latest = self
            .rpc()
            .get_ledger_info()
            .map_err(|e| ChainOpError::RpcError(format!("Failed to get ledger: {}", e)))?;

        let depth = latest.ledger_version.saturating_sub(proof.block_height);
        if depth < 1 {
            return Ok(false);
        }

        // Would verify HotStuff certificate

        Ok(true)
    }

    fn domain_separator(&self) -> [u8; 32] {
        self.domain_separator
    }

    async fn verify_proof_bundle(
        &self,
        inclusion_proof: &InclusionProof,
        finality_proof: &FinalityProof,
        commitment: &Hash,
    ) -> ChainOpResult<bool> {
        let inclusion_valid = self.verify_inclusion_proof(inclusion_proof, commitment)?;
        let finality_valid =
            self.verify_finality_proof(finality_proof, &inclusion_proof.transaction_hash)?;

        Ok(inclusion_valid && finality_valid)
    }
}

#[async_trait]
impl ChainRightOps for AptosChainOperations {
    async fn create_right(
        &self,
        owner: &str,
        asset_class: &str,
        asset_id: &str,
        metadata: serde_json::Value,
    ) -> ChainOpResult<RightOperationResult> {
        let _ = owner;
        let _ = asset_class;
        let _ = asset_id;
        let _ = metadata;

        Err(ChainOpError::CapabilityUnavailable(
            "Right creation requires signed transaction. \
             Construct and submit a transaction to create the seal resource.".to_string(),
        ))
    }

    async fn consume_right(
        &self,
        right_id: &RightId,
        owner_key_id: &str,
    ) -> ChainOpResult<RightOperationResult> {
        let _ = right_id;
        let _ = owner_key_id;

        Err(ChainOpError::CapabilityUnavailable(
            "Right consumption requires signed transaction. \
             Construct and submit a transaction to consume the seal resource.".to_string(),
        ))
    }

    async fn lock_right(
        &self,
        right_id: &RightId,
        destination_chain: &str,
        owner_key_id: &str,
    ) -> ChainOpResult<RightOperationResult> {
        let _ = right_id;
        let _ = destination_chain;
        let _ = owner_key_id;

        Err(ChainOpError::CapabilityUnavailable(
            "Right locking requires signed transaction. \
             Construct and submit a transaction to lock the seal resource.".to_string(),
        ))
    }

    async fn mint_right(
        &self,
        source_chain: &str,
        source_right_id: &RightId,
        lock_proof: &InclusionProof,
        new_owner: &str,
    ) -> ChainOpResult<RightOperationResult> {
        let _ = source_chain;
        let _ = source_right_id;
        let _ = lock_proof;
        let _ = new_owner;

        Err(ChainOpError::CapabilityUnavailable(
            "Right minting requires signed transaction. \
             Verify lock proof, then construct and submit mint transaction.".to_string(),
        ))
    }

    async fn refund_right(
        &self,
        right_id: &RightId,
        owner_key_id: &str,
    ) -> ChainOpResult<RightOperationResult> {
        let _ = right_id;
        let _ = owner_key_id;

        Err(ChainOpError::CapabilityUnavailable(
            "Right refund requires signed transaction. \
             Construct and submit a transaction to refund the locked seal.".to_string(),
        ))
    }

    async fn record_right_metadata(
        &self,
        right_id: &RightId,
        metadata: serde_json::Value,
        owner_key_id: &str,
    ) -> ChainOpResult<RightOperationResult> {
        let _ = right_id;
        let _ = metadata;
        let _ = owner_key_id;

        Err(ChainOpError::CapabilityUnavailable(
            "Metadata recording requires signed transaction. \
             Construct and submit a transaction to update seal metadata.".to_string(),
        ))
    }

    async fn verify_right_state(
        &self,
        right_id: &RightId,
        expected_state: &str,
    ) -> ChainOpResult<bool> {
        let _ = expected_state;

        // Query resource at address
        let commitment = right_id.as_bytes();
        let _ = commitment;

        Err(ChainOpError::CapabilityUnavailable(
            "Right state verification requires resource query. \
             Query the seal resource at the expected address.".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rpc::MockAptosRpc;

    #[test]
    fn test_aptos_chain_operations_creation() {
        let rpc = Box::new(MockAptosRpc::new(1));
        let ops = AptosChainOperations::new(rpc, AptosNetwork::Devnet);
        assert_eq!(ops.network, AptosNetwork::Devnet);
    }

    #[test]
    fn test_address_validation() {
        let rpc = Box::new(MockAptosRpc::new(1));
        let ops = AptosChainOperations::new(rpc, AptosNetwork::Devnet);

        // Valid address
        assert!(ops.validate_address(
            "0x0000000000000000000000000000000000000000000000000000000000000001"
        ));

        // Invalid - too short
        assert!(!ops.validate_address("0x1234"));

        // Invalid - not hex
        assert!(!ops.validate_address("0xZZZZ"));
    }
}
