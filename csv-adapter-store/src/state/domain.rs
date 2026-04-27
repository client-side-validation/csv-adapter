//! Domain types: Rights, transfers, contracts, seals, proofs, transactions.
//!
//! These types represent the core CSV (Client-Side Validation) domain model.

use super::core::Chain;
use serde::{Deserialize, Serialize};

/// Status of a Right.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RightStatus {
    /// Right is active and can be used.
    Active,
    /// Right has been transferred to another owner.
    Transferred,
    /// Right has been consumed (seal used).
    Consumed,
}

impl std::fmt::Display for RightStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RightStatus::Active => write!(f, "active"),
            RightStatus::Transferred => write!(f, "transferred"),
            RightStatus::Consumed => write!(f, "consumed"),
        }
    }
}

/// A tracked Right (represents ownership of an asset/claim).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RightRecord {
    /// Right ID (hash).
    pub id: String,
    /// Chain where this Right is anchored.
    pub chain: Chain,
    /// Seal reference (chain-specific bytes, base64 encoded for JSON).
    pub seal_ref: String,
    /// Current owner address.
    pub owner: String,
    /// Value/amount.
    pub value: u64,
    /// Commitment hash (base64).
    pub commitment: String,
    /// Nullifier (if consumed, base64).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nullifier: Option<String>,
    /// Current status.
    pub status: RightStatus,
    /// Creation timestamp (Unix seconds).
    pub created_at: u64,
}

/// Status of a cross-chain transfer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransferStatus {
    /// Transfer initiated (lock transaction created).
    Initiated,
    /// Assets locked on source chain.
    Locked,
    /// Proof being verified.
    Verifying,
    /// Assets being minted on destination chain.
    Minting,
    /// Transfer completed successfully.
    Completed,
    /// Transfer failed.
    Failed,
}

impl std::fmt::Display for TransferStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransferStatus::Initiated => write!(f, "initiated"),
            TransferStatus::Locked => write!(f, "locked"),
            TransferStatus::Verifying => write!(f, "verifying"),
            TransferStatus::Minting => write!(f, "minting"),
            TransferStatus::Completed => write!(f, "completed"),
            TransferStatus::Failed => write!(f, "failed"),
        }
    }
}

/// A cross-chain transfer record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferRecord {
    /// Transfer ID (hash of source seal + dest chain).
    pub id: String,
    /// Source chain.
    pub source_chain: Chain,
    /// Destination chain.
    pub dest_chain: Chain,
    /// Right ID being transferred.
    pub right_id: String,
    /// Sender address on source chain.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sender_address: Option<String>,
    /// Destination owner address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination_address: Option<String>,
    /// Source transaction hash.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_tx_hash: Option<String>,
    /// Source transaction fee.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_fee: Option<u64>,
    /// Destination transaction hash.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dest_tx_hash: Option<String>,
    /// Destination transaction fee.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dest_fee: Option<u64>,
    /// Destination contract address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination_contract: Option<String>,
    /// Inclusion proof (base64 encoded).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proof: Option<String>,
    /// Transfer status.
    pub status: TransferStatus,
    /// Created timestamp.
    pub created_at: u64,
    /// Completed timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<u64>,
}

/// Deployed contract info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractRecord {
    /// Chain where contract is deployed.
    pub chain: Chain,
    /// Contract address.
    pub address: String,
    /// Deployment transaction hash.
    pub tx_hash: String,
    /// Deployment timestamp.
    pub deployed_at: u64,
}

/// Seal record (single-use seal for CSV).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SealRecord {
    /// Seal reference (base64 encoded).
    pub seal_ref: String,
    /// Chain where seal is anchored.
    pub chain: Chain,
    /// Value associated with seal.
    pub value: u64,
    /// Whether seal has been consumed.
    pub consumed: bool,
    /// Creation timestamp.
    pub created_at: u64,
}

/// Proof record (cryptographic proofs for CSV).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofRecord {
    /// Chain where proof is valid.
    pub chain: Chain,
    /// Right ID this proof is for.
    pub right_id: String,
    /// Proof type (e.g., "inclusion", "exclusion", "transition").
    pub proof_type: String,
    /// Whether proof has been verified.
    pub verified: bool,
    /// Proof data (base64 encoded).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proof_data: Option<String>,
}

/// Transaction type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransactionType {
    /// Simple transfer.
    Transfer,
    /// Contract deployment.
    ContractDeployment,
    /// Contract function call.
    ContractCall,
    /// Right creation.
    RightCreation,
    /// Right transfer.
    RightTransfer,
    /// Seal creation.
    SealCreation,
    /// Seal consumption.
    SealConsumption,
    /// Cross-chain lock.
    CrossChainLock,
    /// Cross-chain mint.
    CrossChainMint,
}

/// Transaction status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransactionStatus {
    /// Transaction pending.
    Pending,
    /// Transaction confirmed.
    Confirmed,
    /// Transaction failed.
    Failed,
}

/// A transaction record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionRecord {
    /// Transaction ID.
    pub id: String,
    /// Chain where transaction occurred.
    pub chain: Chain,
    /// Transaction hash.
    pub tx_hash: String,
    /// Transaction type.
    pub tx_type: TransactionType,
    /// Transaction status.
    pub status: TransactionStatus,
    /// Sender address.
    pub from_address: String,
    /// Recipient address (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_address: Option<String>,
    /// Amount transferred (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<u64>,
    /// Fee paid (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee: Option<u64>,
    /// Block number (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_number: Option<u64>,
    /// Confirmations received (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirmations: Option<u64>,
    /// Creation timestamp.
    pub created_at: u64,
    /// Explorer URL (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explorer_url: Option<String>,
}
