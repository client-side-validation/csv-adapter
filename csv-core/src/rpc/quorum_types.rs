//! RPC quorum observation and decision types.
//!
//! These types replace raw "RPC fallback" logic with a structured quorum
//! consensus model. The runtime MUST NOT fallback to fastest-node, first-node,
//! or majority-by-height. Quorum confidence below threshold must freeze
//! transfer progression.

use alloc::string::String;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::hash::Hash;

/// An RPC observation captured from a single provider.
///
/// The quorum engine aggregates observations from multiple providers and
/// produces a consensus decision. Each observation carries the metadata
/// needed to compute weighted agreement.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RpcObservation {
    /// The RPC endpoint URL.
    pub endpoint: String,
    /// Chain identifier being observed.
    pub chain_id: String,
    /// Observed block/ledger height.
    pub observed_height: u64,
    /// Observed block/ledger hash.
    pub observed_hash: Hash,
    /// Observed finality strength for this observation.
    pub observed_finality: super::FinalityGrade,
    /// Latency in milliseconds for this observation.
    pub latency_ms: u64,
    /// Timestamp of the observation.
    pub timestamp: DateTime<Utc>,
}

/// Finality grade reported by an RPC observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FinalityGrade {
    /// No finality information available.
    None,
    /// Probabilistic with the number of confirmation blocks.
    Probabilistic {
        /// Number of confirmation blocks observed.
        confirmations: u64,
    },
    /// Deterministic/BFT finality.
    Deterministic,
    /// Optimistic confirmation.
    Optimistic,
}

/// A quorum decision produced after aggregating observations.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QuorumDecision {
    /// The canonical hash determined by quorum consensus.
    pub canonical_hash: Hash,
    /// Number of providers that agree on the canonical hash.
    pub agreeing_nodes: usize,
    /// Number of providers that disagree.
    pub disagreeing_nodes: usize,
    /// Confidence ratio (0.0 - 1.0).
    pub confidence: f64,
}

impl RpcObservation {
    /// Create a new RPC observation.
    pub fn new(
        endpoint: String,
        chain_id: String,
        observed_height: u64,
        observed_hash: Hash,
        observed_finality: FinalityGrade,
        latency_ms: u64,
    ) -> Self {
        Self {
            endpoint,
            chain_id,
            observed_height,
            observed_hash,
            observed_finality,
            latency_ms,
            timestamp: Utc::now(),
        }
    }
}