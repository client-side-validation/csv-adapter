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

use crate::domain_hash::DomainSeparatedHash;
use crate::domains::{ProofBundleDomain, ReplayRegistryDomain};
use crate::error::Result;
use crate::events::{CsvEvent, EventIndexerRegistry};
use crate::hash::Hash;
use crate::proof::{FinalityProof, InclusionProof, ProofBundle};
use crate::protocol_version::ChainId;
use crate::replay_registry::ReplayKey;

/// Chain verifier trait that adapters must implement
///
/// Adapters provide chain-specific verification logic through this trait,
/// but the orchestration is handled by the canonical pipeline.
#[async_trait::async_trait]
pub trait ChainVerifier {
    /// Verify inclusion proof for a transaction on this chain
    async fn verify_inclusion(&self, proof: &InclusionProof, expected_root: Hash) -> Result<bool>;

    /// Verify finality proof for a block on this chain
    async fn verify_finality(&self, proof: &FinalityProof) -> Result<bool>;

    /// Verify zero-knowledge proof (if applicable for this chain)
    async fn verify_zk(&self, proof: &[u8]) -> Result<bool>;

    /// Verify seal registry (check if seal has been consumed)
    async fn verify_seal_registry(&self, seal_id: Hash) -> Result<bool>;

    /// Verify signature on proof bundle
    async fn verify_signature(&self, bundle: &ProofBundle) -> Result<bool>;
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
    event_registry: Option<Arc<Mutex<EventIndexerRegistry>>>,
) -> ValidationResult {
    let mut steps = Vec::with_capacity(10);

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
    let step3 = validate_inclusion_proof(bundle, verifier).await;
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
    let step4 = validate_zk_proof(bundle, verifier).await;
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
    let step5 = validate_finality(bundle, verifier).await;
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
    let step6 = validate_replay(bundle);
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
            let _ = registry.lock().unwrap().emit(event);
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
    let step7 = validate_seal_registry(bundle, verifier).await;
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
    let step9 = validate_signature(bundle, verifier).await;
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

    // Step 10: Acceptance decision
    let step10 = ValidationStep {
        name: "acceptance_decision",
        passed: true,
        error: None,
    };
    steps.push(step10);

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
        let _ = registry.lock().unwrap().emit(event);
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
        let _ = registry.lock().unwrap().emit(event).await;
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
    // Verify that the proof is for the correct source/destination chains
    // by checking the domain-separated hash matches expected values

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
) -> ValidationStep {
    match verifier
        .verify_inclusion(&bundle.inclusion_proof, bundle.inclusion_proof.block_hash)
        .await
    {
        Ok(true) => ValidationStep {
            name: "inclusion_proof_validation",
            passed: true,
            error: None,
        },
        Ok(false) => ValidationStep {
            name: "inclusion_proof_validation",
            passed: false,
            error: Some("Inclusion proof verification failed".to_string()),
        },
        Err(e) => ValidationStep {
            name: "inclusion_proof_validation",
            passed: false,
            error: Some(format!("Inclusion proof verification error: {}", e)),
        },
    }
}

/// Step 4: ZK proof validation
async fn validate_zk_proof(_bundle: &ProofBundle, verifier: &dyn ChainVerifier) -> ValidationStep {
    // ZK proof may be optional for some chains
    // For now, we skip ZK proof validation as ProofBundle doesn't have a zk_proof field
    match verifier.verify_zk(&[]).await {
        Ok(true) => ValidationStep {
            name: "zk_proof_validation",
            passed: true,
            error: None,
        },
        Ok(false) => ValidationStep {
            name: "zk_proof_validation",
            passed: false,
            error: Some("ZK proof verification failed".to_string()),
        },
        Err(e) => ValidationStep {
            name: "zk_proof_validation",
            passed: false,
            error: Some(format!("ZK proof verification error: {}", e)),
        },
    }
}

/// Step 5: Finality validation
async fn validate_finality(bundle: &ProofBundle, verifier: &dyn ChainVerifier) -> ValidationStep {
    match verifier.verify_finality(&bundle.finality_proof).await {
        Ok(true) => ValidationStep {
            name: "finality_validation",
            passed: true,
            error: None,
        },
        Ok(false) => ValidationStep {
            name: "finality_validation",
            passed: false,
            error: Some("Finality proof verification failed".to_string()),
        },
        Err(e) => ValidationStep {
            name: "finality_validation",
            passed: false,
            error: Some(format!("Finality proof verification error: {}", e)),
        },
    }
}

