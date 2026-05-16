//! Solana ChainVerifier Implementation
//!
//! This module implements the ChainVerifier trait for Solana,
//! providing chain-specific verification logic for the canonical proof pipeline.

use async_trait::async_trait;
use csv_core::Hash;
use csv_core::proof::{FinalityProof, InclusionProof};
use csv_core::proof_pipeline::ChainVerifier;

use crate::proofs::verify_inclusion_proof;
use crate::rpc::SolanaRpc;

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
}

#[async_trait]
impl ChainVerifier for SolanaVerifier {
    /// Verify inclusion proof for a Solana transaction
    async fn verify_inclusion(
        &self,
        proof: &InclusionProof,
        expected_root: Hash,
    ) -> csv_core::Result<bool> {
        // Use the existing Solana slot proof verification logic
        Ok(verify_inclusion_proof(proof, &expected_root))
    }

    /// Verify finality proof for a Solana block
    async fn verify_finality(&self, proof: &FinalityProof) -> csv_core::Result<bool> {
        // Solana has probabilistic finality - check confirmations
        // Require at least 32 confirmations for Solana finality
        let required_confirmations = 32;
        let is_finalized = proof.confirmations >= required_confirmations;

        Ok(is_finalized)
    }

    /// Verify zero-knowledge proof (if applicable)
    async fn verify_zk(&self, proof: &[u8]) -> csv_core::Result<bool> {
        // Solana doesn't use ZK proofs for basic operations
        // Return true if proof is empty, otherwise verify if needed
        if proof.is_empty() {
            Ok(true)
        } else {
            // Placeholder - would implement actual ZK proof verification if needed
            Ok(true)
        }
    }

    /// Verify seal registry (check if seal has been consumed)
    async fn verify_seal_registry(&self, _seal_id: Hash) -> csv_core::Result<bool> {
        // Placeholder - would query Solana blockchain to check if account is consumed
        Ok(true)
    }

    /// Verify signature on proof bundle
    async fn verify_signature(
        &self,
        _bundle: &csv_core::proof::ProofBundle,
    ) -> csv_core::Result<bool> {
        // Placeholder - would verify signature on proof bundle
        Ok(true)
    }
}
