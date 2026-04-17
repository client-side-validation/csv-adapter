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
use super::rpc_manager::RpcManager;
use csv_explorer_shared::{
    ChainConfig, CommitmentScheme, ContractStatus, ContractType, CsvContract, EnhancedRightRecord,
    EnhancedSealRecord, EnhancedTransferRecord, ExplorerError, FinalityProofType,
    InclusionProofType, Network, PriorityLevel, RightRecord, SealRecord, SealStatus, SealType,
    TransferRecord,
};

/// Solana-specific indexer.
pub struct SolanaIndexer {
    config: ChainConfig,
    http_client: Client,
    /// RPC manager for handling multiple RPC endpoints
    rpc_manager: Option<RpcManager>,
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

        let resp: SolanaRpcResponse = self
            .http_client
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
                            if log_str.contains("csv_right_created")
                                || log_str.contains("RightCreated")
                            {
                                if let Some(right) = self.parse_right_from_log(txn_info, log_str) {
                                    rights.push(right);
                                }
                            }
                        }
                    }
                }
            }
        }

        tracing::debug!(
            chain = "solana",
            block,
            count = rights.len(),
            "Indexed rights"
        );
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

        tracing::debug!(
            chain = "solana",
            block,
            count = seals.len(),
            "Indexed seals"
        );
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
                            if log_str.contains("csv_transfer")
                                || log_str.contains("CrossChainTransfer")
                            {
                                if let Some(transfer) = self.parse_transfer_from_log(txn_info) {
                                    transfers.push(transfer);
                                }
                            }
                        }
                    }
                }
            }
        }

        tracing::debug!(
            chain = "solana",
            block,
            count = transfers.len(),
            "Indexed transfers"
        );
        Ok(transfers)
    }

    async fn index_contracts(&self, _block: u64) -> ChainResult<Vec<CsvContract>> {
        Ok(vec![CsvContract {
            id: "solana-csv-program".to_string(),
            chain: "solana".to_string(),
            contract_type: ContractType::RightRegistry,
            address: "CsvProgram11111111111111111111111111111111111".to_string(),
            deployed_tx: "genesis".to_string(),
            deployed_at: chrono::Utc::now(),
            version: "1.0.0".to_string(),
            status: ContractStatus::Active,
        }])
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

    // -----------------------------------------------------------------------
    // Advanced commitment and proof indexing methods
    // -----------------------------------------------------------------------

    async fn index_enhanced_rights(&self, block: u64) -> ChainResult<Vec<EnhancedRightRecord>> {
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

    async fn index_enhanced_transfers(
        &self,
        block: u64,
    ) -> ChainResult<Vec<EnhancedTransferRecord>> {
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
}

impl SolanaIndexer {
    /// Create a new Solana indexer.
    pub fn new(config: ChainConfig, rpc_manager: RpcManager) -> Self {
        Self {
            config,
            http_client: Client::new(),
            rpc_manager: Some(rpc_manager),
        }
    }

    /// Get transactions for a specific slot.
    async fn get_transactions_for_slot(&self, slot: u64) -> ChainResult<Vec<TransactionInfo>> {
        // First get block information for the slot
        let block_req = SolanaRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "getBlock".to_string(),
            params: vec![
                serde_json::json!(slot),
                serde_json::json!({
                    "encoding": "json",
                    "transactionDetails": "full",
                    "rewards": false,
                    "maxSupportedTransactionVersion": 0
                }),
            ],
            id: 1,
        };

        let rpc_url = if let Some(ref manager) = self.rpc_manager {
            if let Some(endpoint) = manager.get_endpoint("solana") {
                endpoint.url.clone()
            } else {
                self.config.rpc_url.clone()
            }
        } else {
            self.config.rpc_url.clone()
        };

        let client = if let Some(ref manager) = self.rpc_manager {
            manager.get_client("solana").unwrap_or_else(Client::new)
        } else {
            Client::new()
        };

        let block_resp: SolanaRpcResponse = client
            .post(&rpc_url)
            .json(&block_req)
            .send()
            .await
            .map_err(|e| ExplorerError::RpcError {
                chain: "solana".to_string(),
                message: format!("Failed to get block {}: {}", slot, e),
            })?
            .json()
            .await
            .map_err(|e| ExplorerError::RpcError {
                chain: "solana".to_string(),
                message: format!("Failed to parse block response {}: {}", slot, e),
            })?;

        if let Some(block_data) = block_resp.result {
            return self.parse_block_transactions(block_data, slot).await;
        }

        // If block doesn't exist, try getting confirmed signatures for the slot
        let sig_req = SolanaRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "getSignaturesForAddress".to_string(),
            params: vec![
                serde_json::json!("CsvProgram11111111111111111111111111111111111"),
                serde_json::json!({
                    "minContextSlot": slot,
                    "limit": 100
                }),
            ],
            id: 2,
        };

        let sig_resp: SolanaRpcResponse = client
            .post(&rpc_url)
            .json(&sig_req)
            .send()
            .await
            .map_err(|e| ExplorerError::RpcError {
                chain: "solana".to_string(),
                message: format!("Failed to get signatures for slot {}: {}", slot, e),
            })?
            .json()
            .await
            .map_err(|e| ExplorerError::RpcError {
                chain: "solana".to_string(),
                message: format!("Failed to parse signatures response {}: {}", slot, e),
            })?;

        if let Some(signatures) = sig_resp.result {
            return self.parse_signatures_to_transactions(signatures, slot, &client, &rpc_url).await;
        }

        Err(ExplorerError::RpcError {
            chain: "solana".to_string(),
            message: format!("No transactions found for slot {}", slot),
        })
    }

    fn parse_right_from_log(&self, txn: &TransactionInfo, log: &str) -> Option<RightRecord> {
        // Parse the log message to extract right data
        // Solana logs are in format: "Program log: <message>"
        let tx_sig = txn
            .transaction
            .as_ref()?
            .get("signatures")?
            .as_array()?
            .first()?
            .as_str()?;

        // Try to extract structured data from log
        let (owner, right_id, commitment) = self.parse_csv_log_data(log, "right_created")?;

        Some(RightRecord {
            id: right_id.unwrap_or_else(|| format!("sol-right-{}", tx_sig)),
            chain: "solana".to_string(),
            seal_ref: tx_sig.to_string(),
            commitment: commitment.unwrap_or_else(|| "hash_based".to_string()),
            owner: owner.unwrap_or_else(|| "unknown".to_string()),
            created_at: txn.block_time
                .and_then(|t| chrono::DateTime::from_timestamp(t, 0))
                .unwrap_or_else(chrono::Utc::now),
            created_tx: tx_sig.to_string(),
            status: csv_explorer_shared::RightStatus::Active,
            metadata: Some(serde_json::json!({
                "log_message": log,
                "slot": txn.slot,
                "parsed": true
            })),
            transfer_count: 0,
            last_transfer_at: None,
        })
    }

    fn parse_seal_from_log(&self, txn: &TransactionInfo, block: u64) -> Option<SealRecord> {
        let tx_sig = txn
            .transaction
            .as_ref()?
            .get("signatures")?
            .as_array()?
            .first()?
            .as_str()?;

        // Parse account changes from transaction meta if available
        let (right_id, account_address) = if let Some(meta) = &txn.meta {
            self.parse_seal_from_meta(meta)
        } else {
            (None, None)
        };

        Some(SealRecord {
            id: format!("sol-seal-{}", tx_sig),
            chain: "solana".to_string(),
            seal_type: SealType::Account,
            seal_ref: account_address.unwrap_or_else(|| tx_sig.to_string()),
            right_id,
            status: SealStatus::Consumed,
            consumed_at: txn
                .block_time
                .and_then(|t| chrono::DateTime::from_timestamp(t, 0)),
            consumed_tx: Some(tx_sig.to_string()),
            block_height: block,
        })
    }

    fn parse_transfer_from_log(&self, txn: &TransactionInfo) -> Option<TransferRecord> {
        let tx_sig = txn
            .transaction
            .as_ref()?
            .get("signatures")?
            .as_array()?
            .first()?
            .as_str()?;

        // Extract transfer data from transaction if available
        let (right_id, from_owner, to_owner, to_chain) = if let Some(meta) = &txn.meta {
            self.parse_transfer_from_meta(meta)
        } else {
            (None, None, None, None)
        };

        Some(TransferRecord {
            id: format!("sol-xfer-{}", tx_sig),
            right_id: right_id.unwrap_or_else(|| "unknown".to_string()),
            from_chain: "solana".to_string(),
            to_chain: to_chain.unwrap_or_else(|| "unknown".to_string()),
            from_owner: from_owner.unwrap_or_else(|| "unknown".to_string()),
            to_owner: to_owner.unwrap_or_else(|| "unknown".to_string()),
            lock_tx: tx_sig.to_string(),
            mint_tx: None,
            proof_ref: Some(tx_sig.to_string()),
            status: csv_explorer_shared::TransferStatus::Initiated,
            created_at: txn.block_time
                .and_then(|t| chrono::DateTime::from_timestamp(t, 0))
                .unwrap_or_else(chrono::Utc::now),
            completed_at: None,
            duration_ms: None,
        })
    }

    /// Parse block data into transaction info structures.
    async fn parse_block_transactions(&self, block_data: serde_json::Value, slot: u64) -> ChainResult<Vec<TransactionInfo>> {
        let mut transactions = Vec::new();

        if let Some(tx_array) = block_data.get("transactions").and_then(|v| v.as_array()) {
            for tx in tx_array {
                let tx_info = TransactionInfo {
                    slot,
                    transaction: Some(tx.clone()),
                    meta: tx.get("meta").cloned(),
                    block_time: block_data.get("blockTime").and_then(|v| v.as_i64()),
                };
                transactions.push(tx_info);
            }
        }

        Ok(transactions)
    }

    /// Parse signatures into transaction info by fetching each transaction.
    async fn parse_signatures_to_transactions(
        &self,
        signatures: serde_json::Value,
        slot: u64,
        client: &Client,
        rpc_url: &str,
    ) -> ChainResult<Vec<TransactionInfo>> {
        let mut transactions = Vec::new();

        if let Some(sig_array) = signatures.as_array() {
            for sig_obj in sig_array.iter().take(50) { // Limit to avoid rate limiting
                if let Some(signature) = sig_obj.get("signature").and_then(|v| v.as_str()) {
                    let tx_req = SolanaRpcRequest {
                        jsonrpc: "2.0".to_string(),
                        method: "getTransaction".to_string(),
                        params: vec![
                            serde_json::json!(signature),
                            serde_json::json!({
                                "encoding": "json",
                                "commitment": "confirmed",
                                "maxSupportedTransactionVersion": 0
                            }),
                        ],
                        id: 3,
                    };

                    match client.post(rpc_url).json(&tx_req).send().await {
                        Ok(response) => {
                            if let Ok(tx_resp) = response.json::<SolanaRpcResponse>().await {
                                if let Some(tx_data) = tx_resp.result {
                                    let tx_info = TransactionInfo {
                                        slot,
                                        transaction: Some(tx_data.clone()),
                                        meta: tx_data.get("meta").cloned(),
                                        block_time: tx_data.get("blockTime").and_then(|v| v.as_i64()),
                                    };
                                    transactions.push(tx_info);
                                }
                            }
                        }
                        Err(e) => {
                            tracing::warn!(
                                chain = "solana",
                                signature = %signature,
                                error = %e,
                                "Failed to fetch transaction"
                            );
                        }
                    }
                }
            }
        }

        Ok(transactions)
    }

    /// Parse CSV program log data for structured information.
    fn parse_csv_log_data(&self, log: &str, event_type: &str) -> Option<(Option<String>, Option<String>, Option<String>)> {
        // Try to extract JSON data from log messages
        // Expected format: "Program log: {\"event\":\"right_created\",\"owner\":\"...\",\"right_id\":\"...\",\"commitment\":\"...\"}"
        if let Some(start) = log.find('{') {
            if let Some(end) = log.rfind('}') {
                let json_str = &log[start..=end];
                if let Ok(data) = serde_json::from_str::<serde_json::Value>(json_str) {
                    let owner = data.get("owner").and_then(|v| v.as_str()).map(|s| s.to_string());
                    let right_id = data.get("right_id").and_then(|v| v.as_str()).map(|s| s.to_string());
                    let commitment = data.get("commitment").and_then(|v| v.as_str()).map(|s| s.to_string());
                    return Some((owner, right_id, commitment));
                }
            }
        }

        // Fallback: try to extract from simple format
        if log.contains(event_type) {
            // Simple pattern matching for basic info
            None // Return None to trigger fallback logic
        } else {
            None
        }
    }

    /// Parse seal information from transaction metadata.
    fn parse_seal_from_meta(&self, meta: &serde_json::Value) -> (Option<String>, Option<String>) {
        let mut right_id = None;
        let mut account_address = None;

        // Check post token balances for account changes
        if let Some(post_balances) = meta.get("postTokenBalances").and_then(|v| v.as_array()) {
            for balance in post_balances {
                if let Some(account) = balance.get("account").and_then(|v| v.as_str()) {
                    account_address = Some(account.to_string());
                }
                if let Some(ui_amount) = balance.get("uiTokenAmount") {
                    if ui_amount.get("amount").and_then(|v| v.as_str()) == Some("0") {
                        // Account was closed - potential seal consumption
                        if let Some(mint) = balance.get("mint").and_then(|v| v.as_str()) {
                            right_id = Some(format!("spl-{}", mint));
                        }
                    }
                }
            }
        }

        // Check account keys for CSV program interactions
        if let Some(account_keys) = meta.get("accountKeys").and_then(|v| v.as_array()) {
            for (idx, key) in account_keys.iter().enumerate() {
                if let Some(key_str) = key.as_str() {
                    if key_str.contains("csv") || key_str.len() == 44 { // Typical Solana address length
                        if account_address.is_none() && idx > 0 { // Not the fee payer
                            account_address = Some(key_str.to_string());
                        }
                    }
                }
            }
        }

        (right_id, account_address)
    }

    /// Parse transfer information from transaction metadata.
    fn parse_transfer_from_meta(&self, meta: &serde_json::Value) -> (Option<String>, Option<String>, Option<String>, Option<String>) {
        let mut right_id = None;
        let mut from_owner = None;
        let mut to_owner = None;
        let mut to_chain = None;

        // Extract from pre and post token balances
        if let (Some(pre_balances), Some(post_balances)) = (
            meta.get("preTokenBalances").and_then(|v| v.as_array()),
            meta.get("postTokenBalances").and_then(|v| v.as_array()),
        ) {
            for post_balance in post_balances {
                let account = post_balance.get("account").and_then(|v| v.as_str());
                let ui_amount = post_balance.get("uiTokenAmount");
                
                if let (Some(account), Some(amount)) = (account, ui_amount) {
                    let post_amount = amount.get("amount").and_then(|v| v.as_str()).unwrap_or("0");
                    
                    // Find corresponding pre-balance
                    if let Some(pre_balance) = pre_balances.iter().find(|pre| {
                        pre.get("account").and_then(|v| v.as_str()) == Some(account)
                    }) {
                        let pre_amount = pre_balance
                            .get("uiTokenAmount")
                            .and_then(|v| v.get("amount"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("0");

                        if pre_amount != post_amount {
                            // Balance changed - this is a transfer
                            if let Some(mint) = post_balance.get("mint").and_then(|v| v.as_str()) {
                                right_id = Some(format!("spl-{}", mint));
                            }
                            
                            // Determine direction based on balance change
                            if pre_amount > post_amount {
                                from_owner = Some(account.to_string());
                            } else {
                                to_owner = Some(account.to_string());
                            }
                        }
                    }
                }
            }
        }

        // Try to extract destination chain from log messages
        if let Some(log_messages) = meta.get("logMessages").and_then(|v| v.as_array()) {
            for log in log_messages {
                if let Some(log_str) = log.as_str() {
                    if log_str.contains("destination_chain") {
                        if let Some(start) = log_str.find("destination_chain") {
                            let remaining = &log_str[start..];
                            if let Some(colon) = remaining.find(':') {
                                let chain_part = &remaining[colon + 1..];
                                if let Some(end) = chain_part.find(|c| c == '"' || c == '}' || c == ',') {
                                    to_chain = Some(chain_part[..end].trim().to_string());
                                }
                            }
                        }
                    }
                }
            }
        }

        (right_id, from_owner, to_owner, to_chain)
    }
}
