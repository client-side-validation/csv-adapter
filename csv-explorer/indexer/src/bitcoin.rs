/// Bitcoin chain indexer implementation.
///
/// Scans Bitcoin blocks for CSV-related transactions ONLY:
/// - Transactions involving known CSV contracts/addresses
/// - OP_RETURN outputs containing commitment hashes (right creation)
/// - UTXO spending from known CSV-related addresses (seal consumption)
/// - Transactions involving wallet-registered priority addresses
///
/// **IMPORTANT**: This indexer does NOT index all Bitcoin transactions.
/// It only tracks CSV protocol-related data to avoid database bloat.

use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashSet;
use std::sync::Arc;

use super::chain_indexer::{AddressIndexingResult, ChainIndexer, ChainResult};
use csv_explorer_shared::{
    ChainConfig, CommitmentScheme, CsvContract, EnhancedRightRecord, EnhancedSealRecord,
    EnhancedTransferRecord, ExplorerError, FinalityProofType, InclusionProofType, Network,
    PriorityLevel, RightRecord, SealRecord, SealStatus, SealType, TransferRecord,
};

/// Bitcoin-specific indexer.
pub struct BitcoinIndexer {
    config: ChainConfig,
    http_client: Client,
    /// Known CSV-related addresses (contract addresses, registry addresses, etc.)
    csv_addresses: HashSet<String>,
    /// Priority addresses registered by wallets
    priority_addresses: HashSet<String>,
}

#[derive(Debug, Deserialize)]
struct BlockInfo {
    height: u64,
    tx: Vec<TxInfo>,
}

#[derive(Debug, Deserialize)]
struct TxInfo {
    txid: String,
    vout: Vec<VoutInfo>,
    vin: Vec<VinInfo>,
}

#[derive(Debug, Deserialize)]
struct VoutInfo {
    scriptpubkey: Option<String>,
    scriptpubkey_type: Option<String>,
    value: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct VinInfo {
    txid: Option<String>,
    vout: Option<u32>,
}

#[async_trait]
impl ChainIndexer for BitcoinIndexer {
    fn chain_id(&self) -> &str {
        "bitcoin"
    }

    fn chain_name(&self) -> &str {
        "Bitcoin"
    }

    async fn initialize(&self) -> ChainResult<()> {
        tracing::info!(chain = "bitcoin", "Bitcoin indexer initialized");
        Ok(())
    }

    async fn get_chain_tip(&self) -> ChainResult<u64> {
        let url = format!("{}/blocks/tip/height", self.config.rpc_url);
        let height: u64 = self.http_client.get(&url).send().await?.json().await?;
        Ok(height)
    }

    async fn get_latest_synced_block(&self) -> ChainResult<u64> {
        // This should be loaded from the sync repository; returning 0 as default
        Ok(0)
    }

    async fn index_rights(&self, block: u64) -> ChainResult<Vec<RightRecord>> {
        let block_data = self.fetch_block(block).await?;
        let mut rights = Vec::new();

        for tx in &block_data.tx {
            // Scan for OP_RETURN with commitment data
            for vout in &tx.vout {
                if let Some(script) = &vout.scriptpubkey_type {
                    if script == "op_return" {
                        if let Some(right) = self.parse_right_from_op_return(tx, vout, block).await {
                            rights.push(right);
                        }
                    }
                }
            }
        }

        tracing::debug!(chain = "bitcoin", block, count = rights.len(), "Indexed rights");
        Ok(rights)
    }

