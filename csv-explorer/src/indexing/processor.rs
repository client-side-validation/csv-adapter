//! Event processor for indexing pipeline
//!
//! Handles the processing of indexing events and updates the storage layer.

use crate::indexing::events::IndexingEvent;
use crate::indexing::storage::IndexStorage;
use chrono::{DateTime, Utc};
use csv_core::{Chain, Hash, TransferStatus};
use std::sync::Arc;
use std::time::Instant;

/// Event processor for indexing events
pub struct EventProcessor {
    storage: Arc<IndexStorage>,
    stats: std::sync::Mutex<ProcessorStats>,
}

/// Processor statistics
#[derive(Debug, Clone)]
pub struct ProcessorStats {
    pub events_processed: u64,
    pub errors: u64,
    pub average_processing_time: std::time::Duration,
    pub last_processed_time: DateTime<Utc>,
}

impl Default for ProcessorStats {
    fn default() -> Self {
        Self {
            events_processed: 0,
            errors: 0,
            average_processing_time: std::time::Duration::from_secs(0),
            last_processed_time: Utc::now(),
        }
    }
}

impl EventProcessor {
    /// Create a new event processor
    pub fn new(storage: Arc<IndexStorage>) -> Self {
        Self {
            storage,
            stats: std::sync::Mutex::new(ProcessorStats::default()),
        }
    }

    /// Process a single event
    pub async fn process_event(
        &self,
        event: &IndexingEvent,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let start = Instant::now();
        let result = match event {
            IndexingEvent::SanadCreated {
                sanad_id,
                chain,
                owner,
                created_at,
                metadata,
            } => {
                self.handle_sanad_created(*sanad_id, *chain, owner, *created_at, metadata)
                    .await
            }
            IndexingEvent::SanadTransferred {
                sanad_id,
                from_chain,
                to_chain,
                transfer_id,
                created_at,
                proof_bundle,
            } => {
                self.handle_sanad_transferred(
                    *sanad_id,
                    *from_chain,
                    *to_chain,
                    *transfer_id,
                    *created_at,
                    proof_bundle.as_ref(),
                )
                .await
            }
            IndexingEvent::TransferUpdated {
                transfer_id,
                old_status,
                new_status,
                updated_at,
            } => {
                self.handle_transfer_updated(
                    *transfer_id,
                    old_status.clone(),
                    new_status.clone(),
                    *updated_at,
                )
                .await
            }
            IndexingEvent::SanadUpdated {
                sanad_id,
                chain,
                old_metadata,
                new_metadata,
                updated_at,
            } => {
                self.handle_sanad_updated(
                    *sanad_id,
                    *chain,
                    old_metadata,
                    new_metadata,
                    *updated_at,
                )
                .await
            }
            IndexingEvent::ChainSynced {
                chain,
                block_height,
                last_block_hash,
                synced_at,
            } => {
                self.handle_chain_synced(*chain, *block_height, *last_block_hash, *synced_at)
                    .await
            }
            IndexingEvent::Error {
                error,
                chain,
                timestamp,
                context,
            } => {
                self.handle_error(error, chain.as_ref(), *timestamp, context)
                    .await
            }
        };

        let processing_time = start.elapsed();
        {
            let mut stats = self.stats.lock().unwrap();
            stats.events_processed += 1;
            if result.is_err() {
                stats.errors += 1;
            }
            // Update average processing time
            let total_time = stats.average_processing_time.as_millis() as u64
                * (stats.events_processed - 1)
                + processing_time.as_millis() as u64;
            stats.average_processing_time =
                std::time::Duration::from_millis(total_time / stats.events_processed);
            stats.last_processed_time = Utc::now();
        }

        result
    }

    /// Handle sanad created event
    async fn handle_sanad_created(
        &self,
        sanad_id: Hash,
        chain: Chain,
        owner: &str,
        created_at: DateTime<Utc>,
        metadata: &serde_json::Value,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let indexed_sanad = crate::indexing::IndexedSanad {
            id: sanad_id,
            owner: owner.to_string(),
            chain: chain.to_string(),
            created_at,
            updated_at: created_at,
            status: TransferStatus::Initiated,
            metadata: metadata.clone(),
        };

        self.storage.store_sanad(&indexed_sanad).await?;
        Ok(())
    }

