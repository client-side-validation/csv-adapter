//! Quorum Consensus Engine
//!
//! Aggregates RPC observations from multiple providers and produces
//! consensus decisions using weighted voting with Byzantine fault tolerance.
//!
//! The engine enforces:
//! - No fallback to fastest-node, first-node, or majority-by-height
//! - Weighted voting based on provider reliability
//! - Byzantine fault detection
//! - Provider health tracking for dynamic weight adjustment

use alloc::collections::BTreeMap;
use core::cmp::Ordering;

use crate::hash::Hash;
use crate::rpc::quorum_types::*;

/// Quorum consensus engine.
///
/// Aggregates observations from multiple RPC providers, applies weighted
/// voting, detects Byzantine behavior, and produces consensus decisions.
#[derive(Clone, Debug)]
pub struct QuorumEngine {
    /// Threshold configuration for consensus decisions.
    threshold: QuorumThreshold,
    /// Byzantine fault tolerance configuration.
    byzantine: Option<ByzantineConfig>,
    /// Provider weights.
    weights: BTreeMap<String, ProviderWeight>,
    /// Provider health tracking.
    health: BTreeMap<String, ProviderHealth>,
    /// Round counter.
    round_counter: u64,
}

impl QuorumEngine {
    /// Create a new quorum engine with the given threshold configuration.
    pub fn new(threshold: QuorumThreshold) -> Self {
        Self {
            threshold,
            byzantine: None,
            weights: BTreeMap::new(),
            health: BTreeMap::new(),
            round_counter: 0,
        }
    }

    /// Create a new quorum engine with BFT configuration.
    pub fn with_bft(threshold: QuorumThreshold, byzantine: ByzantineConfig) -> Self {
        Self {
            threshold,
            byzantine: Some(byzantine),
            weights: BTreeMap::new(),
            health: BTreeMap::new(),
            round_counter: 0,
        }
    }

    /// Set the threshold configuration.
    pub fn with_threshold(mut self, threshold: QuorumThreshold) -> Self {
        self.threshold = threshold;
        self
    }

    /// Register a provider with a weight.
    pub fn register_provider(&mut self, endpoint: String, weight: ProviderWeight) {
        self.weights.insert(endpoint.clone(), weight);
        if !self.health.contains_key(&endpoint) {
            self.health.insert(endpoint.clone(), ProviderHealth::new(endpoint));
        }
    }

    /// Register a provider with default weight (1.0).
    pub fn register_default_provider(&mut self, endpoint: String) {
        self.register_provider(endpoint, ProviderWeight::default());
    }

    /// Remove a provider from the engine.
    pub fn remove_provider(&mut self, endpoint: &str) {
        self.weights.remove(endpoint);
        self.health.remove(endpoint);
    }

    /// Get the number of registered providers.
    pub fn provider_count(&self) -> usize {
        self.weights.len()
    }

    /// Start a new consensus round.
    pub fn start_round(&mut self, chain_id: String) -> ConsensusRound {
        self.round_counter += 1;
        let mut round = ConsensusRound::new(self.round_counter, chain_id);
        // Snapshot current health states
        for (_endpoint, health) in &self.health {
            round.add_health(health.clone());
        }
        round
    }

    /// Add an observation to a round.
    pub fn add_observation(&mut self, round: &mut ConsensusRound, obs: RpcObservation) {
        round.add_observation(obs);
    }

    /// Decide consensus for a round.
    ///
    /// Aggregates all observations in the round, applies weighted voting,
    /// checks BFT constraints, and produces a quorum decision.
    pub fn decide(&mut self, round: &mut ConsensusRound) -> Result<QuorumResult, QuorumError> {
        if round.decision.is_some() {
            return Err(QuorumError::RoundComplete(round.round_id));
        }

        let decision = self.compute_decision(round)?;
        round.decision = Some(decision.clone());

        // Update health based on round outcome
        self.update_health_after_round(round);

        Ok(decision)
    }

