/// Core explorer types for the CSV Explorer.
///
/// This module defines all the data types used across the explorer,
/// including sanads, transfers, seals, contracts, and chain information.
///
/// ## Protocol Alignment
///
/// Explorer types MUST reuse canonical protocol types from `csv-adapter-core::protocol_version`
/// where applicable. The following types are re-exported from the protocol contract:
///
/// - [`csv_core::Chain`] — Canonical chain identifiers
/// - [`csv_core::TransferStatus`] — Canonical transfer lifecycle
/// - [`csv_core::SyncStatus`] — Indexer sync status
/// - [`csv_core::ErrorCode`] — Machine-readable error codes
///
/// Explorer-specific types (SanadRecord, SealRecord, etc.) wrap these
/// protocol types with additional metadata for display purposes.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

// ===========================================================================
// Re-export canonical protocol types (🔒 STABLE)
// ===========================================================================

// Chain IDs, transfer status, sync status, error codes from protocol contract
pub use csv_core::protocol_version::{
    Chain, ErrorCode, SyncStatus, TransferStatus, PROTOCOL_VERSION,
};

// ===========================================================================
// Explorer-specific enums
// ===========================================================================

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

/// Status of a sanad record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SanadStatus {
    /// Sanad is currently active.
    Active,
    /// Sanad has been spent/consumed.
    Spent,
    /// Sanad is pending confirmation.
    Pending,
}

impl std::fmt::Display for SanadStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SanadStatus::Active => write!(f, "active"),
            SanadStatus::Spent => write!(f, "spent"),
            SanadStatus::Pending => write!(f, "pending"),
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
    /// Sanad registry contract.
    SanadRegistry,
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
            ContractType::SanadRegistry => write!(f, "sanad_registry"),
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

/// A sanad record -- the core entity tracked by the CSV system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SanadRecord {
    /// Sanad identifier (hex-encoded sanad_id).
    pub id: String,
    /// Chain that enforces the seal for this sanad.
    pub chain: String,
    /// Seal reference on the chain.
    pub seal_ref: String,
    /// Commitment hash of the sanad.
    pub commitment: String,
    /// Current owner address.
    pub owner: String,
    /// When the sanad was created.
    pub created_at: DateTime<Utc>,
    /// Transaction that created this sanad.
    pub created_tx: String,
    /// Current status of the sanad.
    pub status: SanadStatus,
    /// Optional metadata associated with the sanad.
    pub metadata: Option<JsonValue>,
    /// Number of times this sanad has been transferred.
    pub transfer_count: u64,
    /// Timestamp of the last transfer, if any.
    pub last_transfer_at: Option<DateTime<Utc>>,
}

/// A cross-chain transfer record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferRecord {
    /// Transfer identifier.
    pub id: String,
    /// Sanad being transferred.
    pub sanad_id: String,
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

/// A seal record -- the mechanism that binds a sanad to a chain.
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
    /// Linked sanad identifier, if known.
    pub sanad_id: Option<String>,
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

/// Filter for querying sanads.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SanadFilter {
    pub chain: Option<String>,
    pub owner: Option<String>,
    pub status: Option<SanadStatus>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Filter for querying transfers.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TransferFilter {
    pub sanad_id: Option<String>,
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
    pub sanad_id: Option<String>,
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
    pub total_sanads: u64,
    pub total_transfers: u64,
    pub total_seals: u64,
    pub total_contracts: u64,
    pub sanads_by_chain: Vec<ChainCount>,
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

// ---------------------------------------------------------------------------
// Priority address indexing types
// ---------------------------------------------------------------------------

/// Priority level for address indexing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum PriorityLevel {
    /// High priority - index immediately and frequently
    High,
    /// Normal priority - index in regular cycle
    #[default]
    Normal,
    /// Low priority - index when resources available
    Low,
}

impl std::fmt::Display for PriorityLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PriorityLevel::High => write!(f, "high"),
            PriorityLevel::Normal => write!(f, "normal"),
            PriorityLevel::Low => write!(f, "low"),
        }
    }
}

/// A registered address with its priority configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriorityAddress {
    /// The address to index.
    pub address: String,
    /// Chain this address belongs to.
    pub chain: String,
    /// Network (mainnet/testnet).
    pub network: Network,
    /// Priority level for indexing.
    pub priority: PriorityLevel,
    /// Wallet ID that owns this address.
    pub wallet_id: String,
    /// When this address was registered.
    pub registered_at: DateTime<Utc>,
    /// Last time this address was indexed.
    pub last_indexed_at: Option<DateTime<Utc>>,
    /// Whether this address is actively being indexed.
    pub is_active: bool,
}

