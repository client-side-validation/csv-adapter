//! Cross-Chain Seal Registry - SECURITY CRITICAL
//!
//! Tracks consumed seals across all chains to detect double-consumption
//! attempts, including cross-chain double-spends.
//!
//! ## Security Purpose
//!
//! This registry is the **primary defense against double-spending** in the CSV protocol.
//! When a Right is consumed, the seal that enforced it is recorded here.
//! Any subsequent attempt to consume the same seal will be detected and rejected.
//!
//! ## Chain-Specific Seal Types
//!
//! | Chain | Seal Type | Identifier | Single-Use Enforcement |
//! |-------|-----------|------------|------------------------|
//! | Bitcoin | UTXO spend | txid:vout | Structural (output consumed) |
//! | Sui | Object deletion | object_id:version | Structural (object deleted) |
//! | Aptos | Resource destruction | resource_address:account | Type-enforced (Move linearity) |
//! | Ethereum | Nullifier registration | nullifier_hash | Contract-enforced (registry) |
//! | Solana | PDA closure | pda_address | Program-enforced (account closed) |
//!
//! ## Cross-Chain Double-Spend Detection
//!
//! The registry maps all seal types to a unified `SealIdentity` that can
//! be compared across chains. This detects:
//! - Same seal used twice on the same chain
//! - Equivalent seals used on different chains (cross-chain double-spend)
//!
//! ## Security Invariants
//!
//! 1. **At-Most-Once Consumption**: Each seal may appear in `consumed_seals`
//!    at most once with status `Unconsumed`. Subsequent consumption attempts
//!    result in `DoubleSpent` status.
//!
//! 2. **Immutable History**: Once a consumption is recorded, it cannot be
//!    removed or altered. This provides an audit trail for forensic analysis.
//!
//! 3. **Chain-Agnostic Detection**: The registry treats seals from different
//!    chains with the same identity as conflicts, preventing cross-chain replays.
//!
//! ## Audit Checklist
//!
//! - [ ] `record_consumption()` correctly detects and records double-spends
//! - [ ] `check_seal_status()` returns accurate status for all seal types
//! - [ ] Registry state persists across application restarts
//! - [ ] No path exists to remove or modify recorded consumptions
//! - [ ] Cross-chain seal identity collisions are properly handled

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::String;
use alloc::vec::Vec;

use crate::hash::Hash;
use crate::right::RightId;
use crate::seal::SealRef;

/// The chain that enforces this seal's single-use.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum ChainId {
    /// Bitcoin chain (UTXO seals)
    Bitcoin,
    /// Sui blockchain (Object seals)
    Sui,
    /// Aptos blockchain (Resource seals)
    Aptos,
    /// Ethereum blockchain (Nullifier seals)
    Ethereum,
    /// Solana blockchain (PDA seals)
    Solana,
    /// Custom or unknown chain
    Custom(String),
}

/// A seal consumption event recording when and where a seal was used.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SealConsumption {
    /// Which chain enforced this consumption
    pub chain: ChainId,
    /// The seal reference (chain-specific format)
    pub seal_ref: SealRef,
    /// The Right that was consumed
    pub right_id: RightId,
    /// Block height when this was consumed
    pub block_height: u64,
    /// Transaction/operation hash that consumed this seal
    pub tx_hash: Hash,
    /// Timestamp (Unix epoch seconds) when this was recorded
    pub recorded_at: u64,
}

/// Result of checking if a seal has been consumed.
#[derive(Debug, Clone)]
#[allow(missing_docs)]
pub enum SealStatus {
    /// Seal has not been consumed
    Unconsumed,
    /// Seal was consumed on a specific chain
    ConsumedOnChain {
        chain: ChainId,
        consumption: SealConsumption,
    },
    /// Seal was consumed on multiple chains (double-spend detected)
    DoubleSpent { consumptions: Vec<SealConsumption> },
}

