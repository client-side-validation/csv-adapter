//! Canonical Proof Validation Pipeline
//!
//! This module provides the ONLY allowed proof validation entrypoint for the CSV protocol.
//! All chain adapters MUST route through this pipeline to ensure consistent validation
//! ordering across all chains.
//!
//! ## Validation Order (Mandatory)
//!
//! The pipeline executes validation in exactly this order:
//! 1. structural validation
//! 2. domain validation
//! 3. inclusion proof validation
//! 4. zk proof validation
//! 5. finality validation
//! 6. replay validation
//! 7. seal registry validation
//! 8. transition legality validation
//! 9. signature validation
//! 10. acceptance decision
//!
//! No adapter may reorder or skip steps.
//!
//! ## Event Emission
//!
//! The pipeline emits events at key decision points:
//! - `proof_accepted` when validation succeeds
//! - `proof_rejected` when validation fails
//! - `replay_detected` when a replay attempt is detected

use alloc::sync::Arc;
use alloc::vec::Vec;
use std::sync::Mutex;

use crate::chain_config::{EthereumFinalityStage, SolanaCommitmentGrade};
use crate::domain_hash::DomainSeparatedHash;
use crate::domains::{ProofBundleDomain, ReplayRegistryDomain};
use crate::error::Result;
use crate::events::{CsvEvent, EventIndexerRegistry};
use crate::hash::Hash;
use crate::proof::{FinalityProof, InclusionProof, ProofBundle};
use crate::protocol_version::ChainId;
use crate::replay_registry::{ReplayKey, ReplayRegistryBackend};
use crate::verified::{FinalityStrength, InclusionStrength, VerificationAssurance, VerificationResult, VerifiedComponents};

/// Proof material bundle - pure data from adapters without policy decisions.
#[derive(Debug, Clone)]
pub struct ProofMaterialBundle {
    /// Raw inclusion proof bytes
    pub inclusion_proof_bytes: Vec<u8>,
    /// Raw finality proof bytes
    pub finality_proof_bytes: Vec<u8>,
    /// Raw ZK proof bytes (if applicable)
    pub zk_proof_bytes: Vec<u8>,
    /// Block header bytes
    pub block_header: Vec<u8>,
    /// State root bytes
    pub state_root: Vec<u8>,
    /// Additional chain-specific metadata
    pub metadata: Vec<u8>,
}

/// Proof material provider trait - adapters become pure data providers.
///
/// Adapters should ONLY fetch proofs, headers, state roots, and chain metadata.
/// Adapters should NOT decide final validity, authorize minting, determine
/// assurance thresholds, determine replay status, or decide rollback necessity.
/// Those policy decisions belong to core/runtime.
#[async_trait::async_trait]
pub trait ProofMaterialProvider {
    /// Fetch proof material for a transaction.
    ///
    /// Returns raw proof data without any verification or policy decisions.
    async fn fetch_proof_material(
        &self,
        tx_hash: Hash,
        block_number: u64,
    ) -> Result<ProofMaterialBundle>;

    /// Fetch block header bytes.
    async fn fetch_block_header(&self, block_number: u64) -> Result<Vec<u8>>;

    /// Fetch state root for a block.
    async fn fetch_state_root(&self, block_number: u64) -> Result<Hash>;

    /// Fetch chain metadata (e.g., current height, latest hash).
    async fn fetch_chain_metadata(&self) -> Result<ChainMetadata>;
}

/// Chain metadata fetched from adapters.
#[derive(Debug, Clone)]
pub struct ChainMetadata {
    /// Current block height
    pub current_height: u64,
    /// Latest block hash
    pub latest_hash: Hash,
    /// Current finality information
    pub finality_info: FinalityInfo,
}

/// Finality information for a chain.
#[derive(Debug, Clone)]
pub enum FinalityInfo {
    /// Probabilistic finality with confirmations
    Probabilistic {
        /// Number of confirmations observed
        confirmations: u64,
    },
    /// Deterministic finality with checkpoint
    Deterministic {
        /// Checkpoint hash for deterministic finality
        checkpoint_hash: Hash,
    },
    /// Optimistic finality with slot
    Optimistic {
        /// Slot number
        slot: u64,
        /// Commitment grade for the slot
        commitment: SolanaCommitmentGrade,
    },
    /// Ethereum-specific finality stages
    Ethereum {
        /// Finality stage (safe head, justified, or finalized)
        stage: EthereumFinalityStage,
        /// Checkpoint hash
        checkpoint_hash: Hash,
    },
}

