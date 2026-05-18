//! Sui ChainVerifier Implementation
//!
//! This module implements the ChainVerifier trait for Sui,
//! providing chain-specific verification logic for the canonical proof pipeline.

use async_trait::async_trait;
use csv_core::proof::{FinalityProof, InclusionProof, ProofBundle};
use csv_core::proof_pipeline::ChainVerifier;
use csv_core::signature::{Signature, SignatureScheme, verify_signatures};
use csv_core::Hash;
use csv_core::verified::{
    FinalityStrength, InclusionStrength, VerificationAssurance, VerificationFailure,
    VerificationResult, VerifiedComponents,
};

use crate::rpc::SuiRpc;

/// Sui verifier implementing ChainVerifier trait
pub struct SuiVerifier {
    /// RPC client for Sui
    rpc: Box<dyn SuiRpc>,
}

impl SuiVerifier {
    /// Create a new Sui verifier
    pub fn new(rpc: Box<dyn SuiRpc>) -> Self {
        Self { rpc }
    }
}

#[async_trait]
impl ChainVerifier for SuiVerifier {
    /// Verify inclusion proof for a Sui transaction
    ///
    /// Verifies that the transaction was included in a Sui checkpoint.
    /// The proof contains the Merkle path from the transaction to the
    /// checkpoint accumulator root.
    async fn verify_inclusion(
        &self,
        proof: &InclusionProof,
        expected_root: Hash,
    ) -> csv_core::Result<VerificationResult> {
        if proof.proof_bytes.is_empty() {
            return Ok(VerificationResult {
                valid: false,
                assurance: VerificationAssurance::Structural,
                verified_components: VerifiedComponents {
                    inclusion: InclusionStrength::None,
                    finality: FinalityStrength::None,
                    replay_checked: false,
                    ownership_signature: false,
                },
                error: Some(VerificationFailure::MissingData(
                    "Inclusion proof bytes are empty".to_string(),
                )),
            });
        }

        // Parse the Sui inclusion proof format:
        // [num_siblings (4 bytes LE)] [sibling_hashes...] [tx_digest (32 bytes)]
        use sha2::{Digest, Sha256};

        let proof_bytes = &proof.proof_bytes;
        if proof_bytes.len() < 4 {
            return Ok(VerificationResult {
                valid: false,
                assurance: VerificationAssurance::Structural,
                verified_components: VerifiedComponents {
                    inclusion: InclusionStrength::None,
                    finality: FinalityStrength::None,
                    replay_checked: false,
                    ownership_signature: false,
                },
                error: Some(VerificationFailure::MissingData(
                    "Inclusion proof bytes too short".to_string(),
                )),
            });
        }

        let num_siblings = u32::from_le_bytes([
            proof_bytes[0],
            proof_bytes[1],
            proof_bytes[2],
            proof_bytes[3],
        ]) as usize;

        let siblings_section_end = 4 + num_siblings * 32;
        if proof_bytes.len() < siblings_section_end {
            return Ok(VerificationResult {
                valid: false,
                assurance: VerificationAssurance::Structural,
                verified_components: VerifiedComponents {
                    inclusion: InclusionStrength::None,
                    finality: FinalityStrength::None,
                    replay_checked: false,
                    ownership_signature: false,
                },
                error: Some(VerificationFailure::InvalidMerklePath),
            });
        }

        // Extract the transaction digest (last 32 bytes after siblings)
        let tx_digest = &proof_bytes[siblings_section_end..siblings_section_end + 32];
        if tx_digest.len() < 32 {
            return Ok(VerificationResult {
                valid: false,
                assurance: VerificationAssurance::Structural,
                verified_components: VerifiedComponents {
                    inclusion: InclusionStrength::None,
                    finality: FinalityStrength::None,
                    replay_checked: false,
                    ownership_signature: false,
                },
                error: Some(VerificationFailure::InvalidMerklePath),
            });
        }

        // Compute the leaf hash: H(tx_digest || block_number)
        let mut leaf_hasher = Sha256::new();
        leaf_hasher.update(b"SUI::CHECKPOINT::LEAF");
        leaf_hasher.update(tx_digest);
        leaf_hasher.update(proof.block_number.to_le_bytes());
        let mut current_hash = leaf_hasher.finalize();

        // Walk the Merkle path using siblings
        for i in 0..num_siblings {
            let sibling_start = 4 + i * 32;
            let sibling_end = sibling_start + 32;
            let sibling = &proof_bytes[sibling_start..sibling_end];

            let mut combined = Sha256::new();
            if (proof.position >> i) & 1 == 0 {
                combined.update(&current_hash);
                combined.update(sibling);
            } else {
                combined.update(sibling);
                combined.update(&current_hash);
            }
            current_hash = combined.finalize();
        }

        // The computed root should match the expected root
        let computed_root = Hash::new(current_hash.into());
        if computed_root == expected_root {
            Ok(VerificationResult {
                valid: true,
                assurance: VerificationAssurance::Cryptographic,
                verified_components: VerifiedComponents {
                    inclusion: InclusionStrength::MerklePath,
                    finality: FinalityStrength::None,
                    replay_checked: false,
                    ownership_signature: false,
                },
                error: None,
            })
        } else {
            Ok(VerificationResult {
                valid: false,
                assurance: VerificationAssurance::PartialCryptographic,
                verified_components: VerifiedComponents {
                    inclusion: InclusionStrength::None,
                    finality: FinalityStrength::None,
                    replay_checked: false,
                    ownership_signature: false,
                },
                error: Some(VerificationFailure::InvalidMerklePath),
            })
        }
    }