    async fn index_seals(&self, block: u64) -> ChainResult<Vec<SealRecord>> {
        // Only index seals for CSV-related and priority addresses
        // Do NOT index every UTXO spend!
        let block_data = self.fetch_block(block).await?;
        let mut seals = Vec::new();

        // Merge CSV and priority addresses
        let relevant_addresses: HashSet<&str> = self
            .csv_addresses
            .iter()
            .map(|a| a.as_str())
            .chain(self.priority_addresses.iter().map(|a| a.as_str()))
            .collect();

        if relevant_addresses.is_empty() {
            // No addresses to track, skip indexing
            tracing::debug!(chain = "bitcoin", block, "No relevant addresses to index");
            return Ok(seals);
        }

        for tx in &block_data.tx {
            // Check if transaction involves our relevant addresses
            let involves_relevant_address = tx.vout.iter().any(|vout| {
                // In real implementation, check if scriptpubkey matches our addresses
                // For now, placeholder
                false
            }) || tx.vin.iter().any(|vin| {
                // Check if spending from a relevant previous output
                false
            });

            if !involves_relevant_address {
                // Skip transactions not related to CSV
                continue;
            }

            // Only track UTXO consumption for CSV-related addresses
            for vin in &tx.vin {
                if let (Some(ref prev_txid), Some(prev_vout)) = (&vin.txid, vin.vout) {
                    // In real implementation, verify the previous output belongs to a CSV address
                    // For now, we only create seal records for known relevant transactions
                }
            }
        }

        tracing::debug!(chain = "bitcoin", block, count = seals.len(), "Indexed seals (CSV-related only)");
        Ok(seals)
    }

    async fn index_transfers(&self, _block: u64) -> ChainResult<Vec<TransferRecord>> {
        // Bitcoin transfers are tracked through cross-chain bridge events
        // This would parse specific transaction patterns indicating transfers
        Ok(Vec::new())
    }

    async fn index_contracts(&self, _block: u64) -> ChainResult<Vec<CsvContract>> {
        // Bitcoin doesn't have smart contracts in the traditional sense
        // This could track specific script patterns or taproot programs
        Ok(Vec::new())
    }

    // -----------------------------------------------------------------------
    // Advanced commitment and proof indexing methods
    // -----------------------------------------------------------------------

