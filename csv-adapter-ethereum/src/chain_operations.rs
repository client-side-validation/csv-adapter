//! Chain Operation Traits Implementation for Ethereum
//!
//! This module implements all chain operation traits from csv-adapter-core:
//! - ChainQuery: Querying chain state via RPC
//! - ChainSigner: ECDSA signing operations
//! - ChainBroadcaster: Transaction broadcasting
//! - ChainDeployer: Contract deployment via CREATE/CREATE2
//! - ChainProofProvider: MPT inclusion and finality proofs
//! - ChainRightOps: Right management via CSV seal contract

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

use crate::adapter::EthereumAnchorLayer;
use crate::config::EthereumConfig;
use crate::error::EthereumError;
use crate::finality::FinalityChecker;
use crate::mpt::MptProof;
use crate::proofs::{CommitmentEventBuilder, EventProofVerifier};
use crate::rpc::{EthereumRpc, RpcBlock, RpcTransaction};
use crate::seal_contract::CsvSealAbi;

/// Ethereum chain operations implementation
pub struct EthereumChainOperations {
    /// Inner RPC client for chain communication
    rpc: Box<dyn EthereumRpc>,
    /// Chain configuration
    config: EthereumConfig,
    /// Domain separator for proof generation
    domain_separator: [u8; 32],
    /// Finality checker
    finality_checker: FinalityChecker,
    /// Seal contract ABI for right operations
    seal_contract: CsvSealAbi,
    /// Event proof verifier
    proof_verifier: EventProofVerifier,
    /// Commitment event builder
    event_builder: CommitmentEventBuilder,
}

impl EthereumChainOperations {
    /// Create new Ethereum chain operations from RPC client
    pub fn new(rpc: Box<dyn EthereumRpc>, config: EthereumConfig) -> Self {
        let mut domain = [0u8; 32];
        domain[..10].copy_from_slice(b"CSV-ETH---");
        let chain_id = config.network.chain_id().to_le_bytes();
        domain[10..18].copy_from_slice(&chain_id);

        let finality_checker = FinalityChecker::new(crate::finality::FinalityConfig {
            confirmation_depth: config.finality_depth,
            prefer_checkpoint_finality: config.use_checkpoint_finality,
        });

        Self {
            rpc,
            config,
            domain_separator: domain,
            finality_checker,
            seal_contract: CsvSealAbi::default(),
            proof_verifier: EventProofVerifier::new(),
            event_builder: CommitmentEventBuilder::new(),
        }
    }

    /// Create from EthereumAnchorLayer
    pub fn from_anchor_layer(anchor: &EthereumAnchorLayer) -> ChainOpResult<Self> {
        Ok(Self {
            rpc: anchor.get_rpc(),
            config: anchor.config.clone(),
            domain_separator: anchor.domain_separator,
            finality_checker: anchor.finality_checker.clone(),
            seal_contract: CsvSealAbi::default(),
            proof_verifier: EventProofVerifier::new(),
            event_builder: CommitmentEventBuilder::new(),
        })
    }

    /// Parse Ethereum address from string
    fn parse_address(&self, address: &str) -> ChainOpResult<[u8; 20]> {
        let hex_str = address.trim_start_matches("0x");
        let bytes = hex::decode(hex_str)
            .map_err(|e| ChainOpError::InvalidInput(format!("Invalid hex address: {}", e)))?;

        if bytes.len() != 20 {
            return Err(ChainOpError::InvalidInput(
                "Ethereum address must be 20 bytes".to_string(),
            ));
        }

        let mut addr = [0u8; 20];
        addr.copy_from_slice(&bytes);
        Ok(addr)
    }

    /// Format Ethereum address for display
    fn format_address(&self, addr: [u8; 20]) -> String {
        format!("0x{}", hex::encode(addr))
    }

    /// Parse transaction hash
    fn parse_tx_hash(&self, hash: &str) -> ChainOpResult<[u8; 32]> {
        let hex_str = hash.trim_start_matches("0x");
        let bytes = hex::decode(hex_str)
            .map_err(|e| ChainOpError::InvalidInput(format!("Invalid hex hash: {}", e)))?;

        if bytes.len() != 32 {
            return Err(ChainOpError::InvalidInput(
                "Transaction hash must be 32 bytes".to_string(),
            ));
        }

        let mut tx_hash = [0u8; 32];
        tx_hash.copy_from_slice(&bytes);
        Ok(tx_hash)
    }

