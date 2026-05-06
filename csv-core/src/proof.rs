//! Proof bundle types for off-chain verification
//!
//! Proof bundles are exchanged between peers for verification.

use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

use crate::dag::DAGSegment;
use crate::hash::Hash;
use crate::seal::{AnchorRef, SealRef};

/// Maximum allowed size for proof bytes (64KB)
pub const MAX_PROOF_BYTES: usize = 64 * 1024;

/// Maximum allowed size for finality data (4KB)
pub const MAX_FINALITY_DATA: usize = 4 * 1024;

/// Maximum allowed size for signatures in a bundle (1MB total)
pub const MAX_SIGNATURES_TOTAL_SIZE: usize = 1024 * 1024;

/// Inclusion proof material
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InclusionProof {
    /// Merkle proof or equivalent
    pub proof_bytes: Vec<u8>,
    /// Block hash containing the commitment
    pub block_hash: Hash,
    /// Position in block (for verification)
    pub position: u64,
}

impl InclusionProof {
    /// Create a new inclusion proof
    ///
    /// # Arguments
    /// * `proof_bytes` - Merkle proof or equivalent (max 64KB)
    /// * `block_hash` - Block hash containing the commitment
    /// * `position` - Position in block (for verification)
    ///
    /// # Errors
    /// Returns an error if proof_bytes exceeds the maximum allowed size
    pub fn new(
        proof_bytes: Vec<u8>,
        block_hash: Hash,
        position: u64,
    ) -> Result<Self, &'static str> {
        if proof_bytes.len() > MAX_PROOF_BYTES {
            return Err("proof_bytes exceeds maximum allowed size (64KB)");
        }
        Ok(Self {
            proof_bytes,
            block_hash,
            position,
        })
    }

    /// Create a new inclusion proof without validation.
    ///
    /// # Safety
    /// This bypasses size and structure validation. Use only for internal protocol conversions
    /// where the input is already known to be valid.
    pub fn new_unchecked(proof_bytes: Vec<u8>, block_hash: Hash, position: u64) -> Self {
        Self {
            proof_bytes,
            block_hash,
            position,
        }
    }

    /// Check if confirmed with given depth
    ///
    /// Note: This default implementation returns false (fail closed).
    /// Chain adapters must implement their own confirmation logic using
    /// chain-specific finality rules. This prevents false positives in
    /// production where unconfirmed proofs could be accepted.
    pub fn is_confirmed(&self, _required_depth: u32) -> bool {
        false
    }
}

/// Finality proof material
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FinalityProof {
    /// Finality checkpoint or depth
    pub finality_data: Vec<u8>,
    /// Number of confirmations or equivalent
    pub confirmations: u64,
    /// Whether finality is deterministic (vs probabilistic)
    pub is_deterministic: bool,
}

impl FinalityProof {
    /// Create a new finality proof
    ///
    /// # Arguments
    /// * `finality_data` - Finality checkpoint or depth (max 4KB)
    /// * `confirmations` - Number of confirmations or equivalent
    /// * `is_deterministic` - Whether finality is deterministic (vs probabilistic)
    ///
    /// # Errors
    /// Returns an error if finality_data exceeds the maximum allowed size
    pub fn new(
        finality_data: Vec<u8>,
        confirmations: u64,
        is_deterministic: bool,
    ) -> Result<Self, &'static str> {
        if finality_data.len() > MAX_FINALITY_DATA {
            return Err("finality_data exceeds maximum allowed size (4KB)");
        }
        Ok(Self {
            finality_data,
            confirmations,
            is_deterministic,
        })
    }

    /// Create a new $1 without validation.
    ///
    /// # Safety
    /// This bypasses validation. Use only for internal protocol conversions.
    pub fn new_unchecked(
        finality_data: Vec<u8>,
        confirmations: u64,
        is_deterministic: bool,
    ) -> Self {
        Self {
            finality_data,
            confirmations,
            is_deterministic,
        }
    }
}

/// Complete proof bundle for peer-to-peer verification
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProofBundle {
    /// State transition DAG segment
    pub transition_dag: DAGSegment,
    /// Authorizing signatures
    pub signatures: Vec<Vec<u8>>,
    /// Seal reference
    pub seal_ref: SealRef,
    /// Anchor reference
    pub anchor_ref: AnchorRef,
    /// Inclusion proof
    pub inclusion_proof: InclusionProof,
    /// Finality proof
    pub finality_proof: FinalityProof,
}

