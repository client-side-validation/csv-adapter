//! Wallet storage using in-memory storage.
//!
//! Simple storage manager for wallets.

use super::encryption::{EncryptedWallet, encrypt, decrypt, EncryptionError};
use csv_core::agent_types::{HasErrorSuggestion, FixAction, error_codes};
use std::collections::HashMap;
use std::sync::Mutex;

/// Storage error type.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    /// Encryption error
    #[error("Encryption error: {0}")]
    EncryptionError(#[from] EncryptionError),
    /// Wallet not found
    #[error("Wallet not found: {0}")]
    NotFound(String),
    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

impl HasErrorSuggestion for StorageError {
    fn error_code(&self) -> &'static str {
        match self {
            StorageError::EncryptionError(e) => e.error_code(),
            StorageError::NotFound(_) => error_codes::WALLET_STORAGE_NOT_FOUND,
            StorageError::SerializationError(_) => error_codes::WALLET_STORAGE_SERIALIZATION,
        }
    }

    fn description(&self) -> String {
        self.to_string()
    }

    fn suggested_fix(&self) -> String {
        match self {
            StorageError::EncryptionError(e) => e.suggested_fix(),
            StorageError::NotFound(id) => {
                format!(
                    "Wallet '{}' not found in storage. Check: \
                     1) The wallet ID is correct, 2) You have created a wallet, \
                     3) You are looking in the right storage location.",
                    id
                )
            }
            StorageError::SerializationError(_) => {
                "Failed to serialize/deserialize wallet data. This may indicate \
                 data corruption or version incompatibility. \
                 Try restoring from a backup or mnemonic.".to_string()
            }
        }
    }

    fn docs_url(&self) -> String {
        match self {
            StorageError::EncryptionError(e) => e.docs_url(),
            _ => error_codes::docs_url(self.error_code()),
        }
    }

    fn fix_action(&self) -> Option<FixAction> {
        match self {
            StorageError::EncryptionError(e) => e.fix_action(),
            StorageError::NotFound(_) => {
                Some(FixAction::CheckState {
                    url: "https://docs.csv.dev/wallet/storage".to_string(),
                    what: "Verify wallet exists and storage location is correct".to_string(),
                })
            }
            StorageError::SerializationError(_) => {
                Some(FixAction::Retry {
                    parameter_changes: std::collections::HashMap::from([
                        ("restore_from_mnemonic".to_string(), "true".to_string()),
                    ]),
                })
            }
        }
    }
}

/// Wallet storage manager (in-memory for now).
pub struct WalletStorage {
    storage: Mutex<HashMap<String, EncryptedWallet>>,
}

impl WalletStorage {
    /// Create new storage manager.
    pub fn new() -> Self {
        Self {
            storage: Mutex::new(HashMap::new()),
        }
    }

    /// Save an encrypted wallet.
    pub fn save_wallet(
        &self,
        wallet_id: &str,
        wallet_data: &[u8],
        password: &str,
    ) -> Result<(), StorageError> {
        let encrypted = encrypt(wallet_data, password)?;
        let mut storage = self.storage.lock().unwrap();
        storage.insert(wallet_id.to_string(), encrypted);
        Ok(())
    }

    /// Load and decrypt a wallet.
    pub fn load_wallet(
        &self,
        wallet_id: &str,
        password: &str,
    ) -> Result<Vec<u8>, StorageError> {
        let storage = self.storage.lock().unwrap();
        let encrypted = storage.get(wallet_id)
            .ok_or_else(|| StorageError::NotFound(wallet_id.to_string()))?;
        
        decrypt(encrypted, password)
    }

    /// Delete a wallet.
    pub fn delete_wallet(&self, wallet_id: &str) -> Result<(), StorageError> {
        let mut storage = self.storage.lock().unwrap();
        storage.remove(wallet_id);
        Ok(())
    }

    /// List all wallet IDs.
    pub fn list_wallets(&self) -> Result<Vec<String>, StorageError> {
        let storage = self.storage.lock().unwrap();
        Ok(storage.keys().cloned().collect())
    }
}

impl Default for WalletStorage {
    fn default() -> Self {
        Self::new()
    }
}

/// Get default wallet storage instance.
pub fn default_storage() -> WalletStorage {
    WalletStorage::new()
}
