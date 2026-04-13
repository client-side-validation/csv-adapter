//! Persistent storage using browser localStorage.

use web_sys::{Storage, Window};
use serde::{Serialize, Deserialize};

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

        let storage = window.local_storage()
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
        self.storage.set_item(&full_key, &json)
            .map_err(|e| StorageError::BrowserError(format!("{:?}", e)))
    }

    /// Load item.
    pub fn load<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<T, StorageError> {
        let full_key = format!("{}:{}", self.prefix, key);

        let json = self.storage.get_item(&full_key)
            .map_err(|e| StorageError::BrowserError(format!("{:?}", e)))?;

        match json {
            Some(json) => serde_json::from_str(&json)
                .map_err(|e| StorageError::SerializeError(e.to_string())),
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
        self.storage.remove_item(&full_key)
            .map_err(|e| StorageError::BrowserError(format!("{:?}", e)))
    }

    /// Get item as string.
    pub fn get_raw(&self, key: &str) -> Result<Option<String>, StorageError> {
        let full_key = format!("{}:{}", self.prefix, key);
        self.storage.get_item(&full_key)
            .map_err(|e| StorageError::BrowserError(format!("{:?}", e)))
    }

    /// Check if key exists.
    pub fn contains(&self, key: &str) -> bool {
        let full_key = format!("{}:{}", self.prefix, key);
        self.storage.get_item(&full_key).ok().flatten().is_some()
    }
}

/// Keys for wallet state persistence.
pub const WALLET_STATE_KEY: &str = "state";
pub const WALLET_MNEMONIC_KEY: &str = "mnemonic";
pub const WALLET_RIGHTS_KEY: &str = "rights";
pub const WALLET_TRANSFERS_KEY: &str = "transfers";
pub const WALLET_SEALS_KEY: &str = "seals";
pub const WALLET_PROOFS_KEY: &str = "proofs";
pub const WALLET_CONTRACTS_KEY: &str = "contracts";
pub const WALLET_SELECTED_CHAIN_KEY: &str = "selected_chain";
pub const WALLET_SELECTED_NETWORK_KEY: &str = "selected_network";

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

/// Persistable subset of app state (without wallet secret).
#[derive(Serialize, Deserialize, Default)]
pub struct PersistedState {
    pub initialized: bool,
    pub selected_chain: String,
    pub selected_network: String,
    pub rights: Vec<PersistedRight>,
    pub transfers: Vec<PersistedTransfer>,
    pub seals: Vec<PersistedSeal>,
    pub proofs: Vec<PersistedProof>,
    pub contracts: Vec<PersistedContract>,
}

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
