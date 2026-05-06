//! Storage layer for indexing pipeline
//!
//! Provides persistent storage for indexed rights, transfers, and events.

use crate::indexing::{IndexedRight, IndexedTransfer, RightsQuery, TransferQuery};
use chrono::{DateTime, Utc};
#[cfg(test)]
use csv_core::TransferStatus;
use csv_core::{Chain, Hash};
use std::collections::HashMap;
use std::sync::Arc;

/// Index storage interface
pub struct IndexStorage {
    // In-memory storage for demo purposes
    // In production, this would be a database connection
    rights: Arc<tokio::sync::RwLock<HashMap<Hash, IndexedRight>>>,
    transfers: Arc<tokio::sync::RwLock<HashMap<Hash, IndexedTransfer>>>,
    chain_sync_status: Arc<tokio::sync::RwLock<HashMap<String, ChainSyncInfo>>>,
    error_logs: Arc<tokio::sync::RwLock<Vec<ErrorLog>>>,
}

/// Chain synchronization information
#[derive(Debug, Clone)]
pub struct ChainSyncInfo {
    pub chain: String,
    pub block_height: u64,
    pub last_block_hash: Hash,
    pub last_sync_time: DateTime<Utc>,
    pub is_syncing: bool,
}

/// Error log entry
#[derive(Debug, Clone)]
pub struct ErrorLog {
    pub error: String,
    pub chain: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub context: serde_json::Value,
}

impl IndexStorage {
    /// Create a new index storage
    pub fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Self {
            rights: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            transfers: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            chain_sync_status: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            error_logs: Arc::new(tokio::sync::RwLock::new(Vec::new())),
        })
    }

    /// Store a right
    pub async fn store_right(
        &self,
        right: &IndexedRight,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut rights = self.rights.write().await;
        rights.insert(right.id, right.clone());
        Ok(())
    }

    /// Store a transfer
    pub async fn store_transfer(
        &self,
        transfer: &IndexedTransfer,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut transfers = self.transfers.write().await;
        transfers.insert(transfer.id, transfer.clone());
        Ok(())
    }

    /// Get right by ID
    pub async fn get_right_by_id(
        &self,
        right_id: &Hash,
    ) -> Result<Option<IndexedRight>, Box<dyn std::error::Error + Send + Sync>> {
        let rights = self.rights.read().await;
        Ok(rights.get(right_id).cloned())
    }

    /// Get transfer by hash
    pub async fn get_transfer_by_hash(
        &self,
        transfer_id: &Hash,
    ) -> Result<Option<IndexedTransfer>, Box<dyn std::error::Error + Send + Sync>> {
        let transfers = self.transfers.read().await;
        Ok(transfers.get(transfer_id).cloned())
    }

    /// Search rights by query
    pub async fn search_rights(
        &self,
        query: &RightsQuery,
    ) -> Result<Vec<IndexedRight>, Box<dyn std::error::Error + Send + Sync>> {
        let rights = self.rights.read().await;
        let mut results = Vec::new();

        for right in rights.values() {
            // Filter by owner
            if let Some(ref owner) = query.owner {
                if right.owner != *owner {
                    continue;
                }
            }

            // Filter by chain
            if let Some(ref chain) = query.chain {
                if right.chain != *chain {
                    continue;
                }
            }

            // Filter by status
            if let Some(ref status) = query.status {
                if right.status != *status {
                    continue;
                }
            }

            results.push(right.clone());
        }

        // Apply pagination
        let offset = query.offset.unwrap_or(0);
        let limit = query.limit.unwrap_or(results.len());
        let end = std::cmp::min(offset + limit, results.len());

        if offset < results.len() {
            Ok(results[offset..end].to_vec())
        } else {
            Ok(Vec::new())
        }
    }

    /// Search transfers by query
    pub async fn search_transfers(
        &self,
        query: &TransferQuery,
    ) -> Result<Vec<IndexedTransfer>, Box<dyn std::error::Error + Send + Sync>> {
        let transfers = self.transfers.read().await;
        let mut results = Vec::new();

        for transfer in transfers.values() {
            // Filter by from chain
            if let Some(ref from_chain) = query.from_chain {
                if transfer.from_chain != *from_chain {
                    continue;
                }
            }

            // Filter by to chain
            if let Some(ref to_chain) = query.to_chain {
                if transfer.to_chain != *to_chain {
                    continue;
                }
            }

            // Filter by status
            if let Some(ref status) = query.status {
                if transfer.status != *status {
                    continue;
                }
            }

            // Filter by time range
            if let Some(start_time) = query.start_time {
                if transfer.created_at < start_time {
                    continue;
                }
            }

            if let Some(end_time) = query.end_time {
                if transfer.created_at > end_time {
                    continue;
                }
            }

            results.push(transfer.clone());
        }

        // Apply pagination
        let offset = query.offset.unwrap_or(0);
        let limit = query.limit.unwrap_or(results.len());
        let end = std::cmp::min(offset + limit, results.len());

        if offset < results.len() {
            Ok(results[offset..end].to_vec())
        } else {
            Ok(Vec::new())
        }
    }

    /// Update chain sync status
    pub async fn update_chain_sync_status(
        &self,
        chain: &Chain,
        block_height: u64,
        last_block_hash: Hash,
        synced_at: DateTime<Utc>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut sync_status = self.chain_sync_status.write().await;

        sync_status.insert(
            chain.to_string(),
            ChainSyncInfo {
                chain: chain.to_string(),
                block_height,
                last_block_hash,
                last_sync_time: synced_at,
                is_syncing: false,
            },
        );

        Ok(())
    }

    /// Get chain sync status
    pub async fn get_chain_sync_status(
        &self,
        chain: &Chain,
    ) -> Result<Option<ChainSyncInfo>, Box<dyn std::error::Error + Send + Sync>> {
        let sync_status = self.chain_sync_status.read().await;
        Ok(sync_status.get(&chain.to_string()).cloned())
    }

    /// Log error
    pub async fn log_error(
        &self,
        error: &str,
        chain: Option<&Chain>,
        timestamp: DateTime<Utc>,
        context: &serde_json::Value,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut error_logs = self.error_logs.write().await;

        error_logs.push(ErrorLog {
            error: error.to_string(),
            chain: chain.map(|c| c.to_string()),
            timestamp,
            context: context.clone(),
        });

        // Keep only last 1000 error logs
        let len = error_logs.len();
        if len > 1000 {
            error_logs.drain(0..len - 1000);
        }

        Ok(())
    }

    /// Get error logs
    pub async fn get_error_logs(
        &self,
        limit: Option<usize>,
    ) -> Result<Vec<ErrorLog>, Box<dyn std::error::Error + Send + Sync>> {
        let error_logs = self.error_logs.read().await;
        let limit = limit.unwrap_or(error_logs.len());

        let start = if error_logs.len() > limit {
            error_logs.len() - limit
        } else {
            0
        };

        Ok(error_logs[start..].to_vec())
    }

    /// Get rights count
    pub async fn get_rights_count(&self) -> u64 {
        let rights = self.rights.read().await;
        rights.len() as u64
    }

    /// Get transfers count
    pub async fn get_transfers_count(&self) -> u64 {
        let transfers = self.transfers.read().await;
        transfers.len() as u64
    }

    /// Get storage statistics
    pub async fn get_storage_stats(
        &self,
    ) -> Result<StorageStats, Box<dyn std::error::Error + Send + Sync>> {
        let rights = self.rights.read().await;
        let transfers = self.transfers.read().await;
        let sync_status = self.chain_sync_status.read().await;
        let error_logs = self.error_logs.read().await;

        Ok(StorageStats {
            total_rights: rights.len(),
            total_transfers: transfers.len(),
            active_chains: sync_status.len(),
            error_count: error_logs.len(),
            last_updated: Utc::now(),
        })
    }

    /// Clear all data (for testing)
    pub async fn clear_all(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.rights.write().await.clear();
        self.transfers.write().await.clear();
        self.chain_sync_status.write().await.clear();
        self.error_logs.write().await.clear();
        Ok(())
    }
}

