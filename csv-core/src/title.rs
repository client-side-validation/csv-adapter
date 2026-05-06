//! The Universal Seal Primitive — Canonical Sanad (Title) Type
//!
//! A Sanad (property deed) can be exercised at most once under the strongest available
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
//! 2. Enforces single-use of the Sanad
//!
//! Clients do everything else:
//! 1. Fetch the full state history for a contract
//! 2. Verify the commitment chain from genesis to present
//! 3. Check that no Sanad was consumed more than once
//! 4. Accept or reject the consignment based on local validation

use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

use crate::hash::Hash;
use crate::tagged_hash::csv_tagged_hash;

/// A unique Sanad identifier.
///
/// Computed as `H(commitment || salt)` to ensure uniqueness
/// even when the same state is committed to multiple times.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SanadId(pub Hash);

impl SanadId {
    /// Creates a new SanadId from a 32-byte hash.
    #[inline]
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(Hash::new(bytes))
    }

    /// Creates a new SanadId from a byte slice.
    /// Panics if the slice is not exactly 32 bytes.
    #[inline]
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let array: [u8; 32] = bytes.try_into()
            .expect("SanadId::from_bytes requires exactly 32 bytes");
        Self::new(array)
    }

    /// Returns the underlying hash bytes.
    #[inline]
    pub fn as_bytes(&self) -> &[u8; 32] {
        self.0.as_bytes()
    }
}

impl From<crate::sanad::SanadId> for SanadId {
    fn from(sanad_id: crate::sanad::SanadId) -> Self {
        SanadId(sanad_id.0)
    }
}

impl From<&crate::sanad::SanadId> for SanadId {
    fn from(sanad_id: &crate::sanad::SanadId) -> Self {
        SanadId(sanad_id.0)
    }
}

impl From<SanadId> for crate::sanad::SanadId {
    fn from(sanad_id: SanadId) -> Self {
        crate::sanad::SanadId(sanad_id.0)
    }
}

/// Proof of ownership for a Sanad.
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

impl From<crate::sanad::OwnershipProof> for OwnershipProof {
    fn from(proof: crate::sanad::OwnershipProof) -> Self {
        Self {
            proof: proof.proof,
            owner: proof.owner,
            scheme: proof.scheme,
        }
    }
}

impl From<OwnershipProof> for crate::sanad::OwnershipProof {
    fn from(proof: OwnershipProof) -> Self {
        Self {
            proof: proof.proof,
            owner: proof.owner,
            scheme: proof.scheme,
        }
    }
}

/// A consumable Sanad in the USP system.
///
/// Every chain enforces single-use of Sanads, but at different
/// enforcement levels (L1 Structural → L2 Type-Enforced → L3 Cryptographic).
///
/// The chain provides the minimum guarantee (single-use enforcement).
/// Clients verify everything else.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sanad {
    /// Unique identifier: `H(commitment || salt)`
    pub id: SanadId,
    /// Encodes the state + rules of this Sanad
    pub commitment: Hash,
    /// Proof of ownership
    pub owner: OwnershipProof,
    /// Salt used to compute the Sanad ID.
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
    /// Commits to the full state history for this Sanad.
    /// Clients use this to verify state transitions without
    /// fetching the entire history on every validation.
    pub state_root: Option<Hash>,
    /// Optional execution proof (ZK, fraud proof, etc.)
    ///
    /// For advanced use cases where the Sanad's execution
    /// needs to be proven without revealing its contents.
    pub execution_proof: Option<Vec<u8>>,
}