/// Cross-chain seal registry - **Critical Security Component**.
///
/// Tracks all consumed seals across all chains and provides
/// double-consumption detection. This is the primary defense against
/// double-spending in the CSV protocol.
///
/// # Security Properties
///
/// 1. **Append-Only Log**: Consumption records are never deleted, providing
///    an immutable audit trail of all seal consumption events.
///
/// 2. **Deterministic Lookup**: Seal identity is derived from normalized
///    bytes, ensuring consistent lookup regardless of chain origin.
///
/// 3. **Cross-Chain Correlation**: The `right_consumption_map` enables
///    tracking all seals consumed by a specific Right across all chains.
///
/// # Thread Safety
///
/// This struct is NOT thread-safe by default. In multi-threaded contexts,
/// external synchronization (Mutex/RwLock) is required.
///
/// # Audit Note
///
/// The `consumed_seals` map is the critical data structure. Its key is
/// `seal_ref.to_vec()` and its value is a vector of consumption events.
/// - Empty vector: No consumption recorded
/// - One entry: Valid single consumption
/// - Multiple entries: Double-spend detected (preserved for forensics)
#[derive(Default)]
pub struct CrossChainSealRegistry {
    /// Map from seal identity to consumption events
    consumed_seals: BTreeMap<Vec<u8>, Vec<SealConsumption>>,
    /// Map from Right ID to seals that consumed it
    right_consumption_map: BTreeMap<Hash, Vec<SealConsumption>>,
    /// Set of known chain identifiers
    known_chains: BTreeSet<ChainId>,
}

