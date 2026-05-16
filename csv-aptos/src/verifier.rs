//! Aptos ChainVerifier Implementation
//!
//! This module implements the ChainVerifier trait for Aptos,
//! providing chain-specific verification logic for the canonical proof pipeline.

use async_trait::async_trait;
use csv_core::Hash;
use csv_core::proof::{FinalityProof, InclusionProof, ProofBundle};
use csv_core::proof_pipeline::ChainVerifier;
use csv_core::signature::{Signature, SignatureScheme, verify_signatures};

use crate::proofs::StateProofVerifier;
use crate::rpc::AptosRpc;

/// Aptos verifier implementing ChainVerifier trait
pub struct AptosVerifier {
    /// RPC client for Aptos
    rpc: Box<dyn AptosRpc>,
}

impl AptosVerifier {
    /// Create a new Aptos verifier
    pub fn new(rpc: Box<dyn AptosRpc>) -> Self {
        Self { rpc }
    }
}

#[async_trait]
impl ChainVerifier for AptosVerifier {
    /// Verify inclusion proof for an Aptos transaction
    ///
    /// Verifies that the accumulator proof validates against the expected state root.
    /// Uses the Aptos accumulator structure where proof_bytes contain the Merkle path.
    async fn verify_inclusion(
        &self,
        proof: &InclusionProof,
        expected_root: Hash,
    ) -> csv_core::Result<bool> {
        // Validate proof bytes are present
        if proof.proof_bytes.is_empty() {
            return Ok(false);
        }

        // Compute the leaf hash from the proof data
        // The proof format is: [num_siblings (4 bytes LE)] [sibling_hashes...] [leaf_data...]
        use sha2::{Digest, Sha256};

        // Parse proof structure
        let proof_bytes = &proof.proof_bytes;
        if proof_bytes.len() < 4 {
            return Ok(false);
        }

        let num_siblings = u32::from_le_bytes([
            proof_bytes[0],
            proof_bytes[1],
            proof_bytes[2],
            proof_bytes[3],
        ]) as usize;

        // Each sibling is 32 bytes (SHA-256 hash)
        let siblings_section_end = 4 + num_siblings * 32;
        if proof_bytes.len() < siblings_section_end {
            return Ok(false);
        }

        // Extract leaf data (starting after siblings)
        let leaf_data = &proof_bytes[siblings_section_end..];

        // Compute leaf hash from leaf data and block position
        let mut leaf_hasher = Sha256::new();
        leaf_hasher.update(b"APTOS::ACCUMULATOR::LEAF");
        leaf_hasher.update(leaf_data);
        leaf_hasher.update(proof.block_number.to_le_bytes());
        let leaf_hash = leaf_hasher.finalize();

        // Walk the Merkle path using siblings
        let mut current_hash = leaf_hash;
        for i in 0..num_siblings {
            let sibling_start = 4 + i * 32;
            let sibling_end = sibling_start + 32;
            let sibling = &proof_bytes[sibling_start..sibling_end];

            let mut combined = Sha256::new();
            // Convention: left child first, then right child
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
        Ok(computed_root == expected_root)
    }

    /// Verify finality proof for an Aptos block
    ///
    /// Aptos has instant finality via HotStuff consensus.
    /// For Aptos, any block that has >0 confirmations is considered finalized.
    async fn verify_finality(&self, proof: &FinalityProof) -> csv_core::Result<bool> {
        // Aptos has instant finality via HotStuff consensus
        // Check that confirmations are above the required threshold
        // Aptos requires at least 1 confirmation (instant finality)
        let required_confirmations = 1;

        if proof.confirmations >= required_confirmations {
            // Verify finality data is present
            if proof.finality_data.is_empty() {
                return Ok(false);
            }

            // In production, the finality_data would contain validator signatures
            // that prove the block was committed by the validator set.
            // For now, we trust the confirmation count + RPC proof.
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Verify zero-knowledge proof (if applicable)
    ///
    /// Aptos doesn't use ZK proofs for basic operations by default.
    /// If proof data is non-empty, it means the caller is requesting
    /// ZK verification, which Aptos currently doesn't support.
    async fn verify_zk(&self, proof: &[u8]) -> csv_core::Result<bool> {
        // Aptos doesn't use ZK proofs for basic operations
        // If ZK proof data is provided, return an error explaining this
        if !proof.is_empty() {
            return Err(csv_core::ProtocolError::VerificationFailed(
                "ZK proofs are not supported for Aptos operations. \
                 Set zk_proof_data to empty for Aptos transactions."
                    .to_string(),
            ));
        }

        // No ZK proof needed for Aptos - pass through
        Ok(true)
    }

    /// Verify seal registry (check if seal has been consumed)
    ///
    /// Queries the Aptos blockchain via RPC to check if the resource
    /// associated with the seal_id has been consumed/destroyed.
    /// Uses StateProofVerifier::verify_resource_exists_async for the actual query.
    async fn verify_seal_registry(&self, seal_id: Hash) -> csv_core::Result<bool> {
        // Convert seal_id to an Aptos address (32 bytes)
        let address = {
            let mut addr = [0u8; 32];
            let bytes = seal_id.as_bytes();
            let copy_len = bytes.len().min(32);
            addr[..copy_len].copy_from_slice(&bytes[..copy_len]);
            addr
        };

        // The seal resource type in the CSV module
        let resource_type = format!(
            "0x{}::csv_seal::Seal",
            hex::encode(&address[..16])
        );

        // Use the existing StateProofVerifier to check resource existence
        // If the resource exists and is NOT consumed, the seal is available
        match StateProofVerifier::verify_resource_exists_async(address, &resource_type, &*self.rpc).await {
            Ok(true) => {
                // Resource exists - check if it's been consumed
                // For the seal registry: return true means seal is NOT consumed (available)
                // We need to check the resource data for consumption status
                match self.rpc.get_resource(address, &resource_type, None).await {
                    Ok(Some(resource)) => {
                        // Check if the resource data indicates consumption
                        // The CSV seal resource stores a consumed flag
                        // For now, if resource exists, the seal is unconsumed (available)
                        Ok(true)
                    }
                    Ok(None) => {
                        // Resource disappeared - must have been consumed
                        Ok(false)
                    }
                    Err(e) => {
                        log::error!("Failed to fetch Aptos seal resource details: {}", e);
                        Err(csv_core::ProtocolError::NetworkError(format!(
                            "Failed to query seal registry: {}",
                            e
                        )))
                    }
                }
            }
            Ok(false) => {
                // Resource doesn't exist - seal was never created or was consumed
                log::warn!(
                    "Seal resource not found at address {} with type {}",
                    hex::encode(address),
                    resource_type
                );
                Ok(false)
            }
            Err(e) => {
                // RPC error - fail closed (assume seal consumed to be safe)
                log::error!(
                    "Failed to query Aptos seal registry for seal {}: {}",
                    hex::encode(seal_id.as_bytes()),
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
    /// using the Aptos Ed25519 signature scheme.
    async fn verify_signature(&self, bundle: &ProofBundle) -> csv_core::Result<bool> {
        if bundle.signatures.is_empty() {
            return Err(csv_core::ProtocolError::SignatureVerificationFailed(
                "No signatures in proof bundle".to_string(),
            ));
        }

        // Parse signatures from the bundle
        let mut signatures = Vec::with_capacity(bundle.signatures.len());

        for (i, sig_bytes) in bundle.signatures.iter().enumerate() {
            // Parse signature format: [pk_len (4 bytes LE)] [public_key] [signature]
            if sig_bytes.len() < 4 {
                return Err(csv_core::ProtocolError::SignatureVerificationFailed(
                    format!("Signature {} too short for header", i),
                ));
            }

            let pk_len =
                u32::from_le_bytes([sig_bytes[0], sig_bytes[1], sig_bytes[2], sig_bytes[3]])
                    as usize;

            if sig_bytes.len() < 4 + pk_len {
                return Err(csv_core::ProtocolError::SignatureVerificationFailed(
                    format!("Signature {} too short for public key", i),
                ));
            }

            let public_key = sig_bytes[4..4 + pk_len].to_vec();
            let signature = sig_bytes[4 + pk_len..].to_vec();

            // The signed message is the DAG root commitment
            let message = bundle.transition_dag.root_commitment.as_bytes().to_vec();

            signatures.push(Signature::new(signature, public_key, message));
        }

        // Verify all signatures using Ed25519 (Aptos's signature scheme)
        verify_signatures(&signatures, SignatureScheme::Ed25519)
            .map_err(|e| csv_core::ProtocolError::SignatureVerificationFailed(e.to_string()))?;

        Ok(true)
    }
}
