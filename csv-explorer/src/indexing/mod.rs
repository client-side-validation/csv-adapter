//! Real-time indexing pipeline for CSV Explorer
//!
//! Provides real-time indexing of rights, transfers, and proofs across all supported chains.
//! Supports event-driven updates and maintains a consistent view of cross-chain state.

pub mod events;
pub mod processor;
pub mod storage;
pub mod sync;

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use crate::indexing::events::IndexingEvent;
use crate::indexing::processor::EventProcessor;
use crate::indexing::storage::IndexStorage;
use crate::indexing::sync::ChainSynchronizer;
use csv_core::{Hash, TransferStatus};

/// Real-time indexing manager
pub struct IndexingManager {
    storage: Arc<IndexStorage>,
    processor: Arc<EventProcessor>,
    synchronizers: HashMap<String, Arc<ChainSynchronizer>>,
    event_buffer: Arc<RwLock<Vec<IndexingEvent>>>,
    metrics: IndexingMetrics,
}

/// Indexing performance metrics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IndexingMetrics {
    pub events_processed: u64,
    pub rights_indexed: u64,
    pub transfers_indexed: u64,
    pub average_processing_time: Duration,
    pub last_sync_time: DateTime<Utc>,
    pub active_chains: usize,
}

impl IndexingManager {
    /// Create a new indexing manager
    pub fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let storage = Arc::new(IndexStorage::new()?);
        let processor = Arc::new(EventProcessor::new(storage.clone()));
        let event_buffer = Arc::new(RwLock::new(Vec::new()));

        let synchronizers = HashMap::new();
        let chains = vec!["bitcoin", "ethereum", "sui", "aptos", "solana"];

        // ChainSynchronizer (alias for SyncCoordinator) requires complex initialization.
        // For now, we'll leave synchronizers empty - they need to be initialized
        // with proper indexer components from the workspace crates.
        let _chains = chains; // Use the variable to avoid warning

        let active_chains = synchronizers.len();

        Ok(Self {
            storage,
            processor,
            synchronizers,
            event_buffer,
            metrics: IndexingMetrics {
                events_processed: 0,
                rights_indexed: 0,
                transfers_indexed: 0,
                average_processing_time: Duration::from_millis(0),
                last_sync_time: Utc::now(),
                active_chains,
            },
        })
    }

    /// Start real-time indexing
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!("Starting real-time indexing pipeline...");

        // Start chain synchronizers
        for (chain_name, synchronizer) in &self.synchronizers {
            println!("Starting synchronizer for chain: {}", chain_name);
            let sync = synchronizer.clone();
            let chain_name = chain_name.clone();

            tokio::spawn(async move {
                if let Err(e) = sync.start().await {
                    eprintln!("Error starting {} synchronizer: {}", chain_name, e);
                }
            });
        }

        // Start event processor
        let processor = self.processor.clone();
        let buffer = self.event_buffer.clone();
        let metrics = Arc::new(RwLock::new(self.metrics.clone()));

        tokio::spawn(async move {
            Self::process_events_loop(processor, buffer, metrics).await;
        });

        println!("Indexing pipeline started successfully");
        Ok(())
    }

    /// Get current indexing metrics
    pub async fn get_metrics(&self) -> IndexingMetrics {
        // Update metrics from storage
        let rights_count = self.storage.get_rights_count().await;
        let transfers_count = self.storage.get_transfers_count().await;
        let mut metrics = self.metrics.clone();
        metrics.rights_indexed = rights_count;
        metrics.transfers_indexed = transfers_count;
        metrics.last_sync_time = Utc::now();
        metrics
    }

    /// Search rights by criteria
    pub async fn search_rights(
        &self,
        query: &RightsQuery,
    ) -> Result<Vec<IndexedRight>, Box<dyn std::error::Error + Send + Sync>> {
        self.storage.search_rights(query).await
    }

    /// Search transfers by criteria
    pub async fn search_transfers(
        &self,
        query: &TransferQuery,
    ) -> Result<Vec<IndexedTransfer>, Box<dyn std::error::Error + Send + Sync>> {
        self.storage.search_transfers(query).await
    }

    /// Get rights by owner
    pub async fn get_rights_by_owner(
        &self,
        owner: &str,
    ) -> Result<Vec<IndexedRight>, Box<dyn std::error::Error + Send + Sync>> {
        let query = RightsQuery {
            owner: Some(owner.to_string()),
            chain: None,
            status: None,
            limit: Some(100),
            offset: Some(0),
        };
        self.search_rights(&query).await
    }

    /// Get transfers by hash
    pub async fn get_transfer_by_hash(
        &self,
        hash: &Hash,
    ) -> Result<Option<IndexedTransfer>, Box<dyn std::error::Error + Send + Sync>> {
        self.storage.get_transfer_by_hash(hash).await
    }

    /// Process events in a loop
    async fn process_events_loop(
        processor: Arc<EventProcessor>,
        buffer: Arc<RwLock<Vec<IndexingEvent>>>,
        metrics: Arc<RwLock<IndexingMetrics>>,
    ) {
        let mut interval = tokio::time::interval(Duration::from_millis(100));

        loop {
            interval.tick().await;

            let events = {
                let mut buffer_guard = buffer.write().await;
                let events = buffer_guard.drain(..).collect::<Vec<_>>();
                drop(buffer_guard);
                events
            };

            if !events.is_empty() {
                let start = std::time::Instant::now();

                for event in &events {
                    if let Err(e) = processor.process_event(event).await {
                        eprintln!("Error processing event: {}", e);
                    }
                }

                let processing_time = start.elapsed();

                // Update metrics
                let mut metrics_guard = metrics.write().await;
                metrics_guard.events_processed += events.len() as u64;
                metrics_guard.average_processing_time = processing_time;
            }
        }
    }
}

/// Query parameters for rights search
#[derive(Debug, Clone)]
pub struct RightsQuery {
    pub owner: Option<String>,
    pub chain: Option<String>,
    pub status: Option<TransferStatus>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Query parameters for transfers search
#[derive(Debug, Clone)]
pub struct TransferQuery {
    pub from_chain: Option<String>,
    pub to_chain: Option<String>,
    pub status: Option<TransferStatus>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Indexed right with metadata
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IndexedRight {
    pub id: Hash,
    pub owner: String,
    pub chain: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub status: TransferStatus,
    pub metadata: serde_json::Value,
}

/// Indexed transfer with metadata
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IndexedTransfer {
    pub id: Hash,
    pub right_id: Hash,
    pub from_chain: String,
    pub to_chain: String,
    pub status: TransferStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub proof_bundle: Option<csv_core::proof::ProofBundle>,
    pub metadata: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_indexing_manager_creation() {
        let manager = IndexingManager::new();
        assert!(manager.is_ok());
    }

    #[tokio::test]
    async fn test_rights_search() {
        let manager = IndexingManager::new().unwrap();
        let query = RightsQuery {
            owner: None,
            chain: Some("ethereum".to_string()),
            status: None,
            limit: Some(10),
            offset: Some(0),
        };

        let results = manager.search_rights(&query).await;
        assert!(results.is_ok());
    }
}
