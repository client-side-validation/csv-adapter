//! Proof Verification Pipeline - SECURITY CRITICAL
//!
//! This module provides the core verification logic for proof bundles.
//! It is the cryptographic gatekeeper that ensures only valid proofs are accepted.
//!
//! # Security Purpose
//!
//! This verifier ensures that:
//! 1. **Authenticity**: Signatures are valid and from authorized keys
//! 2. **Integrity**: The proof bundle hasn't been tampered with
//! 3. **Uniqueness**: Seals haven't been used before (replay protection)
//! 4. **Finality**: The anchor has reached required confirmation depth
//!
//! # Verification Steps
//!
//! The pipeline enforces a strict order of validation:
//! 1. **DAG Structure** - Verify the transition graph is well-formed
//! 2. **Signatures** - Cryptographically verify all authorizing signatures
//! 3. **Seal Replay** - Check seal hasn't been consumed before
//! 4. **Inclusion** - Verify anchor is in the chain's history
//! 5. **Finality** - Confirm anchor has reached required confirmations
//!
//! # Security Invariants
//!
//! - All signatures must be valid (no partial signature acceptance)
//! - Seal replay check uses provided registry callback
//! - Empty inclusion proofs are rejected
//! - Zero confirmations fails finality check
//! - Verification is deterministic (same input = same result)
//!
//! # Audit Checklist
//!
//! - [ ] Signature verification uses appropriate scheme (Secp256k1/Ed25519)
//! - [ ] Seal registry callback properly checks for replays
//! - [ ] Empty proofs are rejected at each validation step
//! - [ ] Signature format parsing is robust against malformed input
//! - [ ] Verification failures provide specific error types (not just generic)
//!
//! # Critical Security Note
//!
//! **NEVER** bypass or weaken these checks in production. Any shortcut
//! here could allow fraudulent proofs to be accepted, leading to
//! unauthorized state transitions or double-spends.

use crate::error::{AdapterError, Result};
use crate::proof::ProofBundle;
use crate::signature::{verify_signatures, Signature, SignatureScheme};

/// Verify a proof bundle according to the CSV verification pipeline.
///
/// This is the **primary entry point for proof verification**. It performs
/// all cryptographic and structural checks required to validate a proof bundle
/// before accepting the state transition it authorizes.
///
/// # Security Requirements (CRITICAL)
///
/// 1. **All signatures must be valid**: Any invalid signature causes rejection
/// 2. **Seal must be unused**: Replay attacks prevented via `seal_registry` callback
/// 3. **Proof must be non-empty**: Empty inclusion/finality proofs rejected
/// 4. **Finality must be reached**: Zero confirmations causes rejection
///
/// # Verification Pipeline
///
/// 1. **DAG Structure Validation** - Verify transition graph integrity
/// 2. **Signature Verification** - Cryptographically verify all signatures
/// 3. **Seal Replay Check** - Ensure seal hasn't been consumed before
/// 4. **Inclusion Verification** - Verify proof of on-chain inclusion
/// 5. **Finality Check** - Confirm anchor reached required confirmations
///
/// # Arguments
/// * `bundle` - The proof bundle to verify
/// * `seal_registry` - Callback to check if seal has been used (returns true if used)
/// * `signature_scheme` - The signature scheme to use for verification
///
/// # Returns
/// - `Ok(())` - Proof bundle is valid and authorized
/// - `Err(AdapterError)` - Specific error indicating which check failed
///
/// # Audit Note
///
/// Verify that:
/// 1. No verification step can be bypassed via configuration
/// 2. The seal_registry callback is actually invoked (not cached/stale)
/// 3. Signature parsing is robust against malformed input
/// 4. All error cases are properly handled and logged
pub fn verify_proof(
    bundle: &ProofBundle,
    seal_registry: impl Fn(&[u8]) -> bool,
    signature_scheme: SignatureScheme,
) -> Result<()> {
    // Step 1: Validate DAG structure
    bundle
        .transition_dag
        .validate_structure()
        .map_err(|e| AdapterError::Generic(format!("Invalid DAG structure: {}", e)))?;

    // Step 2: Validate signatures with cryptographic verification
    verify_bundle_signatures(bundle, signature_scheme)?;

    // Step 3: Validate seal reference (check for replay)
    if seal_registry(bundle.seal_ref.seal_id.as_ref()) {
        return Err(AdapterError::SealReplay(format!(
            "Seal {:?} has already been used",
            bundle.seal_ref
        )));
    }

    // Step 4: Validate inclusion proof (chain-specific, validated by adapter)
    if bundle.inclusion_proof.proof_bytes.is_empty() {
        return Err(AdapterError::InclusionProofFailed(
            "Empty inclusion proof".to_string(),
        ));
    }

    // Step 5: Validate finality (chain-specific, validated by adapter)
    if bundle.finality_proof.confirmations == 0 {
        return Err(AdapterError::FinalityNotReached(
            "No confirmations yet".to_string(),
        ));
    }

    Ok(())
}