impl CrossChainSealRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a seal consumption event.
    ///
    /// This is the **primary double-spend prevention method**. It atomically:
    /// 1. Checks if the seal has been consumed before
    /// 2. If not, records the consumption and returns Ok
    /// 3. If yes, records the attempted double-spend and returns Err
    ///
    /// # Security Requirements (CRITICAL)
    /// - MUST check existing consumptions before recording
    /// - MUST record double-spend attempts for forensic analysis
    /// - MUST update both `consumed_seals` and `right_consumption_map`
    /// - MUST be atomic (no race conditions between check and record)
    ///
    /// # Returns
    /// - `Ok(())` if this is the first valid consumption of this seal
    /// - `Err(DoubleSpendError)` if the seal was already consumed
    ///   (the new consumption is still recorded for auditing purposes)
    ///
    /// # Audit Note
    /// The double-spend detection logic must be carefully verified:
    /// 1. The `is_double_spend` check uses `contains_key()` and checks if
    ///    the existing entry is non-empty
    /// 2. Even on double-spend, we record the attempt for forensics
    /// 3. The error includes details of both the original and new consumption
    ///
    /// # Security Impact
    /// A bug in this method could allow double-spending. Verify:
    /// - No TOCTOU (time-of-check-time-of-use) race conditions
    /// - Proper handling of all seal reference types
    /// - Immutable recording of all consumption events
    pub fn record_consumption(
        &mut self,
        consumption: SealConsumption,
    ) -> Result<(), Box<DoubleSpendError>> {
        let seal_key = consumption.seal_ref.to_vec();
        let is_double_spend = self.consumed_seals.contains_key(&seal_key)
            && !self
                .consumed_seals
                .get(&seal_key)
                .map_or(true, |v| v.is_empty());

        // Track known chains
        self.known_chains.insert(consumption.chain.clone());

        // Check if already consumed
        if is_double_spend {
            let existing = self.consumed_seals.get(&seal_key).unwrap();
            let is_cross_chain = existing.iter().any(|e| e.chain != consumption.chain);

            let err = DoubleSpendError {
                seal_ref: consumption.seal_ref.clone(),
                existing_consumptions: existing.clone(),
                new_consumption: consumption.clone(),
                is_cross_chain,
            };

            // Still record for auditing purposes
            self.consumed_seals
                .entry(seal_key)
                .or_default()
                .push(consumption.clone());

            self.right_consumption_map
                .entry(consumption.right_id.0)
                .or_default()
                .push(consumption);

            return Err(Box::new(err));
        }

        // Record the consumption
        self.consumed_seals
            .entry(seal_key)
            .or_default()
            .push(consumption.clone());

        // Track by Right ID
        self.right_consumption_map
            .entry(consumption.right_id.0)
            .or_default()
            .push(consumption);

        Ok(())
    }

    /// Check the status of a seal.
    ///
    /// This method queries the registry to determine if a seal has been
    /// consumed and on which chain(s).
    ///
    /// # Security Requirements
    /// - MUST accurately report all recorded consumptions
    /// - MUST distinguish between `Unconsumed`, `ConsumedOnChain`, and `DoubleSpent`
    /// - MUST handle all seal reference types correctly
    ///
    /// # Returns
    /// - `SealStatus::Unconsumed` - Seal has never been recorded
    /// - `SealStatus::ConsumedOnChain` - Seal consumed exactly once
    /// - `SealStatus::DoubleSpent` - Seal consumed multiple times (attack detected)
    ///
    /// # Audit Note
    /// Verify this method correctly handles edge cases:
    /// - Empty seal references
    /// - Seals consumed on multiple different chains
    /// - Seals consumed multiple times on the same chain
    pub fn check_seal_status(&self, seal_ref: &SealRef) -> SealStatus {
        let key = seal_ref.to_vec();

        match self.consumed_seals.get(&key) {
            None => SealStatus::Unconsumed,
            Some(consumptions) if consumptions.len() == 1 => {
                let c = &consumptions[0];
                SealStatus::ConsumedOnChain {
                    chain: c.chain.clone(),
                    consumption: c.clone(),
                }
            }
            Some(consumptions) => SealStatus::DoubleSpent {
                consumptions: consumptions.clone(),
            },
        }
    }

    /// Check if a seal has been consumed (anywhere).
    pub fn is_seal_consumed(&self, seal_ref: &SealRef) -> bool {
        self.consumed_seals.contains_key(&seal_ref.to_vec())
    }

    /// Get all consumption events for a specific seal.
    pub fn get_consumption_history(&self, seal_ref: &SealRef) -> Vec<SealConsumption> {
        self.consumed_seals
            .get(&seal_ref.to_vec())
            .cloned()
            .unwrap_or_default()
    }

    /// Get all seals consumed by a specific Right.
    pub fn get_seals_for_right(&self, right_id: &RightId) -> Vec<SealConsumption> {
        self.right_consumption_map
            .get(&right_id.0)
            .cloned()
            .unwrap_or_default()
    }

    /// Get all known chains.
    pub fn known_chains(&self) -> Vec<&ChainId> {
        self.known_chains.iter().collect()
    }

    /// Get total number of unique seals tracked.
    pub fn total_seals(&self) -> usize {
        self.consumed_seals.len()
    }

    /// Get number of double-spend incidents detected.
    pub fn double_spend_count(&self) -> usize {
        self.consumed_seals
            .values()
            .filter(|consumptions| consumptions.len() > 1)
            .count()
    }
}

/// Error returned when a double-spend is detected.
#[derive(Debug, Clone)]
#[allow(missing_docs)]
pub struct DoubleSpendError {
    /// The seal that was double-spent
    pub seal_ref: SealRef,
    /// Existing consumption events
    pub existing_consumptions: Vec<SealConsumption>,
    /// The new consumption attempt
    pub new_consumption: SealConsumption,
    /// Whether this is a cross-chain double-spend
    pub is_cross_chain: bool,
}

impl core::fmt::Display for DoubleSpendError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.is_cross_chain {
            write!(
                f,
                "Cross-chain double-spend detected for seal {:?}",
                self.seal_ref
            )
        } else {
            write!(f, "Same-chain replay detected for seal {:?}", self.seal_ref)
        }
    }
}

