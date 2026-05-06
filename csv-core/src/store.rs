//! Persistent seal and anchor storage
//!
//! Provides a trait-based abstraction for persisting consumed seals,
//! published anchors, and chain state across restarts.

use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

use crate::hash::Hash;

/// A persisted seal record
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SealRecord {
    /// Chain identifier (e.g., "bitcoin", "ethereum")
    pub chain: String,
    /// Seal identifier (chain-specific encoding)
    pub seal_id: Vec<u8>,
    /// Block height when the seal was consumed
    pub consumed_at_height: u64,
    /// Commitment hash that consumed this seal
    pub commitment_hash: Hash,
    /// Timestamp (Unix epoch seconds) when recorded
    pub recorded_at: u64,
}

/// A persisted anchor record
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnchorRecord {
    /// Chain identifier
    pub chain: String,
    /// Anchor identifier (chain-specific encoding)
    pub anchor_id: Vec<u8>,
    /// Block height where the anchor was included
    pub block_height: u64,
    /// Commitment hash that was anchored
    pub commitment_hash: Hash,
    /// Whether this anchor has been finalized
    pub is_finalized: bool,
    /// Number of confirmations at time of recording
    pub confirmations: u64,
    /// Timestamp (Unix epoch seconds)
    pub recorded_at: u64,
}

/// Trait for persistent seal and anchor storage
pub trait SealStore: Send + Sync {
    /// Save a consumed seal record
    fn save_seal(&mut self, record: &SealRecord) -> Result<(), StoreError>;

    /// Check if a seal has been consumed
    fn is_seal_consumed(&self, chain: &str, seal_id: &[u8]) -> Result<bool, StoreError>;

    /// Get all consumed seals for a chain
    fn get_seals(&self, chain: &str) -> Result<Vec<SealRecord>, StoreError>;

    /// Remove a seal record (for reorg rollback)
    fn remove_seal(&mut self, chain: &str, seal_id: &[u8]) -> Result<(), StoreError>;

    /// Remove all seals consumed after a given height (reorg rollback)
    fn remove_seals_after(&mut self, chain: &str, height: u64) -> Result<usize, StoreError>;

    /// Save a published anchor record
    fn save_anchor(&mut self, record: &AnchorRecord) -> Result<(), StoreError>;

    /// Check if an anchor exists
    fn has_anchor(&self, chain: &str, anchor_id: &[u8]) -> Result<bool, StoreError>;

    /// Update anchor finalization status
    fn finalize_anchor(
        &mut self,
        chain: &str,
        anchor_id: &[u8],
        confirmations: u64,
    ) -> Result<(), StoreError>;

    /// Get anchors that are not yet finalized
    fn pending_anchors(&self, chain: &str) -> Result<Vec<AnchorRecord>, StoreError>;

    /// Remove anchors published after a given height (reorg rollback)
    fn remove_anchors_after(&mut self, chain: &str, height: u64) -> Result<usize, StoreError>;

    /// Get the highest recorded block height for a chain
    fn highest_block(&self, chain: &str) -> Result<u64, StoreError>;
}

/// In-memory store for testing and lightweight use
pub struct InMemorySealStore {
    seals: Vec<SealRecord>,
    anchors: Vec<AnchorRecord>,
    rights: Vec<RightRecord>,
}

impl InMemorySealStore {
    /// Create a new empty in-memory store
    pub fn new() -> Self {
        Self {
            seals: Vec::new(),
            anchors: Vec::new(),
            rights: Vec::new(),
        }
    }
}

impl Default for InMemorySealStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SealStore for InMemorySealStore {
    fn save_seal(&mut self, record: &SealRecord) -> Result<(), StoreError> {
        self.seals.push(record.clone());
        Ok(())
    }

    fn is_seal_consumed(&self, chain: &str, seal_id: &[u8]) -> Result<bool, StoreError> {
        Ok(self
            .seals
            .iter()
            .any(|s| s.chain == chain && s.seal_id == seal_id))
    }

    fn get_seals(&self, chain: &str) -> Result<Vec<SealRecord>, StoreError> {
        Ok(self
            .seals
            .iter()
            .filter(|s| s.chain == chain)
            .cloned()
            .collect())
    }

