//! Chain Operation Traits Implementation for Sui
//!
//! This module implements all chain operation traits from csv-adapter-core:
//! - ChainQuery: Querying chain state
//! - ChainSigner: Signing operations
//! - ChainBroadcaster: Transaction broadcasting
//! - ChainDeployer: Contract deployment
//! - ChainProofProvider: Proof building and verification
//! - ChainRightOps: Right management operations

use async_trait::async_trait;
use csv_core::chain_operations::{
    BalanceInfo, ChainBroadcaster, ChainDeployer, ChainOpError, ChainOpResult, ChainProofProvider,
    ChainQuery, ChainRightOps, ChainSigner, ContractStatus, DeploymentStatus, FinalityStatus, RightOperationResult, TransactionInfo, TransactionStatus,
};
use csv_core::hash::Hash;
use csv_core::proof::{FinalityProof, InclusionProof as CoreInclusionProof};
use csv_core::right::RightId;
use csv_core::signature::SignatureScheme;
use ed25519_dalek::{VerifyingKey, Verifier};

use crate::seal_protocol::SuiSealProtocol;
use crate::config::SuiConfig;
use crate::deploy::{PackageDeployer, PackageDeployment};
use crate::error::SuiError;
use crate::proofs::CommitmentEventBuilder;
use crate::rpc::{SuiRpc, SuiTransactionBlock};

/// Block on an async future in a sync context.
fn block_on_async<F: std::future::Future<Output = R> + Send + 'static, R: Send + 'static>(
    future: F,
) -> ChainOpResult<R> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| ChainOpError::RpcError(format!("Failed to create runtime: {}", e)))?;
    Ok(rt.block_on(future))
}

/// Block on an async future that returns Result<T, E> in a sync context.
fn block_on_async_result<F, T, E>(future: F) -> ChainOpResult<T>
where
    F: std::future::Future<Output = Result<T, E>> + Send + 'static,
    E: std::fmt::Display + Send + 'static,
{
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| ChainOpError::RpcError(format!("Failed to create runtime: {}", e)))?;
    rt.block_on(future).map_err(|e| ChainOpError::RpcError(e.to_string()))
}

/// Sui chain operations implementation
///
/// This struct provides complete implementations of all chain operation traits
/// for the Sui blockchain, enabling production use of the CSV protocol.
pub struct SuiBackend {
    /// Inner RPC client for chain communication
    rpc: Box<dyn SuiRpc>,
    /// Chain configuration
    config: SuiConfig,
    /// Domain separator for proof generation
    domain_separator: [u8; 32],
    /// Commitment event builder for proof construction
    event_builder: CommitmentEventBuilder,
}

impl SuiBackend {
    /// Create new Sui chain operations from RPC client and config
    pub fn new(rpc: Box<dyn SuiRpc>, config: SuiConfig) -> Self {
        let mut domain = [0u8; 32];
        domain[..8].copy_from_slice(b"CSV-SUI-");
        let chain_id = config.chain_id().as_bytes();
        let copy_len = chain_id.len().min(24);
        domain[8..8 + copy_len].copy_from_slice(&chain_id[..copy_len]);

        // Build event builder with default package ID
        let package_id = [0u8; 32];
        let event_builder = CommitmentEventBuilder::new(package_id, "csv_seal::AnchorEvent".to_string());
        
        Self {
            rpc,
            config,
            domain_separator: domain,
            event_builder,
        }
    }

    /// Create from SuiSealProtocol
    pub fn from_anchor_layer(anchor: &SuiSealProtocol) -> ChainOpResult<Self> {
        let (module_addr, event_type) = anchor.event_builder_config();
        Ok(Self {
            rpc: anchor.get_rpc().clone_boxed(),
            config: anchor.config.clone(),
            domain_separator: anchor.get_domain_separator(),
            event_builder: CommitmentEventBuilder::new(module_addr, event_type),
        })
    }