use serde::{Deserialize, Serialize};

/// Optimized cross-chain seal registry with O(1) lookups using HashMap and Bloom filter.
///
/// This is a high-performance version of `CrossChainSealRegistry` that provides:
/// - O(1) seal existence checks via bloom filter for negative lookups
/// - O(1) exact lookups via HashMap for consumed seals
/// - Configurable capacity for pre-allocated storage
///
/// Use this when performance is critical and you need to handle large numbers of seals.
#[cfg(feature = "std")]
pub struct OptimizedCrossChainSealRegistry {
    /// Map from seal identity to consumption events (HashMap for O(1) lookups)
    consumed_seals: std::collections::HashMap<Vec<u8>, Vec<SealConsumption>>,
    /// Map from Right ID to seals that consumed it
    right_consumption_map: std::collections::HashMap<Hash, Vec<SealConsumption>>,
    /// Set of known chain identifiers
    known_chains: BTreeSet<ChainId>,
    /// Bloom filter for fast negative lookups
    bloom_filter: crate::performance::BloomFilter,
    /// Cache for recent seal status checks
    status_cache: std::collections::HashMap<Vec<u8>, SealStatus>,
    /// Maximum cache size for status cache
    max_cache_size: usize,
}

#[cfg(feature = "std")]
impl OptimizedCrossChainSealRegistry {
    /// Create a new optimized registry with default capacity.
    pub fn new() -> Self {
        Self::with_capacity(100_000, 0.01)
    }

    /// Create a new optimized registry with specified bloom filter capacity.
    ///
    /// # Arguments
    /// * `capacity` - Expected number of seals (for bloom filter sizing)
    /// * `false_positive_rate` - Acceptable false positive rate for bloom filter (0.0-1.0)
    pub fn with_capacity(capacity: usize, false_positive_rate: f64) -> Self {
        // Ensure valid bloom filter parameters
        let capacity = capacity.max(100); // Minimum 100 items
        let false_positive_rate = false_positive_rate.clamp(0.0001, 0.5); // Valid range

        Self {
            consumed_seals: std::collections::HashMap::with_capacity(capacity),
            right_consumption_map: std::collections::HashMap::with_capacity(capacity),
            known_chains: BTreeSet::new(),
            bloom_filter: crate::performance::BloomFilter::new(capacity, false_positive_rate),
            status_cache: std::collections::HashMap::with_capacity(1000),
            max_cache_size: 1000,
        }
    }

    /// Record a seal consumption event with optimized O(1) checks.
    pub fn record_consumption(
        &mut self,
        consumption: SealConsumption,
    ) -> Result<(), Box<DoubleSpendError>> {
        let seal_key = consumption.seal_ref.to_vec();
        let seal_hash = Hash::new(
            seal_key
                .as_slice()
                .try_into()
                .unwrap_or_else(|_| {
                    let mut arr = [0u8; 32];
                    let len = seal_key.len().min(32);
                    arr[..len].copy_from_slice(&seal_key[..len]);
                    arr
                }),
        );

        // Fast bloom filter check first (O(1))
        let might_exist = self.bloom_filter.might_contain(&seal_hash);

        // If bloom filter says no seal exists, we can skip HashMap lookup
        let is_double_spend = if might_exist {
            // Bloom filter says it might exist - need to check HashMap
            self.consumed_seals.contains_key(&seal_key)
                && !self
                    .consumed_seals
                    .get(&seal_key)
                    .map_or(true, |v| v.is_empty())
        } else {
            // Bloom filter says definitely not present - fast path
            false
        };

        // Track known chains
        self.known_chains.insert(consumption.chain.clone());

        // Add to bloom filter (always add, even on double-spend for auditing)
        self.bloom_filter.insert(&seal_hash);

        // Clear status cache entry if it exists
        self.status_cache.remove(&seal_key);

        // Check if already consumed
        if is_double_spend {
            let existing = self.consumed_seals.get(&seal_key).unwrap();
            let is_cross_chain = existing.iter().any(|e| e.chain != consumption.chain);

            let err = DoubleSpendError {
                seal_ref: consumption.seal_ref.clone(),
                existing_consumptions: existing.clone(),
                new_consumption: consumption.clone(),
                is_cross_chain,
            };

            // Still record for auditing purposes
            self.consumed_seals
                .entry(seal_key)
                .or_default()
                .push(consumption.clone());

            self.right_consumption_map
                .entry(consumption.right_id.0)
                .or_default()
                .push(consumption);

            return Err(Box::new(err));
        }

        // Record the consumption
        self.consumed_seals
            .entry(seal_key)
            .or_default()
            .push(consumption.clone());

        // Track by Right ID
        self.right_consumption_map
            .entry(consumption.right_id.0)
            .or_default()
            .push(consumption);

        Ok(())
    }

