//! Sui ChainVerifier Implementation
//!
//! This module implements the ChainVerifier trait for Sui,
//! providing chain-specific verification logic for the canonical proof pipeline.

use async_trait::async_trait;
use csv_core::proof::{FinalityProof, InclusionProof, ProofBundle};
use csv_core::proof_pipeline::ChainVerifier;
use csv_core::signature::{Signature, SignatureScheme, verify_signatures};
use csv_core::Hash;

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
    ) -> csv_core::Result<bool> {
        if proof.proof_bytes.is_empty() {
            return Ok(false);
        }

        // Parse the Sui inclusion proof format:
        // [num_siblings (4 bytes LE)] [sibling_hashes...] [tx_digest (32 bytes)]
        use sha2::{Digest, Sha256};

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

        let siblings_section_end = 4 + num_siblings * 32;
        if proof_bytes.len() < siblings_section_end {
            return Ok(false);
        }

        // Extract the transaction digest (last 32 bytes after siblings)
        let tx_digest = &proof_bytes[siblings_section_end..siblings_section_end + 32];
        if tx_digest.len() < 32 {
            return Ok(false);
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
        Ok(computed_root == expected_root)
    }

    /// Verify finality proof for a Sui block
    ///
    /// Sui has instant finality via Narwhal/Bullshark consensus.
    /// Check if the checkpoint has sufficient confirmations.
    async fn verify_finality(&self, proof: &FinalityProof) -> csv_core::Result<bool> {
        // Sui has instant finality, so 1 confirmation is sufficient
        let required_confirmations = 1;

        if proof.confirmations >= required_confirmations {
            if proof.finality_data.is_empty() {
                return Ok(false);
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Verify zero-knowledge proof (if applicable)
    ///
    /// Sui doesn't use ZK proofs for basic operations.
    async fn verify_zk(&self, proof: &[u8]) -> csv_core::Result<bool> {
        if !proof.is_empty() {
            return Err(csv_core::ProtocolError::VerificationFailed(
                "ZK proofs are not supported for Sui operations. \
                 Set zk_proof_data to empty for Sui transactions."
                    .to_string(),
            ));
        }
        Ok(true)
    }

    /// Verify seal registry (check if seal has been consumed)
    ///
    /// Queries the Sui blockchain via RPC to check if the object
    /// associated with the seal_id has been deleted/consumed.
    async fn verify_seal_registry(&self, seal_id: Hash) -> csv_core::Result<bool> {
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
                    Ok(false)
                } else {
                    // Object exists and is not consumed - seal is available
                    Ok(true)
                }
            }
            Ok(None) => {
                // Object doesn't exist - seal was never created or was consumed
                log::warn!(
                    "Sui object not found: {}",
                    hex::encode(object_id)
                );
                Ok(false)
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
    async fn verify_signature(&self, bundle: &ProofBundle) -> csv_core::Result<bool> {
        if bundle.signatures.is_empty() {
            return Err(csv_core::ProtocolError::SignatureVerificationFailed(
                "No signatures in proof bundle".to_string(),
            ));
        }

        // Parse signatures from the bundle
        let mut signatures = Vec::with_capacity(bundle.signatures.len());

        for (i, sig_bytes) in bundle.signatures.iter().enumerate() {
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

        // Sui primarily uses Ed25519 for signatures
        verify_signatures(&signatures, SignatureScheme::Ed25519)
            .map_err(|e| csv_core::ProtocolError::SignatureVerificationFailed(e.to_string()))?;

        Ok(true)
    }
}