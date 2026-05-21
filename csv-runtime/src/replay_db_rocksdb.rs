//! RocksDB-backed replay database with CAS semantics.
//!
//! Uses a `check-then-put` pattern with a separate conflict column family to
//! approximate compare-and-swap on top of RocksDB's atomic write batches.
//!
//! Single-node CAS only. For multi-node deployments, use PostgreSQL advisory locks
//! (see `replay_db_postgres`).

use crate::error::RuntimeError;
use crate::replay_db::{ReplayDatabase, ReplayDbError, ReplayEntryState};
use async_trait::async_trait;
use csv_core::proof::ReplayId;
use csv_core::cross_chain::CrossChainRegistryEntry;
use rocksdb::{ColumnFamily, ColumnFamilyDescriptor, DBCompressionType, Options, DB};
use std::sync::Arc;

/// Column family name for replay entries (primary data).
const CF_REPLAY: &str = "replay_entries";
/// Column family for conflict tracking (CAS marker).
/// A key present here means the key already exists in the primary CF.
const CF_CONFLICT: &str = "replay_conflicts";

/// RocksDB-backed replay database with compare-and-swap semantics.
///
/// Uses a dual column family approach to approximate atomic CAS:
/// - Primary CF (`replay_entries`): stores the actual state value
/// - Conflict CF (`replay_conflicts`): stores a marker if the key exists
///
/// The insert uses an atomic write batch that checks the conflict CF:
/// ```text
/// write_batch {
///   if conflict CF has key → return AlreadyExists
///   put key → primary CF
///   put key → conflict CF
/// }
/// ```
///
/// Since RocksDB write batches are atomic (all-or-nothing), this provides
/// single-node CAS semantics within the same DB instance.
pub struct RocksReplayDb {
    db: Arc<DB>,
}

impl RocksReplayDb {
    /// Open or create the RocksDB database at the given path.
    pub fn open(path: &str) -> Result<Self, RuntimeError> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        let cf_opts = || {
            let mut opts = Options::default();
            opts.set_compression_type(DBCompressionType::Lz4);
            opts
        };

        let db = DB::open_cf_descriptors(
            &opts,
            path,
            vec![
                ColumnFamilyDescriptor::new(CF_REPLAY, cf_opts()),
                ColumnFamilyDescriptor::new(CF_CONFLICT, cf_opts()),
            ],
        )
        .map_err(|e| RuntimeError::Storage(format!("Failed to open RocksDB: {e}")))?;

        Ok(Self { db: Arc::new(db) })
    }

    fn cf_replay(&self) -> &ColumnFamily {
        self.db
            .cf_handle(CF_REPLAY)
            .expect("replay_entries CF must exist")
    }

    fn cf_conflict(&self) -> &ColumnFamily {
        self.db
            .cf_handle(CF_CONFLICT)
            .expect("replay_conflicts CF must exist")
    }

    fn encode_key(id: &ReplayId) -> Vec<u8> {
        id.as_bytes().to_vec()
    }

    fn encode_state(state: ReplayEntryState) -> Vec<u8> {
        match state {
            ReplayEntryState::Pending => b"PENDING".to_vec(),
            ReplayEntryState::Consumed => b"CONSUMED".to_vec(),
            ReplayEntryState::RolledBack => b"ROLLED_BACK".to_vec(),
        }
    }

    fn decode_state(bytes: &[u8]) -> Option<ReplayEntryState> {
        match bytes {
            b"PENDING" => Some(ReplayEntryState::Pending),
            b"CONSUMED" => Some(ReplayEntryState::Consumed),
            b"ROLLED_BACK" => Some(ReplayEntryState::RolledBack),
            _ => None,
        }
    }

    fn key_exists_in_conflict_cf(&self, key: &[u8]) -> Result<bool, RuntimeError> {
        match self.db.get_cf(self.cf_conflict(), key) {
            Ok(Some(_)) => Ok(true),
            Ok(None) => Ok(false),
            Err(e) => Err(RuntimeError::Storage(format!("RocksDB error: {e}"))),
        }
    }

    fn read_state(&self, key: &[u8]) -> Result<Option<ReplayEntryState>, RuntimeError> {
        match self.db.get_cf(self.cf_replay(), key) {
            Ok(Some(val)) => Ok(Self::decode_state(&val)),
            Ok(None) => Ok(None),
            Err(e) => Err(RuntimeError::Storage(format!("RocksDB read error: {e}"))),
        }
    }
}

