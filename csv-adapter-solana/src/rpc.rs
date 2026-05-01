//! RPC client for Solana adapter

#[cfg(feature = "rpc")]
use std::time::Duration;

use solana_sdk::{
    account::Account, pubkey::Pubkey, signature::Signature, transaction::Transaction,
};

use crate::error::{SolanaError, SolanaResult};
use crate::types::{AccountChange, ConfirmationStatus};

/// Trait for Solana RPC operations
#[async_trait::async_trait]
pub trait SolanaRpc: Send + Sync {
    /// Get account info
    async fn get_account(&self, pubkey: &Pubkey) -> SolanaResult<Account>;

    /// Get multiple accounts
    async fn get_multiple_accounts(&self, pubkeys: &[Pubkey])
        -> SolanaResult<Vec<Option<Account>>>;

    /// Get transaction with status
    async fn get_transaction(&self, signature: &Signature) -> SolanaResult<String>;

    /// Send transaction
    async fn send_transaction(&self, transaction: &Transaction) -> SolanaResult<Signature>;

    /// Get latest slot
    async fn get_latest_slot(&self) -> SolanaResult<u64>;

    /// Get slot with commitment
    async fn get_slot_with_commitment(&self, commitment: &str) -> SolanaResult<u64>;

    /// Get account changes between slots
    async fn get_account_changes(
        &self,
        from_slot: u64,
        to_slot: u64,
    ) -> SolanaResult<Vec<AccountChange>>;

    /// Wait for transaction confirmation
    async fn wait_for_confirmation(
        &self,
        signature: &Signature,
    ) -> SolanaResult<ConfirmationStatus>;

    /// Get minimum balance for rent exemption
    async fn get_minimum_balance_for_rent_exemption(&self, data_len: usize) -> SolanaResult<u64>;
}

/// Real RPC client implementation using solana-rpc-client
#[cfg(feature = "rpc")]
pub struct RealSolanaRpc {
    client: solana_rpc_client::rpc_client::RpcClient,
}

#[cfg(feature = "rpc")]
impl RealSolanaRpc {
    /// Create new RPC client with default commitment
    pub fn new(rpc_url: &str) -> Self {
        let client = solana_rpc_client::rpc_client::RpcClient::new(rpc_url.to_string());
        Self { client }
    }

    /// Create with specific commitment level
    pub fn with_commitment(rpc_url: &str, commitment: &str) -> Self {
        let commitment_config = match commitment {
            "processed" => solana_sdk::commitment_config::CommitmentConfig::processed(),
            "confirmed" => solana_sdk::commitment_config::CommitmentConfig::confirmed(),
            "finalized" => solana_sdk::commitment_config::CommitmentConfig::finalized(),
            _ => solana_sdk::commitment_config::CommitmentConfig::confirmed(),
        };
        let client = solana_rpc_client::rpc_client::RpcClient::new_with_commitment(
            rpc_url.to_string(),
            commitment_config,
        );
        Self { client }
    }

    /// Get the underlying RPC client for advanced operations
    pub fn underlying_client(&self) -> &solana_rpc_client::rpc_client::RpcClient {
        &self.client
    }
}

#[cfg(feature = "rpc")]
#[async_trait::async_trait]
impl SolanaRpc for RealSolanaRpc {
    async fn get_account(&self, pubkey: &Pubkey) -> SolanaResult<Account> {
        self.client
            .get_account(pubkey)
            .map_err(|e| SolanaError::Rpc(format!("Failed to get account {}: {}", pubkey, e)))
    }

    async fn get_multiple_accounts(
        &self,
        pubkeys: &[Pubkey],
    ) -> SolanaResult<Vec<Option<Account>>> {
        self.client
            .get_multiple_accounts(pubkeys)
            .map_err(|e| SolanaError::Rpc(format!("Failed to get multiple accounts: {}", e)))
    }

    async fn get_transaction(&self, signature: &Signature) -> SolanaResult<String> {
        let tx = self
            .client
            .get_transaction(signature, solana_sdk::commitment_config::UiTransactionEncoding::Json)
            .map_err(|e| SolanaError::Rpc(format!("Failed to get transaction: {}", e)))?;

        serde_json::to_string(&tx)
            .map_err(|e| SolanaError::Serialization(format!("Failed to serialize transaction: {}", e)))
    }

    async fn send_transaction(&self, transaction: &Transaction) -> SolanaResult<Signature> {
        self.client
            .send_transaction(transaction)
            .map_err(|e| SolanaError::Rpc(format!("Failed to send transaction: {}", e)))
    }

    async fn get_latest_slot(&self) -> SolanaResult<u64> {
        self.client
            .get_slot()
            .map_err(|e| SolanaError::Rpc(format!("Failed to get slot: {}", e)))
    }