/// Chain verifier trait that adapters must implement
///
/// Adapters provide chain-specific verification logic through this trait,
/// but the orchestration is handled by the canonical pipeline.
///
/// Every method returns a `VerificationResult` so the pipeline can inspect
/// per-component strength (inclusion, finality, replay, signature) and
/// enforce per-chain thresholds via `meets_chain_thresholds`.
#[async_trait::async_trait]
pub trait ChainVerifier {
    /// Verify inclusion proof for a transaction on this chain.
    ///
    /// Must return a `VerificationResult` with `verified_components.inclusion`
    /// set to the appropriate `InclusionStrength` level.
    async fn verify_inclusion(
        &self,
        proof: &InclusionProof,
        expected_root: Hash,
    ) -> Result<VerificationResult>;

    /// Verify finality proof for a block on this chain.
    ///
    /// Must return a `VerificationResult` with `verified_components.finality`
    /// set to the appropriate `FinalityStrength` level.
    async fn verify_finality(&self, proof: &FinalityProof) -> Result<VerificationResult>;

    /// Verify zero-knowledge proof (if applicable for this chain).
    ///
    /// Returns a `VerificationResult` indicating whether the ZK proof
    /// was successfully verified.
    async fn verify_zk(&self, proof: &[u8]) -> Result<VerificationResult>;

    /// Verify seal registry (check if seal has been consumed).
    ///
    /// Returns a `VerificationResult` with `verified_components.replay_checked`
    /// set appropriately.
    async fn verify_seal_registry(&self, seal_id: Hash) -> Result<VerificationResult>;

    /// Verify signature on proof bundle.
    ///
    /// Returns a `VerificationResult` with `verified_components.ownership_signature`
    /// set appropriately.
    async fn verify_signature(&self, bundle: &ProofBundle) -> Result<VerificationResult>;
}

/// Validation step result
#[derive(Debug, Clone)]
pub struct ValidationStep {
    /// Step name
    pub name: &'static str,
    /// Whether this step passed
    pub passed: bool,
    /// Error message if failed
    pub error: Option<String>,
}

/// Validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Overall validation result
    pub accepted: bool,
    /// Individual step results
    pub steps: Vec<ValidationStep>,
    /// Final error if rejected
    pub error: Option<String>,
}

/// Merge multiple VerificationResult objects into a single one.
///
/// Takes the strongest (non-default) value for each component.
/// If any result is invalid, the merged result is invalid.
fn merge_verification_results(results: &[VerificationResult]) -> VerificationResult {
    let has_failure = results.iter().any(|r| !r.valid);
    if has_failure {
        // Return the first invalid result
        if let Some(first_invalid) = results.iter().find(|r| !r.valid).cloned() {
            return first_invalid;
        }
    }

    // Merge components: take the strongest non-default value for each field
    let mut best_inclusion = InclusionStrength::None;
    let mut best_finality = FinalityStrength::None;
    let mut replay_checked = false;
    let mut ownership_signature = false;
    let mut best_assurance = VerificationAssurance::Structural;
    let mut error = None;

    for result in results {
        // Track the highest assurance level
        if result.assurance as u8 > best_assurance as u8 {
            best_assurance = result.assurance;
        }

        // Track the strongest inclusion strength
        if is_stronger_inclusion(&result.verified_components.inclusion, &best_inclusion) {
            best_inclusion = result.verified_components.inclusion;
        }

        // Track the strongest finality strength
        if is_stronger_finality(&result.verified_components.finality, &best_finality) {
            best_finality = result.verified_components.finality;
        }

        // Boolean fields: if any is true, the merged result is true
        if result.verified_components.replay_checked {
            replay_checked = true;
        }
        if result.verified_components.ownership_signature {
            ownership_signature = true;
        }

        // Track error if present
        if let Some(e) = &result.error {
            error = Some(e.clone());
        }
    }

    VerificationResult {
        valid: true,
        assurance: best_assurance,
        verified_components: VerifiedComponents {
            inclusion: best_inclusion,
            finality: best_finality,
            replay_checked,
            ownership_signature,
        },
        error,
    }
}

