//! Persistent storage using browser localStorage with unified storage format.
//!
//! This module uses the unified storage types from csv-adapter-store, allowing
//! seamless data sharing between csv-wallet (browser) and csv-cli (desktop).

use csv_adapter_core::agent_types::{HasErrorSuggestion, FixAction, error_codes};
use serde::{Deserialize, Serialize};
use web_sys::{Storage, Window};

/// Storage error.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    /// Browser API error
    #[error("Browser API error: {0}")]
    BrowserError(String),
    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializeError(String),
    /// Not found
    #[error("Not found: {0}")]
    NotFound(String),
}

impl HasErrorSuggestion for StorageError {
    fn error_code(&self) -> &'static str {
        error_codes::WALLET_BROWSER_STORAGE
    }

    fn description(&self) -> String {
        self.to_string()
    }

    fn suggested_fix(&self) -> String {
        match self {
            StorageError::BrowserError(_) => {
                "Browser storage API error. Check: \
                 1) LocalStorage is enabled in your browser, \
                 2) You are not in private/incognito mode, \
                 3) Storage quota has not been exceeded. \
                 Try clearing some storage or using a different browser.".to_string()
            }
            StorageError::SerializeError(_) => {
                "Failed to serialize data for browser storage. \
                 Ensure all data types are JSON-serializable.".to_string()
            }
            StorageError::NotFound(key) => {
                format!(
                    "Item '{}' not found in browser storage. \
                     It may have been deleted or never saved.",
                    key
                )
            }
        }
    }

    fn docs_url(&self) -> String {
        error_codes::docs_url(self.error_code())
    }

    fn fix_action(&self) -> Option<FixAction> {
        match self {
            StorageError::BrowserError(_) => {
                Some(FixAction::CheckState {
                    url: "https://docs.csv.dev/wallet/browser-storage".to_string(),
                    what: "Verify localStorage is enabled and not full".to_string(),
                })
            }
            _ => None,
        }
    }
}

/// LocalStorage-based storage manager.
#[derive(Clone)]
pub struct LocalStorageManager {
    storage: Storage,
    prefix: String,
}

impl LocalStorageManager {
    /// Create new storage manager.
    pub fn new(prefix: &str) -> Result<Self, StorageError> {
        let window: Window = web_sys::window()
            .ok_or_else(|| StorageError::BrowserError("No window object".to_string()))?;

        let storage = window
            .local_storage()
            .map_err(|e| StorageError::BrowserError(format!("{:?}", e)))?
            .ok_or_else(|| StorageError::BrowserError("localStorage not available".to_string()))?;

        Ok(Self {
            storage,
            prefix: prefix.to_string(),
        })
    }

    /// Save item.
    pub fn save<T: Serialize>(&self, key: &str, value: &T) -> Result<(), StorageError> {
        let json = serde_json::to_string(value)
            .map_err(|e| StorageError::SerializeError(e.to_string()))?;

        let full_key = format!("{}:{}", self.prefix, key);
        self.storage
            .set_item(&full_key, &json)
            .map_err(|e| StorageError::BrowserError(format!("{:?}", e)))
    }

    /// Load item.
    pub fn load<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<T, StorageError> {
        let full_key = format!("{}:{}", self.prefix, key);

        let json = self
            .storage
            .get_item(&full_key)
            .map_err(|e| StorageError::BrowserError(format!("{:?}", e)))?;

        match json {
            Some(json) => {
                serde_json::from_str(&json).map_err(|e| StorageError::SerializeError(e.to_string()))
            }
            None => Err(StorageError::NotFound(key.to_string())),
        }
    }

    /// Try to load item, returning None on failure.
    pub fn try_load<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Option<T> {
        let full_key = format!("{}:{}", self.prefix, key);
        let json = self.storage.get_item(&full_key).ok()??;
        serde_json::from_str(&json).ok()
    }

    /// Delete item.
    pub fn delete(&self, key: &str) -> Result<(), StorageError> {
        let full_key = format!("{}:{}", self.prefix, key);
        self.storage
            .remove_item(&full_key)
            .map_err(|e| StorageError::BrowserError(format!("{:?}", e)))
    }

    /// Get item as string.
    pub fn get_raw(&self, key: &str) -> Result<Option<String>, StorageError> {
        let full_key = format!("{}:{}", self.prefix, key);
        self.storage
            .get_item(&full_key)
            .map_err(|e| StorageError::BrowserError(format!("{:?}", e)))
    }

    /// Set raw string value.
    pub fn set_raw(&self, key: &str, value: &str) -> Result<(), StorageError> {
        let full_key = format!("{}:{}", self.prefix, key);
        self.storage
            .set_item(&full_key, value)
            .map_err(|e| StorageError::BrowserError(format!("{:?}", e)))
    }

    /// Check if key exists.
    pub fn contains(&self, key: &str) -> bool {
        let full_key = format!("{}:{}", self.prefix, key);
        self.storage.get_item(&full_key).ok().flatten().is_some()
    }
}

