//! Shared Event Schemas for Cross-Chain Transfers
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
use crate::title::SanadId;

/// Standard event names in the CSV protocol
pub mod event_names {
    /// Sanad created on chain
    pub const SANAD_CREATED: &str = "SanadCreated";
    /// Sanad consumed (spent)
    pub const SANAD_CONSUMED: &str = "SanadConsumed";
    /// Sanad locked for cross-chain transfer
    pub const CROSS_CHAIN_LOCK: &str = "CrossChainLock";
    /// Sanad minted on destination chain
    pub const CROSS_CHAIN_MINT: &str = "CrossChainMint";
    /// Sanad refunded after timeout
    pub const CROSS_CHAIN_REFUND: &str = "CrossChainRefund";
    /// Sanad transferred to new owner
    pub const SANAD_TRANSFERRED: &str = "SanadTransferred";
    /// Nullifier registered for spent sanad
    pub const NULLIFIER_REGISTERED: &str = "NullifierRegistered";
    /// Sanad metadata recorded
    pub const SANAD_METADATA_RECORDED: &str = "SanadMetadataRecorded";
}

/// Standard metadata field names
pub mod metadata_fields {
    /// Unique sanad identifier
    pub const SANAD_ID: &str = "sanad_id";
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
    /// Metadata value
    pub const METADATA: &str = "metadata";
    /// Destination chain for cross-chain transfers
    pub const DESTINATION_CHAIN: &str = "destination_chain";
    /// Source chain for cross-chain transfers
    pub const SOURCE_CHAIN: &str = "source_chain";
    /// Block height where event occurred
    pub const BLOCK_HEIGHT: &str = "block_height";
    /// Transaction hash
    pub const TX_HASH: &str = "tx_hash";
}

/// Base event data structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CsvEvent {
    /// Event type identifier
    pub event_type: String,
    /// Chain where event occurred
    pub chain: String,
    /// Block height where event occurred
    pub block_height: u64,
    /// Transaction hash
    pub tx_hash: String,
    /// Event timestamp (unix timestamp in seconds)
    pub timestamp: u64,
    /// Event-specific data
    pub data: EventData,
    /// Optional metadata
    pub metadata: Option<serde_json::Value>,
}

/// Event data variants
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EventData {
    /// Sanad created event
    SanadCreated {
        sanad_id: SanadId,
        owner: String,
        commitment: Hash,
        asset_class: String,
        asset_id: String,
        metadata: Option<serde_json::Value>,
    },
    /// Sanad consumed event
    SanadConsumed {
        sanad_id: SanadId,
        nullifier: Hash,
        consumed_by: String,
    },
    /// Sanad transferred event
    SanadTransferred {
        sanad_id: SanadId,
        from: String,
        to: String,
        metadata: Option<serde_json::Value>,
    },
    /// Sanad-chain lock event
    CrossChainLock {
        sanad_id: SanadId,
        source_chain: String,
        destination_chain: String,
        destination_owner: String,
        proof_hash: Hash,
    },
    /// Cross-chain mint event
    CrossChainMint {
        sanad_id: SanadId,
        source_chain: String,
        source_sanad_id: SanadId,
        owner: String,
        proof_hash: Hash,
    },
    /// Cross-chain refund event
    CrossChainRefund {
        sanad_id: SanadId,
        source_chain: String,
        destination_chain: String,
        refunded_to: String,
    },
    /// Nullifier registered event
    NullifierRegistered {
        sanad_id: SanadId,
        nullifier: Hash,
        chain: String,
    },
    /// Sanad metadata recorded event
    SanadMetadataRecorded {
        sanad_id: SanadId,
        metadata: serde_json::Value,
    },
}

impl CsvEvent {
    /// Create a new SanadCreated event
    pub fn sanad_created(
        chain: &str,
        block_height: u64,
        tx_hash: &str,
        timestamp: u64,
        sanad_id: SanadId,
        owner: &str,
        commitment: Hash,
        asset_class: &str,
        asset_id: &str,
        metadata: Option<serde_json::Value>,
    ) -> Self {
        Self {
            event_type: event_names::SANAD_CREATED.to_string(),
            chain: chain.to_string(),
            block_height,
            tx_hash: tx_hash.to_string(),
            timestamp,
            data: EventData::SanadCreated {
                sanad_id,
                owner: owner.to_string(),
                commitment,
                asset_class: asset_class.to_string(),
                asset_id: asset_id.to_string(),
                metadata,
            },
            metadata: None,
        }
    }

    /// Create a new SanadConsumed event
    pub fn sanad_consumed(
        chain: &str,
        block_height: u64,
        tx_hash: &str,
        timestamp: u64,
        sanad_id: SanadId,
        nullifier: Hash,
        consumed_by: &str,
    ) -> Self {
        Self {
            event_type: event_names::SANAD_CONSUMED.to_string(),
            chain: chain.to_string(),
            block_height,
            tx_hash: tx_hash.to_string(),
            timestamp,
            data: EventData::SanadConsumed {
                sanad_id,
                nullifier,
                consumed_by: consumed_by.to_string(),
            },
            metadata: None,
        }
    }