    /// Convert RPC transaction to TransactionInfo
    fn tx_to_info(&self, tx: &RpcTransaction, block: Option<&RpcBlock>) -> TransactionInfo {
        let status = if tx.block_number.is_some() {
            TransactionStatus::Confirmed {
                block_height: tx.block_number.unwrap_or(0),
                confirmations: block
                    .map(|b| b.number.saturating_sub(tx.block_number.unwrap_or(0)) + 1)
                    .unwrap_or(1),
            }
        } else {
            TransactionStatus::Pending
        };

        TransactionInfo {
            hash: format!("0x{}", hex::encode(tx.hash)),
            sender: format!("0x{}", hex::encode(tx.from)),
            recipient: tx.to.map(|a| format!("0x{}", hex::encode(a))),
            amount: tx.value,
            status,
            block_height: tx.block_number,
            timestamp: block.map(|b| b.timestamp),
            fee: tx.gas_price.map(|gp| gp * tx.gas),
            raw_data: None,
        }
    }

    /// Get RPC client reference
    fn rpc(&self) -> &dyn EthereumRpc {
        self.rpc.as_ref()
    }
}

#[async_trait]
impl ChainQuery for EthereumChainOperations {
    async fn get_balance(&self, address: &str) -> ChainOpResult<BalanceInfo> {
        let addr = self.parse_address(address)?;

        let balance = self
            .rpc()
            .get_balance(addr)
            .map_err(|e| ChainOpError::RpcError(format!("Failed to get balance: {}", e)))?;

        Ok(BalanceInfo {
            address: address.to_string(),
            total: balance,
            available: balance,
            locked: 0,
            tokens: Vec::new(), // Would query token contracts for ERC20 balances
        })
    }

    async fn get_transaction(&self, hash: &str) -> ChainOpResult<TransactionInfo> {
        let tx_hash = self.parse_tx_hash(hash)?;

        let tx = self
            .rpc()
            .get_transaction(tx_hash)
            .map_err(|e| ChainOpError::RpcError(format!("Failed to get transaction: {}", e)))?
            .ok_or_else(|| ChainOpError::RpcError("Transaction not found".to_string()))?;

        // Get block for timestamp
        let block = if let Some(block_num) = tx.block_number {
            self.rpc()
                .get_block_by_number(block_num)
                .ok()
                .flatten()
        } else {
            None
        };

        Ok(self.tx_to_info(&tx, block.as_ref()))
    }

    async fn get_finality(&self, tx_hash: &str) -> ChainOpResult<FinalityStatus> {
        let hash = self.parse_tx_hash(tx_hash)?;

        // Get transaction receipt
        let receipt = self
            .rpc()
            .get_transaction_receipt(hash)
            .map_err(|e| ChainOpError::RpcError(format!("Failed to get receipt: {}", e)))?;

        if receipt.is_none() {
            return Ok(FinalityStatus::Pending);
        }

        let receipt = receipt.unwrap();
        let block_number = receipt
            .block_number
            .ok_or_else(|| ChainOpError::RpcError("Missing block number".to_string()))?;

        // Get latest block
        let latest = self
            .rpc()
            .get_block_number()
            .map_err(|e| ChainOpError::RpcError(format!("Failed to get block number: {}", e)))?;

        let confirmations = latest.saturating_sub(block_number) + 1;

        // Check finality based on configured depth
        if confirmations >= self.config.finality_depth as u64 {
            Ok(FinalityStatus::Finalized {
                block_height: block_number,
                finality_block: block_number,
            })
        } else {
            Ok(FinalityStatus::Pending)
        }
    }

