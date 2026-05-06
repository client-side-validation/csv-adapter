/// Sync coordination for multi-chain indexing.
///
/// Fixes applied:
/// 1. `start_block` priority: DB always wins over config (resumes progress).
/// 2. `process_block` double-fetch eliminated — data returned from process_block is used directly.
/// 3. `reindex_from` now passes `from_block` instead of dropping it.
/// 4. `start()` spawns one task per chain (honours `concurrency` config).
/// 5. `AdvancedProofRepository::init()` called on construction so enhanced tables exist.
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::RwLock;
use tokio::task::JoinSet;
use tokio::time::sleep;

use csv_explorer_indexer::ChainIndexer;
use csv_explorer_shared::{ChainConfig, ChainInfo, ChainStatus, ExplorerError, IndexerStatus};

use csv_explorer_storage::repositories::{
    AdvancedProofRepository, ContractsRepository, SanadsRepository, SealsRepository,
    SyncRepository, TransfersRepository,
};
use sqlx::SqlitePool;

/// Sync coordinator that manages multiple chain indexers.
pub struct SyncCoordinator {
    indexers: Vec<Arc<dyn ChainIndexer>>,
    pool: SqlitePool,
    sync_repo: SyncRepository,
    sanads_repo: SanadsRepository,
    seals_repo: SealsRepository,
    transfers_repo: TransfersRepository,
    contracts_repo: ContractsRepository,
    advanced_repo: AdvancedProofRepository,
    chain_configs: std::collections::HashMap<String, ChainConfig>,
    concurrency: usize,
    batch_size: u64,
    poll_interval_ms: u64,
    running: Arc<RwLock<bool>>,
    chain_states: Arc<RwLock<Vec<ChainSyncState>>>,
    total_indexed: Arc<RwLock<u64>>,
}

#[derive(Debug, Clone)]
pub struct ChainSyncState {
    chain_id: String,
    chain_name: String,
    status: ChainStatus,
    latest_block: u64,
    latest_slot: Option<u64>,
    rpc_url: String,
    #[allow(dead_code)]
    network: String,
}

