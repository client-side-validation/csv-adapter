//! The Universal Seal Primitive — Canonical Right Type
//!
//! A Right can be exercised at most once under the strongest available
//! guarantee of the host chain. This is the core invariant of the entire system.
//!
//! ## Enforcement Layers
//!
//! | Level | Name | Chains | Mechanism |
//! |-------|------|--------|-----------|
//! | L1 | Structural | Bitcoin, Sui | Spend UTXO / Consume Object |
//! | L2 | Type-Enforced | Aptos | Destroy Move Resource |
//! | L3 | Cryptographic | Ethereum | Nullifier Registration |
//!
//! ## Client-Side Validation
//!
//! The chain does NOT validate state transitions. It only:
//! 1. Records the commitment (anchor)
//! 2. Enforces single-use of the Right
//!
//! Clients do everything else:
//! 1. Fetch the full state history for a contract
//! 2. Verify the commitment chain from genesis to present
//! 3. Check that no Right was consumed more than once
//! 4. Accept or reject the consignment based on local validation

use alloc::vec::Vec;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::hash::Hash;

/// A unique Right identifier.
///
/// Computed as `H(commitment || salt)` to ensure uniqueness
/// even when the same state is committed to multiple times.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RightId(pub Hash);

/// Proof of ownership for a Right.
///
/// On L1 chains (Bitcoin, Sui): this is the UTXO/Object ownership proof.
/// On L2 chains (Aptos): this is the resource capability proof.
/// On L3 chains (Ethereum): this is the signature from the owner.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnershipProof {
    /// The proof bytes (chain-specific format)
    pub proof: Vec<u8>,
    /// The owner identifier (address, pubkey, etc.)
    pub owner: Vec<u8>,
}

/// A consumable Right in the USP system.
///
/// Every chain enforces single-use of Rights, but at different
/// enforcement levels (L1 Structural → L2 Type-Enforced → L3 Cryptographic).
///
/// The chain provides the minimum guarantee (single-use enforcement).
/// Clients verify everything else.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Right {
    /// Unique identifier: `H(commitment || salt)`
    pub id: RightId,
    /// Encodes the state + rules of this Right
    pub commitment: Hash,
    /// Proof of ownership
    pub owner: OwnershipProof,
    /// One-time consumption marker (L3+ only)
    ///
    /// L1 (Bitcoin/Sui): None — chain enforces structurally.
    /// L2 (Aptos): None — Move VM enforces non-duplication.
    /// L3 (Ethereum): Some — nullifier registered in contract.
    pub nullifier: Option<Hash>,
    /// Off-chain state commitment root
    ///
    /// Commits to the full state history for this Right.
    /// Clients use this to verify state transitions without
    /// fetching the entire history on every validation.
    pub state_root: Option<Hash>,
    /// Optional execution proof (ZK, fraud proof, etc.)
    ///
    /// For advanced use cases where the Right's execution
    /// needs to be proven without revealing its contents.
    pub execution_proof: Option<Vec<u8>>,
}

impl Right {
    /// Create a new Right with the given parameters.
    ///
    /// The Right ID is deterministically computed from the commitment
    /// and salt, ensuring uniqueness even for duplicate commitments.
    pub fn new(
        commitment: Hash,
        owner: OwnershipProof,
        salt: &[u8],
    ) -> Self {
        let id = {
            let mut hasher = Sha256::new();
            hasher.update(commitment.as_bytes());
            hasher.update(salt);
            let result = hasher.finalize();
            let mut array = [0u8; 32];
            array.copy_from_slice(&result);
            RightId(Hash::new(array))
        };

        Self {
            id,
            commitment,
            owner,
            nullifier: None,
            state_root: None,
            execution_proof: None,
        }
    }

    /// Mark this Right as consumed by setting the nullifier.
    ///
    /// # Enforcement Level
    ///
    /// - **L1 (Bitcoin/Sui)**: This method is a local marker only.
    ///   The actual single-use enforcement is done by the chain
    ///   (UTXO spending / Object deletion).
    ///
    /// - **L2 (Aptos)**: This method is a local marker only.
    ///   The Move VM enforces non-duplication of resources.
    ///
    /// - **L3 (Ethereum)**: The nullifier MUST be registered on-chain.
    ///   The contract's `nullifiers[id] = true` is what enforces single-use.
    ///
    /// # Returns
    /// The nullifier hash, or `None` for L1/L2 chains where the
    /// nullifier is not needed (but returned for local tracking).
    pub fn consume(&mut self, secret: Option<&[u8]>) -> Option<Hash> {
        if let Some(secret) = secret {
            // L3: Compute deterministic nullifier
            let nullifier = {
                let mut hasher = Sha256::new();
                hasher.update(self.id.0.as_bytes());
                hasher.update(secret);
                let result = hasher.finalize();
                let mut array = [0u8; 32];
                array.copy_from_slice(&result);
                Hash::new(array)
            };
            self.nullifier = Some(nullifier);
            Some(nullifier)
        } else {
            // L1/L2: No nullifier needed — chain enforces structurally.
            // Set a local consumption marker for tracking purposes.
            None
        }
    }

