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

/// Weight assigned to an RPC provider for quorum voting.
///
/// Higher weight means the provider's opinion counts more toward consensus.
/// Weights are normalized during quorum computation.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProviderWeight {
    raw: f64,
}

impl ProviderWeight {
    /// Create a new provider weight from a raw value.
    ///
    /// # Errors
    /// Returns `QuorumError::InvalidProviderWeight` if `raw` is zero, negative, NaN, or infinite.
    pub fn new(raw: f64) -> Result<Self, QuorumError> {
        if raw <= 0.0 || raw.is_nan() || raw.is_infinite() {
            return Err(QuorumError::InvalidProviderWeight(raw));
        }
        Ok(Self { raw })
    }

    /// Get the raw weight value.
    pub fn raw(self) -> f64 {
        self.raw
    }

    /// Create a zero weight (used for suspended providers).
    pub fn zero() -> Self {
        Self { raw: 0.0 }
    }
}

impl Default for ProviderWeight {
    fn default() -> Self {
        Self { raw: 1.0 }
    }
}

/// Quorum threshold configuration.
///
/// Defines the minimum agreement required for consensus.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QuorumThreshold {
    /// Minimum fraction of total weight that must agree (0.0 - 1.0).
    pub min_agreement_ratio: f64,
    /// Minimum number of distinct providers that must agree.
    pub min_provider_count: usize,
    /// Maximum allowed divergence between top groups (0.0 - 1.0).
    /// If the ratio of second-best to best weight exceeds this, consensus fails.
    pub max_divergence_ratio: f64,
}

impl QuorumThreshold {
    /// Create a new quorum threshold configuration.
    ///
    /// # Errors
    /// Returns `QuorumError::InvalidThreshold` if any parameter is out of valid range.
    pub fn new(
        min_agreement_ratio: f64,
        min_provider_count: usize,
        max_divergence_ratio: f64,
    ) -> Result<Self, QuorumError> {
        if !(0.0..=1.0).contains(&min_agreement_ratio) {
            return Err(QuorumError::InvalidThreshold("min_agreement_ratio must be 0.0-1.0".into()));
        }
        if min_agreement_ratio < 0.5 {
            return Err(QuorumError::InvalidThreshold(
                "min_agreement_ratio must be >= 0.5".into(),
            ));
        }
        if min_provider_count < 1 {
            return Err(QuorumError::InvalidThreshold(
                "min_provider_count must be >= 1".into(),
            ));
        }
        if !(0.0..=1.0).contains(&max_divergence_ratio) {
            return Err(QuorumError::InvalidThreshold(
                "max_divergence_ratio must be 0.0-1.0".into(),
            ));
        }
        Ok(Self {
            min_agreement_ratio,
            min_provider_count,
            max_divergence_ratio,
        })
    }

    /// Standard Byzantine fault tolerant threshold: (2f+1)/3f+1 where f=1 → 2/3.
    pub fn bft_strict() -> Self {
        Self {
            min_agreement_ratio: 0.667,
            min_provider_count: 2,
            max_divergence_ratio: 0.5,
        }
    }

    /// Relaxed threshold for degraded networks: simple majority.
    pub fn majority() -> Self {
        Self {
            min_agreement_ratio: 0.51,
            min_provider_count: 2,
            max_divergence_ratio: 0.8,
        }
    }
}

impl Default for QuorumThreshold {
    fn default() -> Self {
        Self::bft_strict()
    }
}

/// Byzantine fault tolerance parameters.
///
/// In a BFT system with f faulty providers, we need at least 3f+1 total
/// providers to reach consensus.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ByzantineConfig {
    /// Maximum number of faulty/malicious providers tolerated.
    pub max_faulty: usize,
    /// Total providers expected.
    pub total_providers: usize,
}