impl SyncCoordinator {
    /// Create a new sync coordinator.
    ///
    /// Accepts `Box<dyn ChainIndexer>` — wraps internally as `Arc` for task spawning.
    pub fn new(
        indexers: Vec<Box<dyn ChainIndexer>>,
        pool: SqlitePool,
        chain_configs: std::collections::HashMap<String, ChainConfig>,
        concurrency: usize,
        batch_size: u64,
        poll_interval_ms: u64,
    ) -> Self {
        let arc_indexers: Vec<Arc<dyn ChainIndexer>> =
            indexers.into_iter().map(Arc::from).collect();

        let chain_states: Vec<ChainSyncState> = arc_indexers
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
            indexers: arc_indexers,
            pool: pool.clone(),
            sync_repo: SyncRepository::new(pool.clone()),
            sanads_repo: SanadsRepository::new(pool.clone()),
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

    pub fn get_pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Initialize all chain indexers AND ensure advanced tables exist.
    pub async fn initialize(
        &self,
        chain_configs: &std::collections::HashMap<String, ChainConfig>,
    ) -> Result<(), ExplorerError> {
        // FIX: create enhanced_sanads / enhanced_seals / enhanced_transfers tables
        self.advanced_repo
            .init()
            .await
            .map_err(ExplorerError::Database)?;

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

    // -----------------------------------------------------------------------
    // FIX: spawn one task per chain; honour `concurrency` via semaphore
    // -----------------------------------------------------------------------

    /// Start the sync loop for all enabled chains (concurrent, not serial).
    pub async fn start(&self) -> Result<(), ExplorerError> {
        {
            let mut running = self.running.write().await;
            if *running {
                return Ok(());
            }
            *running = true;
        }

        tracing::info!(concurrency = self.concurrency, "Starting sync coordinator");

        let semaphore = Arc::new(tokio::sync::Semaphore::new(self.concurrency));

        loop {
            if !*self.running.read().await {
                break;
            }

            let mut join_set: JoinSet<()> = JoinSet::new();

            for indexer in &self.indexers {
                let indexer = Arc::clone(indexer);
                let chain_config = self.chain_configs.get(indexer.chain_id()).cloned();
                let sync_repo = self.sync_repo.clone();
                let sanads_repo = self.sanads_repo.clone();
                let seals_repo = self.seals_repo.clone();
                let transfers_repo = self.transfers_repo.clone();
                let contracts_repo = self.contracts_repo.clone();
                let advanced_repo = self.advanced_repo.clone();
                let chain_states = Arc::clone(&self.chain_states);
                let total_indexed = Arc::clone(&self.total_indexed);
                let batch_size = self.batch_size;
                let sem = Arc::clone(&semaphore);

                join_set.spawn(async move {
                    let _permit = sem.acquire().await.expect("semaphore closed");
                    let ctx = SyncContext::new(
                        &sync_repo,
                        &sanads_repo,
                        &seals_repo,
                        &transfers_repo,
                        &contracts_repo,
                        &advanced_repo,
                        batch_size,
                        &chain_states,
                        &total_indexed,
                        chain_config.as_ref(),
                    );
                    if let Err(e) = sync_chain(indexer.as_ref(), &ctx).await {
                        tracing::error!(
                            chain = indexer.chain_id(),
                            error = %e,
                            "Chain sync error"
                        );
                    }
                });
            }

            // Wait for all chains this cycle
            while join_set.join_next().await.is_some() {}

            sleep(Duration::from_millis(self.poll_interval_ms)).await;
        }

        tracing::info!("Sync coordinator stopped");
        Ok(())
    }

    pub async fn stop(&self) -> Result<(), ExplorerError> {
        *self.running.write().await = false;
        tracing::info!("Stopping sync coordinator");
        Ok(())
    }

    pub async fn status(&self) -> IndexerStatus {
        let states = self.chain_states.read().await;
        let chains = states
            .iter()
            .map(|state| {
                let network = self
                    .chain_configs
                    .get(&state.chain_id)
                    .map(|c| c.network)
                    .unwrap_or(csv_explorer_shared::Network::Mainnet);

                ChainInfo {
                    id: state.chain_id.clone(),
                    name: state.chain_name.clone(),
                    network,
                    status: state.status,
                    latest_block: state.latest_block,
                    latest_slot: state.latest_slot,
                    rpc_url: state.rpc_url.clone(),
                    sync_lag: 0,
                }
            })
            .collect();

        IndexerStatus {
            chains,
            total_indexed_blocks: *self.total_indexed.read().await,
            is_running: *self.running.read().await,
            started_at: None,
            uptime_seconds: None,
        }
    }

    pub async fn sync_chain(&self, chain_id: &str) -> Result<(), ExplorerError> {
        self.sync_chain_from(chain_id, None).await
    }

    pub async fn sync_chain_from_block(
        &self,
        chain_id: &str,
        from_block: u64,
    ) -> Result<(), ExplorerError> {
        self.sync_chain_from(chain_id, Some(from_block)).await
    }

    async fn sync_chain_from(
        &self,
        chain_id: &str,
        override_start_block: Option<u64>,
    ) -> Result<(), ExplorerError> {
        let indexer = self
            .indexers
            .iter()
            .find(|idx| idx.chain_id() == chain_id)
            .ok_or_else(|| ExplorerError::Internal(format!("Chain {} not found", chain_id)))?;

        let effective_config = self.chain_configs.get(chain_id).cloned().map(|mut c| {
            if let Some(b) = override_start_block {
                c.start_block = Some(b);
            }
            c
        });

        let ctx = SyncContext::new(
            &self.sync_repo,
            &self.sanads_repo,
            &self.seals_repo,
            &self.transfers_repo,
            &self.contracts_repo,
            &self.advanced_repo,
            self.batch_size,
            &self.chain_states,
            &self.total_indexed,
            effective_config.as_ref(),
        );
        sync_chain(indexer.as_ref(), &ctx).await
    }

    // -----------------------------------------------------------------------
    // FIX: reindex_from actually uses from_block
    // -----------------------------------------------------------------------

    pub async fn reindex_from(&self, chain_id: &str, from_block: u64) -> Result<(), ExplorerError> {
        self.sync_repo.reset(chain_id).await?;
        // FIX: was `self.sync_chain(chain_id)` — from_block was silently dropped
        self.sync_chain_from_block(chain_id, from_block).await
    }

    pub async fn reset_sync(&self) -> Result<(), ExplorerError> {
        self.sync_repo.reset_all().await?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Sync Context - encapsulates all repository and state dependencies
// ---------------------------------------------------------------------------

/// Context passed to sync operations containing all repository and state dependencies.
pub struct SyncContext<'a> {
    /// Repository for sync progress tracking
    pub sync_repo: &'a SyncRepository,
    /// Repository for sanads data
    pub sanads_repo: &'a SanadsRepository,
    /// Repository for seals data
    pub seals_repo: &'a SealsRepository,
    /// Repository for transfers data
    pub transfers_repo: &'a TransfersRepository,
    /// Repository for contracts data
    pub contracts_repo: &'a ContractsRepository,
    /// Repository for advanced proof data
    pub advanced_repo: &'a AdvancedProofRepository,
    /// Number of blocks to process per batch
    pub batch_size: u64,
    /// Shared chain state tracking
    pub chain_states: &'a Arc<RwLock<Vec<ChainSyncState>>>,
    /// Shared total indexed counter
    pub total_indexed: &'a Arc<RwLock<u64>>,
    /// Chain-specific configuration
    pub chain_config: Option<&'a ChainConfig>,
}

impl<'a> SyncContext<'a> {
    /// Create a new sync context
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        sync_repo: &'a SyncRepository,
        sanads_repo: &'a SanadsRepository,
        seals_repo: &'a SealsRepository,
        transfers_repo: &'a TransfersRepository,
        contracts_repo: &'a ContractsRepository,
        advanced_repo: &'a AdvancedProofRepository,
        batch_size: u64,
        chain_states: &'a Arc<RwLock<Vec<ChainSyncState>>>,
        total_indexed: &'a Arc<RwLock<u64>>,
        chain_config: Option<&'a ChainConfig>,
    ) -> Self {
        Self {
            sync_repo,
            sanads_repo,
            seals_repo,
            transfers_repo,
            contracts_repo,
            advanced_repo,
            batch_size,
            chain_states,
            total_indexed,
            chain_config,
        }
    }
}

// ---------------------------------------------------------------------------
// Core per-chain sync function
// ---------------------------------------------------------------------------

async fn sync_chain(
    indexer: &dyn ChainIndexer,
    ctx: &SyncContext<'_>,
) -> Result<(), ExplorerError> {
    let chain_id = indexer.chain_id();

    // -----------------------------------------------------------------------
    // FIX: correct priority order
    //   1. DB progress (resume where we left off)          ← highest priority
    //   2. config start_block (first-ever run override)
    //   3. genesis (0)                                     ← fallback
    // -----------------------------------------------------------------------
    let db_block = ctx.sync_repo.get_latest_block(chain_id).await?;

    let from_block = match db_block {
        Some(block) => {
            tracing::debug!(chain = chain_id, block, "Resuming sync from database");
            block
        }
        None => {
            let configured = ctx.chain_config.and_then(|c| c.start_block).unwrap_or(0);
            if configured > 0 {
                tracing::info!(
                    chain = chain_id,
                    start_block = configured,
                    "Starting initial sync from configured start_block"
                );
            } else {
                tracing::warn!(
                    chain = chain_id,
                    "No start_block configured, syncing from genesis (block 0)"
                );
            }
            configured
        }
    };

    // Get chain tip — update state regardless
    let tip = match indexer.get_chain_tip().await {
        Ok(tip) => {
            update_chain_state(ctx.chain_states, chain_id, tip, ChainStatus::Syncing).await;
            tip
        }
        Err(e) => {
            update_chain_state(ctx.chain_states, chain_id, 0, ChainStatus::Error).await;
            tracing::warn!(chain = chain_id, error = %e, "Failed to get chain tip");
            return Err(e);
        }
    };

    if from_block >= tip {
        update_chain_state(ctx.chain_states, chain_id, tip, ChainStatus::Synced).await;
        return Ok(());
    }

    let start = from_block + 1;
    let end = (start + ctx.batch_size - 1).min(tip);

    tracing::debug!(chain = chain_id, from = start, to = end, "Syncing batch");

    let mut current = start;
    while current <= end {
        // -------------------------------------------------------------------
        // FIX: use parallel fetches per block; no double-fetch via process_block
        // -------------------------------------------------------------------
        let (sanads_res, seals_res, transfers_res, contracts_res) = tokio::join!(
            indexer.index_sanads(current),
            indexer.index_seals(current),
            indexer.index_transfers(current),
            indexer.index_contracts(current),
        );
        let (enh_sanads_res, enh_seals_res, enh_transfers_res) = tokio::join!(
            indexer.index_enhanced_sanads(current),
            indexer.index_enhanced_seals(current),
            indexer.index_enhanced_transfers(current),
        );

        // Store basic records
        match sanads_res {
            Ok(sanads) => {
                for sanad in &sanads {
                    if let Err(e) = ctx.sanads_repo.insert(sanad).await {
                        tracing::warn!(chain = chain_id, block = current, sanad_id = %sanad.id, error = %e, "Failed to insert sanad");
                    }
                }
            }
            Err(e) => {
                tracing::warn!(chain = chain_id, block = current, error = %e, "index_sanads failed")
            }
        }

        match seals_res {
            Ok(seals) => {
                for seal in &seals {
                    if let Err(e) = ctx.seals_repo.insert(seal).await {
                        tracing::warn!(chain = chain_id, block = current, seal_id = %seal.id, error = %e, "Failed to insert seal");
                    }
                }
            }
            Err(e) => {
                tracing::warn!(chain = chain_id, block = current, error = %e, "index_seals failed")
            }
        }

        match transfers_res {
            Ok(transfers) => {
                for transfer in &transfers {
                    if let Err(e) = ctx.transfers_repo.insert(transfer).await {
                        tracing::warn!(chain = chain_id, block = current, transfer_id = %transfer.id, error = %e, "Failed to insert transfer");
                    }
                }
            }
            Err(e) => {
                tracing::warn!(chain = chain_id, block = current, error = %e, "index_transfers failed")
            }
        }

        match contracts_res {
            Ok(contracts) => {
                for contract in &contracts {
                    if let Err(e) = ctx.contracts_repo.insert(contract).await {
                        tracing::warn!(chain = chain_id, block = current, contract_id = %contract.id, error = %e, "Failed to insert contract");
                    }
                }
            }
            Err(e) => {
                tracing::warn!(chain = chain_id, block = current, error = %e, "index_contracts failed")
            }
        }

        // Store enhanced records
        if let Ok(enhanced_sanads) = enh_sanads_res {
            for sanad in &enhanced_sanads {
                if let Err(e) = ctx.advanced_repo.insert_enhanced_sanad(sanad).await {
                    tracing::warn!(chain = chain_id, block = current, sanad_id = %sanad.id, error = %e, "Failed to insert enhanced sanad");
                }
            }
        }

        if let Ok(enhanced_seals) = enh_seals_res {
            for seal in &enhanced_seals {
                if let Err(e) = ctx.advanced_repo.insert_enhanced_seal(seal).await {
                    tracing::warn!(chain = chain_id, block = current, seal_id = %seal.id, error = %e, "Failed to insert enhanced seal");
                }
            }
        }

        if let Ok(enhanced_transfers) = enh_transfers_res {
            for transfer in &enhanced_transfers {
                if let Err(e) = ctx.advanced_repo.insert_enhanced_transfer(transfer).await {
                    tracing::warn!(chain = chain_id, block = current, transfer_id = %transfer.id, error = %e, "Failed to insert enhanced transfer");
                }
            }
        }

        // Persist progress
        ctx.sync_repo
            .update_progress(chain_id, current, None)
            .await?;
        {
            *ctx.total_indexed.write().await += 1;
        }

        current += 1;
    }

    update_chain_state(ctx.chain_states, chain_id, end, ChainStatus::Synced).await;
    tracing::info!(chain = chain_id, block = end, "Synced to block");
    Ok(())
}

async fn update_chain_state(
    chain_states: &Arc<RwLock<Vec<ChainSyncState>>>,
    chain_id: &str,
    latest_block: u64,
    status: ChainStatus,
) {
    let mut states = chain_states.write().await;
    if let Some(state) = states.iter_mut().find(|s| s.chain_id == chain_id) {
        state.latest_block = latest_block;
        state.status = status;
    }
}

/// Alias for backward compatibility
pub type ChainSynchronizer = SyncCoordinator;
