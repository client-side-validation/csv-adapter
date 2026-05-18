//! Durable event store abstraction and implementations

use std::string::String;
use std::vec::Vec;

use crate::event_envelope::RuntimeEventEnvelope;

/// Synchronous event store used to persist envelopes before publishing.
pub trait EventStore: Send + Sync {
    /// Persist the provided envelope durably.
    fn persist(&self, envelope: &RuntimeEventEnvelope) -> Result<(), String>;
}

/// In-memory event store for non-persistent deployments and tests.
pub struct InMemoryEventStore {
    /// Stored envelopes in insertion order.
    pub records: std::sync::Mutex<Vec<RuntimeEventEnvelope>>,
}

impl InMemoryEventStore {
    /// Create a new in-memory event store.
    pub fn new() -> Self {
        Self {
            records: std::sync::Mutex::new(Vec::new()),
        }
    }
}

impl EventStore for InMemoryEventStore {
    fn persist(&self, envelope: &RuntimeEventEnvelope) -> Result<(), String> {
        let mut guard = self.records.lock().map_err(|e| e.to_string())?;
        guard.push(envelope.clone());
        Ok(())
    }
}

#[cfg(feature = "persistent")]
/// RocksDB-based event store for durability when `persistent` feature is enabled.
pub struct RocksDbEventStore {
    db: rocksdb::DB,
}

#[cfg(feature = "persistent")]
impl RocksDbEventStore {
    /// Open a RocksDB-backed event store at the given filesystem path.
    pub fn open(path: &str) -> Result<Self, String> {
        let opts = rocksdb::Options::default();
        match rocksdb::DB::open(&opts, path) {
            Ok(db) => Ok(Self { db }),
            Err(e) => Err(e.to_string()),
        }
    }
}

#[cfg(feature = "persistent")]
impl EventStore for RocksDbEventStore {
    fn persist(&self, envelope: &RuntimeEventEnvelope) -> Result<(), String> {
        let key = envelope.event_id.as_bytes();
        let value = match serde_json::to_vec(envelope) {
            Ok(v) => v,
            Err(e) => return Err(e.to_string()),
        };
        self.db.put(key, value).map_err(|e| e.to_string())
    }
}