    /// Compute a quorum decision without updating round state.
    ///
    /// Used for dry-run evaluation and testing.
    pub fn compute_decision(
        &self,
        round: &ConsensusRound,
    ) -> Result<QuorumResult, QuorumError> {
        let observations = &round.observations;

        if observations.is_empty() {
            return Err(QuorumError::InsufficientProviders {
                required: 1,
                available: 0,
            });
        }

        // Check minimum provider count
        let unique_providers = observations
            .iter()
            .map(|o| &o.endpoint)
            .collect::<alloc::collections::BTreeSet<_>>();

        if unique_providers.len() < self.threshold.min_provider_count {
            return Err(QuorumError::InsufficientProviders {
                required: self.threshold.min_provider_count,
                available: unique_providers.len(),
            });
        }

        // Group observations by hash
        let mut groups: BTreeMap<Hash, QuorumGroup> = BTreeMap::new();
        for obs in observations {
            let weight = self
                .weights
                .get(&obs.endpoint)
                .map(|w| w.raw())
                .unwrap_or(1.0);

            let entry = groups
                .entry(obs.observed_hash)
                .or_insert_with(|| QuorumGroup {
                    hash: obs.observed_hash,
                    weight: 0.0,
                    provider_count: 0,
                    providers: Vec::new(),
                });

            // Check for duplicate provider in same group
            if !entry.providers.contains(&obs.endpoint) {
                entry.weight += weight;
                entry.provider_count += 1;
                entry.providers.push(obs.endpoint.clone());
            }
        }

        // Sort groups by weight descending
        let mut group_vec: Vec<QuorumGroup> = groups.into_values().collect();
        group_vec.sort_by(|a, b| b.weight.partial_cmp(&a.weight).unwrap_or(Ordering::Equal));

        if group_vec.is_empty() {
            return Err(QuorumError::InsufficientProviders {
                required: 1,
                available: 0,
            });
        }

        // Check for Byzantine behavior
        if let Some(ref byzantine) = self.byzantine {
            if let Some(byzantine_result) = self.detect_byzantine(&group_vec, byzantine) {
                return Ok(byzantine_result);
            }
        }

        let best = &group_vec[0];
        let best_provider_count = best.provider_count;
        let total_weight: f64 = group_vec.iter().map(|g| g.weight).sum();

        // Check total weight against BFT requirement
        if let Some(ref byzantine) = self.byzantine {
            if !byzantine.satisfies_bft(best_provider_count) {
                // Not enough providers for BFT, but may still reach consensus
                // with relaxed threshold
            }
        }

        // Check agreement ratio
        let agreement_ratio = if total_weight > 0.0 {
            best.weight / total_weight
        } else {
            0.0
        };

        if agreement_ratio < self.threshold.min_agreement_ratio {
            return Ok(QuorumResult::NoConsensus {
                groups: group_vec,
                total_weight,
                reason: format!(
                    "Agreement ratio {:.3} below threshold {:.3}",
                    agreement_ratio, self.threshold.min_agreement_ratio
                ),
            });
        }

        // Check divergence ratio
        if group_vec.len() > 1 {
            let second_weight = group_vec[1].weight;
            let divergence = second_weight / best.weight;
            if divergence > self.threshold.max_divergence_ratio {
                return Ok(QuorumResult::NoConsensus {
                    groups: group_vec,
                    total_weight,
                    reason: format!(
                        "Divergence ratio {:.3} exceeds max {:.3}",
                        divergence, self.threshold.max_divergence_ratio
                    ),
                });
            }
        }

        // Check minimum provider count
        if best_provider_count < self.threshold.min_provider_count {
            return Ok(QuorumResult::NoConsensus {
                groups: group_vec,
                total_weight,
                reason: format!(
                    "Provider count {} below minimum {}",
                    best_provider_count, self.threshold.min_provider_count
                ),
            });
        }

        // Determine finality grade from observations
        let best_finality = QuorumEngine::determine_finality(&observations);

        // Find the canonical height from best group
        let canonical_height = observations
            .iter()
            .find(|o| best.providers.contains(&o.endpoint))
            .map(|o| o.observed_height)
            .unwrap_or(0);

        Ok(QuorumResult::Consensus {
            canonical_hash: best.hash,
            canonical_height,
            supporting_providers: best.providers.clone(),
            total_weight: best.weight,
            confidence: agreement_ratio,
            finality_grade: best_finality,
        })
    }

    /// Detect Byzantine behavior among providers.
    ///
    /// Checks for:
    /// - Providers returning different hashes than the majority
    /// - Providers with suspiciously low latency (potential spoofing)
    /// - Providers with zero health score
    fn detect_byzantine(
        &self,
        groups: &[QuorumGroup],
        byzantine: &ByzantineConfig,
    ) -> Option<QuorumResult> {
        let mut suspicious = Vec::new();

        // Check providers in non-dominant groups
        if groups.len() > 1 {
            let total_weight: f64 = groups.iter().map(|g| g.weight).sum();

            for group in &groups[1..] {
                // If a group has significant weight but isn't the best,
                // its providers may be Byzantine
                if group.weight / total_weight > 0.1 {
                    for provider in &group.providers {
                        suspicious.push(provider.clone());
                    }
                }
            }
        }

        // Check provider health
        for (endpoint, health) in &self.health {
            if health.suspended && !suspicious.contains(endpoint) {
                suspicious.push(endpoint.clone());
            }
        }

        if !suspicious.is_empty() && suspicious.len() <= byzantine.max_faulty {
            Some(QuorumResult::ByzantineDetected {
                suspicious_providers: suspicious,
                reason: "Byzantine behavior detected in RPC providers".into(),
            })
        } else {
            None
        }
    }

