//! Proof bundle types for off-chain verification
//!
//! Proof bundles are exchanged between peers for verification.

#![allow(missing_docs)]

use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

use crate::canonical::to_canonical_cbor;
use crate::dag::DAGSegment;
use crate::hash::Hash;
use crate::seal::{CommitAnchor, SealPoint};

/// Explicit proof lifecycle stages. A proof may only advance forward.
/// No transfer may mint unless the phase reaches `ConsensusBound`.
/// Authorization for mint is determined by `VerificationResult::meets_chain_thresholds`,
/// not by comparing this enum to `ConsensusBound` directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ProofPhase {
    Constructed = 0,
    StructuralValidated = 1,
    CryptographicallyValidated = 2,
    FinalityValidated = 3,
    ReplayChecked = 4,
    ConsensusBound = 5,
}

/// Globally unique transfer identity. Prevents replay across process restarts
/// and across chain reorganizations.
///
/// Every transfer MUST derive a ReplayId before any state transition.
/// The replay database is append-only; a ReplayId already present means
/// the transfer has been seen before.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReplayId([u8; 32]);

impl ReplayId {
    /// Derive a ReplayId from all inputs that uniquely identify a transfer.
    /// The hash binds together source chain, transaction, seal, transition,
    /// and destination chain so that no two legitimate transfers share an ID.
    /// Uses canonical CBOR serialization + tagged hashing.
    pub fn derive(
        source_chain: &str,
        source_txid: &[u8],
        source_output_index: u32,
        seal_id: &[u8],
        transition_id: &[u8],
        destination_chain: &str,
    ) -> Self {
        #[derive(Serialize)]
        struct ReplayIdInputs<'a> {
            source_chain: &'a str,
            source_txid: &'a [u8],
            source_output_index: u32,
            seal_id: &'a [u8],
            transition_id: &'a [u8],
            destination_chain: &'a str,
        }
        let inputs = ReplayIdInputs {
            source_chain,
            source_txid,
            source_output_index,
            seal_id,
            transition_id,
            destination_chain,
        };
        let cbor = to_canonical_cbor(&inputs).unwrap_or_default();
        ReplayId(crate::tagged_hash::csv_tagged_hash("csv.replay-id.v1", &cbor))
    }

    /// Return the raw 32-byte replay ID.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

#[cfg(test)]
mod replay_id_tests {
    use super::*;

    #[test]
    fn test_replay_id_determinism() {
        let id1 = ReplayId::derive(
            "bitcoin",
            &[1u8; 32],
            0,
            &[2u8; 32],
            &[3u8; 32],
            "ethereum",
        );
        let id2 = ReplayId::derive(
            "bitcoin",
            &[1u8; 32],
            0,
            &[2u8; 32],
            &[3u8; 32],
            "ethereum",
        );
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_replay_id_uniqueness() {
        let id1 = ReplayId::derive(
            "bitcoin",
            &[1u8; 32],
            0,
            &[2u8; 32],
            &[3u8; 32],
            "ethereum",
        );
        let id2 = ReplayId::derive(
            "bitcoin",
            &[1u8; 32],
            0,
            &[2u8; 32],
            &[3u8; 32],
            "solana", // different destination
        );
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_replay_id_different_txid() {
        let id1 = ReplayId::derive(
            "bitcoin",
            &[1u8; 32],
            0,
            &[2u8; 32],
            &[3u8; 32],
            "ethereum",
        );
        let id2 = ReplayId::derive(
            "bitcoin",
            &[9u8; 32],
            0,
            &[2u8; 32],
            &[3u8; 32],
            "ethereum",
        );
        assert_ne!(id1, id2);
    }
}

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
    /// Block number containing the commitment
    pub block_number: u64,
    /// Position in block (for verification)
    pub position: u64,
}

impl InclusionProof {
    /// Create a new inclusion proof
    ///
    /// # Arguments
    /// * `proof_bytes` - Merkle proof or equivalent (max 64KB)
    /// * `block_hash` - Block hash containing the commitment
    /// * `block_number` - Block number containing the commitment
    /// * `position` - Position in block (for verification)
    ///
    /// # Errors
    /// Returns an error if proof_bytes exceeds the maximum allowed size
    pub fn new(
        proof_bytes: Vec<u8>,
        block_hash: Hash,
        block_number: u64,
        position: u64,
    ) -> Result<Self, &'static str> {
        if proof_bytes.len() > MAX_PROOF_BYTES {
            return Err("proof_bytes exceeds maximum allowed size (64KB)");
        }
        Ok(Self {
            proof_bytes,
            block_hash,
            block_number,
            position,
        })
    }