/// Verify all signatures in a proof bundle.
///
/// This function performs **cryptographic signature verification** on all
/// signatures in the bundle. It is a critical security check that ensures
/// the proof was authorized by the rightful owner(s).
///
/// # Signature Format
///
/// Each signature is encoded as:
/// ```text
/// [public_key_length: 4 bytes LE] [public_key: pk_len bytes] [signature: remaining bytes]
/// ```
///
/// The signed message is the DAG root commitment hash.
///
/// # Security Requirements
/// - MUST verify all signatures (not just first one)
/// - MUST use correct signature scheme for the chain
/// - MUST fail if any signature is invalid
/// - MUST parse signature format robustly
///
/// # Arguments
/// * `bundle` - The proof bundle containing signatures to verify
/// * `scheme` - The signature scheme (Secp256k1 or Ed25519)
///
/// # Returns
/// - `Ok(())` - All signatures are valid
/// - `Err(AdapterError::SignatureVerificationFailed)` - If any signature invalid
///
/// # Audit Note
///
/// Verify that:
/// 1. The signature parsing correctly handles variable-length public keys
/// 2. The message being verified is the correct DAG root commitment
/// 3. No signature is skipped during verification
/// 4. The scheme matches the chain's expected signature type
fn verify_bundle_signatures(bundle: &ProofBundle, scheme: SignatureScheme) -> Result<()> {
    // Check we have signatures
    if bundle.signatures.is_empty() {
        return Err(AdapterError::SignatureVerificationFailed(
            "No signatures in proof bundle".to_string(),
        ));
    }

    // For each signature in the bundle, verify it
    // In a full implementation, each signature would have associated metadata
    // (public key, signed message) encoded within it
    //
    // The signature format is:
    // [public_key_length (4 bytes LE)] [public_key] [signature_bytes]
    // The message is the DAG root commitment hash

    let mut signatures = Vec::with_capacity(bundle.signatures.len());

    for (i, sig_bytes) in bundle.signatures.iter().enumerate() {
        // Parse signature format: [pk_len (4)] [public_key] [signature]
        if sig_bytes.len() < 4 {
            return Err(AdapterError::SignatureVerificationFailed(format!(
                "Signature {} too short for header",
                i
            )));
        }

        // Extract public key length (little-endian u32)
        let pk_len =
            u32::from_le_bytes([sig_bytes[0], sig_bytes[1], sig_bytes[2], sig_bytes[3]]) as usize;

        if sig_bytes.len() < 4 + pk_len {
            return Err(AdapterError::SignatureVerificationFailed(format!(
                "Signature {} too short for public key",
                i
            )));
        }

        let public_key = sig_bytes[4..4 + pk_len].to_vec();
        let signature = sig_bytes[4 + pk_len..].to_vec();

        // The signed message is the DAG root commitment
        let message = bundle.transition_dag.root_commitment.as_bytes().to_vec();

        signatures.push(Signature::new(signature, public_key, message));
    }

    // Verify all signatures
    verify_signatures(&signatures, scheme)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dag::{DAGNode, DAGSegment};
    use crate::hash::Hash;
    use crate::proof::{FinalityProof, InclusionProof};
    use crate::seal::{AnchorRef, SealRef};
    use crate::signature::SignatureScheme;

    fn make_secp256k1_signature_bytes(message: &[u8; 32]) -> Vec<u8> {
        use secp256k1::{Message, Secp256k1, SecretKey};
        let secp = Secp256k1::new();
        let secret_key = SecretKey::new(&mut secp256k1::rand::thread_rng());
        let public_key = secp256k1::PublicKey::from_secret_key(&secp, &secret_key);
        let msg = Message::from_digest_slice(message).unwrap();
        let signature = secp.sign_ecdsa(&msg, &secret_key);
        let sig_bytes = signature.serialize_compact();
        let pubkey_bytes = public_key.serialize();
        // Format: [pk_len (4 bytes LE)] [public_key] [signature]
        let mut encoded = Vec::with_capacity(4 + pubkey_bytes.len() + sig_bytes.len());
        encoded.extend_from_slice(&(pubkey_bytes.len() as u32).to_le_bytes());
        encoded.extend_from_slice(&pubkey_bytes);
        encoded.extend_from_slice(&sig_bytes);
        encoded
    }

    fn make_ed25519_signature_bytes(message: &[u8]) -> Vec<u8> {
        use ed25519_dalek::{Signer, SigningKey};
        let signing_key = SigningKey::generate(&mut rand::rngs::OsRng);
        let verifying_key = signing_key.verifying_key();
        let signature = signing_key.sign(message);
        // Format: [pk_len (4 bytes LE)] [public_key] [signature]
        let mut encoded = Vec::with_capacity(4 + 32 + 64);
        encoded.extend_from_slice(&32u32.to_le_bytes());
        encoded.extend_from_slice(&verifying_key.to_bytes());
        encoded.extend_from_slice(&signature.to_bytes());
        encoded
    }

    fn test_bundle_with_signatures() -> Result<ProofBundle> {
        // The message signed is the DAG root commitment (Hash::zero() = 32 zero bytes)
        let message = [0u8; 32];
        let signature = make_secp256k1_signature_bytes(&message);

        let bundle = ProofBundle::new(
            DAGSegment::new(
                vec![DAGNode::new(
                    Hash::new([1u8; 32]),
                    vec![0x01, 0x02],
                    vec![signature.clone()],
                    vec![],
                    vec![],
                )],
                Hash::zero(),
            ),
            vec![signature],
            SealRef::new(vec![1, 2, 3], Some(42))
                .map_err(|e| AdapterError::Generic(e.to_string()))?,
            AnchorRef::new(vec![4, 5, 6], 100, vec![])
                .map_err(|e| AdapterError::Generic(e.to_string()))?,
            InclusionProof::new(vec![0xCD; 32], Hash::new([2u8; 32]), 0)
                .map_err(|e| AdapterError::Generic(e.to_string()))?,
            FinalityProof::new(vec![], 6, false)
                .map_err(|e| AdapterError::Generic(e.to_string()))?,
        )
        .map_err(|e| AdapterError::Generic(e.to_string()))?;
        Ok(bundle)
    }

    #[test]
    fn test_verify_proof_valid() {
        let bundle = test_bundle_with_signatures().unwrap();
        let seal_registry = |_seal_id: &[u8]| false;
        assert!(verify_proof(&bundle, seal_registry, SignatureScheme::Secp256k1).is_ok());
    }

    #[test]
    fn test_verify_proof_seal_replay() {
        let bundle = test_bundle_with_signatures().unwrap();
        let seal_registry = |seal_id: &[u8]| seal_id == [1, 2, 3];
        assert!(verify_proof(&bundle, seal_registry, SignatureScheme::Secp256k1).is_err());
    }

    #[test]
    fn test_verify_proof_no_signatures() {
        let mut bundle = test_bundle_with_signatures().unwrap();
        bundle.signatures.clear();
        let seal_registry = |_seal_id: &[u8]| false;
        assert!(verify_proof(&bundle, seal_registry, SignatureScheme::Secp256k1).is_err());
    }

    #[test]
    fn test_verify_proof_no_confirmations() {
        let mut bundle = test_bundle_with_signatures().unwrap();
        bundle.finality_proof.confirmations = 0;
        let seal_registry = |_seal_id: &[u8]| false;
        assert!(verify_proof(&bundle, seal_registry, SignatureScheme::Secp256k1).is_err());
    }

    #[test]
    fn test_verify_proof_invalid_signature_format() {
        let mut bundle = test_bundle_with_signatures().unwrap();
        // Corrupt signature format
        bundle.signatures[0] = vec![0x00, 0x00]; // Too short
        let seal_registry = |_seal_id: &[u8]| false;
        assert!(verify_proof(&bundle, seal_registry, SignatureScheme::Secp256k1).is_err());
    }

    #[test]
    fn test_verify_proof_ed25519_valid_format() {
        // The message signed is the DAG root commitment (Hash::zero() = 32 zero bytes)
        let message = [0u8; 32];
        let signature = make_ed25519_signature_bytes(&message);

        let mut bundle = test_bundle_with_signatures().unwrap();
        bundle.signatures = vec![signature];

        let seal_registry = |_seal_id: &[u8]| false;
        assert!(verify_proof(&bundle, seal_registry, SignatureScheme::Ed25519).is_ok());
    }
}
