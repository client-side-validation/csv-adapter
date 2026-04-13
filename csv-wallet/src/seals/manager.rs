//! Seal manager.
//!
//! Core seal management operations.

use csv_adapter_core::{Chain, RightId};
use serde::{Serialize, Deserialize};

/// Seal status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SealStatus {
    /// Seal is unconsumed
    Unconsumed,
    /// Seal has been consumed
    Consumed {
        /// Chain where it was consumed
        consumed_on: Chain,
        /// Transaction hash
        tx_hash: String,
        /// Block height
        block_height: u64,
        /// Right ID that was transferred
        right_id: String,
    },
    /// Seal was double-spent (security issue)
    DoubleSpent,
}

/// Seal record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SealRecord {
    /// Seal ID
    pub id: String,
    /// Chain
    pub chain: Chain,
    /// Status
    pub status: SealStatus,
    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last updated timestamp
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// Associated Right ID
    pub right_id: Option<RightId>,
    /// Value (if applicable)
    pub value: Option<u64>,
}

/// Seal manager for creating and managing seals.
pub struct SealManager {
    /// Store for seals
    store: crate::seals::SealStore,
}

impl SealManager {
    /// Create a new seal manager.
    pub fn new(store: crate::seals::SealStore) -> Self {
        Self { store }
    }

    /// Create a new seal on a specific chain.
    pub fn create_seal(&self, chain: Chain, value: Option<u64>) -> Result<SealRecord, String> {
        let seal_id = format!("seal_{}_{}", chain, chrono::Utc::now().timestamp_millis());
        
        let record = SealRecord {
            id: seal_id.clone(),
            chain,
            status: SealStatus::Unconsumed,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            right_id: None,
            value,
        };

        self.store.save_seal(&record).map_err(|e| format!("{}", e))?;
        
        Ok(record)
    }

    /// Get a seal by ID.
    pub fn get_seal(&self, seal_id: &str) -> Result<SealRecord, String> {
        self.store.get_seal(seal_id).map_err(|e| format!("{}", e))
    }

    /// List all seals.
    pub fn list_seals(&self, chain: Option<Chain>) -> Result<Vec<SealRecord>, String> {
        self.store.list_seals(chain).map_err(|e| format!("{}", e))
    }

    /// Update seal status.
    pub fn update_seal_status(
        &self,
        seal_id: &str,
        status: SealStatus,
    ) -> Result<(), String> {
        let mut seal = self.store.get_seal(seal_id).map_err(|e| format!("{}", e))?;
        seal.status = status;
        seal.updated_at = chrono::Utc::now();
        self.store.save_seal(&seal).map_err(|e| format!("{}", e))
    }

    /// Check if a seal is consumed.
    pub fn is_seal_consumed(&self, seal_id: &str) -> Result<bool, String> {
        let seal = self.store.get_seal(seal_id).map_err(|e| format!("{}", e))?;
        Ok(!matches!(seal.status, SealStatus::Unconsumed))
    }

    /// Get seals for a specific right.
    pub fn get_seals_for_right(&self, right_id: &RightId) -> Result<Vec<SealRecord>, String> {
        self.store.get_seals_for_right(right_id).map_err(|e| format!("{}", e))
    }

    /// Get seal history.
    pub fn get_seal_history(&self, limit: usize) -> Result<Vec<SealRecord>, String> {
        self.store.get_seal_history(limit).map_err(|e| format!("{}", e))
    }
}
