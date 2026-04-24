//! Blockchain service types and error definitions.

use csv_adapter_core::Chain;
use serde::{Deserialize, Serialize};

/// Blockchain operation error.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockchainError {
    pub message: String,
    pub chain: Option<Chain>,
    pub code: Option<u32>,
}

impl std::fmt::Display for BlockchainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Blockchain error: {}", self.message)
    }
}

impl std::error::Error for BlockchainError {}

/// Transaction receipt.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransactionReceipt {
    pub tx_hash: String,
    pub block_number: Option<u64>,
    pub gas_used: Option<u64>,
    pub status: TransactionStatus,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum TransactionStatus {
    Pending,
    Confirmed,
    Failed(String),
}

/// Cross-chain transfer status.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum CrossChainStatus {
    Initiated,
    Locked,
    ProofGenerated,
    ProofVerified,
    Minted,
    Completed,
    Failed(String),
}

/// Proof data for cross-chain verification.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CrossChainProof {
    pub source_chain: Chain,
    pub target_chain: Chain,
    pub right_id: String,
    pub lock_tx_hash: String,
    pub proof_data: ProofData,
    pub timestamp: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ProofData {
    Merkle {
        root: String,
        path: Vec<String>,
        leaf: String,
    },
    Mpt {
        account_proof: Vec<String>,
        storage_proof: Vec<String>,
        value: String,
    },
    Checkpoint {
        checkpoint_digest: String,
        transaction_block: u64,
        certificate: String,
    },
    Ledger {
        ledger_version: u64,
        proof: Vec<u8>,
        root_hash: String,
    },
}

/// Contract deployment info.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContractDeployment {
    pub chain: Chain,
    pub contract_address: String,
    pub tx_hash: String,
    pub deployed_at: u64,
    pub contract_type: ContractType,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ContractType {
    Registry,
    Bridge,
    Lock,
}

/// Result of a cross-chain transfer.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CrossChainTransferResult {
    pub transfer_id: String,
    pub lock_tx_hash: String,
    pub mint_tx_hash: String,
    pub proof: Option<CrossChainProof>,
    pub status: CrossChainStatus,
}

/// Map of deployed contracts by chain.
pub type ContractDeployments = std::collections::HashMap<Chain, ContractDeployment>;
