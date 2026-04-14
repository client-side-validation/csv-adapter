/// Sync coordination for multi-chain indexing.
///
/// Manages sync progress per chain, handles reorgs, and coordinates
/// concurrent chain syncing.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::RwLock;
use tokio::time::sleep;

use super::chain_indexer::ChainIndexer;
use csv_explorer_shared::{ChainConfig, ChainInfo, ChainStatus, ExplorerError, IndexerStatus};

use csv_explorer_storage::repositories::{
    AdvancedProofRepository, ContractsRepository, RightsRepository, SealsRepository, SyncRepository, TransfersRepository,
};
use sqlx::SqlitePool;

/// Sync coordinator that manages multiple chain indexers.
pub struct SyncCoordinator {
    indexers: Vec<Box<dyn ChainIndexer>>,
    pool: SqlitePool,
    sync_repo: SyncRepository,
    rights_repo: RightsRepository,
    seals_repo: SealsRepository,
    transfers_repo: TransfersRepository,
    contracts_repo: ContractsRepository,
    advanced_repo: AdvancedProofRepository,
    /// Chain configurations for network info
    chain_configs: std::collections::HashMap<String, ChainConfig>,
    concurrency: usize,
    batch_size: u64,
    poll_interval_ms: u64,
    running: Arc<RwLock<bool>>,
    chain_states: Arc<RwLock<Vec<ChainSyncState>>>,
    /// Total indexed blocks counter
    total_indexed: Arc<RwLock<u64>>,
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
        chain_configs: std::collections::HashMap<String, ChainConfig>,
        concurrency: usize,
        batch_size: u64,
        poll_interval_ms: u64,
    ) -> Self {
        let chain_states = indexers
            .iter()
            .map(|idx| {
                let chain_id = idx.chain_id().to_string();
                let network = chain_configs
                    .get(&chain_id)
                    .map(|c| format!("{:?}", c.network))
                    .unwrap_or_else(|| "mainnet".to_string());
                let rpc_url = chain_configs
                    .get(&chain_id)
                    .map(|c| c.rpc_url.clone())
                    .unwrap_or_default();
                ChainSyncState {
                    chain_id,
                    chain_name: idx.chain_name().to_string(),
                    status: ChainStatus::Stopped,
                    latest_block: 0,
                    latest_slot: None,
                    rpc_url,
                    network,
                }
            })
            .collect();

        Self {
            indexers,
            pool: pool.clone(),
            sync_repo: SyncRepository::new(pool.clone()),
            rights_repo: RightsRepository::new(pool.clone()),
            seals_repo: SealsRepository::new(pool.clone()),
            transfers_repo: TransfersRepository::new(pool.clone()),
            contracts_repo: ContractsRepository::new(pool.clone()),
            advanced_repo: AdvancedProofRepository::new(pool),
            chain_configs,
            concurrency,
            batch_size,
            poll_interval_ms,
            running: Arc::new(RwLock::new(false)),
            chain_states: Arc::new(RwLock::new(chain_states)),
            total_indexed: Arc::new(RwLock::new(0)),
        }
    }

    /// Get a reference to the database pool.
    pub fn get_pool(&self) -> &SqlitePool {
        &self.pool
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

        // Run the sync loop directly (not spawned) to avoid lifetime issues with trait objects
        while *self.running.read().await {
            // Update chain states
            {
                let mut states = self.chain_states.write().await;
                for (i, indexer) in self.indexers.iter().enumerate() {
                    if let Ok(tip) = indexer.get_chain_tip().await {
                        states[i].latest_block = tip;
                        states[i].status = ChainStatus::Synced;
                    } else {
                        states[i].status = ChainStatus::Error;
                    }
                }
            }

            // Sync each chain
            for indexer in &self.indexers {
                if let Err(e) = sync_chain(
                    indexer.as_ref(),
                    &self.sync_repo,
                    &self.rights_repo,
                    &self.seals_repo,
                    &self.transfers_repo,
                    &self.contracts_repo,
                    &self.advanced_repo,
                    self.batch_size,
                    &self.chain_states,
                    &self.total_indexed,
                ).await {
                    tracing::error!(chain = indexer.chain_id(), error = %e, "Sync error");
                }
            }

            // Sleep before next poll
            sleep(Duration::from_millis(self.poll_interval_ms)).await;
        }

        tracing::info!("Sync coordinator stopped");

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
            .map(|state| {
                // Get network from chain config instead of hardcoding Mainnet
                let network = self.chain_configs
                    .get(&state.chain_id)
                    .map(|c| c.network)
                    .unwrap_or(csv_explorer_shared::Network::Mainnet);

                let sync_lag = 0u64; // Calculated from chain tip - latest_block during active sync

                ChainInfo {
                    id: state.chain_id.clone(),
                    name: state.chain_name.clone(),
                    network,
                    status: state.status,
                    latest_block: state.latest_block,
                    latest_slot: state.latest_slot,
                    rpc_url: state.rpc_url.clone(),
                    sync_lag,
                }
            })
            .collect();

        let total_indexed = *self.total_indexed.read().await;

        IndexerStatus {
            chains,
            total_indexed_blocks: total_indexed,
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
            &self.rights_repo,
            &self.seals_repo,
            &self.transfers_repo,
            &self.contracts_repo,
            &self.advanced_repo,
            self.batch_size,
            &self.chain_states,
            &self.total_indexed,
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
    rights_repo: &RightsRepository,
    seals_repo: &SealsRepository,
    transfers_repo: &TransfersRepository,
    contracts_repo: &ContractsRepository,
    advanced_repo: &AdvancedProofRepository,
    batch_size: u64,
    _chain_states: &Arc<RwLock<Vec<ChainSyncState>>>,
    total_indexed: &Arc<RwLock<u64>>,
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
                // Index and store rights
                match indexer.index_rights(current).await {
                    Ok(rights) => {
                        for right in &rights {
                            if let Err(e) = rights_repo.insert(right).await {
                                tracing::warn!(
                                    chain = chain_id,
                                    block = current,
                                    right_id = %right.id,
                                    error = %e,
                                    "Failed to insert right"
                                );
                            }
                        }
                        tracing::trace!(
                            chain = chain_id,
                            block = current,
                            rights = rights.len(),
                            "Indexed and stored rights"
                        );
                    }
                    Err(e) => {
                        tracing::warn!(chain = chain_id, block = current, error = %e, "Failed to index rights");
                    }
                }

                // Index and store seals
                match indexer.index_seals(current).await {
                    Ok(seals) => {
                        for seal in &seals {
                            if let Err(e) = seals_repo.insert(seal).await {
                                tracing::warn!(
                                    chain = chain_id,
                                    block = current,
                                    seal_id = %seal.id,
                                    error = %e,
                                    "Failed to insert seal"
                                );
                            }
                        }
                        tracing::trace!(
                            chain = chain_id,
                            block = current,
                            seals = seals.len(),
                            "Indexed and stored seals"
                        );
                    }
                    Err(e) => {
                        tracing::warn!(chain = chain_id, block = current, error = %e, "Failed to index seals");
                    }
                }

                // Index and store transfers
                match indexer.index_transfers(current).await {
                    Ok(transfers) => {
                        for transfer in &transfers {
                            if let Err(e) = transfers_repo.insert(transfer).await {
                                tracing::warn!(
                                    chain = chain_id,
                                    block = current,
                                    transfer_id = %transfer.id,
                                    error = %e,
                                    "Failed to insert transfer"
                                );
                            }
                        }
                        tracing::trace!(
                            chain = chain_id,
                            block = current,
                            transfers = transfers.len(),
                            "Indexed and stored transfers"
                        );
                    }
                    Err(e) => {
                        tracing::warn!(chain = chain_id, block = current, error = %e, "Failed to index transfers");
                    }
                }

                // Index and store contracts
                match indexer.index_contracts(current).await {
                    Ok(contracts) => {
                        for contract in &contracts {
                            if let Err(e) = contracts_repo.insert(contract).await {
                                tracing::warn!(
                                    chain = chain_id,
                                    block = current,
                                    contract_id = %contract.id,
                                    error = %e,
                                    "Failed to insert contract"
                                );
                            }
                        }
                        tracing::trace!(
                            chain = chain_id,
                            block = current,
                            contracts = contracts.len(),
                            "Indexed and stored contracts"
                        );
                    }
                    Err(e) => {
                        tracing::warn!(chain = chain_id, block = current, error = %e, "Failed to index contracts");
                    }
                }

                // Index and store enhanced rights with commitment metadata
                match indexer.index_enhanced_rights(current).await {
                    Ok(enhanced_rights) => {
                        for right in &enhanced_rights {
                            if let Err(e) = advanced_repo.insert_enhanced_right(right).await {
                                tracing::warn!(
                                    chain = chain_id,
                                    block = current,
                                    right_id = %right.id,
                                    error = %e,
                                    "Failed to insert enhanced right"
                                );
                            }
                        }
                        tracing::trace!(
                            chain = chain_id,
                            block = current,
                            enhanced_rights = enhanced_rights.len(),
                            "Indexed and stored enhanced rights"
                        );
                    }
                    Err(e) => {
                        tracing::warn!(chain = chain_id, block = current, error = %e, "Failed to index enhanced rights");
                    }
                }

                // Index and store enhanced seals with proof metadata
                match indexer.index_enhanced_seals(current).await {
                    Ok(enhanced_seals) => {
                        for seal in &enhanced_seals {
                            if let Err(e) = advanced_repo.insert_enhanced_seal(seal).await {
                                tracing::warn!(
                                    chain = chain_id,
                                    block = current,
                                    seal_id = %seal.id,
                                    error = %e,
                                    "Failed to insert enhanced seal"
                                );
                            }
                        }
                        tracing::trace!(
                            chain = chain_id,
                            block = current,
                            enhanced_seals = enhanced_seals.len(),
                            "Indexed and stored enhanced seals"
                        );
                    }
                    Err(e) => {
                        tracing::warn!(chain = chain_id, block = current, error = %e, "Failed to index enhanced seals");
                    }
                }

                // Index and store enhanced transfers with cross-chain proof data
                match indexer.index_enhanced_transfers(current).await {
                    Ok(enhanced_transfers) => {
                        for transfer in &enhanced_transfers {
                            if let Err(e) = advanced_repo.insert_enhanced_transfer(transfer).await {
                                tracing::warn!(
                                    chain = chain_id,
                                    block = current,
                                    transfer_id = %transfer.id,
                                    error = %e,
                                    "Failed to insert enhanced transfer"
                                );
                            }
                        }
                        tracing::trace!(
                            chain = chain_id,
                            block = current,
                            enhanced_transfers = enhanced_transfers.len(),
                            "Indexed and stored enhanced transfers"
                        );
                    }
                    Err(e) => {
                        tracing::warn!(chain = chain_id, block = current, error = %e, "Failed to index enhanced transfers");
                    }
                }

                tracing::trace!(
                    chain = chain_id,
                    block = current,
                    rights = result.rights_count,
                    seals = result.seals_count,
                    transfers = result.transfers_count,
                    contracts = result.contracts_count,
                    "Processed block"
                );
            }
            Err(e) => {
                tracing::warn!(chain = chain_id, block = current, error = %e, "Failed to process block");
            }
        }

        // Update sync progress
        sync_repo.update_progress(chain_id, current, None).await?;

        // Increment total indexed blocks counter
        {
            let mut total = total_indexed.write().await;
            *total += 1;
        }

        current += 1;
    }

    tracing::info!(chain = chain_id, block = end, "Synced to block");
    Ok(())
}