    /// Check the status of a seal with bloom filter optimization.
    pub fn check_seal_status(&mut self, seal_ref: &SealRef) -> SealStatus {
        let key = seal_ref.to_vec();

        // Check cache first (O(1))
        if let Some(cached) = self.status_cache.get(&key) {
            return cached.clone();
        }

        let seal_hash = Hash::new(
            key.as_slice()
                .try_into()
                .unwrap_or_else(|_| {
                    let mut arr = [0u8; 32];
                    let len = key.len().min(32);
                    arr[..len].copy_from_slice(&key[..len]);
                    arr
                }),
        );

        // Fast bloom filter check (O(1)) - if negative, seal is definitely not consumed
        if !self.bloom_filter.might_contain(&seal_hash) {
            let status = SealStatus::Unconsumed;
            self.cache_status(key, status.clone());
            return status;
        }

        // Bloom filter says it might exist - check HashMap (O(1))
        let status = match self.consumed_seals.get(&key) {
            None => SealStatus::Unconsumed,
            Some(consumptions) if consumptions.len() == 1 => {
                let c = &consumptions[0];
                SealStatus::ConsumedOnChain {
                    chain: c.chain.clone(),
                    consumption: c.clone(),
                }
            }
            Some(consumptions) => SealStatus::DoubleSpent {
                consumptions: consumptions.clone(),
            },
        };

        self.cache_status(key, status.clone());
        status
    }

    /// Immutable version of check_seal_status (no caching).
    pub fn check_seal_status_immutable(&self, seal_ref: &SealRef) -> SealStatus {
        let key = seal_ref.to_vec();
        let seal_hash = Hash::new(
            key.as_slice()
                .try_into()
                .unwrap_or_else(|_| {
                    let mut arr = [0u8; 32];
                    let len = key.len().min(32);
                    arr[..len].copy_from_slice(&key[..len]);
                    arr
                }),
        );

        // Fast bloom filter check (O(1))
        if !self.bloom_filter.might_contain(&seal_hash) {
            return SealStatus::Unconsumed;
        }

        // Check HashMap
        match self.consumed_seals.get(&key) {
            None => SealStatus::Unconsumed,
            Some(consumptions) if consumptions.len() == 1 => {
                let c = &consumptions[0];
                SealStatus::ConsumedOnChain {
                    chain: c.chain.clone(),
                    consumption: c.clone(),
                }
            }
            Some(consumptions) => SealStatus::DoubleSpent {
                consumptions: consumptions.clone(),
            },
        }
    }

    /// Cache a status result.
    fn cache_status(&mut self, key: Vec<u8>, status: SealStatus) {
        if self.status_cache.len() >= self.max_cache_size {
            // Simple eviction: clear half the cache
            let keys_to_remove: Vec<_> = self.status_cache.keys().take(self.max_cache_size / 2).cloned().collect();
            for k in keys_to_remove {
                self.status_cache.remove(&k);
            }
        }
        self.status_cache.insert(key, status);
    }

