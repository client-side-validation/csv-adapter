//! Cross-Chain Seal Registry
//!
//! Tracks consumed seals across all chains to detect double-consumption
//! attempts, including cross-chain double-spends.
//!
//! ## Purpose
//!
//! When a Right is consumed, the seal that enforced it is recorded here.
//! This allows the client to detect if the same seal (or equivalent seal
//! on another chain) has been used more than once.
//!
//! ## Chain-Specific Seal Types
//!
//! | Chain | Seal Type | Identifier |
//! |-------|-----------|------------|
//! | Bitcoin | UTXO spend | txid:vout |
//! | Sui | Object deletion | object_id:version |
//! | Aptos | Resource destruction | resource_address:account |
//! | Ethereum | Nullifier registration | nullifier_hash |
//!
//! ## Cross-Chain Detection
//!
//! The registry maps all seal types to a unified `SealIdentity` that can
//! be compared across chains. This detects:
//! - Same seal used twice on the same chain
//! - Equivalent seals used on different chains (cross-chain double-spend)

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::String;
use alloc::vec::Vec;

use crate::hash::Hash;
use crate::right::RightId;
use crate::seal::SealRef;

/// The chain that enforces this seal's single-use.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ChainId {
    Bitcoin,
    Sui,
    Aptos,
    Ethereum,
    Custom(String),
}

/// A seal consumption event.
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
pub enum SealStatus {
    /// Seal has not been consumed
    Unconsumed,
    /// Seal was consumed on this chain
    ConsumedOnChain {
        chain: ChainId,
        consumption: SealConsumption,
    },
    /// Seal was consumed on multiple chains (double-spend!)
    DoubleSpent { consumptions: Vec<SealConsumption> },
}

/// Cross-chain seal registry.
///
/// Tracks all consumed seals across all chains and provides
/// double-consumption detection.
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
    /// # Returns
    /// - `Ok(())` if this is the first consumption of this seal
    /// - `Err(double_spend)` if the seal was already consumed
    ///   (but the consumption is still recorded for auditing)
    pub fn record_consumption(
        &mut self,
        consumption: SealConsumption,
    ) -> Result<(), DoubleSpendError> {
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
                .or_insert_with(Vec::new)
                .push(consumption.clone());

            self.right_consumption_map
                .entry(consumption.right_id.0)
                .or_insert_with(Vec::new)
                .push(consumption);

            return Err(err);
        }

        // Record the consumption
        self.consumed_seals
            .entry(seal_key)
            .or_insert_with(Vec::new)
            .push(consumption.clone());

        // Track by Right ID
        self.right_consumption_map
            .entry(consumption.right_id.0)
            .or_insert_with(Vec::new)
            .push(consumption);

        Ok(())
    }

    /// Check the status of a seal.
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
}
