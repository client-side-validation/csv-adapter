/// Multi-chain indexer daemon for the CSV Explorer.
///
/// Coordinates chain-specific indexers, manages sync progress,
/// and exposes metrics for monitoring.

pub mod aptos;
pub mod bitcoin;
pub mod chain_indexer;
pub mod ethereum;
pub mod metrics;
pub mod solana;
pub mod sui;
pub mod sync;

pub use chain_indexer::{ChainIndexer, ChainResult};
pub use sync::SyncCoordinator;

use shared::{ChainConfig, ExplorerConfig, ExplorerError, Result};
use sqlx::SqlitePool;

/// The main indexer that wraps all chain indexers.
pub struct Indexer {
    config: ExplorerConfig,
    coordinator: SyncCoordinator,
}

impl Indexer {
    /// Create a new indexer with the given configuration and database pool.
    pub async fn new(config: ExplorerConfig, pool: SqlitePool) -> Result<Self> {
        // Build chain indexers based on configuration
        let mut indexers: Vec<Box<dyn ChainIndexer>> = Vec::new();

        // Bitcoin
        if let Some(btc_config) = config.chains.get("bitcoin") {
            if btc_config.enabled {
                indexers.push(Box::new(bitcoin::BitcoinIndexer::new(btc_config.clone())));
            }
        }

        // Ethereum
        if let Some(eth_config) = config.chains.get("ethereum") {
            if eth_config.enabled {
                indexers.push(Box::new(ethereum::EthereumIndexer::new(eth_config.clone())));
            }
        }

        // Sui
        if let Some(sui_config) = config.chains.get("sui") {
            if sui_config.enabled {
                indexers.push(Box::new(sui::SuiIndexer::new(sui_config.clone())));
            }
        }

        // Aptos
        if let Some(aptos_config) = config.chains.get("aptos") {
            if aptos_config.enabled {
                indexers.push(Box::new(aptos::AptosIndexer::new(aptos_config.clone())));
            }
        }

        // Solana
        if let Some(sol_config) = config.chains.get("solana") {
            if sol_config.enabled {
                indexers.push(Box::new(solana::SolanaIndexer::new(sol_config.clone())));
            }
        }

        let coordinator = SyncCoordinator::new(
            indexers,
            pool,
            config.indexer.concurrency,
            config.indexer.batch_size,
            config.indexer.poll_interval_ms,
        );

        Ok(Self { config, coordinator })
    }

    /// Initialize all chain indexers.
    pub async fn initialize(&self) -> Result<()> {
        self.coordinator.initialize(&self.config.chains).await?;
        Ok(())
    }

    /// Start the indexer daemon.
    pub async fn start(&self) -> Result<()> {
        self.coordinator.start().await?;
        tracing::info!("Indexer daemon started");
        Ok(())
    }

    /// Stop the indexer daemon.
    pub async fn stop(&self) -> Result<()> {
        self.coordinator.stop().await?;
        tracing::info!("Indexer daemon stopped");
        Ok(())
    }

    /// Get the current indexer status.
    pub async fn status(&self) -> shared::IndexerStatus {
        self.coordinator.status().await
    }

    /// Force sync a specific chain.
    pub async fn sync_chain(&self, chain: &str) -> Result<()> {
        self.coordinator.sync_chain(chain).await
    }

    /// Reindex a chain from a specific block.
    pub async fn reindex_from(&self, chain: &str, from_block: u64) -> Result<()> {
        self.coordinator.reindex_from(chain, from_block).await
    }

    /// Reset all sync progress.
    pub async fn reset_sync(&self) -> Result<()> {
        self.coordinator.reset_sync().await
    }
}