    /// Create a new inclusion proof without validation.
    ///
    /// # Safety
    /// The caller MUST ensure the proof_bytes are valid and non-empty.
    /// Violating this causes undefined behavior in proof verification.
    pub unsafe fn new_unchecked(
        proof_bytes: Vec<u8>,
        block_hash: Hash,
        block_number: u64,
        position: u64,
    ) -> Self {
        Self {
            proof_bytes,
            block_hash,
            block_number,
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
        if confirmations == 0 && !is_deterministic {
            return Err("Zero confirmations not allowed for probabilistic finality");
        }
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
    /// The caller MUST ensure the finality_data is valid for the target chain.
    pub unsafe fn new_unchecked(
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
    pub seal_ref: SealPoint,
    /// Anchor reference
    pub anchor_ref: CommitAnchor,
    /// Inclusion proof
    pub inclusion_proof: InclusionProof,
    /// Finality proof
    pub finality_proof: FinalityProof,
    /// Provenance metadata for tracking proof origin and verification chain
    pub provenance: Option<crate::provenance::ProofProvenance>,
    /// Deterministic certification for reproducible verification
    pub certification: Option<crate::certification::ProofCertification>,
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
        seal_ref: SealPoint,
        anchor_ref: CommitAnchor,
        inclusion_proof: InclusionProof,
        finality_proof: FinalityProof,
    ) -> Result<Self, &'static str> {
        Self::with_certification(
            transition_dag,
            signatures,
            seal_ref,
            anchor_ref,
            inclusion_proof,
            finality_proof,
            None,
            None,
        )
    }

    /// Create a new proof bundle with provenance metadata
    ///
    /// # Arguments
    /// * `transition_dag` - State transition DAG segment
    /// * `signatures` - Authorizing signatures (total max 1MB)
    /// * `seal_ref` - Seal reference
    /// * `anchor_ref` - Anchor reference
    /// * `inclusion_proof` - Inclusion proof
    /// * `finality_proof` - Finality proof
    /// * `provenance` - Optional provenance metadata
    ///
    /// # Errors
    /// Returns an error if signatures exceed the maximum total size
    pub fn with_provenance(
        transition_dag: DAGSegment,
        signatures: Vec<Vec<u8>>,
        seal_ref: SealPoint,
        anchor_ref: CommitAnchor,
        inclusion_proof: InclusionProof,
        finality_proof: FinalityProof,
        provenance: Option<crate::provenance::ProofProvenance>,
    ) -> Result<Self, &'static str> {
        Self::with_certification(
            transition_dag,
            signatures,
            seal_ref,
            anchor_ref,
            inclusion_proof,
            finality_proof,
            provenance,
            None,
        )
    }