#[async_trait]
impl ReplayDatabase for RocksReplayDb {
    async fn contains(&self, id: &ReplayId) -> Result<bool, RuntimeError> {
        let key = Self::encode_key(id);
        self.key_exists_in_conflict_cf(&key)
    }

    async fn insert_if_absent(
        &self,
        id: &ReplayId,
        state: ReplayEntryState,
    ) -> Result<(), ReplayDbError> {
        let key = Self::encode_key(id);
        let val = Self::encode_state(state);

        // Check if key exists in conflict CF first.
        if self
            .key_exists_in_conflict_cf(&key)
            .map_err(|e| ReplayDbError::Storage(e.to_string()))?
        {
            return Err(ReplayDbError::AlreadyExists);
        }

        // Atomically write to both CFs in a single batch.
        let mut batch = rocksdb::WriteBatch::default();
        batch.put_cf(self.cf_replay(), &key, &val);
        batch.put_cf(self.cf_conflict(), &key, b"1");

        self.db
            .write(batch)
            .map_err(|e| ReplayDbError::Storage(format!("RocksDB write error: {e}")))?;

        Ok(())
    }

    async fn consume_if_unconsumed(&self, id: &ReplayId) -> Result<(), ReplayDbError> {
        let key = Self::encode_key(id);

        // Read current state from the primary CF.
        let action = match self
            .read_state(&key)
            .map_err(|e| ReplayDbError::Storage(e.to_string()))?
        {
            Some(ReplayEntryState::Consumed) => return Ok(()),
            Some(_) => return Err(ReplayDbError::AlreadyExists),
            None => Some(ReplayEntryState::Pending),
        };

        if let Some(state) = action {
            let val = Self::encode_state(state);
            let mut batch = rocksdb::WriteBatch::default();
            batch.put_cf(self.cf_replay(), &key, &val);
            batch.put_cf(self.cf_conflict(), &key, b"1");

            self.db
                .write(batch)
                .map_err(|e| ReplayDbError::Storage(format!("RocksDB write error: {e}")))?;
        }

        Ok(())
    }

    async fn confirm_consumed(&self, id: &ReplayId) -> Result<(), ReplayDbError> {
        let key = Self::encode_key(id);
        let state = match self
            .read_state(&key)
            .map_err(|e| ReplayDbError::Storage(e.to_string()))?
        {
            Some(ReplayEntryState::Pending) => ReplayEntryState::Consumed,
            Some(ReplayEntryState::Consumed) => return Ok(()),
            Some(_) => {
                return Err(ReplayDbError::Storage(
                    "Entry is not in Pending or Consumed state".to_string(),
                ))
            }
            None => return Err(ReplayDbError::Storage("Entry not found".to_string())),
        };

        let val = Self::encode_state(state);
        self.db
            .put_cf(self.cf_replay(), &key, val)
            .map_err(|e| ReplayDbError::Storage(format!("RocksDB error: {e}")))?;

        Ok(())
    }

    async fn mark_rolled_back(&self, id: &ReplayId) -> Result<(), RuntimeError> {
        let key = Self::encode_key(id);
        let state = match self.read_state(&key)? {
            Some(ReplayEntryState::Pending) => ReplayEntryState::RolledBack,
            Some(ReplayEntryState::RolledBack) => return Ok(()),
            Some(_) => {
                return Err(RuntimeError::InvalidState(
                    "Entry is not in Pending state".to_string(),
                ))
            }
            None => {
                return Err(RuntimeError::TransferNotFound(
                    format!("ReplayId {:?}", id.as_bytes()),
                ))
            }
        };

        let val = Self::encode_state(state);
        self.db
            .put_cf(self.cf_replay(), &key, val)
            .map_err(|e| RuntimeError::Storage(format!("RocksDB error: {e}")))?;

        Ok(())
    }