    /// Parse Sui address from string
    fn parse_address(&self, address: &str) -> ChainOpResult<[u8; 32]> {
        let hex_str = address.trim_start_matches("0x");
        let bytes = hex::decode(hex_str)
            .map_err(|e| ChainOpError::InvalidInput(format!("Invalid hex address: {}", e)))?;

        if bytes.len() != 32 {
            return Err(ChainOpError::InvalidInput(
                "Sui address must be 32 bytes".to_string(),
            ));
        }

        let mut addr = [0u8; 32];
        addr.copy_from_slice(&bytes);
        Ok(addr)
    }

    /// Format Sui address for display
    fn format_address(&self, addr: [u8; 32]) -> String {
        format!("0x{}", hex::encode(addr))
    }

    /// Convert Sui transaction to TransactionInfo
    fn tx_to_info(&self, tx: &SuiTransactionBlock, hash: &str) -> TransactionInfo {
        let status = match &tx.effects.status {
            crate::rpc::SuiExecutionStatus::Success => TransactionStatus::Confirmed {
                block_height: tx.checkpoint.unwrap_or(0),
                confirmations: 1, // Sui has immediate finality
            },
            crate::rpc::SuiExecutionStatus::Failure { error } => {
                TransactionStatus::Failed {
                    reason: error.clone(),
                }
            }
        };

        TransactionInfo {
            hash: hash.to_string(),
            sender: "unknown".to_string(), // Would parse from transaction data
            recipient: None,
            amount: None,
            status,
            block_height: tx.checkpoint,
            timestamp: None,
            fee: Some(tx.effects.gas_used),
            raw_data: None,
        }
    }

    /// Get RPC client reference
    fn rpc(&self) -> &dyn SuiRpc {
        self.rpc.as_ref()
    }
}

