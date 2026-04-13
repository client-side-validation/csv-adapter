//! Asset management service.
//!
//! Provides local storage and management of asset records using LocalStorage.

use chrono::{DateTime, Utc};
use csv_adapter_core::Chain;
use serde::{Deserialize, Serialize};

use crate::storage::{asset_storage, LocalStorageManager};

/// An asset record stored locally.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetRecord {
    pub right_id: String,
    pub chain: String,
    pub commitment: String,
    pub owner_address: String,
    pub value: Option<f64>,
    pub value_currency: String,
    pub created_at: DateTime<Utc>,
}

impl AssetRecord {
    /// Parse chain from string.
    pub fn chain_enum(&self) -> Result<Chain, String> {
        self.chain.parse()
    }
}

/// Error type for asset operations.
#[derive(Debug, thiserror::Error)]
pub enum AssetError {
    #[error("Storage error: {0}")]
    Storage(String),
    #[error("Asset not found: {0}")]
    NotFound(String),
    #[error("Invalid asset data: {0}")]
    InvalidData(String),
}

impl From<crate::storage::StorageError> for AssetError {
    fn from(err: crate::storage::StorageError) -> Self {
        match err {
            crate::storage::StorageError::NotFound(id) => AssetError::NotFound(id),
            e => AssetError::Storage(e.to_string()),
        }
    }
}

/// Manager for asset records using LocalStorage.
pub struct AssetManager {
    storage: LocalStorageManager,
}

impl AssetManager {
    /// Create a new AssetManager.
    pub fn new(storage: LocalStorageManager) -> Result<Self, AssetError> {
        Ok(Self { storage })
    }

    /// Add a new asset record and persist it.
    pub fn add_asset(&self, record: &AssetRecord) -> Result<(), AssetError> {
        self.storage.save(&record.right_id, record)?;
        Ok(())
    }

    /// Get an asset record by its right_id.
    pub fn get_asset(&self, right_id: &str) -> Result<AssetRecord, AssetError> {
        let record = self.storage.load::<AssetRecord>(right_id)?;
        Ok(record)
    }

    /// List all asset records.
    pub fn list_assets(&self) -> Result<Vec<AssetRecord>, AssetError> {
        let window =
            web_sys::window().ok_or_else(|| AssetError::Storage("No window object".to_string()))?;
        let storage = window
            .local_storage()
            .map_err(|e| AssetError::Storage(format!("{:?}", e)))?
            .ok_or_else(|| AssetError::Storage("localStorage not available".to_string()))?;

        let prefix = format!("csv-assets:");
        let mut records = Vec::new();

        for i in 0..storage.length().unwrap_or(0) {
            if let Some(key) = storage.key(i).ok().flatten() {
                if key.starts_with(&prefix) {
                    let record_key = key.strip_prefix(&prefix).unwrap_or(&key);
                    if let Ok(record) = self.storage.load::<AssetRecord>(record_key) {
                        records.push(record);
                    }
                }
            }
        }

        // Sort by created_at descending
        records.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(records)
    }

    /// Get all assets for a specific chain.
    pub fn get_by_chain(&self, chain: &str) -> Result<Vec<AssetRecord>, AssetError> {
        let all = self.list_assets()?;
        Ok(all.into_iter().filter(|a| a.chain == chain).collect())
    }

    /// Update the value of an existing asset.
    pub fn update_value(
        &self,
        right_id: &str,
        new_value: Option<f64>,
        currency: &str,
    ) -> Result<AssetRecord, AssetError> {
        let mut record = self.get_asset(right_id)?;
        record.value = new_value;
        record.value_currency = currency.to_string();
        self.storage.save(&record.right_id, &record)?;
        Ok(record)
    }

    /// Calculate total value across all assets, grouped by currency.
    /// Returns a map of currency -> total value.
    pub fn total_value(&self) -> Result<Vec<(String, f64)>, AssetError> {
        let assets = self.list_assets()?;
        let mut totals: std::collections::HashMap<String, f64> = std::collections::HashMap::new();

        for asset in assets {
            if let (Some(value), ref currency) = (asset.value, asset.value_currency) {
                if !currency.is_empty() {
                    *totals.entry(currency.clone()).or_insert(0.0) += value;
                }
            }
        }

        Ok(totals.into_iter().collect())
    }

    /// Delete an asset by right_id.
    pub fn delete_asset(&self, right_id: &str) -> Result<(), AssetError> {
        self.storage.delete(right_id)?;
        Ok(())
    }
}

/// Convenience function to get a ready-to-use AssetManager.
pub fn asset_manager() -> Result<AssetManager, AssetError> {
    let storage = asset_storage()?;
    AssetManager::new(storage)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_record_serialization() {
        let record = AssetRecord {
            right_id: "asset-001".to_string(),
            chain: "bitcoin".to_string(),
            commitment: "commit-xyz".to_string(),
            owner_address: "addr123".to_string(),
            value: Some(1.5),
            value_currency: "BTC".to_string(),
            created_at: Utc::now(),
        };

        let json = serde_json::to_string(&record).unwrap();
        let decoded: AssetRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.right_id, record.right_id);
        assert_eq!(decoded.chain, "bitcoin");
        assert_eq!(decoded.value, Some(1.5));
        assert_eq!(decoded.value_currency, "BTC");
    }

    #[test]
    fn asset_record_with_no_value() {
        let record = AssetRecord {
            right_id: "asset-002".to_string(),
            chain: "ethereum".to_string(),
            commitment: "commit-abc".to_string(),
            owner_address: "addr456".to_string(),
            value: None,
            value_currency: "ETH".to_string(),
            created_at: Utc::now(),
        };

        let json = serde_json::to_string(&record).unwrap();
        let decoded: AssetRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.value, None);
    }
}
