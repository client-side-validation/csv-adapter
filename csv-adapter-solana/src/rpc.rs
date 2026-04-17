//! RPC client for Solana adapter

use solana_client::{rpc_client::RpcClient, nonblocking::rpc_client::RpcClient as AsyncRpcClient};
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey, signature::Signature};
use solana_transaction_status::{TransactionConfirmationStatus, TransactionStatus};
use std::time::Duration;

use crate::error::{SolanaError, SolanaResult};
use crate::types::{AccountChange, ConfirmationStatus};

/// Trait for Solana RPC operations
pub trait SolanaRpc: Send + Sync {
    /// Get account info
    async fn get_account(&self, pubkey: &Pubkey) -> SolanaResult<solana_sdk::account::Account>;

    /// Get multiple accounts
    async fn get_multiple_accounts(&self, pubkeys: &[Pubkey]) -> SolanaResult<Vec<Option<solana_sdk::account::Account>>>;

    /// Get transaction with status
    async fn get_transaction(&self, signature: &Signature) -> SolanaResult<TransactionStatus>;

    /// Send transaction
    async fn send_transaction(&self, transaction: &solana_sdk::transaction::Transaction) -> SolanaResult<Signature>;

    /// Get latest slot
    async fn get_latest_slot(&self) -> SolanaResult<u64>;

    /// Get slot with commitment
    async fn get_slot_with_commitment(&self, commitment: CommitmentConfig) -> SolanaResult<u64>;

    /// Get account changes between slots
    async fn get_account_changes(&self, from_slot: u64, to_slot: u64) -> SolanaResult<Vec<AccountChange>>;

    /// Wait for transaction confirmation
    async fn wait_for_confirmation(&self, signature: &Signature) -> SolanaResult<ConfirmationStatus>;
}

/// Real RPC client implementation
#[cfg(feature = "rpc")]
pub struct RealSolanaRpc {
    client: AsyncRpcClient,
    timeout: Duration,
}

#[cfg(feature = "rpc")]
impl RealSolanaRpc {
    /// Create new RPC client
    pub fn new(rpc_url: &str, timeout_seconds: u64) -> Self {
        Self {
            client: AsyncRpcClient::new(rpc_url.to_string()),
            timeout: Duration::from_secs(timeout_seconds),
        }
    }

    /// Create with commitment
    pub fn with_commitment(rpc_url: &str, commitment: CommitmentConfig, timeout_seconds: u64) -> Self {
        Self {
            client: AsyncRpcClient::new_with_commitment(rpc_url.to_string(), commitment),
            timeout: Duration::from_secs(timeout_seconds),
        }
    }
}

#[cfg(feature = "rpc")]
#[async_trait::async_trait]
impl SolanaRpc for RealSolanaRpc {
    async fn get_account(&self, pubkey: &Pubkey) -> SolanaResult<solana_sdk::account::Account> {
        let account = tokio::time::timeout(
            self.timeout,
            self.client.get_account_with_commitment(pubkey, CommitmentConfig::confirmed())
        )
        .await
        .map_err(|_| SolanaError::Rpc("Timeout getting account".to_string()))?
        .map_err(|e| SolanaError::Rpc(format!("Failed to get account: {}", e)))?;

        account.value.ok_or_else(|| SolanaError::AccountNotFound(pubkey.to_string()))
    }

    async fn get_multiple_accounts(&self, pubkeys: &[Pubkey]) -> SolanaResult<Vec<Option<solana_sdk::account::Account>>> {
        let accounts = tokio::time::timeout(
            self.timeout,
            self.client.get_multiple_accounts_with_commitment(pubkeys, CommitmentConfig::confirmed())
        )
        .await
        .map_err(|_| SolanaError::Rpc("Timeout getting multiple accounts".to_string()))?
        .map_err(|e| SolanaError::Rpc(format!("Failed to get multiple accounts: {}", e)))?;

        Ok(accounts.value)
    }

    async fn get_transaction(&self, signature: &Signature) -> SolanaResult<TransactionStatus> {
        let status = tokio::time::timeout(
            self.timeout,
            self.client.get_transaction_with_commitment(signature, CommitmentConfig::confirmed())
        )
        .await
        .map_err(|_| SolanaError::Rpc("Timeout getting transaction".to_string()))?
        .map_err(|e| SolanaError::Rpc(format!("Failed to get transaction: {}", e)))?;

        Ok(status.value)
    }

    async fn send_transaction(&self, transaction: &solana_sdk::transaction::Transaction) -> SolanaResult<Signature> {
        let signature = tokio::time::timeout(
            self.timeout,
            self.client.send_and_confirm_transaction_with_spinner_and_commitment(
                transaction,
                CommitmentConfig::confirmed(),
                &self.client,
            )
        )
        .await
        .map_err(|_| SolanaError::Rpc("Timeout sending transaction".to_string()))?
        .map_err(|e| SolanaError::Transaction(format!("Failed to send transaction: {}", e)))?;

        Ok(signature)
    }

