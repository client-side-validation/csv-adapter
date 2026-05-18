//! Replay Registry for Cross-Chain Attack Prevention
//!
//! This module provides replay protection by tracking all validated proofs
//! across chains to prevent cross-chain replay attacks.
//!
//! ## Security Purpose
//!
//! The replay registry is the primary defense against cross-chain replay attacks.
//! When a proof is validated, it is recorded here. Any subsequent attempt to
//! replay the same proof (same proof_hash, seal_id, commitment_hash) will be
//! detected and rejected, even across different chains.
//!
//! ## Persistence
//!
//! The registry MUST persist across:
//! - application restart
//! - crash recovery
//! - node migration
//!
//! This is achieved through the persistent backend in csv-store.
//!
//! ## Usage
//!
//! Use [`ReplayRegistry`] for in-memory replay protection (tests, development).
//! Use [`PersistentReplayRegistry`] for production deployments with SQLite persistence.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

use crate::domain_hash::DomainSeparatedHash;
use crate::domains::ReplayRegistryDomain;
use crate::hash::Hash;
use crate::protocol_version::ChainId;

/// Replay key that uniquely identifies a proof for replay detection
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReplayKey {
    /// Hash of the proof bundle
    pub proof_hash: Hash,
    /// Seal ID that was consumed
    pub seal_id: Hash,
    /// Commitment hash
    pub commitment_hash: Hash,
    /// Source chain
    pub source_chain: ChainId,
    /// Destination chain
    pub destination_chain: ChainId,
}

impl ReplayKey {
    /// Create a new replay key
    pub fn new(
        proof_hash: Hash,
        seal_id: Hash,
        commitment_hash: Hash,
        source_chain: ChainId,
        destination_chain: ChainId,
    ) -> Self {
        Self {
            proof_hash,
            seal_id,
            commitment_hash,
            source_chain,
            destination_chain,
        }
    }

    /// Compute the domain-separated hash of this replay key
    ///
    /// This hash is used as the primary key in the replay registry.
    pub fn hash(&self) -> Hash {
        let mut payload = Vec::new();
        payload.extend_from_slice(self.proof_hash.as_bytes());
        payload.extend_from_slice(self.seal_id.as_bytes());
        payload.extend_from_slice(self.commitment_hash.as_bytes());
        payload.extend_from_slice(self.source_chain.as_bytes());
        payload.extend_from_slice(self.destination_chain.as_bytes());

        DomainSeparatedHash::<ReplayRegistryDomain>::hash(&payload)
    }
}

/// Replay registry entry
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReplayEntry {
    /// The replay key
    pub key: ReplayKey,
    /// Timestamp when this proof was first seen (Unix epoch seconds)
    pub first_seen_at: u64,
    /// Number of replay attempts detected
    pub replay_attempts: u64,
    /// Whether this proof has been accepted
    pub accepted: bool,
}

/// In-memory replay registry
///
/// This is the in-memory representation. For persistence, use the
/// ReplayRegistryStore from csv-store.
#[derive(Default)]
pub struct ReplayRegistry {
    /// Map from replay key hash to entry
    entries: BTreeMap<Hash, ReplayEntry>,
}

impl ReplayRegistry {
    /// Create a new empty replay registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a proof in the replay registry
    ///
    /// Returns true if this is the first time seeing this proof,
    /// false if it's a replay attempt.
    pub fn record_proof(&mut self, key: ReplayKey, timestamp: u64) -> bool {
        let key_hash = key.hash();

        match self.entries.get(&key_hash) {
            Some(entry) => {
                // Replay attempt detected
                let entry = entry.clone();
                let mut updated = entry;
                updated.replay_attempts += 1;
                self.entries.insert(key_hash, updated);
                false
            }
            None => {
                // First time seeing this proof
                let entry = ReplayEntry {
                    key,
                    first_seen_at: timestamp,
                    replay_attempts: 0,
                    accepted: false,
                };
                self.entries.insert(key_hash, entry);
                true
            }
        }
    }

    /// Check if a proof has been seen before
    pub fn has_been_seen(&self, key: &ReplayKey) -> bool {
        let key_hash = key.hash();
        self.entries.contains_key(&key_hash)
    }

    /// Mark a proof as accepted
    pub fn mark_accepted(&mut self, key: &ReplayKey) {
        let key_hash = key.hash();
        if let Some(entry) = self.entries.get_mut(&key_hash) {
            entry.accepted = true;
        }
    }

    /// Get the number of replay attempts for a proof
    pub fn replay_attempts(&self, key: &ReplayKey) -> u64 {
        let key_hash = key.hash();
        self.entries
            .get(&key_hash)
            .map(|e| e.replay_attempts)
            .unwrap_or(0)
    }

