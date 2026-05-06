//! Seal manager.
//!
//! Core seal management operations.

use csv_core::{Chain, RightId, SealRef};
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
    /// Seal ID (human-readable format)
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
    /// Real chain-native seal reference (from chain adapter)
    /// This is the actual on-chain seal identifier, NOT a timestamp-based fake ID
    pub seal_ref: Option<SealRef>,
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
    ///
    /// # IMPORTANT: Protocol Correctness
    /// The `seal_ref` MUST be a real chain-native seal identifier obtained from
    /// a chain adapter's `create_seal()` method. This function will fail if no
    /// real seal reference is provided.
    ///
    /// # Arguments
    /// * `chain` - The blockchain where the seal is created
    /// * `value` - Optional value/funding for the seal (chain-specific units)
    /// * `seal_ref` - The real chain-native seal reference from the chain adapter (REQUIRED)
    ///
    /// # Returns
    /// A `SealRecord` with the real on-chain seal reference stored
    ///
    /// # Errors
    /// Returns an error if `seal_ref` is None (fake seals not allowed)
    pub fn create_seal(
        &self,
        chain: Chain,
        value: Option<u64>,
        seal_ref: Option<SealRef>,
    ) -> Result<SealRecord, String> {
        let seal_ref = seal_ref.ok_or_else(|| {
            "Protocol violation: Cannot create seal without a real chain-native SealRef. \
             Use the chain adapter's create_seal() method to obtain a real seal reference.".to_string()
        })?;

        let seal_id = hex::encode(&seal_ref.seal_id);

        let record = SealRecord {
            id: seal_id.clone(),
            chain,
            status: SealStatus::Unconsumed,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            right_id: None,
            value,
            seal_ref: Some(seal_ref),
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
