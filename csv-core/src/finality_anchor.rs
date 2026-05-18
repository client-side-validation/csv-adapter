//! Restart-safe finality anchoring — canonical chain snapshot persistence.
//!
//! On runtime restart, the latest anchor is loaded and compared against the
//! chain's current state. If ancestor continuity is broken (silent historical
//! reorg), a force rollback is triggered before any new transfer proceeds.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::hash::Hash;
use crate::protocol_version::ChainId;

/// A persisted snapshot of a chain's finalized state.
///
/// This anchor is the starting point for restart-safety checks. The runtime
/// persists one anchor per chain after each finality progression, and on
/// restart loads the latest anchor and verifies ancestor continuity against
/// the current chain tip.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FinalityAnchor {
    /// Chain this anchor applies to.
    pub chain: ChainId,
    /// The finalized block/checkpoint height.
    pub finalized_height: u64,
    /// The finalized block/checkpoint hash.
    pub finalized_hash: Hash,
    /// Cumulative work (Bitcoin/Proof-of-work chains). `None` for DPoS/BFT.
    pub cumulative_work: Option<u128>,
    /// When this anchor was persisted.
    pub finalized_at: DateTime<Utc>,
}

/// Every finalized ancestor hash for at least `max_safe_reorg_depth * 4` blocks.
///
/// This provides the historical chain continuity proof needed to detect
/// long-range reorgs that occurred while the runtime was offline.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AncestorContinuityProof {
    /// Chain this proof applies to.
    pub chain: ChainId,
    /// The anchor at the time this proof was generated.
    pub anchor: FinalityAnchor,
    /// Ancestor hashes in descending height order (tip → genesis).
    pub ancestor_hashes: Vec<(u64, Hash)>,
    /// Minimum depth of ancestor hashes stored.
    pub min_depth: u64,
}

impl FinalityAnchor {
    /// Create a new finality anchor.
    pub fn new(
        chain: ChainId,
        finalized_height: u64,
        finalized_hash: Hash,
        cumulative_work: Option<u128>,
    ) -> Self {
        Self {
            chain,
            finalized_height,
            finalized_hash,
            cumulative_work,
            finalized_at: Utc::now(),
        }
    }

    /// Verify that a new anchor is a valid successor (same chain, higher height).
    pub fn is_valid_successor(&self, new: &FinalityAnchor) -> bool {
        self.chain == new.chain
            && new.finalized_height > self.finalized_height
            && new.finalized_hash != Hash::zero()
    }
}

impl AncestorContinuityProof {
    /// The first ancestor hash at or below the reorg safety threshold.
    pub fn safety_hash(&self, max_safe_reorg_depth: u64) -> Option<(u64, Hash)> {
        let threshold = self
            .anchor
            .finalized_height
            .saturating_sub(max_safe_reorg_depth);
        self.ancestor_hashes
            .iter()
            .find(|(h, _)| *h <= threshold)
            .copied()
    }
}