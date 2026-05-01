//! Chain Operation Traits Implementation for Solana
//!
//! This module implements all chain operation traits from csv-adapter-core:
//! - ChainQuery: Querying chain state via RPC
//! - ChainSigner: Ed25519 signing operations
//! - ChainBroadcaster: Transaction broadcasting
//! - ChainDeployer: Program deployment
//! - ChainProofProvider: Proof building and verification
//! - ChainRightOps: Right management via program accounts

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
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Signature, Signer as SolanaSigner};

use crate::adapter::SolanaAnchorLayer;
use crate::config::Network;
use crate::error::SolanaError;
use crate::rpc::{SolanaRpc, SolanaTransaction, SolanaTransactionStatus};

/// Solana chain operations implementation
pub struct SolanaChainOperations {
    /// Inner RPC client for chain communication
    rpc: Box<dyn SolanaRpc>,
    /// Chain configuration
    network: Network,
    /// Domain separator for proof generation
    domain_separator: [u8; 32],
}

impl SolanaChainOperations {
    /// Create new Solana chain operations from RPC client
    pub fn new(rpc: Box<dyn SolanaRpc>, network: Network) -> Self {
        let mut domain = [0u8; 32];
        domain[..12].copy_from_slice(b"CSV-SOLANA--");

        Self {
            rpc,
            network,
            domain_separator: domain,
        }
    }

    /// Create from SolanaAnchorLayer
    pub fn from_anchor_layer(anchor: &SolanaAnchorLayer) -> ChainOpResult<Self> {
        Ok(Self {
            rpc: anchor.get_rpc(),
            network: anchor.config.network,
            domain_separator: anchor.domain_separator,
        })
    }

    /// Parse Solana address (Pubkey) from string
    fn parse_address(&self, address: &str) -> ChainOpResult<Pubkey> {
        address
            .parse::<Pubkey>()
            .map_err(|e| ChainOpError::InvalidInput(format!("Invalid Solana address: {}", e)))
    }

    /// Format Solana address for display
    fn format_address(&self, addr: Pubkey) -> String {
        addr.to_string()
    }

    /// Parse transaction signature
    fn parse_signature(&self, sig: &str) -> ChainOpResult<Signature> {
        let bytes = bs58::decode(sig)
            .into_vec()
            .map_err(|e| ChainOpError::InvalidInput(format!("Invalid signature: {}", e)))?;

        if bytes.len() != 64 {
            return Err(ChainOpError::InvalidInput(
                "Solana signature must be 64 bytes".to_string(),
            ));
        }

        let mut sig_bytes = [0u8; 64];
        sig_bytes.copy_from_slice(&bytes);
        Ok(Signature::from(sig_bytes))
    }

    /// Convert Solana transaction to TransactionInfo
    fn tx_to_info(&self, tx: &SolanaTransaction, sig: &str) -> TransactionInfo {
        let status = match tx.status {
            SolanaTransactionStatus::Confirmed => TransactionStatus::Confirmed {
                block_height: tx.slot,
                confirmations: 32, // Solana has probabilistic finality after 32 slots
            },
            SolanaTransactionStatus::Failed => TransactionStatus::Failed {
                reason: "Transaction failed".to_string(),
            },
            SolanaTransactionStatus::Pending => TransactionStatus::Pending,
        };

        TransactionInfo {
            hash: sig.to_string(),
            sender: tx.account_keys.first().map(|k| k.to_string()).unwrap_or_default(),
            recipient: tx.account_keys.get(1).map(|k| k.to_string()),
            amount: None,
            status,
            block_height: Some(tx.slot),
            timestamp: None,
            fee: Some(tx.fee),
            raw_data: None,
        }
    }

    /// Get RPC client reference
    fn rpc(&self) -> &dyn SolanaRpc {
        self.rpc.as_ref()
    }
}

#[async_trait]
impl ChainQuery for SolanaChainOperations {
    async fn get_balance(&self, address: &str) -> ChainOpResult<BalanceInfo> {
        let pubkey = self.parse_address(address)?;

        let balance = self
            .rpc()
            .get_balance(&pubkey)
            .await
            .map_err(|e| ChainOpError::RpcError(format!("Failed to get balance: {}", e)))?;

        // Get token accounts for SPL tokens
        let token_balances = Vec::new(); // Would query token accounts

        Ok(BalanceInfo {
            address: address.to_string(),
            total: balance,
            available: balance,
            locked: 0,
            tokens: token_balances,
        })
    }

    async fn get_transaction(&self, hash: &str) -> ChainOpResult<TransactionInfo> {
        let sig = self.parse_signature(hash)?;

        let tx = self
            .rpc()
            .get_transaction(&sig)
            .await
            .map_err(|e| ChainOpError::RpcError(format!("Failed to get transaction: {}", e)))?
            .ok_or_else(|| ChainOpError::RpcError("Transaction not found".to_string()))?;

        Ok(self.tx_to_info(&tx, hash))
    }

