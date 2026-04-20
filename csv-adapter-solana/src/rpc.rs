//! RPC client for Solana adapter

use solana_sdk::{
    pubkey::Pubkey,
    signature::Signature,
    transaction::Transaction,
    account::Account,
};

use crate::error::{SolanaError, SolanaResult};
use crate::types::{AccountChange, ConfirmationStatus};

/// Trait for Solana RPC operations
#[async_trait::async_trait]
pub trait SolanaRpc: Send + Sync {
    /// Get account info
    async fn get_account(&self, pubkey: &Pubkey) -> SolanaResult<Account>;

    /// Get multiple accounts
    async fn get_multiple_accounts(&self, pubkeys: &[Pubkey]) -> SolanaResult<Vec<Option<Account>>>;

    /// Get transaction with status
    async fn get_transaction(&self, signature: &Signature) -> SolanaResult<String>;

    /// Send transaction
    async fn send_transaction(&self, transaction: &Transaction) -> SolanaResult<Signature>;

    /// Get latest slot
    async fn get_latest_slot(&self) -> SolanaResult<u64>;

    /// Get slot with commitment
    async fn get_slot_with_commitment(&self, commitment: &str) -> SolanaResult<u64>;

    /// Get account changes between slots
    async fn get_account_changes(&self, from_slot: u64, to_slot: u64) -> SolanaResult<Vec<AccountChange>>;

    /// Wait for transaction confirmation
    async fn wait_for_confirmation(&self, signature: &Signature) -> SolanaResult<ConfirmationStatus>;
}

/// Real RPC client implementation
#[cfg(feature = "rpc")]
pub struct RealSolanaRpc {
    rpc_url: String,
    timeout: Duration,
}

#[cfg(feature = "rpc")]
impl RealSolanaRpc {
    /// Create new RPC client
    pub fn new(rpc_url: &str, timeout_seconds: u64) -> Self {
        Self {
            rpc_url: rpc_url.to_string(),
            timeout: Duration::from_secs(timeout_seconds),
        }
    }

    /// Create with commitment
    pub fn with_commitment(rpc_url: &str, _commitment: &str, timeout_seconds: u64) -> Self {
        Self {
            rpc_url: rpc_url.to_string(),
            timeout: Duration::from_secs(timeout_seconds),
        }
    }
}

#[cfg(feature = "rpc")]
#[async_trait::async_trait]
impl SolanaRpc for RealSolanaRpc {
    async fn get_account(&self, _pubkey: &Pubkey) -> SolanaResult<Account> {
        // Simplified implementation - would need actual RPC client
        Err(SolanaError::Rpc("RPC client not implemented".to_string()))
    }

    async fn get_multiple_accounts(&self, _pubkeys: &[Pubkey]) -> SolanaResult<Vec<Option<Account>>> {
        // Simplified implementation - would need actual RPC client
        Err(SolanaError::Rpc("RPC client not implemented".to_string()))
    }

    async fn get_transaction(&self, _signature: &Signature) -> SolanaResult<String> {
        // Simplified implementation - would need actual RPC client
        Err(SolanaError::Rpc("RPC client not implemented".to_string()))
    }

    async fn send_transaction(&self, _transaction: &Transaction) -> SolanaResult<Signature> {
        // Simplified implementation - would need actual RPC client
        Err(SolanaError::Rpc("RPC client not implemented".to_string()))
    }

    async fn get_latest_slot(&self) -> SolanaResult<u64> {
        // Simplified implementation - would need actual RPC client
        Err(SolanaError::Rpc("RPC client not implemented".to_string()))
    }

    async fn get_slot_with_commitment(&self, _commitment: &str) -> SolanaResult<u64> {
        // Simplified implementation - would need actual RPC client
        Err(SolanaError::Rpc("RPC client not implemented".to_string()))
    }

    async fn get_account_changes(&self, _from_slot: u64, _to_slot: u64) -> SolanaResult<Vec<AccountChange>> {
        // Simplified implementation - would need actual RPC client
        Ok(vec![])
    }

    async fn wait_for_confirmation(&self, _signature: &Signature) -> SolanaResult<ConfirmationStatus> {
        // Simplified implementation - would need actual RPC client
        Ok(ConfirmationStatus::Confirmed)
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

    async fn get_multiple_accounts(&self, pubkeys: &[Pubkey]) -> SolanaResult<Vec<Option<Account>>> {
        Ok(pubkeys.iter().map(|pk| self.accounts.get(pk).cloned()).collect())
    }

    async fn get_transaction(&self, _signature: &Signature) -> SolanaResult<String> {
        Err(SolanaError::Rpc("Mock RPC: get_transaction not implemented".to_string()))
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

    async fn get_account_changes(&self, _from_slot: u64, _to_slot: u64) -> SolanaResult<Vec<AccountChange>> {
        Ok(vec![])
    }

    async fn wait_for_confirmation(&self, _signature: &Signature) -> SolanaResult<ConfirmationStatus> {
        Ok(ConfirmationStatus::Confirmed)
    }
}
