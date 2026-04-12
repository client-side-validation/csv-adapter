/// Aptos chain indexer implementation.
///
/// Subscribes to Aptos ledger state updates and tracks:
/// - Resource creation/destruction for seals
/// - Move events from CSV modules
/// - Transaction finality

use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;

use super::chain_indexer::{ChainIndexer, ChainResult};
use shared::{
    ChainConfig, ContractStatus, ContractType, CsvContract, ExplorerError, RightRecord,
    SealRecord, SealStatus, SealType, TransferRecord,
};

/// Aptos-specific indexer.
pub struct AptosIndexer {
    config: ChainConfig,
    http_client: Client,
}

#[derive(Debug, Deserialize)]
struct LedgerInfo {
    block_height: String,
    ledger_version: String,
    ledger_timestamp: String,
}

#[derive(Debug, Deserialize)]
struct BlockTransactions {
    transactions: Vec<AptosTxn>,
}

#[derive(Debug, Deserialize)]
struct AptosTxn {
    hash: String,
    version: String,
    events: Option<Vec<AptosEvent>>,
    timestamp: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AptosEvent {
    guid: Option<serde_json::Value>,
    sequence_number: String,
    type_: String,
    data: serde_json::Value,
}

#[async_trait]
impl ChainIndexer for AptosIndexer {
    fn chain_id(&self) -> &str {
        "aptos"
    }

    fn chain_name(&self) -> &str {
        "Aptos"
    }

    async fn initialize(&self) -> ChainResult<()> {
        tracing::info!(chain = "aptos", "Aptos indexer initialized");
        Ok(())
    }

    async fn get_chain_tip(&self) -> ChainResult<u64> {
        let url = format!("{}/v1", self.config.rpc_url.trim_end_matches("/v1"));
        let info: LedgerInfo = self.http_client.get(&url).send().await?.json().await?;
        Ok(info.ledger_version.parse::<u64>().unwrap_or(0))
    }

    async fn get_latest_synced_block(&self) -> ChainResult<u64> {
        Ok(0)
    }

    async fn index_rights(&self, block: u64) -> ChainResult<Vec<RightRecord>> {
        let txns = self.fetch_block_transactions(block).await?;
        let mut rights = Vec::new();

        for txn in &txns {
            if let Some(events) = &txn.events {
                for event in events {
                    if event.type_.contains("RightCreated") || event.type_.contains("new_right") {
                        if let Some(right) = self.parse_right_from_event(event, &txn.hash) {
                            rights.push(right);
                        }
                    }
                }
            }
        }

        tracing::debug!(chain = "aptos", block, count = rights.len(), "Indexed rights");
        Ok(rights)
    }

    async fn index_seals(&self, block: u64) -> ChainResult<Vec<SealRecord>> {
        let txns = self.fetch_block_transactions(block).await?;
        let mut seals = Vec::new();

        for txn in &txns {
            if let Some(events) = &txn.events {
                for event in events {
                    if event.type_.contains("SealCreated")
                        || event.type_.contains("SealConsumed")
                        || event.type_.contains("nullifier")
                    {
                        if let Some(seal) = self.parse_seal_from_event(event, block) {
                            seals.push(seal);
                        }
                    }
                }
            }
        }

        tracing::debug!(chain = "aptos", block, count = seals.len(), "Indexed seals");
        Ok(seals)
    }

    async fn index_transfers(&self, block: u64) -> ChainResult<Vec<TransferRecord>> {
        let txns = self.fetch_block_transactions(block).await?;
        let mut transfers = Vec::new();

        for txn in &txns {
            if let Some(events) = &txn.events {
                for event in events {
                    if event.type_.contains("CrossChainTransfer")
                        || event.type_.contains("bridge_transfer")
                    {
                        if let Some(transfer) = self.parse_transfer_from_event(event, &txn.hash) {
                            transfers.push(transfer);
                        }
                    }
                }
            }
        }

        tracing::debug!(chain = "aptos", block, count = transfers.len(), "Indexed transfers");
        Ok(transfers)
    }

    async fn index_contracts(&self, _block: u64) -> ChainResult<Vec<CsvContract>> {
        Ok(vec![
            CsvContract {
                id: "aptos-csv-module".to_string(),
                chain: "aptos".to_string(),
                contract_type: ContractType::NullifierRegistry,
                address: "0x0000000000000000000000000000000000000000000000000000000000000001".to_string(),
                deployed_tx: "genesis".to_string(),
                deployed_at: chrono::Utc::now(),
                version: "1.0.0".to_string(),
                status: ContractStatus::Active,
            },
        ])
    }
}

impl AptosIndexer {
    /// Create a new Aptos indexer.
    pub fn new(config: ChainConfig) -> Self {
        Self {
            config,
            http_client: Client::new(),
        }
    }

    /// Fetch transactions for a specific block/version.
    async fn fetch_block_transactions(&self, version: u64) -> ChainResult<Vec<AptosTxn>> {
        // Aptos uses version-based indexing
        let base_url = self.config.rpc_url.trim_end_matches('/');
        let url = format!("{}/v1/transactions?start={}&limit=100", base_url, version);

        let resp: Vec<AptosTxn> = self.http_client.get(&url).send().await?.json().await?;
        Ok(resp)
    }

    fn parse_right_from_event(&self, event: &AptosEvent, tx_hash: &str) -> Option<RightRecord> {
        let right_id = event.data.get("right_id")?.as_str()?.to_string();
        let owner = event.data.get("owner")?.as_str()?.to_string();
        let commitment = event.data.get("commitment")?.as_str()?.to_string();

        Some(RightRecord {
            id: right_id,
            chain: "aptos".to_string(),
            seal_ref: format!("aptos-event-{}", event.sequence_number),
            commitment,
            owner,
            created_at: chrono::Utc::now(),
            created_tx: tx_hash.to_string(),
            status: shared::RightStatus::Active,
            metadata: Some(event.data.clone()),
            transfer_count: 0,
            last_transfer_at: None,
        })
    }

    fn parse_seal_from_event(&self, event: &AptosEvent, block: u64) -> Option<SealRecord> {
        let seal_id = event
            .data
            .get("seal_id")
            .or_else(|| event.data.get("nullifier"))?
            .as_str()?
            .to_string();

        let is_consumed = event.type_.contains("Consumed") || event.type_.contains("spent");

        Some(SealRecord {
            id: seal_id,
            chain: "aptos".to_string(),
            seal_type: SealType::Resource,
            seal_ref: format!("aptos-event-{}", event.sequence_number),
            right_id: None,
            status: if is_consumed {
                SealStatus::Consumed
            } else {
                SealStatus::Available
            },
            consumed_at: None,
            consumed_tx: None,
            block_height: block,
        })
    }

    fn parse_transfer_from_event(&self, event: &AptosEvent, tx_hash: &str) -> Option<TransferRecord> {
        let right_id = event.data.get("right_id")?.as_str()?.to_string();
        let from_chain = event.data.get("from_chain")?.as_str()?.to_string();
        let to_chain = event.data.get("to_chain")?.as_str()?.to_string();

        Some(TransferRecord {
            id: format!("aptos-xfer-{}", tx_hash),
            right_id,
            from_chain,
            to_chain,
            from_owner: "aptos-sender".to_string(),
            to_owner: "pending".to_string(),
            lock_tx: tx_hash.to_string(),
            mint_tx: None,
            proof_ref: None,
            status: shared::TransferStatus::Pending,
            created_at: chrono::Utc::now(),
            completed_at: None,
            duration_ms: None,
        })
    }
}
