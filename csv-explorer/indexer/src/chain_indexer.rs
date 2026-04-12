/// Trait definition for chain-specific indexers.
///
/// Each chain implementation implements this trait to provide a uniform
/// interface for block scanning, event parsing, and data indexing.

use async_trait::async_trait;

use shared::{CsvContract, ExplorerError, RightRecord, SealRecord, TransferRecord};

/// Result type alias for chain indexer operations.
pub type ChainResult<T> = std::result::Result<T, ExplorerError>;

/// Trait that each chain-specific indexer must implement.
///
/// The indexer daemon calls these methods to sync data from each chain.
#[async_trait]
pub trait ChainIndexer: Send + Sync {
    /// Unique identifier for this chain (e.g., "bitcoin", "ethereum").
    fn chain_id(&self) -> &str;

    /// Human-readable name for this chain.
    fn chain_name(&self) -> &str;

    /// Initialize the chain indexer (connect to RPC, validate config).
    async fn initialize(&self) -> ChainResult<()>;

    /// Get the current tip block number from the chain.
    async fn get_chain_tip(&self) -> ChainResult<u64>;

    /// Get the latest block number that has been fully synced.
    async fn get_latest_synced_block(&self) -> ChainResult<u64>;

    /// Index rights found in a specific block.
    async fn index_rights(&self, block: u64) -> ChainResult<Vec<RightRecord>>;

    /// Index seals found in a specific block.
    async fn index_seals(&self, block: u64) -> ChainResult<Vec<SealRecord>>;

    /// Index transfers found in a specific block.
    async fn index_transfers(&self, block: u64) -> ChainResult<Vec<TransferRecord>>;

    /// Index contract deployments/events found in a specific block.
    async fn index_contracts(&self, block: u64) -> ChainResult<Vec<CsvContract>>;

    /// Parse a single block and return the latest block height processed.
    /// This is a convenience method that calls all index_* methods.
    async fn process_block(&self, block: u64) -> ChainResult<BlockIndexResult> {
        let rights = self.index_rights(block).await?;
        let seals = self.index_seals(block).await?;
        let transfers = self.index_transfers(block).await?;
        let contracts = self.index_contracts(block).await?;

        Ok(BlockIndexResult {
            block,
            rights_count: rights.len() as u64,
            seals_count: seals.len() as u64,
            transfers_count: transfers.len() as u64,
            contracts_count: contracts.len() as u64,
        })
    }
}

/// Result of processing a single block.
#[derive(Debug, Clone)]
pub struct BlockIndexResult {
    pub block: u64,
    pub rights_count: u64,
    pub seals_count: u64,
    pub transfers_count: u64,
    pub contracts_count: u64,
}