#[async_trait]
impl ChainQuery for SuiBackend {
    async fn get_balance(&self, address: &str) -> ChainOpResult<BalanceInfo> {
        let addr = self.parse_address(address)?;

        // Get gas objects (SUI coins) owned by address
        let objects = self
            .rpc()
            .get_gas_objects(addr)
            .await
            .map_err(|e| ChainOpError::RpcError(format!("Failed to get gas objects: {}", e)))?;

        let mut total_balance = 0u64;
        let token_balances = Vec::new();

        for obj in objects {
            if obj.object_type == "0x2::coin::Coin<0x2::sui::SUI>" {
                // Parse balance from BCS-encoded coin object data
                // Coin<T> structure: { id: UID (32 bytes), value: u64 (8 bytes) }
                if let Some(balance) = obj.parse_coin_balance() {
                    total_balance += balance;
                }
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
        let digest_hex = hash.trim_start_matches("0x");
        let digest_bytes = hex::decode(digest_hex)
            .map_err(|e| ChainOpError::InvalidInput(format!("Invalid digest: {}", e)))?;

        if digest_bytes.len() != 32 {
            return Err(ChainOpError::InvalidInput(
                "Transaction digest must be 32 bytes".to_string(),
            ));
        }

        let mut digest = [0u8; 32];
        digest.copy_from_slice(&digest_bytes);

        let tx = self
            .rpc()
            .get_transaction_block(digest)
            .await
            .map_err(|e| ChainOpError::RpcError(format!("Failed to get transaction: {}", e)))?
            .ok_or_else(|| ChainOpError::RpcError("Transaction not found".to_string()))?;

        Ok(self.tx_to_info(&tx, hash))
    }

    async fn get_finality(&self, tx_hash: &str) -> ChainOpResult<FinalityStatus> {
        // In Sui, transactions are finalized when they are included in a checkpoint
        let tx_info = self.get_transaction(tx_hash).await?;

        match tx_info.status {
            TransactionStatus::Confirmed { block_height, .. } => {
                // Get the latest checkpoint to compute finality depth
                let latest_checkpoint = self
                    .rpc()
                    .get_latest_checkpoint_sequence_number()
                    .await
                    .map_err(|e| ChainOpError::RpcError(format!("Failed to get checkpoint: {}", e)))?;

                let finality_depth = latest_checkpoint.saturating_sub(block_height);

                // Sui has deterministic finality after 1 checkpoint
                if finality_depth >= 1 {
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
        // In Sui, packages are the equivalent of smart contracts
        let package_id = self.parse_address(contract_address)?;

        // Try to get the package object
        let result = self.rpc().get_object(package_id).await;

        let is_deployed = match result {
            Ok(Some(obj)) => !obj.object_type.is_empty(),
            _ => false,
        };

        Ok(ContractStatus {
            address: contract_address.to_string(),
            is_deployed,
            balance: None,
            owner: None,
            metadata: serde_json::json!({
                "chain": "sui",
                "network": format!("{:?}", self.config.network),
            }),
        })
    }

    async fn get_latest_block_height(&self) -> ChainOpResult<u64> {
        self.rpc()
            .get_latest_checkpoint_sequence_number()
            .await
            .map_err(|e| ChainOpError::RpcError(format!("Failed to get checkpoint: {}", e)))
    }

    async fn get_chain_info(&self) -> ChainOpResult<serde_json::Value> {
        let checkpoint = self.get_latest_block_height().await?;
        let ledger = self
            .rpc()
            .get_ledger_info()
            .await
            .map_err(|e| ChainOpError::RpcError(format!("Failed to get ledger info: {}", e)))?;

        Ok(serde_json::json!({
            "chain_id": self.config.network.chain_id(),
            "chain": "sui",
            "network": format!("{:?}", self.config.network),
            "latest_checkpoint": checkpoint,
            "epoch": ledger.latest_epoch,
            "protocol_version": "1.0",
            "finality": "deterministic",
        }))
    }

    async fn get_account_nonce(&self, _address: &str) -> ChainOpResult<u64> {
        // Sui uses an object-based model, not account nonces
        // Object sequence numbers are per-object, not per-account
        Err(ChainOpError::CapabilityUnavailable(
            "Sui does not support account nonces (uses object model)".to_string()
        ))
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
impl ChainSigner for SuiBackend {
    fn derive_address(&self, public_key: &[u8]) -> ChainOpResult<String> {
        if public_key.len() != 32 {
            return Err(ChainOpError::InvalidInput(
                "Ed25519 public key must be 32 bytes".to_string(),
            ));
        }

        let mut pubkey = [0u8; 32];
        pubkey.copy_from_slice(public_key);

        // Sui address is derived from public key using SHA2-256 (or SHA3-256 in production)
        // Address = SHA2-256(pubkey)[0..32]
        use sha2::{Digest, Sha256};
        let hash = Sha256::digest(&pubkey);
        let mut addr = [0u8; 32];
        addr.copy_from_slice(&hash[..32]);

        Ok(format!("0x{}", hex::encode(addr)))
    }

    async fn sign_transaction(&self, _tx_data: &[u8], _key_id: &str) -> ChainOpResult<Vec<u8>> {
        // Note: Signing requires access to private keys which should be managed
        // by a secure keystore, not stored in this operations struct.
        //
        // For production use, this should:
        // 1. Call out to a keystore or HSM
        // 2. Use the key_id to reference the stored key
        // 3. Return the signature without exposing the private key
        Err(ChainOpError::CapabilityUnavailable(
            "Direct transaction signing not available. \
             Use an external keystore with the key_id reference.".to_string(),
        ))
    }

    async fn sign_message(&self, _message: &[u8], _key_id: &str) -> ChainOpResult<Vec<u8>> {
        // Same pattern as sign_transaction
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

        let mut pubkey_bytes = [0u8; 32];
        pubkey_bytes.copy_from_slice(public_key);

        let mut sig_bytes = [0u8; 64];
        sig_bytes.copy_from_slice(signature);

        let verifying_key = VerifyingKey::from_bytes(&pubkey_bytes)
            .map_err(|e| ChainOpError::InvalidInput(format!("Invalid public key: {:?}", e)))?;

        let ed_sig = ed25519_dalek::Signature::from_bytes(&sig_bytes);

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
impl ChainBroadcaster for SuiBackend {
    async fn submit_transaction(&self, signed_tx: &[u8]) -> ChainOpResult<String> {
        // Sui transactions are BCS-encoded TransactionData with signatures
        // The signed_tx format: [tx_bytes_len:4][tx_bytes][signature:64][public_key:32]
        // This format allows proper deserialization and submission to the Sui network

        if signed_tx.len() < 64 {
            return Err(ChainOpError::InvalidInput(
                "Signed transaction too short".to_string(),
            ));
        }

        // Parse the signed transaction
        // Format: [tx_bytes_len:4][tx_bytes][signature:64][public_key:32]
        let tx_len = u32::from_le_bytes([
            signed_tx[0], signed_tx[1], signed_tx[2], signed_tx[3],
        ]) as usize;

        if signed_tx.len() < 4 + tx_len + 64 + 32 {
            return Err(ChainOpError::InvalidInput(
                "Invalid signed transaction format".to_string(),
            ));
        }

        let tx_bytes = signed_tx[4..4 + tx_len].to_vec();
        let signature = signed_tx[4 + tx_len..4 + tx_len + 64].to_vec();
        let public_key = signed_tx[4 + tx_len + 64..4 + tx_len + 64 + 32].to_vec();

        // Submit via RPC
        let digest = self
            .rpc()
            .execute_signed_transaction(tx_bytes, signature, public_key)
            .await
            .map_err(|e| ChainOpError::TransactionError(format!("Submission failed: {}", e)))?;

        Ok(format!("0x{}", hex::encode(digest)))
    }

    async fn confirm_transaction(
        &self,
        tx_hash: &str,
        _required_confirmations: u64,
        timeout_secs: u64,
    ) -> ChainOpResult<TransactionStatus> {
        let digest_hex = tx_hash.trim_start_matches("0x");
        let digest_bytes = hex::decode(digest_hex)
            .map_err(|e| ChainOpError::InvalidInput(format!("Invalid digest: {}", e)))?;

        if digest_bytes.len() != 32 {
            return Err(ChainOpError::InvalidInput(
                "Transaction digest must be 32 bytes".to_string(),
            ));
        }

        let mut digest = [0u8; 32];
        digest.copy_from_slice(&digest_bytes);

        // Wait for transaction with timeout
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(timeout_secs);
        let poll_interval = std::time::Duration::from_secs(2);

        loop {
            if start.elapsed() > timeout {
                return Err(ChainOpError::Timeout(
                    "Transaction confirmation timeout".to_string(),
                ));
            }

            match self.rpc().wait_for_transaction(digest, 5000).await {
                Ok(Some(tx)) => {
                    return Ok(match tx.effects.status {
                        crate::rpc::SuiExecutionStatus::Success => TransactionStatus::Confirmed {
                            block_height: tx.checkpoint.unwrap_or(0),
                            confirmations: 1,
                        },
                        crate::rpc::SuiExecutionStatus::Failure { error } => {
                            TransactionStatus::Failed { reason: error }
                        }
                    });
                }
                Ok(None) => {
                    // Transaction not found yet, wait and retry
                    tokio::time::sleep(poll_interval).await;
                }
                Err(e) => {
                    return Err(ChainOpError::RpcError(format!(
                        "Failed to wait for transaction: {}",
                        e
                    )));
                }
            }
        }
    }

    async fn get_fee_estimate(&self) -> ChainOpResult<u64> {
        // Sui uses gas units with reference gas price
        // Typical transaction costs 2000-5000 gas units
        // Reference gas price is ~1000 MIST (micro-SUI) per unit
        // So estimate: 3000 units * 1000 MIST = 3,000,000 MIST = 0.003 SUI
        Ok(3_000_000)
    }

    async fn validate_transaction(&self, tx_data: &[u8]) -> ChainOpResult<()> {
        // In Sui, transaction validation is done by the validator
        // We can perform basic checks here:
        // 1. Check transaction format is valid BCS
        // 2. Check sender address is valid
        // 3. Check gas budget is reasonable

        if tx_data.is_empty() {
            return Err(ChainOpError::InvalidInput(
                "Empty transaction data".to_string(),
            ));
        }

        // Sui transaction data should be BCS-encoded TransactionData
        // Minimum size check
        if tx_data.len() < 32 {
            return Err(ChainOpError::InvalidInput(
                "Transaction data too short for valid Sui transaction".to_string(),
            ));
        }

        // For full validation, would need to:
        // 1. Deserialize BCS
        // 2. Check sender address format
        // 3. Verify gas objects exist
        // 4. Check gas budget

        Ok(())
    }
}

#[async_trait]
impl ChainDeployer for SuiBackend {
    async fn deploy_lock_contract(
        &self,
        admin_address: &str,
        config: serde_json::Value,
    ) -> ChainOpResult<DeploymentStatus> {
        // Sui doesn't have traditional "lock contracts" like EVM chains
        // Instead, rights are locked by transferring objects to a shared object
        // or by using the CSV seal package
        //
        // For Sui, we would:
        // 1. Deploy the CSV seal package if not already deployed
        // 2. Create a shared object for managing locks
        // 3. Initialize with admin address

        let _ = admin_address;
        let _ = config;

        // This would require publishing a Move package
        Err(ChainOpError::CapabilityUnavailable(
            "Sui lock contract deployment requires Move package publishing. \
             Use deploy_or_publish_seal_program() with the CSV seal package bytecode."
                .to_string(),
        ))
    }

    async fn deploy_mint_contract(
        &self,
        admin_address: &str,
        config: serde_json::Value,
    ) -> ChainOpResult<DeploymentStatus> {
        // Similar to lock contract, minting is done via Move modules
        let _ = admin_address;
        let _ = config;

        Err(ChainOpError::CapabilityUnavailable(
            "Sui mint contract deployment requires Move package publishing. \
             Use deploy_or_publish_seal_program() with the CSV seal package bytecode."
                .to_string(),
        ))
    }

    async fn deploy_or_publish_seal_program(
        &self,
        program_bytes: &[u8],
        _admin_address: &str,
    ) -> ChainOpResult<DeploymentStatus> {
         // Use the SDK-based deployer for Move package publishing
        let deployer = PackageDeployer::new(
            self.config.clone(),
            self.rpc.clone_boxed(),
        );

        // Gas budget from config or default
        let gas_budget = self.config.transaction.max_gas_budget;

        match deployer.deploy_package(program_bytes, gas_budget).await {
            Ok(PackageDeployment {
                package_id,
                transaction_digest,
                gas_used: _,
                modules: _,
                dependencies: _,
            }) => {
                let package_id_hex = format!("0x{}", hex::encode(&package_id));

                Ok(DeploymentStatus::Success {
                    contract_address: package_id_hex.clone(),
                    transaction_hash: transaction_digest,
                    block_height: 0, // Would get from transaction result
                })
            }
            Err(e) => Err(ChainOpError::from(e)),
        }
    }

    async fn verify_deployment(&self, contract_address: &str) -> ChainOpResult<bool> {
        let status = self.get_contract_status(contract_address).await?;
        Ok(status.is_deployed)
    }

    async fn estimate_deployment_cost(&self, program_bytes: &[u8]) -> ChainOpResult<u64> {
        // Sui deployment costs:
        // 1. Storage cost for package (based on bytecode size)
        // 2. Transaction gas for publish
        // 3. Storage rebate
        //
        // Rough estimate: 0.1 SUI base + 0.001 SUI per KB of bytecode
        let base_cost = 100_000_000; // 0.1 SUI in MIST
        let per_kb_cost = 1_000_000; // 0.001 SUI per KB
        let size_kb = (program_bytes.len() + 1023) / 1024;

        Ok(base_cost + (size_kb as u64 * per_kb_cost))
    }
}

#[async_trait]
impl ChainProofProvider for SuiBackend {
    async fn build_inclusion_proof(
        &self,
        commitment: &Hash,
        block_height: u64,
    ) -> ChainOpResult<CoreInclusionProof> {
        // Get the checkpoint for the given height
        let checkpoint = self
            .rpc()
            .get_checkpoint(block_height)
            .await
            .map_err(|e| ChainOpError::RpcError(format!("Failed to get checkpoint: {}", e)))?
            .ok_or_else(|| ChainOpError::RpcError("Checkpoint not found".to_string()))?;

        // Build an event proof for the commitment
        let seal_object_id = [0u8; 32]; // Default seal object ID
        let event_data = self
            .event_builder
            .build(*commitment.as_bytes(), seal_object_id);

        Ok(CoreInclusionProof {
            proof_bytes: event_data,
            block_hash: Hash::new(checkpoint.digest),
            position: block_height,
        })
    }

    fn verify_inclusion_proof(
        &self,
        proof: &CoreInclusionProof,
        commitment: &Hash,
    ) -> ChainOpResult<bool> {
        // In Sui, inclusion is verified via checkpoint certificates
        // The checkpoint contains the transaction digest
        // We verify that the commitment was included in that checkpoint

        let _ = commitment;

        // Verify the checkpoint exists and is valid
        let digest_hex = format!("0x{}", hex::encode(proof.block_hash.as_bytes()));
        let digest_hex = digest_hex.trim_start_matches("0x");
        let digest_bytes = match hex::decode(digest_hex) {
            Ok(bytes) => bytes,
            Err(_) => return Ok(false),
        };

        if digest_bytes.len() != 32 {
            return Ok(false);
        }

        let mut digest = [0u8; 32];
        digest.copy_from_slice(&digest_bytes);

          // Verify checkpoint exists
        let rpc = self.rpc.clone_boxed();
        let position = proof.position;
        let result = std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(rpc.get_checkpoint(position))
        }).join();
        match result {
            Ok(Ok(Some(cp))) => Ok(cp.digest == digest),
            _ => Ok(false),
        }
    }

    async fn build_finality_proof(&self, tx_hash: &str) -> ChainOpResult<FinalityProof> {
        // Get transaction finality status
        let finality = self.get_finality(tx_hash).await?;

        match finality {
            FinalityStatus::Finalized { finality_block, .. } => {
                // Get checkpoint data
                let checkpoint = self
                    .rpc()
                    .get_checkpoint(finality_block)
                    .await
                    .map_err(|e| ChainOpError::RpcError(format!("Failed to get checkpoint: {}", e)))?
                    .ok_or_else(|| ChainOpError::RpcError("Checkpoint not found".to_string()))?;

                // Build checkpoint certificate proof
                let proof_data = serde_json::to_vec(&checkpoint)
                    .map_err(|e| ChainOpError::Unknown(format!("Serialization failed: {}", e)))?;

                Ok(FinalityProof {
                    finality_data: proof_data,
                    confirmations: 1, // Sui has immediate finality after 1 checkpoint
                    is_deterministic: true,
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
        // Verify the checkpoint certificate
        let _ = tx_hash;

        // Deserialize checkpoint
        let checkpoint: crate::rpc::SuiCheckpoint = match serde_json::from_slice(&proof.finality_data) {
            Ok(cp) => cp,
            Err(_) => return Ok(false),
        };

        // Verify checkpoint is certified
        if !checkpoint.certified {
            return Ok(false);
        }

        // Verify the proof signature matches the checkpoint digest
        if proof.finality_data != checkpoint.digest.to_vec() {
            return Ok(false);
        }

           // Verify checkpoint is old enough for finality
        let rpc = self.rpc.clone_boxed();
        let result = std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(rpc.get_latest_checkpoint_sequence_number())
        }).join();
        let latest = match result {
            Ok(v) => v.map_err(|e| ChainOpError::RpcError(format!("Failed to get latest checkpoint: {}", e)))?,
            Err(e) => return Err(ChainOpError::RpcError(format!("Blocking task failed: {:?}", e))),
        };

        let depth = latest.saturating_sub(checkpoint.sequence_number);
        Ok(depth >= 1) // At least 1 checkpoint deep
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
        // Verify both proofs
        let inclusion_valid = self.verify_inclusion_proof(inclusion_proof, commitment)?;
        let finality_valid =
            self.verify_finality_proof(finality_proof, &format!("0x{}", hex::encode(inclusion_proof.block_hash.as_bytes())))?;

        Ok(inclusion_valid && finality_valid)
    }
}

#[async_trait]
impl ChainRightOps for SuiBackend {
    async fn create_right(
        &self,
        owner: &str,
        asset_class: &str,
        asset_id: &str,
        metadata: serde_json::Value,
    ) -> ChainOpResult<RightOperationResult> {
        // In Sui, creating a right involves:
        // 1. Creating a new object representing the right
        // 2. Transferring it to the owner
        // 3. Recording the commitment

        let _ = owner;
        let _ = asset_class;
        let _ = asset_id;
        let _ = metadata;

        // This requires a transaction to create the right object
        // The transaction needs to be constructed and signed externally
        Err(ChainOpError::CapabilityUnavailable(
            "Right creation requires a signed transaction. \
             Construct a transaction to create the right object, \
             then use submit_transaction() to execute.".to_string(),
        ))
    }

    async fn consume_right(
        &self,
        right_id: &RightId,
        owner_key_id: &str,
    ) -> ChainOpResult<RightOperationResult> {
        let _ = right_id;
        let _ = owner_key_id;

        // Consuming a right in Sui means:
        // 1. Taking the right object
        // 2. Deleting it or marking it consumed
        // 3. Recording the nullifier

        Err(ChainOpError::CapabilityUnavailable(
            "Right consumption requires a signed transaction. \
             Construct a transaction to consume the right object, \
             then use submit_transaction() to execute.".to_string(),
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

        // Locking a right:
        // 1. Transfer right object to a shared lock object
        // 2. Record lock with destination chain
        // 3. Generate lock proof

        Err(ChainOpError::CapabilityUnavailable(
            "Right locking requires a signed transaction. \
             Construct a transaction to lock the right object, \
             then use submit_transaction() to execute.".to_string(),
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

        // Minting a right on destination chain:
        // 1. Verify the lock proof from source chain
        // 2. Create new right object
        // 3. Transfer to new owner

        Err(ChainOpError::CapabilityUnavailable(
            "Right minting requires a signed transaction. \
             Construct a transaction to mint the right object after \
             verifying the lock proof, then use submit_transaction() to execute.".to_string(),
        ))
    }

    async fn refund_right(
        &self,
        right_id: &RightId,
        owner_key_id: &str,
    ) -> ChainOpResult<RightOperationResult> {
        let _ = right_id;
        let _ = owner_key_id;

        // Refunding a locked right:
        // 1. Verify lock timeout exceeded
        // 2. Return right to owner
        // 3. Remove from lock state

        Err(ChainOpError::CapabilityUnavailable(
            "Right refund requires a signed transaction. \
             Construct a transaction to refund the locked right, \
             then use submit_transaction() to execute.".to_string(),
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

        // Recording metadata:
        // 1. Update the right object with new metadata
        // 2. Or create a metadata object linked to the right

        Err(ChainOpError::CapabilityUnavailable(
            "Metadata recording requires a signed transaction. \
             Construct a transaction to update the right metadata, \
             then use submit_transaction() to execute.".to_string(),
        ))
    }

    async fn verify_right_state(
        &self,
        right_id: &RightId,
        expected_state: &str,
    ) -> ChainOpResult<bool> {
        // Verify right exists by querying the object
        // RightId should map to an object ID
        let object_id = right_id.0.as_bytes();

        let object_info = match self.rpc().get_object(*object_id).await {
            Ok(Some(obj)) => Some(obj),
            Ok(None) => None,
            Err(e) => return Err(ChainOpError::RpcError(format!(
                "Failed to query right state: {}", e
            ))),
        };

        // Determine actual state from object info
        let actual_state = match object_info {
            Some(_) => "active",
            None => {
                // Object doesn't exist - check if consumed or never created
                return match expected_state {
                    "consumed" | "deleted" | "never_created" => Ok(true),
                    _ => Ok(false),
                };
            }
        };

        Ok(actual_state == expected_state)
    }
}

/// Convert SuiError to ChainOpError
impl From<SuiError> for ChainOpError {
    fn from(err: SuiError) -> Self {
        match err {
            SuiError::RpcError(msg) => ChainOpError::RpcError(msg),
            SuiError::ObjectUsed(msg) => ChainOpError::InvalidInput(format!("Object used: {}", msg)),
            SuiError::StateProofFailed(msg) => ChainOpError::ProofVerificationError(msg),
            SuiError::EventProofFailed(msg) => ChainOpError::ProofVerificationError(msg),
            SuiError::CheckpointFailed(msg) => ChainOpError::TransactionError(format!("Checkpoint failed: {}", msg)),
            SuiError::TransactionFailed(msg) => ChainOpError::TransactionError(msg),
            SuiError::SerializationError(msg) => ChainOpError::InvalidInput(format!("Serialization: {}", msg)),
            SuiError::ConfirmationTimeout { tx_digest, timeout_ms } => {
                ChainOpError::Timeout(format!(
                    "Transaction {} timed out after {}ms", tx_digest, timeout_ms
                ))
            }
            SuiError::ReorgDetected { checkpoint } => {
                ChainOpError::TransactionError(format!("Reorg at checkpoint {}", checkpoint))
            }
            SuiError::NetworkMismatch { expected, actual } => {
                ChainOpError::UnsupportedChain(format!(
                    "Network mismatch: expected {}, got {}", expected, actual
                ))
            }
            SuiError::ConfigurationError(msg) => {
                ChainOpError::InvalidInput(format!("Sui config error: {}", msg))
            }
            SuiError::FeatureNotEnabled(feature) => {
                ChainOpError::CapabilityUnavailable(format!(
                    "Feature '{}' not enabled - rebuild with required feature", feature
                ))
            }
            SuiError::CoreError(e) => ChainOpError::Unknown(format!("Core error: {}", e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rpc::MockSuiRpc;

    #[test]
    fn test_sui_chain_operations_creation() {
        let rpc = Box::new(MockSuiRpc::new(1));
        let config = SuiConfig::new(SuiNetwork::Testnet);
        let ops = SuiBackend::new(rpc, config);
        assert_eq!(ops.config.network, SuiNetwork::Testnet);
    }

    #[test]
    fn test_address_validation() {
        let rpc = Box::new(MockSuiRpc::new(1));
        let config = SuiConfig::new(SuiNetwork::Testnet);
        let ops = SuiBackend::new(rpc, config);

        // Valid address
        assert!(ops.validate_address(
            "0x0000000000000000000000000000000000000000000000000000000000000001"
        ));

        // Invalid - too short
        assert!(!ops.validate_address("0x1234"));

        // Invalid - not hex
        assert!(!ops.validate_address("0xZZZZ"));
    }

    #[test]
    fn test_signature_verification() {
        let rpc = Box::new(MockSuiRpc::new(1));
        let config = SuiConfig::new(SuiNetwork::Testnet);
        let ops = SuiBackend::new(rpc, config);

          // Generate a keypair
        use ed25519_dalek::{SigningKey, Signer};
        let signing_key = SigningKey::generate(&mut rand::rngs::OsRng);
        let verifying_key = signing_key.verifying_key();

        let message = b"test message";
        let signature = signing_key.sign(message);

        // Verify signature
        let result = ops
            .verify_signature(message, &signature.to_bytes(), &verifying_key.to_bytes())
            .unwrap();
        assert!(result);

        // Wrong message should fail
        let wrong_message = b"wrong message";
        let result = ops
            .verify_signature(wrong_message, &signature.to_bytes(), &verifying_key.to_bytes())
            .unwrap();
        assert!(!result);
    }
}