    fn remove_seal(&mut self, chain: &str, seal_id: &[u8]) -> Result<(), StoreError> {
        self.seals
            .retain(|s| !(s.chain == chain && s.seal_id == seal_id));
        Ok(())
    }

    fn remove_seals_after(&mut self, chain: &str, height: u64) -> Result<usize, StoreError> {
        let before = self.seals.len();
        self.seals
            .retain(|s| !(s.chain == chain && s.consumed_at_height > height));
        Ok(before - self.seals.len())
    }

    fn save_anchor(&mut self, record: &AnchorRecord) -> Result<(), StoreError> {
        self.anchors.push(record.clone());
        Ok(())
    }

    fn has_anchor(&self, chain: &str, anchor_id: &[u8]) -> Result<bool, StoreError> {
        Ok(self
            .anchors
            .iter()
            .any(|a| a.chain == chain && a.anchor_id == anchor_id))
    }

    fn finalize_anchor(
        &mut self,
        chain: &str,
        anchor_id: &[u8],
        confirmations: u64,
    ) -> Result<(), StoreError> {
        if let Some(a) = self
            .anchors
            .iter_mut()
            .find(|a| a.chain == chain && a.anchor_id == anchor_id)
        {
            a.is_finalized = true;
            a.confirmations = confirmations;
        }
        Ok(())
    }

    fn pending_anchors(&self, chain: &str) -> Result<Vec<AnchorRecord>, StoreError> {
        Ok(self
            .anchors
            .iter()
            .filter(|a| a.chain == chain && !a.is_finalized)
            .cloned()
            .collect())
    }

    fn remove_anchors_after(&mut self, chain: &str, height: u64) -> Result<usize, StoreError> {
        let before = self.anchors.len();
        self.anchors
            .retain(|a| !(a.chain == chain && a.block_height > height));
        Ok(before - self.anchors.len())
    }

    fn highest_block(&self, chain: &str) -> Result<u64, StoreError> {
        Ok(self
            .anchors
            .iter()
            .filter(|a| a.chain == chain)
            .map(|a| a.block_height)
            .max()
            .unwrap_or(0))
    }
}

impl RightStore for InMemorySealStore {
    fn save_right(&mut self, record: &RightRecord) -> Result<(), StoreError> {
        // Check for duplicate
        if self.has_right(&record.right_id)? {
            return Err(StoreError::DuplicateRecord(format!(
                "Right with ID {:?} already exists",
                record.right_id
            )));
        }
        self.rights.push(record.clone());
        Ok(())
    }

    fn get_right(&self, right_id: &crate::right::RightId) -> Result<Option<RightRecord>, StoreError> {
        Ok(self
            .rights
            .iter()
            .find(|r| r.right_id.0.as_bytes() == right_id.0.as_bytes())
            .cloned())
    }

    fn list_rights_by_chain(&self, chain: &str) -> Result<Vec<RightRecord>, StoreError> {
        Ok(self
            .rights
            .iter()
            .filter(|r| r.chain == chain)
            .cloned()
            .collect())
    }

    fn list_rights_by_owner(&self, owner: &[u8]) -> Result<Vec<RightRecord>, StoreError> {
        Ok(self
            .rights
            .iter()
            .filter(|r| r.owner == owner)
            .cloned()
            .collect())
    }

    fn consume_right(
        &mut self,
        right_id: &crate::right::RightId,
        consumed_at: u64,
    ) -> Result<(), StoreError> {
        if let Some(r) = self
            .rights
            .iter_mut()
            .find(|r| r.right_id.0.as_bytes() == right_id.0.as_bytes())
        {
            if r.consumed {
                return Err(StoreError::DuplicateRecord(format!(
                    "Right {:?} already consumed",
                    right_id
                )));
            }
            r.consumed = true;
            r.consumed_at = Some(consumed_at);
            Ok(())
        } else {
            Err(StoreError::NotFound(format!(
                "Right {:?} not found",
                right_id
            )))
        }
    }