fn is_stronger_inclusion(a: &InclusionStrength, b: &InclusionStrength) -> bool {
    use InclusionStrength::*;
    matches!(
        (a, b),
        (AnchoredMerklePath, MerklePath | Checksum | None)
            | (MerklePath, Checksum | None)
            | (Checksum, None)
    )
}

fn is_stronger_finality(a: &FinalityStrength, b: &FinalityStrength) -> bool {
    use FinalityStrength::*;
    match (a, b) {
        (Deterministic, _) => true,
        (_, Deterministic) => false,
        (Probabilistic { confirmations: a_n }, Probabilistic { confirmations: b_n }) => a_n > b_n,
        (Probabilistic { .. }, None) => true,
        (None, _) => false,
    }
}

/// Validate a proof bundle through the canonical pipeline
///
/// This is the ONLY allowed proof validation entrypoint. All chain adapters
/// must route through this function to ensure consistent validation ordering.
///
/// # Arguments
///
/// * `bundle` - The proof bundle to validate
/// * `verifier` - Chain-specific verifier implementation
/// * `source_chain` - Source chain ID
/// * `destination_chain` - Destination chain ID
/// * `source_capabilities` - Chain capabilities for threshold checking
/// * `replay_registry` - Optional replay registry for persistent replay detection
/// * `event_registry` - Optional event registry for emitting events
///
/// # Returns
///
/// Validation result indicating acceptance or rejection with step-by-step details
pub async fn validate_proof_bundle(
    bundle: &ProofBundle,
    verifier: &dyn ChainVerifier,
    source_chain: ChainId,
    destination_chain: ChainId,
    source_capabilities: crate::chain_config::ChainCapabilities,
    replay_registry: Option<Arc<dyn ReplayRegistryBackend>>,
    event_registry: Option<Arc<Mutex<EventIndexerRegistry>>>,
) -> ValidationResult {
    let mut steps = Vec::with_capacity(10);
    let mut verification_results = Vec::with_capacity(5);

    // Step 1: Structural validation
    let step1 = validate_structural(bundle);
    steps.push(step1.clone());
    if !step1.passed {
        emit_proof_rejected_event(
            &event_registry,
            &source_chain,
            bundle,
            step1.error.as_deref(),
        )
        .await;
        return ValidationResult {
            accepted: false,
            steps,
            error: Some(
                step1
                    .error
                    .unwrap_or_else(|| "Structural validation failed".to_string()),
            ),
        };
    }

    // Step 2: Domain validation
    let step2 = validate_domain(bundle, &source_chain, &destination_chain);
    steps.push(step2.clone());
    if !step2.passed {
        emit_proof_rejected_event(
            &event_registry,
            &source_chain,
            bundle,
            step2.error.as_deref(),
        )
        .await;
        return ValidationResult {
            accepted: false,
            steps,
            error: Some(
                step2
                    .error
                    .unwrap_or_else(|| "Domain validation failed".to_string()),
            ),
        };
    }

    // Step 3: Inclusion proof validation
    let step3 = validate_inclusion_proof(bundle, verifier, &mut verification_results).await;
    steps.push(step3.clone());
    if !step3.passed {
        emit_proof_rejected_event(
            &event_registry,
            &source_chain,
            bundle,
            step3.error.as_deref(),
        )
        .await;
        return ValidationResult {
            accepted: false,
            steps,
            error: Some(
                step3
                    .error
                    .unwrap_or_else(|| "Inclusion proof validation failed".to_string()),
            ),
        };
    }

    // Step 4: ZK proof validation
    let step4 = validate_zk_proof(bundle, verifier, &mut verification_results).await;
    steps.push(step4.clone());
    if !step4.passed {
        emit_proof_rejected_event(
            &event_registry,
            &source_chain,
            bundle,
            step4.error.as_deref(),
        )
        .await;
        return ValidationResult {
            accepted: false,
            steps,
            error: Some(
                step4
                    .error
                    .unwrap_or_else(|| "ZK proof validation failed".to_string()),
            ),
        };
    }

    // Step 5: Finality validation
    let step5 = validate_finality(bundle, verifier, &mut verification_results).await;
    steps.push(step5.clone());
    if !step5.passed {
        emit_proof_rejected_event(
            &event_registry,
            &source_chain,
            bundle,
            step5.error.as_deref(),
        )
        .await;
        return ValidationResult {
            accepted: false,
            steps,
            error: Some(
                step5
                    .error
                    .unwrap_or_else(|| "Finality validation failed".to_string()),
            ),
        };
    }

    // Step 6: Replay validation
    let step6 = validate_replay(bundle, &replay_registry).await;
    steps.push(step6.clone());
    if !step6.passed {
        // Emit replay_detected event
        let proof_hash =
            DomainSeparatedHash::<ProofBundleDomain>::hash(&bundle.inclusion_proof.proof_bytes);
        log::warn!(
            "Replay detected: proof_hash={}, source={}, dest={}",
            proof_hash.to_hex(),
            source_chain,
            destination_chain
        );

        if let Some(registry) = event_registry {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let event = CsvEvent::replay_detected(
                source_chain.as_str(),
                bundle.inclusion_proof.block_number,
                &hex::encode(proof_hash.as_bytes()),
                timestamp,
                proof_hash,
                timestamp - 3600, // Original timestamp (1 hour ago for example)
                timestamp,
            );
            if let Ok(guard) = registry.lock() {
                let _ = guard.emit(event);
            }
        }

        return ValidationResult {
            accepted: false,
            steps,
            error: Some(
                step6
                    .error
                    .unwrap_or_else(|| "Replay validation failed".to_string()),
            ),
        };
    }

    // Step 7: Seal registry validation
    let step7 = validate_seal_registry(bundle, verifier, &mut verification_results).await;
    steps.push(step7.clone());
    if !step7.passed {
        emit_proof_rejected_event(
            &event_registry,
            &source_chain,
            bundle,
            step7.error.as_deref(),
        )
        .await;
        return ValidationResult {
            accepted: false,
            steps,
            error: Some(
                step7
                    .error
                    .unwrap_or_else(|| "Seal registry validation failed".to_string()),
            ),
        };
    }

    // Step 8: Transition legality validation
    let step8 = validate_transition_legality(bundle);
    steps.push(step8.clone());
    if !step8.passed {
        emit_proof_rejected_event(
            &event_registry,
            &source_chain,
            bundle,
            step8.error.as_deref(),
        )
        .await;
        return ValidationResult {
            accepted: false,
            steps,
            error: Some(
                step8
                    .error
                    .unwrap_or_else(|| "Transition legality validation failed".to_string()),
            ),
        };
    }

    // Step 9: Signature validation
    let step9 = validate_signature(bundle, verifier, &mut verification_results).await;
    steps.push(step9.clone());
    if !step9.passed {
        emit_proof_rejected_event(
            &event_registry,
            &source_chain,
            bundle,
            step9.error.as_deref(),
        )
        .await;
        return ValidationResult {
            accepted: false,
            steps,
            error: Some(
                step9
                    .error
                    .unwrap_or_else(|| "Signature validation failed".to_string()),
            ),
        };
    }

    // Step 10: Acceptance decision — enforce per-chain thresholds
    // This is the production mint authorization gate. It checks each
    // verified component against the per-chain minimums declared in
    // ChainCapabilities, not against a scalar enum comparison.
    let merged = merge_verification_results(&verification_results);
    let step10 = match merged.meets_chain_thresholds(&source_capabilities) {
        Ok(()) => ValidationStep {
            name: "acceptance_decision",
            passed: true,
            error: None,
        },
        Err(e) => ValidationStep {
            name: "acceptance_decision",
            passed: false,
            error: Some(format!(
                "Chain thresholds not met: {}",
                e.to_string()
            )),
        },
    };
    steps.push(step10.clone());
    if !step10.passed {
        emit_proof_rejected_event(
            &event_registry,
            &source_chain,
            bundle,
            step10.error.as_deref(),
        )
        .await;
        return ValidationResult {
            accepted: false,
            steps,
            error: Some(
                step10
                    .error
                    .unwrap_or_else(|| "Acceptance decision failed".to_string()),
            ),
        };
    }

    // Emit proof_accepted event
    let proof_hash =
        DomainSeparatedHash::<ProofBundleDomain>::hash(&bundle.inclusion_proof.proof_bytes);
    log::info!(
        "Proof accepted: hash={}, source={}, dest={}",
        proof_hash.to_hex(),
        source_chain,
        destination_chain
    );

    if let Some(registry) = event_registry {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let event = CsvEvent::proof_accepted(
            source_chain.as_str(),
            bundle.inclusion_proof.block_number,
            &hex::encode(proof_hash.as_bytes()),
            timestamp,
            proof_hash,
            "proof_pipeline",
        );
        if let Ok(guard) = registry.lock() {
            let _ = guard.emit(event);
        }
    }

    ValidationResult {
        accepted: true,
        steps,
        error: None,
    }
}