impl ProofBundle {
    /// Create a new proof bundle
    ///
    /// # Arguments
    /// * `transition_dag` - State transition DAG segment
    /// * `signatures` - Authorizing signatures (total max 1MB)
    /// * `seal_ref` - Seal reference
    /// * `anchor_ref` - Anchor reference
    /// * `inclusion_proof` - Inclusion proof
    /// * `finality_proof` - Finality proof
    ///
    /// # Errors
    /// Returns an error if signatures exceed the maximum total size
    pub fn new(
        transition_dag: DAGSegment,
        signatures: Vec<Vec<u8>>,
        seal_ref: SealRef,
        anchor_ref: AnchorRef,
        inclusion_proof: InclusionProof,
        finality_proof: FinalityProof,
    ) -> Result<Self, &'static str> {
        // Validate total signature size
        let total_sig_size: usize = signatures.iter().map(|s| s.len()).sum();
        if total_sig_size > MAX_SIGNATURES_TOTAL_SIZE {
            return Err("total signatures size exceeds maximum allowed (1MB)");
        }
        Ok(Self {
            transition_dag,
            signatures,
            seal_ref,
            anchor_ref,
            inclusion_proof,
            finality_proof,
        })
    }

    /// Create a new $1 without validation.
    ///
    /// # Safety
    /// This bypasses validation. Use only for internal protocol conversions.
    pub fn new_unchecked(
        transition_dag: DAGSegment,
        signatures: Vec<Vec<u8>>,
        seal_ref: SealRef,
        anchor_ref: AnchorRef,
        inclusion_proof: InclusionProof,
        finality_proof: FinalityProof,
    ) -> Self {
        Self {
            transition_dag,
            signatures,
            seal_ref,
            anchor_ref,
            inclusion_proof,
            finality_proof,
        }
    }

    /// Serialize the proof bundle
    pub fn to_bytes(&self) -> Result<Vec<u8>, bincode::Error> {
        bincode::serialize(self)
    }

    /// Deserialize the proof bundle with size limit (10MB max)
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, bincode::Error> {
        const MAX_SIZE: usize = 10 * 1024 * 1024; // 10MB
        if bytes.len() > MAX_SIZE {
            return Err(bincode::ErrorKind::Custom(format!(
                "ProofBundle too large: {} bytes (max {})",
                bytes.len(),
                MAX_SIZE
            ))
            .into());
        }
        bincode::deserialize(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inclusion_proof_creation() {
        let proof = InclusionProof::new(vec![0xAB; 64], Hash::new([1u8; 32]), 42).unwrap();
        assert_eq!(proof.position, 42);
    }

    #[test]
    fn test_finality_proof_creation() {
        let proof = FinalityProof::new(vec![0xCD; 32], 6, false).unwrap();
        assert_eq!(proof.confirmations, 6);
        assert!(!proof.is_deterministic);
    }

    #[test]
    fn test_proof_bundle_serialization() {
        let bundle = ProofBundle::new(
            DAGSegment::new(vec![], Hash::zero()),
            vec![vec![0xAB; 64]],
            SealRef::new(vec![1, 2, 3], Some(42)).unwrap(),
            AnchorRef::new(vec![4, 5, 6], 100, vec![]).unwrap(),
            InclusionProof::new(vec![], Hash::zero(), 0).unwrap(),
            FinalityProof::new(vec![], 6, false).unwrap(),
        )
        .unwrap();

        let bytes = bundle.to_bytes().unwrap();
        let restored = ProofBundle::from_bytes(&bytes).unwrap();
        assert_eq!(bundle, restored);
    }

    #[test]
    fn test_inclusion_proof_too_large() {
        let large_proof = vec![0u8; MAX_PROOF_BYTES + 1];
        let result = InclusionProof::new(large_proof, Hash::zero(), 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_finality_proof_too_large() {
        let large_data = vec![0u8; MAX_FINALITY_DATA + 1];
        let result = FinalityProof::new(large_data, 6, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_proof_bundle_signatures_too_large() {
        let large_sigs = vec![vec![0u8; MAX_SIGNATURES_TOTAL_SIZE / 2 + 1]; 2];
        let result = ProofBundle::new(
            DAGSegment::new(vec![], Hash::zero()),
            large_sigs,
            SealRef::new(vec![1, 2, 3], Some(42)).unwrap(),
            AnchorRef::new(vec![4, 5, 6], 100, vec![]).unwrap(),
            InclusionProof::new(vec![], Hash::zero(), 0).unwrap(),
            FinalityProof::new(vec![], 6, false).unwrap(),
        );
        assert!(result.is_err());
    }
}