impl ByzantineConfig {
    /// Create a new Byzantine fault tolerance configuration.
    ///
    /// # Errors
    /// Returns `QuorumError::InvalidByzantineConfig` if the configuration
    /// does not satisfy the BFT requirement of `total_providers >= 3*max_faulty + 1`.
    pub fn new(max_faulty: usize, total_providers: usize) -> Result<Self, QuorumError> {
        if total_providers < 1 {
            return Err(QuorumError::InvalidByzantineConfig(
                "total_providers must be >= 1".into(),
            ));
        }
        if max_faulty >= total_providers {
            return Err(QuorumError::InvalidByzantineConfig(
                "max_faulty must be < total_providers".into(),
            ));
        }
        if total_providers < 3 * max_faulty + 1 {
            return Err(QuorumError::InvalidByzantineConfig(format!(
                "total_providers ({}) must be >= 3*max_faulty+1 ({})",
                total_providers,
                3 * max_faulty + 1
            )));
        }
        Ok(Self {
            max_faulty,
            total_providers,
        })
    }

    /// Minimum quorum size for BFT: 2f+1.
    pub fn min_quorum_size(&self) -> usize {
        2 * self.max_faulty + 1
    }

    /// Check if the given number of agreeing providers satisfies BFT.
    pub fn satisfies_bft(&self, agreeing: usize) -> bool {
        agreeing >= self.min_quorum_size()
    }
}

/// Health score for an RPC provider.
///
/// Tracks provider reliability over time for dynamic weight adjustment.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProviderHealth {
    /// Endpoint URL.
    pub endpoint: String,
    /// Success rate (0.0 - 1.0).
    pub success_rate: f64,
    /// Average latency in milliseconds.
    pub avg_latency_ms: f64,
    /// Number of consecutive failures.
    pub consecutive_failures: u32,
    /// Last successful observation timestamp.
    pub last_success: Option<DateTime<Utc>>,
    /// Whether this provider is temporarily suspended.
    pub suspended: bool,
}

impl ProviderHealth {
    /// Create a new provider health tracker for the given endpoint.
    pub fn new(endpoint: String) -> Self {
        Self {
            endpoint,
            success_rate: 1.0,
            avg_latency_ms: 0.0,
            consecutive_failures: 0,
            last_success: None,
            suspended: false,
        }
    }

    /// Record a successful RPC response with the given latency in milliseconds.
    pub fn record_success(&mut self, latency_ms: u64) {
        self.consecutive_failures = 0;
        self.last_success = Some(Utc::now());
        // Exponential moving average with alpha=0.1
        self.avg_latency_ms = self.avg_latency_ms * 0.9 + (latency_ms as f64) * 0.1;
        // Update success rate with exponential moving average
        self.success_rate = self.success_rate * 0.9 + 1.0 * 0.1;
    }

    /// Record an RPC failure, updating consecutive failure count and suspension state.
    pub fn record_failure(&mut self) {
        self.consecutive_failures += 1;
        // Update success rate with exponential moving average
        self.success_rate = self.success_rate * 0.9 + 0.0 * 0.1;
        // Suspend after 5 consecutive failures
        if self.consecutive_failures >= 5 {
            self.suspended = true;
        }
    }

    /// Compute a composite health score in [0.0, 1.0] based on success rate,
    /// latency, and suspension state.
   pub fn health_score(&self) -> f64 {
        // Compute a composite health score in [0.0, 1.0] based on success rate,
        // latency, and suspension state.
        let mut score = self.success_rate * 0.5;
        // Latency bonus: lower latency = higher score
        let latency_score = if self.avg_latency_ms > 0.0 {
            1.0 / (1.0 + self.avg_latency_ms / 1000.0)
        } else {
            1.0
        };
        score += latency_score * 0.5;
        // Suspension penalty
        if self.suspended {
            score *= 0.1;
        }
        score.min(1.0)
    }
}

/// A quorum consensus round.
///
/// Tracks the state of a single quorum decision attempt.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsensusRound {
    /// Unique round identifier.
    pub round_id: u64,
    /// Chain being observed.
    pub chain_id: String,
    /// Observations collected in this round.
    pub observations: Vec<RpcObservation>,
    /// Provider health states at round start.
    pub provider_health: alloc::collections::BTreeMap<String, ProviderHealth>,
    /// Computed decision (if any).
    pub decision: Option<QuorumResult>,
}

impl ConsensusRound {
    /// Create a new consensus round with the given ID and chain identifier.
    pub fn new(round_id: u64, chain_id: String) -> Self {
        Self {
            round_id,
            chain_id,
            observations: Vec::new(),
            provider_health: alloc::collections::BTreeMap::new(),
            decision: None,
        }
    }