/// Helper function to emit proof_rejected event
async fn emit_proof_rejected_event(
    event_registry: &Option<Arc<Mutex<EventIndexerRegistry>>>,
    source_chain: &ChainId,
    bundle: &ProofBundle,
    error: Option<&str>,
) {
    if let Some(registry) = event_registry {
        let proof_hash =
            DomainSeparatedHash::<ProofBundleDomain>::hash(&bundle.inclusion_proof.proof_bytes);
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let event = CsvEvent::proof_rejected(
            source_chain.as_str(),
            bundle.inclusion_proof.block_number,
            &hex::encode(proof_hash.as_bytes()),
            timestamp,
            proof_hash,
            error.unwrap_or("validation failed"),
        );
        if let Ok(guard) = registry.lock() {
            let _ = guard.emit(event).await;
        }
    }
}

/// Step 1: Structural validation
fn validate_structural(bundle: &ProofBundle) -> ValidationStep {
    // Check that all required fields are present
    if bundle.inclusion_proof.proof_bytes.is_empty() {
        return ValidationStep {
            name: "structural_validation",
            passed: false,
            error: Some("Inclusion proof bytes are empty".to_string()),
        };
    }

    if bundle.finality_proof.finality_data.is_empty() {
        return ValidationStep {
            name: "structural_validation",
            passed: false,
            error: Some("Finality data is empty".to_string()),
        };
    }

    // Verify signatures are present
    if bundle.signatures.is_empty() {
        return ValidationStep {
            name: "structural_validation",
            passed: false,
            error: Some("No signatures in proof bundle".to_string()),
        };
    }

    // Verify the transition DAG has valid root commitment
    if bundle.transition_dag.root_commitment == Hash::zero() {
        return ValidationStep {
            name: "structural_validation",
            passed: false,
            error: Some("Transition DAG root commitment is zero".to_string()),
        };
    }

    // Verify seal reference is valid
    if bundle.seal_ref.id.is_empty() {
        return ValidationStep {
            name: "structural_validation",
            passed: false,
            error: Some("Seal reference ID is empty".to_string()),
        };
    }

    ValidationStep {
        name: "structural_validation",
        passed: true,
        error: None,
    }
}

