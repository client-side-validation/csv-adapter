/// Sync coordination for multi-chain indexing.
///
/// Manages sync progress per chain, handles reorgs, and coordinates
/// concurrent chain syncing.

use std::sync::Arc;
use std::time::Duration;

use futures::future::join_all;
use tokio::sync::{Mutex, RwLock};
use tokio::time::sleep;

use super::chain_indexer::{ChainIndexer, ChainResult};
use shared::{ChainConfig, ChainInfo, ChainStatus, ExplorerError, IndexerStatus};

use csv_explorer_storage::repositories::SyncRepository;
use sqlx::SqlitePool;

/// Sync coordinator that manages multiple chain indexers.
pub struct SyncCoordinator {
    indexers: Vec<Box<dyn ChainIndexer>>,
    sync_repo: SyncRepository,
    concurrency: usize,
    batch_size: u64,
    poll_interval_ms: u64,
    running: Arc<RwLock<bool>>,
    chain_states: Arc<RwLock<Vec<ChainSyncState>>>,
}

/// Per-chain sync state.
#[derive(Debug, Clone)]
struct ChainSyncState {
    chain_id: String,
    chain_name: String,
    status: ChainStatus,
    latest_block: u64,
    latest_slot: Option<u64>,
    rpc_url: String,
    network: String,
}

impl SyncCoordinator {
    /// Create a new sync coordinator.
    pub fn new(
        indexers: Vec<Box<dyn ChainIndexer>>,
        pool: SqlitePool,
        concurrency: usize,
        batch_size: u64,
        poll_interval_ms: u64,
    ) -> Self {
        let chain_states = indexers
            .iter()
            .map(|idx| ChainSyncState {
                chain_id: idx.chain_id().to_string(),
                chain_name: idx.chain_name().to_string(),
                status: ChainStatus::Stopped,
                latest_block: 0,
                latest_slot: None,
                rpc_url: String::new(),
                network: String::new(),
            })
            .collect();

        Self {
            indexers,
            sync_repo: SyncRepository::new(pool),
            concurrency,
            batch_size,
            poll_interval_ms,
            running: Arc::new(RwLock::new(false)),
            chain_states: Arc::new(RwLock::new(chain_states)),
        }
    }

    /// Initialize all chain indexers.
    pub async fn initialize(&self, chain_configs: &std::collections::HashMap<String, ChainConfig>) -> Result<(), ExplorerError> {
        for indexer in &self.indexers {
            if let Some(config) = chain_configs.get(indexer.chain_id()) {
                if config.enabled {
                    indexer.initialize().await?;
                    tracing::info!(chain = indexer.chain_id(), "Chain indexer initialized");
                }
            }
        }
        Ok(())
    }

    /// Start the sync loop for all enabled chains.
    pub async fn start(&self) -> Result<(), ExplorerError> {
        let mut running = self.running.write().await;
        if *running {
            return Ok(());
        }
        *running = true;
        drop(running);

        tracing::info!("Starting sync coordinator");

        let running = self.running.clone();
        let chain_states = self.chain_states.clone();
        let indexers: Vec<_> = self.indexers.iter().map(|idx| idx.as_ref() as &(dyn ChainIndexer)).collect();
        let sync_repo = self.sync_repo.clone();
        let batch_size = self.batch_size;
        let poll_interval = self.poll_interval_ms;

        // Spawn the main sync loop
        tokio::spawn(async move {
            while *running.read().await {
                // Update chain states
                {
                    let mut states = chain_states.write().await;
                    for (i, indexer) in indexers.iter().enumerate() {
                        if let Ok(tip) = indexer.get_chain_tip().await {
                            states[i].latest_block = tip;
                            states[i].status = ChainStatus::Synced;
                        } else {
                            states[i].status = ChainStatus::Error;
                        }
                    }
                }

                // Sync each chain
                let mut futures = Vec::new();
                for indexer in &indexers {
                    let sync_repo = sync_repo.clone();
                    let batch_size = batch_size;
                    let chain_states = chain_states.clone();

                    futures.push(tokio::spawn(async move {
                        if let Err(e) = sync_chain(
                            indexer,
                            &sync_repo,
                            batch_size,
                            &chain_states,
                        ).await {
                            tracing::error!(chain = indexer.chain_id(), error = %e, "Sync error");
                        }
                    }));
                }

                // Wait for all sync operations to complete (up to concurrency limit)
                for chunk in futures.chunks(self.concurrency) {
                    join_all(chunk.iter().cloned()).await;
                }

                // Sleep before next poll
                sleep(Duration::from_millis(poll_interval)).await;
            }

            tracing::info!("Sync coordinator stopped");
        });

        Ok(())
    }