    /// Verify finality proof for a Sui block
    ///
    /// Sui has instant finality via Narwhal/Bullshark consensus.
    /// Check if the checkpoint has sufficient confirmations.
    async fn verify_finality(&self, proof: &FinalityProof) -> csv_core::Result<VerificationResult> {
        // Sui has instant finality, so 1 confirmation is sufficient
        let required_confirmations = 1;

        if proof.confirmations < required_confirmations {
            return Ok(VerificationResult {
                valid: false,
                assurance: VerificationAssurance::Structural,
                verified_components: VerifiedComponents {
                    inclusion: InclusionStrength::None,
                    finality: FinalityStrength::None,
                    replay_checked: false,
                    ownership_signature: false,
                },
                error: Some(VerificationFailure::FinalityNotReached {
                    required: required_confirmations,
                    actual: proof.confirmations,
                }),
            });
        }

        if proof.finality_data.is_empty() {
            return Ok(VerificationResult {
                valid: false,
                assurance: VerificationAssurance::Structural,
                verified_components: VerifiedComponents {
                    inclusion: InclusionStrength::None,
                    finality: FinalityStrength::None,
                    replay_checked: false,
                    ownership_signature: false,
                },
                error: Some(VerificationFailure::MissingData(
                    "Finality data is empty".to_string(),
                )),
            });
        }

        Ok(VerificationResult {
            valid: true,
            assurance: VerificationAssurance::ConsensusBound,
            verified_components: VerifiedComponents {
                inclusion: InclusionStrength::None,
                finality: FinalityStrength::Deterministic,
                replay_checked: false,
                ownership_signature: false,
            },
            error: None,
        })
    }

    /// Verify zero-knowledge proof (if applicable)
    ///
    /// Sui doesn't use ZK proofs for basic operations.
    async fn verify_zk(&self, proof: &[u8]) -> csv_core::Result<VerificationResult> {
        if !proof.is_empty() {
            return Ok(VerificationResult {
                valid: false,
                assurance: VerificationAssurance::Structural,
                verified_components: VerifiedComponents {
                    inclusion: InclusionStrength::None,
                    finality: FinalityStrength::None,
                    replay_checked: false,
                    ownership_signature: false,
                },
                error: Some(VerificationFailure::UnsupportedCapability(
                    "ZK proofs are not supported for Sui operations. Set zk_proof_data to empty for Sui transactions."
                        .to_string(),
                )),
            });
        }
        Ok(VerificationResult {
            valid: true,
            assurance: VerificationAssurance::PartialCryptographic,
            verified_components: VerifiedComponents {
                inclusion: InclusionStrength::None,
                finality: FinalityStrength::None,
                replay_checked: false,
                ownership_signature: false,
            },
            error: None,
        })
    }

