//! Bitcoin SPV inclusion proofs

use crate::types::BitcoinInclusionProof;
use csv_adapter_core::Hash;
use sha2::{Digest, Sha256};

/// Verify a Merkle proof for transaction inclusion
///
/// # Arguments
/// * `txid` - Transaction ID to verify
/// * `merkle_root` - Merkle root from block header
/// * `proof` - Merkle branch proof
pub fn verify_merkle_proof(
    txid: &[u8; 32],
    merkle_root: &[u8; 32],
    proof: &BitcoinInclusionProof,
) -> bool {
    if proof.merkle_branch.is_empty() {
        // Single transaction case (txid == merkle_root)
        return txid == merkle_root;
    }

    let mut current_hash = *txid;

    for branch_hash in &proof.merkle_branch {
        let mut hasher = Sha256::new();
        // Bitcoin uses double SHA-256 for Merkle trees
        hasher.update(current_hash);
        hasher.update(branch_hash);
        let first_hash = hasher.finalize_reset();

        let mut hasher2 = Sha256::new();
        hasher2.update(first_hash);
        current_hash = hasher2.finalize().into();
    }

    current_hash == *merkle_root
}

/// Convert Bitcoin inclusion proof to core type
pub fn to_core_inclusion_proof(proof: &BitcoinInclusionProof) -> csv_adapter_core::InclusionProof {
    let mut proof_bytes = Vec::new();
    for branch in &proof.merkle_branch {
        proof_bytes.extend_from_slice(branch);
    }
    proof_bytes.extend_from_slice(&proof.block_hash);
    proof_bytes.extend_from_slice(&proof.tx_index.to_le_bytes());
    proof_bytes.extend_from_slice(&proof.block_height.to_le_bytes());

    csv_adapter_core::InclusionProof::new_unchecked(
        proof_bytes,
        Hash::new(proof.block_hash),
        proof.tx_index as u64,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merkle_proof_single_tx() {
        let txid = [1u8; 32];
        let proof = BitcoinInclusionProof::new(vec![], txid, 0, 100);
        assert!(verify_merkle_proof(&txid, &txid, &proof));
    }
}
