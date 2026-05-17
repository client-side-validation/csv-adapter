//! Durable replay database trait
//!
//! Behavioral invariants:
//! 1. Append-only: entries are never deleted, only marked rolled-back.
//! 2. Compare-and-swap insert: `insert_if_absent` must fail if the key exists at write time,
//!    not only at read time. A read-then-write with a gap is not sufficient.
//! 3. Durability: a crash after a successful `insert` must not lose the entry.
//! 4. The implementation must document its behavior under concurrent writers.
//!
//! The specific storage backend (RocksDB, SQLite, PostgreSQL + advisory lock,
//! distributed CAS store) is left to the implementation. Do not freeze the backend
//! choice in this interface.

#![allow(missing_docs)]


use csv_core::proof::ReplayId;
use crate::error::RuntimeError;

/// State of a replay entry
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayEntryState {
    /// Insert recorded; mint has not yet been confirmed on-chain.
    Pending,
    /// Mint confirmed on-chain. Terminal state.
    Consumed,
    /// Transfer failed after insert; recovery coordinator may retry.
    RolledBack,
}

/// Errors from replay database operations
#[derive(Debug, thiserror::Error)]
pub enum ReplayDbError {
    /// ReplayId already present — replay attempt or concurrent insert
    #[error("ReplayId already present — replay attempt or concurrent insert")]
    AlreadyExists,
    /// Storage error
    #[error("Storage error: {0}")]
    Storage(String),
}

/// Trait for replay database implementations.
///
/// ## Concurrency Notes
///
/// **Unresolved problem 1 — concurrent coordinators**: Two coordinator processes checking
/// the same ReplayId simultaneously both find it absent, both proceed to `insert`, both
/// attempt to mint. A local `RocksDB` insert is not a distributed compare-and-swap. The
/// trait's `insert_if_absent` MUST provide compare-and-swap semantics — return an error
/// if the key already exists at the moment of write, not just at the moment of read.
///
/// **Unresolved problem 2 — partial failure between insert and mint**: The coordinator
/// inserts the ReplayId before minting (intentionally — to block duplicate mints on retry).
/// If the mint then fails, the transfer is permanently poisoned with no recovery path. The
/// rollback protocol for this case requires: (a) a separate `pending` state before `consumed`,
/// (b) a timeout-based expiry for `pending` entries, (c) a recovery coordinator that can
/// promote `pending` → `consumed` after verifying the mint on-chain, or demote
/// `pending` → `available` after confirming the mint never landed. This protocol is not
/// specified here and must be designed before production.
///
/// **Unresolved problem 3 — Byzantine destination adapter**: The destination chain adapter
/// returns a `MintReceipt` claiming success. The coordinator currently trusts this. Before
/// marking a transfer complete, the coordinator must independently verify the mint
/// transaction is present on-chain — not via the same adapter instance that performed the
/// mint, but via a quorum verification call against independent RPC nodes.
#[async_trait::async_trait]
pub trait ReplayDatabase: Send + Sync {
    /// Returns true if this ReplayId has been seen before.
    /// This check alone is not sufficient for concurrent-safe replay prevention —
    /// use `insert_if_absent` for the atomic check-and-record operation.
    async fn contains(&self, id: &ReplayId) -> Result<bool, RuntimeError>;

    /// Atomically record a ReplayId as pending mint.
    /// Returns `Err(AlreadyExists)` if the key is present at the moment of write.
    /// This is a compare-and-swap operation, not a blind insert.
    /// Callers must handle `AlreadyExists` as a replay attempt, not a transient error.
    async fn insert_if_absent(
        &self,
        id: &ReplayId,
        state: ReplayEntryState,
    ) -> Result<(), ReplayDbError>;

    /// Promote a `Pending` entry to `Consumed` after mint is confirmed on-chain.
    async fn confirm_consumed(&self, id: &ReplayId) -> Result<(), ReplayDbError>;

    /// Mark a `Pending` entry as rolled-back (append-only: entry remains, state changes).
    async fn mark_rolled_back(&self, id: &ReplayId) -> Result<(), RuntimeError>;
}