    /// Get all entries
    pub fn entries(&self) -> Vec<&ReplayEntry> {
        self.entries.values().collect()
    }

    /// Get the total number of tracked proofs
    pub fn total_proofs(&self) -> usize {
        self.entries.len()
    }

    /// Get the number of replay attempts detected
    pub fn total_replay_attempts(&self) -> u64 {
        self.entries.values().map(|e| e.replay_attempts).sum()
    }

    /// Idempotent consume-if-unconsumed operation.
    ///
    /// This is the ONLY safe way to consume a seal. It uses atomic
    /// compare-and-swap semantics to prevent double-consume races.
    ///
    /// Returns Ok(true) if the seal was successfully consumed.
    /// Returns Ok(false) if the seal was already consumed (idempotent).
    /// Returns Err if the operation failed due to a replay attack.
    pub fn consume_if_unconsumed(&mut self, key: ReplayKey, timestamp: u64) -> Result<bool, String> {
        let key_hash = key.hash();

        match self.entries.get(&key_hash) {
            Some(entry) => {
                if entry.accepted {
                    // Seal already consumed - idempotent success
                    Ok(false)
                } else {
                    // Entry exists but not yet accepted - replay attempt
                    Err(format!(
                        "Replay attack detected: proof key already exists but not accepted"
                    ))
                }
            }
            None => {
                // First time seeing this proof - consume it
                let entry = ReplayEntry {
                    key,
                    first_seen_at: timestamp,
                    replay_attempts: 0,
                    accepted: true,
                };
                self.entries.insert(key_hash, entry);
                Ok(true)
            }
        }
    }
}

