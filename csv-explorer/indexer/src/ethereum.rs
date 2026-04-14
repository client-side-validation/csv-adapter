/// Ethereum chain indexer implementation.
///
/// Scans Ethereum blocks for CSV contract events including:
/// - RightCreated events
/// - SealConsumed events
/// - CrossChainTransfer events
/// - Contract deployments
/// - Nullifier registry state

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::chain_indexer::{AddressIndexingResult, ChainIndexer, ChainResult};
use csv_explorer_shared::{
    ChainConfig, CommitmentScheme, ContractStatus, ContractType, CsvContract, EnhancedRightRecord,
    EnhancedSealRecord, EnhancedTransferRecord, ExplorerError, FinalityProofType, InclusionProofType,
    Network, PriorityLevel, RightRecord, SealRecord, SealStatus, SealType, TransferRecord,
};

/// Ethereum-specific indexer.
pub struct EthereumIndexer {
    config: ChainConfig,
    http_client: Client,
    /// Known CSV contract addresses on Ethereum.
    csv_contracts: HashMap<String, ContractType>,
}

/// JSON-RPC request body for Ethereum.
#[derive(Debug, Serialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    params: Vec<serde_json::Value>,
    id: u64,
}

/// JSON-RPC response body.
#[derive(Debug, Deserialize)]
struct JsonRpcResponse {
    result: Option<serde_json::Value>,
    error: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct BlockData {
    number: String,
    transactions: Vec<TxData>,
}

#[derive(Debug, Deserialize)]
struct TxData {
    hash: String,
    input: String,
    logs: Option<Vec<LogData>>,
}

#[derive(Debug, Deserialize)]
struct LogData {
    address: String,
    topics: Vec<String>,
    data: String,
    block_number: String,
    transaction_hash: String,
}

// CSV event signatures (keccak256 hashes)
const RIGHT_CREATED_SIG: &str =
    "0x0000000000000000000000000000000000000000000000000000000000000001";
const SEAL_CONSUMED_SIG: &str =
    "0x0000000000000000000000000000000000000000000000000000000000000002";
const CROSS_CHAIN_TRANSFER_SIG: &str =
    "0x0000000000000000000000000000000000000000000000000000000000000003";

#[async_trait]
impl ChainIndexer for EthereumIndexer {
    fn chain_id(&self) -> &str {
        "ethereum"
    }

    fn chain_name(&self) -> &str {
        "Ethereum"
    }

    async fn initialize(&self) -> ChainResult<()> {
        tracing::info!(chain = "ethereum", "Ethereum indexer initialized");
        Ok(())
    }

    async fn get_chain_tip(&self) -> ChainResult<u64> {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "eth_blockNumber".to_string(),
            params: vec![],
            id: 1,
        };

        let resp: JsonRpcResponse = self.http_client
            .post(&self.config.rpc_url)
            .json(&req)
            .send()
            .await?
            .json()
            .await?;

        if let Some(result) = resp.result {
            let hex_str = result.as_str().unwrap_or("0x0");
            let block = u64::from_str_radix(hex_str.trim_start_matches("0x"), 16).unwrap_or(0);
            Ok(block)
        } else {
            Err(ExplorerError::RpcError {
                chain: "ethereum".to_string(),
                message: "Failed to get block number".to_string(),
            })
        }
    }

    async fn get_latest_synced_block(&self) -> ChainResult<u64> {
        Ok(0)
    }

    async fn index_rights(&self, block: u64) -> ChainResult<Vec<RightRecord>> {
        let block_data = self.fetch_block(block).await?;
        let mut rights = Vec::new();

        for tx in &block_data.transactions {
            if let Some(logs) = &tx.logs {
                for log in logs {
                    if log.topics.first().map(|s| s.as_str()) == Some(RIGHT_CREATED_SIG) {
                        if let Some(right) = self.parse_right_from_log(log, &tx.hash) {
                            rights.push(right);
                        }
                    }
                }
            }
        }

        tracing::debug!(chain = "ethereum", block, count = rights.len(), "Indexed rights");
        Ok(rights)
    }

    async fn index_seals(&self, block: u64) -> ChainResult<Vec<SealRecord>> {
        let block_data = self.fetch_block(block).await?;
        let mut seals = Vec::new();

        for tx in &block_data.transactions {
            if let Some(logs) = &tx.logs {
                for log in logs {
                    if log.topics.first().map(|s| s.as_str()) == Some(SEAL_CONSUMED_SIG) {
                        if let Some(seal) = self.parse_seal_from_log(log, block) {
                            seals.push(seal);
                        }
                    }
                }
            }
        }

        tracing::debug!(chain = "ethereum", block, count = seals.len(), "Indexed seals");
        Ok(seals)
    }