    /// Create a new SanadTransferred event
    pub fn sanad_transferred(
        chain: &str,
        block_height: u64,
        tx_hash: &str,
        timestamp: u64,
        sanad_id: SanadId,
        from: &str,
        to: &str,
        metadata: Option<serde_json::Value>,
    ) -> Self {
        Self {
            event_type: event_names::SANAD_TRANSFERRED.to_string(),
            chain: chain.to_string(),
            block_height,
            tx_hash: tx_hash.to_string(),
            timestamp,
            data: EventData::SanadTransferred {
                sanad_id,
                from: from.to_string(),
                to: to.to_string(),
                metadata,
            },
            metadata: None,
        }
    }

    /// Create a new CrossChainLock event
    pub fn cross_chain_lock(
        chain: &str,
        block_height: u64,
        tx_hash: &str,
        timestamp: u64,
        sanad_id: SanadId,
        source_chain: &str,
        destination_chain: &str,
        destination_owner: &str,
        proof_hash: Hash,
    ) -> Self {
        Self {
            event_type: event_names::CROSS_CHAIN_LOCK.to_string(),
            chain: chain.to_string(),
            block_height,
            tx_hash: tx_hash.to_string(),
            timestamp,
            data: EventData::CrossChainLock {
                sanad_id,
                source_chain: source_chain.to_string(),
                destination_chain: destination_chain.to_string(),
                destination_owner: destination_owner.to_string(),
                proof_hash,
            },
            metadata: None,
        }
    }

    /// Create a new CrossChainMint event
    pub fn cross_chain_mint(
        chain: &str,
        block_height: u64,
        tx_hash: &str,
        timestamp: u64,
        sanad_id: SanadId,
        source_chain: &str,
        source_sanad_id: SanadId,
        owner: &str,
        proof_hash: Hash,
    ) -> Self {
        Self {
            event_type: event_names::CROSS_CHAIN_MINT.to_string(),
            chain: chain.to_string(),
            block_height,
            tx_hash: tx_hash.to_string(),
            timestamp,
            data: EventData::CrossChainMint {
                sanad_id,
                source_chain: source_chain.to_string(),
                source_sanad_id,
                owner: owner.to_string(),
                proof_hash,
            },
            metadata: None,
        }
    }

    /// Create a new CrossChainRefund event
    pub fn cross_chain_refund(
        chain: &str,
        block_height: u64,
        tx_hash: &str,
        timestamp: u64,
        sanad_id: SanadId,
        source_chain: &str,
        destination_chain: &str,
        refunded_to: &str,
    ) -> Self {
        Self {
            event_type: event_names::CROSS_CHAIN_REFUND.to_string(),
            chain: chain.to_string(),
            block_height,
            tx_hash: tx_hash.to_string(),
            timestamp,
            data: EventData::CrossChainRefund {
                sanad_id,
                source_chain: source_chain.to_string(),
                destination_chain: destination_chain.to_string(),
                refunded_to: refunded_to.to_string(),
            },
            metadata: None,
        }
    }

    /// Create a new NullifierRegistered event
    pub fn nullifier_registered(
        chain: &str,
        block_height: u64,
        tx_hash: &str,
        timestamp: u64,
        sanad_id: SanadId,
        nullifier: Hash,
    ) -> Self {
        Self {
            event_type: event_names::NULLIFIER_REGISTERED.to_string(),
            chain: chain.to_string(),
            block_height,
            tx_hash: tx_hash.to_string(),
            timestamp,
            data: EventData::NullifierRegistered {
                sanad_id,
                nullifier,
                chain: chain.to_string(),
            },
            metadata: None,
        }
    }

    /// Create a new SanadMetadataRecorded event
    pub fn sanad_metadata_recorded(
        chain: &str,
        block_height: u64,
        tx_hash: &str,
        timestamp: u64,
        sanad_id: SanadId,
        metadata: serde_json::Value,
    ) -> Self {
        Self {
            event_type: event_names::SANAD_METADATA_RECORDED.to_string(),
            chain: chain.to_string(),
            block_height,
            tx_hash: tx_hash.to_string(),
            timestamp,
            data: EventData::SanadMetadataRecorded {
                sanad_id,
                metadata,
            },
            metadata: None,
        }
    }
}

/// Event filter for querying events
#[derive(Debug, Clone, Default)]
pub struct EventFilter {
    /// Filter by event type
    pub event_type: Option<String>,
    /// Filter by chain
    pub chain: Option<String>,
    /// Filter by sanad ID
    pub sanad_id: Option<SanadId>,
    /// Filter by owner address
    pub owner: Option<String>,
    /// Filter by date range (start)
    pub from_timestamp: Option<u64>,
    /// Filter by date range (end)
    pub to_timestamp: Option<u64>,
    /// Maximum number of results
    pub limit: Option<usize>,
}