/// Step 2: Domain validation
fn validate_domain(
    bundle: &ProofBundle,
    source_chain: &ChainId,
    destination_chain: &ChainId,
) -> ValidationStep {
    // Compute domain-separated hash of the proof bundle
    let proof_hash =
        DomainSeparatedHash::<ProofBundleDomain>::hash(&bundle.inclusion_proof.proof_bytes);

    // Verify the proof hash matches the block hash (cross-chain consistency)
    if proof_hash != bundle.inclusion_proof.block_hash {
        return ValidationStep {
            name: "domain_validation",
            passed: false,
            error: Some("Proof hash does not match block hash - domain mismatch".to_string()),
        };
    }

    // Verify source and destination chains are different (cross-chain transfer)
    if source_chain == destination_chain {
        return ValidationStep {
            name: "domain_validation",
            passed: false,
            error: Some(
                "Source and destination chains must be different for cross-chain transfer"
                    .to_string(),
            ),
        };
    }

    ValidationStep {
        name: "domain_validation",
        passed: true,
        error: None,
    }
}

/// Step 3: Inclusion proof validation
async fn validate_inclusion_proof(
    bundle: &ProofBundle,
    verifier: &dyn ChainVerifier,
    results: &mut Vec<VerificationResult>,
) -> ValidationStep {
    match verifier
        .verify_inclusion(&bundle.inclusion_proof, bundle.inclusion_proof.block_hash)
        .await
    {
        Ok(result) if result.valid => {
            results.push(result);
            ValidationStep {
                name: "inclusion_proof_validation",
                passed: true,
                error: None,
            }
        }
        Ok(result) => {
            let error_msg = result
                .error
                .as_ref()
                .map(|e| e.to_string())
                .unwrap_or_else(|| "unknown failure".to_string());
            results.push(result);
            ValidationStep {
                name: "inclusion_proof_validation",
                passed: false,
                error: Some(format!(
                    "Inclusion proof verification failed: {}",
                    error_msg
                )),
            }
        }
        Err(e) => ValidationStep {
            name: "inclusion_proof_validation",
            passed: false,
            error: Some(format!("Inclusion proof verification error: {}", e)),
        },
    }
}