/// Status of priority address indexing.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PriorityIndexingStatus {
    /// Total number of registered addresses.
    pub total_addresses: u64,
    /// Number of addresses currently being indexed.
    pub active_indexing: u64,
    /// Number of addresses fully indexed.
    pub completed_indexing: u64,
    /// Recent indexing activities.
    pub recent_activities: Vec<IndexingActivity>,
}

/// A single indexing activity record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexingActivity {
    /// Address that was indexed.
    pub address: String,
    /// Chain that was indexed.
    pub chain: String,
    /// Network (mainnet/testnet).
    pub network: Network,
    /// What was indexed (sanads, seals, transfers).
    pub indexed_type: String,
    /// Number of items indexed.
    pub items_count: u64,
    /// When this indexing occurred.
    pub timestamp: DateTime<Utc>,
    /// Whether indexing was successful.
    pub success: bool,
    /// Error message if indexing failed.
    pub error: Option<String>,
}

// ===========================================================================
// Shared Event Schema Integration (Production Guarantee Plan Phase 6)
// ===========================================================================

/// Re-export canonical event types from csv-adapter-core
pub use csv_core::events::{
    CsvEvent, EventData, EventFilter as CsvEventFilter, EventFinalityStatus,
    event_names, metadata_fields,
};

/// Bridge trait for converting explorer records to CsvEvent
///
/// Implement this trait on chain indexers to enable shared schema compliance.
pub trait ToCsvEvent {
    /// Convert this record to a canonical CsvEvent
    fn to_csv_event(&self, chain_id: &str, block_info: BlockInfo) -> CsvEvent;
}

/// Block metadata for event conversion
#[derive(Debug, Clone)]
pub struct BlockInfo {
    pub height: u64,
    pub hash: String,
    pub timestamp: u64,
    pub tx_hash: String,
    pub log_index: u64,
}

impl SanadRecord {
    /// Convert SanadRecord to CsvEvent::SanadCreated
    pub fn to_sanad_created_event(&self, block_info: BlockInfo) -> CsvEvent {
        use csv_core::events::event_names;
        use csv_core::hash::Hash;
        use csv_core::sanad::SanadId;

        // Parse the sanad ID from hex string
        let id_bytes = hex::decode(&self.id).unwrap_or_default();
        let id_hash = if id_bytes.len() == 32 {
            Hash::new(id_bytes.try_into().unwrap_or([0u8; 32]))
        } else {
            Hash::zero()
        };

        let data = EventData::empty()
            .with_sanad_id(SanadId(id_hash))
            .with_owner(&self.owner);

        CsvEvent {
            event_name: event_names::SANAD_CREATED.to_string(),
            chain_id: self.chain.clone(),
            block_height: block_info.height,
            block_hash: block_info.hash,
            tx_hash: block_info.tx_hash,
            log_index: block_info.log_index,
            timestamp: block_info.timestamp,
            data,
            finality_status: EventFinalityStatus::Confirmed,
        }
    }
}

impl TransferRecord {
    /// Convert TransferRecord to CsvEvent::SanadTransferred or CrossChainLock/Mint
    pub fn to_transfer_event(&self, block_info: BlockInfo, is_cross_chain: bool) -> CsvEvent {
        use csv_core::events::event_names;
        use csv_core::hash::Hash;
        use csv_core::sanad::SanadId;

        let event_name = if is_cross_chain {
            if self.from_chain == self.to_chain {
                event_names::CROSS_CHAIN_LOCK
            } else {
                event_names::CROSS_CHAIN_MINT
            }
        } else {
            event_names::SANAD_TRANSFERRED
        };

        // Parse the sanad ID from hex string
        let id_bytes = hex::decode(&self.sanad_id).unwrap_or_default();
        let id_hash = if id_bytes.len() == 32 {
            Hash::new(id_bytes.try_into().unwrap_or([0u8; 32]))
        } else {
            Hash::zero()
        };

        let data = EventData::empty()
            .with_sanad_id(SanadId(id_hash))
            .with_previous_owner(&self.from_owner)
            .with_new_owner(&self.to_owner)
            .with_source_chain(&self.from_chain)
            .with_destination_chain(&self.to_chain);

        CsvEvent {
            event_name: event_name.to_string(),
            chain_id: self.from_chain.clone(),
            block_height: block_info.height,
            block_hash: block_info.hash,
            tx_hash: block_info.tx_hash,
            log_index: block_info.log_index,
            timestamp: block_info.timestamp,
            data,
            finality_status: EventFinalityStatus::Confirmed,
        }
    }
}