    async fn get_slot_with_commitment(&self, commitment: &str) -> SolanaResult<u64> {
        let commitment_config = match commitment {
            "processed" => solana_sdk::commitment_config::CommitmentConfig::processed(),
            "confirmed" => solana_sdk::commitment_config::CommitmentConfig::confirmed(),
            "finalized" => solana_sdk::commitment_config::CommitmentConfig::finalized(),
            _ => solana_sdk::commitment_config::CommitmentConfig::confirmed(),
        };

        self.client
            .get_slot_with_commitment(commitment_config)
            .map_err(|e| SolanaError::Rpc(format!("Failed to get slot with commitment: {}", e)))
    }

    async fn get_account_changes(
        &self,
        _from_slot: u64,
        _to_slot: u64,
    ) -> SolanaResult<Vec<AccountChange>> {
        // Account changes tracking requires more complex logic with
        // pre/post balance tracking. Returning empty for now - this
        // should be implemented with proper account state diffing.
        // TODO: Implement full account change tracking using get_block
        // with pre/post balance metadata.
        Ok(vec![])
    }

    async fn wait_for_confirmation(
        &self,
        signature: &Signature,
    ) -> SolanaResult<ConfirmationStatus> {
        // Poll for confirmation with exponential backoff
        let mut retries = 0;
        let max_retries = 30;

        while retries < max_retries {
            match self.client.get_signature_statuses(&[*signature]) {
                Ok(response) => {
                    if let Some(Some(status)) = response.value.get(0) {
                        if status.confirmation_status.is_some() {
                            let conf_status = status.confirmation_status.as_ref().unwrap();
                            return Ok(match conf_status {
                                solana_rpc_client::rpc_client::TransactionConfirmationStatus::Processed => ConfirmationStatus::Processed,
                                solana_rpc_client::rpc_client::TransactionConfirmationStatus::Confirmed => ConfirmationStatus::Confirmed,
                                solana_rpc_client::rpc_client::TransactionConfirmationStatus::Finalized => ConfirmationStatus::Finalized,
                            });
                        }
                        if status.err.is_some() {
                            return Err(SolanaError::TransactionFailed(
                                status.err.as_ref().unwrap().to_string()
                            ));
                        }
                    }
                }
                Err(_) => {}
            }

            retries += 1;
            tokio::time::sleep(tokio::time::Duration::from_millis(500 * retries.min(10))).await;
        }

        Err(SolanaError::Timeout("Transaction confirmation timeout".to_string()))
    }

    async fn get_minimum_balance_for_rent_exemption(&self, data_len: usize) -> SolanaResult<u64> {
        self.client
            .get_minimum_balance_for_rent_exemption(data_len)
            .map_err(|e| SolanaError::Rpc(format!("Failed to get rent exemption: {}", e)))
    }
}

/// Mock RPC client for testing
pub struct MockSolanaRpc {
    accounts: std::collections::HashMap<Pubkey, Account>,
}

impl Default for MockSolanaRpc {
    fn default() -> Self {
        Self::new()
    }
}

impl MockSolanaRpc {
    /// Create new mock RPC
    pub fn new() -> Self {
        Self {
            accounts: std::collections::HashMap::new(),
        }
    }

    /// Add account to mock
    pub fn add_account(&mut self, pubkey: Pubkey, account: Account) {
        self.accounts.insert(pubkey, account);
    }
}

#[async_trait::async_trait]
impl SolanaRpc for MockSolanaRpc {
    async fn get_account(&self, pubkey: &Pubkey) -> SolanaResult<Account> {
        self.accounts
            .get(pubkey)
            .cloned()
            .ok_or_else(|| SolanaError::AccountNotFound(pubkey.to_string()))
    }

    async fn get_multiple_accounts(
        &self,
        pubkeys: &[Pubkey],
    ) -> SolanaResult<Vec<Option<Account>>> {
        Ok(pubkeys
            .iter()
            .map(|pk| self.accounts.get(pk).cloned())
            .collect())
    }

    async fn get_transaction(&self, _signature: &Signature) -> SolanaResult<String> {
        Err(SolanaError::Rpc(
            "Mock RPC: get_transaction not implemented".to_string(),
        ))
    }

    async fn send_transaction(&self, _transaction: &Transaction) -> SolanaResult<Signature> {
        Ok(Signature::new_unique())
    }

    async fn get_latest_slot(&self) -> SolanaResult<u64> {
        Ok(1000)
    }

    async fn get_slot_with_commitment(&self, _commitment: &str) -> SolanaResult<u64> {
        Ok(1000)
    }

    async fn get_account_changes(
        &self,
        _from_slot: u64,
        _to_slot: u64,
    ) -> SolanaResult<Vec<AccountChange>> {
        Ok(vec![])
    }

    async fn wait_for_confirmation(
        &self,
        _signature: &Signature,
    ) -> SolanaResult<ConfirmationStatus> {
        Ok(ConfirmationStatus::Confirmed)
    }

    async fn get_minimum_balance_for_rent_exemption(&self, _data_len: usize) -> SolanaResult<u64> {
        // Mock value - rent exemption for typical program size
        Ok(6_900_000_000)
    }
}
