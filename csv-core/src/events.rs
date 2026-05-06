//! Shared Event Schemas for Cross-Chain Rights
//!
//! This module defines standardized event types for the CSV protocol.
//! These events are used by:
//! - Chain adapters to emit events
//! - Explorer indexer plugins to index chain events
//! - Contract/program implementations to emit events
//! - SDKs to parse and display event data
//!
//! All events follow a consistent schema for maximum interoperability.

use serde::{Deserialize, Serialize};

use crate::hash::Hash;
use crate::right::RightId;

/// Standard event names in the CSV protocol
pub mod event_names {
    /// Right created on chain
    pub const RIGHT_CREATED: &str = "RightCreated";
    /// Right consumed (spent)
    pub const RIGHT_CONSUMED: &str = "RightConsumed";
    /// Right locked for cross-chain transfer
    pub const CROSS_CHAIN_LOCK: &str = "CrossChainLock";
    /// Right minted on destination chain
    pub const CROSS_CHAIN_MINT: &str = "CrossChainMint";
    /// Right refunded after timeout
    pub const CROSS_CHAIN_REFUND: &str = "CrossChainRefund";
    /// Right transferred to new owner
    pub const RIGHT_TRANSFERRED: &str = "RightTransferred";
    /// Nullifier registered for spent right
    pub const NULLIFIER_REGISTERED: &str = "NullifierRegistered";
    /// Right metadata recorded
    pub const RIGHT_METADATA_RECORDED: &str = "RightMetadataRecorded";
}

/// Standard metadata field names
pub mod metadata_fields {
    /// Unique right identifier
    pub const RIGHT_ID: &str = "right_id";
    /// Cryptographic commitment
    pub const COMMITMENT: &str = "commitment";
    /// Owner address
    pub const OWNER: &str = "owner";
    /// Chain identifier
    pub const CHAIN_ID: &str = "chain_id";
    /// Asset class (e.g., "RGB", "ERC20", "NFT")
    pub const ASSET_CLASS: &str = "asset_class";
    /// Specific asset identifier
    pub const ASSET_ID: &str = "asset_id";
    /// Hash of metadata
    pub const METADATA_HASH: &str = "metadata_hash";
    /// Proof system used (e.g., "Merkle", "STARK", "SNARK")
    pub const PROOF_SYSTEM: &str = "proof_system";
    /// Root of the proof
    pub const PROOF_ROOT: &str = "proof_root";
    /// Source chain for cross-chain
    pub const SOURCE_CHAIN: &str = "source_chain";
    /// Destination chain for cross-chain
    pub const DESTINATION_CHAIN: &str = "destination_chain";
    /// Transaction hash
    pub const TX_HASH: &str = "tx_hash";
    /// Block height
    pub const BLOCK_HEIGHT: &str = "block_height";
    /// Finality status
    pub const FINALITY_STATUS: &str = "finality_status";
}

/// Event finality status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum EventFinalityStatus {
    /// Event is pending/in mempool
    Pending,
    /// Event is confirmed but not finalized
    Confirmed,
    /// Event is finalized (safe from reorg)
    Finalized,
    /// Event was orphaned due to reorg
    Orphaned,
}

/// Core event envelope - all chain events use this structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsvEvent {
    /// Event name (use constants from event_names module)
    pub event_name: String,
    /// Chain where event occurred
    pub chain_id: String,
    /// Block height
    pub block_height: u64,
    /// Block hash
    pub block_hash: String,
    /// Transaction hash
    pub tx_hash: String,
    /// Log/index position in block
    pub log_index: u64,
    /// Timestamp (unix seconds)
    pub timestamp: u64,
    /// Event-specific data
    pub data: EventData,
    /// Finality status
    pub finality_status: EventFinalityStatus,
}

