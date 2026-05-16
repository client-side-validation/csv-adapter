//! Bitcoin ChainVerifier Implementation
//!
//! This module implements the ChainVerifier trait for Bitcoin,
//! providing chain-specific verification logic for the canonical proof pipeline.

use async_trait::async_trait;
use csv_core::Hash;
use csv_core::proof::{FinalityProof, InclusionProof};
use csv_core::proof_pipeline::ChainVerifier;

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
}

#[async_trait]
impl ChainVerifier for BitcoinVerifier {
    /// Verify inclusion proof for a Bitcoin transaction
    async fn verify_inclusion(
        &self,
        proof: &InclusionProof,
        expected_root: Hash,
    ) -> csv_core::Result<bool> {
        use bitcoin_hashes::{Hash as BitcoinHash, sha256d};

        const PREFIX: &[u8] = b"CSV-BITCOIN-BLOCK-PROOF";
        let expected_len = PREFIX.len() + 8 + 32 + 32 + 32 + 32;
        if proof.proof_bytes.len() != expected_len || !proof.proof_bytes.starts_with(PREFIX) {
            return Ok(false);
        }

        let mut offset = PREFIX.len();
        let height = u64::from_le_bytes(proof.proof_bytes[offset..offset + 8].try_into().map_err(
            |_| csv_core::ProtocolError::InvalidInput("Invalid Bitcoin proof height".to_string()),
        )?);
        offset += 8;
        let txid = &proof.proof_bytes[offset..offset + 32];
        offset += 32;
        let commitment = &proof.proof_bytes[offset..offset + 32];
        offset += 32;
        let embedded_block_hash = &proof.proof_bytes[offset..offset + 32];
        offset += 32;
        let embedded_checksum = &proof.proof_bytes[offset..offset + 32];

        if height != proof.block_number
            || height != proof.position
            || embedded_block_hash != proof.block_hash.as_bytes()
            || (expected_root != Hash::zero() && expected_root != proof.block_hash)
        {
            return Ok(false);
        }

        let mut checksum_data = Vec::with_capacity(8 + 32 + 32 + 32);
        checksum_data.extend_from_slice(&height.to_le_bytes());
        checksum_data.extend_from_slice(txid);
        checksum_data.extend_from_slice(commitment);
        checksum_data.extend_from_slice(embedded_block_hash);
        let checksum = sha256d::Hash::hash(&checksum_data);

        Ok(checksum.to_byte_array().as_slice() == embedded_checksum)
    }

    /// Verify finality proof for a Bitcoin block
    async fn verify_finality(&self, proof: &FinalityProof) -> csv_core::Result<bool> {
        // Bitcoin finality is probabilistic - check confirmations
        // FinalityProof has confirmations field directly
        let confirmations = proof.confirmations;

        // Require at least 6 confirmations for Bitcoin finality
        Ok(confirmations >= 6)
    }

    /// Verify zero-knowledge proof (if applicable)
    async fn verify_zk(&self, _proof: &[u8]) -> csv_core::Result<bool> {
        // Bitcoin SPV doesn't use ZK proofs
        Ok(true)
    }

    /// Verify seal registry (check if seal has been consumed)
    async fn verify_seal_registry(&self, _seal_id: Hash) -> csv_core::Result<bool> {
        // Placeholder - would query Bitcoin blockchain to check if UTXO is spent
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
