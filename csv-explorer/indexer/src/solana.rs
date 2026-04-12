/// Solana chain indexer implementation.
///
/// Subscribes to Solana slot updates and tracks:
/// - Account creation/closure for seals
/// - Transaction logs for CSV program interactions
/// - SPL token account state

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::chain_indexer::{ChainIndexer, ChainResult};
use shared::{
    ChainConfig, ContractStatus, ContractType, CsvContract, ExplorerError, RightRecord,
    SealRecord, SealStatus, SealType, TransferRecord,
};

/// Solana-specific indexer.
pub struct SolanaIndexer {
    config: ChainConfig,
    http_client: Client,
}

#[derive(Debug, Serialize)]
struct SolanaRpcRequest {
    jsonrpc: String,
    method: String,
    params: Vec<serde_json::Value>,
    id: u64,
}

#[derive(Debug, Deserialize)]
struct SolanaRpcResponse {
    result: Option<serde_json::Value>,
    error: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct SlotInfo {
    slot: u64,
    parent: Option<u64>,
    root: u64,
}

#[derive(Debug, Deserialize)]
struct TransactionInfo {
    slot: u64,
    transaction: Option<serde_json::Value>,
    meta: Option<serde_json::Value>,
    block_time: Option<i64>,
}

#[async_trait]
impl ChainIndexer for SolanaIndexer {
    fn chain_id(&self) -> &str {
        "solana"
    }

    fn chain_name(&self) -> &str {
        "Solana"
    }

    async fn initialize(&self) -> ChainResult<()> {
        tracing::info!(chain = "solana", "Solana indexer initialized");
        Ok(())
    }

    async fn get_chain_tip(&self) -> ChainResult<u64> {
        let req = SolanaRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "getSlot".to_string(),
            params: vec![],
            id: 1,
        };

        let resp: SolanaRpcResponse = self.http_client
            .post(&self.config.rpc_url)
            .json(&req)
            .send()
            .await?
            .json()
            .await?;

        if let Some(result) = resp.result {
            let slot = result.get("slot").and_then(|v| v.as_u64()).unwrap_or(0);
            Ok(slot)
        } else {
            Err(ExplorerError::RpcError {
                chain: "solana".to_string(),
                message: "Failed to get slot".to_string(),
            })
        }
    }

    async fn get_latest_synced_block(&self) -> ChainResult<u64> {
        Ok(0)
    }

    async fn index_rights(&self, block: u64) -> ChainResult<Vec<RightRecord>> {
        // On Solana, rights are tracked through account state and transaction logs
        let txns = self.get_transactions_for_slot(block).await?;
        let mut rights = Vec::new();

        for txn_info in &txns {
            if let Some(meta) = &txn_info.meta {
                if let Some(logs) = meta.get("logMessages").and_then(|v| v.as_array()) {
                    for log in logs {
                        if let Some(log_str) = log.as_str() {
                            if log_str.contains("csv_right_created") || log_str.contains("RightCreated") {
                                if let Some(right) = self.parse_right_from_log(txn_info, log_str) {
                                    rights.push(right);
                                }
                            }
                        }
                    }
                }
            }
        }

        tracing::debug!(chain = "solana", block, count = rights.len(), "Indexed rights");
        Ok(rights)
    }

    async fn index_seals(&self, block: u64) -> ChainResult<Vec<SealRecord>> {
        let txns = self.get_transactions_for_slot(block).await?;
        let mut seals = Vec::new();

        for txn_info in &txns {
            if let Some(meta) = &txn_info.meta {
                // Check for account state changes indicating seal consumption
                if let Some(post_balances) = meta.get("postTokenBalances") {
                    let _ = post_balances; // Track SPL token account changes
                }

                if let Some(logs) = meta.get("logMessages").and_then(|v| v.as_array()) {
                    for log in logs {
                        if let Some(log_str) = log.as_str() {
                            if log_str.contains("csv_seal") || log_str.contains("SealConsumed") {
                                if let Some(seal) = self.parse_seal_from_log(txn_info, block) {
                                    seals.push(seal);
                                }
                            }
                        }
                    }
                }
            }
        }

        tracing::debug!(chain = "solana", block, count = seals.len(), "Indexed seals");
        Ok(seals)
    }

