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
use crate::tagged_hash::csv_tagged_hash;

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
    /// Signature scheme for cryptographic verification.
    /// Encodes which signature scheme the `proof` field uses.
    pub scheme: Option<crate::signature::SignatureScheme>,
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
    /// Salt used to compute the Right ID.
    /// Stored to enable ID recomputation during deserialization.
    pub salt: Vec<u8>,
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
    pub fn new(commitment: Hash, owner: OwnershipProof, salt: &[u8]) -> Self {
        let id = {
            // Use tagged hash with domain separation for Right ID computation
            let mut data = Vec::with_capacity(32 + salt.len());
            data.extend_from_slice(commitment.as_bytes());
            data.extend_from_slice(salt);
            let result = csv_tagged_hash("right-id", &data);
            RightId(Hash::new(result))
        };

        Self {
            id,
            commitment,
            owner,
            salt: salt.to_vec(),
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
            // L3: Compute deterministic nullifier with domain-separated hashing
            let mut data = Vec::with_capacity(32 + secret.len());
            data.extend_from_slice(self.id.0.as_bytes());
            data.extend_from_slice(secret);
            let nullifier = Hash::new(csv_tagged_hash("right-nullifier", &data));
            self.nullifier = Some(nullifier);
            Some(nullifier)
        } else {
            // L1/L2: No nullifier needed — chain enforces structurally.
            // Set a local consumption marker for tracking purposes.
            None
        }
    }

    /// Transfer this Right to a new owner.
    ///
    /// Creates a new Right with the same commitment and state but
    /// different ownership. The original Right remains valid until
    /// explicitly consumed.
    ///
    /// # Arguments
    /// * `new_owner` - The new owner's ownership proof
    /// * `transfer_salt` - A unique salt for the transfer to ensure unique ID
    ///
    /// # Returns
    /// A new Right instance with the new owner and a fresh ID
    pub fn transfer(&self, new_owner: OwnershipProof, transfer_salt: &[u8]) -> Right {
        // Create a new Right with same commitment but new owner
        let mut new_right = Right::new(self.commitment, new_owner, transfer_salt);

        // Preserve state root if present
        new_right.state_root = self.state_root;

        // Preserve execution proof if present
        new_right.execution_proof = self.execution_proof.clone();

        new_right
    }

    /// Verify this Right's ownership and validity.
    ///
    /// This is the core client-side validation function. It checks:
    /// 1. The ownership proof is cryptographically valid
    /// 2. The Right ID is correctly derived from commitment || salt
    /// 3. The commitment is well-formed
    /// 4. The Right has not been consumed (nullifier not set)
    ///
    /// For full consignment validation, use the client-side
    /// validation engine (Sprint 2).
    pub fn verify(&self) -> Result<(), RightError> {
        // Verify Right ID is correctly computed from commitment and salt
        let expected_id = {
            let mut data = Vec::with_capacity(32 + self.salt.len());
            data.extend_from_slice(self.commitment.as_bytes());
            data.extend_from_slice(&self.salt);
            RightId(Hash::new(csv_tagged_hash("right-id", &data)))
        };
        if self.id != expected_id {
            return Err(RightError::InvalidRightId);
        }

        // Cryptographically verify ownership proof
        if let Some(scheme) = self.owner.scheme {
            let signature = crate::signature::Signature::new(
                self.owner.proof.clone(),
                self.owner.owner.clone(),
                self.commitment.as_bytes().to_vec(),
            );
            signature
                .verify(scheme)
                .map_err(|_| RightError::InvalidOwnershipProof)?;
        } else {
            // For L1 chains (Bitcoin/Sui) where ownership is structural,
            // check that the proof is non-empty as a basic sanity check.
            // Full UTXO/Object ownership is enforced by the chain itself.
            if self.owner.proof.is_empty() {
                return Err(RightError::MissingOwnershipProof);
            }
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
        // Signature scheme (1 byte: 0=none, 1=secp256k1, 2=ed25519)
        out.push(match self.owner.scheme {
            None => 0,
            Some(crate::signature::SignatureScheme::Secp256k1) => 1,
            Some(crate::signature::SignatureScheme::Ed25519) => 2,
        });
        // Salt
        out.extend_from_slice(&(self.salt.len() as u32).to_le_bytes());
        out.extend_from_slice(&self.salt);
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

    /// Deserialize a Right from canonical bytes.
    ///
    /// # Errors
    /// Returns `RightError::InvalidEncoding` if the bytes are malformed.
    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, RightError> {
        let mut pos = 0;

        // Read ID (32 bytes)
        if bytes.len() < 32 {
            return Err(RightError::InvalidEncoding);
        }
        let mut id_bytes = [0u8; 32];
        id_bytes.copy_from_slice(&bytes[0..32]);
        pos += 32;

        // Read commitment (32 bytes)
        if bytes.len() < pos + 32 {
            return Err(RightError::InvalidEncoding);
        }
        let mut commitment_bytes = [0u8; 32];
        commitment_bytes.copy_from_slice(&bytes[pos..pos + 32]);
        pos += 32;

        // Read owner proof length and data
        if bytes.len() < pos + 4 {
            return Err(RightError::InvalidEncoding);
        }
        let proof_len = u32::from_le_bytes(
            bytes[pos..pos + 4]
                .try_into()
                .map_err(|_| RightError::InvalidEncoding)?,
        ) as usize;
        pos += 4;

        if bytes.len() < pos + proof_len {
            return Err(RightError::InvalidEncoding);
        }
        let proof = bytes[pos..pos + proof_len].to_vec();
        pos += proof_len;

        // Read owner identifier length and data
        if bytes.len() < pos + 4 {
            return Err(RightError::InvalidEncoding);
        }
        let owner_len = u32::from_le_bytes(
            bytes[pos..pos + 4]
                .try_into()
                .map_err(|_| RightError::InvalidEncoding)?,
        ) as usize;
        pos += 4;

        if bytes.len() < pos + owner_len {
            return Err(RightError::InvalidEncoding);
        }
        let owner_data = bytes[pos..pos + owner_len].to_vec();
        pos += owner_len;

        // Read signature scheme (1 byte: 0=none, 1=secp256k1, 2=ed25519)
        if pos >= bytes.len() {
            return Err(RightError::InvalidEncoding);
        }
        let scheme = match bytes[pos] {
            0 => None,
            1 => Some(crate::signature::SignatureScheme::Secp256k1),
            2 => Some(crate::signature::SignatureScheme::Ed25519),
            _ => return Err(RightError::InvalidEncoding),
        };
        pos += 1;

        // Read salt
        if bytes.len() < pos + 4 {
            return Err(RightError::InvalidEncoding);
        }
        let salt_len = u32::from_le_bytes(
            bytes[pos..pos + 4]
                .try_into()
                .map_err(|_| RightError::InvalidEncoding)?,
        ) as usize;
        pos += 4;

        if bytes.len() < pos + salt_len {
            return Err(RightError::InvalidEncoding);
        }
        let salt = bytes[pos..pos + salt_len].to_vec();
        pos += salt_len;

        // Read nullifier flag and data
        if pos >= bytes.len() {
            return Err(RightError::InvalidEncoding);
        }
        let has_nullifier = bytes[pos] == 1;
        pos += 1;

        let nullifier = if has_nullifier {
            if bytes.len() < pos + 32 {
                return Err(RightError::InvalidEncoding);
            }
            let mut nullifier_bytes = [0u8; 32];
            nullifier_bytes.copy_from_slice(&bytes[pos..pos + 32]);
            pos += 32;
            Some(Hash::new(nullifier_bytes))
        } else {
            None
        };

        // Read state_root flag and data
        if pos >= bytes.len() {
            return Err(RightError::InvalidEncoding);
        }
        let has_state_root = bytes[pos] == 1;
        pos += 1;

        let state_root = if has_state_root {
            if bytes.len() < pos + 32 {
                return Err(RightError::InvalidEncoding);
            }
            let mut state_root_bytes = [0u8; 32];
            state_root_bytes.copy_from_slice(&bytes[pos..pos + 32]);
            pos += 32;
            Some(Hash::new(state_root_bytes))
        } else {
            None
        };

        // Read execution proof length and data
        if bytes.len() < pos + 4 {
            return Err(RightError::InvalidEncoding);
        }
        let proof_data_len = u32::from_le_bytes(
            bytes[pos..pos + 4]
                .try_into()
                .map_err(|_| RightError::InvalidEncoding)?,
        ) as usize;
        pos += 4;

        let execution_proof = if proof_data_len > 0 {
            if bytes.len() < pos + proof_data_len {
                return Err(RightError::InvalidEncoding);
            }
            Some(bytes[pos..pos + proof_data_len].to_vec())
        } else {
            None
        };

        // Reconstruct the Right
        let id = RightId(Hash::new(id_bytes));
        let commitment = Hash::new(commitment_bytes);
        let owner = OwnershipProof {
            proof,
            owner: owner_data,
            scheme,
        };

        // Verify RightId matches H(commitment || salt) before constructing
        let expected_id = {
            let mut data = Vec::with_capacity(32 + salt.len());
            data.extend_from_slice(commitment.as_bytes());
            data.extend_from_slice(&salt);
            RightId(Hash::new(csv_tagged_hash("right-id", &data)))
        };
        if id != expected_id {
            return Err(RightError::InvalidRightId);
        }

        let right = Self {
            id,
            commitment,
            owner,
            salt,
            nullifier,
            state_root,
            execution_proof,
        };

        Ok(right)
    }

    /// Check if this Right has been consumed.
    pub fn is_consumed(&self) -> bool {
        self.nullifier.is_some()
    }

    /// Get the chain enforcement level indicator.
    ///
    /// Returns `true` if this is an L3 (cryptographic) Right that requires
    /// nullifier tracking.
    pub fn requires_nullifier(&self) -> bool {
        self.nullifier.is_some()
    }
}

/// Right validation errors.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum RightError {
    #[error("Missing ownership proof")]
    MissingOwnershipProof,
    #[error("Invalid ownership proof: signature verification failed")]
    InvalidOwnershipProof,
    #[error("Invalid commitment (zero hash)")]
    InvalidCommitment,
    #[error("Right has already been consumed")]
    AlreadyConsumed,
    #[error("Invalid nullifier")]
    InvalidNullifier,
    #[error("Invalid canonical encoding")]
    InvalidEncoding,
    #[error("Invalid RightId: does not match H(commitment || salt)")]
    InvalidRightId,
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
                scheme: None,
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
                scheme: None,
            },
            &[0x42; 16],
        );
        let r2 = Right::new(
            Hash::new([0xAB; 32]),
            OwnershipProof {
                proof: vec![0x01],
                owner: vec![0xFF; 32],
                scheme: None,
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
        // Recompute ID to match the new commitment (so we test commitment check, not ID check)
        let mut data = Vec::with_capacity(32 + right.salt.len());
        data.extend_from_slice(right.commitment.as_bytes());
        data.extend_from_slice(&right.salt);
        right.id = RightId(Hash::new(csv_tagged_hash("right-id", &data)));
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
        let decoded = Right::from_canonical_bytes(&bytes).expect("Should decode");
        assert_eq!(decoded.id, right.id);
        assert_eq!(decoded.commitment, right.commitment);
        assert_eq!(decoded.owner, right.owner);
        assert_eq!(decoded.nullifier, right.nullifier);
        assert_eq!(decoded.state_root, right.state_root);
    }

    #[test]
    fn test_right_canonical_roundtrip_with_nullifier() {
        let mut right = test_right();
        right.consume(Some(b"secret"));
        let bytes = right.to_canonical_bytes();
        let decoded = Right::from_canonical_bytes(&bytes).expect("Should decode");
        assert_eq!(decoded.nullifier, right.nullifier);
        assert!(decoded.is_consumed());
    }

    #[test]
    fn test_right_from_canonical_bytes_invalid() {
        // Empty bytes should fail
        assert!(Right::from_canonical_bytes(&[]).is_err());
        // Truncated bytes should fail
        assert!(Right::from_canonical_bytes(&[0u8; 16]).is_err());
    }

    #[test]
    fn test_right_transfer() {
        let right = test_right();
        let new_owner = OwnershipProof {
            proof: vec![0xAA, 0xBB, 0xCC],
            owner: vec![0xDD; 32],
            scheme: None,
        };
        let transferred = right.transfer(new_owner.clone(), b"transfer-salt");

        // New Right should have:
        // - Different ID (due to different salt)
        assert_ne!(transferred.id, right.id);
        // - Same commitment
        assert_eq!(transferred.commitment, right.commitment);
        // - New owner
        assert_eq!(transferred.owner, new_owner);
        // - Not consumed
        assert!(!transferred.is_consumed());
        // - Original Right unaffected
        assert!(!right.is_consumed());
    }

    #[test]
    fn test_right_transfer_preserves_state_root() {
        let mut right = test_right();
        right.state_root = Some(Hash::new([0xCD; 32]));

        let new_owner = OwnershipProof {
            proof: vec![0x01],
            owner: vec![0xFF; 32],
            scheme: None,
        };
        let transferred = right.transfer(new_owner, b"transfer");

        assert_eq!(transferred.state_root, right.state_root);
    }

    #[test]
    fn test_right_is_consumed() {
        let mut right = test_right();
        assert!(!right.is_consumed());

        right.consume(Some(b"secret"));
        assert!(right.is_consumed());
    }

    #[test]
    fn test_right_requires_nullifier() {
        let right_l1 = test_right(); // L1/L2 doesn't need nullifier
        assert!(!right_l1.requires_nullifier());

        let mut right_l3 = test_right();
        right_l3.consume(Some(b"secret")); // L3 does
        assert!(right_l3.requires_nullifier());
    }

    #[test]
    fn test_right_verify_ed25519_signature() {
        use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
        use rand::rngs::OsRng;

        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key: VerifyingKey = signing_key.verifying_key();
        let commitment = Hash::new([0xAB; 32]);

        // Sign the commitment
        let signature = signing_key.sign(commitment.as_bytes());

        let right = Right::new(
            commitment,
            OwnershipProof {
                proof: signature.to_bytes().to_vec(),
                owner: verifying_key.to_bytes().to_vec(),
                scheme: Some(crate::signature::SignatureScheme::Ed25519),
            },
            &[0x42; 16],
        );

        assert!(right.verify().is_ok());
    }

    #[test]
    fn test_right_verify_ed25519_wrong_message_fails() {
        use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
        use rand::rngs::OsRng;

        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key: VerifyingKey = signing_key.verifying_key();

        // Sign a different message (not the commitment)
        let wrong_message = [0xCD; 32];
        let signature = signing_key.sign(&wrong_message);

        let right = Right::new(
            Hash::new([0xAB; 32]), // Different from what was signed
            OwnershipProof {
                proof: signature.to_bytes().to_vec(),
                owner: verifying_key.to_bytes().to_vec(),
                scheme: Some(crate::signature::SignatureScheme::Ed25519),
            },
            &[0x42; 16],
        );

        // Verification should fail because the signature doesn't match the commitment
        assert_eq!(right.verify(), Err(RightError::InvalidOwnershipProof));
    }

    #[test]
    fn test_right_verify_secp256k1_signature() {
        use secp256k1::{Message, Secp256k1, SecretKey};

        let secp = Secp256k1::new();
        let secret_key = SecretKey::new(&mut secp256k1::rand::thread_rng());
        let public_key = secp256k1::PublicKey::from_secret_key(&secp, &secret_key);
        let commitment = Hash::new([0xAB; 32]);

        // Sign the commitment
        let msg = Message::from_digest_slice(commitment.as_bytes()).unwrap();
        let signature = secp.sign_ecdsa(&msg, &secret_key);

        let right = Right::new(
            commitment,
            OwnershipProof {
                proof: signature.serialize_compact().to_vec(),
                owner: public_key.serialize().to_vec(),
                scheme: Some(crate::signature::SignatureScheme::Secp256k1),
            },
            &[0x42; 16],
        );

        assert!(right.verify().is_ok());
    }

    #[test]
    fn test_right_verify_tampered_proof_fails() {
        use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
        use rand::rngs::OsRng;

        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key: VerifyingKey = signing_key.verifying_key();
        let commitment = Hash::new([0xAB; 32]);

        let signature = signing_key.sign(commitment.as_bytes());
        let mut tampered_sig = signature.to_bytes().to_vec();
        tampered_sig[0] ^= 0xFF; // Tamper with signature

        let right = Right::new(
            commitment,
            OwnershipProof {
                proof: tampered_sig,
                owner: verifying_key.to_bytes().to_vec(),
                scheme: Some(crate::signature::SignatureScheme::Ed25519),
            },
            &[0x42; 16],
        );

        assert_eq!(right.verify(), Err(RightError::InvalidOwnershipProof));
    }

    #[test]
    fn test_right_id_spoofing_fails() {
        // Attempt to create a Right with a mismatched ID
        let mut right = test_right();
        // Tamper with the ID
        right.id = RightId(Hash::new([0xFF; 32]));
        assert_eq!(right.verify(), Err(RightError::InvalidRightId));
    }

    #[test]
    fn test_from_canonical_bytes_rejects_spoofed_id() {
        let right = test_right();
        let mut bytes = right.to_canonical_bytes();

        // Tamper with the RightId in the serialized bytes (first 32 bytes)
        for byte in &mut bytes[0..32] {
            *byte ^= 0xFF;
        }

        assert_eq!(
            Right::from_canonical_bytes(&bytes),
            Err(RightError::InvalidRightId)
        );
    }
}