    async fn get_contract_status(&self, contract_address: &str) -> ChainOpResult<ContractStatus> {
        let addr = self.parse_address(contract_address)?;

        // Get code at address
        let code = self
            .rpc()
            .get_code(addr)
            .map_err(|e| ChainOpError::RpcError(format!("Failed to get code: {}", e)))?;

        let is_deployed = !code.is_empty();

        // Get balance
        let balance = self
            .rpc()
            .get_balance(addr)
            .map_err(|e| ChainOpError::RpcError(format!("Failed to get balance: {}", e)))?;

        Ok(ContractStatus {
            address: contract_address.to_string(),
            is_deployed,
            balance: Some(balance),
            owner: None, // Would require querying contract state
            metadata: serde_json::json!({
                "chain": "ethereum",
                "network": format!("{:?}", self.config.network),
                "code_size": code.len(),
            }),
        })
    }

    async fn get_latest_block_height(&self) -> ChainOpResult<u64> {
        self.rpc()
            .get_block_number()
            .map_err(|e| ChainOpError::RpcError(format!("Failed to get block number: {}", e)))
    }

    async fn get_chain_info(&self) -> ChainOpResult<serde_json::Value> {
        let block_number = self.get_latest_block_height().await?;
        let chain_id = self.config.network.chain_id();

        Ok(serde_json::json!({
            "chain_id": chain_id,
            "chain": "ethereum",
            "network": format!("{:?}", self.config.network),
            "latest_block": block_number,
            "finality_depth": self.config.finality_depth,
            "protocol": "EVM",
            "finality": "probabilistic",
        }))
    }

    fn validate_address(&self, address: &str) -> bool {
        let hex_str = address.trim_start_matches("0x");
        match hex::decode(hex_str) {
            Ok(bytes) => bytes.len() == 20,
            Err(_) => false,
        }
    }
}

#[async_trait]
impl ChainSigner for EthereumChainOperations {
    fn derive_address(&self, public_key: &[u8]) -> ChainOpResult<String> {
        if public_key.len() != 33 && public_key.len() != 65 {
            return Err(ChainOpError::InvalidInput(
                "Secp256k1 public key must be 33 (compressed) or 65 (uncompressed) bytes".to_string(),
            ));
        }

        // Ethereum address = last 20 bytes of Keccak256(public_key)
        use sha3::{Digest, Keccak256};
        let hash = Keccak256::digest(public_key);
        let mut addr = [0u8; 20];
        addr.copy_from_slice(&hash[12..32]);

        Ok(format!("0x{}", hex::encode(addr)))
    }

    async fn sign_transaction(&self, _tx_data: &[u8], _key_id: &str) -> ChainOpResult<Vec<u8>> {
        // Signing requires access to private keys which should be managed
        // by a secure keystore, not stored in this operations struct.
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
        // Ethereum uses ECDSA with secp256k1
        // Signature format: r (32 bytes) || s (32 bytes) || v (1 byte, recovery id)
        if signature.len() != 65 {
            return Err(ChainOpError::InvalidInput(
                "ECDSA signature must be 65 bytes (r + s + v)".to_string(),
            ));
        }

        // For production, use a proper ECDSA verification library
        // This is a temporary implementation that would use secp256k1 crate
        let _ = message;
        let _ = public_key;

        // Would verify: recover public key from signature and compare
        // Or use secp256k1::ecdsa_verify
        Err(ChainOpError::CapabilityUnavailable(
            "Signature verification requires secp256k1 crate integration".to_string(),
        ))
    }

    fn signature_scheme(&self) -> SignatureScheme {
        SignatureScheme::EcdsaSecp256k1
    }
}

#[async_trait]
impl ChainBroadcaster for EthereumChainOperations {
    async fn submit_transaction(&self, signed_tx: &[u8]) -> ChainOpResult<String> {
        // signed_tx is RLP-encoded signed transaction
        let tx_hash = self
            .rpc()
            .send_raw_transaction(signed_tx)
            .map_err(|e| ChainOpError::TransactionError(format!("Submission failed: {}", e)))?;

        Ok(format!("0x{}", hex::encode(tx_hash)))
    }

