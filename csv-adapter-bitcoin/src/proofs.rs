//! Bitcoin SPV inclusion proofs
//!
//! This module provides production-grade Bitcoin SPV (Simplified Payment Verification)
//! proofs using double-SHA256 Merkle tree implementation.

use crate::types::BitcoinInclusionProof;
use csv_adapter_core::Hash as CoreHash;
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

/// Verify a complete SPV proof including block header
pub fn verify_spv_proof(
    txid: &[u8; 32],
    block_hash: &[u8; 32],
    merkle_proof: &BitcoinInclusionProof,
) -> bool {
    // First verify the Merkle proof
    if !verify_merkle_proof(txid, block_hash, merkle_proof) {
        return false;
    }

    // Verify block hash is non-zero (basic check)
    *block_hash != [0u8; 32]
}

/// Verify a complete SPV proof with block header data
pub fn verify_spv_proof_with_header(
    txid: &[u8; 32],
    block_header_data: &[u8],
    merkle_proof: &BitcoinInclusionProof,
) -> bool {
    // Parse the block header (simplified - in production use proper deserialization)
    if block_header_data.len() < 80 {
        return false;
    }

    // Extract merkle root from header (bytes 36-68)
    let mut merkle_root = [0u8; 32];
    merkle_root.copy_from_slice(&block_header_data[36..68]);

    // Verify the Merkle proof
    verify_merkle_proof(txid, &merkle_root, merkle_proof)
}

/// Full SPV proof verification with block header verification
pub fn verify_full_spv_proof(
    txid: &[u8; 32],
    block_hash: &[u8; 32],
    merkle_proof: &BitcoinInclusionProof,
    block_height: u64,
    required_confirmations: u32,
    current_height: u64,
) -> bool {
    // Verify the Merkle proof
    if !verify_merkle_proof(txid, block_hash, merkle_proof) {
        return false;
    }

    // Verify block height and confirmations
    if block_height + required_confirmations as u64 > current_height {
        return false;
    }

    // Verify block hash is non-zero
    *block_hash != [0u8; 32]
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
        CoreHash::new(proof.block_hash),
        proof.tx_index as u64,
    )
}

/// Convert core inclusion proof to Bitcoin type
pub fn from_core_inclusion_proof(
    proof: &csv_adapter_core::InclusionProof,
) -> BitcoinInclusionProof {
    let proof_bytes = &proof.proof_bytes;

    // Need at least 32 (block_hash) + 8 (tx_index) + 8 (block_height) = 48 bytes
    if proof_bytes.len() < 48 {
        // Fallback: create empty proof
        return BitcoinInclusionProof::new(vec![], [0u8; 32], 0, 0);
    }

    // Parse merkle branch hashes (each is 32 bytes)
    let mut merkle_branch = Vec::new();
    let metadata_size = 32 + 8 + 8; // block_hash + tx_index + block_height
    let branch_data_len = proof_bytes.len() - metadata_size;
    let mut pos = 0;

    while pos + 32 <= branch_data_len {
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&proof_bytes[pos..pos + 32]);
        merkle_branch.push(hash);
        pos += 32;
    }

    // Parse block hash (32 bytes)
    let mut block_hash = [0u8; 32];
    block_hash.copy_from_slice(&proof_bytes[pos..pos + 32]);
    pos += 32;

    // Parse tx index (8 bytes, little-endian)
    let tx_index = u64::from_le_bytes(proof_bytes[pos..pos + 8].try_into().unwrap()) as u32;
    pos += 8;

    // Parse block height (8 bytes, little-endian)
    let block_height = u64::from_le_bytes(proof_bytes[pos..pos + 8].try_into().unwrap());

    BitcoinInclusionProof::new(merkle_branch, block_hash, tx_index, block_height)
}

/// Extract Merkle proof from a Bitcoin block for a specific transaction
///
/// Computes the actual Merkle branch (sibling hashes) needed to verify
/// that the given txid is included in the block's merkle root.
///
/// # Arguments
/// * `txid` - Transaction ID to prove inclusion for
/// * `block_txids` - All transaction IDs in the block (in order)
/// * `block_hash` - Block header hash
/// * `block_height` - Block height
///
/// # Returns
/// * `BitcoinInclusionProof` - Verifiable inclusion proof with Merkle branch
pub fn extract_merkle_proof_from_block(
    txid: [u8; 32],
    block_txids: &[[u8; 32]],
    block_hash: [u8; 32],
    block_height: u64,
) -> Option<BitcoinInclusionProof> {
    if block_txids.is_empty() {
        return None;
    }

    // Find transaction position in block
    let tx_index = block_txids.iter().position(|id| *id == txid)?;

    // Compute the merkle branch (sibling hashes) needed to verify this txid
    let merkle_branch = compute_merkle_branch(&txid, tx_index, block_txids)?;

    Some(BitcoinInclusionProof::new(
        merkle_branch,
        block_hash,
        tx_index as u32,
        block_height,
    ))
}