    /// Check if a seal has been consumed (anywhere) with O(1) bloom filter check.
    pub fn is_seal_consumed(&self, seal_ref: &SealRef) -> bool {
        let seal_key = seal_ref.to_vec();
        let seal_hash = Hash::new(
            seal_key
                .as_slice()
                .try_into()
                .unwrap_or_else(|_| {
                    let mut arr = [0u8; 32];
                    let len = seal_key.len().min(32);
                    arr[..len].copy_from_slice(&seal_key[..len]);
                    arr
                }),
        );

        // Fast bloom filter check first (O(1))
        if !self.bloom_filter.might_contain(&seal_hash) {
            return false;
        }

        // Bloom filter says might exist - confirm with HashMap
        self.consumed_seals.contains_key(&seal_key)
    }

    /// Get all consumption events for a specific seal.
    pub fn get_consumption_history(&self, seal_ref: &SealRef) -> Vec<SealConsumption> {
        let key = seal_ref.to_vec();
        self.consumed_seals
            .get(&key)
            .cloned()
            .unwrap_or_default()
    }

    /// Get all seals consumed by a specific Right.
    pub fn get_seals_for_right(&self, right_id: &RightId) -> Vec<SealConsumption> {
        self.right_consumption_map
            .get(&right_id.0)
            .cloned()
            .unwrap_or_default()
    }

    /// Get all known chains.
    pub fn known_chains(&self) -> Vec<&ChainId> {
        self.known_chains.iter().collect()
    }

    /// Get total number of unique seals tracked.
    pub fn total_seals(&self) -> usize {
        self.consumed_seals.len()
    }

    /// Get number of double-spend incidents detected.
    pub fn double_spend_count(&self) -> usize {
        self.consumed_seals
            .values()
            .filter(|consumptions| consumptions.len() > 1)
            .count()
    }

    /// Get bloom filter statistics.
    pub fn bloom_stats(&self) -> crate::performance::FilterStats {
        self.bloom_filter.stats()
    }

    /// Clear the status cache.
    pub fn clear_cache(&mut self) {
        self.status_cache.clear();
    }

    /// Pre-populate the bloom filter from existing seals.
    pub fn rebuild_bloom_filter(&mut self) {
        let capacity = self.consumed_seals.len().max(1000);
        let mut new_filter = crate::performance::BloomFilter::new(capacity, 0.01);

        for key in self.consumed_seals.keys() {
            let seal_hash = Hash::new(
                key.as_slice()
                    .try_into()
                    .unwrap_or_else(|_| {
                        let mut arr = [0u8; 32];
                        let len = key.len().min(32);
                        arr[..len].copy_from_slice(&key[..len]);
                        arr
                    }),
            );
            new_filter.insert(&seal_hash);
        }

        self.bloom_filter = new_filter;
    }
}