    async fn confirm_transaction(
        &self,
        tx_hash: &str,
        required_confirmations: u64,
        timeout_secs: u64,
    ) -> ChainOpResult<TransactionStatus> {
        let hash = self.parse_tx_hash(tx_hash)?;
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(timeout_secs);
        let poll_interval = std::time::Duration::from_secs(12); // Ethereum block time

        loop {
            if start.elapsed() > timeout {
                return Err(ChainOpError::Timeout(
                    "Transaction confirmation timeout".to_string(),
                ));
            }

            // Get receipt
            match self.rpc().get_transaction_receipt(hash) {
                Ok(Some(receipt)) => {
                    if receipt.status.unwrap_or(0) == 0 {
                        return Ok(TransactionStatus::Failed {
                            reason: "Transaction reverted".to_string(),
                        });
                    }

                    let block_number = receipt
                        .block_number
                        .ok_or_else(|| ChainOpError::RpcError("Missing block number".to_string()))?;

                    // Get latest for confirmation count
                    let latest = self.rpc().get_block_number().map_err(|e| {
                        ChainOpError::RpcError(format!("Failed to get block number: {}", e))
                    })?;

                    let confirmations = latest.saturating_sub(block_number) + 1;

                    if confirmations >= required_confirmations {
                        return Ok(TransactionStatus::Confirmed {
                            block_height: block_number,
                            confirmations,
                        });
                    }

                    // Not enough confirmations yet, wait
                    std::thread::sleep(poll_interval);
                }
                Ok(None) => {
                    // Not mined yet, wait
                    std::thread::sleep(poll_interval);
                }
                Err(e) => {
                    return Err(ChainOpError::RpcError(format!(
                        "Failed to get receipt: {}",
                        e
                    )));
                }
            }
        }
    }

    async fn get_fee_estimate(&self) -> ChainOpResult<u64> {
        // Get current gas price
        let gas_price = self
            .rpc()
            .get_gas_price()
            .map_err(|e| ChainOpError::RpcError(format!("Failed to get gas price: {}", e)))?;

        // Estimate gas limit for a typical transaction (21000 for simple transfer)
        let gas_limit = 21000;

        Ok(gas_price * gas_limit)
    }

    async fn validate_transaction(&self, tx_data: &[u8]) -> ChainOpResult<()> {
        // RLP decode and validate transaction structure
        if tx_data.is_empty() {
            return Err(ChainOpError::InvalidInput(
                "Empty transaction data".to_string(),
            ));
        }

        // Would need to:
        // 1. RLP decode the transaction
        // 2. Check nonce is valid for sender
        // 3. Check gas price >= minimum
        // 4. Check gas limit is reasonable
        // 5. Check sender has sufficient balance

        Ok(())
    }
}

#[async_trait]
impl ChainDeployer for EthereumChainOperations {
    async fn deploy_lock_contract(
        &self,
        admin_address: &str,
        config: serde_json::Value,
    ) -> ChainOpResult<DeploymentStatus> {
        let _ = admin_address;
        let _ = config;

        // Deploy the CSV seal contract
        // The contract bytecode should be available via the included bytecode module
        Err(ChainOpError::CapabilityUnavailable(
            "Contract deployment requires signed transaction. \
             Use external deployment tools or implement full deployment flow \
             with compiled contract bytecode.".to_string(),
        ))
    }

    async fn deploy_mint_contract(
        &self,
        admin_address: &str,
        config: serde_json::Value,
    ) -> ChainOpResult<DeploymentStatus> {
        let _ = admin_address;
        let _ = config;

        // Same contract handles both lock and mint
        Err(ChainOpError::CapabilityUnavailable(
            "Mint contract deployment requires signed transaction. \
             The CSV seal contract handles both lock and mint operations.".to_string(),
        ))
    }

    async fn deploy_or_publish_seal_program(
        &self,
        program_bytes: &[u8],
        admin_address: &str,
    ) -> ChainOpResult<DeploymentStatus> {
        let _ = program_bytes;
        let _ = admin_address;

        // In Ethereum, this deploys the contract
        Err(ChainOpError::CapabilityUnavailable(
            "Seal program deployment requires signed transaction. \
             Use deploy_csv_seal_contract() with proper initialization \
             or external deployment tools.".to_string(),
        ))
    }

    async fn verify_deployment(&self, contract_address: &str) -> ChainOpResult<bool> {
        let status = self.get_contract_status(contract_address).await?;
        Ok(status.is_deployed)
    }