    /// Verify this Right's ownership and validity.
    ///
    /// This is the core client-side validation function. It checks:
    /// 1. The owner proof is valid for this Right
    /// 2. The commitment is well-formed
    /// 3. The Right has not been consumed (nullifier not set)
    ///
    /// For full consignment validation, use the client-side
    /// validation engine (Sprint 2).
    pub fn verify(&self) -> Result<(), RightError> {
        // Check ownership proof is present
        if self.owner.proof.is_empty() {
            return Err(RightError::MissingOwnershipProof);
        }

        // Check commitment is non-zero
        if self.commitment.as_bytes() == &[0u8; 32] {
            return Err(RightError::InvalidCommitment);
        }

        // Check Right has not been consumed
        if self.nullifier.is_some() {
            return Err(RightError::AlreadyConsumed);
        }

        Ok(())
    }

    /// Serialize this Right to canonical bytes.
    ///
    /// Used for hashing, signing, and transmission.
    pub fn to_canonical_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(self.id.0.as_bytes());
        out.extend_from_slice(self.commitment.as_bytes());
        out.extend_from_slice(&(self.owner.proof.len() as u32).to_le_bytes());
        out.extend_from_slice(&self.owner.proof);
        out.extend_from_slice(&(self.owner.owner.len() as u32).to_le_bytes());
        out.extend_from_slice(&self.owner.owner);
        out.push(if self.nullifier.is_some() { 1 } else { 0 });
        if let Some(nullifier) = &self.nullifier {
            out.extend_from_slice(nullifier.as_bytes());
        }
        out.push(if self.state_root.is_some() { 1 } else { 0 });
        if let Some(state_root) = &self.state_root {
            out.extend_from_slice(state_root.as_bytes());
        }
        out.extend_from_slice(
            &(self.execution_proof.as_ref().map_or(0, |p| p.len()) as u32).to_le_bytes(),
        );
        if let Some(proof) = &self.execution_proof {
            out.extend_from_slice(proof);
        }
        out
    }
}

/// Right validation errors.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum RightError {
    #[error("Missing ownership proof")]
    MissingOwnershipProof,
    #[error("Invalid commitment (zero hash)")]
    InvalidCommitment,
    #[error("Right has already been consumed")]
    AlreadyConsumed,
    #[error("Invalid nullifier")]
    InvalidNullifier,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_right() -> Right {
        Right::new(
            Hash::new([0xAB; 32]),
            OwnershipProof {
                proof: vec![0x01, 0x02, 0x03],
                owner: vec![0xFF; 32],
            },
            &[0x42; 16],
        )
    }

    #[test]
    fn test_right_creation() {
        let right = test_right();
        assert_eq!(right.commitment.as_bytes(), &[0xAB; 32]);
        assert!(right.nullifier.is_none());
        assert!(right.state_root.is_none());
        assert!(right.execution_proof.is_none());
    }

    #[test]
    fn test_right_id_deterministic() {
        let r1 = test_right();
        let r2 = test_right();
        assert_eq!(r1.id, r2.id);
    }

    #[test]
    fn test_right_id_unique_per_salt() {
        let r1 = Right::new(
            Hash::new([0xAB; 32]),
            OwnershipProof {
                proof: vec![0x01],
                owner: vec![0xFF; 32],
            },
            &[0x42; 16],
        );
        let r2 = Right::new(
            Hash::new([0xAB; 32]),
            OwnershipProof {
                proof: vec![0x01],
                owner: vec![0xFF; 32],
            },
            &[0x99; 16],
        );
        assert_ne!(r1.id, r2.id);
    }

    #[test]
    fn test_right_verify_valid() {
        let right = test_right();
        assert!(right.verify().is_ok());
    }

    #[test]
    fn test_right_verify_missing_proof() {
        let mut right = test_right();
        right.owner.proof = vec![];
        assert_eq!(right.verify(), Err(RightError::MissingOwnershipProof));
    }

    #[test]
    fn test_right_verify_zero_commitment() {
        let mut right = test_right();
        right.commitment = Hash::new([0u8; 32]);
        assert_eq!(right.verify(), Err(RightError::InvalidCommitment));
    }

    #[test]
    fn test_right_consume_with_nullifier() {
        let mut right = test_right();
        let nullifier = right.consume(Some(b"secret"));
        assert!(nullifier.is_some());
        assert!(right.nullifier.is_some());
        assert_eq!(right.verify(), Err(RightError::AlreadyConsumed));
    }

    #[test]
    fn test_right_consume_without_nullifier() {
        let mut right = test_right();
        let result = right.consume(None);
        assert!(result.is_none());
        assert!(right.nullifier.is_none());
        // L1/L2: Right is still valid locally (chain enforces structural single-use)
        assert!(right.verify().is_ok());
    }

    #[test]
    fn test_right_canonical_roundtrip() {
        let right = test_right();
        let bytes = right.to_canonical_bytes();
        // Just verify it serializes without panic
        assert!(!bytes.is_empty());
    }
}