    /// Add an observation to this round.
    pub fn add_observation(&mut self, obs: RpcObservation) {
        self.observations.push(obs);
    }

    /// Add provider health state to this round.
    pub fn add_health(&mut self, health: ProviderHealth) {
        self.provider_health
            .insert(health.endpoint.clone(), health);
    }
}

/// Result of a quorum consensus decision.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum QuorumResult {
    /// Consensus reached with the agreed data and supporting providers.
    Consensus {
        /// The canonical block hash agreed upon by the quorum.
        canonical_hash: Hash,
        /// The canonical block height.
        canonical_height: u64,
        /// Endpoints of providers supporting this consensus.
        supporting_providers: Vec<String>,
        /// Combined weight of supporting providers.
        total_weight: f64,
        /// Confidence score in [0.0, 1.0].
        confidence: f64,
        /// Finality grade assigned to this consensus.
        finality_grade: FinalityGrade,
    },
    /// No consensus reached; includes diagnostic info.
    NoConsensus {
        /// Groups of providers that failed to reach agreement.
        groups: Vec<QuorumGroup>,
        /// Total weight across all groups.
        total_weight: f64,
        /// Reason consensus was not reached.
        reason: String,
    },
    /// Byzantine behavior detected in one or more providers.
    ByzantineDetected {
        /// Endpoints of suspicious providers.
        suspicious_providers: Vec<String>,
        /// Reason Byzantine behavior was detected.
        reason: String,
    },
}

/// A group of providers agreeing on the same observation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QuorumGroup {
    /// The hash agreed upon.
    pub hash: Hash,
    /// Total weight of agreeing providers.
    pub weight: f64,
    /// Number of providers in the group.
    pub provider_count: usize,
    /// Provider endpoints.
    pub providers: Vec<String>,
}

/// Errors that can occur during quorum computation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum QuorumError {
    /// Invalid provider weight.
    InvalidProviderWeight(f64),
    /// Invalid threshold configuration.
    InvalidThreshold(String),
    /// Invalid Byzantine configuration.
    InvalidByzantineConfig(String),
    /// Not enough providers to form a quorum.
    InsufficientProviders {
        /// Number of providers required.
        required: usize,
        /// Number of providers available.
        available: usize,
    },
    /// Quorum threshold not met.
    ThresholdNotMet {
        /// Required threshold value.
        required: f64,
        /// Achieved value.
        achieved: f64,
    },
    /// Byzantine behavior detected.
    ByzantineBehavior {
        /// Endpoint of the suspicious provider.
        provider: String,
        /// Reason Byzantine behavior was detected.
        reason: String,
    },
    /// Round already has a decision.
    RoundComplete(u64),
}

impl core::fmt::Display for QuorumError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidProviderWeight(w) => write!(f, "Invalid provider weight: {}", w),
            Self::InvalidThreshold(msg) => write!(f, "Invalid threshold: {}", msg),
            Self::InvalidByzantineConfig(msg) => write!(f, "Invalid Byzantine config: {}", msg),
            Self::InsufficientProviders { required, available } => {
                write!(f, "Insufficient providers: need {}, have {}", required, available)
            }
            Self::ThresholdNotMet { required, achieved } => {
                write!(
                    f,
                    "Quorum threshold not met: required={}, achieved={:.3}",
                    required, achieved
                )
            }
            Self::ByzantineBehavior { provider, reason } => {
                write!(f, "Byzantine behavior from {}: {}", provider, reason)
            }
            Self::RoundComplete(id) => write!(f, "Round {} already has a decision", id),
        }
    }
}

/// Finality grade for quorum decisions.
///
/// Represents the finality strength determined by quorum consensus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuorumFinality {
    /// No finality information available.
    Unknown,
    /// Probabilistic finality with confirmation count.
    Probabilistic {
        /// Number of block confirmations.
        confirmations: u64,
    },
    /// BFT finality confirmed by quorum.
    BftFinal,
    /// Finality confirmed but below BFT threshold.
    WeakFinal,
}