/// Core explorer types for the CSV Explorer.
///
/// This module defines all the data types used across the explorer,
/// including rights, transfers, seals, contracts, and chain information.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// Network type for chain configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Network {
    Mainnet,
    Testnet,
    Devnet,
}

impl std::fmt::Display for Network {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Network::Mainnet => write!(f, "mainnet"),
            Network::Testnet => write!(f, "testnet"),
            Network::Devnet => write!(f, "devnet"),
        }
    }
}

/// Status of a chain indexer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChainStatus {
    /// Chain is actively syncing.
    Syncing,
    /// Chain is fully synced and caught up.
    Synced,
    /// Chain indexer is stopped.
    Stopped,
    /// Chain indexer encountered an error.
    Error,
}

impl std::fmt::Display for ChainStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChainStatus::Syncing => write!(f, "syncing"),
            ChainStatus::Synced => write!(f, "synced"),
            ChainStatus::Stopped => write!(f, "stopped"),
            ChainStatus::Error => write!(f, "error"),
        }
    }
}

/// Status of a right record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RightStatus {
    /// Right is currently active.
    Active,
    /// Right has been spent/consumed.
    Spent,
    /// Right is pending confirmation.
    Pending,
}

impl std::fmt::Display for RightStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RightStatus::Active => write!(f, "active"),
            RightStatus::Spent => write!(f, "spent"),
            RightStatus::Pending => write!(f, "pending"),
        }
    }
}

/// Status of a cross-chain transfer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransferStatus {
    /// Transfer is pending (lock submitted, waiting for proof).
    Pending,
    /// Transfer is in progress (proof generated, minting on destination).
    InProgress,
    /// Transfer completed successfully.
    Completed,
    /// Transfer failed.
    Failed,
}

impl std::fmt::Display for TransferStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransferStatus::Pending => write!(f, "pending"),
            TransferStatus::InProgress => write!(f, "in_progress"),
            TransferStatus::Completed => write!(f, "completed"),
            TransferStatus::Failed => write!(f, "failed"),
        }
    }
}

/// Type of seal on a chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SealType {
    /// UTXO-based seal (Bitcoin).
    Utxo,
    /// Object-based seal (Sui).
    Object,
    /// Resource-based seal (Aptos).
    Resource,
    /// Nullifier-based seal.
    Nullifier,
    /// Account-based seal (Solana).
    Account,
}

impl std::fmt::Display for SealType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SealType::Utxo => write!(f, "utxo"),
            SealType::Object => write!(f, "object"),
            SealType::Resource => write!(f, "resource"),
            SealType::Nullifier => write!(f, "nullifier"),
            SealType::Account => write!(f, "account"),
        }
    }
}

/// Status of a seal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SealStatus {
    /// Seal is available/unused.
    Available,
    /// Seal has been consumed.
    Consumed,
}

impl std::fmt::Display for SealStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SealStatus::Available => write!(f, "available"),
            SealStatus::Consumed => write!(f, "consumed"),
        }
    }
}

/// Type of CSV contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContractType {
    /// Nullifier registry contract.
    NullifierRegistry,
    /// State commitment contract.
    StateCommitment,
    /// Right registry contract.
    RightRegistry,
    /// Bridge/transfer contract.
    Bridge,
    /// Generic program/module.
    Other,
}

impl std::fmt::Display for ContractType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContractType::NullifierRegistry => write!(f, "nullifier_registry"),
            ContractType::StateCommitment => write!(f, "state_commitment"),
            ContractType::RightRegistry => write!(f, "right_registry"),
            ContractType::Bridge => write!(f, "bridge"),
            ContractType::Other => write!(f, "other"),
        }
    }
}

/// Status of a deployed contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContractStatus {
    /// Contract is active and in use.
    Active,
    /// Contract has been deprecated.
    Deprecated,
    /// Contract had an issue.
    Error,
}

impl std::fmt::Display for ContractStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContractStatus::Active => write!(f, "active"),
            ContractStatus::Deprecated => write!(f, "deprecated"),
            ContractStatus::Error => write!(f, "error"),
        }
    }
}

// ---------------------------------------------------------------------------
// Structs
// ---------------------------------------------------------------------------

/// Information about a blockchain chain being indexed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainInfo {
    /// Unique chain identifier (e.g., "bitcoin", "ethereum").
    pub id: String,
    /// Human-readable chain name.
    pub name: String,
    /// Network type (mainnet, testnet, devnet).
    pub network: Network,
    /// Current status of the chain indexer.
    pub status: ChainStatus,
    /// Latest block number indexed.
    pub latest_block: u64,
    /// Latest slot number (for slot-based chains like Solana), if applicable.
    pub latest_slot: Option<u64>,
    /// RPC endpoint URL for the chain.
    pub rpc_url: String,
    /// Sync lag in blocks behind the chain tip.
    #[serde(default)]
    pub sync_lag: u64,
}

