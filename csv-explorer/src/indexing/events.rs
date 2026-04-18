//! Indexing events for real-time updates
//!
//! Defines the event types that drive the indexing pipeline
//! and provides utilities for event handling.

use csv_adapter_core::{Hash, Chain};
#[cfg(test)]
use csv_adapter_core::TransferStatus;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Indexing event types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IndexingEvent {
    /// Right created on a chain
    RightCreated {
        right_id: Hash,
        chain: Chain,
        owner: String,
        created_at: DateTime<Utc>,
        metadata: serde_json::Value,
    },
    
    /// Right transferred between chains
    RightTransferred {
        right_id: Hash,
        from_chain: Chain,
        to_chain: Chain,
        transfer_id: Hash,
        created_at: DateTime<Utc>,
        proof_bundle: Option<csv_adapter_core::proof::ProofBundle>,
    },
    
    /// Transfer status updated
    TransferUpdated {
        transfer_id: Hash,
        old_status: csv_adapter_core::TransferStatus,
        new_status: csv_adapter_core::TransferStatus,
        updated_at: DateTime<Utc>,
    },

    /// Right metadata updated
    RightUpdated {
        right_id: Hash,
        chain: Chain,
        old_metadata: serde_json::Value,
        new_metadata: serde_json::Value,
        updated_at: DateTime<Utc>,
    },

    /// Chain synchronization event
    ChainSynced {
        chain: Chain,
        block_height: u64,
        last_block_hash: Hash,
        synced_at: DateTime<Utc>,
    },

    /// Error event
    Error {
        error: String,
        chain: Option<Chain>,
        timestamp: DateTime<Utc>,
        context: serde_json::Value,
    },
}

impl IndexingEvent {
    /// Get event type name
    pub fn event_type(&self) -> &'static str {
        match self {
            IndexingEvent::RightCreated { .. } => "right_created",
            IndexingEvent::RightTransferred { .. } => "right_transferred",
            IndexingEvent::TransferUpdated { .. } => "transfer_updated",
            IndexingEvent::RightUpdated { .. } => "right_updated",
            IndexingEvent::ChainSynced { .. } => "chain_synced",
            IndexingEvent::Error { .. } => "error",
        }
    }
    
    /// Get chain associated with event
    pub fn chain(&self) -> Option<&Chain> {
        match self {
            IndexingEvent::RightCreated { chain, .. } => Some(chain),
            IndexingEvent::RightTransferred { from_chain, .. } => Some(from_chain),
            IndexingEvent::TransferUpdated { .. } => None,
            IndexingEvent::RightUpdated { chain, .. } => Some(chain),
            IndexingEvent::ChainSynced { chain, .. } => Some(chain),
            IndexingEvent::Error { chain, .. } => chain.as_ref(),
        }
    }
    
    /// Get timestamp for event
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            IndexingEvent::RightCreated { created_at, .. } => *created_at,
            IndexingEvent::RightTransferred { created_at, .. } => *created_at,
            IndexingEvent::TransferUpdated { updated_at, .. } => *updated_at,
            IndexingEvent::RightUpdated { updated_at, .. } => *updated_at,
            IndexingEvent::ChainSynced { synced_at, .. } => *synced_at,
            IndexingEvent::Error { timestamp, .. } => *timestamp,
        }
    }
    
    /// Check if event is critical for indexing
    pub fn is_critical(&self) -> bool {
        matches!(
            self,
            IndexingEvent::RightCreated { .. }
                | IndexingEvent::RightTransferred { .. }
                | IndexingEvent::TransferUpdated { .. }
                | IndexingEvent::RightUpdated { .. }
        )
    }
    
    /// Convert event to JSON for logging
    pub fn to_json(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::to_value(self)
    }
}

/// Event filter for subscription
#[derive(Debug, Clone)]
pub struct EventFilter {
    pub chains: Option<Vec<Chain>>,
    pub event_types: Option<Vec<String>>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
}

impl EventFilter {
    /// Create a new filter
    pub fn new() -> Self {
        Self {
            chains: None,
            event_types: None,
            start_time: None,
            end_time: None,
        }
    }
    
    /// Filter by chains
    pub fn chains(mut self, chains: Vec<Chain>) -> Self {
        self.chains = Some(chains);
        self
    }
    
