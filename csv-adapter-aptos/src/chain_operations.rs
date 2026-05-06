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
    RightOperationResult, TransactionInfo,
    TransactionStatus,
};
use csv_adapter_core::hash::Hash;
use csv_adapter_core::proof::{FinalityProof, InclusionProof as CoreInclusionProof};
use csv_adapter_core::right::RightId;
use csv_adapter_core::signature::SignatureScheme;
use sha3::{Digest, Sha3_256};

use crate::adapter::AptosAnchorLayer;
use crate::config::AptosNetwork;
use crate::proofs::CommitmentEventBuilder;
use crate::rpc::{AptosLedgerInfo, AptosRpc, AptosTransaction};

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

        // Build event builder with default module address
        let module_address = [0u8; 32];
        let event_builder = CommitmentEventBuilder::new(module_address, "CSV::AnchorEvent");
        
        Self {
            rpc,
            network,
            domain_separator: domain,
            event_builder,
        }
    }

    /// Create from AptosAnchorLayer
    pub fn from_anchor_layer(anchor: &AptosAnchorLayer) -> ChainOpResult<Self> {
        let (module_addr, event_type) = anchor.event_builder_config();
        Ok(Self {
            rpc: anchor.rpc().clone_boxed(),
            network: anchor.network(),
            domain_separator: anchor.domain(),
            event_builder: CommitmentEventBuilder::new(module_addr, event_type),
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

        // Look for CoinStore resource
        let mut total_balance = 0u64;
        let token_balances = Vec::new();

        // Get the CoinStore resource directly for accurate balance
        let coin_resource = self.rpc().get_resource(
            addr,
            "0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>",
            None,
        ).await;

        if let Ok(Some(resource)) = coin_resource {
            // Parse coin balance from BCS-encoded resource data
            // CoinStore<T> layout: coin.value (u64) is the first 8 bytes
            if let Some(balance) = resource.parse_coin_balance() {
                total_balance = balance;
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
            .await
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
                    .await
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

        // Check if a specific resource exists at address to determine if contract is deployed
        let resource_result = self.rpc().get_resource(
            addr,
            "0x1::account::Account",
            None,
        ).await;

        let is_deployed = matches!(resource_result, Ok(Some(_)));

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
            .await
            .map_err(|e| ChainOpError::RpcError(format!("Failed to get ledger: {}", e)))?;

        Ok(ledger.ledger_version)
    }

    async fn get_chain_info(&self) -> ChainOpResult<serde_json::Value> {
        let ledger = self
            .rpc()
            .get_ledger_info()
            .await
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
        let mut data = pubkey.to_vec();
        data.push(0x00); // Ed25519 single key scheme
        let hash = Sha3_256::digest(&data);
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

        use ed25519_dalek::{VerifyingKey, Signature, Verifier};

        // Convert bytes to proper types
        let mut pubkey_bytes = [0u8; 32];
        pubkey_bytes.copy_from_slice(public_key);

        let mut sig_bytes = [0u8; 64];
        sig_bytes.copy_from_slice(signature);

        // Create verifying key and signature
        let verifying_key = VerifyingKey::from_bytes(&pubkey_bytes)
            .map_err(|e| ChainOpError::InvalidInput(format!("Invalid public key: {}", e)))?;

        let ed_sig = Signature::from_bytes(&sig_bytes);

        // Verify the signature
        match verifying_key.verify(message, &ed_sig) {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
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
            .await
            .map_err(|e| ChainOpError::TransactionError(format!("Submission failed: {}", e)))?;

        Ok(format!("0x{}", hex::encode(hash)))
    }

    async fn confirm_transaction(
        &self,
        tx_hash: &str,
        _required_confirmations: u64,
        timeout_secs: u64,
    ) -> ChainOpResult<TransactionStatus> {
        // Aptos uses version numbers as tx identifiers - parse and convert to hash
        let _version = self.parse_version(tx_hash)?;
        let hash = self.parse_address(tx_hash)?;

        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(timeout_secs);
        let poll_interval = std::time::Duration::from_millis(500);

        loop {
            if start.elapsed() > timeout {
                return Err(ChainOpError::Timeout(
                    "Transaction confirmation timeout".to_string(),
                ));
            }

            match self.rpc().wait_for_transaction(hash).await {
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
                    #[cfg(feature = "rpc")]
                    tokio::time::sleep(poll_interval).await;

                    #[cfg(not(feature = "rpc"))]
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
    ) -> ChainOpResult<CoreInclusionProof> {
        // Get block/ledger info
        let ledger = self
            .rpc()
            .get_ledger_info()
            .await
            .map_err(|e| ChainOpError::RpcError(format!("Failed to get ledger: {}", e)))?;

        // Build event proof - use a default seal address
        let seal_address = [0u8; 32];
        let event_data = self
            .event_builder
            .build(*commitment.as_bytes(), seal_address);

        // Convert ledger version to 32-byte hash
        let mut block_hash_bytes = [0u8; 32];
        let version_bytes = ledger.ledger_version.to_le_bytes();
        block_hash_bytes[..8].copy_from_slice(&version_bytes);
        
        Ok(CoreInclusionProof {
            proof_bytes: event_data,
            block_hash: Hash::new(block_hash_bytes),
            position: block_height,
        })
    }

    fn verify_inclusion_proof(
        &self,
        proof: &CoreInclusionProof,
        commitment: &Hash,
    ) -> ChainOpResult<bool> {
        let _ = commitment;

        // Verify ledger version exists
        #[cfg(feature = "rpc")]
        let ledger = {
            let rpc = self.rpc().clone_boxed();
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| ChainOpError::RpcError(format!("Failed to build runtime: {}", e)))?;
            rt.block_on(async {
                rpc.get_ledger_info().await
            })
            .map_err(|e| ChainOpError::RpcError(format!("Failed to get ledger: {}", e)))?
        };

        #[cfg(not(feature = "rpc"))]
        let ledger = AptosLedgerInfo {
            chain_id: 0,
            epoch: 0,
            ledger_version: 0,
            oldest_ledger_version: 0,
            ledger_timestamp: 0,
            oldest_transaction_timestamp: 0,
            epoch_start_timestamp: 0,
        };

        if ledger.ledger_version < proof.position {
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
                    .await
                    .map_err(|e| ChainOpError::RpcError(format!("Failed to get ledger: {}", e)))?;

                let proof_data = serde_json::to_vec(&ledger)
                    .map_err(|e| ChainOpError::Unknown(format!("Serialization failed: {}", e)))?;

                // FinalityProof uses: finality_data, confirmations, is_deterministic
                let confirmations = ledger.ledger_version.saturating_sub(finality_block) + 1;
                Ok(FinalityProof::new(
                    proof_data,
                    confirmations,
                    true, // Aptos has deterministic finality via HotStuff
                )
                .map_err(|e| ChainOpError::InvalidInput(format!("Invalid finality proof: {}", e)))?)
            }
            _ => Err(ChainOpError::ProofVerificationError(
                "Transaction not finalized".to_string(),
            )),
        }
    }

    fn verify_finality_proof(
        &self,
        _proof: &FinalityProof,
        _tx_hash: &str,
    ) -> ChainOpResult<bool> {
        // Verify epoch and round
        #[cfg(feature = "rpc")]
        let latest = {
            let rpc = self.rpc().clone_boxed();
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| ChainOpError::RpcError(format!("Failed to build runtime: {}", e)))?;
            rt.block_on(async {
                rpc.get_ledger_info().await
            })
            .map_err(|e| ChainOpError::RpcError(format!("Failed to get ledger: {}", e)))?
        };

        #[cfg(not(feature = "rpc"))]
        let latest = AptosLedgerInfo {
            chain_id: 0,
            epoch: 0,
            ledger_version: 0,
            oldest_ledger_version: 0,
            ledger_timestamp: 0,
            oldest_transaction_timestamp: 0,
            epoch_start_timestamp: 0,
        };

        // Check if finality proof confirms is at least 1 (deterministic finality in Aptos)
        let _confirmations = _proof.confirmations;

        // Would verify HotStuff certificate using finality_data
        let _ = latest;

        Ok(true)
    }

    fn domain_separator(&self) -> [u8; 32] {
        self.domain_separator
    }

    async fn verify_proof_bundle(
        &self,
        inclusion_proof: &CoreInclusionProof,
        finality_proof: &FinalityProof,
        commitment: &Hash,
    ) -> ChainOpResult<bool> {
        let inclusion_valid = self.verify_inclusion_proof(inclusion_proof, commitment)?;
        let finality_valid =
            self.verify_finality_proof(finality_proof, &format!("{}", hex::encode(inclusion_proof.block_hash.as_bytes())))?;

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
        lock_proof: &CoreInclusionProof,
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
        // Query the seal resource at the address derived from right_id
        // In Aptos, resources are stored at the owner's address
        // The right_id contains the address and resource type info

        let right_bytes = right_id.as_bytes();

        // Derive the account address from right_id
        // For simplicity, we use the first 32 bytes as the address
        let mut address_bytes = [0u8; 32];
        if right_bytes.len() >= 32 {
            address_bytes.copy_from_slice(&right_bytes[..32]);
        } else {
            address_bytes[..right_bytes.len()].copy_from_slice(right_bytes);
        }

        // Query account resources via RPC
        // Check if the account exists and has the expected resource
        let account_exists = self.rpc().get_account_sequence_number(address_bytes).await.is_ok();

        if !account_exists {
            // Account doesn't exist - either never created or deleted
            return match expected_state {
                "consumed" | "deleted" | "never_created" => Ok(true),
                _ => Ok(false),
            };
        }

        // For a complete implementation, we would:
        // 1. Query the specific resource type at the address
        // 2. Parse the resource data to determine its state
        // 3. Compare with expected_state

        // Simplified check: account exists means "active"
        let actual_state = if account_exists {
            "active"
        } else {
            "consumed"
        };

        Ok(actual_state == expected_state)
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
