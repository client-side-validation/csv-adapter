//! Bitcoin ChainVerifier Implementation
//!
//! This module implements the ChainVerifier trait for Bitcoin,
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

use crate::rpc::BitcoinRpc;

/// Bitcoin verifier implementing ChainVerifier trait
pub struct BitcoinVerifier {
    /// RPC client for Bitcoin
    rpc: Box<dyn BitcoinRpc + Send + Sync>,
}

impl BitcoinVerifier {
    /// Create a new Bitcoin verifier
    pub fn new(rpc: Box<dyn BitcoinRpc + Send + Sync>) -> Self {
        Self { rpc }
    }

    /// Convert a seal_id to an outpoint string for Bitcoin RPC queries
    fn seal_id_to_outpoint(&self, seal_id: Hash) -> csv_core::Result<(String, u32)> {
        // The seal_id encodes the outpoint as: [txid (32 bytes)] [vout (4 bytes LE)]
        let bytes = seal_id.as_bytes();
        if bytes.len() < 36 {
            return Err(csv_core::ProtocolError::InvalidInput(
                format!("Bitcoin seal_id too short: expected >= 36 bytes, got {}", bytes.len())
            ));
        }

        let txid = hex::encode(&bytes[..32]);
        let vout = u32::from_le_bytes(bytes[32..36].try_into().map_err(|_| {
            csv_core::ProtocolError::InvalidInput("Failed to read vout from seal_id".to_string())
        })?);

        Ok((txid, vout))
    }
}

#[async_trait]
impl ChainVerifier for BitcoinVerifier {
    /// Verify inclusion proof for a Bitcoin transaction
    async fn verify_inclusion(
        &self,
        proof: &InclusionProof,
        expected_root: Hash,
    ) -> csv_core::Result<VerificationResult> {
        use crate::proofs::{from_core_inclusion_proof, verify_merkle_proof};

        // Convert core inclusion proof to Bitcoin-specific type
        let bitcoin_proof = from_core_inclusion_proof(proof);

        // Extract txid from the proof (first 32 bytes of proof_bytes)
        if proof.proof_bytes.len() < 32 {
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
                    "Inclusion proof bytes too short for txid extraction".to_string(),
                )),
            });
        }
        let mut txid = [0u8; 32];
        txid.copy_from_slice(&proof.proof_bytes[..32]);

        // Use proper Merkle proof verification instead of checksum
        let merkle_root = if expected_root != Hash::zero() {
            expected_root.as_bytes()
        } else {
            proof.block_hash.as_bytes()
        };

        let merkle_ok = verify_merkle_proof(&txid, merkle_root, &bitcoin_proof);

        if merkle_ok {
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

    /// Verify finality proof for a Bitcoin block
    async fn verify_finality(&self, proof: &FinalityProof) -> csv_core::Result<VerificationResult> {
        let confirmations = proof.confirmations;
        // Use ChainCapabilities for finality depth - policy belongs to core, not adapter
        let required = csv_core::chain_config::ChainCapabilities::bitcoin().finality_depth;

        if confirmations < required {
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
                    required,
                    actual: confirmations,
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
                finality: FinalityStrength::Probabilistic { confirmations },
                replay_checked: false,
                ownership_signature: false,
            },
            error: None,
        })
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
                    "ZK proofs should use BitcoinSpvProver, not the verifier directly. For SPV verification without ZK, ensure zk_proof_data is empty."
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
    /// Queries the Bitcoin blockchain via RPC to check if the UTXO
    /// associated with the seal_id has been spent.
    async fn verify_seal_registry(&self, seal_id: Hash) -> csv_core::Result<VerificationResult> {
        // Parse the seal_id as a Bitcoin outpoint (txid:vout)
        let (txid_hex, vout) = self.seal_id_to_outpoint(seal_id)?;

        // Decode hex txid back to bytes for the RPC call
        let txid_bytes = hex::decode(&txid_hex)
            .map_err(|e| csv_core::ProtocolError::InvalidInput(
                format!("Failed to decode Bitcoin txid from seal_id: {}", e)
            ))?;

        let mut txid = [0u8; 32];
        let copy_len = txid_bytes.len().min(32);
        txid[..copy_len].copy_from_slice(&txid_bytes[..copy_len]);

        // Query the Bitcoin RPC to check if the UTXO is still unspent
        // The BitcoinRpc trait methods are synchronous, so we call directly.
        // This may block the current thread briefly during RPC calls.
        match self.rpc.is_utxo_unspent(txid, vout) {
            Ok(true) => {
                // UTXO exists and is unspent - seal is available
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
            Ok(false) => {
                // UTXO doesn't exist or was spent - seal is consumed
                log::warn!(
                    "Bitcoin UTXO not found (spent or never created): {}:{}",
                    txid_hex,
                    vout
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
                    "Failed to query Bitcoin UTXO {}:{}: {}",
                    txid_hex,
                    vout,
                    e
                );
                Err(csv_core::ProtocolError::NetworkError(format!(
                    "Failed to query Bitcoin seal registry: {}",
                    e
                )))
            }
        }
    }

    /// Verify signature on proof bundle
    ///
    /// Parses signatures from the proof bundle and verifies them
    /// using the Bitcoin secp256k1 signature scheme.
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

        // Bitcoin uses secp256k1 (ECDSA) for signatures
        verify_signatures(&signatures, SignatureScheme::Secp256k1)
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