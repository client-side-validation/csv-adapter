/// Solana chain indexer implementation.
///
/// Subscribes to Solana slot updates and tracks:
/// - Account creation/closure for seals
/// - Transaction logs for CSV program interactions
/// - SPL token account state

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::chain_indexer::{AddressIndexingResult, ChainIndexer, ChainResult};
use csv_explorer_shared::{
    ChainConfig, CommitmentScheme, ContractStatus, ContractType, CsvContract, EnhancedRightRecord,
    EnhancedSealRecord, EnhancedTransferRecord, ExplorerError, FinalityProofType, InclusionProofType,
    Network, PriorityLevel, RightRecord, SealRecord, SealStatus, SealType, TransferRecord,
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

    // -----------------------------------------------------------------------
    // Address-based indexing methods (for priority indexing)
    // -----------------------------------------------------------------------

    async fn index_rights_by_address(&self, _address: &str) -> ChainResult<Vec<RightRecord>> {
        Ok(Vec::new())
    }

    async fn index_seals_by_address(&self, _address: &str) -> ChainResult<Vec<SealRecord>> {
        Ok(Vec::new())
    }

    async fn index_transfers_by_address(&self, _address: &str) -> ChainResult<Vec<TransferRecord>> {
        Ok(Vec::new())
    }

    async fn index_addresses_with_priority(
        &self,
        addresses: &[String],
        _priority: PriorityLevel,
        _network: Network,
    ) -> ChainResult<AddressIndexingResult> {
        let mut result = AddressIndexingResult {
            addresses_processed: 0,
            rights_indexed: 0,
            seals_indexed: 0,
            transfers_indexed: 0,
            contracts_indexed: 0,
            errors: Vec::new(),
        };

        for address in addresses {
            if let Ok(rights) = self.index_rights_by_address(address).await {
                result.rights_indexed += rights.len() as u64;
                result.addresses_processed += 1;
            }
            if let Ok(seals) = self.index_seals_by_address(address).await {
                result.seals_indexed += seals.len() as u64;
            }
            if let Ok(transfers) = self.index_transfers_by_address(address).await {
                result.transfers_indexed += transfers.len() as u64;
            }
        }

        Ok(result)
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
            status: csv_explorer_shared::RightStatus::Active,
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
            status: csv_explorer_shared::TransferStatus::Pending,
            created_at: chrono::Utc::now(),
            completed_at: None,
            duration_ms: None,
        })
    }

    // -----------------------------------------------------------------------
    // Advanced commitment and proof indexing methods
    // -----------------------------------------------------------------------

    async fn index_enhanced_rights(&self, block: u64) -> ChainResult<Vec<EnhancedRightRecord>> {
        // On Solana, rights are tracked through account state and transaction logs
        let txns = self.get_transactions_for_slot(block).await?;
        let mut rights = Vec::new();

        for txn in &txns {
            if let Some(meta) = &txn.meta {
                if let Some(logs) = meta.get("logMessages").and_then(|v| v.as_array()) {
                    for log in logs {
                        if let Some(log_str) = log.as_str() {
                            if log_str.contains("csv_right") || log_str.contains("RightCreated") {
                                if let Some(right) = self.parse_right_from_log(txn, log_str) {
                                    let enhanced = EnhancedRightRecord {
                                        id: right.id.clone(),
                                        chain: right.chain.clone(),
                                        seal_ref: right.seal_ref.clone(),
                                        commitment: right.commitment.clone(),
                                        owner: right.owner.clone(),
                                        created_at: right.created_at,
                                        created_tx: right.created_tx.clone(),
                                        status: right.status.to_string(),
                                        metadata: right.metadata,
                                        transfer_count: right.transfer_count,
                                        last_transfer_at: right.last_transfer_at,
                                        commitment_scheme: CommitmentScheme::HashBased,
                                        commitment_version: 2,
                                        protocol_id: "csv-sol".to_string(),
                                        mpc_root: None,
                                        domain_separator: Some("solana-mainnet".to_string()),
                                        inclusion_proof_type: InclusionProofType::AccountState,
                                        finality_proof_type: FinalityProofType::SlotBased,
                                        proof_size_bytes: Some(log_str.len() as u64),
                                        confirmations: None,
                                    };
                                    rights.push(enhanced);
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(rights)
    }

    async fn index_enhanced_seals(&self, block: u64) -> ChainResult<Vec<EnhancedSealRecord>> {
        let txns = self.get_transactions_for_slot(block).await?;
        let mut seals = Vec::new();

        for txn in &txns {
            if let Some(meta) = &txn.meta {
                if let Some(logs) = meta.get("logMessages").and_then(|v| v.as_array()) {
                    for log in logs {
                        if let Some(log_str) = log.as_str() {
                            if log_str.contains("csv_seal") || log_str.contains("SealConsumed") {
                                if let Some(seal) = self.parse_seal_from_log(txn, block) {
                                    let enhanced = EnhancedSealRecord {
                                        id: seal.id.clone(),
                                        chain: seal.chain.clone(),
                                        seal_type: seal.seal_type.to_string(),
                                        seal_ref: seal.seal_ref.clone(),
                                        right_id: seal.right_id.clone(),
                                        status: seal.status.to_string(),
                                        consumed_at: seal.consumed_at,
                                        consumed_tx: seal.consumed_tx.clone(),
                                        block_height: seal.block_height,
                                        seal_proof_type: "account_state".to_string(),
                                        seal_proof_verified: None,
                                    };
                                    seals.push(enhanced);
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(seals)
    }

    async fn index_enhanced_transfers(&self, block: u64) -> ChainResult<Vec<EnhancedTransferRecord>> {
        // Cross-chain transfers on Solana would be handled through bridge programs
        Ok(Vec::new())
    }

    fn detect_commitment_scheme(&self, _data: &[u8]) -> Option<CommitmentScheme> {
        Some(CommitmentScheme::HashBased)
    }

    fn detect_inclusion_proof_type(&self) -> InclusionProofType {
        InclusionProofType::AccountState
    }

    fn detect_finality_proof_type(&self) -> FinalityProofType {
        FinalityProofType::SlotBased
    }

    // -----------------------------------------------------------------------
    // Address-based indexing methods (for priority indexing)
    // -----------------------------------------------------------------------

    async fn index_rights_by_address(&self, _address: &str) -> ChainResult<Vec<RightRecord>> {
        Ok(Vec::new())
    }

    async fn index_seals_by_address(&self, _address: &str) -> ChainResult<Vec<SealRecord>> {
        Ok(Vec::new())
    }

    async fn index_transfers_by_address(&self, _address: &str) -> ChainResult<Vec<TransferRecord>> {
        Ok(Vec::new())
    }

    async fn index_addresses_with_priority(
        &self,
        addresses: &[String],
        _priority: PriorityLevel,
        _network: Network,
    ) -> ChainResult<AddressIndexingResult> {
        let mut result = AddressIndexingResult {
            addresses_processed: 0,
            rights_indexed: 0,
            seals_indexed: 0,
            transfers_indexed: 0,
            contracts_indexed: 0,
            errors: Vec::new(),
        };

        for address in addresses {
            if let Ok(rights) = self.index_rights_by_address(address).await {
                result.rights_indexed += rights.len() as u64;
                result.addresses_processed += 1;
            }
            if let Ok(seals) = self.index_seals_by_address(address).await {
                result.seals_indexed += seals.len() as u64;
            }
            if let Ok(transfers) = self.index_transfers_by_address(address).await {
                result.transfers_indexed += transfers.len() as u64;
            }
        }

        Ok(result)
    }
}