    /// Filter by event types
    pub fn event_types(mut self, types: Vec<String>) -> Self {
        self.event_types = Some(types);
        self
    }
    
    /// Filter by time range
    pub fn time_range(mut self, start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        self.start_time = Some(start);
        self.end_time = Some(end);
        self
    }
    
    /// Check if event matches filter
    pub fn matches(&self, event: &IndexingEvent) -> bool {
        // Chain filter
        if let Some(ref chains) = self.chains {
            if let Some(event_chain) = event.chain() {
                if !chains.contains(event_chain) {
                    return false;
                }
            }
        }
        
        // Event type filter
        if let Some(ref event_types) = self.event_types {
            if !event_types.contains(&event.event_type().to_string()) {
                return false;
            }
        }
        
        // Time range filter
        let event_time = event.timestamp();
        if let Some(start_time) = self.start_time {
            if event_time < start_time {
                return false;
            }
        }
        if let Some(end_time) = self.end_time {
            if event_time > end_time {
                return false;
            }
        }
        
        true
    }
}

impl Default for EventFilter {
    fn default() -> Self {
        Self::new()
    }
}

/// Event batch for efficient processing
#[derive(Debug, Clone)]
pub struct EventBatch {
    pub events: Vec<IndexingEvent>,
    pub batch_id: String,
    pub created_at: DateTime<Utc>,
    pub size_bytes: usize,
}

impl EventBatch {
    /// Create a new event batch
    pub fn new(events: Vec<IndexingEvent>) -> Self {
        let size_bytes = events.iter()
            .map(|e| e.to_json().unwrap_or_default().to_string().len())
            .sum();

        Self {
            events,
            batch_id: uuid::Uuid::new_v4().to_string(),
            created_at: Utc::now(),
            size_bytes,
        }
    }
    
    /// Get batch size
    pub fn len(&self) -> usize {
        self.events.len()
    }
    
    /// Check if batch is empty
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
    
    /// Split batch into smaller chunks
    pub fn chunk(&self, chunk_size: usize) -> Vec<Self> {
        self.events
            .chunks(chunk_size)
            .map(|chunk| Self::new(chunk.to_vec()))
            .collect()
    }
}

/// Event statistics
#[derive(Debug, Clone, Default)]
pub struct EventStats {
    pub total_events: u64,
    pub right_created: u64,
    pub right_transferred: u64,
    pub transfer_updated: u64,
    pub right_updated: u64,
    pub chain_synced: u64,
    pub errors: u64,
    pub average_events_per_second: f64,
}

impl EventStats {
    /// Create new stats
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Update stats with event
    pub fn update(&mut self, event: &IndexingEvent) {
        self.total_events += 1;
        
        match event {
            IndexingEvent::RightCreated { .. } => self.right_created += 1,
            IndexingEvent::RightTransferred { .. } => self.right_transferred += 1,
            IndexingEvent::TransferUpdated { .. } => self.transfer_updated += 1,
            IndexingEvent::RightUpdated { .. } => self.right_updated += 1,
            IndexingEvent::ChainSynced { .. } => self.chain_synced += 1,
            IndexingEvent::Error { .. } => self.errors += 1,
        }
    }
    
    /// Calculate events per second
    pub fn calculate_rate(&mut self, duration: Duration) {
        if duration.as_secs() > 0 {
            self.average_events_per_second = self.total_events as f64 / duration.as_secs() as f64;
        }
    }
}

use std::time::Duration;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_event_filter() {
        let filter = EventFilter::new()
            .chains(vec![Chain::Ethereum])
            .event_types(vec!["right_created".to_string()]);

        let event = IndexingEvent::RightCreated {
            right_id: Hash::zero(),
            chain: Chain::Ethereum,
            owner: "test".to_string(),
            created_at: Utc::now(),
            metadata: serde_json::json!({}),
        };

        assert!(filter.matches(&event));
    }

    #[test]
    fn test_event_batch() {
        let events = vec![
            IndexingEvent::RightCreated {
                right_id: Hash::zero(),
                chain: Chain::Ethereum,
                owner: "test".to_string(),
                created_at: Utc::now(),
                metadata: serde_json::json!({}),
            }
        ];

        let batch = EventBatch::new(events);
        assert_eq!(batch.len(), 1);
        assert!(!batch.is_empty());
    }
}