/// Event indexer interface
#[async_trait::async_trait]
pub trait EventIndexer: Send + Sync {
    /// Emit an event
    async fn emit(&self, event: CsvEvent) -> Result<(), Box<dyn std::error::Error>>;

    /// Query events with filter
    async fn query(
        &self,
        filter: &EventFilter,
    ) -> Result<Vec<CsvEvent>, Box<dyn std::error::Error>>;

    /// Get event by sanad ID
    async fn get_by_sanad_id(
        &self,
        sanad_id: &SanadId,
    ) -> Result<Vec<CsvEvent>, Box<dyn std::error::Error>>;
}

/// Event indexer registry
#[derive(Debug, Default)]
pub struct EventIndexerRegistry {
    indexers: Vec<Box<dyn EventIndexer>>,
}

impl EventIndexerRegistry {
    /// Create new registry
    pub fn new() -> Self {
        Self {
            indexers: Vec::new(),
        }
    }

    /// Register an indexer
    pub fn register(&mut self, indexer: Box<dyn EventIndexer>) {
        self.indexers.push(indexer);
    }

    /// Emit event to all registered indexers
    pub async fn emit(&self, event: CsvEvent) -> Result<(), Box<dyn std::error::Error>> {
        for indexer in &self.indexers {
            indexer.emit(event.clone()).await?;
        }
        Ok(())
    }

    /// Query events from all registered indexers
    pub async fn query(
        &self,
        filter: &EventFilter,
    ) -> Result<Vec<CsvEvent>, Box<dyn std::error::Error>> {
        let mut events = Vec::new();
        for indexer in &self.indexers {
            let indexer_events = indexer.query(filter).await?;
            events.extend(indexer_events);
        }
        Ok(events)
    }
}

/// Event finality status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EventFinalityStatus {
    /// Event is pending confirmation
    Pending,
    /// Event has been confirmed
    Confirmed {
        /// Number of confirmations
        confirmations: u64,
    },
    /// Event has reached finality
    Finalized,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_serialization() {
        let event = CsvEvent::sanad_created(
            "bitcoin",
            100,
            "tx123",
            1700000000,
            SanadId::new([0xAB; 32]),
            "owner1",
            Hash::new([0xCD; 32]),
            "RGB",
            "asset1",
            None,
        );

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("SanadCreated"));
        assert!(json.contains("bitcoin"));

        let deserialized: CsvEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.event_type, event.event_type);
        assert_eq!(deserialized.chain, event.chain);
    }

    #[test]
    fn test_event_filter() {
        let filter = EventFilter {
            event_type: Some("SanadCreated".to_string()),
            chain: Some("bitcoin".to_string()),
            ..Default::default()
        };

        assert_eq!(filter.event_type, Some("SanadCreated".to_string()));
        assert_eq!(filter.chain, Some("bitcoin".to_string()));
    }

    #[test]
    fn test_event_names() {
        assert_eq!(event_names::SANAD_CREATED, "SanadCreated");
        assert_eq!(event_names::SANAD_CONSUMED, "SanadConsumed");
        assert_eq!(event_names::CROSS_CHAIN_LOCK, "CrossChainLock");
        assert_eq!(event_names::CROSS_CHAIN_MINT, "CrossChainMint");
        assert_eq!(event_names::CROSS_CHAIN_REFUND, "CrossChainRefund");
        assert_eq!(event_names::SANAD_TRANSFERRED, "SanadTransferred");
        assert_eq!(event_names::NULLIFIER_REGISTERED, "NullifierRegistered");
        assert_eq!(
            event_names::SANAD_METADATA_RECORDED,
            "SanadMetadataRecorded"
        );
    }

    #[test]
    fn test_metadata_fields() {
        assert_eq!(metadata_fields::SANAD_ID, "sanad_id");
        assert_eq!(metadata_fields::COMMITMENT, "commitment");
        assert_eq!(metadata_fields::OWNER, "owner");
        assert_eq!(metadata_fields::CHAIN_ID, "chain_id");
        assert_eq!(metadata_fields::ASSET_CLASS, "asset_class");
        assert_eq!(metadata_fields::ASSET_ID, "asset_id");
        assert_eq!(metadata_fields::METADATA, "metadata");
        assert_eq!(
            metadata_fields::DESTINATION_CHAIN,
            "destination_chain"
        );
        assert_eq!(metadata_fields::SOURCE_CHAIN, "source_chain");
        assert_eq!(metadata_fields::BLOCK_HEIGHT, "block_height");
        assert_eq!(metadata_fields::TX_HASH, "tx_hash");
    }
}