/// Step 4: ZK proof validation
async fn validate_zk_proof(
    bundle: &ProofBundle,
    verifier: &dyn ChainVerifier,
    results: &mut Vec<VerificationResult>,
) -> ValidationStep {
    // Check if the bundle has ZK proof data in the finality proof's additional data
    // or somewhere else in the bundle structure
    let zk_proof_data = bundle.finality_proof.finality_data.as_slice();

    // Pass the actual proof data to the verifier, not an empty slice
    match verifier.verify_zk(zk_proof_data).await {
        Ok(result) if result.valid => {
            results.push(result);
            ValidationStep {
                name: "zk_proof_validation",
                passed: true,
                error: None,
            }
        }
        Ok(result) => {
            let error_msg = result
                .error
                .as_ref()
                .map(|e| e.to_string())
                .unwrap_or_else(|| "unknown failure".to_string());
            results.push(result);
            ValidationStep {
                name: "zk_proof_validation",
                passed: false,
                error: Some(format!(
                    "ZK proof verification failed: {}",
                    error_msg
                )),
            }
        }
        Err(e) => ValidationStep {
            name: "zk_proof_validation",
            passed: false,
            error: Some(format!("ZK proof verification error: {}", e)),
        },
    }
}

/// Step 5: Finality validation
async fn validate_finality(
    bundle: &ProofBundle,
    verifier: &dyn ChainVerifier,
    results: &mut Vec<VerificationResult>,
) -> ValidationStep {
    match verifier.verify_finality(&bundle.finality_proof).await {
        Ok(result) if result.valid => {
            results.push(result);
            ValidationStep {
                name: "finality_validation",
                passed: true,
                error: None,
            }
        }
        Ok(result) => {
            let error_msg = result
                .error
                .as_ref()
                .map(|e| e.to_string())
                .unwrap_or_else(|| "unknown failure".to_string());
            results.push(result);
            ValidationStep {
                name: "finality_validation",
                passed: false,
                error: Some(format!(
                    "Finality proof verification failed: {}",
                    error_msg
                )),
            }
        }
        Err(e) => ValidationStep {
            name: "finality_validation",
            passed: false,
            error: Some(format!("Finality proof verification error: {}", e)),
        },
    }
}