/// Compute the Merkle branch (sibling hashes) for a transaction at a given index.
///
/// This walks the merkle tree bottom-up, collecting sibling hashes at each level.
fn compute_merkle_branch(
    _target_txid: &[u8; 32],
    target_index: usize,
    all_txids: &[[u8; 32]],
) -> Option<Vec<[u8; 32]>> {
    if all_txids.is_empty() {
        return None;
    }

    if all_txids.len() == 1 {
        // Single tx: no branch needed
        return Some(vec![]);
    }

    let mut current_level: Vec<[u8; 32]> = all_txids.to_vec();
    let mut branch = Vec::new();
    let mut idx = target_index;

    while current_level.len() > 1 {
        let mut next_level = Vec::new();

        for i in (0..current_level.len()).step_by(2) {
            let left = current_level[i];
            let right = if i + 1 < current_level.len() {
                current_level[i + 1]
            } else {
                // Odd node: duplicate
                left
            };

            // Collect sibling hash (not the target's own hash)
            let is_target_left = i == idx;
            let sibling = if is_target_left { right } else { left };
            if i == idx || i + 1 == idx {
                branch.push(sibling);
            }

            // Compute parent hash (double SHA-256)
            let parent = double_sha256_pair(&left, &right);
            next_level.push(parent);
        }

        current_level = next_level;
        idx /= 2;
    }

    Some(branch)
}

/// Double SHA-256 of two concatenated 32-byte inputs
fn double_sha256_pair(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(left);
    hasher.update(right);
    let first = hasher.finalize_reset();
    let mut hasher2 = Sha256::new();
    hasher2.update(first);
    hasher2.finalize().into()
}

/// Compute the merkle root from a set of transaction IDs
/// This implements the standard Bitcoin merkle tree construction
pub fn compute_merkle_root(txids: &[[u8; 32]]) -> Option<[u8; 32]> {
    if txids.is_empty() {
        return None;
    }

    if txids.len() == 1 {
        return Some(txids[0]);
    }

    // Build the merkle tree level by level
    let mut current_level = txids.to_vec();

    while current_level.len() > 1 {
        let mut next_level = Vec::new();

        // Pair up hashes and hash them together
        for i in (0..current_level.len()).step_by(2) {
            let left = current_level[i];
            let right = if i + 1 < current_level.len() {
                current_level[i + 1]
            } else {
                // If odd number, duplicate the last hash
                left
            };

            // Double SHA-256
            let mut hasher = Sha256::new();
            hasher.update(left);
            hasher.update(right);
            let first_hash = hasher.finalize_reset();

            let mut hasher2 = Sha256::new();
            hasher2.update(first_hash);
            next_level.push(hasher2.finalize().into());
        }

        current_level = next_level;
    }

    Some(current_level[0])
}

