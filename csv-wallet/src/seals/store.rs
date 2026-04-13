//! Seal storage.
//!
//! In-memory storage for seal records.

use super::manager::SealRecord;
use csv_adapter_core::{Chain, RightId};
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

    /// Get seals for a specific right.
    pub fn get_seals_for_right(
        &self,
        right_id: &RightId,
    ) -> Result<Vec<SealRecord>, SealStoreError> {
        let seals = self.seals.lock().unwrap();
        Ok(seals.values()
            .filter(|s| s.right_id.as_ref() == Some(right_id))
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
