//! Seal management service.
//!
//! Provides local storage and management of seal records using LocalStorage.

use chrono::{DateTime, Utc};
use csv_core::mcp::{error_codes, FixAction, HasErrorSuggestion};
use csv_core::Chain;
use serde::{Deserialize, Serialize};

use crate::storage::{seal_storage, LocalStorageManager};

/// Seal status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SealStatus {
    Unconsumed,
    Consumed,
    DoubleSpent,
}

/// A seal record stored locally.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SealRecord {
    pub id: String,
    pub chain: String,
    pub status: SealStatus,
    pub value: u64,
    pub created_at: DateTime<Utc>,
    pub right_id: String,
}

impl SealRecord {
    /// Parse chain from string.
    pub fn chain_enum(&self) -> Result<Chain, String> {
        self.chain.parse()
    }
}

/// Seal status filter for listing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatusFilter {
    All,
    ByStatus(SealStatus),
    ByChain(String),
}

/// Error type for seal operations.
#[derive(Debug, thiserror::Error)]
pub enum SealError {
    #[error("Storage error: {0}")]
    Storage(String),
    #[error("Seal not found: {0}")]
    NotFound(String),
    #[error("Invalid seal data: {0}")]
    InvalidData(String),
}

impl HasErrorSuggestion for SealError {
    fn error_code(&self) -> &'static str {
        error_codes::WALLET_SEAL_SERVICE_ERROR
    }

    fn description(&self) -> String {
        self.to_string()
    }

    fn suggested_fix(&self) -> String {
        match self {
            SealError::Storage(_) => {
                "Seal storage error. Check localStorage is enabled and not full. \
                 Try clearing old seals or using a different browser."
                    .to_string()
            }
            SealError::NotFound(id) => {
                format!(
                    "Seal '{}' not found. Check: 1) The seal ID is correct, \
                     2) The seal was successfully created, 3) Storage hasn't been cleared.",
                    id
                )
            }
            SealError::InvalidData(_) => {
                "Invalid seal data format. The seal record may be corrupted. \
                 Try recreating the seal with valid parameters."
                    .to_string()
            }
        }
    }

    fn docs_url(&self) -> String {
        error_codes::docs_url(self.error_code())
    }

    fn fix_action(&self) -> Option<FixAction> {
        match self {
            SealError::NotFound(_) => Some(FixAction::CheckState {
                url: "https://docs.csv.dev/seals".to_string(),
                what: "Verify seal exists in storage".to_string(),
            }),
            SealError::InvalidData(_) => Some(FixAction::Retry {
                parameter_changes: std::collections::HashMap::from([(
                    "recreate_seal".to_string(),
                    "true".to_string(),
                )]),
            }),
            _ => None,
        }
    }
}

impl From<crate::storage::StorageError> for SealError {
    fn from(err: crate::storage::StorageError) -> Self {
        match err {
            crate::storage::StorageError::NotFound(id) => SealError::NotFound(id),
            e => SealError::Storage(e.to_string()),
        }
    }
}

/// Manager for seal records using LocalStorage.
pub struct SealManager {
    storage: LocalStorageManager,
}

impl SealManager {
    /// Create a new SealManager.
    pub fn new(storage: LocalStorageManager) -> Result<Self, SealError> {
        Ok(Self { storage })
    }

    /// Create a new seal record and persist it.
    pub fn create_seal(&self, record: &SealRecord) -> Result<(), SealError> {
        self.storage.save(&record.id, record)?;
        Ok(())
    }

    /// Get a seal record by its ID.
    pub fn get_seal(&self, id: &str) -> Result<SealRecord, SealError> {
        let record = self.storage.load::<SealRecord>(id)?;
        Ok(record)
    }

    /// Update the status of an existing seal.
    pub fn update_status(&self, id: &str, new_status: SealStatus) -> Result<SealRecord, SealError> {
        let mut record = self.get_seal(id)?;
        record.status = new_status;
        self.storage.save(&record.id, &record)?;
        Ok(record)
    }

    /// List all seals, optionally filtered by status or chain.
    pub fn list_seals(&self, filter: StatusFilter) -> Result<Vec<SealRecord>, SealError> {
        // LocalStorageManager doesn't have list_keys, so we iterate through all storage keys
        let window =
            web_sys::window().ok_or_else(|| SealError::Storage("No window object".to_string()))?;
        let storage = window
            .local_storage()
            .map_err(|e| SealError::Storage(format!("{:?}", e)))?
            .ok_or_else(|| SealError::Storage("localStorage not available".to_string()))?;

        let prefix = "csv-seals:".to_string();
        let mut records = Vec::new();

        for i in 0..storage.length().unwrap_or(0) {
            if let Some(key) = storage.key(i).ok().flatten() {
                if key.starts_with(&prefix) {
                    let record_key = key.strip_prefix(&prefix).unwrap_or(&key);
                    if let Ok(record) = self.storage.load::<SealRecord>(record_key) {
                        let matches = match &filter {
                            StatusFilter::All => true,
                            StatusFilter::ByStatus(status) => record.status == *status,
                            StatusFilter::ByChain(chain) => record.chain == *chain,
                        };
                        if matches {
                            records.push(record);
                        }
                    }
                }
            }
        }

        // Sort by created_at descending
        records.sort_by_key(|record| std::cmp::Reverse(record.created_at));
        Ok(records)
    }

    /// Convenience method: get all seals for a specific chain.
    pub fn get_seals_by_chain(&self, chain: &str) -> Result<Vec<SealRecord>, SealError> {
        self.list_seals(StatusFilter::ByChain(chain.to_string()))
    }

    /// Delete a seal by ID.
    pub fn delete_seal(&self, id: &str) -> Result<(), SealError> {
        self.storage.delete(id)?;
        Ok(())
    }
}

/// Convenience function to get a ready-to-use SealManager.
pub fn seal_manager() -> Result<SealManager, SealError> {
    let storage = seal_storage()?;
    SealManager::new(storage)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seal_record_serialization() {
        let record = SealRecord {
            id: "seal-1".to_string(),
            chain: "bitcoin".to_string(),
            status: SealStatus::Unconsumed,
            value: 1000,
            created_at: Utc::now(),
            right_id: "right-abc".to_string(),
        };

        let json = serde_json::to_string(&record).unwrap();
        let decoded: SealRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.id, record.id);
        assert_eq!(decoded.chain, "bitcoin");
        assert_eq!(decoded.status, SealStatus::Unconsumed);
    }
}
