//! Core AnchorLayer trait
//!
//! This trait defines the interface that all chain-specific adapters must implement.

use crate::dag::DAGSegment;
use crate::error::Result;
use crate::hash::Hash;
use crate::proof::ProofBundle;
use crate::signature::SignatureScheme;

/// The AnchorLayer trait defines the interface for chain-specific adapters.
///
/// Implementors must provide:
/// - Seal creation and management
/// - Commitment publication
/// - Inclusion proof extraction
/// - Finality verification
/// - Seal enforcement (anti-replay)
pub trait AnchorLayer {
    /// Chain-specific seal reference type
    type SealRef;

    /// Chain-specific anchor reference type
    type AnchorRef;

    /// Chain-specific inclusion proof type
    type InclusionProof;

    /// Chain-specific finality proof type
    type FinalityProof;

    /// Publish a commitment under a single-use seal.
    ///
    /// Returns a reference that can be used for inclusion/finality proofs.
    ///
    /// # Arguments
    /// * `commitment` - The commitment hash to publish
    /// * `seal` - The seal reference authorizing this commitment
    fn publish(&self, commitment: Hash, seal: Self::SealRef) -> Result<Self::AnchorRef>;

    /// Extract inclusion proof from the base layer.
    ///
    /// # Arguments
    /// * `anchor` - The anchor reference to verify
    fn verify_inclusion(&self, anchor: Self::AnchorRef) -> Result<Self::InclusionProof>;

    /// Verify finality according to base-layer rules.
    ///
    /// # Arguments
    /// * `anchor` - The anchor reference to verify finality for
    fn verify_finality(&self, anchor: Self::AnchorRef) -> Result<Self::FinalityProof>;

    /// Enforce that the seal is single-use and non-replayable.
    ///
    /// # Arguments
    /// * `seal` - The seal reference to enforce
    fn enforce_seal(&self, seal: Self::SealRef) -> Result<()>;

    /// Create a new seal for authorizing state transitions.
    ///
    /// # Arguments
    /// * `value` - Optional value/funding for the seal (chain-specific units)
    fn create_seal(&self, value: Option<u64>) -> Result<Self::SealRef>;

    /// Compute a commitment hash from components.
    ///
    /// # Arguments
    /// * `contract_id` - Unique contract identifier
    /// * `previous_commitment` - Previous commitment hash
    /// * `transition_payload_hash` - Hash of the transition payload
    /// * `seal_ref` - Seal reference
    fn hash_commitment(
        &self,
        contract_id: Hash,
        previous_commitment: Hash,
        transition_payload_hash: Hash,
        seal_ref: &Self::SealRef,
    ) -> Hash;

    /// Build a complete proof bundle for peer-to-peer verification.
    ///
    /// # Arguments
    /// * `anchor` - The anchor reference
    /// * `transition_dag` - The state transition DAG segment
    fn build_proof_bundle(
        &self,
        anchor: Self::AnchorRef,
        transition_dag: DAGSegment,
    ) -> Result<ProofBundle>;

    /// Handle rollback of an anchor due to chain reorg.
    ///
    /// # Arguments
    /// * `anchor` - The anchor reference to invalidate
    fn rollback(&self, anchor: Self::AnchorRef) -> Result<()>;

    /// Get the domain separator for this adapter.
    fn domain_separator(&self) -> [u8; 32];

    /// Get the signature scheme used by this chain.
    ///
    /// This is used by the proof verification pipeline to select
    /// the appropriate cryptographic verification algorithm.
    fn signature_scheme(&self) -> SignatureScheme;
}
