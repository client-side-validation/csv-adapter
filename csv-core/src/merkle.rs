//! Merkle tree and proof types for cryptographic inclusion proofs.
//!
//! This module provides generic Merkle tree construction and verification
//! used across chain adapters for SPV-style proofs.

use alloc::vec::Vec;

use crate::hash::Hash;

/// A Merkle proof consisting of sibling hashes and their positions.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MerkleProof {
    /// The sibling hashes along the path from leaf to root.
    pub siblings: Vec<Hash>,
    /// The leaf index in the tree.
    pub leaf_index: usize,
    /// The total number of leaves.
    pub leaf_count: usize,
}

impl MerkleProof {
    /// Verify this proof against a known root.
    pub fn verify(&self, leaf: Hash, root: Hash) -> bool {
        if self.leaf_count == 0 {
            return false;
        }
        let mut current = leaf;
        let mut index = self.leaf_index;

        for (i, sibling) in self.siblings.iter().enumerate() {
            let expected_level_size = (self.leaf_count + (1 << i) - 1) >> i;
            if index >= expected_level_size {
                return false;
            }

            current = if index % 2 == 0 {
                Hash::combine(&current, sibling)
            } else {
                Hash::combine(sibling, &current)
            };
            index /= 2;
        }

        current == root
    }
}

/// A Merkle tree built from a list of leaf hashes.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MerkleTree {
    /// The root hash of the tree.
    pub root: Hash,
    /// All leaf hashes.
    pub leaves: Vec<Hash>,
    /// The number of levels in the tree.
    pub depth: usize,
}

impl MerkleTree {
    /// Build a Merkle tree from leaf hashes.
    pub fn new(leaves: Vec<Hash>) -> Self {
        if leaves.is_empty() {
            return Self {
                root: Hash::default(),
                leaves: Vec::new(),
                depth: 0,
            };
        }

        let mut current_level = leaves.clone();
        let mut depth = 0usize;

        while current_level.len() > 1 {
            let mut next_level = Vec::new();
            let chunked = current_level.chunks(2);
            for chunk in chunked {
                match chunk {
                    [left, right] => next_level.push(Hash::combine(left, right)),
                    [single] => next_level.push(*single),
                    _ => unreachable!(),
                }
            }
            current_level = next_level;
            depth += 1;
        }

        Self {
            root: current_level[0],
            leaves,
            depth,
        }
    }

    /// Generate a Merkle proof for the leaf at the given index.
    pub fn proof(&self, leaf_index: usize) -> Option<MerkleProof> {
        if leaf_index >= self.leaves.len() {
            return None;
        }

        let mut siblings = Vec::new();
        let mut index = leaf_index;
        let mut current_leaves = self.leaves.clone();

        while current_leaves.len() > 1 {
            let mut next_level = Vec::new();
            let mut next_siblings = Vec::new();

            for chunk in current_leaves.chunks(2) {
                match chunk {
                    [left, right] => {
                        let combined = Hash::combine(left, right);
                        if index % 2 == 0 {
                            next_siblings.push(*right);
                        } else {
                            next_siblings.push(*left);
                        }
                        next_level.push(combined);
                    }
                    [single] => {
                        if index % 2 == 1 {
                            next_siblings.push(*single);
                        }
                        next_level.push(*single);
                    }
                    _ => unreachable!(),
                }
            }

            siblings.extend(next_siblings);
            current_leaves = next_level;
            index /= 2;
        }

        Some(MerkleProof {
            siblings,
            leaf_index: leaf_index,
            leaf_count: self.leaves.len(),
        })
    }
}
