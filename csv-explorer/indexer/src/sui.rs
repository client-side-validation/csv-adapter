/// Sui chain indexer implementation.
///
/// Subscribes to Sui checkpoint events and tracks:
/// - Object creation/deletion for seals
/// - Move events from CSV packages
/// - Checkpoint finality
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::chain_indexer::ChainIndexer;
use super::chain_indexer::ChainResult;
use super::rpc_manager::RpcManager;
use csv_explorer_shared::{
    ChainConfig, CommitmentScheme, ContractStatus, ContractType, CsvContract, EnhancedRightRecord,
    EnhancedSealRecord, EnhancedTransferRecord, ExplorerError, FinalityProofType,
    InclusionProofType, Network, PriorityLevel, RightRecord, SealRecord, SealStatus, SealType,
    TransferRecord,
};

use crate::chain_indexer::AddressIndexingResult;

/// Sui-specific indexer.
pub struct SuiIndexer {
    config: ChainConfig,
    rpc_manager: Option<Arc<RpcManager>>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
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
#[allow(dead_code)]
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
        let rpc_url = if let Some(ref manager) = self.rpc_manager {
            if let Some(endpoint) = manager.get_endpoint("sui") {
                endpoint.url.clone()
            } else {
                self.config.rpc_url.clone()
            }
        } else {
            self.config.rpc_url.clone()
        };