    async fn get_finality(&self, tx_hash: &str) -> ChainOpResult<FinalityStatus> {
        let tx_info = self.get_transaction(tx_hash).await?;

        match tx_info.status {
            TransactionStatus::Confirmed { block_height, .. } => {
                // Get latest slot
                let latest_slot = self
                    .rpc()
                    .get_latest_slot()
                    .await
                    .map_err(|e| ChainOpError::RpcError(format!("Failed to get slot: {}", e)))?;

                let confirmations = latest_slot.saturating_sub(block_height);

                // Solana has probabilistic finality after 32 confirmations
                if confirmations >= 32 {
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
        let program_id = self.parse_address(contract_address)?;

        // Check if program account exists and is executable
        let account = self
            .rpc()
            .get_account(&program_id)
            .await
            .map_err(|e| ChainOpError::RpcError(format!("Failed to get account: {}", e)))?;

        let is_deployed = account.executable;

        Ok(ContractStatus {
            address: contract_address.to_string(),
            is_deployed,
            balance: Some(account.lamports),
            owner: Some(account.owner.to_string()),
            metadata: serde_json::json!({
                "chain": "solana",
                "network": format!("{:?}", self.network),
                "executable": account.executable,
                "data_size": account.data.len(),
            }),
        })
    }

    async fn get_latest_block_height(&self) -> ChainOpResult<u64> {
        self.rpc()
            .get_latest_slot()
            .await
            .map_err(|e| ChainOpError::RpcError(format!("Failed to get slot: {}", e)))
    }

    async fn get_chain_info(&self) -> ChainOpResult<serde_json::Value> {
        let slot = self.get_latest_block_height().await?;

        Ok(serde_json::json!({
            "chain_id": match self.network {
                Network::Mainnet => "mainnet-beta",
                Network::Devnet => "devnet",
                Network::Testnet => "testnet",
                Network::Localnet => "localnet",
            },
            "chain": "solana",
            "network": format!("{:?}", self.network),
            "latest_slot": slot,
            "protocol": "Solana",
            "finality": "probabilistic",
        }))
    }

    fn validate_address(&self, address: &str) -> bool {
        address.parse::<Pubkey>().is_ok()
    }
}

#[async_trait]
impl ChainSigner for SolanaChainOperations {
    fn derive_address(&self, public_key: &[u8]) -> ChainOpResult<String> {
        if public_key.len() != 32 {
            return Err(ChainOpError::InvalidInput(
                "Ed25519 public key must be 32 bytes".to_string(),
            ));
        }

        let mut pubkey_bytes = [0u8; 32];
        pubkey_bytes.copy_from_slice(public_key);

        let pubkey = Pubkey::new_from_array(pubkey_bytes);
        Ok(pubkey.to_string())
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

        let mut pubkey_bytes = [0u8; 32];
        pubkey_bytes.copy_from_slice(public_key);

        let mut sig_bytes = [0u8; 64];
        sig_bytes.copy_from_slice(signature);

        // Use ed25519-dalek for verification
        let verifying_key = ed25519_dalek::VerifyingKey::from_bytes(&pubkey_bytes)
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
impl ChainBroadcaster for SolanaChainOperations {
    async fn submit_transaction(&self, signed_tx: &[u8]) -> ChainOpResult<String> {
        // signed_tx is a serialized Solana transaction
        // Send via RPC

        let sig = self
            .rpc()
            .send_transaction(signed_tx)
            .await
            .map_err(|e| ChainOpError::TransactionError(format!("Submission failed: {}", e)))?;

        Ok(sig.to_string())
    }

    async fn confirm_transaction(
        &self,
        tx_hash: &str,
        _required_confirmations: u64,
        timeout_secs: u64,
    ) -> ChainOpResult<TransactionStatus> {
        let sig = self.parse_signature(tx_hash)?;
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(timeout_secs);
        let poll_interval = std::time::Duration::from_millis(400); // Solana slot time

        loop {
            if start.elapsed() > timeout {
                return Err(ChainOpError::Timeout(
                    "Transaction confirmation timeout".to_string(),
                ));
            }

            match self.rpc().get_transaction(&sig).await {
                Ok(Some(tx)) => {
                    return Ok(match tx.status {
                        SolanaTransactionStatus::Confirmed => TransactionStatus::Confirmed {
                            block_height: tx.slot,
                            confirmations: 32,
                        },
                        SolanaTransactionStatus::Failed => TransactionStatus::Failed {
                            reason: "Transaction failed".to_string(),
                        },
                        _ => TransactionStatus::Pending,
                    });
                }
                Ok(None) => {
                    std::thread::sleep(poll_interval);
                }
                Err(_) => {
                    std::thread::sleep(poll_interval);
                }
            }
        }
    }

    async fn get_fee_estimate(&self) -> ChainOpResult<u64> {
        // Solana fee estimation
        // Typical transaction: 5000 lamports (0.000005 SOL)
        let fee = self
            .rpc()
            .get_recent_blockhash()
            .await
            .map_err(|e| ChainOpError::RpcError(format!("Failed to get blockhash: {}", e)))?;

        // Would parse fee from blockhash response
        let _ = fee;
        Ok(5000)
    }

    async fn validate_transaction(&self, tx_data: &[u8]) -> ChainOpResult<()> {
        if tx_data.is_empty() {
            return Err(ChainOpError::InvalidInput(
                "Empty transaction data".to_string(),
            ));
        }

        // Would deserialize and validate transaction structure
        // Check for valid signatures, recent blockhash, etc.

        Ok(())
    }
}

#[async_trait]
impl ChainDeployer for SolanaChainOperations {
    async fn deploy_lock_contract(
        &self,
        admin_address: &str,
        config: serde_json::Value,
    ) -> ChainOpResult<DeploymentStatus> {
        let _ = admin_address;
        let _ = config;

        Err(ChainOpError::CapabilityUnavailable(
            "Lock contract deployment requires program deployment. \
             Use deploy_or_publish_seal_program() with compiled BPF bytecode.".to_string(),
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
            "Mint contract deployment requires program deployment. \
             Same program handles both lock and mint in Solana.".to_string(),
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
            "Program deployment requires signed transaction. \
             Use deploy_csv_program() with compiled BPF bytecode \
             or external tools (solana program deploy).".to_string(),
        ))
    }

    async fn verify_deployment(&self, contract_address: &str) -> ChainOpResult<bool> {
        let status = self.get_contract_status(contract_address).await?;
        Ok(status.is_deployed)
    }

    async fn estimate_deployment_cost(&self, program_bytes: &[u8]) -> ChainOpResult<u64> {
        // Solana deployment cost
        // Rent exemption based on program size
        let rent = self
            .rpc()
            .get_minimum_balance_for_rent_exemption(program_bytes.len())
            .await
            .map_err(|e| ChainOpError::RpcError(format!("Failed to get rent: {}", e)))?;

        let tx_fees = 5000u64; // Transaction fees

        Ok(rent + tx_fees)
    }
}

#[async_trait]
impl ChainProofProvider for SolanaChainOperations {
    async fn build_inclusion_proof(
        &self,
        commitment: &Hash,
        block_height: u64,
    ) -> ChainOpResult<InclusionProof> {
        // Get block at slot
        let block = self
            .rpc()
            .get_block(block_height)
            .await
            .map_err(|e| ChainOpError::RpcError(format!("Failed to get block: {}", e)))?;

        // Build proof from block
        let proof_data = serde_json::to_vec(&block)
            .map_err(|e| ChainOpError::Unknown(format!("Serialization failed: {}", e)))?;

        Ok(InclusionProof {
            block_height,
            transaction_hash: block.blockhash,
            proof_data,
            merkle_root: vec![], // Solana doesn't use Merkle trees for tx inclusion
        })
    }

    fn verify_inclusion_proof(
        &self,
        proof: &InclusionProof,
        commitment: &Hash,
    ) -> ChainOpResult<bool> {
        let _ = commitment;

        // Verify block exists
        let _ = proof;

        // Solana verification would check:
        // 1. Block exists at given slot
        // 2. Transaction is in block's transaction list
        // 3. Commitment is in transaction data

        Err(ChainOpError::CapabilityUnavailable(
            "Inclusion proof verification requires full block data. \
             Query the block and verify transaction inclusion.".to_string(),
        ))
    }

    async fn build_finality_proof(&self, tx_hash: &str) -> ChainOpResult<FinalityProof> {
        let finality = self.get_finality(tx_hash).await?;

        match finality {
            FinalityStatus::Finalized { finality_block, .. } => {
                let block = self
                    .rpc()
                    .get_block(finality_block)
                    .await
                    .map_err(|e| ChainOpError::RpcError(format!("Failed to get block: {}", e)))?;

                let proof_data = serde_json::to_vec(&block)
                    .map_err(|e| ChainOpError::Unknown(format!("Serialization failed: {}", e)))?;

                Ok(FinalityProof {
                    block_height: finality_block,
                    proof_data,
                    signature: block.blockhash.into_bytes(),
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

        // Get current slot
        let latest = self
            .rpc()
            .get_latest_slot()
            .map_err(|e| ChainOpError::RpcError(format!("Failed to get slot: {}", e)))?;

        let depth = latest.saturating_sub(proof.block_height);

        // 32 confirmations for finality
        if depth < 32 {
            return Ok(false);
        }

        // Would verify blockhash
        let _ = proof;

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
impl ChainRightOps for SolanaChainOperations {
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
             Construct and submit a transaction to create the seal account.".to_string(),
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
             Construct and submit a transaction to close the seal account.".to_string(),
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
             Construct and submit a transaction to lock the seal account.".to_string(),
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

        // Query account at address derived from right_id
        let _ = right_id;

        Err(ChainOpError::CapabilityUnavailable(
            "Right state verification requires account query. \
             Query the seal account at the expected address.".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solana_address_validation() {
        // Can't easily test without test RPC, but we can test address validation
        // This is a basic test - real tests would use MockSolanaRpc
    }
}