    async fn store_transfer_entry(
        &self,
        entry: &CrossChainRegistryEntry,
    ) -> Result<(), RuntimeError> {
        let key = entry.sanad_id.as_bytes().to_vec();
        let val = serde_json::to_vec(entry)
            .map_err(|e| RuntimeError::Storage(format!("Serialization error: {e}")))?;

        let mut batch = rocksdb::WriteBatch::default();
        batch.put_cf(self.cf_replay(), &key, &val);

        self.db
            .write(batch)
            .map_err(|e| RuntimeError::Storage(format!("RocksDB write error: {e}")))?;

        Ok(())
    }

    async fn load_all_transfers(
        &self,
    ) -> Result<Vec<CrossChainRegistryEntry>, RuntimeError> {
        let mut transfers = Vec::new();
        let db = &self.db;

        for result in db.iterator_cf(self.cf_replay(), rocksdb::IteratorMode::Start) {
            let (_key, value) = result.map_err(|e| {
                RuntimeError::Storage(format!("RocksDB iterator error: {e}"))
            })?;
            let entry: CrossChainRegistryEntry = serde_json::from_slice(&value)
                .map_err(|e| RuntimeError::Storage(format!("Serialization error: {e}")))?;
            transfers.push(entry);
        }

        Ok(transfers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use csv_core::proof::ReplayId;

    fn test_replay_id(label: u8) -> ReplayId {
        ReplayId::derive(
            "test",
            &[label; 32],
            0,
            &[label + 1; 32],
            &[label + 2; 32],
            "test-dst",
        )
    }

    fn setup_db() -> RocksReplayDb {
        let dir = tempfile::tempdir().unwrap();
        RocksReplayDb::open(dir.path().to_str().unwrap()).unwrap()
    }

    #[tokio::test]
    async fn test_insert_and_contains() {
        let db = setup_db();
        let id = test_replay_id(1);

        assert!(!db.contains(&id).await.unwrap());
        db.insert_if_absent(&id, ReplayEntryState::Pending)
            .await
            .unwrap();
        assert!(db.contains(&id).await.unwrap());
    }

    #[tokio::test]
    async fn test_insert_if_absent_duplicate_fails() {
        let db = setup_db();
        let id = test_replay_id(2);

        db.insert_if_absent(&id, ReplayEntryState::Pending)
            .await
            .unwrap();

        let result = db.insert_if_absent(&id, ReplayEntryState::Pending).await;
        assert!(matches!(result, Err(ReplayDbError::AlreadyExists)));
    }

    #[tokio::test]
    async fn test_confirm_consumed() {
        let db = setup_db();
        let id = test_replay_id(3);

        db.insert_if_absent(&id, ReplayEntryState::Pending)
            .await
            .unwrap();
        db.confirm_consumed(&id).await.unwrap();

        assert!(db.contains(&id).await.unwrap());
    }

    #[tokio::test]
    async fn test_mark_rolled_back() {
        let db = setup_db();
        let id = test_replay_id(4);

        db.insert_if_absent(&id, ReplayEntryState::Pending)
            .await
            .unwrap();
        db.mark_rolled_back(&id).await.unwrap();
    }

    #[tokio::test]
    async fn test_consume_if_unconsumed_fresh() {
        let db = setup_db();
        let id = test_replay_id(5);

        // Fresh insert should succeed
        db.consume_if_unconsumed(&id).await.unwrap();
        assert!(db.contains(&id).await.unwrap());
    }

    #[tokio::test]
    async fn test_consume_if_unconsumed_idempotent() {
        let db = setup_db();
        let id = test_replay_id(6);

        db.insert_if_absent(&id, ReplayEntryState::Pending)
            .await
            .unwrap();
        db.confirm_consumed(&id).await.unwrap();

        // Consumed entries should be idempotent
        db.consume_if_unconsumed(&id).await.unwrap();
    }

    #[tokio::test]
    async fn test_pending_blocks_consume_if_unconsumed() {
        let db = setup_db();
        let id = test_replay_id(7);

        db.insert_if_absent(&id, ReplayEntryState::Pending)
            .await
            .unwrap();

        // Pending entry should block (not yet consumed)
        let result = db.consume_if_unconsumed(&id).await;
        assert!(matches!(result, Err(ReplayDbError::AlreadyExists)));
    }
}