/// Event data payload - contains event-specific fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventData {
    /// Right ID (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub right_id: Option<RightId>,
    /// Commitment hash (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commitment: Option<Hash>,
    /// Owner address
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    /// Asset class
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asset_class: Option<String>,
    /// Asset identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asset_id: Option<String>,
    /// Metadata hash
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata_hash: Option<Hash>,
    /// Proof system
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proof_system: Option<String>,
    /// Proof root
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proof_root: Option<Hash>,
    /// Source chain (for cross-chain)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_chain: Option<String>,
    /// Destination chain (for cross-chain)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination_chain: Option<String>,
    /// Previous owner (for transfers)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_owner: Option<String>,
    /// New owner (for transfers)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_owner: Option<String>,
    /// Nullifier (for consumed rights)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nullifier: Option<Hash>,
    /// Lock ID (for cross-chain locks)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lock_id: Option<String>,
    /// Expiration time (for locks)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiration: Option<u64>,
    /// Additional chain-specific data
    #[serde(flatten)]
    pub extra: serde_json::Value,
}

impl EventData {
    /// Create empty event data
    pub fn empty() -> Self {
        Self {
            right_id: None,
            commitment: None,
            owner: None,
            asset_class: None,
            asset_id: None,
            metadata_hash: None,
            proof_system: None,
            proof_root: None,
            source_chain: None,
            destination_chain: None,
            previous_owner: None,
            new_owner: None,
            nullifier: None,
            lock_id: None,
            expiration: None,
            extra: serde_json::Value::Null,
        }
    }

    /// Builder-style setter for right_id
    pub fn with_right_id(mut self, right_id: RightId) -> Self {
        self.right_id = Some(right_id);
        self
    }

    /// Builder-style setter for commitment
    pub fn with_commitment(mut self, commitment: Hash) -> Self {
        self.commitment = Some(commitment);
        self
    }

    /// Builder-style setter for owner
    pub fn with_owner(mut self, owner: impl Into<String>) -> Self {
        self.owner = Some(owner.into());
        self
    }

    /// Builder-style setter for asset class
    pub fn with_asset_class(mut self, asset_class: impl Into<String>) -> Self {
        self.asset_class = Some(asset_class.into());
        self
    }

    /// Builder-style setter for asset_id
    pub fn with_asset_id(mut self, asset_id: impl Into<String>) -> Self {
        self.asset_id = Some(asset_id.into());
        self
    }

    /// Builder-style setter for metadata_hash
    pub fn with_metadata_hash(mut self, metadata_hash: Hash) -> Self {
        self.metadata_hash = Some(metadata_hash);
        self
    }

    /// Builder-style setter for proof_system
    pub fn with_proof_system(mut self, proof_system: impl Into<String>) -> Self {
        self.proof_system = Some(proof_system.into());
        self
    }

    /// Builder-style setter for proof_root
    pub fn with_proof_root(mut self, proof_root: Hash) -> Self {
        self.proof_root = Some(proof_root);
        self
    }

    /// Builder-style setter for source_chain
    pub fn with_source_chain(mut self, source_chain: impl Into<String>) -> Self {
        self.source_chain = Some(source_chain.into());
        self
    }

    /// Builder-style setter for destination_chain
    pub fn with_destination_chain(mut self, destination_chain: impl Into<String>) -> Self {
        self.destination_chain = Some(destination_chain.into());
        self
    }

    /// Builder-style setter for previous_owner
    pub fn with_previous_owner(mut self, previous_owner: impl Into<String>) -> Self {
        self.previous_owner = Some(previous_owner.into());
        self
    }

    /// Builder-style setter for new_owner
    pub fn with_new_owner(mut self, new_owner: impl Into<String>) -> Self {
        self.new_owner = Some(new_owner.into());
        self
    }

    /// Builder-style setter for nullifier
    pub fn with_nullifier(mut self, nullifier: Hash) -> Self {
        self.nullifier = Some(nullifier);
        self
    }

    /// Builder-style setter for lock_id
    pub fn with_lock_id(mut self, lock_id: impl Into<String>) -> Self {
        self.lock_id = Some(lock_id.into());
        self
    }

    /// Builder-style setter for expiration
    pub fn with_expiration(mut self, expiration: u64) -> Self {
        self.expiration = Some(expiration);
        self
    }
}