    fn list_consumed_rights(&self) -> Result<Vec<RightRecord>, StoreError> {
        Ok(self
            .rights
            .iter()
            .filter(|r| r.consumed)
            .cloned()
            .collect())
    }

    fn list_active_rights(&self) -> Result<Vec<RightRecord>, StoreError> {
        Ok(self
            .rights
            .iter()
            .filter(|r| !r.consumed)
            .cloned()
            .collect())
    }

    fn has_right(&self, right_id: &crate::right::RightId) -> Result<bool, StoreError> {
        Ok(self
            .rights
            .iter()
            .any(|r| r.right_id.0.as_bytes() == right_id.0.as_bytes()))
    }

    fn delete_right(&mut self, right_id: &crate::right::RightId) -> Result<(), StoreError> {
        let before = self.rights.len();
        self.rights
            .retain(|r| r.right_id.0.as_bytes() != right_id.0.as_bytes());
        if self.rights.len() == before {
            return Err(StoreError::NotFound(format!(
                "Right {:?} not found",
                right_id
            )));
        }
        Ok(())
    }
}

/// A persisted Right record
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RightRecord {
    /// Right ID (unique identifier)
    pub right_id: crate::right::RightId,
    /// The chain where this Right is anchored
    pub chain: String,
    /// Owner identifier (address or pubkey)
    pub owner: Vec<u8>,
    /// The serialized Right data
    pub right_data: Vec<u8>,
    /// Whether the Right has been consumed
    pub consumed: bool,
    /// Timestamp (Unix epoch seconds) when recorded
    pub recorded_at: u64,
    /// Timestamp (Unix epoch seconds) when consumed (if applicable)
    pub consumed_at: Option<u64>,
}

/// Trait for persistent Right storage
///
/// This trait extends the seal storage with Right-specific operations
/// required by the RightsManager facade.
pub trait RightStore: Send + Sync {
    /// Save a Right to the store
    fn save_right(&mut self, record: &RightRecord) -> Result<(), StoreError>;

    /// Get a Right by its ID
    fn get_right(&self, right_id: &crate::right::RightId) -> Result<Option<RightRecord>, StoreError>;

    /// List all Rights for a specific chain
    fn list_rights_by_chain(&self, chain: &str) -> Result<Vec<RightRecord>, StoreError>;

    /// List all Rights for a specific owner
    fn list_rights_by_owner(&self, owner: &[u8]) -> Result<Vec<RightRecord>, StoreError>;

    /// Mark a Right as consumed
    fn consume_right(
        &mut self,
        right_id: &crate::right::RightId,
        consumed_at: u64,
    ) -> Result<(), StoreError>;

    /// List consumed Rights
    fn list_consumed_rights(&self) -> Result<Vec<RightRecord>, StoreError>;

    /// List unconsumed (active) Rights
    fn list_active_rights(&self) -> Result<Vec<RightRecord>, StoreError>;

    /// Check if a Right exists
    fn has_right(&self, right_id: &crate::right::RightId) -> Result<bool, StoreError>;

    /// Delete a Right (for administrative purposes)
    fn delete_right(&mut self, right_id: &crate::right::RightId) -> Result<(), StoreError>;
}

/// Store error types
#[derive(Debug)]
#[allow(missing_docs)]
pub enum StoreError {
    /// Database I/O error
    IoError(String),
    /// Serialization error
    SerializationError(String),
    /// Duplicate record
    DuplicateRecord(String),
    /// Record not found
    NotFound(String),
}