    async fn index_transfers(&self, block: u64) -> ChainResult<Vec<TransferRecord>> {
        let block_data = self.fetch_block(block).await?;
        let mut transfers = Vec::new();

        for tx in &block_data.transactions {
            if let Some(logs) = &tx.logs {
                for log in logs {
                    if log.topics.first().map(|s| s.as_str()) == Some(CROSS_CHAIN_TRANSFER_SIG) {
                        if let Some(transfer) = self.parse_transfer_from_log(log, &tx.hash) {
                            transfers.push(transfer);
                        }
                    }
                }
            }
        }

        tracing::debug!(chain = "ethereum", block, count = transfers.len(), "Indexed transfers");
        Ok(transfers)
    }

    async fn index_contracts(&self, block: u64) -> ChainResult<Vec<CsvContract>> {
        // Track known CSV contracts
        let mut contracts = Vec::new();

        for (address, contract_type) in &self.csv_contracts {
            contracts.push(CsvContract {
                id: format!("eth-{}", address),
                chain: "ethereum".to_string(),
                contract_type: *contract_type,
                address: address.clone(),
                deployed_tx: "genesis".to_string(),
                deployed_at: chrono::Utc::now(),
                version: "1.0.0".to_string(),
                status: ContractStatus::Active,
            });
        }

        Ok(contracts)
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
        _priority: csv_explorer_shared::PriorityLevel,
        _network: csv_explorer_shared::Network,
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

impl EthereumIndexer {
    /// Create a new Ethereum indexer.
    pub fn new(config: ChainConfig) -> Self {
        let mut csv_contracts = HashMap::new();
        // In production, these would be loaded from configuration
        csv_contracts.insert("0x0000000000000000000000000000000000000000".to_string(), ContractType::NullifierRegistry);
        csv_contracts.insert("0x0000000000000000000000000000000000000001".to_string(), ContractType::RightRegistry);
        csv_contracts.insert("0x0000000000000000000000000000000000000002".to_string(), ContractType::Bridge);

        Self {
            config,
            http_client: Client::new(),
            csv_contracts,
        }
    }

    /// Fetch block data via JSON-RPC.
    async fn fetch_block(&self, block: u64) -> ChainResult<BlockData> {
        let block_hex = format!("0x{:x}", block);
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "eth_getBlockByNumber".to_string(),
            params: vec![
                serde_json::Value::String(block_hex),
                serde_json::Value::Bool(true), // include transactions
            ],
            id: 1,
        };

        let resp: JsonRpcResponse = self.http_client
            .post(&self.config.rpc_url)
            .json(&req)
            .send()
            .await?
            .json()
            .await?;

        if let Some(result) = resp.result {
            let block_data: BlockData = serde_json::from_value(result).map_err(|e| ExplorerError::RpcParseError {
                chain: "ethereum".to_string(),
                message: e.to_string(),
            })?;
            Ok(block_data)
        } else {
            Err(ExplorerError::RpcError {
                chain: "ethereum".to_string(),
                message: format!("Failed to get block {}", block),
            })
        }
    }

    fn parse_right_from_log(&self, log: &LogData, tx_hash: &str) -> Option<RightRecord> {
        // Parse event topics and data to extract right fields
        if log.topics.len() < 3 {
            return None;
        }

        let right_id = log.topics.get(1).cloned().unwrap_or_default();
        let owner = log.topics.get(2).cloned().unwrap_or_default();

        Some(RightRecord {
            id: right_id.trim_start_matches("0x").to_string(),
            chain: "ethereum".to_string(),
            seal_ref: log.topics.first().cloned().unwrap_or_default(),
            commitment: log.data.clone(),
            owner: owner.trim_start_matches("0x").to_string(),
            created_at: chrono::Utc::now(),
            created_tx: tx_hash.to_string(),
            status: csv_explorer_shared::RightStatus::Active,
            metadata: None,
            transfer_count: 0,
            last_transfer_at: None,
        })
    }

    fn parse_seal_from_log(&self, log: &LogData, block: u64) -> Option<SealRecord> {
        Some(SealRecord {
            id: format!("eth-seal-{}", log.transaction_hash),
            chain: "ethereum".to_string(),
            seal_type: SealType::Nullifier,
            seal_ref: log.data.clone(),
            right_id: None,
            status: SealStatus::Consumed,
            consumed_at: Some(chrono::Utc::now()),
            consumed_tx: Some(log.transaction_hash.clone()),
            block_height: block,
        })
    }

    fn parse_transfer_from_log(&self, log: &LogData, tx_hash: &str) -> Option<TransferRecord> {
        Some(TransferRecord {
            id: format!("eth-xfer-{}", tx_hash),
            right_id: "unknown".to_string(),
            from_chain: "ethereum".to_string(),
            to_chain: "unknown".to_string(),
            from_owner: "unknown".to_string(),
            to_owner: "unknown".to_string(),
            lock_tx: tx_hash.to_string(),
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
        let block_data = self.fetch_block(block).await?;
        let mut rights = Vec::new();

        for tx in &block_data.transactions {
            if let Some(logs) = &tx.logs {
                for log in logs {
                    if log.topics.first().map(|s| s.as_str()) == Some(RIGHT_CREATED_SIG) {
                        if let Some(right) = self.parse_right_from_log(log, &tx.hash) {
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
                                commitment_scheme: CommitmentScheme::KZG,
                                commitment_version: 2,
                                protocol_id: "csv-eth".to_string(),
                                mpc_root: None,
                                domain_separator: Some("ethereum-mainnet".to_string()),
                                inclusion_proof_type: InclusionProofType::MerklePatricia,
                                finality_proof_type: FinalityProofType::FinalizedBlock,
                                proof_size_bytes: Some(log.data.len() as u64),
                                confirmations: None,
                            };
                            rights.push(enhanced);
                        }
                    }
                }
            }
        }

        Ok(rights)
    }

    async fn index_enhanced_seals(&self, block: u64) -> ChainResult<Vec<EnhancedSealRecord>> {
        let block_data = self.fetch_block(block).await?;
        let mut seals = Vec::new();

        for tx in &block_data.transactions {
            if let Some(logs) = &tx.logs {
                for log in logs {
                    if log.topics.first().map(|s| s.as_str()) == Some(SEAL_CONSUMED_SIG) {
                        let seal = EnhancedSealRecord {
                            id: format!("eth-seal-{}", log.transaction_hash),
                            chain: "ethereum".to_string(),
                            seal_type: "nullifier".to_string(),
                            seal_ref: log.data.clone(),
                            right_id: None,
                            status: "consumed".to_string(),
                            consumed_at: Some(chrono::Utc::now()),
                            consumed_tx: Some(log.transaction_hash.clone()),
                            block_height: block,
                            seal_proof_type: "merkle_patricia".to_string(),
                            seal_proof_verified: None,
                        };
                        seals.push(seal);
                    }
                }
            }
        }

        Ok(seals)
    }

    async fn index_enhanced_transfers(&self, block: u64) -> ChainResult<Vec<EnhancedTransferRecord>> {
        let block_data = self.fetch_block(block).await?;
        let mut transfers = Vec::new();

        for tx in &block_data.transactions {
            if let Some(logs) = &tx.logs {
                for log in logs {
                    if log.topics.first().map(|s| s.as_str()) == Some(CROSS_CHAIN_TRANSFER_SIG) {
                        if let Some(transfer) = self.parse_transfer_from_log(log, &tx.hash) {
                            let enhanced = EnhancedTransferRecord {
                                id: transfer.id.clone(),
                                right_id: transfer.right_id.clone(),
                                from_chain: transfer.from_chain.clone(),
                                to_chain: transfer.to_chain.clone(),
                                from_owner: transfer.from_owner.clone(),
                                to_owner: transfer.to_owner.clone(),
                                lock_tx: transfer.lock_tx.clone(),
                                mint_tx: transfer.mint_tx.clone(),
                                proof_ref: transfer.proof_ref.clone(),
                                status: transfer.status.to_string(),
                                created_at: transfer.created_at,
                                completed_at: transfer.completed_at,
                                duration_ms: transfer.duration_ms,
                                cross_chain_proof_type: Some("merkle_patricia".to_string()),
                                bridge_contract: Some(log.address.clone()),
                                bridge_proof_verified: None,
                            };
                            transfers.push(enhanced);
                        }
                    }
                }
            }
        }

        Ok(transfers)
    }

    fn detect_commitment_scheme(&self, _data: &[u8]) -> Option<CommitmentScheme> {
        // Ethereum CSV uses KZG commitments (PLONK-style)
        Some(CommitmentScheme::KZG)
    }

    fn detect_inclusion_proof_type(&self) -> InclusionProofType {
        InclusionProofType::MerklePatricia
    }

    fn detect_finality_proof_type(&self) -> FinalityProofType {
        FinalityProofType::FinalizedBlock
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
