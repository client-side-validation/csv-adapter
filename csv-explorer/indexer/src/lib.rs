/// Multi-chain indexer daemon for the CSV Explorer.
///
/// Coordinates chain-specific indexers, manages sync progress,
/// and exposes metrics for monitoring.
pub mod aptos;
pub mod bitcoin;
pub mod chain_indexer;
pub mod ethereum;
pub mod metrics;
pub mod rpc_manager;
pub mod solana;
pub mod sui;
pub mod sync;
pub mod wallet_bridge;

pub use chain_indexer::{AddressIndexingResult, ChainIndexer, ChainResult};
pub use rpc_manager::{load_rpc_config, AuthType, RpcConfig, RpcEndpoint, RpcManager, RpcType};
pub use sync::SyncCoordinator;
pub use wallet_bridge::{WalletIndexerBridge, WalletIndexerBridgeConfig};

use csv_explorer_shared::{ExplorerConfig, Result};
use sqlx::SqlitePool;

/// The main indexer that wraps all chain indexers.
pub struct Indexer {
    config: ExplorerConfig,
    coordinator: SyncCoordinator,
    wallet_bridge: Option<WalletIndexerBridge>,
    rpc_manager: RpcManager,
}

impl Indexer {
    /// Create a new indexer with the given configuration and database pool.
    pub async fn new(config: ExplorerConfig, pool: SqlitePool) -> Result<Self> {
        // Load RPC configuration
        let rpc_manager = RpcManager::new(load_rpc_config()?);

        // Build chain indexers based on configuration
        let mut indexers: Vec<Box<dyn ChainIndexer>> = Vec::new();

        // Bitcoin
        if let Some(btc_config) = config.chains.get("bitcoin") {
            if btc_config.enabled {
                indexers.push(Box::new(bitcoin::BitcoinIndexer::new(
                    btc_config.clone(),
                    rpc_manager.clone(),
                )));
            }
        }

        // Ethereum
        if let Some(eth_config) = config.chains.get("ethereum") {
            if eth_config.enabled {
                indexers.push(Box::new(ethereum::EthereumIndexer::new(
                    eth_config.clone(),
                    rpc_manager.clone(),
                )));
            }
        }

        // Sui
        if let Some(sui_config) = config.chains.get("sui") {
            if sui_config.enabled {
                indexers.push(Box::new(sui::SuiIndexer::new(
                    sui_config.clone(),
                    rpc_manager.clone(),
                )));
            }
        }

        // Aptos
        if let Some(aptos_config) = config.chains.get("aptos") {
            if aptos_config.enabled {
                indexers.push(Box::new(aptos::AptosIndexer::new(
                    aptos_config.clone(),
                    rpc_manager.clone(),
                )));
            }
        }

        // Solana
        if let Some(sol_config) = config.chains.get("solana") {
            if sol_config.enabled {
                indexers.push(Box::new(solana::SolanaIndexer::new(
                    sol_config.clone(),
                    rpc_manager.clone(),
                )));
            }
        }

        let coordinator = SyncCoordinator::new(
            indexers,
            pool.clone(),
            config.chains.clone(),
            config.indexer.concurrency,
            config.indexer.batch_size,
            config.indexer.poll_interval_ms,
        );

        Ok(Self {
            config,
            coordinator,
            wallet_bridge: None,
            rpc_manager,
        })
    }

    /// Initialize the wallet-indexer bridge with priority indexing.
    pub async fn with_wallet_bridge(mut self, config: WalletIndexerBridgeConfig) -> Result<Self> {
        // Rebuild indexers as Arc<dyn ChainIndexer> using the existing rpc_manager
        let mut indexers: Vec<std::sync::Arc<dyn ChainIndexer>> = Vec::new();

        // Bitcoin
        if let Some(btc_config) = self.config.chains.get("bitcoin") {
            if btc_config.enabled {
                indexers.push(std::sync::Arc::new(bitcoin::BitcoinIndexer::new(
                    btc_config.clone(),
                    self.rpc_manager.clone(),
                )));
            }
        }

        // Ethereum
        if let Some(eth_config) = self.config.chains.get("ethereum") {
            if eth_config.enabled {
                indexers.push(std::sync::Arc::new(ethereum::EthereumIndexer::new(
                    eth_config.clone(),
                    self.rpc_manager.clone(),
                )));
            }
        }

        // Sui
        if let Some(sui_config) = self.config.chains.get("sui") {
            if sui_config.enabled {
                indexers.push(std::sync::Arc::new(sui::SuiIndexer::new(
                    sui_config.clone(),
                    self.rpc_manager.clone(),
                )));
            }
        }

        // Aptos
        if let Some(aptos_config) = self.config.chains.get("aptos") {
            if aptos_config.enabled {
                indexers.push(std::sync::Arc::new(aptos::AptosIndexer::new(
                    aptos_config.clone(),
                    self.rpc_manager.clone(),
                )));
            }
        }

        // Solana
        if let Some(sol_config) = self.config.chains.get("solana") {
            if sol_config.enabled {
                indexers.push(std::sync::Arc::new(solana::SolanaIndexer::new(
                    sol_config.clone(),
                    self.rpc_manager.clone(),
                )));
            }
        }

        let bridge =
            WalletIndexerBridge::new(self.coordinator.get_pool().clone(), indexers, config);

        bridge.initialize().await?;

        self.wallet_bridge = Some(bridge);
        Ok(self)
    }

    /// Get a reference to the wallet bridge if available.
    pub fn wallet_bridge(&self) -> Option<&WalletIndexerBridge> {
        self.wallet_bridge.as_ref()
    }

    /// Initialize all chain indexers.
    pub async fn initialize(&self) -> Result<()> {
        self.coordinator.initialize(&self.config.chains).await?;
        Ok(())
    }

    /// Start the indexer daemon.
    pub async fn start(&self) -> Result<()> {
        // Start wallet bridge if available
        if let Some(bridge) = &self.wallet_bridge {
            let bridge_clone = bridge.clone();
            tokio::spawn(async move {
                if let Err(e) = bridge_clone.start().await {
                    tracing::error!(error = %e, "Wallet bridge error");
                }
            });
        }

        self.coordinator.start().await?;
        tracing::info!("Indexer daemon started");
        Ok(())
    }

    /// Stop the indexer daemon.
    pub async fn stop(&self) -> Result<()> {
        if let Some(bridge) = &self.wallet_bridge {
            bridge.stop().await?;
        }
        self.coordinator.stop().await?;
        tracing::info!("Indexer daemon stopped");
        Ok(())
    }

    /// Get the current indexer status.
    pub async fn status(&self) -> csv_explorer_shared::IndexerStatus {
        self.coordinator.status().await
    }

    /// Force sync a specific chain.
    pub async fn sync_chain(&self, chain: &str) -> Result<()> {
        self.coordinator.sync_chain(chain).await
    }

    /// Force sync a specific chain from a specific block (overrides config).
    pub async fn sync_chain_from_block(&self, chain: &str, from_block: u64) -> Result<()> {
        self.coordinator
            .sync_chain_from_block(chain, from_block)
            .await
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
