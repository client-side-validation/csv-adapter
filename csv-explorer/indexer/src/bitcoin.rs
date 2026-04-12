/// Bitcoin chain indexer implementation.
///
/// Scans Bitcoin blocks for CSV-related transactions including:
/// - OP_RETURN outputs containing commitment hashes
/// - Tapret commitments in taproot outputs
/// - UTXO state tracking for seal consumption
/// - Right creation from specific output patterns

use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use std::sync::Arc;

use super::chain_indexer::{ChainIndexer, ChainResult};
use shared::{ChainConfig, CsvContract, ExplorerError, RightRecord, SealRecord, SealStatus, SealType, TransferRecord};

/// Bitcoin-specific indexer.
pub struct BitcoinIndexer {
    config: ChainConfig,
    http_client: Client,
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
        let block_data = self.fetch_block(block).await?;
        let mut seals = Vec::new();

        for tx in &block_data.tx {
            // Track UTXO consumption for seal tracking
            for (vin_idx, vin) in tx.vin.iter().enumerate() {
                if let (Some(ref prev_txid), Some(prev_vout)) = (&vin.txid, vin.vout) {
                    let seal = SealRecord {
                        id: format!("btc-{}-{}-{}", prev_txid, prev_vout, tx.txid),
                        chain: "bitcoin".to_string(),
                        seal_type: SealType::Utxo,
                        seal_ref: format!("{}:{}", prev_txid, prev_vout),
                        right_id: None,
                        status: SealStatus::Consumed,
                        consumed_at: Some(chrono::Utc::now()),
                        consumed_tx: Some(tx.txid.clone()),
                        block_height: block,
                    };
                    seals.push(seal);
                }
            }
        }

        tracing::debug!(chain = "bitcoin", block, count = seals.len(), "Indexed seals");
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
}

impl BitcoinIndexer {
    /// Create a new Bitcoin indexer.
    pub fn new(config: ChainConfig) -> Self {
        Self {
            config,
            http_client: Client::new(),
        }
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
        // In a real implementation, this would parse the OP_RETURN data
        // to extract commitment hash, right_id, owner, etc.
        let _ = (vout, block);
        // Placeholder: real parsing logic would go here
        if vout.scriptpubkey_type.as_deref() == Some("op_return") {
            Some(RightRecord {
                id: format!("btc-right-{}", tx.txid),
                chain: "bitcoin".to_string(),
                seal_ref: format!("{}:0", tx.txid),
                commitment: "pending_parse".to_string(),
                owner: "unknown".to_string(),
                created_at: chrono::Utc::now(),
                created_tx: tx.txid.clone(),
                status: shared::RightStatus::Active,
                metadata: None,
                transfer_count: 0,
                last_transfer_at: None,
            })
        } else {
            None
        }
    }
}