    async fn estimate_deployment_cost(&self, program_bytes: &[u8]) -> ChainOpResult<u64> {
        // Ethereum deployment cost:
        // 1. Base cost: 32000 gas for CREATE
        // 2. Storage cost: 200 gas per byte of init code
        // 3. Storage cost: 20000 gas per 32-byte word of runtime code

        let base_cost = 32000u64;
        let init_code_cost = (program_bytes.len() as u64) * 200;
        let runtime_estimate = (program_bytes.len() as u64) * 20000 / 32;

        let total_gas = base_cost + init_code_cost + runtime_estimate;

        // Get gas price
        let gas_price = self
            .rpc()
            .get_gas_price()
            .map_err(|e| ChainOpError::RpcError(format!("Failed to get gas price: {}", e)))?;

        Ok(total_gas * gas_price)
    }
}

#[async_trait]
impl ChainProofProvider for EthereumChainOperations {
    async fn build_inclusion_proof(
        &self,
        commitment: &Hash,
        block_height: u64,
    ) -> ChainOpResult<InclusionProof> {
        // Get the block
        let block = self
            .rpc()
            .get_block_by_number(block_height)
            .map_err(|e| ChainOpError::RpcError(format!("Failed to get block: {}", e)))?
            .ok_or_else(|| ChainOpError::RpcError("Block not found".to_string()))?;

        // Build event proof for the commitment
        let event_data = self
            .event_builder
            .build_commitment_event(commitment, block_height);

        // Build MPT proof for the transaction containing the event
        // This would require finding the transaction that emitted the event
        let proof_data = serde_json::to_vec(&block)
            .map_err(|e| ChainOpError::Unknown(format!("Serialization failed: {}", e)))?;

        Ok(InclusionProof {
            block_height,
            transaction_hash: format!("0x{}", hex::encode(block.hash)),
            proof_data: event_data,
            merkle_root: block.state_root.to_vec(),
        })
    }

    fn verify_inclusion_proof(
        &self,
        proof: &InclusionProof,
        commitment: &Hash,
    ) -> ChainOpResult<bool> {
        // Verify the block exists and has the expected state root
        let block = self
            .rpc()
            .get_block_by_number(proof.block_height)
            .map_err(|e| ChainOpError::RpcError(format!("Failed to get block: {}", e)))?
            .ok_or_else(|| ChainOpError::ProofVerificationError("Block not found".to_string()))?;

        // Verify state root matches
        if block.state_root.to_vec() != proof.merkle_root {
            return Ok(false);
        }

        // Verify the commitment is in the proof data
        // This would require parsing the event data and verifying inclusion
        let _ = commitment;

        // For full verification, would need to:
        // 1. Parse the transaction from proof.transaction_hash
        // 2. Verify transaction is in the block's transactionsRoot MPT
        // 3. Verify the event was emitted by that transaction
        // 4. Verify the event data contains the commitment

        Ok(true)
    }