        let req = SuiRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "suix_getLatestCheckpointSequenceNumber".to_string(),
            params: vec![],
            id: 1,
        };

        let client = self
            .rpc_manager
            .as_ref()
            .and_then(|m| m.get_client("sui"))
            .unwrap_or_default();
        let resp: serde_json::Value = client
            .post(&rpc_url)
            .json(&req)
            .send()
            .await?
            .json()
            .await?;

        if let Some(result) = resp.get("result") {
            // Handle both string and number formats from Sui RPC
            let checkpoint_num = if let Some(s) = result.as_str() {
                s.parse::<u64>().unwrap_or(0)
            } else if let Some(n) = result.as_u64() {
                n
            } else {
                0
            };
            Ok(checkpoint_num)
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

        for txn in &checkpoint.transactions {
            if let Some(events) = &txn.events {
                for event in events {
                    // Match AnchorEvent or RightCreated events from CSV packages
                    // Pattern: {package_id}::csv_seal::AnchorEvent or similar
                    if event.type_.contains("csv_seal") && event.type_.contains("AnchorEvent") {
                        if let Some(right) = self.parse_right_from_event(event, &txn.digest) {
                            rights.push(right);
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

        for txn in &checkpoint.transactions {
            if let Some(events) = &txn.events {
                for event in events {
                    // Match seal-related events from CSV packages
                    if event.type_.contains("csv_seal") || event.type_.contains("Seal") {
                        if let Some(seal) = self.parse_seal_from_event(event, block) {
                            seals.push(seal);
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

        for txn in &checkpoint.transactions {
            if let Some(events) = &txn.events {
                for event in events {
                    // Match cross-chain transfer events
                    if event.type_.contains("CrossChain") || event.type_.contains("bridge") {
                        if let Some(transfer) = self.parse_transfer_from_event(event, &txn.digest) {
                            transfers.push(transfer);
                        }
                    }
                }
            }
        }

        tracing::debug!(
            chain = "sui",
            block,
            count = transfers.len(),
            "Indexed transfers"
        );
        Ok(transfers)
    }

    async fn index_contracts(&self, _block: u64) -> ChainResult<Vec<CsvContract>> {
        // Return known CSV packages on Sui
        Ok(vec![CsvContract {
            id: "sui-csv-package".to_string(),
            chain: "sui".to_string(),
            contract_type: ContractType::RightRegistry,
            address: "0x0000000000000000000000000000000000000000000000000000000000000000"
                .to_string(),
            deployed_tx: "genesis".to_string(),
            deployed_at: chrono::Utc::now(),
            version: "1.0.0".to_string(),
            status: ContractStatus::Active,
        }])
    }

    // -----------------------------------------------------------------------
    // Advanced commitment and proof indexing methods
    // -----------------------------------------------------------------------

    async fn index_enhanced_rights(&self, block: u64) -> ChainResult<Vec<EnhancedRightRecord>> {
        let checkpoint = self.fetch_checkpoint(block).await?;
        let mut rights = Vec::new();

        for txn in &checkpoint.transactions {
            if let Some(events) = &txn.events {
                for event in events {
                    if event.type_.contains("csv_seal") && event.type_.contains("AnchorEvent") {
                        if let Some(right) = self.parse_right_from_event(event, &txn.digest) {
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
                                protocol_id: "csv-sui".to_string(),
                                mpc_root: None,
                                domain_separator: Some("sui-mainnet".to_string()),
                                inclusion_proof_type: InclusionProofType::ObjectProof,
                                finality_proof_type: FinalityProofType::Checkpoint,
                                proof_size_bytes: None,
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
        let checkpoint = self.fetch_checkpoint(block).await?;
        let mut seals = Vec::new();

        for txn in &checkpoint.transactions {
            if let Some(events) = &txn.events {
                for event in events {
                    if event.type_.contains("csv_seal") || event.type_.contains("Seal") {
                        if let Some(seal) = self.parse_seal_from_event(event, block) {
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
                                seal_proof_type: "object_proof".to_string(),
                                seal_proof_verified: None,
                            };
                            seals.push(enhanced);
                        }
                    }
                }
            }
        }

        Ok(seals)
    }

    async fn index_enhanced_transfers(
        &self,
        block: u64,
    ) -> ChainResult<Vec<EnhancedTransferRecord>> {
        let checkpoint = self.fetch_checkpoint(block).await?;
        let mut transfers = Vec::new();

        for txn in &checkpoint.transactions {
            if let Some(events) = &txn.events {
                for event in events {
                    if event.type_.contains("CrossChain") || event.type_.contains("bridge") {
                        if let Some(transfer) = self.parse_transfer_from_event(event, &txn.digest) {
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
                                cross_chain_proof_type: Some("object_proof".to_string()),
                                bridge_contract: Some(event.package_id.clone()),
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
        Some(CommitmentScheme::HashBased)
    }

    fn detect_inclusion_proof_type(&self) -> InclusionProofType {
        InclusionProofType::ObjectProof
    }

    fn detect_finality_proof_type(&self) -> FinalityProofType {
        FinalityProofType::Checkpoint
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
        _addresses: &[String],
        _priority: PriorityLevel,
        _network: Network,
    ) -> ChainResult<AddressIndexingResult> {
        Ok(AddressIndexingResult {
            addresses_processed: 0,
            rights_indexed: 0,
            seals_indexed: 0,
            transfers_indexed: 0,
            contracts_indexed: 0,
            errors: Vec::new(),
        })
    }
}

impl SuiIndexer {
    /// Create a new Sui indexer with RPC manager support.
    pub fn new(config: ChainConfig, rpc_manager: RpcManager) -> Self {
        Self {
            config,
            rpc_manager: Some(Arc::new(rpc_manager)),
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

        let rpc_url = if let Some(ref manager) = self.rpc_manager {
            if let Some(endpoint) = manager.get_endpoint("sui") {
                endpoint.url.clone()
            } else {
                self.config.rpc_url.clone()
            }
        } else {
            self.config.rpc_url.clone()
        };

        let client = if let Some(ref manager) = self.rpc_manager {
            manager.get_client("sui").unwrap_or_else(Client::new)
        } else {
            Client::new()
        };

        let resp: serde_json::Value = client
            .post(&rpc_url)
            .json(&req)
            .send()
            .await?
            .json()
            .await?;

        if let Some(result) = resp.get("result") {
            let checkpoint: CheckpointData =
                serde_json::from_value(result.clone()).map_err(|e| {
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
            status: csv_explorer_shared::RightStatus::Active,
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

    fn parse_transfer_from_event(
        &self,
        event: &EventData,
        tx_digest: &str,
    ) -> Option<TransferRecord> {
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
            status: csv_explorer_shared::TransferStatus::Initiated,
            created_at: chrono::Utc::now(),
            completed_at: None,
            duration_ms: None,
        })
    }
}