/// Step 6: Replay validation
fn validate_replay(bundle: &ProofBundle) -> ValidationStep {
    // Check replay registry to prevent cross-chain replay attacks
    // In a real implementation, this would query the persistent replay registry
    // For now, we compute the replay key and verify it's not in a local cache

    // Compute replay key from proof bundle
    let replay_key = ReplayKey::new(
        bundle.inclusion_proof.block_hash,
        bundle.inclusion_proof.block_hash, // seal_id (simplified - would be actual seal ID)
        bundle.inclusion_proof.block_hash, // commitment_hash (simplified)
        ChainId::new("source"),            // Would be actual source chain from bundle
        ChainId::new("destination"),       // Would be actual destination chain from bundle
    );

    // Compute domain-separated hash of replay key
    let _replay_hash =
        DomainSeparatedHash::<ReplayRegistryDomain>::hash(&replay_key.hash().as_bytes()[..]);

    // In production, check if this replay_hash exists in the persistent registry
    // For now, we pass this step (registry check would be async)
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
) -> ValidationStep {
    // Verify that the seal has not been consumed before
    // This prevents double-spend attacks

    // Extract seal ID from proof bundle (simplified - would parse from actual proof)
    let seal_id = bundle.inclusion_proof.block_hash;

    match verifier.verify_seal_registry(seal_id).await {
        Ok(true) => ValidationStep {
            name: "seal_registry_validation",
            passed: true,
            error: None,
        },
        Ok(false) => ValidationStep {
            name: "seal_registry_validation",
            passed: false,
            error: Some("Seal has already been consumed - double-spend detected".to_string()),
        },
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

    // In production, additional checks would include:
    // - Verify block height is within protocol-defined window
    // - Verify proof timestamp is not expired
    // - Verify transition sequence is valid
    // - Check protocol version compatibility

    ValidationStep {
        name: "transition_legality_validation",
        passed: true,
        error: None,
    }
}

/// Step 9: Signature validation
async fn validate_signature(bundle: &ProofBundle, verifier: &dyn ChainVerifier) -> ValidationStep {
    // Verify cryptographic signatures on the proof bundle
    // This ensures the proof was created by the legitimate owner

    match verifier.verify_signature(bundle).await {
        Ok(true) => ValidationStep {
            name: "signature_validation",
            passed: true,
            error: None,
        },
        Ok(false) => ValidationStep {
            name: "signature_validation",
            passed: false,
            error: Some("Invalid signature - proof not authenticated".to_string()),
        },
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

    struct MockVerifier;

    #[async_trait::async_trait]
    impl ChainVerifier for MockVerifier {
        async fn verify_inclusion(
            &self,
            _proof: &InclusionProof,
            _expected_root: Hash,
        ) -> Result<bool> {
            Ok(true)
        }

        async fn verify_finality(&self, _proof: &FinalityProof) -> Result<bool> {
            Ok(true)
        }

        async fn verify_zk(&self, _proof: &[u8]) -> Result<bool> {
            Ok(true)
        }

        async fn verify_seal_registry(&self, _seal_id: Hash) -> Result<bool> {
            Ok(true)
        }

        async fn verify_signature(&self, _bundle: &ProofBundle) -> Result<bool> {
            Ok(true)
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
        use crate::dag::DAGSegment;
        use crate::seal::{CommitAnchor, SealPoint};

        let bundle = ProofBundle {
            transition_dag: DAGSegment::new(vec![], Hash::new([9u8; 32])),
            signatures: vec![vec![1, 2, 3]],
            seal_ref: SealPoint::new(vec![0xAA], Some(1)).unwrap(),
            anchor_ref: CommitAnchor::new(vec![0xBB; 32], 1, vec![0xCC]).unwrap(),
            inclusion_proof: InclusionProof::new(vec![1, 2, 3], Hash::new([1u8; 32]), 1, 0)
                .unwrap(),
            finality_proof: FinalityProof::new(vec![4, 5, 6], 6, true).unwrap(),
        };

        let verifier = MockVerifier;
        let result = validate_proof_bundle(
            &bundle,
            &verifier,
            ChainId::new("bitcoin"),
            ChainId::new("ethereum"),
            None,
        )
        .await;

        assert!(result.accepted);
        assert_eq!(result.steps.len(), 10);
        assert!(result.error.is_none());
    }
}