    /// Verify seal registry (check if seal has been consumed)
    ///
    /// Queries the Sui blockchain via RPC to check if the object
    /// associated with the seal_id has been deleted/consumed.
    async fn verify_seal_registry(&self, seal_id: Hash) -> csv_core::Result<VerificationResult> {
        // Convert seal_id to a Sui object ID (32 bytes)
        let object_id = seal_id.as_bytes();

        // Query the Sui RPC to get the object at this address
        match self.rpc.get_object(object_id).await {
            Ok(Some(object)) => {
                // Object exists - check if it's been consumed (deleted)
                // In Sui, a consumed object has owner == None or is deleted
                let is_consumed = object
                    .get("owner")
                    .and_then(|o| o.as_str())
                    .map(|o| o == "Deleted" || o.is_empty())
                    .unwrap_or(false);

                if is_consumed {
                    // Object has been consumed - seal is NOT available
                    Ok(VerificationResult {
                        valid: false,
                        assurance: VerificationAssurance::Structural,
                        verified_components: VerifiedComponents {
                            inclusion: InclusionStrength::None,
                            finality: FinalityStrength::None,
                            replay_checked: true,
                            ownership_signature: false,
                        },
                        error: Some(VerificationFailure::ReplayDetected),
                    })
                } else {
                    // Object exists and is not consumed - seal is available
                    Ok(VerificationResult {
                        valid: true,
                        assurance: VerificationAssurance::PartialCryptographic,
                        verified_components: VerifiedComponents {
                            inclusion: InclusionStrength::None,
                            finality: FinalityStrength::None,
                            replay_checked: true,
                            ownership_signature: false,
                        },
                        error: None,
                    })
                }
            }
            Ok(None) => {
                // Object doesn't exist - seal was never created or was consumed
                log::warn!(
                    "Sui object not found: {}",
                    hex::encode(object_id)
                );
                Ok(VerificationResult {
                    valid: false,
                    assurance: VerificationAssurance::Structural,
                    verified_components: VerifiedComponents {
                        inclusion: InclusionStrength::None,
                        finality: FinalityStrength::None,
                        replay_checked: true,
                        ownership_signature: false,
                    },
                    error: Some(VerificationFailure::ReplayDetected),
                })
            }
            Err(e) => {
                log::error!(
                    "Failed to query Sui seal registry for seal {}: {}",
                    hex::encode(object_id),
                    e
                );
                Err(csv_core::ProtocolError::NetworkError(format!(
                    "Failed to query seal registry: {}",
                    e
                )))
            }
        }
    }

    /// Verify signature on proof bundle
    ///
    /// Parses signatures from the proof bundle and verifies them
    /// using the Sui Ed25519 signature scheme.
    async fn verify_signature(&self, bundle: &ProofBundle) -> csv_core::Result<VerificationResult> {
        if bundle.signatures.is_empty() {
            return Ok(VerificationResult {
                valid: false,
                assurance: VerificationAssurance::Structural,
                verified_components: VerifiedComponents {
                    inclusion: InclusionStrength::None,
                    finality: FinalityStrength::None,
                    replay_checked: false,
                    ownership_signature: false,
                },
                error: Some(VerificationFailure::InvalidOwnershipSignature),
            });
        }

        // Parse signatures from the bundle
        let mut signatures = Vec::with_capacity(bundle.signatures.len());

        for (i, sig_bytes) in bundle.signatures.iter().enumerate() {
            if sig_bytes.len() < 4 {
                return Ok(VerificationResult {
                    valid: false,
                    assurance: VerificationAssurance::Structural,
                    verified_components: VerifiedComponents {
                        inclusion: InclusionStrength::None,
                        finality: FinalityStrength::None,
                        replay_checked: false,
                        ownership_signature: false,
                    },
                    error: Some(VerificationFailure::InvalidOwnershipSignature),
                });
            }

            let pk_len =
                u32::from_le_bytes([sig_bytes[0], sig_bytes[1], sig_bytes[2], sig_bytes[3]])
                    as usize;

            if sig_bytes.len() < 4 + pk_len {
                return Ok(VerificationResult {
                    valid: false,
                    assurance: VerificationAssurance::Structural,
                    verified_components: VerifiedComponents {
                        inclusion: InclusionStrength::None,
                        finality: FinalityStrength::None,
                        replay_checked: false,
                        ownership_signature: false,
                    },
                    error: Some(VerificationFailure::InvalidOwnershipSignature),
                });
            }

            let public_key = sig_bytes[4..4 + pk_len].to_vec();
            let signature = sig_bytes[4 + pk_len..].to_vec();

            // The signed message is the DAG root commitment
            let message = bundle.transition_dag.root_commitment.as_bytes().to_vec();

            signatures.push(Signature::new(signature, public_key, message));
        }

        // Sui primarily uses Ed25519 for signatures
        verify_signatures(&signatures, SignatureScheme::Ed25519)
            .map_err(|e| csv_core::ProtocolError::SignatureVerificationFailed(e.to_string()))?;

        Ok(VerificationResult {
            valid: true,
            assurance: VerificationAssurance::PartialCryptographic,
            verified_components: VerifiedComponents {
                inclusion: InclusionStrength::None,
                finality: FinalityStrength::None,
                replay_checked: false,
                ownership_signature: true,
            },
            error: None,
        })
    }
}