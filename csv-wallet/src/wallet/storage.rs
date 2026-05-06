//! Wallet storage - Re-export from csv-adapter-store browser storage.
//!
//! This module provides browser-specific localStorage persistence.

pub use csv_adapter_store::browser_storage::*;

/// Backward-compatible type alias for storage error.
pub type StorageError = BrowserStorageError;

/// Wallet storage manager type alias.
pub type WalletStorage = LocalStorageManager;

/// Storage keys used by the wallet.
pub mod keys {
    /// Key for unified storage (sanads, transfers, seals, proofs, contracts).
    pub const UNIFIED_STORAGE_KEY: &str = "csv_unified_storage";

    /// Key for wallet mnemonic/seed data.
    pub const WALLET_MNEMONIC_KEY: &str = "csv_wallet_mnemonic";

    /// Key for legacy wallet data.
    pub const WALLET_DATA_KEY: &str = "csv_wallet_data";
}