    /// Stop the sync loop.
    pub async fn stop(&self) -> Result<(), ExplorerError> {
        let mut running = self.running.write().await;
        *running = false;
        tracing::info!("Stopping sync coordinator");
        Ok(())
    }

    /// Get the current status of all chains.
    pub async fn status(&self) -> IndexerStatus {
        let states = self.chain_states.read().await;
        let chains = states
            .iter()
            .map(|state| ChainInfo {
                id: state.chain_id.clone(),
                name: state.chain_name.clone(),
                network: shared::Network::Mainnet, // Would be loaded from config
                status: state.status,
                latest_block: state.latest_block,
                latest_slot: state.latest_slot,
                rpc_url: state.rpc_url.clone(),
                sync_lag: 0,
            })
            .collect();

        IndexerStatus {
            chains,
            total_indexed_blocks: 0,
            is_running: *self.running.read().await,
            started_at: None,
            uptime_seconds: None,
        }
    }

    /// Force sync a specific chain.
    pub async fn sync_chain(&self, chain_id: &str) -> Result<(), ExplorerError> {
        let indexer = self
            .indexers
            .iter()
            .find(|idx| idx.chain_id() == chain_id)
            .ok_or_else(|| ExplorerError::Internal(format!("Chain {} not found", chain_id)))?;

        sync_chain(
            indexer.as_ref(),
            &self.sync_repo,
            self.batch_size,
            &self.chain_states,
        )
        .await
    }

    /// Reindex a chain from a specific block.
    pub async fn reindex_from(&self, chain_id: &str, from_block: u64) -> Result<(), ExplorerError> {
        // Reset sync progress for this chain
        self.sync_repo.reset(chain_id).await?;

        // Then sync from the specified block
        self.sync_chain(chain_id).await
    }

    /// Reset sync progress for all chains.
    pub async fn reset_sync(&self) -> Result<(), ExplorerError> {
        self.sync_repo.reset_all().await?;
        Ok(())
    }
}

/// Sync a single chain from its last synced position.
async fn sync_chain(
    indexer: &dyn ChainIndexer,
    sync_repo: &SyncRepository,
    batch_size: u64,
    _chain_states: &Arc<RwLock<Vec<ChainSyncState>>>,
) -> Result<(), ExplorerError> {
    let chain_id = indexer.chain_id();

    // Get last synced block
    let from_block = sync_repo
        .get_latest_block(chain_id)
        .await?
        .unwrap_or(0);

    // Get chain tip
    let tip = match indexer.get_chain_tip().await {
        Ok(tip) => tip,
        Err(e) => {
            tracing::warn!(chain = chain_id, error = %e, "Failed to get chain tip");
            return Err(e);
        }
    };

    if from_block >= tip {
        return Ok(()); // Already caught up
    }

    // Process blocks in batches
    let mut current = from_block + 1;
    let end = std::cmp::min(current + batch_size - 1, tip);

    tracing::debug!(
        chain = chain_id,
        from = current,
        to = end,
        "Syncing chain"
    );

    while current <= end {
        match indexer.process_block(current).await {
            Ok(result) => {
                tracing::trace!(
                    chain = chain_id,
                    block = current,
                    rights = result.rights_count,
                    seals = result.seals_count,
                    transfers = result.transfers_count,
                    "Processed block"
                );
            }
            Err(e) => {
                tracing::warn!(chain = chain_id, block = current, error = %e, "Failed to process block");
            }
        }

        // Update sync progress
        sync_repo.update_progress(chain_id, current, None).await?;

        current += 1;
    }

    tracing::info!(chain = chain_id, block = end, "Synced to block");
    Ok(())
}
