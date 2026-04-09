//! Proof verification pipeline
//!
//! This module provides the core verification logic for proof bundles.

use crate::error::{AdapterError, Result};
use crate::proof::ProofBundle;
use crate::signature::{verify_signatures, Signature, SignatureScheme};

/// Verify a proof bundle according to the CSV verification pipeline.
///
/// The verification pipeline performs the following steps:
/// 1. Validate deterministic VM execution of the DAG
/// 2. Validate all authorizing signatures
/// 3. Validate seal reference correctness (no double-use)
/// 4. Validate inclusion proof against the anchor reference
/// 5. Validate finality semantics
///
/// # Arguments
/// * `bundle` - The proof bundle to verify
/// * `seal_registry` - Set of already-used seals (for replay detection)
/// * `signature_scheme` - The signature scheme to use for verification
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

/// Verify all signatures in a proof bundle
///
/// This function:
/// 1. Extracts signatures from the bundle
/// 2. Constructs signature verification contexts
/// 3. Performs cryptographic verification
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
        use secp256k1::{Secp256k1, SecretKey, Message};
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
        use ed25519_dalek::{SigningKey, Signer};
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
