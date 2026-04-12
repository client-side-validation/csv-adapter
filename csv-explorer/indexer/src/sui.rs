/// Sui chain indexer implementation.
///
/// Subscribes to Sui checkpoint events and tracks:
/// - Object creation/deletion for seals
/// - Move events from CSV packages
/// - Checkpoint finality

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::chain_indexer::{ChainIndexer, ChainResult};
use shared::{
    ChainConfig, ContractStatus, ContractType, CsvContract, ExplorerError, RightRecord,
    SealRecord, SealStatus, SealType, TransferRecord,
};

/// Sui-specific indexer.
pub struct SuiIndexer {
    config: ChainConfig,
    http_client: Client,
}

#[derive(Debug, Deserialize)]
struct CheckpointData {
    sequence_number: String,
    transactions: Vec<TxnData>,
    timestamp_ms: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TxnData {
    digest: String,
    events: Option<Vec<EventData>>,
}

#[derive(Debug, Deserialize)]
struct EventData {
    package_id: String,
    transaction_module: String,
    sender: String,
    type_: String,
    parsed_json: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct SuiRpcRequest {
    jsonrpc: String,
    method: String,
    params: Vec<serde_json::Value>,
    id: u64,
}

#[async_trait]
impl ChainIndexer for SuiIndexer {
    fn chain_id(&self) -> &str {
        "sui"
    }

    fn chain_name(&self) -> &str {
        "Sui"
    }

    async fn initialize(&self) -> ChainResult<()> {
        tracing::info!(chain = "sui", "Sui indexer initialized");
        Ok(())
    }

    async fn get_chain_tip(&self) -> ChainResult<u64> {
        let req = SuiRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "suix_getLatestCheckpointSequenceNumber".to_string(),
            params: vec![],
            id: 1,
        };

        let resp: serde_json::Value = self.http_client
            .post(&self.config.rpc_url)
            .json(&req)
            .send()
            .await?
            .json()
            .await?;

        if let Some(result) = resp.get("result").and_then(|v| v.as_str()) {
            Ok(result.parse::<u64>().unwrap_or(0))
        } else {
            Err(ExplorerError::RpcError {
                chain: "sui".to_string(),
                message: "Failed to get latest checkpoint".to_string(),
            })
        }
    }

    async fn get_latest_synced_block(&self) -> ChainResult<u64> {
        Ok(0)
    }

    async fn index_rights(&self, block: u64) -> ChainResult<Vec<RightRecord>> {
        let checkpoint = self.fetch_checkpoint(block).await?;
        let mut rights = Vec::new();

        if let Some(txns) = checkpoint.transactions {
            for txn in &txns {
                if let Some(events) = &txn.events {
                    for event in events {
                        if event.type_.contains("RightCreated") {
                            if let Some(right) = self.parse_right_from_event(event, &txn.digest) {
                                rights.push(right);
                            }
                        }
                    }
                }
            }
        }

        tracing::debug!(chain = "sui", block, count = rights.len(), "Indexed rights");
        Ok(rights)
    }

    async fn index_seals(&self, block: u64) -> ChainResult<Vec<SealRecord>> {
        let checkpoint = self.fetch_checkpoint(block).await?;
        let mut seals = Vec::new();

        if let Some(txns) = checkpoint.transactions {
            for txn in &txns {
                if let Some(events) = &txn.events {
                    for event in events {
                        if event.type_.contains("SealCreated") || event.type_.contains("SealConsumed") {
                            if let Some(seal) = self.parse_seal_from_event(event, block) {
                                seals.push(seal);
                            }
                        }
                    }
                }
            }
        }

        tracing::debug!(chain = "sui", block, count = seals.len(), "Indexed seals");
        Ok(seals)
    }

    async fn index_transfers(&self, block: u64) -> ChainResult<Vec<TransferRecord>> {
        let checkpoint = self.fetch_checkpoint(block).await?;
        let mut transfers = Vec::new();

        if let Some(txns) = checkpoint.transactions {
            for txn in &txns {
                if let Some(events) = &txn.events {
                    for event in events {
                        if event.type_.contains("CrossChainTransfer") {
                            if let Some(transfer) = self.parse_transfer_from_event(event, &txn.digest) {
                                transfers.push(transfer);
                            }
                        }
                    }
                }
            }
        }

        tracing::debug!(chain = "sui", block, count = transfers.len(), "Indexed transfers");
        Ok(transfers)
    }