/// In-memory implementation of ReplayDatabase for testing.
/// Not suitable for production — does not provide durability or concurrent safety.
pub struct InMemoryReplayDb {
    entries: std::sync::Arc<std::sync::RwLock<std::collections::HashMap<ReplayId, ReplayEntryState>>>,
}

impl InMemoryReplayDb {
    /// Create a new in-memory replay database
    pub fn new() -> Self {
        Self {
            entries: std::sync::Arc::new(std::sync::RwLock::new(
                std::collections::HashMap::new(),
            )),
        }
    }
}

impl Default for InMemoryReplayDb {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl ReplayDatabase for InMemoryReplayDb {
    async fn contains(&self, id: &ReplayId) -> Result<bool, RuntimeError> {
        let entries = self.entries.read().unwrap();
        Ok(entries.contains_key(id))
    }

    async fn insert_if_absent(
        &self,
        id: &ReplayId,
        state: ReplayEntryState,
    ) -> Result<(), ReplayDbError> {
        let mut entries = self.entries.write().unwrap();
        if entries.contains_key(id) {
            return Err(ReplayDbError::AlreadyExists);
        }
        entries.insert(*id, state);
        Ok(())
    }

    async fn confirm_consumed(&self, id: &ReplayId) -> Result<(), ReplayDbError> {
        let mut entries = self.entries.write().unwrap();
        match entries.get_mut(id) {
            Some(current) if *current == ReplayEntryState::Pending => {
                *current = ReplayEntryState::Consumed;
                Ok(())
            }
            Some(_) => Err(ReplayDbError::Storage(
                "Entry is not in Pending state".to_string(),
            )),
            None => Err(ReplayDbError::Storage(
                "Entry not found".to_string(),
            )),
        }
    }

    async fn mark_rolled_back(&self, id: &ReplayId) -> Result<(), RuntimeError> {
        let mut entries = self.entries.write().unwrap();
        match entries.get_mut(id) {
            Some(current) if *current == ReplayEntryState::Pending => {
                *current = ReplayEntryState::RolledBack;
                Ok(())
            }
            Some(_) => Err(RuntimeError::InvalidState(
                "Entry is not in Pending state".to_string(),
            )),
            None => Err(RuntimeError::TransferNotFound(
                format!("ReplayId {:?}", id.as_bytes()),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_insert_and_contains() {
        let db = InMemoryReplayDb::new();
        let id = ReplayId::derive(
            "bitcoin",
            &[1u8; 32],
            0,
            &[2u8; 32],
            &[3u8; 32],
            "ethereum",
        );

        assert!(!db.contains(&id).await.unwrap());
        db.insert_if_absent(&id, ReplayEntryState::Pending)
            .await
            .unwrap();
        assert!(db.contains(&id).await.unwrap());
    }

    #[tokio::test]
    async fn test_insert_if_absent_duplicate_fails() {
        let db = InMemoryReplayDb::new();
        let id = ReplayId::derive(
            "bitcoin",
            &[1u8; 32],
            0,
            &[2u8; 32],
            &[3u8; 32],
            "ethereum",
        );

        db.insert_if_absent(&id, ReplayEntryState::Pending)
            .await
            .unwrap();

        let result = db.insert_if_absent(&id, ReplayEntryState::Pending).await;
        assert!(matches!(result, Err(ReplayDbError::AlreadyExists)));
    }

    #[tokio::test]
    async fn test_confirm_consumed() {
        let db = InMemoryReplayDb::new();
        let id = ReplayId::derive(
            "bitcoin",
            &[1u8; 32],
            0,
            &[2u8; 32],
            &[3u8; 32],
            "ethereum",
        );

        db.insert_if_absent(&id, ReplayEntryState::Pending)
            .await
            .unwrap();
        db.confirm_consumed(&id).await.unwrap();

        // Should still be contained
        assert!(db.contains(&id).await.unwrap());
    }

    #[tokio::test]
    async fn test_mark_rolled_back() {
        let db = InMemoryReplayDb::new();
        let id = ReplayId::derive(
            "bitcoin",
            &[1u8; 32],
            0,
            &[2u8; 32],
            &[3u8; 32],
            "ethereum",
        );

        db.insert_if_absent(&id, ReplayEntryState::Pending)
            .await
            .unwrap();
        db.mark_rolled_back(&id).await.unwrap();
    }
}