/// A right record -- the core entity tracked by the CSV system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RightRecord {
    /// Right identifier (hex-encoded right_id).
    pub id: String,
    /// Chain that enforces the seal for this right.
    pub chain: String,
    /// Seal reference on the chain.
    pub seal_ref: String,
    /// Commitment hash of the right.
    pub commitment: String,
    /// Current owner address.
    pub owner: String,
    /// When the right was created.
    pub created_at: DateTime<Utc>,
    /// Transaction that created this right.
    pub created_tx: String,
    /// Current status of the right.
    pub status: RightStatus,
    /// Optional metadata associated with the right.
    pub metadata: Option<JsonValue>,
    /// Number of times this right has been transferred.
    pub transfer_count: u64,
    /// Timestamp of the last transfer, if any.
    pub last_transfer_at: Option<DateTime<Utc>>,
}

/// A cross-chain transfer record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferRecord {
    /// Transfer identifier.
    pub id: String,
    /// Right being transferred.
    pub right_id: String,
    /// Source chain.
    pub from_chain: String,
    /// Destination chain.
    pub to_chain: String,
    /// Previous owner (source chain).
    pub from_owner: String,
    /// New owner (destination chain).
    pub to_owner: String,
    /// Source chain lock transaction.
    pub lock_tx: String,
    /// Destination chain mint transaction (if completed).
    pub mint_tx: Option<String>,
    /// Proof reference (if available).
    pub proof_ref: Option<String>,
    /// Current transfer status.
    pub status: TransferStatus,
    /// When the transfer was initiated.
    pub created_at: DateTime<Utc>,
    /// When the transfer completed, if applicable.
    pub completed_at: Option<DateTime<Utc>>,
    /// Duration of the transfer in milliseconds, if completed.
    pub duration_ms: Option<u64>,
}

/// A seal record -- the mechanism that binds a right to a chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SealRecord {
    /// Seal identifier.
    pub id: String,
    /// Chain where the seal exists.
    pub chain: String,
    /// Type of seal.
    pub seal_type: SealType,
    /// Chain-specific seal reference.
    pub seal_ref: String,
    /// Linked right identifier, if known.
    pub right_id: Option<String>,
    /// Current seal status.
    pub status: SealStatus,
    /// When the seal was consumed, if applicable.
    pub consumed_at: Option<DateTime<Utc>>,
    /// Transaction that consumed the seal, if applicable.
    pub consumed_tx: Option<String>,
    /// Block height where the seal was created.
    pub block_height: u64,
}

/// A deployed CSV contract/program on a chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsvContract {
    /// Contract/program identifier.
    pub id: String,
    /// Chain where the contract is deployed.
    pub chain: String,
    /// Type of contract.
    pub contract_type: ContractType,
    /// Contract address.
    pub address: String,
    /// Deployment transaction.
    pub deployed_tx: String,
    /// When the contract was deployed.
    pub deployed_at: DateTime<Utc>,
    /// Contract version.
    pub version: String,
    /// Current contract status.
    pub status: ContractStatus,
}

// ---------------------------------------------------------------------------
// Filter types
// ---------------------------------------------------------------------------

/// Filter for querying rights.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RightFilter {
    pub chain: Option<String>,
    pub owner: Option<String>,
    pub status: Option<RightStatus>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Filter for querying transfers.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TransferFilter {
    pub right_id: Option<String>,
    pub from_chain: Option<String>,
    pub to_chain: Option<String>,
    pub status: Option<TransferStatus>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Filter for querying seals.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SealFilter {
    pub chain: Option<String>,
    pub seal_type: Option<SealType>,
    pub status: Option<SealStatus>,
    pub right_id: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Filter for querying contracts.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContractFilter {
    pub chain: Option<String>,
    pub contract_type: Option<ContractType>,
    pub status: Option<ContractStatus>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

// ---------------------------------------------------------------------------
// Stats types
// ---------------------------------------------------------------------------

/// Aggregate statistics for the explorer.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExplorerStats {
    pub total_rights: u64,
    pub total_transfers: u64,
    pub total_seals: u64,
    pub total_contracts: u64,
    pub rights_by_chain: Vec<ChainCount>,
    pub transfers_by_chain_pair: Vec<ChainPairCount>,
    pub active_seals_by_chain: Vec<ChainCount>,
    pub transfer_success_rate: f64,
    pub average_transfer_time_ms: Option<u64>,
}

/// Count of items on a specific chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainCount {
    pub chain: String,
    pub count: u64,
}

/// Count of transfers between a pair of chains.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainPairCount {
    pub from_chain: String,
    pub to_chain: String,
    pub count: u64,
}

// ---------------------------------------------------------------------------
// Indexer status types
// ---------------------------------------------------------------------------

/// Overall indexer status across all chains.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IndexerStatus {
    pub chains: Vec<ChainInfo>,
    pub total_indexed_blocks: u64,
    pub is_running: bool,
    pub started_at: Option<DateTime<Utc>>,
    pub uptime_seconds: Option<u64>,
}