/// Storage keys
pub const UNIFIED_STORAGE_KEY: &str = "unified_storage";
pub const WALLET_MNEMONIC_KEY: &str = "mnemonic_encrypted";

/// Get wallet storage instance.
pub fn wallet_storage() -> Result<LocalStorageManager, StorageError> {
    LocalStorageManager::new("csv-wallet")
}

/// Get seal storage instance.
pub fn seal_storage() -> Result<LocalStorageManager, StorageError> {
    LocalStorageManager::new("csv-seals")
}

/// Get asset storage instance.
pub fn asset_storage() -> Result<LocalStorageManager, StorageError> {
    LocalStorageManager::new("csv-assets")
}

/// Unified storage manager - handles the complete unified storage format
pub struct UnifiedStorageManager {
    storage: LocalStorageManager,
}

impl UnifiedStorageManager {
    /// Create new unified storage manager
    pub fn new() -> Result<Self, StorageError> {
        Ok(Self {
            storage: wallet_storage()?,
        })
    }

    /// Load unified storage from localStorage
    pub fn load(&self) -> Result<UnifiedStorage, StorageError> {
        self.storage
            .try_load::<UnifiedStorage>(UNIFIED_STORAGE_KEY)
            .ok_or_else(|| StorageError::NotFound(UNIFIED_STORAGE_KEY.to_string()))
    }

    /// Load or create with defaults
    pub fn load_or_default(&self) -> UnifiedStorage {
        self.load().unwrap_or_else(|_| UnifiedStorage::new().with_defaults())
    }

    /// Save unified storage to localStorage
    pub fn save(&self, storage: &UnifiedStorage) -> Result<(), StorageError> {
        self.storage.save(UNIFIED_STORAGE_KEY, storage)
    }

    /// Save wallet mnemonic (encrypted separately for security)
    pub fn save_mnemonic(&self, encrypted_mnemonic: &str) -> Result<(), StorageError> {
        self.storage.set_raw(WALLET_MNEMONIC_KEY, encrypted_mnemonic)
    }

    /// Load encrypted mnemonic
    pub fn load_mnemonic(&self) -> Result<Option<String>, StorageError> {
        self.storage.get_raw(WALLET_MNEMONIC_KEY)
    }

    /// Check if unified storage exists
    pub fn exists(&self) -> bool {
        self.storage.contains(UNIFIED_STORAGE_KEY)
    }

    /// Export unified storage as JSON string (for CLI import)
    pub fn export_json(&self) -> Result<String, StorageError> {
        let storage = self.load()?;
        serde_json::to_string_pretty(&storage)
            .map_err(|e| StorageError::SerializeError(e.to_string()))
    }

    /// Import unified storage from JSON string (from CLI export)
    pub fn import_json(&self, json: &str) -> Result<(), StorageError> {
        let storage: UnifiedStorage = serde_json::from_str(json)
            .map_err(|e| StorageError::SerializeError(e.to_string()))?;
        self.save(&storage)
    }
}


/// Legacy state format for migration purposes (deprecated)
#[derive(Serialize, Deserialize, Default)]
pub struct LegacyState {
    pub initialized: bool,
    pub selected_chain: String,
    pub selected_network: String,
    pub rights: Vec<PersistedRight>,
    pub transfers: Vec<PersistedTransfer>,
    pub seals: Vec<PersistedSeal>,
    pub proofs: Vec<PersistedProof>,
    pub contracts: Vec<PersistedContract>,
}

/// Legacy persisted types for migration
#[derive(Serialize, Deserialize)]
pub struct PersistedRight {
    pub id: String,
    pub chain: String,
    pub value: u64,
    pub status: String,
    pub owner: String,
}

#[derive(Serialize, Deserialize)]
pub struct PersistedTransfer {
    pub id: String,
    pub from_chain: String,
    pub to_chain: String,
    pub right_id: String,
    pub dest_owner: String,
    pub status: String,
    pub created_at: u64,
}

#[derive(Serialize, Deserialize)]
pub struct PersistedSeal {
    pub seal_ref: String,
    pub chain: String,
    pub value: u64,
    pub consumed: bool,
    pub created_at: u64,
}

#[derive(Serialize, Deserialize)]
pub struct PersistedProof {
    pub chain: String,
    pub right_id: String,
    pub proof_type: String,
    pub verified: bool,
}

#[derive(Serialize, Deserialize)]
pub struct PersistedContract {
    pub chain: String,
    pub address: String,
    pub tx_hash: String,
    pub deployed_at: u64,
}

// Re-export unified types
pub use csv_adapter_store::unified::{
    Chain, ChainConfig, ContractRecord, FaucetConfig, GasAccount, Network, 
    ProofRecord, RightRecord, RightStatus, SealRecord, TransactionRecord,
    TransactionStatus, TransactionType, TransferRecord, TransferStatus, 
    WalletAccount, WalletConfig, UnifiedStorage,
};