/// Event filter for querying indexed events
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EventFilter {
    /// Filter by event name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_name: Option<String>,
    /// Filter by chain ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chain_id: Option<String>,
    /// Filter by right ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub right_id: Option<RightId>,
    /// Filter by owner
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    /// Filter by asset class
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asset_class: Option<String>,
    /// Filter by asset ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asset_id: Option<String>,
    /// Start block height
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_block: Option<u64>,
    /// End block height
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_block: Option<u64>,
    /// Filter by source chain (for cross-chain)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_chain: Option<String>,
    /// Filter by destination chain (for cross-chain)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination_chain: Option<String>,
    /// Minimum finality status required
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_finality: Option<EventFinalityStatus>,
}

/// Trait for event indexing plugins
///
/// Each chain adapter implements this trait to index chain-specific events
/// into the shared event format.
pub trait EventIndexer {
    /// Get the chain ID this indexer handles
    fn chain_id(&self) -> &'static str;

    /// Parse a chain-specific log/event into CsvEvent
    fn parse_event(
        &self,
        raw_log: &[u8],
        block_height: u64,
        block_hash: &str,
        tx_hash: &str,
        log_index: u64,
    ) -> Option<CsvEvent>;

    /// Get filter for events this indexer cares about
    fn subscription_filter(&self) -> EventFilter;

    /// Check if an event is valid and final
    fn validate_event(&self, event: &CsvEvent) -> bool;
}

/// Registry for event indexers
pub struct EventIndexerRegistry {
    indexers: Vec<Box<dyn EventIndexer>>,
}

impl EventIndexerRegistry {
    /// Create a new registry
    pub fn new() -> Self {
        Self {
            indexers: Vec::new(),
        }
    }

    /// Register an indexer
    pub fn register(&mut self, indexer: Box<dyn EventIndexer>) {
        self.indexers.push(indexer);
    }

    /// Get indexer for a chain
    pub fn get_indexer(&self, chain_id: &str) -> Option<&dyn EventIndexer> {
        self.indexers
            .iter()
            .find(|i| i.chain_id() == chain_id)
            .map(|i| i.as_ref())
    }

    /// Get all registered chain IDs
    pub fn supported_chains(&self) -> Vec<&'static str> {
        self.indexers.iter().map(|i| i.chain_id()).collect()
    }

    /// Parse an event using the appropriate indexer
    pub fn parse_event(
        &self,
        chain_id: &str,
        raw_log: &[u8],
        block_height: u64,
        block_hash: &str,
        tx_hash: &str,
        log_index: u64,
    ) -> Option<CsvEvent> {
        self.get_indexer(chain_id)?.parse_event(
            raw_log,
            block_height,
            block_hash,
            tx_hash,
            log_index,
        )
    }
}

impl Default for EventIndexerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_data_builder() {
        let data = EventData::empty()
            .with_owner("0x1234")
            .with_asset_class("RGB")
            .with_asset_id("asset-1");

        assert_eq!(data.owner, Some("0x1234".to_string()));
        assert_eq!(data.asset_class, Some("RGB".to_string()));
        assert_eq!(data.asset_id, Some("asset-1".to_string()));
    }

    #[test]
    fn test_event_names() {
        assert_eq!(event_names::RIGHT_CREATED, "RightCreated");
        assert_eq!(event_names::CROSS_CHAIN_LOCK, "CrossChainLock");
    }

    #[test]
    fn test_metadata_fields() {
        assert_eq!(metadata_fields::RIGHT_ID, "right_id");
        assert_eq!(metadata_fields::CHAIN_ID, "chain_id");
    }

    #[test]
    fn test_event_filter_default() {
        let filter = EventFilter::default();
        assert!(filter.event_name.is_none());
        assert!(filter.chain_id.is_none());
    }

    #[test]
    fn test_event_finality_status_serialization() {
        let status = EventFinalityStatus::Finalized;
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("Finalized"));
    }
}