    async fn get_latest_slot(&self) -> SolanaResult<u64> {
        let slot = tokio::time::timeout(
            self.timeout,
            self.client.get_slot_with_commitment(CommitmentConfig::confirmed())
        )
        .await
        .map_err(|_| SolanaError::Rpc("Timeout getting slot".to_string()))?
        .map_err(|e| SolanaError::Rpc(format!("Failed to get slot: {}", e)))?;

        Ok(slot)
    }

    async fn get_slot_with_commitment(&self, commitment: CommitmentConfig) -> SolanaResult<u64> {
        let slot = tokio::time::timeout(
            self.timeout,
            self.client.get_slot_with_commitment(commitment)
        )
        .await
        .map_err(|_| SolanaError::Rpc("Timeout getting slot".to_string()))?
        .map_err(|e| SolanaError::Rpc(format!("Failed to get slot: {}", e)))?;

        Ok(slot)
    }

    async fn get_account_changes(&self, _from_slot: u64, _to_slot: u64) -> SolanaResult<Vec<AccountChange>> {
        // This would require more complex implementation using account changes API
        // For now, return empty
        Ok(vec![])
    }

    async fn wait_for_confirmation(&self, signature: &Signature) -> SolanaResult<ConfirmationStatus> {
        // Poll for confirmation status
        let mut confirmed = false;
        let mut finalized = false;

        for _ in 0..30 { // Max 30 attempts
            let status = self.get_transaction(signature).await?;
            
            match status.confirmation_status() {
                Some(TransactionConfirmationStatus::Processed) => {
                    if !confirmed {
                        confirmed = true;
                        return Ok(ConfirmationStatus::Processed);
                    }
                }
                Some(TransactionConfirmationStatus::Confirmed) => {
                    if !confirmed {
                        confirmed = true;
                        return Ok(ConfirmationStatus::Confirmed);
                    }
                }
                Some(TransactionConfirmationStatus::Finalized) => {
                    finalized = true;
                    return Ok(ConfirmationStatus::Finalized);
                }
                None => {
                    // Transaction not found, continue waiting
                }
            }

            tokio::time::sleep(Duration::from_secs(2)).await;
        }

        if finalized {
            Ok(ConfirmationStatus::Finalized)
        } else if confirmed {
            Ok(ConfirmationStatus::Confirmed)
        } else {
            Err(SolanaError::Commitment("Transaction not confirmed within timeout".to_string()))
        }
    }
}

/// Mock RPC client for testing
pub struct MockSolanaRpc {
    accounts: std::collections::HashMap<Pubkey, solana_sdk::account::Account>,
}

impl MockSolanaRpc {
    /// Create new mock RPC
    pub fn new() -> Self {
        Self {
            accounts: std::collections::HashMap::new(),
        }
    }

    /// Add account to mock
    pub fn add_account(&mut self, pubkey: Pubkey, account: solana_sdk::account::Account) {
        self.accounts.insert(pubkey, account);
    }
}

#[async_trait::async_trait]
impl SolanaRpc for MockSolanaRpc {
    async fn get_account(&self, pubkey: &Pubkey) -> SolanaResult<solana_sdk::account::Account> {
        self.accounts
            .get(pubkey)
            .cloned()
            .ok_or_else(|| SolanaError::AccountNotFound(pubkey.to_string()))
    }

    async fn get_multiple_accounts(&self, pubkeys: &[Pubkey]) -> SolanaResult<Vec<Option<solana_sdk::account::Account>>> {
        Ok(pubkeys.iter().map(|pk| self.accounts.get(pk).cloned()).collect())
    }

    async fn get_transaction(&self, _signature: &Signature) -> SolanaResult<TransactionStatus> {
        Err(SolanaError::Rpc("Mock RPC: get_transaction not implemented".to_string()))
    }

    async fn send_transaction(&self, _transaction: &solana_sdk::transaction::Transaction) -> SolanaResult<Signature> {
        Ok(Signature::new_unique())
    }

    async fn get_latest_slot(&self) -> SolanaResult<u64> {
        Ok(1000)
    }

    async fn get_slot_with_commitment(&self, _commitment: CommitmentConfig) -> SolanaResult<u64> {
        Ok(1000)
    }

    async fn get_account_changes(&self, _from_slot: u64, _to_slot: u64) -> SolanaResult<Vec<AccountChange>> {
        Ok(vec![])
    }

    async fn wait_for_confirmation(&self, _signature: &Signature) -> SolanaResult<ConfirmationStatus> {
        Ok(ConfirmationStatus::Confirmed)
    }
}