/// Storage statistics
#[derive(Debug, Clone)]
pub struct StorageStats {
    pub total_rights: usize,
    pub total_transfers: usize,
    pub active_chains: usize,
    pub error_count: usize,
    pub last_updated: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_storage_creation() {
        let storage = IndexStorage::new();
        assert!(storage.is_ok());
    }

    #[tokio::test]
    async fn test_right_storage() {
        let storage = IndexStorage::new().unwrap();

        let right = IndexedRight {
            id: Hash::zero(),
            owner: "test".to_string(),
            chain: "ethereum".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            status: TransferStatus::Initiated,
            metadata: serde_json::json!({}),
        };

        // Store right
        storage.store_right(&right).await.unwrap();

        // Get right
        let retrieved = storage.get_right_by_id(&right.id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().owner, "test");

        // Search rights
        let query = RightsQuery {
            owner: Some("test".to_string()),
            chain: None,
            status: None,
            limit: Some(10),
            offset: Some(0),
        };

        let results = storage.search_rights(&query).await.unwrap();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_transfer_storage() {
        let storage = IndexStorage::new().unwrap();

        let transfer = IndexedTransfer {
            id: Hash::zero(),
            right_id: Hash::zero(),
            from_chain: "ethereum".to_string(),
            to_chain: "sui".to_string(),
            status: TransferStatus::Initiated,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            proof_bundle: None,
            metadata: serde_json::json!({}),
        };

        // Store transfer
        storage.store_transfer(&transfer).await.unwrap();

        // Get transfer
        let retrieved = storage.get_transfer_by_hash(&transfer.id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().from_chain, "ethereum");

        // Search transfers
        let query = TransferQuery {
            from_chain: Some("ethereum".to_string()),
            to_chain: None,
            status: None,
            start_time: None,
            end_time: None,
            limit: Some(10),
            offset: Some(0),
        };

        let results = storage.search_transfers(&query).await.unwrap();
        assert_eq!(results.len(), 1);
    }
}