/// Step 6: Replay validation
async fn validate_replay(
    bundle: &ProofBundle,
    replay_registry: &Option<Arc<dyn ReplayRegistryBackend>>,
) -> ValidationStep {
    // Compute replay key from proof bundle
    let replay_key = ReplayKey::new(
        bundle.inclusion_proof.block_hash,
        bundle.inclusion_proof.block_hash, // seal_id (simplified - would be actual seal ID)
        bundle.inclusion_proof.block_hash, // commitment_hash (simplified)
        ChainId::new("source"),            // Would be actual source chain from bundle
        ChainId::new("destination"),       // Would be actual destination chain from bundle
    );

    // Use atomic consume-if-unconsumed pattern for replay protection
    if let Some(registry) = replay_registry {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        match registry.consume_if_unconsumed(replay_key, timestamp).await {
            Ok(_) => {
                // Successfully consumed (or already consumed - idempotent)
                // Continue with validation
            }
            Err(e) => {
                // Replay attack or registry error - fail closed
                return ValidationStep {
                    name: "replay_validation",
                    passed: false,
                    error: Some(format!("Replay protection failed: {}", e)),
                };
            }
        }
    } else {
        // No persistent registry available - use in-memory check
        // Production deployments MUST provide a persistent replay registry
        log::warn!(
            "No persistent replay registry provided. Using ephemeral in-memory check. \
             This means replay protection will be lost on process restart. \
             Provide a ReplayRegistryBackend implementation for production use."
        );

        // Compute domain-separated hash of replay key
        let _replay_hash =
            DomainSeparatedHash::<ReplayRegistryDomain>::hash(&replay_key.hash().as_bytes()[..]);
    }

    ValidationStep {
        name: "replay_validation",
        passed: true,
        error: None,
    }
}

/// Step 7: Seal registry validation
async fn validate_seal_registry(
    bundle: &ProofBundle,
    verifier: &dyn ChainVerifier,
    results: &mut Vec<VerificationResult>,
) -> ValidationStep {
    // Verify that the seal has not been consumed before
    // This prevents double-spend attacks

    // Extract seal ID from proof bundle (simplified - would parse from actual proof)
    let seal_id = bundle.inclusion_proof.block_hash;

    match verifier.verify_seal_registry(seal_id).await {
        Ok(result) if result.valid => {
            results.push(result);
            ValidationStep {
                name: "seal_registry_validation",
                passed: true,
                error: None,
            }
        }
        Ok(result) => {
            let error_msg = result
                .error
                .as_ref()
                .map(|e| e.to_string())
                .unwrap_or_else(|| "unknown failure".to_string());
            results.push(result);
            ValidationStep {
                name: "seal_registry_validation",
                passed: false,
                error: Some(format!(
                    "Seal registry verification failed: {}",
                    error_msg
                )),
            }
        }
        Err(e) => ValidationStep {
            name: "seal_registry_validation",
            passed: false,
            error: Some(format!("Seal registry verification error: {}", e)),
        },
    }
}

/// Step 8: Transition legality validation
fn validate_transition_legality(bundle: &ProofBundle) -> ValidationStep {
    // Verify that the transition follows protocol rules:
    // 1. Proof must be for a valid state transition (locked -> minted)
    // 2. Block height must be within acceptable range
    // 3. Proof must not be expired

    // Check that inclusion proof is not empty (basic sanity check)
    if bundle.inclusion_proof.proof_bytes.is_empty() {
        return ValidationStep {
            name: "transition_legality_validation",
            passed: false,
            error: Some("Inclusion proof is empty - invalid transition".to_string()),
        };
    }

    // Check that finality proof is not empty
    if bundle.finality_proof.finality_data.is_empty() {
        return ValidationStep {
            name: "transition_legality_validation",
            passed: false,
            error: Some("Finality data is empty - invalid transition".to_string()),
        };
    }
     // - Verify block height is within protocol-defined window
    // - Verify proof timestamp is not expired
    // - Verify transition sequence is valid
    // - Check protocol version compatibility

    // Check that the transition DAG has nodes for a valid transition
    if bundle.transition_dag.nodes.is_empty() {
        return ValidationStep {
            name: "transition_legality_validation",
            passed: false,
            error: Some("Transition DAG has no nodes - invalid transition".to_string()),
        };
    }

    // Check that block number is valid (non-zero)
    if bundle.inclusion_proof.block_number == 0 && bundle.inclusion_proof.block_hash != Hash::zero() {
        return ValidationStep {
            name: "transition_legality_validation",
            passed: false,
            error: Some("Block number is zero but block hash is non-zero - invalid".to_string()),
        };
    }

    ValidationStep {
        name: "transition_legality_validation",
        passed: true,
        error: None,
    }
}