    async fn index_enhanced_rights(
        &self,
        block: u64,
    ) -> ChainResult<Vec<EnhancedRightRecord>> {
        // Index rights with commitment scheme detection
        let block_data = self.fetch_block(block).await?;
        let mut rights = Vec::new();

        for tx in &block_data.tx {
            for vout in &tx.vout {
                if let Some(script) = &vout.scriptpubkey_type {
                    if script == "op_return" {
                        if let Some(right) = self.parse_right_from_op_return(tx, vout, block).await {
                            // Detect commitment scheme from OP_RETURN data
                            let scheme = self.detect_commitment_scheme(&[]).unwrap_or(CommitmentScheme::HashBased);

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
                                commitment_scheme: scheme,
                                commitment_version: 2,
                                protocol_id: "csv-btc".to_string(),
                                mpc_root: None,
                                domain_separator: Some("bitcoin-mainnet".to_string()),
                                inclusion_proof_type: InclusionProofType::Merkle,
                                finality_proof_type: FinalityProofType::ConfirmationDepth,
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

    async fn index_enhanced_seals(
        &self,
        block: u64,
    ) -> ChainResult<Vec<EnhancedSealRecord>> {
        let block_data = self.fetch_block(block).await?;
        let mut seals = Vec::new();

        for tx in &block_data.tx {
            for vin in &tx.vin {
                if let (Some(ref prev_txid), Some(prev_vout)) = (&vin.txid, vin.vout) {
                    let seal = EnhancedSealRecord {
                        id: format!("btc-{}-{}-{}", prev_txid, prev_vout, tx.txid),
                        chain: "bitcoin".to_string(),
                        seal_type: "utxo".to_string(),
                        seal_ref: format!("{}:{}", prev_txid, prev_vout),
                        right_id: None,
                        status: "consumed".to_string(),
                        consumed_at: Some(chrono::Utc::now()),
                        consumed_tx: Some(tx.txid.clone()),
                        block_height: block,
                        seal_proof_type: "merkle".to_string(),
                        seal_proof_verified: None,
                    };
                    seals.push(seal);
                }
            }
        }

        Ok(seals)
    }

    async fn index_enhanced_transfers(
        &self,
        _block: u64,
    ) -> ChainResult<Vec<EnhancedTransferRecord>> {
        Ok(Vec::new())
    }

    fn detect_commitment_scheme(&self, _data: &[u8]) -> Option<CommitmentScheme> {
        // For Bitcoin, default to hash-based (SHA-256 commitments)
        Some(CommitmentScheme::HashBased)
    }

    fn detect_inclusion_proof_type(&self) -> InclusionProofType {
        InclusionProofType::Merkle
    }

    fn detect_finality_proof_type(&self) -> FinalityProofType {
        FinalityProofType::ConfirmationDepth
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

impl BitcoinIndexer {
    /// Create a new Bitcoin indexer.
    pub fn new(config: ChainConfig) -> Self {
        Self {
            config,
            http_client: Client::new(),
            csv_addresses: HashSet::new(),
            priority_addresses: HashSet::new(),
        }
    }

    /// Register a known CSV-related address to track.
    pub fn register_csv_address(&mut self, address: String) {
        self.csv_addresses.insert(address);
    }

    /// Register a priority address from wallet.
    pub fn register_priority_address(&mut self, address: String) {
        self.priority_addresses.insert(address);
    }

    /// Remove a priority address.
    pub fn unregister_priority_address(&mut self, address: &str) {
        self.priority_addresses.remove(address);
    }

    /// Fetch block data from the mempool.space API.
    async fn fetch_block(&self, block: u64) -> ChainResult<BlockInfo> {
        let url = format!("{}/block/{}", self.config.rpc_url, block);
        // In a real implementation, this would fetch the block hash first,
        // then fetch all transactions. Simplified here.
        let block_hash: String = self.http_client.get(&url).send().await?.text().await?;
        let tx_url = format!("{}/block/{}/txids", self.config.rpc_url, block_hash);
        // Full implementation would fetch each tx
        let _txids: Vec<String> = self.http_client.get(&tx_url).send().await?.json().await?;

        // Return stub data for structure demonstration
        Ok(BlockInfo {
            height: block,
            tx: Vec::new(),
        })
    }

    /// Parse a right from an OP_RETURN output.
    async fn parse_right_from_op_return(
        &self,
        tx: &TxInfo,
        vout: &VoutInfo,
        block: u64,
    ) -> Option<RightRecord> {
        // CSV commitment patterns in OP_RETURN:
        // Tapret: OP_RETURN <65 bytes> = protocol_id(32) || nonce(1) || commitment(32)
        // Opret:  OP_RETURN <64 bytes> = protocol_id(32) || commitment(32)
        
        let script_hex = vout.scriptpubkey.as_ref()?;
        
        // Extract OP_RETURN payload (skip "6a" OP_RETURN opcode)
        let payload_hex = script_hex.strip_prefix("6a")?;
        let payload = hex::decode(payload_hex).ok()?;
        
        // Check for CSV commitment patterns (64 or 65 bytes)
        if payload.len() != 64 && payload.len() != 65 {
            return None;
        }
        
        // Extract protocol_id (first 32 bytes)
        let protocol_id = &payload[0..32];
        
        // Check if this is a known CSV protocol ID
        // CSV protocol IDs start with "CSV-" prefix
        if &protocol_id[0..4] != b"CSV-" {
            return None;
        }
        
        // Extract commitment hash (last 32 bytes, or bytes 33-64 for 65-byte with nonce)
        let commitment_hash = if payload.len() == 65 {
            // Tapret with nonce: protocol_id(32) || nonce(1) || commitment(32)
            &payload[33..65]
        } else {
            // Opret: protocol_id(32) || commitment(32)
            &payload[32..64]
        };
        
        // Derive owner from the input (seal UTXO spender)
        let owner = tx.vin.first()
            .and_then(|vin| vin.txid.as_ref())
            .map(|txid| format!("btc-{}", &txid[..8]))
            .unwrap_or_else(|| "unknown".to_string());
        
        let protocol_str = String::from_utf8_lossy(protocol_id).trim_end_matches('\0').to_string();
        let commitment_hex = hex::encode(commitment_hash);
        
        Some(RightRecord {
            id: format!("btc-{}-{}", tx.txid, commitment_hex[..16].to_string()),
            chain: "bitcoin".to_string(),
            seal_ref: format!("{}:0", tx.txid),
            commitment: commitment_hex,
            owner,
            created_at: chrono::Utc::now(),
            created_tx: tx.txid.clone(),
            status: csv_explorer_shared::RightStatus::Active,
            metadata: Some(serde_json::json!({
                "protocol_id": protocol_str,
                "commitment_scheme": "hash_based",
                "inclusion_proof": "merkle"
            })),
            transfer_count: 0,
            last_transfer_at: None,
        })
    }
}