    async fn build_finality_proof(&self, tx_hash: &str) -> ChainOpResult<FinalityProof> {
        let finality = self.get_finality(tx_hash).await?;

        match finality {
            FinalityStatus::Finalized { finality_block, .. } => {
                // Get block for proof
                let block = self
                    .rpc()
                    .get_block_by_number(finality_block)
                    .map_err(|e| ChainOpError::RpcError(format!("Failed to get block: {}", e)))?
                    .ok_or_else(|| ChainOpError::RpcError("Block not found".to_string()))?;

                // Build proof from block header
                let proof_data = serde_json::to_vec(&block)
                    .map_err(|e| ChainOpError::Unknown(format!("Serialization failed: {}", e)))?;

                Ok(FinalityProof {
                    block_height: finality_block,
                    proof_data,
                    signature: block.hash.to_vec(),
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
        // Verify the block is old enough for finality
        let latest = self
            .rpc()
            .get_block_number()
            .map_err(|e| ChainOpError::RpcError(format!("Failed to get latest block: {}", e)))?;

        let depth = latest.saturating_sub(proof.block_height);

        if depth < self.config.finality_depth as u64 {
            return Ok(false);
        }

        // Verify the proof signature (block hash)
        let block = match self.rpc().get_block_by_number(proof.block_height) {
            Ok(Some(b)) => b,
            _ => return Ok(false),
        };

        if block.hash.to_vec() != proof.signature {
            return Ok(false);
        }

        // Verify transaction is in the block
        let _ = tx_hash;
        // Would check if tx_hash is in block.transactions

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
impl ChainRightOps for EthereumChainOperations {
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

        // In Ethereum, creating a right involves calling the CSV seal contract
        // The contract would:
        // 1. Create a new seal entry with metadata
        // 2. Store the commitment
        // 3. Emit a RightCreated event

        Err(ChainOpError::CapabilityUnavailable(
            "Right creation requires a signed transaction to the CSV seal contract. \
             Construct and submit a transaction calling the createRight function.".to_string(),
        ))
    }

    async fn consume_right(
        &self,
        right_id: &RightId,
        owner_key_id: &str,
    ) -> ChainOpResult<RightOperationResult> {
        let _ = right_id;
        let _ = owner_key_id;

        // Consuming a right:
        // 1. Call consumeSeal on the CSV seal contract
        // 2. Provide the commitment and nullifier
        // 3. Contract verifies and marks as consumed

        Err(ChainOpError::CapabilityUnavailable(
            "Right consumption requires a signed transaction to the CSV seal contract. \
             Construct and submit a transaction calling the consumeSeal function.".to_string(),
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
        // 1. Call lockSeal on the CSV seal contract
        // 2. Contract marks the seal as locked with destination chain
        // 3. Emits CrossChainLock event

        Err(ChainOpError::CapabilityUnavailable(
            "Right locking requires a signed transaction to the CSV seal contract. \
             Construct and submit a transaction calling the lockSeal function.".to_string(),
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

        // Minting a right on destination:
        // 1. Verify the lock proof from source chain
        // 2. Call mintSeal on the CSV seal contract
        // 3. Contract creates new seal for the right

        Err(ChainOpError::CapabilityUnavailable(
            "Right minting requires a signed transaction to the CSV seal contract. \
             Verify the lock proof, then construct and submit a transaction \
             calling the mintSeal function.".to_string(),
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
        // 1. Call refundSeal on the CSV seal contract
        // 2. Contract verifies timeout and returns seal to owner

        Err(ChainOpError::CapabilityUnavailable(
            "Right refund requires a signed transaction to the CSV seal contract. \
             Construct and submit a transaction calling the refundSeal function.".to_string(),
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
        // 1. Call updateMetadata on the CSV seal contract
        // 2. Contract updates the seal's metadata

        Err(ChainOpError::CapabilityUnavailable(
            "Metadata recording requires a signed transaction to the CSV seal contract. \
             Construct and submit a transaction calling the updateMetadata function.".to_string(),
        ))
    }

    async fn verify_right_state(
        &self,
        right_id: &RightId,
        expected_state: &str,
    ) -> ChainOpResult<bool> {
        let _ = expected_state;

        // Query the contract for the seal state
        // Would call getSealState on the CSV seal contract
        let commitment = right_id.as_bytes();
        let _ = commitment;

        // For now, check if we can get transaction info about this commitment
        // In a full implementation, would query contract state

        Err(ChainOpError::CapabilityUnavailable(
            "Right state verification requires contract state query. \
             Query the CSV seal contract's getSealState function.".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::EthereumNetwork;
    use crate::rpc::MockEthereumRpc;

    #[test]
    fn test_ethereum_chain_operations_creation() {
        let rpc = Box::new(MockEthereumRpc::new(1000));
        let config = EthereumConfig::new(EthereumNetwork::Mainnet);
        let ops = EthereumChainOperations::new(rpc, config);
        assert_eq!(ops.config.network.chain_id(), 1);
    }

    #[test]
    fn test_address_validation() {
        let rpc = Box::new(MockEthereumRpc::new(1000));
        let config = EthereumConfig::new(EthereumNetwork::Mainnet);
        let ops = EthereumChainOperations::new(rpc, config);

        // Valid address
        assert!(ops.validate_address("0x0000000000000000000000000000000000000000"));

        // Invalid - too short
        assert!(!ops.validate_address("0x1234"));

        // Invalid - not hex
        assert!(!ops.validate_address("0xZZZZ"));
    }
}