    /// Create a new proof bundle with certification
    ///
    /// # Arguments
    /// * `transition_dag` - State transition DAG segment
    /// * `signatures` - Authorizing signatures (total max 1MB)
    /// * `seal_ref` - Seal reference
    /// * `anchor_ref` - Anchor reference
    /// * `inclusion_proof` - Inclusion proof
    /// * `finality_proof` - Finality proof
    /// * `provenance` - Optional provenance metadata
    /// * `certification` - Optional deterministic certification
    ///
    /// # Errors
    /// Returns an error if signatures exceed the maximum total size
    pub fn with_certification(
        transition_dag: DAGSegment,
        signatures: Vec<Vec<u8>>,
        seal_ref: SealPoint,
        anchor_ref: CommitAnchor,
        inclusion_proof: InclusionProof,
        finality_proof: FinalityProof,
        provenance: Option<crate::provenance::ProofProvenance>,
        certification: Option<crate::certification::ProofCertification>,
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
            provenance,
            certification,
        })
    }

    /// Set the provenance metadata
    pub fn set_provenance(&mut self, provenance: crate::provenance::ProofProvenance) {
        self.provenance = Some(provenance);
    }

    /// Get the provenance metadata
    pub fn provenance(&self) -> Option<&crate::provenance::ProofProvenance> {
        self.provenance.as_ref()
    }

    /// Check if the proof bundle has complete provenance
    pub fn has_complete_provenance(&self) -> bool {
        self.provenance
            .as_ref()
            .map(|p| p.is_verification_complete())
            .unwrap_or(false)
    }

    /// Set the certification metadata
    pub fn set_certification(&mut self, certification: crate::certification::ProofCertification) {
        self.certification = Some(certification);
    }

    /// Get the certification metadata
    pub fn certification(&self) -> Option<&crate::certification::ProofCertification> {
        self.certification.as_ref()
    }

    /// Check if the proof bundle has deterministic certification
    pub fn has_certification(&self) -> bool {
        self.certification.is_some()
    }

    /// Create a new $1 without validation.
    ///
    /// # Safety
    /// The caller MUST ensure all fields are valid and consistent.
    pub unsafe fn new_unchecked(
        transition_dag: DAGSegment,
        signatures: Vec<Vec<u8>>,
        seal_ref: SealPoint,
        anchor_ref: CommitAnchor,
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
            provenance: None,
            certification: None,
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
        let proof = InclusionProof::new(vec![1, 2, 3], Hash::zero(), 0, 0).unwrap();
        assert_eq!(proof.proof_bytes, vec![1, 2, 3]);
    }

    #[test]
    fn test_proof_bundle_without_provenance() {
        let proof = FinalityProof::new(vec![0xCD; 32], 6, false).unwrap();
        assert_eq!(proof.confirmations, 6);
        assert!(!proof.is_deterministic);
    }

    #[test]
    fn test_proof_bundle_serialization() {
        let bundle = ProofBundle::new(
            DAGSegment::new(vec![], Hash::zero()),
            vec![vec![0xAB; 64]],
            SealPoint::new(vec![1, 2, 3], Some(42)).unwrap(),
            CommitAnchor::new(vec![4, 5, 6], 100, vec![]).unwrap(),
            InclusionProof::new(vec![], Hash::zero(), 0, 0).unwrap(),
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
        let result = InclusionProof::new(large_proof, Hash::zero(), 0, 0);
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
            SealPoint::new(vec![1, 2, 3], Some(42)).unwrap(),
            CommitAnchor::new(vec![4, 5, 6], 100, vec![]).unwrap(),
            InclusionProof::new(vec![], Hash::zero(), 0, 0).unwrap(),
            FinalityProof::new(vec![], 6, false).unwrap(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_proof_bundle_provenance() {
        let mut bundle = ProofBundle::new(
            DAGSegment::new(vec![], Hash::zero()),
            vec![],
            SealPoint::new(vec![1, 2, 3], Some(42)).unwrap(),
            CommitAnchor::new(vec![4, 5, 6], 100, vec![]).unwrap(),
            InclusionProof::new(vec![], Hash::zero(), 0, 0).unwrap(),
            FinalityProof::new(vec![], 6, false).unwrap(),
        )
        .unwrap();

        assert!(bundle.provenance().is_none());
        assert!(!bundle.has_complete_provenance());

        let mut provenance = crate::provenance::ProofProvenance::new(
            "bitcoin".to_string(),
            1000,
            "runtime-1".to_string(),
            vec![1u8; 32],
        );

        provenance.add_verification_step(crate::provenance::VerificationStep::new(
            crate::provenance::VerificationStepType::ProofCreation,
            "adapter".to_string(),
            true,
        ));

        bundle.set_provenance(provenance);
        assert!(bundle.provenance().is_some());
        assert!(!bundle.has_complete_provenance()); // Not complete without all steps
    }

    #[test]
    fn test_proof_bundle_certification() {
        let mut bundle = ProofBundle::new(
            DAGSegment::new(vec![], Hash::zero()),
            vec![],
            SealPoint::new(vec![1, 2, 3], Some(42)).unwrap(),
            CommitAnchor::new(vec![4, 5, 6], 100, vec![]).unwrap(),
            InclusionProof::new(vec![], Hash::zero(), 0, 0).unwrap(),
            FinalityProof::new(vec![], 6, false).unwrap(),
        )
        .unwrap();

        assert!(bundle.certification().is_none());
        assert!(!bundle.has_certification());

        let inputs = crate::certification::VerificationInputs::new(
            vec![1u8; 32],
            vec![2u8; 32],
            vec![3u8; 32],
            crate::certification::ChainMetadata::new("bitcoin".to_string(), 1000, vec![4u8; 32]),
            crate::certification::RuntimePolicyConfig::new(6, false, 3, true),
        );

        let outputs = crate::certification::VerificationOutputs::new(
            true,
            "verified".to_string(),
            crate::certification::VerificationStrength::maximum(),
        );

        let certification = crate::certification::ProofCertification::new(
            "runtime-1".to_string(),
            inputs,
            outputs,
        );

        bundle.set_certification(certification);
        assert!(bundle.certification().is_some());
        assert!(bundle.has_certification());
    }
}