    async fn index_contracts(&self, _block: u64) -> ChainResult<Vec<CsvContract>> {
        // Return known CSV packages on Sui
        Ok(vec![
            CsvContract {
                id: "sui-csv-package".to_string(),
                chain: "sui".to_string(),
                contract_type: ContractType::RightRegistry,
                address: "0x0000000000000000000000000000000000000000000000000000000000000000".to_string(),
                deployed_tx: "genesis".to_string(),
                deployed_at: chrono::Utc::now(),
                version: "1.0.0".to_string(),
                status: ContractStatus::Active,
            },
        ])
    }
}

impl SuiIndexer {
    /// Create a new Sui indexer.
    pub fn new(config: ChainConfig) -> Self {
        Self {
            config,
            http_client: Client::new(),
        }
    }

    /// Fetch checkpoint data via JSON-RPC.
    async fn fetch_checkpoint(&self, sequence: u64) -> ChainResult<CheckpointData> {
        let req = SuiRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "sui_getCheckpoint".to_string(),
            params: vec![serde_json::Value::String(sequence.to_string())],
            id: 1,
        };

        let resp: serde_json::Value = self.http_client
            .post(&self.config.rpc_url)
            .json(&req)
            .send()
            .await?
            .json()
            .await?;

        if let Some(result) = resp.get("result") {
            let checkpoint: CheckpointData = serde_json::from_value(result.clone()).map_err(|e| {
                ExplorerError::RpcParseError {
                    chain: "sui".to_string(),
                    message: e.to_string(),
                }
            })?;
            Ok(checkpoint)
        } else {
            Err(ExplorerError::RpcError {
                chain: "sui".to_string(),
                message: format!("Failed to get checkpoint {}", sequence),
            })
        }
    }

    fn parse_right_from_event(&self, event: &EventData, tx_digest: &str) -> Option<RightRecord> {
        let parsed = event.parsed_json.as_ref()?;
        let right_id = parsed.get("right_id")?.as_str()?.to_string();
        let owner = parsed.get("owner")?.as_str()?.to_string();
        let commitment = parsed.get("commitment")?.as_str()?.to_string();

        Some(RightRecord {
            id: right_id,
            chain: "sui".to_string(),
            seal_ref: event.package_id.clone(),
            commitment,
            owner,
            created_at: chrono::Utc::now(),
            created_tx: tx_digest.to_string(),
            status: shared::RightStatus::Active,
            metadata: None,
            transfer_count: 0,
            last_transfer_at: None,
        })
    }

    fn parse_seal_from_event(&self, event: &EventData, block: u64) -> Option<SealRecord> {
        let parsed = event.parsed_json.as_ref()?;
        let seal_id = parsed.get("seal_id")?.as_str()?.to_string();

        Some(SealRecord {
            id: seal_id,
            chain: "sui".to_string(),
            seal_type: SealType::Object,
            seal_ref: event.package_id.clone(),
            right_id: None,
            status: if event.type_.contains("Consumed") {
                SealStatus::Consumed
            } else {
                SealStatus::Available
            },
            consumed_at: None,
            consumed_tx: None,
            block_height: block,
        })
    }

    fn parse_transfer_from_event(&self, event: &EventData, tx_digest: &str) -> Option<TransferRecord> {
        let parsed = event.parsed_json.as_ref()?;
        let right_id = parsed.get("right_id")?.as_str()?.to_string();
        let from_chain = parsed.get("from_chain")?.as_str()?.to_string();
        let to_chain = parsed.get("to_chain")?.as_str()?.to_string();

        Some(TransferRecord {
            id: format!("sui-xfer-{}", tx_digest),
            right_id,
            from_chain,
            to_chain,
            from_owner: event.sender.clone(),
            to_owner: "pending".to_string(),
            lock_tx: tx_digest.to_string(),
            mint_tx: None,
            proof_ref: None,
            status: shared::TransferStatus::Pending,
            created_at: chrono::Utc::now(),
            completed_at: None,
            duration_ms: None,
        })
    }
}