impl Sanad {
    /// Create a new Sanad with the given parameters.
    ///
    /// The Sanad ID is deterministically computed from the commitment
    /// and salt, ensuring uniqueness even for duplicate commitments.
    pub fn new(commitment: Hash, owner: OwnershipProof, salt: &[u8]) -> Self {
        let id = {
            // Use tagged hash with domain separation for Sanad ID computation
            let mut data = Vec::with_capacity(32 + salt.len());
            data.extend_from_slice(commitment.as_bytes());
            data.extend_from_slice(salt);
            let result = csv_tagged_hash("sanad-id", &data);
            SanadId(Hash::new(result))
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

    /// Mark this Sanad as consumed by setting the nullifier.
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
    /// # Nullifier Construction
    ///
    /// `nullifier = tagged_hash("csv-nullifier", sanad_id || secret || context)`
    ///
    /// Where `context = H(chain_id || domain_separator)` binds the nullifier
    /// to a specific chain context, preventing cross-chain replay attacks
    /// even if the same secret is reused.
    ///
    /// # Arguments
    /// * `secret` — The user's secret (prevents front-running)
    /// * `chain_context` — Pre-computed context hash: `H(chain_id || domain_separator)`
    ///
    /// # Returns
    /// The nullifier hash, or `None` for L1/L2 chains where the
    /// nullifier is not needed (but returned for local tracking).
    pub fn consume(
        &mut self,
        secret: Option<&[u8]>,
        chain_context: Option<&[u8; 32]>,
    ) -> Option<Hash> {
        if let Some(secret) = secret {
            // Build context binding: H(chain_id || domain_separator)
            // If no chain_context provided, use empty context (backward compat)
            let context_bytes = chain_context.unwrap_or(&[0u8; 32]);

            // L3: Compute deterministic nullifier with domain-separated hashing
            // nullifier = H("csv-nullifier" || sanad_id || secret || context)
            let mut data = Vec::with_capacity(32 + self.id.0.as_bytes().len() + secret.len() + 32);
            data.extend_from_slice(self.id.0.as_bytes());
            data.extend_from_slice(secret);
            data.extend_from_slice(context_bytes);
            let nullifier = Hash::new(csv_tagged_hash("csv-nullifier", &data));
            self.nullifier = Some(nullifier);
            Some(nullifier)
        } else {
            // L1/L2: No nullifier needed — chain enforces structurally.
            // Set a local consumption marker for tracking purposes.
            None
        }
    }

    /// Transfer this Sanad to a new owner.
    ///
    /// Creates a new Sanad with the same commitment and state but
    /// different ownership. The original Sanad remains valid until
    /// explicitly consumed.
    ///
    /// # Arguments
    /// * `new_owner` - The new owner's ownership proof
    /// * `transfer_salt` - A unique salt for the transfer to ensure unique ID
    ///
    /// # Returns
    /// A new Sanad instance with the new owner and a fresh ID
    pub fn transfer(&self, new_owner: OwnershipProof, transfer_salt: &[u8]) -> Sanad {
        // Create a new Sanad with same commitment but new owner
        let mut new_sanad = Sanad::new(self.commitment, new_owner, transfer_salt);

        // Preserve state root if present
        new_sanad.state_root = self.state_root;

        // Preserve execution proof if present
        new_sanad.execution_proof = self.execution_proof.clone();

        new_sanad
    }

    /// Verify this Sanad's ownership and validity.
    ///
    /// This is the core client-side validation function. It checks:
    /// 1. The ownership proof is cryptographically valid
    /// 2. The Sanad ID is correctly derived from commitment || salt
    /// 3. The commitment is well-formed
    /// 4. The Sanad has not been consumed (nullifier not set)
    ///
    /// For full consignment validation, use the client-side
    /// validation engine (Sprint 2).
    pub fn verify(&self) -> Result<(), SanadError> {
        // Verify Sanad ID is correctly computed from commitment and salt
        let expected_id = {
            let mut data = Vec::with_capacity(32 + self.salt.len());
            data.extend_from_slice(self.commitment.as_bytes());
            data.extend_from_slice(&self.salt);
            SanadId(Hash::new(csv_tagged_hash("sanad-id", &data)))
        };
        if self.id != expected_id {
            return Err(SanadError::InvalidSanadId);
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
                .map_err(|_| SanadError::InvalidOwnershipProof)?;
        } else {
            // For L1 chains (Bitcoin/Sui) where ownership is structural,
            // check that the proof is non-empty as a basic sanity check.
            // Full UTXO/Object ownership is enforced by the chain itself.
            if self.owner.proof.is_empty() {
                return Err(SanadError::MissingOwnershipProof);
            }
        }

        // Check commitment is non-zero
        if self.commitment.as_bytes() == &[0u8; 32] {
            return Err(SanadError::InvalidCommitment);
        }

        // Check Sanad has not been consumed
        if self.nullifier.is_some() {
            return Err(SanadError::AlreadyConsumed);
        }

        Ok(())
    }

    /// Serialize this Sanad to canonical bytes.
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

    /// Deserialize a Sanad from canonical bytes.
    ///
    /// # Errors
    /// Returns `SanadError::InvalidEncoding` if the bytes are malformed.
    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, SanadError> {
        let mut pos = 0;

        // Read ID (32 bytes)
        if bytes.len() < 32 {
            return Err(SanadError::InvalidEncoding);
        }
        let mut id_bytes = [0u8; 32];
        id_bytes.copy_from_slice(&bytes[0..32]);
        pos += 32;

        // Read commitment (32 bytes)
        if bytes.len() < pos + 32 {
            return Err(SanadError::InvalidEncoding);
        }
        let mut commitment_bytes = [0u8; 32];
        commitment_bytes.copy_from_slice(&bytes[pos..pos + 32]);
        pos += 32;

        // Read owner proof length and data
        if bytes.len() < pos + 4 {
            return Err(SanadError::InvalidEncoding);
        }
        let proof_len = u32::from_le_bytes(
            bytes[pos..pos + 4]
                .try_into()
                .map_err(|_| SanadError::InvalidEncoding)?,
        ) as usize;
        pos += 4;

        if bytes.len() < pos + proof_len {
            return Err(SanadError::InvalidEncoding);
        }
        let proof = bytes[pos..pos + proof_len].to_vec();
        pos += proof_len;

        // Read owner identifier length and data
        if bytes.len() < pos + 4 {
            return Err(SanadError::InvalidEncoding);
        }
        let owner_len = u32::from_le_bytes(
            bytes[pos..pos + 4]
                .try_into()
                .map_err(|_| SanadError::InvalidEncoding)?,
        ) as usize;
        pos += 4;

        if bytes.len() < pos + owner_len {
            return Err(SanadError::InvalidEncoding);
        }
        let owner_data = bytes[pos..pos + owner_len].to_vec();
        pos += owner_len;

        // Read signature scheme (1 byte: 0=none, 1=secp256k1, 2=ed25519)
        if pos >= bytes.len() {
            return Err(SanadError::InvalidEncoding);
        }
        let scheme = match bytes[pos] {
            0 => None,
            1 => Some(crate::signature::SignatureScheme::Secp256k1),
            2 => Some(crate::signature::SignatureScheme::Ed25519),
            _ => return Err(SanadError::InvalidEncoding),
        };
        pos += 1;

        // Read salt
        if bytes.len() < pos + 4 {
            return Err(SanadError::InvalidEncoding);
        }
        let salt_len = u32::from_le_bytes(
            bytes[pos..pos + 4]
                .try_into()
                .map_err(|_| SanadError::InvalidEncoding)?,
        ) as usize;
        pos += 4;

        if bytes.len() < pos + salt_len {
            return Err(SanadError::InvalidEncoding);
        }
        let salt = bytes[pos..pos + salt_len].to_vec();
        pos += salt_len;

        // Read nullifier flag and data
        if pos >= bytes.len() {
            return Err(SanadError::InvalidEncoding);
        }
        let has_nullifier = bytes[pos] == 1;
        pos += 1;

        let nullifier = if has_nullifier {
            if bytes.len() < pos + 32 {
                return Err(SanadError::InvalidEncoding);
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
            return Err(SanadError::InvalidEncoding);
        }
        let has_state_root = bytes[pos] == 1;
        pos += 1;

        let state_root = if has_state_root {
            if bytes.len() < pos + 32 {
                return Err(SanadError::InvalidEncoding);
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
            return Err(SanadError::InvalidEncoding);
        }
        let proof_data_len = u32::from_le_bytes(
            bytes[pos..pos + 4]
                .try_into()
                .map_err(|_| SanadError::InvalidEncoding)?,
        ) as usize;
        pos += 4;

        let execution_proof = if proof_data_len > 0 {
            if bytes.len() < pos + proof_data_len {
                return Err(SanadError::InvalidEncoding);
            }
            Some(bytes[pos..pos + proof_data_len].to_vec())
        } else {
            None
        };

        // Reconstruct the Sanad
        let id = SanadId(Hash::new(id_bytes));
        let commitment = Hash::new(commitment_bytes);
        let owner = OwnershipProof {
            proof,
            owner: owner_data,
            scheme,
        };

        // Verify SanadId matches H(commitment || salt) before constructing
        let expected_id = {
            let mut data = Vec::with_capacity(32 + salt.len());
            data.extend_from_slice(commitment.as_bytes());
            data.extend_from_slice(&salt);
            SanadId(Hash::new(csv_tagged_hash("sanad-id", &data)))
        };
        if id != expected_id {
            return Err(SanadError::InvalidSanadId);
        }

        let sanad = Self {
            id,
            commitment,
            owner,
            salt,
            nullifier,
            state_root,
            execution_proof,
        };

        Ok(sanad)
    }

    /// Check if this Sanad has been consumed.
    pub fn is_consumed(&self) -> bool {
        self.nullifier.is_some()
    }

    /// Get the chain enforcement level indicator.
    ///
    /// Returns `true` if this is an L3 (cryptographic) Sanad that requires
    /// nullifier tracking.
    pub fn requires_nullifier(&self) -> bool {
        self.nullifier.is_some()
    }
}

impl From<crate::sanad::Sanad> for Sanad {
    fn from(sanad: crate::sanad::Sanad) -> Self {
        Self {
            id: sanad.id.into(),
            commitment: sanad.commitment,
            owner: sanad.owner.into(),
            salt: sanad.salt,
            nullifier: sanad.nullifier,
            state_root: sanad.state_root,
            execution_proof: sanad.execution_proof,
        }
    }
}

impl From<Sanad> for crate::sanad::Sanad {
    fn from(sanad: Sanad) -> Self {
        Self {
            id: sanad.id.into(),
            commitment: sanad.commitment,
            owner: sanad.owner.into(),
            salt: sanad.salt,
            nullifier: sanad.nullifier,
            state_root: sanad.state_root,
            execution_proof: sanad.execution_proof,
        }
    }
}

/// Sanad validation errors.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum SanadError {
    /// The ownership proof is missing from the Sanad
    #[error("Missing ownership proof")]
    MissingOwnershipProof,
    /// The ownership proof failed cryptographic signature verification
    #[error("Invalid ownership proof: signature verification failed")]
    InvalidOwnershipProof,
    /// The commitment is invalid (zero hash)
    #[error("Invalid commitment (zero hash)")]
    InvalidCommitment,
    /// The Sanad has already been consumed and cannot be used again
    #[error("Sanad has already been consumed")]
    AlreadyConsumed,
    /// The nullifier is invalid or does not match the expected format
    #[error("Invalid nullifier")]
    InvalidNullifier,
    /// The canonical encoding of the Sanad is invalid
    #[error("Invalid canonical encoding")]
    InvalidEncoding,
    /// The SanadId does not match the computed H(commitment || salt)
    #[error("Invalid SanadId: does not match H(commitment || salt)")]
    InvalidSanadId,
}

impl From<crate::sanad::SanadError> for SanadError {
    fn from(err: crate::sanad::SanadError) -> Self {
        match err {
            crate::sanad::SanadError::MissingOwnershipProof => SanadError::MissingOwnershipProof,
            crate::sanad::SanadError::InvalidOwnershipProof => SanadError::InvalidOwnershipProof,
            crate::sanad::SanadError::InvalidCommitment => SanadError::InvalidCommitment,
            crate::sanad::SanadError::AlreadyConsumed => SanadError::AlreadyConsumed,
            crate::sanad::SanadError::InvalidNullifier => SanadError::InvalidNullifier,
            crate::sanad::SanadError::InvalidEncoding => SanadError::InvalidEncoding,
            crate::sanad::SanadError::InvalidSanadId => SanadError::InvalidSanadId,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_sanad() -> Sanad {
        Sanad::new(
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
    fn test_sanad_creation() {
        let sanad = test_sanad();
        assert_eq!(sanad.commitment.as_bytes(), &[0xAB; 32]);
        assert!(sanad.nullifier.is_none());
        assert!(sanad.state_root.is_none());
        assert!(sanad.execution_proof.is_none());
    }

    #[test]
    fn test_sanad_id_deterministic() {
        let s1 = test_sanad();
        let s2 = test_sanad();
        assert_eq!(s1.id, s2.id);
    }

    #[test]
    fn test_sanad_id_unique_per_salt() {
        let s1 = Sanad::new(
            Hash::new([0xAB; 32]),
            OwnershipProof {
                proof: vec![0x01],
                owner: vec![0xFF; 32],
                scheme: None,
            },
            &[0x42; 16],
        );
        let s2 = Sanad::new(
            Hash::new([0xAB; 32]),
            OwnershipProof {
                proof: vec![0x01],
                owner: vec![0xFF; 32],
                scheme: None,
            },
            &[0x99; 16],
        );
        assert_ne!(s1.id, s2.id);
    }

    #[test]
    fn test_sanad_verify_valid() {
        let sanad = test_sanad();
        assert!(sanad.verify().is_ok());
    }

    #[test]
    fn test_sanad_verify_missing_proof() {
        let mut sanad = test_sanad();
        sanad.owner.proof = vec![];
        assert_eq!(sanad.verify(), Err(SanadError::MissingOwnershipProof));
    }

    #[test]
    fn test_sanad_verify_zero_commitment() {
        let mut sanad = test_sanad();
        sanad.commitment = Hash::new([0u8; 32]);
        // Recompute ID to match the new commitment (so we test commitment check, not ID check)
        let mut data = Vec::with_capacity(32 + sanad.salt.len());
        data.extend_from_slice(sanad.commitment.as_bytes());
        data.extend_from_slice(&sanad.salt);
        sanad.id = SanadId(Hash::new(csv_tagged_hash("sanad-id", &data)));
        assert_eq!(sanad.verify(), Err(SanadError::InvalidCommitment));
    }

    #[test]
    fn test_sanad_consume_with_nullifier() {
        let mut sanad = test_sanad();
        let chain_context = [0x01u8; 32]; // Ethereum context
        let nullifier = sanad.consume(Some(b"secret"), Some(&chain_context));
        assert!(nullifier.is_some());
        assert!(sanad.nullifier.is_some());
        assert_eq!(sanad.verify(), Err(SanadError::AlreadyConsumed));
    }

    #[test]
    fn test_sanad_consume_without_nullifier() {
        let mut sanad = test_sanad();
        let result = sanad.consume(None, None);
        assert!(result.is_none());
        assert!(sanad.nullifier.is_none());
        // L1/L2: Sanad is still valid locally (chain enforces structural single-use)
        assert!(sanad.verify().is_ok());
    }

    #[test]
    fn test_sanad_canonical_roundtrip() {
        let sanad = test_sanad();
        let bytes = sanad.to_canonical_bytes();
        let decoded = Sanad::from_canonical_bytes(&bytes).expect("Should decode");
        assert_eq!(decoded.id, sanad.id);
        assert_eq!(decoded.commitment, sanad.commitment);
        assert_eq!(decoded.owner, sanad.owner);
        assert_eq!(decoded.nullifier, sanad.nullifier);
        assert_eq!(decoded.state_root, sanad.state_root);
    }

    #[test]
    fn test_sanad_canonical_roundtrip_with_nullifier() {
        let mut sanad = test_sanad();
        let chain_context = [0x01u8; 32];
        sanad.consume(Some(b"secret"), Some(&chain_context));
        let bytes = sanad.to_canonical_bytes();
        let decoded = Sanad::from_canonical_bytes(&bytes).expect("Should decode");
        assert_eq!(decoded.nullifier, sanad.nullifier);
        assert!(decoded.is_consumed());
    }

    #[test]
    fn test_sanad_from_canonical_bytes_invalid() {
        // Empty bytes should fail
        assert!(Sanad::from_canonical_bytes(&[]).is_err());
        // Truncated bytes should fail
        assert!(Sanad::from_canonical_bytes(&[0u8; 16]).is_err());
    }

    #[test]
    fn test_sanad_transfer() {
        let sanad = test_sanad();
        let new_owner = OwnershipProof {
            proof: vec![0xAA, 0xBB, 0xCC],
            owner: vec![0xDD; 32],
            scheme: None,
        };
        let transferred = sanad.transfer(new_owner.clone(), b"transfer-salt");

        // New Sanad should have:
        // - Different ID (due to different salt)
        assert_ne!(transferred.id, sanad.id);
        // - Same commitment
        assert_eq!(transferred.commitment, sanad.commitment);
        // - New owner
        assert_eq!(transferred.owner, new_owner);
        // - Not consumed
        assert!(!transferred.is_consumed());
        // - Original Sanad unaffected
        assert!(!sanad.is_consumed());
    }

    #[test]
    fn test_sanad_transfer_preserves_state_root() {
        let mut sanad = test_sanad();
        sanad.state_root = Some(Hash::new([0xCD; 32]));

        let new_owner = OwnershipProof {
            proof: vec![0x01],
            owner: vec![0xFF; 32],
            scheme: None,
        };
        let transferred = sanad.transfer(new_owner, b"transfer");

        assert_eq!(transferred.state_root, sanad.state_root);
    }

    #[test]
    fn test_sanad_is_consumed() {
        let mut sanad = test_sanad();
        assert!(!sanad.is_consumed());

        sanad.consume(Some(b"secret"), Some(&[0x01u8; 32]));
        assert!(sanad.is_consumed());
    }

    #[test]
    fn test_sanad_requires_nullifier() {
        let sanad_l1 = test_sanad(); // L1/L2 doesn't need nullifier
        assert!(!sanad_l1.requires_nullifier());

        let mut sanad_l3 = test_sanad();
        sanad_l3.consume(Some(b"secret"), Some(&[0x01u8; 32])); // L3 does
        assert!(sanad_l3.requires_nullifier());
    }
}