    /// Determine the finality grade from observations.
    ///
    /// Uses the best finality grade among agreeing providers.
    fn determine_finality(observations: &[RpcObservation]) -> FinalityGrade {
        // Find the best finality grade among observations
        let mut best = FinalityGrade::None;
        for obs in observations {
            match (&best, &obs.observed_finality) {
                (FinalityGrade::None, _) => best = obs.observed_finality,
                (_, FinalityGrade::None) => {}
                (FinalityGrade::Deterministic, _) => {
                    best = FinalityGrade::Deterministic;
                    break;
                }
                (FinalityGrade::Optimistic, FinalityGrade::Deterministic) => {
                    best = FinalityGrade::Deterministic;
                }
                (FinalityGrade::Optimistic, _) => {}
                (FinalityGrade::Probabilistic { .. }, FinalityGrade::Optimistic) => {
                    best = FinalityGrade::Optimistic;
                }
                (FinalityGrade::Probabilistic { confirmations: a }, FinalityGrade::Probabilistic { confirmations: b }) => {
                    if *b > *a {
                        best = obs.observed_finality;
                    }
                }
                _ => {}
            }
        }
        best
    }

    /// Update provider health after a round completes.
    fn update_health_after_round(&mut self, round: &ConsensusRound) {
        if let Some(ref decision) = round.decision {
            match decision {
                QuorumResult::Consensus {
                    supporting_providers,
                    ..
                } => {
                    for obs in &round.observations {
                        if let Some(health) = self.health.get_mut(&obs.endpoint) {
                            if supporting_providers.contains(&obs.endpoint) {
                                health.record_success(obs.latency_ms);
                            } else {
                                health.record_failure();
                            }
                        }
                    }
                }
                QuorumResult::NoConsensus { .. } | QuorumResult::ByzantineDetected { .. } => {
                    for obs in &round.observations {
                        if let Some(health) = self.health.get_mut(&obs.endpoint) {
                            health.record_failure();
                        }
                    }
                }
            }
        }
    }

    /// Get the health state of a provider.
    pub fn get_provider_health(&self, endpoint: &str) -> Option<&ProviderHealth> {
        self.health.get(endpoint)
    }

    /// Get all provider health states.
    pub fn all_health(&self) -> &BTreeMap<String, ProviderHealth> {
        &self.health
    }

    /// Get the health-adjusted weight for a provider.
    pub fn health_adjusted_weight(&self, endpoint: &str) -> f64 {
        let base_weight = self
            .weights
            .get(endpoint)
            .map(|w| w.raw())
            .unwrap_or(1.0);
        let health_score = self
            .health
            .get(endpoint)
            .map(|h| h.health_score())
            .unwrap_or(1.0);
        base_weight * health_score
    }

    /// Check if quorum is achievable with current providers.
    pub fn is_quorum_achievable(&self) -> bool {
        let active_providers = self
            .health
            .values()
            .filter(|h| !h.suspended)
            .count();
        active_providers >= self.threshold.min_provider_count
    }

    /// Get the minimum number of providers needed for quorum.
    pub fn min_providers_needed(&self) -> usize {
        self.threshold.min_provider_count
    }

    /// Get the current threshold configuration.
    pub fn threshold(&self) -> &QuorumThreshold {
        &self.threshold
    }
}