/// Persistent replay registry backend trait
///
/// This trait abstracts the persistence layer, allowing consumers to
/// implement their own storage backend for replay protection.
///
/// The csv-store crate provides a SQLite implementation:
/// [`csv_store::ReplayRegistryStore`](https://docs.rs/csv-store/latest/csv_store/struct.ReplayRegistryStore.html)
#[async_trait::async_trait]
pub trait ReplayRegistryBackend: Send + Sync + 'static {
    /// Record a proof, returning true if first time, false if replay
    async fn record_proof(
        &self,
        key: ReplayKey,
        timestamp: u64,
    ) -> crate::error::Result<bool>;
    /// Check if a proof has been seen before
    async fn has_been_seen(
        &self,
        key: &ReplayKey,
    ) -> crate::error::Result<bool>;
    /// Mark a proof as accepted
    async fn mark_accepted(
        &self,
        key: &ReplayKey,
    ) -> crate::error::Result<()>;
    /// Get replay attempt count for a key
    async fn replay_attempts(
        &self,
        key: &ReplayKey,
    ) -> crate::error::Result<u64>;
    /// Get total tracked proofs
    async fn total_proofs(
        &self,
    ) -> crate::error::Result<usize>;
    /// Get total replay attempts detected
    async fn total_replay_attempts(
        &self,
    ) -> crate::error::Result<u64>;

    /// Idempotent consume-if-unconsumed operation.
    ///
    /// This is the ONLY safe way to consume a seal. It uses atomic
    /// compare-and-swap semantics to prevent double-consume races.
    ///
    /// Returns Ok(true) if the seal was successfully consumed.
    /// Returns Ok(false) if the seal was already consumed (idempotent).
    /// Returns Err if the operation failed due to a database error or replay attack.
    async fn consume_if_unconsumed(
        &self,
        key: ReplayKey,
        timestamp: u64,
    ) -> crate::error::Result<bool>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replay_key_creation() {
        let key = ReplayKey::new(
            Hash::new([1u8; 32]),
            Hash::new([2u8; 32]),
            Hash::new([3u8; 32]),
            ChainId::new("bitcoin"),
            ChainId::new("ethereum"),
        );
        assert_eq!(key.proof_hash, Hash::new([1u8; 32]));
    }

    #[test]
    fn test_replay_key_hash() {
        let key1 = ReplayKey::new(
            Hash::new([1u8; 32]),
            Hash::new([2u8; 32]),
            Hash::new([3u8; 32]),
            ChainId::new("bitcoin"),
            ChainId::new("ethereum"),
        );

        let key2 = ReplayKey::new(
            Hash::new([1u8; 32]),
            Hash::new([2u8; 32]),
            Hash::new([3u8; 32]),
            ChainId::new("bitcoin"),
            ChainId::new("ethereum"),
        );

        assert_eq!(key1.hash(), key2.hash());
    }

    #[test]
    fn test_replay_key_hash_different_chains() {
        let key1 = ReplayKey::new(
            Hash::new([1u8; 32]),
            Hash::new([2u8; 32]),
            Hash::new([3u8; 32]),
            ChainId::new("bitcoin"),
            ChainId::new("ethereum"),
        );

        let key2 = ReplayKey::new(
            Hash::new([1u8; 32]),
            Hash::new([2u8; 32]),
            Hash::new([3u8; 32]),
            ChainId::new("bitcoin"),
            ChainId::new("solana"),
        );

        assert_ne!(key1.hash(), key2.hash());
    }

    #[test]
    fn test_record_proof_first_time() {
        let mut registry = ReplayRegistry::new();
        let key = ReplayKey::new(
            Hash::new([1u8; 32]),
            Hash::new([2u8; 32]),
            Hash::new([3u8; 32]),
            ChainId::new("bitcoin"),
            ChainId::new("ethereum"),
        );

        let first_time = registry.record_proof(key.clone(), 1000);
        assert!(first_time);
        assert_eq!(registry.total_proofs(), 1);
    }

    #[test]
    fn test_record_proof_replay() {
        let mut registry = ReplayRegistry::new();
        let key = ReplayKey::new(
            Hash::new([1u8; 32]),
            Hash::new([2u8; 32]),
            Hash::new([3u8; 32]),
            ChainId::new("bitcoin"),
            ChainId::new("ethereum"),
        );

        registry.record_proof(key.clone(), 1000);
        let replay = registry.record_proof(key.clone(), 2000);

        assert!(!replay);
        assert_eq!(registry.replay_attempts(&key), 1);
    }

    #[test]
    fn test_has_been_seen() {
        let mut registry = ReplayRegistry::new();
        let key = ReplayKey::new(
            Hash::new([1u8; 32]),
            Hash::new([2u8; 32]),
            Hash::new([3u8; 32]),
            ChainId::new("bitcoin"),
            ChainId::new("ethereum"),
        );

        assert!(!registry.has_been_seen(&key));
        registry.record_proof(key.clone(), 1000);
        assert!(registry.has_been_seen(&key));
    }

    #[test]
    fn test_mark_accepted() {
        let mut registry = ReplayRegistry::new();
        let key = ReplayKey::new(
            Hash::new([1u8; 32]),
            Hash::new([2u8; 32]),
            Hash::new([3u8; 32]),
            ChainId::new("bitcoin"),
            ChainId::new("ethereum"),
        );

        registry.record_proof(key.clone(), 1000);
        registry.mark_accepted(&key);

        assert!(registry.entries()[0].accepted);
    }

    #[test]
    fn test_consume_if_unconsumed_first_time() {
        let mut registry = ReplayRegistry::new();
        let key = ReplayKey::new(
            Hash::new([1u8; 32]),
            Hash::new([2u8; 32]),
            Hash::new([3u8; 32]),
            ChainId::new("bitcoin"),
            ChainId::new("ethereum"),
        );

        let result = registry.consume_if_unconsumed(key.clone(), 1000);
        assert!(result.is_ok());
        assert!(result.unwrap(), "First consumption should succeed");
        assert_eq!(registry.total_proofs(), 1);
    }

    #[test]
    fn test_consume_if_unconsumed_idempotent() {
        let mut registry = ReplayRegistry::new();
        let key = ReplayKey::new(
            Hash::new([1u8; 32]),
            Hash::new([2u8; 32]),
            Hash::new([3u8; 32]),
            ChainId::new("bitcoin"),
            ChainId::new("ethereum"),
        );

        // First consumption
        let result1 = registry.consume_if_unconsumed(key.clone(), 1000);
        assert!(result1.is_ok());
        assert!(result1.unwrap(), "First consumption should succeed");

        // Second consumption - should be idempotent
        let result2 = registry.consume_if_unconsumed(key.clone(), 2000);
        assert!(result2.is_ok());
        assert!(!result2.unwrap(), "Second consumption should return false (idempotent)");
    }

    #[test]
    fn test_consume_if_unconsumed_replay_attack() {
        let mut registry = ReplayRegistry::new();
        let key = ReplayKey::new(
            Hash::new([1u8; 32]),
            Hash::new([2u8; 32]),
            Hash::new([3u8; 32]),
            ChainId::new("bitcoin"),
            ChainId::new("ethereum"),
        );

        // Record proof but don't mark as accepted
        registry.record_proof(key.clone(), 1000);

        // Attempt to consume - should detect replay attack
        let result = registry.consume_if_unconsumed(key.clone(), 2000);
        assert!(result.is_err(), "Should detect replay attack");
    }
}