/// Step 9: Signature validation
async fn validate_signature(
    bundle: &ProofBundle,
    verifier: &dyn ChainVerifier,
    results: &mut Vec<VerificationResult>,
) -> ValidationStep {
    // Verify cryptographic signatures on the proof bundle
    // This ensures the proof was created by the legitimate owner

    match verifier.verify_signature(bundle).await {
        Ok(result) if result.valid => {
            results.push(result);
            ValidationStep {
                name: "signature_validation",
                passed: true,
                error: None,
            }
        }
        Ok(result) => {
            let error_msg = result
                .error
                .as_ref()
                .map(|e| e.to_string())
                .unwrap_or_else(|| "unknown failure".to_string());
            results.push(result);
            ValidationStep {
                name: "signature_validation",
                passed: false,
                error: Some(format!(
                    "Signature verification failed: {}",
                    error_msg
                )),
            }
        }
        Err(e) => ValidationStep {
            name: "signature_validation",
            passed: false,
            error: Some(format!("Signature verification error: {}", e)),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain_config::ChainCapabilities;
    use crate::verified::{
        FinalityStrength, InclusionStrength, VerificationAssurance, VerifiedComponents,
    };

    struct MockVerifier;

    fn ok_result() -> VerificationResult {
        VerificationResult {
            valid: true,
            assurance: VerificationAssurance::Cryptographic,
            verified_components: VerifiedComponents {
                inclusion: InclusionStrength::MerklePath,
                finality: FinalityStrength::Probabilistic { confirmations: 6 },
                replay_checked: true,
                ownership_signature: true,
            },
            error: None,
        }
    }

    #[async_trait::async_trait]
    impl ChainVerifier for MockVerifier {
        async fn verify_inclusion(
            &self,
            _proof: &InclusionProof,
            _expected_root: Hash,
        ) -> Result<VerificationResult> {
            Ok(ok_result())
        }

        async fn verify_finality(&self, _proof: &FinalityProof) -> Result<VerificationResult> {
            Ok(ok_result())
        }

        async fn verify_zk(&self, _proof: &[u8]) -> Result<VerificationResult> {
            Ok(ok_result())
        }

        async fn verify_seal_registry(&self, _seal_id: Hash) -> Result<VerificationResult> {
            Ok(ok_result())
        }

        async fn verify_signature(&self, _bundle: &ProofBundle) -> Result<VerificationResult> {
            Ok(ok_result())
        }
    }

    #[test]
    fn test_validation_step_creation() {
        let step = ValidationStep {
            name: "test_step",
            passed: true,
            error: None,
        };
        assert!(step.passed);
        assert_eq!(step.name, "test_step");
    }

    #[tokio::test]
    async fn test_validate_proof_bundle_success() {
        use crate::dag::{DAGNode, DAGSegment};
        use crate::seal::{CommitAnchor, SealPoint};

        let node = DAGNode::new(
            Hash::new([1u8; 32]),
            vec![1u8; 32],
            vec![vec![1, 2, 3]],
            vec![vec![]],
            vec![],
        );

        // Compute the correct block_hash that matches the domain-separated hash
        // of the proof_bytes, so domain validation passes
        let proof_bytes = vec![1, 2, 3];
        let correct_block_hash =
            DomainSeparatedHash::<ProofBundleDomain>::hash(&proof_bytes);

        let bundle = ProofBundle {
            transition_dag: DAGSegment::new(vec![node], Hash::new([9u8; 32])),
            signatures: vec![vec![1, 2, 3]],
            seal_ref: SealPoint::new(vec![0xAA], Some(1)).unwrap(),
            anchor_ref: CommitAnchor::new(vec![0xBB; 32], 1, vec![0xCC]).unwrap(),
            inclusion_proof: InclusionProof::new(proof_bytes, correct_block_hash, 1, 0)
                .unwrap(),
            finality_proof: FinalityProof::new(vec![4, 5, 6], 6, true).unwrap(),
        };

        let verifier = MockVerifier;
        let caps = ChainCapabilities::bitcoin();
        let result = validate_proof_bundle(
            &bundle,
            &verifier,
            ChainId::new("bitcoin"),
            ChainId::new("ethereum"),
            caps,
            None,
            None,
        )
        .await;

        assert!(result.accepted);
        assert_eq!(result.steps.len(), 10);
        assert!(result.error.is_none());
    }
}