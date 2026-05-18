//! Solana ChainVerifier Implementation
//!
//! This module implements the ChainVerifier trait for Solana,
//! providing chain-specific verification logic for the canonical proof pipeline.

use async_trait::async_trait;
use csv_core::Hash;
use csv_core::proof::{FinalityProof, InclusionProof, ProofBundle};
use csv_core::proof_pipeline::ChainVerifier;
use csv_core::signature::{Signature, SignatureScheme, verify_signatures};
use csv_core::verified::{
    FinalityStrength, InclusionStrength, VerificationAssurance, VerificationFailure,
    VerificationResult, VerifiedComponents,
};

use crate::proofs::verify_inclusion_proof;
use crate::rpc::SolanaRpc;
use solana_sdk::pubkey::Pubkey;

/// Solana verifier implementing ChainVerifier trait
pub struct SolanaVerifier {
    /// RPC client for Solana
    rpc: Box<dyn SolanaRpc>,
}

impl SolanaVerifier {
    /// Create a new Solana verifier
    pub fn new(rpc: Box<dyn SolanaRpc>) -> Self {
        Self { rpc }
    }

    /// Convert a seal_id Hash to a Solana Pubkey
    fn seal_id_to_pubkey(&self, seal_id: Hash) -> Pubkey {
        let bytes = seal_id.as_bytes();
        let mut pubkey_bytes = [0u8; 32];
        let copy_len = bytes.len().min(32);
        pubkey_bytes[..copy_len].copy_from_slice(&bytes[..copy_len]);
        Pubkey::new_from_array(pubkey_bytes)
    }
}

#[async_trait]
impl ChainVerifier for SolanaVerifier {
    /// Verify inclusion proof for a Solana transaction
    async fn verify_inclusion(
        &self,
        proof: &InclusionProof,
        expected_root: Hash,
    ) -> csv_core::Result<VerificationResult> {
        // Use the existing Solana slot proof verification logic
        let result = verify_inclusion_proof(proof, &expected_root);
        if result {
            Ok(VerificationResult {
                valid: true,
                assurance: VerificationAssurance::PartialCryptographic,
                verified_components: VerifiedComponents {
                    inclusion: InclusionStrength::AnchoredMerklePath,
                    finality: FinalityStrength::None,
                    replay_checked: false,
                    ownership_signature: false,
                },
                error: None,
            })
        } else {
            Ok(VerificationResult {
                valid: false,
                assurance: VerificationAssurance::Structural,
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

    /// Verify finality proof for a Solana block
    async fn verify_finality(&self, proof: &FinalityProof) -> csv_core::Result<VerificationResult> {
        // Solana has probabilistic finality - check confirmations
        let required_confirmations = 32;

        if proof.confirmations >= required_confirmations {
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
                assurance: VerificationAssurance::PartialCryptographic,
                verified_components: VerifiedComponents {
                    inclusion: InclusionStrength::None,
                    finality: FinalityStrength::Probabilistic {
                        confirmations: proof.confirmations,
                    },
                    replay_checked: false,
                    ownership_signature: false,
                },
                error: None,
            })
        } else {
            Ok(VerificationResult {
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
            })
        }
    }

    /// Verify zero-knowledge proof (if applicable)
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
                    "ZK proofs are not supported for Solana operations. Set zk_proof_data to empty for Solana transactions."
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
    /// Queries the Solana blockchain via RPC to check if the PDA account
    /// associated with the seal_id has been closed/consumed.
    async fn verify_seal_registry(&self, seal_id: Hash) -> csv_core::Result<VerificationResult> {
        // Convert the seal_id to a Pubkey (the PDA address of the seal)
        let pubkey = self.seal_id_to_pubkey(seal_id);

        // Query the Solana RPC to get the account at this pubkey
        match self.rpc.get_account(&pubkey) {
            Ok(account) => {
                // Account exists - check if it's been consumed
                // In Solana, a consumed (closed) PDA has:
                // - lamports == 0 (lamports transferred to sysvar)
                // - data.len() == 0 or owner == System program
                // System program ID: 11111111111111111111111111111111
                let system_program_id = Pubkey::from([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
                let is_consumed = account.lamports == 0
                    || account.data.is_empty()
                    || account.owner == system_program_id;

                if is_consumed {
                    // Seal has been consumed - not available
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
                    // Account exists with data - seal is available
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
            Err(e) => {
                // Check if the account simply doesn't exist (never created or was closed)
                // vs a genuine RPC error
                let err_str = e.to_string().to_lowercase();
                if err_str.contains("not found")
                    || err_str.contains("no account")
                    || err_str.contains("does not exist")
                    || err_str.contains("address not found")
                {
                    // Account doesn't exist on-chain — was never created or has been closed/consumed
                    log::warn!(
                        "Solana seal account not found for seal {}: {}",
                        hex::encode(seal_id.as_bytes()),
                        e
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
                } else {
                    // Genuine RPC error — fail loudly rather than returning a silent false
                    log::error!(
                        "Solana RPC error while querying seal registry for {}: {}",
                        hex::encode(seal_id.as_bytes()),
                        e
                    );
                    Err(csv_core::ProtocolError::NetworkError(format!(
                        "Failed to query Solana seal registry: {}",
                        e
                    )))
                }
            }
        }
    }

    /// Verify signature on proof bundle
    ///
    /// Parses signatures from the proof bundle and verifies them
    /// using the Solana Ed25519 signature scheme.
    async fn verify_signature(
        &self,
        bundle: &ProofBundle,
    ) -> csv_core::Result<VerificationResult> {
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

        for sig_bytes in bundle.signatures.iter() {
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
            let message = bundle.transition_dag.root_commitment.as_bytes().to_vec();

            signatures.push(Signature::new(signature, public_key, message));
        }

        // Solana primarily uses Ed25519 for signatures
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