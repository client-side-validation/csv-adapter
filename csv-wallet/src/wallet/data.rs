//! Wallet data - collection of per-chain accounts.

use crate::wallet::account::ChainAccount;
use csv_core::Chain;
use serde::{Deserialize, Serialize};

/// Complete wallet data — collection of per-chain accounts.
#[derive(Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct WalletData {
    /// All accounts (multiple per chain allowed)
    pub accounts: Vec<ChainAccount>,
    /// Last selected account ID
    pub selected_account_id: Option<String>,
}

impl WalletData {
    /// Add an account.
    pub fn add_account(&mut self, account: ChainAccount) {
        self.accounts.push(account);
    }

    /// Remove an account by ID.
    pub fn remove_account(&mut self, id: &str) -> bool {
        let len_before = self.accounts.len();
        self.accounts.retain(|a| a.id != id);
        self.accounts.len() < len_before
    }

    /// Get accounts for a specific chain.
    pub fn accounts_for_chain(&self, chain: Chain) -> Vec<&ChainAccount> {
        self.accounts.iter().filter(|a| a.chain == chain).collect()
    }

    /// Get the gas account address for a chain (first account for now).
    pub fn get_gas_account(&self, chain: &Chain) -> Option<String> {
        self.accounts_for_chain(*chain)
            .first()
            .map(|a| a.address.clone())
    }

    /// Get accounts count for a chain.
    pub fn account_count_for_chain(&self, chain: Chain) -> usize {
        self.accounts.iter().filter(|a| a.chain == chain).count()
    }

    /// Get account by ID.
    pub fn get_account(&self, id: &str) -> Option<&ChainAccount> {
        self.accounts.iter().find(|a| a.id == id)
    }

    /// Get mutable account by ID.
    pub fn get_account_mut(&mut self, id: &str) -> Option<&mut ChainAccount> {
        self.accounts.iter_mut().find(|a| a.id == id)
    }

    /// Get all accounts.
    pub fn all_accounts(&self) -> Vec<ChainAccount> {
        self.accounts.clone()
    }

    /// Get total account count.
    pub fn total_accounts(&self) -> usize {
        self.accounts.len()
    }

    /// Check if wallet has any accounts.
    pub fn is_empty(&self) -> bool {
        self.accounts.is_empty()
    }

    /// Refresh/update an account address.
    pub fn refresh_address(&mut self, chain: Chain, old_address: &str, new_address: String) {
        if let Some(account) = self
            .accounts
            .iter_mut()
            .find(|a| a.chain == chain && a.address == old_address)
        {
            account.address = new_address;
        }
    }

    /// Select an account.
    pub fn select_account(&mut self, id: String) {
        self.selected_account_id = Some(id);
    }

    /// Clear selection.
    pub fn clear_selection(&mut self) {
        self.selected_account_id = None;
    }

    /// Export as JSON string.
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|e| format!("Failed to serialize: {}", e))
    }

    /// Import from JSON string.
    pub fn from_json(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| format!("Failed to parse JSON: {}", e))
    }
}