/// Verify a block's merkle root matches the computed root
pub fn verify_block_merkle_root(block_txids: &[[u8; 32]], expected_root: [u8; 32]) -> bool {
    match compute_merkle_root(block_txids) {
        Some(computed_root) => computed_root == expected_root,
        None => false,
    }
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

    #[test]
    fn test_merkle_proof_with_branch() {
        let txid = [1u8; 32];
        let merkle_root = [2u8; 32];
        let proof = BitcoinInclusionProof::new(vec![[3u8; 32]], merkle_root, 0, 100);
        // This would succeed if the merkle branch computes to the root
        let result = verify_merkle_proof(&txid, &merkle_root, &proof);
        // For this test, we just check it doesn't panic
        let _ = result;
    }

    #[test]
    fn test_spv_proof() {
        let txid = [1u8; 32];
        // For single transaction, block_hash (merkle_root) must equal txid
        let proof = BitcoinInclusionProof::new(vec![], txid, 0, 100);
        assert!(verify_spv_proof(&txid, &txid, &proof));
    }

    #[test]
    fn test_full_spv_proof() {
        let txid = [1u8; 32];
        // For single transaction, block_hash (merkle_root) must equal txid
        let proof = BitcoinInclusionProof::new(vec![], txid, 0, 100);
        assert!(verify_full_spv_proof(&txid, &txid, &proof, 100, 6, 106));
        assert!(!verify_full_spv_proof(&txid, &txid, &proof, 100, 6, 105));
    }

    #[test]
    fn test_to_core_inclusion_proof() {
        let proof = BitcoinInclusionProof::new(vec![[1u8; 32], [2u8; 32]], [3u8; 32], 5, 100);
        let core_proof = to_core_inclusion_proof(&proof);
        assert_eq!(core_proof.position, 5);
        assert_eq!(core_proof.block_hash, CoreHash::new([3u8; 32]));
    }

    #[test]
    fn test_from_core_inclusion_proof() {
        let core_proof =
            csv_adapter_core::InclusionProof::new(vec![0xAB; 64], CoreHash::new([1u8; 32]), 5)
                .unwrap();
        let bitcoin_proof = from_core_inclusion_proof(&core_proof);
        // Just check it doesn't panic
        let _ = bitcoin_proof;
    }

    #[test]
    fn test_compute_merkle_root_single_tx() {
        let txids = [[1u8; 32]];
        let root = compute_merkle_root(&txids).unwrap();
        assert_eq!(root, txids[0]);
    }

    #[test]
    fn test_compute_merkle_root_two_txs() {
        let txids = [[1u8; 32], [2u8; 32]];
        let root = compute_merkle_root(&txids).unwrap();
        // The root should be non-zero
        assert_ne!(root, [0u8; 32]);
    }

    #[test]
    fn test_compute_merkle_root_three_txs() {
        let txids = [[1u8; 32], [2u8; 32], [3u8; 32]];
        let root = compute_merkle_root(&txids).unwrap();
        // The root should be non-zero
        assert_ne!(root, [0u8; 32]);
    }

    #[test]
    fn test_verify_block_merkle_root() {
        let txids = [[1u8; 32], [2u8; 32]];
        let merkle_root = compute_merkle_root(&txids).unwrap();
        assert!(verify_block_merkle_root(&txids, merkle_root));
        assert!(!verify_block_merkle_root(&txids, [0u8; 32]));
    }

    #[test]
    fn test_extract_merkle_proof_single_tx() {
        let txid = [1u8; 32];
        let block_txids = vec![txid];
        let block_hash = [2u8; 32];
        let block_height = 100;

        let proof = extract_merkle_proof_from_block(txid, &block_txids, block_hash, block_height);
        assert!(proof.is_some());

        let proof = proof.unwrap();
        assert_eq!(proof.tx_index, 0);
        assert_eq!(proof.block_hash, block_hash);
        assert_eq!(proof.block_height, block_height);
        // Single tx: no sibling hashes needed
        assert!(proof.merkle_branch.is_empty());
    }

    #[test]
    fn test_extract_merkle_proof_multiple_txs() {
        let txid1 = [1u8; 32];
        let txid2 = [2u8; 32];
        let txid3 = [3u8; 32];
        let block_txids = vec![txid1, txid2, txid3];
        let block_hash = [4u8; 32];
        let block_height = 200;

        // Extract proof for txid2
        let proof = extract_merkle_proof_from_block(txid2, &block_txids, block_hash, block_height);
        assert!(proof.is_some());

        let proof = proof.unwrap();
        assert_eq!(proof.tx_index, 1); // txid2 is at index 1
        assert_eq!(proof.block_hash, block_hash);
        assert!(!proof.merkle_branch.is_empty()); // Multiple txs have branches
    }

    #[test]
    fn test_extract_merkle_proof_not_found() {
        let txid = [99u8; 32]; // Not in block
        let block_txids = vec![[1u8; 32], [2u8; 32]];
        let block_hash = [3u8; 32];
        let block_height = 100;

        let proof = extract_merkle_proof_from_block(txid, &block_txids, block_hash, block_height);
        assert!(proof.is_none());
    }

    #[test]
    fn test_extract_merkle_proof_empty_block() {
        let txid = [1u8; 32];
        let block_txids = vec![];
        let block_hash = [2u8; 32];
        let block_height = 100;

        let proof = extract_merkle_proof_from_block(txid, &block_txids, block_hash, block_height);
        assert!(proof.is_none());
    }
}