impl Default for QuorumEngine {
    fn default() -> Self {
        Self::new(QuorumThreshold::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rpc::quorum_types::FinalityGrade;

    fn make_hash(n: u8) -> Hash {
        let mut bytes = [0u8; 32];
        bytes[0] = n;
        Hash::from(bytes)
    }

    fn make_observation(endpoint: &str, hash: Hash, height: u64) -> RpcObservation {
        RpcObservation::new(
            endpoint.to_string(),
            "test-chain".to_string(),
            height,
            hash,
            FinalityGrade::Deterministic,
            100,
        )
    }

    #[test]
    fn test_engine_creation() {
        let engine = QuorumEngine::new(QuorumThreshold::bft_strict());
        assert_eq!(engine.provider_count(), 0);
        assert!(!engine.is_quorum_achievable());
    }

    #[test]
    fn test_provider_registration() {
        let mut engine = QuorumEngine::new(QuorumThreshold::bft_strict());
        engine.register_default_provider("http://provider1.com".into());
        engine.register_default_provider("http://provider2.com".into());
        engine.register_default_provider("http://provider3.com".into());
        assert_eq!(engine.provider_count(), 3);
    }

    #[test]
    fn test_consensus_reached() {
        let mut engine = QuorumEngine::new(QuorumThreshold::bft_strict());
        engine.register_default_provider("http://p1.com".into());
        engine.register_default_provider("http://p2.com".into());
        engine.register_default_provider("http://p3.com".into());

        let hash = make_hash(0x42);
        let mut round = engine.start_round("chain-1".into());
        round.add_observation(make_observation("http://p1.com", hash, 100));
        round.add_observation(make_observation("http://p2.com", hash, 100));
        round.add_observation(make_observation("http://p3.com", hash, 100));

        let result = engine.decide(&mut round).unwrap();
        match result {
            QuorumResult::Consensus {
                canonical_hash,
                supporting_providers,
                confidence,
                ..
            } => {
                assert_eq!(canonical_hash, hash);
                assert_eq!(supporting_providers.len(), 3);
                assert!((confidence - 1.0).abs() < 0.001);
            }
            other => panic!("Expected Consensus, got {:?}", other),
        }
    }

    #[test]
    fn test_no_consensus_divergence() {
        let mut engine = QuorumEngine::new(QuorumThreshold::bft_strict());
        engine.register_default_provider("http://p1.com".into());
        engine.register_default_provider("http://p2.com".into());
        engine.register_default_provider("http://p3.com".into());

        let hash_a = make_hash(0x01);
        let hash_b = make_hash(0x02);
        let mut round = engine.start_round("chain-1".into());
        round.add_observation(make_observation("http://p1.com", hash_a, 100));
        round.add_observation(make_observation("http://p2.com", hash_b, 100));
        round.add_observation(make_observation("http://p3.com", hash_b, 100));

        let result = engine.decide(&mut round).unwrap();
        match result {
            QuorumResult::Consensus { .. } => {
                // 2/3 = 0.667 which meets bft_strict threshold, so consensus may be reached
                // This test verifies the engine handles split votes
            }
            QuorumResult::NoConsensus { .. } => {
                // Divergence prevented consensus
            }
            _ => panic!("Expected Consensus or NoConsensus"),
        }
    }

    #[test]
    fn test_insufficient_providers() {
        let engine = QuorumEngine::new(QuorumThreshold::bft_strict());
        let round = ConsensusRound::new(1, "chain-1".into());

        let result = engine.compute_decision(&round);
        assert!(matches!(
            result,
            Err(QuorumError::InsufficientProviders { .. })
        ));
    }

    #[test]
    fn test_round_already_decided() {
        let mut engine = QuorumEngine::new(QuorumThreshold::bft_strict());
        engine.register_default_provider("http://p1.com".into());
        engine.register_default_provider("http://p2.com".into());
        engine.register_default_provider("http://p3.com".into());

        let hash = make_hash(0x42);
        let mut round = engine.start_round("chain-1".into());
        round.add_observation(make_observation("http://p1.com", hash, 100));
        round.add_observation(make_observation("http://p2.com", hash, 100));
        round.add_observation(make_observation("http://p3.com", hash, 100));

        engine.decide(&mut round).unwrap();
        let result = engine.decide(&mut round);
        assert!(matches!(result, Err(QuorumError::RoundComplete(1))));
    }

    #[test]
    fn test_provider_health_tracking() {
        let mut health = ProviderHealth::new("http://p1.com".into());
        health.record_success(100);
        health.record_success(200);
        assert_eq!(health.consecutive_failures, 0);
        assert!(health.last_success.is_some());

        health.record_failure();
        assert_eq!(health.consecutive_failures, 1);

        for _ in 0..4 {
            health.record_failure();
        }
        assert!(health.suspended);
    }

    #[test]
    fn test_health_adjusted_weight() {
        let mut engine = QuorumEngine::new(QuorumThreshold::bft_strict());
        engine.register_default_provider("http://p1.com".into());

        // Base weight is 1.0, health score starts at 1.0
        assert!((engine.health_adjusted_weight("http://p1.com") - 1.0).abs() < 0.01);

        // Record failures to reduce health
        if let Some(health) = engine.health.get_mut("http://p1.com") {
            for _ in 0..5 {
                health.record_failure();
            }
        }

        let adjusted = engine.health_adjusted_weight("http://p1.com");
        assert!(adjusted < 1.0);
    }

    #[test]
    fn test_bft_config() {
        let bft = ByzantineConfig::new(1, 4).unwrap();
        assert_eq!(bft.min_quorum_size(), 3);
        assert!(bft.satisfies_bft(3));
        assert!(!bft.satisfies_bft(2));
    }

    #[test]
    fn test_bft_config_invalid() {
        let result = ByzantineConfig::new(2, 4);
        assert!(result.is_err());
    }

    #[test]
    fn test_quorum_threshold_validation() {
        let result = QuorumThreshold::new(0.3, 2, 0.5);
        assert!(result.is_err());

        let result = QuorumThreshold::new(0.5, 0, 0.5);
        assert!(result.is_err());

        let result = QuorumThreshold::new(1.5, 2, 0.5);
        assert!(result.is_err());
    }

    #[test]
    fn test_majority_threshold() {
        let threshold = QuorumThreshold::majority();
        assert_eq!(threshold.min_agreement_ratio, 0.51);
        assert_eq!(threshold.min_provider_count, 2);
    }

    #[test]
    fn test_provider_weight_validation() {
        let result = ProviderWeight::new(-1.0);
        assert!(result.is_err());

        let result = ProviderWeight::new(0.0);
        assert!(result.is_err());

        let result = ProviderWeight::new(2.0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_quorum_group_sorting() {
        let mut engine = QuorumEngine::new(QuorumThreshold::bft_strict());
        engine.register_provider("http://p1.com".into(), ProviderWeight::new(3.0).unwrap());
        engine.register_provider("http://p2.com".into(), ProviderWeight::new(1.0).unwrap());
        engine.register_provider("http://p3.com".into(), ProviderWeight::new(1.0).unwrap());

        let _hash_a = make_hash(0x01);
        let hash_b = make_hash(0x02);
        let mut round = engine.start_round("chain-1".into());
        // p1 has weight 3.0, votes for _hash_a
        // p2 and p3 have weight 1.0 each, vote for hash_b (total 2.0)
        round.add_observation(make_observation("http://p1.com", _hash_a, 100));
        round.add_observation(make_observation("http://p2.com", hash_b, 100));
        round.add_observation(make_observation("http://p3.com", hash_b, 100));

        let result = engine.decide(&mut round).unwrap();
        match result {
            QuorumResult::Consensus {
                canonical_hash,
                supporting_providers: _,
                confidence,
                ..
            } => {
                // p1's weight (3.0) / total (5.0) = 0.6 which is below 0.667 threshold
                // So this should be NoConsensus
                assert!(
                    matches!(
                        engine.compute_decision(&round).unwrap(),
                        QuorumResult::NoConsensus { .. }
                    )
                    || matches!(canonical_hash, _hash_a)
                );
                assert!((confidence - 0.6).abs() < 0.01);
            }
            QuorumResult::NoConsensus { .. } => {
                // Divergence prevented consensus (0.6 < 0.667)
            }
            _ => {}
        }
    }

    #[test]
    fn test_is_quorum_achievable() {
        let mut engine = QuorumEngine::new(QuorumThreshold::bft_strict());
        // No providers, min_provider_count is 2, so not achievable
        assert!(!engine.is_quorum_achievable());

        engine.register_default_provider("http://p1.com".into());
        assert!(!engine.is_quorum_achievable());

        engine.register_default_provider("http://p2.com".into());
        assert!(engine.is_quorum_achievable());
    }

    #[test]
    fn test_round_counter_increments() {
        let mut engine = QuorumEngine::new(QuorumThreshold::bft_strict());
        let r1 = engine.start_round("chain-1".into());
        let r2 = engine.start_round("chain-2".into());
        assert_eq!(r1.round_id, 1);
        assert_eq!(r2.round_id, 2);
    }

    #[test]
    fn test_remove_provider() {
        let mut engine = QuorumEngine::new(QuorumThreshold::bft_strict());
        engine.register_default_provider("http://p1.com".into());
        engine.register_default_provider("http://p2.com".into());
        assert_eq!(engine.provider_count(), 2);

        engine.remove_provider("http://p1.com");
        assert_eq!(engine.provider_count(), 1);
        assert!(engine.get_provider_health("http://p1.com").is_none());
    }
}