    /// Handle sanad transferred event
    async fn handle_sanad_transferred(
        &self,
        sanad_id: Hash,
        from_chain: Chain,
        to_chain: Chain,
        transfer_id: Hash,
        created_at: DateTime<Utc>,
        proof_bundle: Option<&csv_core::proof::ProofBundle>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let indexed_transfer = crate::indexing::IndexedTransfer {
            id: transfer_id,
            sanad_id,
            from_chain: from_chain.to_string(),
            to_chain: to_chain.to_string(),
            status: TransferStatus::Initiated,
            created_at,
            updated_at: created_at,
            proof_bundle: proof_bundle.cloned(),
            metadata: serde_json::json!({
                "from_chain": from_chain.to_string(),
                "to_chain": to_chain.to_string(),
            }),
        };

        self.storage.store_transfer(&indexed_transfer).await?;

        // Update sanad status and chain
        if let Some(mut sanad) = self.storage.get_sanad_by_id(&sanad_id).await? {
            sanad.status = TransferStatus::Initiated;
            sanad.chain = to_chain.to_string();
            sanad.updated_at = Utc::now();
            self.storage.store_sanad(&sanad).await?;
        }

        Ok(())
    }

    /// Handle transfer updated event
    async fn handle_transfer_updated(
        &self,
        transfer_id: Hash,
        _old_status: TransferStatus,
        new_status: TransferStatus,
        updated_at: DateTime<Utc>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(mut transfer) = self.storage.get_transfer_by_hash(&transfer_id).await? {
            transfer.status = new_status.clone();
            transfer.updated_at = updated_at;
            self.storage.store_transfer(&transfer).await?;

            // Update sanad status if transfer is completed
            if new_status.is_completed() {
                if let Some(mut sanad) = self.storage.get_sanad_by_id(&transfer.sanad_id).await? {
                    sanad.status = TransferStatus::Completed;
                    sanad.updated_at = updated_at;
                    self.storage.store_sanad(&sanad).await?;
                }
            }
        }

        Ok(())
    }

    /// Handle sanad updated event
    async fn handle_sanad_updated(
        &self,
        sanad_id: Hash,
        _chain: Chain,
        _old_metadata: &serde_json::Value,
        new_metadata: &serde_json::Value,
        updated_at: DateTime<Utc>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(mut sanad) = self.storage.get_sanad_by_id(&sanad_id).await? {
            sanad.metadata = new_metadata.clone();
            sanad.updated_at = updated_at;
            self.storage.store_sanad(&sanad).await?;
        }

        Ok(())
    }

    /// Handle chain synced event
    async fn handle_chain_synced(
        &self,
        chain: Chain,
        block_height: u64,
        last_block_hash: Hash,
        synced_at: DateTime<Utc>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.storage
            .update_chain_sync_status(&chain, block_height, last_block_hash, synced_at)
            .await?;
        Ok(())
    }

    /// Handle error event
    async fn handle_error(
        &self,
        error: &str,
        chain: Option<&Chain>,
        timestamp: DateTime<Utc>,
        context: &serde_json::Value,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Log error to storage for monitoring
        self.storage
            .log_error(error, chain, timestamp, context)
            .await?;
        Ok(())
    }

    /// Get processor statistics
    pub fn get_stats(&self) -> ProcessorStats {
        self.stats.lock().unwrap().clone()
    }

    /// Reset statistics
    pub fn reset_stats(&self) {
        let mut stats = self.stats.lock().unwrap();
        *stats = ProcessorStats::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indexing::storage::IndexStorage;

    #[tokio::test]
    async fn test_event_processor_creation() {
        let storage = Arc::new(IndexStorage::new().unwrap());
        let processor = EventProcessor::new(storage);

        assert_eq!(processor.get_stats().events_processed, 0);
        assert_eq!(processor.get_stats().errors, 0);
    }

    #[tokio::test]
    async fn test_sanad_created_event() {
        let storage = Arc::new(IndexStorage::new().unwrap());
        let processor = EventProcessor::new(storage);

        let event = IndexingEvent::SanadCreated {
            sanad_id: Hash::zero(),
            chain: Chain::Ethereum,
            owner: "test_owner".to_string(),
            created_at: Utc::now(),
            metadata: serde_json::json!({"test": "data"}),
        };

        let result = processor.process_event(&event).await;
        assert!(result.is_ok());
        let stats = processor.get_stats();
        assert_eq!(stats.events_processed, 1);
        assert_eq!(stats.errors, 0);
    }
}