impl core::fmt::Display for StoreError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            StoreError::IoError(msg) => write!(f, "I/O error: {}", msg),
            StoreError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            StoreError::DuplicateRecord(msg) => write!(f, "Duplicate record: {}", msg),
            StoreError::NotFound(msg) => write!(f, "Not found: {}", msg),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_seal_record(chain: &str, height: u64) -> SealRecord {
        SealRecord {
            chain: chain.to_string(),
            seal_id: vec![1, 2, 3],
            consumed_at_height: height,
            commitment_hash: Hash::new([0xAA; 32]),
            recorded_at: 1700000000,
        }
    }

    fn test_anchor_record(chain: &str, height: u64) -> AnchorRecord {
        AnchorRecord {
            chain: chain.to_string(),
            anchor_id: vec![4, 5, 6],
            block_height: height,
            commitment_hash: Hash::new([0xBB; 32]),
            is_finalized: false,
            confirmations: 0,
            recorded_at: 1700000000,
        }
    }

    #[test]
    fn test_store_seal_and_check() {
        let mut store = InMemorySealStore::new();
        let record = test_seal_record("bitcoin", 100);
        store.save_seal(&record).unwrap();
        assert!(store.is_seal_consumed("bitcoin", &[1, 2, 3]).unwrap());
        assert!(!store.is_seal_consumed("ethereum", &[1, 2, 3]).unwrap());
    }

    #[test]
    fn test_remove_seal() {
        let mut store = InMemorySealStore::new();
        store.save_seal(&test_seal_record("bitcoin", 100)).unwrap();
        store.remove_seal("bitcoin", &[1, 2, 3]).unwrap();
        assert!(!store.is_seal_consumed("bitcoin", &[1, 2, 3]).unwrap());
    }

    #[test]
    fn test_remove_seals_after_height() {
        let mut store = InMemorySealStore::new();
        store.save_seal(&test_seal_record("bitcoin", 100)).unwrap();
        store.save_seal(&test_seal_record("bitcoin", 150)).unwrap();
        store.save_seal(&test_seal_record("bitcoin", 200)).unwrap();
        let removed = store.remove_seals_after("bitcoin", 150).unwrap();
        assert_eq!(removed, 1);
        assert!(store.is_seal_consumed("bitcoin", &[1, 2, 3]).unwrap());
    }

    #[test]
    fn test_anchor_lifecycle() {
        let mut store = InMemorySealStore::new();
        let anchor = test_anchor_record("bitcoin", 100);
        store.save_anchor(&anchor).unwrap();
        assert!(store.has_anchor("bitcoin", &[4, 5, 6]).unwrap());

        // Initially not finalized
        let pending = store.pending_anchors("bitcoin").unwrap();
        assert_eq!(pending.len(), 1);

        // Finalize
        store.finalize_anchor("bitcoin", &[4, 5, 6], 6).unwrap();
        let pending = store.pending_anchors("bitcoin").unwrap();
        assert!(pending.is_empty());
    }

    #[test]
    fn test_remove_anchors_after_height() {
        let mut store = InMemorySealStore::new();
        store
            .save_anchor(&test_anchor_record("bitcoin", 100))
            .unwrap();
        store
            .save_anchor(&test_anchor_record("bitcoin", 200))
            .unwrap();
        store
            .save_anchor(&test_anchor_record("bitcoin", 300))
            .unwrap();
        let removed = store.remove_anchors_after("bitcoin", 200).unwrap();
        assert_eq!(removed, 1);
        assert!(store.has_anchor("bitcoin", &[4, 5, 6]).unwrap());
    }

    #[test]
    fn test_highest_block() {
        let mut store = InMemorySealStore::new();
        store
            .save_anchor(&test_anchor_record("bitcoin", 100))
            .unwrap();
        store
            .save_anchor(&test_anchor_record("bitcoin", 300))
            .unwrap();
        store
            .save_anchor(&test_anchor_record("bitcoin", 200))
            .unwrap();
        assert_eq!(store.highest_block("bitcoin").unwrap(), 300);
        assert_eq!(store.highest_block("ethereum").unwrap(), 0);
    }

    #[test]
    fn test_multi_chain_isolation() {
        let mut store = InMemorySealStore::new();
        store.save_seal(&test_seal_record("bitcoin", 100)).unwrap();
        store.save_seal(&test_seal_record("ethereum", 200)).unwrap();

        assert_eq!(store.get_seals("bitcoin").unwrap().len(), 1);
        assert_eq!(store.get_seals("ethereum").unwrap().len(), 1);
    }

    #[test]
    fn test_store_error_display() {
        let err = StoreError::IoError("disk full".to_string());
        assert!(err.to_string().contains("disk full"));
    }
}