    async fn index_transfers(&self, block: u64) -> ChainResult<Vec<TransferRecord>> {
        let txns = self.get_transactions_for_slot(block).await?;
        let mut transfers = Vec::new();

        for txn_info in &txns {
            if let Some(meta) = &txn_info.meta {
                if let Some(logs) = meta.get("logMessages").and_then(|v| v.as_array()) {
                    for log in logs {
                        if let Some(log_str) = log.as_str() {
                            if log_str.contains("csv_transfer") || log_str.contains("CrossChainTransfer") {
                                if let Some(transfer) = self.parse_transfer_from_log(txn_info) {
                                    transfers.push(transfer);
                                }
                            }
                        }
                    }
                }
            }
        }

        tracing::debug!(chain = "solana", block, count = transfers.len(), "Indexed transfers");
        Ok(transfers)
    }

    async fn index_contracts(&self, _block: u64) -> ChainResult<Vec<CsvContract>> {
        Ok(vec![
            CsvContract {
                id: "solana-csv-program".to_string(),
                chain: "solana".to_string(),
                contract_type: ContractType::RightRegistry,
                address: "CsvProgram11111111111111111111111111111111111".to_string(),
                deployed_tx: "genesis".to_string(),
                deployed_at: chrono::Utc::now(),
                version: "1.0.0".to_string(),
                status: ContractStatus::Active,
            },
        ])
    }
}

impl SolanaIndexer {
    /// Create a new Solana indexer.
    pub fn new(config: ChainConfig) -> Self {
        Self {
            config,
            http_client: Client::new(),
        }
    }

    /// Get transactions for a specific slot.
    async fn get_transactions_for_slot(&self, slot: u64) -> ChainResult<Vec<TransactionInfo>> {
        let req = SolanaRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "getSignaturesForAddress".to_string(),
            params: vec![
                serde_json::json!("CsvProgram11111111111111111111111111111111111"),
                serde_json::json!({
                    "minContextSlot": slot,
                    "limit": 100
                }),
            ],
            id: 1,
        };

        let resp: SolanaRpcResponse = self.http_client
            .post(&self.config.rpc_url)
            .json(&req)
            .send()
            .await?
            .json()
            .await?;

        // In a real implementation, we would fetch each transaction
        // For now, return empty as the structure is set up
        if resp.result.is_some() {
            Ok(Vec::new())
        } else {
            Err(ExplorerError::RpcError {
                chain: "solana".to_string(),
                message: format!("Failed to get transactions for slot {}", slot),
            })
        }
    }

    fn parse_right_from_log(&self, txn: &TransactionInfo, log: &str) -> Option<RightRecord> {
        // Parse the log message to extract right data
        // Solana logs are in format: "Program log: <message>"
        let _ = log;
        let tx_sig = txn.transaction.as_ref()?.get("signatures")?.as_array()?.first()?.as_str()?;

        Some(RightRecord {
            id: format!("sol-right-{}", tx_sig),
            chain: "solana".to_string(),
            seal_ref: tx_sig.to_string(),
            commitment: "parsed_from_log".to_string(),
            owner: "unknown".to_string(),
            created_at: chrono::Utc::now(),
            created_tx: tx_sig.to_string(),
            status: shared::RightStatus::Active,
            metadata: None,
            transfer_count: 0,
            last_transfer_at: None,
        })
    }

    fn parse_seal_from_log(&self, txn: &TransactionInfo, block: u64) -> Option<SealRecord> {
        let tx_sig = txn.transaction.as_ref()?.get("signatures")?.as_array()?.first()?.as_str()?;

        Some(SealRecord {
            id: format!("sol-seal-{}", tx_sig),
            chain: "solana".to_string(),
            seal_type: SealType::Account,
            seal_ref: tx_sig.to_string(),
            right_id: None,
            status: SealStatus::Consumed,
            consumed_at: txn.block_time.map(|t| chrono::DateTime::from_timestamp(t, 0)).flatten(),
            consumed_tx: Some(tx_sig.to_string()),
            block_height: block,
        })
    }

    fn parse_transfer_from_log(&self, txn: &TransactionInfo) -> Option<TransferRecord> {
        let tx_sig = txn.transaction.as_ref()?.get("signatures")?.as_array()?.first()?.as_str()?;

        Some(TransferRecord {
            id: format!("sol-xfer-{}", tx_sig),
            right_id: "unknown".to_string(),
            from_chain: "solana".to_string(),
            to_chain: "unknown".to_string(),
            from_owner: "unknown".to_string(),
            to_owner: "unknown".to_string(),
            lock_tx: tx_sig.to_string(),
            mint_tx: None,
            proof_ref: None,
            status: shared::TransferStatus::Pending,
            created_at: chrono::Utc::now(),
            completed_at: None,
            duration_ms: None,
        })
    }
}
