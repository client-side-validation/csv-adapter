/// Aptos chain indexer implementation.
///
/// Subscribes to Aptos ledger state updates and tracks:
/// - Resource creation/destruction for seals
/// - Move events from CSV modules
/// - Transaction finality

use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;

use super::chain_indexer::{AddressIndexingResult, ChainIndexer, ChainResult};
use csv_explorer_shared::{
    ChainConfig, CommitmentScheme, ContractStatus, ContractType, CsvContract, EnhancedRightRecord,
    EnhancedSealRecord, EnhancedTransferRecord, ExplorerError, FinalityProofType, InclusionProofType,
    Network, PriorityLevel, RightRecord, SealRecord, SealStatus, SealType, TransferRecord,
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
                    // Match AnchorEvent from CSV modules
                    // Pattern: {module_address}::csv_seal::AnchorEvent
                    if event.type_.contains("csv_seal") && event.type_.contains("AnchorEvent") {
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
                    // Match seal-related events from CSV modules
                    if event.type_.contains("csv_seal") || event.type_.contains("Seal") {
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
                    // Match cross-chain transfer events
                    if event.type_.contains("CrossChain") || event.type_.contains("bridge_transfer") {
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

    // -----------------------------------------------------------------------
    // Advanced commitment and proof indexing methods
    // -----------------------------------------------------------------------

    async fn index_enhanced_rights(&self, block: u64) -> ChainResult<Vec<EnhancedRightRecord>> {
        let txns = self.fetch_block_transactions(block).await?;
        let mut rights = Vec::new();

        for txn in &txns {
            if let Some(events) = &txn.events {
                for event in events {
                    if event.type_.contains("csv_seal") && event.type_.contains("AnchorEvent") {
                        if let Some(right) = self.parse_right_from_event(event, &txn.hash) {
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
                                protocol_id: "csv-apt".to_string(),
                                mpc_root: None,
                                domain_separator: Some("aptos-mainnet".to_string()),
                                inclusion_proof_type: InclusionProofType::Accumulator,
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
        let txns = self.fetch_block_transactions(block).await?;
        let mut seals = Vec::new();

        for txn in &txns {
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
                                seal_proof_type: "accumulator".to_string(),
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

    async fn index_enhanced_transfers(&self, block: u64) -> ChainResult<Vec<EnhancedTransferRecord>> {
        let txns = self.fetch_block_transactions(block).await?;
        let mut transfers = Vec::new();

        for txn in &txns {
            if let Some(events) = &txn.events {
                for event in events {
                    if event.type_.contains("CrossChain") || event.type_.contains("bridge_transfer") {
                        if let Some(transfer) = self.parse_transfer_from_event(event, &txn.hash) {
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
                                cross_chain_proof_type: Some("accumulator".to_string()),
                                bridge_contract: Some("0x1::csv_bridge".to_string()),
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
        InclusionProofType::Accumulator
    }

    fn detect_finality_proof_type(&self) -> FinalityProofType {
        FinalityProofType::Checkpoint
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
            status: csv_explorer_shared::RightStatus::Active,
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
            status: csv_explorer_shared::TransferStatus::Pending,
            created_at: chrono::Utc::now(),
            completed_at: None,
            duration_ms: None,
        })
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
