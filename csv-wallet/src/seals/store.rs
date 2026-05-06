//! Seal storage.
//!
//! In-memory storage for seal records.

use super::manager::SealRecord;
use csv_core::{Chain, SanadId};
use csv_core::mcp::{HasErrorSuggestion, FixAction, error_codes};
use std::collections::HashMap;
use std::sync::Mutex;

/// Seal store error.
#[derive(Debug, thiserror::Error)]
pub enum SealStoreError {
    /// Not found
    #[error("Seal not found: {0}")]
    NotFound(String),
    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

impl HasErrorSuggestion for SealStoreError {
    fn error_code(&self) -> &'static str {
        match self {
            SealStoreError::NotFound(_) => error_codes::WALLET_SEAL_NOT_FOUND,
            SealStoreError::SerializationError(_) => error_codes::WALLET_STORAGE_SERIALIZATION,
        }
    }

    fn description(&self) -> String {
        self.to_string()
    }

    fn suggested_fix(&self) -> String {
        match self {
            SealStoreError::NotFound(id) => {
                format!(
                    "Seal '{}' not found in storage. \
                     Check: 1) The seal ID is correct, \
                     2) The seal was created successfully, \
                     3) You are looking in the correct seal store.",
                    id
                )
            }
            SealStoreError::SerializationError(_) => {
                "Failed to serialize seal data. Ensure the seal structure \
                 is valid and all required fields are present.".to_string()
            }
        }
    }

    fn docs_url(&self) -> String {
        error_codes::docs_url(self.error_code())
    }

    fn fix_action(&self) -> Option<FixAction> {
        match self {
            SealStoreError::NotFound(_) => {
                Some(FixAction::CheckState {
                    url: "https://docs.csv.dev/seals".to_string(),
                    what: "Verify seal exists and was created successfully".to_string(),
                })
            }
            _ => None,
        }
    }
}

/// Seal store for in-memory storage.
pub struct SealStore {
    seals: Mutex<HashMap<String, SealRecord>>,
}

impl SealStore {
    /// Create a new seal store.
    pub fn new() -> Self {
        Self {
            seals: Mutex::new(HashMap::new()),
        }
    }

    /// Save a seal record.
    pub fn save_seal(&self, seal: &SealRecord) -> Result<(), SealStoreError> {
        let mut seals = self.seals.lock().unwrap();
        seals.insert(seal.id.clone(), seal.clone());
        Ok(())
    }

    /// Get a seal by ID.
    pub fn get_seal(&self, seal_id: &str) -> Result<SealRecord, SealStoreError> {
        let seals = self.seals.lock().unwrap();
        seals.get(seal_id)
            .cloned()
            .ok_or_else(|| SealStoreError::NotFound(seal_id.to_string()))
    }

    /// List all seals, optionally filtered by chain.
    pub fn list_seals(&self, chain: Option<Chain>) -> Result<Vec<SealRecord>, SealStoreError> {
        let seals = self.seals.lock().unwrap();
        Ok(seals.values()
            .filter(|s| chain.map_or(true, |c| s.chain == c))
            .cloned()
            .collect())
    }

    /// Get seals for a specific sanad.
    pub fn get_seals_for_sanad(
        &self,
        sanad_id: &SanadId,
    ) -> Result<Vec<SealRecord>, SealStoreError> {
        let seals = self.seals.lock().unwrap();
        Ok(seals.values()
            .filter(|s| s.sanad_id.as_ref() == Some(sanad_id))
            .cloned()
            .collect())
    }

    /// Get seal history (most recent first).
    pub fn get_seal_history(&self, limit: usize) -> Result<Vec<SealRecord>, SealStoreError> {
        let mut seals: Vec<SealRecord> = self.seals.lock().unwrap().values().cloned().collect();
        seals.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(seals.into_iter().take(limit).collect())
    }
}

impl Default for SealStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Get default seal store instance.
pub fn default_seal_store() -> SealStore {
    SealStore::new()
}