#[cfg(feature = "std")]
impl Default for OptimizedCrossChainSealRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_consumption(chain: ChainId, seal_bytes: Vec<u8>, right_id: RightId) -> SealConsumption {
        SealConsumption {
            chain,
            seal_ref: SealRef::new(seal_bytes, None).unwrap(),
            right_id,
            block_height: 100,
            tx_hash: Hash::new([0xAB; 32]),
            recorded_at: 1_000_000,
        }
    }

    #[test]
    fn test_record_single_consumption() {
        let mut registry = CrossChainSealRegistry::new();
        let right_id = RightId(Hash::new([0xCD; 32]));
        let consumption = make_consumption(ChainId::Bitcoin, vec![0x01], right_id);

        assert!(registry.record_consumption(consumption).is_ok());
        assert_eq!(registry.total_seals(), 1);
        assert_eq!(registry.double_spend_count(), 0);
    }

    #[test]
    fn test_detect_same_chain_replay() {
        let mut registry = CrossChainSealRegistry::new();
        let right_id = RightId(Hash::new([0xCD; 32]));
        let seal_bytes = vec![0x01];

        let consumption1 = make_consumption(ChainId::Bitcoin, seal_bytes.clone(), right_id);
        registry.record_consumption(consumption1).unwrap();

        // Try to consume the same seal again on Bitcoin
        let right_id2 = RightId(Hash::new([0xEF; 32]));
        let consumption2 = make_consumption(ChainId::Bitcoin, seal_bytes, right_id2);
        let result = registry.record_consumption(consumption2);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(!err.is_cross_chain);
    }

    #[test]
    fn test_detect_cross_chain_double_spend() {
        let mut registry = CrossChainSealRegistry::new();
        let right_id = RightId(Hash::new([0xCD; 32]));
        let seal_bytes = vec![0x01];

        // Consume on Bitcoin
        let consumption1 = make_consumption(ChainId::Bitcoin, seal_bytes.clone(), right_id.clone());
        registry.record_consumption(consumption1).unwrap();

        // Try to consume on Ethereum (cross-chain double-spend)
        let consumption2 = make_consumption(ChainId::Ethereum, seal_bytes, right_id);
        let result = registry.record_consumption(consumption2);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.is_cross_chain);
        assert_eq!(err.existing_consumptions.len(), 1);
    }

    #[test]
    fn test_seal_status_unconsumed() {
        let registry = CrossChainSealRegistry::new();
        let seal = SealRef::new(vec![0x01], None).unwrap();

        assert!(matches!(
            registry.check_seal_status(&seal),
            SealStatus::Unconsumed
        ));
    }

    #[test]
    fn test_seal_status_consumed() {
        let mut registry = CrossChainSealRegistry::new();
        let right_id = RightId(Hash::new([0xCD; 32]));
        let seal = SealRef::new(vec![0x01], None).unwrap();

        let consumption = make_consumption(ChainId::Bitcoin, vec![0x01], right_id);
        registry.record_consumption(consumption).unwrap();

        match registry.check_seal_status(&seal) {
            SealStatus::ConsumedOnChain { chain, .. } => {
                assert_eq!(chain, ChainId::Bitcoin);
            }
            _ => panic!("Expected ConsumedOnChain"),
        }
    }

    #[test]
    fn test_seal_status_double_spent() {
        let mut registry = CrossChainSealRegistry::new();
        let right_id = RightId(Hash::new([0xCD; 32]));
        let seal = SealRef::new(vec![0x01], None).unwrap();
        let seal_bytes = vec![0x01];

        // Consume on Bitcoin
        let c1 = make_consumption(ChainId::Bitcoin, seal_bytes.clone(), right_id.clone());
        registry.record_consumption(c1).unwrap();

        // Try to consume on Ethereum (will be recorded in history but flagged as double-spend)
        let c2 = make_consumption(ChainId::Ethereum, seal_bytes, right_id.clone());

        // Note: record_consumption returns error, but we can still check status
        let _ = registry.record_consumption(c2);

        assert!(matches!(
            registry.check_seal_status(&seal),
            SealStatus::DoubleSpent { .. }
        ));
    }

    #[test]
    fn test_known_chains() {
        let mut registry = CrossChainSealRegistry::new();
        assert_eq!(registry.known_chains().len(), 0);

        let right_id = RightId(Hash::new([0xCD; 32]));
        let c1 = make_consumption(ChainId::Bitcoin, vec![0x01], right_id.clone());
        registry.record_consumption(c1).unwrap();

        assert_eq!(registry.known_chains().len(), 1);
    }

    #[test]
    #[cfg(feature = "std")]
    fn test_optimized_registry_basic() {
        let mut registry = OptimizedCrossChainSealRegistry::new();
        let right_id = RightId(Hash::new([0xCD; 32]));
        let consumption = make_consumption(ChainId::Bitcoin, vec![0x01], right_id);

        assert!(registry.record_consumption(consumption).is_ok());
        assert_eq!(registry.total_seals(), 1);
        assert_eq!(registry.double_spend_count(), 0);

        // Check bloom filter stats
        let stats = registry.bloom_stats();
        assert!(stats.bit_count > 0);
        assert!(stats.hash_count > 0);
    }

    #[test]
    #[cfg(feature = "std")]
    fn test_optimized_registry_bloom_filter_negative_lookup() {
        let mut registry = OptimizedCrossChainSealRegistry::new();
        let seal = SealRef::new(vec![0x99], None).unwrap();

        // Check unconsumed seal - should be O(1) via bloom filter
        let status = registry.check_seal_status(&seal);
        assert!(matches!(status, SealStatus::Unconsumed));

        // Bloom filter should have been checked
        let stats = registry.bloom_stats();
        assert!(stats.bit_count > 0);
    }

    #[test]
    #[cfg(feature = "std")]
    fn test_optimized_registry_double_spend_detection() {
        let mut registry = OptimizedCrossChainSealRegistry::new();
        let right_id = RightId(Hash::new([0xCD; 32]));
        let seal_bytes = vec![0x01];

        // Consume on Bitcoin
        let c1 = make_consumption(ChainId::Bitcoin, seal_bytes.clone(), right_id.clone());
        registry.record_consumption(c1).unwrap();

        // Try same-chain double spend
        let c2 = make_consumption(ChainId::Bitcoin, seal_bytes.clone(), right_id.clone());
        let result = registry.record_consumption(c2);
        assert!(result.is_err());

        // Try cross-chain double spend
        let c3 = make_consumption(ChainId::Ethereum, seal_bytes, right_id);
        let result = registry.record_consumption(c3);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.is_cross_chain);
    }

    #[test]
    #[cfg(feature = "std")]
    fn test_optimized_registry_status_caching() {
        let mut registry = OptimizedCrossChainSealRegistry::new();
        let right_id = RightId(Hash::new([0xCD; 32]));
        let seal = SealRef::new(vec![0x01], None).unwrap();

        // First check (cache miss)
        let _ = registry.check_seal_status(&seal);

        // Record consumption
        let c1 = make_consumption(ChainId::Bitcoin, vec![0x01], right_id);
        registry.record_consumption(c1).unwrap();

        // Second check (should hit cache now)
        let _ = registry.check_seal_status(&seal);

        // Clear cache and verify
        registry.clear_cache();
    }

    #[test]
    #[cfg(feature = "std")]
    fn test_optimized_registry_is_seal_consumed() {
        let mut registry = OptimizedCrossChainSealRegistry::new();
        let right_id = RightId(Hash::new([0xCD; 32]));
        let seal = SealRef::new(vec![0x01], None).unwrap();

        // Not consumed yet - bloom filter should give O(1) negative result
        assert!(!registry.is_seal_consumed(&seal));

        // Consume it
        let c1 = make_consumption(ChainId::Bitcoin, vec![0x01], right_id);
        registry.record_consumption(c1).unwrap();

        // Now consumed
        assert!(registry.is_seal_consumed(&seal));
    }

    #[test]
    #[cfg(feature = "std")]
    fn test_optimized_registry_rebuild_bloom_filter() {
        let mut registry = OptimizedCrossChainSealRegistry::new();

        // Add some seals
        for i in 0..100u8 {
            let right_id = RightId(Hash::new([i; 32]));
            let c = make_consumption(ChainId::Bitcoin, vec![i], right_id);
            registry.record_consumption(c).unwrap();
        }

        // Rebuild bloom filter
        registry.rebuild_bloom_filter();

        // Verify all seals still detectable
        for i in 0..100u8 {
            let seal = SealRef::new(vec![i], None).unwrap();
            assert!(registry.is_seal_consumed(&seal), "Seal {} should be consumed", i);
        }

        // Verify unconsumed seal still returns false
        let unconsumed = SealRef::new(vec![0xFF], None).unwrap();
        assert!(!registry.is_seal_consumed(&unconsumed));
    }
}
